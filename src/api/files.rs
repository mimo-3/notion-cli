use serde_json::{json, Value};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::client::pagination::PaginationOpts;
use crate::client::NotionClient;
use crate::error::CliError;

const SINGLE_PART_LIMIT: usize = 20 * 1024 * 1024;
const PART_SIZE: usize = 20 * 1024 * 1024;
const MAX_PARTS: usize = 10_000;

fn upload_part_count(file_size: usize) -> Result<u32, CliError> {
    let parts = if file_size <= SINGLE_PART_LIMIT {
        1
    } else {
        file_size.div_ceil(PART_SIZE)
    };
    if parts > MAX_PARTS {
        return Err(CliError::Config(format!(
            "file requires {parts} parts; Notion allows at most {MAX_PARTS}"
        )));
    }
    Ok(parts as u32)
}

impl NotionClient {
    /// Step 1: Create a file upload session.
    pub async fn create_file_upload(
        &self,
        filename: &str,
        content_type: &str,
        number_of_parts: u32,
    ) -> Result<Value, CliError> {
        if !(1..=MAX_PARTS as u32).contains(&number_of_parts) {
            return Err(CliError::Config(
                "number_of_parts must be between 1 and 10000".to_string(),
            ));
        }

        let body = if number_of_parts == 1 {
            json!({
                "mode": "single_part",
                "filename": filename,
                "content_type": content_type,
            })
        } else {
            json!({
                "mode": "multi_part",
                "number_of_parts": number_of_parts,
                "filename": filename,
                "content_type": content_type,
            })
        };
        self.post("/v1/file_uploads", &body).await
    }

    /// Send file data to the Notion API.
    pub async fn upload_file_part(
        &self,
        file_upload_id: &str,
        data: Vec<u8>,
        filename: &str,
        content_type: &str,
        part_number: Option<u32>,
    ) -> Result<Value, CliError> {
        // Validate before entering the retry closure. Building a new form for each
        // attempt is necessary because multipart request bodies are consumed.
        reqwest::multipart::Part::bytes(Vec::new())
            .mime_str(content_type)
            .map_err(|e| CliError::Config(format!("Invalid content type: {e}")))?;

        let url = self.api_url(&format!("/v1/file_uploads/{file_upload_id}/send"))?;
        let filename = filename.to_string();
        let content_type = content_type.to_string();
        let mut headers = self.notion_headers();
        headers.remove(reqwest::header::CONTENT_TYPE);

        self.request_with_retry(|| {
            let file = reqwest::multipart::Part::bytes(data.clone())
                .file_name(filename.clone())
                .mime_str(&content_type)
                .expect("content type was validated before sending");
            let mut form = reqwest::multipart::Form::new().part("file", file);
            if let Some(part_number) = part_number {
                form = form.text("part_number", part_number.to_string());
            }

            self.http
                .post(url.clone())
                .headers(headers.clone())
                .multipart(form)
        })
        .await
    }

    /// Step 3: Mark the file upload as complete.
    pub async fn complete_file_upload(&self, file_upload_id: &str) -> Result<Value, CliError> {
        let body = json!({});
        self.post(
            &format!("/v1/file_uploads/{file_upload_id}/complete"),
            &body,
        )
        .await
    }

    /// Get file upload metadata.
    pub async fn get_file_upload(&self, file_upload_id: &str) -> Result<Value, CliError> {
        self.get(&format!("/v1/file_uploads/{file_upload_id}"))
            .await
    }

    /// List file uploads.
    pub async fn list_file_uploads(&self, opts: &PaginationOpts) -> Result<Vec<Value>, CliError> {
        self.paginate_get("/v1/file_uploads", opts).await
    }

