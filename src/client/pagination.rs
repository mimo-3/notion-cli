use serde_json::Value;

use super::NotionClient;
use crate::error::CliError;

/// Options controlling pagination behavior.
#[derive(Debug, Clone)]
pub struct PaginationOpts {
    pub page_size: u8,
    pub start_cursor: Option<String>,
    pub fetch_all: bool,
    pub limit: Option<u32>,
}

impl Default for PaginationOpts {
    fn default() -> Self {
        Self {
            page_size: 50,
            start_cursor: None,
            fetch_all: false,
            limit: None,
        }
    }
}

impl NotionClient {
    /// Paginate through a POST endpoint (e.g. search, db query).
    /// Collects all results into a Vec.
    pub async fn paginate_post(
        &self,
        path: &str,
        base_body: &Value,
        opts: &PaginationOpts,
    ) -> Result<Vec<Value>, CliError> {
        let mut all_results = Vec::new();
        let mut cursor = opts.start_cursor.clone();
        let limit = opts.limit.unwrap_or(u32::MAX);

        loop {
            let mut body = base_body.clone();
            if let Some(obj) = body.as_object_mut() {
                obj.insert(
                    "page_size".to_string(),
                    Value::Number(opts.page_size.into()),
                );
                if let Some(ref c) = cursor {
                    obj.insert("start_cursor".to_string(), Value::String(c.clone()));
                }
            }

            let response = self.post(path, &body).await?;

            if let Some(results) = response.get("results").and_then(|v| v.as_array()) {
                for item in results {
                    if all_results.len() as u32 >= limit {
                        return Ok(all_results);
                    }
                    all_results.push(item.clone());
                }
            }

            if !opts.fetch_all && opts.limit.is_none() {
                // Only fetch first page
                break;
            }

            if all_results.len() as u32 >= limit {
                break;
            }

            let has_more = response
                .get("has_more")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if !has_more {
                break;
            }

            cursor = response
                .get("next_cursor")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }

        Ok(all_results)
    }

    /// Paginate through a GET endpoint with query params (e.g. block children).
    pub async fn paginate_get(
        &self,
        base_path: &str,
        opts: &PaginationOpts,
    ) -> Result<Vec<Value>, CliError> {
        let mut all_results = Vec::new();
        let mut cursor = opts.start_cursor.clone();
        let limit = opts.limit.unwrap_or(u32::MAX);

        loop {
            let mut path = format!("{base_path}?page_size={}", opts.page_size);
            if let Some(ref c) = cursor {
                path.push_str(&format!("&start_cursor={c}"));
            }

            let response = self.get(&path).await?;

            if let Some(results) = response.get("results").and_then(|v| v.as_array()) {
                for item in results {
                    if all_results.len() as u32 >= limit {
                        return Ok(all_results);
                    }
                    all_results.push(item.clone());
                }
            }

            if !opts.fetch_all && opts.limit.is_none() {
                break;
            }

            if all_results.len() as u32 >= limit {
                break;
            }

            let has_more = response
                .get("has_more")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if !has_more {
                break;
            }

            cursor = response
                .get("next_cursor")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }

        Ok(all_results)
    }
}
