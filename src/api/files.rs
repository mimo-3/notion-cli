use serde_json::{json, Value};

use crate::client::pagination::PaginationOpts;
use crate::client::NotionClient;
use crate::error::CliError;

const PART_SIZE: usize = 5 * 1024 * 1024; // 5MB

impl NotionClient {
    /// Step 1: Create a file upload session.
    pub async fn create_file_upload(
        &self,
        filename: &str,
        content_type: &str,
        parent_page_id: &str,
        number_of_parts: u32,
    ) -> Result<Value, CliError> {
        let body = json!({
            "mode": "multi_part",
            "number_of_parts": number_of_parts,
            "filename": filename,
            "content_type": content_type,
            "parent": {
                "type": "page_id",
                "page_id": parent_page_id,
            }
        });
        self.post("/v1/file-uploads", &body).await
    }

    /// Step 2: Upload file data to the pre-signed URL.
    /// The upload URL is NOT on api.notion.com — it's a pre-signed S3-like URL.
    pub async fn upload_file_part(
        &self,
        upload_url: &str,
        data: Vec<u8>,
        part_number: u32,
        content_type: &str,
    ) -> Result<(), CliError> {
        let form = reqwest::multipart::Form::new()
            .part(
                "file",
                reqwest::multipart::Part::bytes(data)
                    .file_name("file")
                    .mime_str(content_type)
                    .map_err(|e| CliError::Config(format!("Invalid content type: {e}")))?,
            )
            .text("part_number", part_number.to_string());

        let response = self
            .http
            .post(upload_url)
            .headers(self.notion_headers())
            .multipart(form)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(CliError::Api {
                status: status.as_u16(),
                code: "upload_failed".to_string(),
                message: error_body,
            });
        }
        Ok(())
    }

    /// Step 3: Mark the file upload as complete.
    pub async fn complete_file_upload(&self, file_upload_id: &str) -> Result<Value, CliError> {
        let body = json!({});
        self.post(
            &format!("/v1/file-uploads/{file_upload_id}/complete"),
            &body,
        )
        .await
    }

    /// Get file upload metadata.
    pub async fn get_file_upload(&self, file_upload_id: &str) -> Result<Value, CliError> {
        self.get(&format!("/v1/file-uploads/{file_upload_id}")).await
    }

    /// List file uploads.
    pub async fn list_file_uploads(&self, opts: &PaginationOpts) -> Result<Vec<Value>, CliError> {
        self.paginate_get("/v1/file-uploads", opts).await
    }

    /// Full upload flow: create session, upload parts, complete.
    pub async fn upload_file(
        &self,
        file_data: Vec<u8>,
        filename: &str,
        content_type: &str,
        parent_page_id: &str,
    ) -> Result<Value, CliError> {
        let file_size = file_data.len();
        let number_of_parts = ((file_size + PART_SIZE - 1) / PART_SIZE).max(1) as u32;

        eprintln!(
            "Creating upload session for '{}' ({} bytes, {} part(s))...",
            filename, file_size, number_of_parts
        );

        // Step 1: Create upload session
        let session = self
            .create_file_upload(filename, content_type, parent_page_id, number_of_parts)
            .await?;

        let file_upload_id = session
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CliError::Config("Missing file upload id in response".to_string()))?
            .to_string();

        // Extract part upload URLs
        let upload_urls: Vec<String> = session
            .get("part_upload_urls")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Step 2: Upload parts
        for part_idx in 0..number_of_parts {
            let start = part_idx as usize * PART_SIZE;
            let end = (start + PART_SIZE).min(file_size);
            let chunk = file_data[start..end].to_vec();

            let upload_url = if upload_urls.len() > part_idx as usize {
                &upload_urls[part_idx as usize]
            } else {
                return Err(CliError::Config(format!(
                    "Missing upload URL for part {}",
                    part_idx + 1
                )));
            };

            eprintln!(
                "Uploading part {}/{} ({} bytes)...",
                part_idx + 1,
                number_of_parts,
                chunk.len()
            );

            self.upload_file_part(upload_url, chunk, part_idx + 1, content_type)
                .await?;
        }

        // Step 3: Complete
        eprintln!("Completing upload...");
        let result = self.complete_file_upload(&file_upload_id).await?;

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