    /// Full upload flow from a file path, buffering at most one part at a time.
    pub async fn upload_file_path(
        &self,
        file_path: &Path,
        filename: &str,
        content_type: &str,
    ) -> Result<Value, CliError> {
        let file_size = usize::try_from(std::fs::metadata(file_path)?.len()).map_err(|_| {
            CliError::Config("file is too large for this platform to address".to_string())
        })?;
        let number_of_parts = upload_part_count(file_size)?;

        eprintln!(
            "Creating upload session for '{}' ({} bytes, {} part(s))...",
            filename, file_size, number_of_parts
        );

        if self.dry_run {
            eprintln!("[dry-run] POST /v1/file_uploads");
            for part_number in 1..=number_of_parts {
                eprintln!(
                    "[dry-run] POST /v1/file_uploads/<id>/send (part {part_number}/{number_of_parts})"
                );
            }
            if number_of_parts > 1 {
                eprintln!("[dry-run] POST /v1/file_uploads/<id>/complete");
            }
            return Ok(json!({}));
        }

        // Step 1: Create upload session
        let session = self
            .create_file_upload(filename, content_type, number_of_parts)
            .await?;

        let file_upload_id = session
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CliError::Config("Missing file upload id in response".to_string()))?
            .to_string();

        // Step 2: Upload parts
        let mut file = std::fs::File::open(file_path)?;
        let mut last_send_result = None;
        for part_idx in 0..number_of_parts {
            let start = part_idx as usize * PART_SIZE;
            let end = (start + PART_SIZE).min(file_size);
            let mut chunk = vec![0; end - start];
            file.seek(SeekFrom::Start(start as u64))?;
            file.read_exact(&mut chunk)?;

            eprintln!(
                "Uploading part {}/{} ({} bytes)...",
                part_idx + 1,
                number_of_parts,
                chunk.len()
            );

            last_send_result = Some(
                self.upload_file_part(
                    &file_upload_id,
                    chunk,
                    filename,
                    content_type,
                    (number_of_parts > 1).then_some(part_idx + 1),
                )
                .await?,
            );
        }

        // Single-part uploads become uploaded after send; only multi-part uploads
        // have a completion step.
        let result = if number_of_parts > 1 {
            eprintln!("Completing upload...");
            self.complete_file_upload(&file_upload_id).await?
        } else {
            last_send_result.expect("a single-part upload always sends one part")
        };

        eprintln!("Upload complete. File ID: {}", file_upload_id);
        Ok(result)
    }
}

/// Detect content type from file extension.
pub fn detect_content_type(filename: &str) -> &'static str {
    let ext = filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "zip" => "application/zip",
        "csv" => "text/csv",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use url::Url;
    use wiremock::{
        matchers::{body_json, header, method, path, query_param},
        Mock, MockBuilder, MockServer, ResponseTemplate,
    };

    use super::*;

    const TOKEN: &str = "secret_test_token";
    const VERSION: &str = "2026-03-11";

    fn client_for(server: &MockServer) -> NotionClient {
        NotionClient::new(TOKEN.to_string())
            .unwrap()
            .with_base_url(Url::parse(&format!("{}/", server.uri())).unwrap())
    }

    fn file_upload_response(id: &str) -> Value {
        json!({"object": "file_upload", "id": id, "status": "pending"})
    }

    fn authenticated(mock: MockBuilder) -> MockBuilder {
        mock.and(header("authorization", format!("Bearer {TOKEN}")))
            .and(header("notion-version", VERSION))
    }

    #[test]
    fn file_size_selects_single_or_valid_multi_part_mode() {
        assert_eq!(upload_part_count(0).unwrap(), 1);
        assert_eq!(upload_part_count(SINGLE_PART_LIMIT).unwrap(), 1);
        assert_eq!(upload_part_count(SINGLE_PART_LIMIT + 1).unwrap(), 2);
        assert_eq!(upload_part_count(PART_SIZE * 2).unwrap(), 2);
        assert_eq!(upload_part_count(PART_SIZE * 2 + 1).unwrap(), 3);
        assert!(upload_part_count(PART_SIZE * (MAX_PARTS + 1)).is_err());
    }

    #[tokio::test]
    async fn create_uses_official_path_and_single_part_body() {
        let server = MockServer::start().await;
        authenticated(
            Mock::given(method("POST"))
                .and(path("/v1/file_uploads"))
                .and(body_json(json!({
                    "mode": "single_part",
                    "filename": "notes.txt",
                    "content_type": "text/plain"
                }))),
        )
        .respond_with(ResponseTemplate::new(200).set_body_json(file_upload_response("fu_1")))
        .expect(1)
        .mount(&server)
        .await;

        client_for(&server)
            .create_file_upload("notes.txt", "text/plain", 1)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn create_multi_part_includes_number_of_parts() {
        let server = MockServer::start().await;
        authenticated(
            Mock::given(method("POST"))
                .and(path("/v1/file_uploads"))
                .and(body_json(json!({
                    "mode": "multi_part",
                    "number_of_parts": 2,
                    "filename": "archive.zip",
                    "content_type": "application/zip"
                }))),
        )
        .respond_with(ResponseTemplate::new(200).set_body_json(file_upload_response("fu_2")))
        .expect(1)
        .mount(&server)
        .await;

        client_for(&server)
            .create_file_upload("archive.zip", "application/zip", 2)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn send_posts_authenticated_multipart_to_notion_origin() {
        let notion = MockServer::start().await;
        authenticated(Mock::given(method("POST")).and(path("/v1/file_uploads/fu_1/send")))
            .respond_with(ResponseTemplate::new(200).set_body_json(file_upload_response("fu_1")))
            .expect(1)
            .mount(&notion)
            .await;

        client_for(&notion)
            .upload_file_part(
                "fu_1",
                b"test file contents".to_vec(),
                "notes.txt",
                "text/plain",
                Some(2),
            )
            .await
            .unwrap();

        let requests = notion.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        let content_type = request
            .headers
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(content_type.starts_with("multipart/form-data; boundary="));

        let body = String::from_utf8_lossy(&request.body);
        assert!(body.contains("name=\"file\""));
        assert!(body.contains("filename=\"notes.txt\""));
        assert!(body.contains("test file contents"));
        assert!(body.contains("name=\"part_number\""));
        assert!(body.contains("\r\n\r\n2\r\n"));
    }

    #[tokio::test]
    async fn single_part_send_omits_part_number() {
        let notion = MockServer::start().await;
        authenticated(Mock::given(method("POST")).and(path("/v1/file_uploads/fu_1/send")))
            .respond_with(ResponseTemplate::new(200).set_body_json(file_upload_response("fu_1")))
            .mount(&notion)
            .await;

        client_for(&notion)
            .upload_file_part("fu_1", b"one".to_vec(), "one.txt", "text/plain", None)
            .await
            .unwrap();

        let requests = notion.received_requests().await.unwrap();
        let body = String::from_utf8_lossy(&requests[0].body);
        assert!(!body.contains("part_number"));
    }

    #[tokio::test]
    async fn retrieve_list_and_complete_use_official_contracts() {
        let server = MockServer::start().await;
        authenticated(Mock::given(method("GET")).and(path("/v1/file_uploads/fu_1")))
            .respond_with(ResponseTemplate::new(200).set_body_json(file_upload_response("fu_1")))
            .expect(1)
            .mount(&server)
            .await;
        authenticated(
            Mock::given(method("GET"))
                .and(path("/v1/file_uploads"))
                .and(query_param("page_size", "50")),
        )
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "results": [file_upload_response("fu_1")],
            "has_more": false,
            "next_cursor": null
        })))
        .expect(1)
        .mount(&server)
        .await;
        authenticated(
            Mock::given(method("POST"))
                .and(path("/v1/file_uploads/fu_1/complete"))
                .and(body_json(json!({}))),
        )
        .respond_with(ResponseTemplate::new(200).set_body_json(file_upload_response("fu_1")))
        .expect(1)
        .mount(&server)
        .await;

        let client = client_for(&server);
        client.get_file_upload("fu_1").await.unwrap();
        let listed = client
            .list_file_uploads(&PaginationOpts::default())
            .await
            .unwrap();
        assert_eq!(listed.len(), 1);
        client.complete_file_upload("fu_1").await.unwrap();
    }

    #[tokio::test]
    async fn send_never_treats_file_upload_id_as_an_absolute_target() {
        let notion = MockServer::start().await;
        let attacker = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(file_upload_response("bad")))
            .mount(&attacker)
            .await;

        let malicious_id = format!("{}/capture", attacker.uri());
        let result = client_for(&notion)
            .upload_file_part(
                &malicious_id,
                b"data".to_vec(),
                "file.txt",
                "text/plain",
                None,
            )
            .await;

        assert!(result.is_err());
        assert!(attacker.received_requests().await.unwrap().is_empty());
    }
}
