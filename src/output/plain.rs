use std::io::Write;

use crate::error::CliError;

/// Write a JSON value in a human-readable plain-text format.
pub fn write_plain(value: &serde_json::Value, writer: &mut dyn Write) -> Result<(), CliError> {
    match value {
        serde_json::Value::Object(map) => {
            // Try to detect Notion object type and render accordingly
            let obj_type = map.get("object").and_then(|v| v.as_str());
            match obj_type {
                Some("page") => write_page_plain(map, writer)?,
                Some("database") => write_database_plain(map, writer)?,
                Some("user") => write_user_plain(map, writer)?,
                Some("block") => write_block_plain(map, writer)?,
                Some("comment") => write_comment_plain(map, writer)?,
                Some("list") => {
                    if let Some(results) = map.get("results").and_then(|v| v.as_array()) {
                        for item in results {
                            write_plain(item, writer)?;
                            writeln!(writer)?;
                        }
                        let total = results.len();
                        if let Some(true) = map.get("has_more").and_then(|v| v.as_bool()) {
                            writeln!(writer, "--- ({total} results, more available) ---")?;
                        } else {
                            writeln!(writer, "--- ({total} results) ---")?;
                        }
                    }
                }
                _ => {
                    // Generic object: print key-value pairs
                    for (key, val) in map {
                        writeln!(writer, "{key}: {}", format_value_short(val))?;
                    }
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                write_plain(item, writer)?;
                writeln!(writer)?;
            }
        }
        other => {
            writeln!(writer, "{}", format_value_short(other))?;
        }
    }
    Ok(())
}

fn write_page_plain(
    map: &serde_json::Map<String, serde_json::Value>,
    writer: &mut dyn Write,
) -> Result<(), CliError> {
    let id = map.get("id").and_then(|v| v.as_str()).unwrap_or("?");
    let title = extract_page_title(map);
    let url = map.get("url").and_then(|v| v.as_str()).unwrap_or("");

    writeln!(writer, "Page: {title}")?;
    writeln!(writer, "  ID:  {id}")?;
    if !url.is_empty() {
        writeln!(writer, "  URL: {url}")?;
    }
    if let Some(created) = map.get("created_time").and_then(|v| v.as_str()) {
        writeln!(writer, "  Created: {created}")?;
    }
    if let Some(edited) = map.get("last_edited_time").and_then(|v| v.as_str()) {
        writeln!(writer, "  Edited:  {edited}")?;
    }
    Ok(())
}

fn write_database_plain(
    map: &serde_json::Map<String, serde_json::Value>,
    writer: &mut dyn Write,
) -> Result<(), CliError> {
    let id = map.get("id").and_then(|v| v.as_str()).unwrap_or("?");
    let title = extract_title_from_rich_text(map.get("title"));
    let url = map.get("url").and_then(|v| v.as_str()).unwrap_or("");

    writeln!(writer, "Database: {title}")?;
    writeln!(writer, "  ID:  {id}")?;
    if !url.is_empty() {
        writeln!(writer, "  URL: {url}")?;
    }
    Ok(())
}

fn write_user_plain(
    map: &serde_json::Map<String, serde_json::Value>,
    writer: &mut dyn Write,
) -> Result<(), CliError> {
    let id = map.get("id").and_then(|v| v.as_str()).unwrap_or("?");
    let name = map.get("name").and_then(|v| v.as_str()).unwrap_or("?");
    let user_type = map.get("type").and_then(|v| v.as_str()).unwrap_or("?");

    writeln!(writer, "User: {name} ({user_type})")?;
    writeln!(writer, "  ID: {id}")?;
    if let Some(email) = map
        .get("person")
        .and_then(|p| p.get("email"))
        .and_then(|v| v.as_str())
    {
        writeln!(writer, "  Email: {email}")?;
    }
    Ok(())
}

fn write_block_plain(
    map: &serde_json::Map<String, serde_json::Value>,
    writer: &mut dyn Write,
) -> Result<(), CliError> {
    let id = map.get("id").and_then(|v| v.as_str()).unwrap_or("?");
    let block_type = map.get("type").and_then(|v| v.as_str()).unwrap_or("?");

    writeln!(writer, "[{block_type}] ({id})")?;

    // Try to extract text content
    if let Some(type_data) = map.get(block_type) {
        if let Some(rich_text) = type_data.get("rich_text").and_then(|v| v.as_array()) {
            let text: String = rich_text
                .iter()
                .filter_map(|rt| rt.get("plain_text").and_then(|v| v.as_str()))
                .collect();
            if !text.is_empty() {
                writeln!(writer, "  {text}")?;
            }
        }
    }
    Ok(())
}

fn write_comment_plain(
    map: &serde_json::Map<String, serde_json::Value>,
    writer: &mut dyn Write,
) -> Result<(), CliError> {
    let id = map.get("id").and_then(|v| v.as_str()).unwrap_or("?");
    let text = extract_title_from_rich_text(map.get("rich_text"));
    let created = map
        .get("created_time")
        .and_then(|v| v.as_str())
        .unwrap_or("?");

    writeln!(writer, "Comment ({id}) [{created}]")?;
    writeln!(writer, "  {text}")?;
    Ok(())
}

fn extract_page_title(map: &serde_json::Map<String, serde_json::Value>) -> String {
    if let Some(props) = map.get("properties").and_then(|v| v.as_object()) {
        for (_key, prop_val) in props {
            if prop_val.get("type").and_then(|v| v.as_str()) == Some("title") {
                return extract_title_from_rich_text(prop_val.get("title"));
            }
        }
    }
    "(untitled)".to_string()
}

fn extract_title_from_rich_text(value: Option<&serde_json::Value>) -> String {
    if let Some(arr) = value.and_then(|v| v.as_array()) {
        let text: String = arr
            .iter()
            .filter_map(|rt| rt.get("plain_text").and_then(|v| v.as_str()))
            .collect();
        if !text.is_empty() {
            return text;
        }
    }
    "(untitled)".to_string()
}

fn format_value_short(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => "(null)".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Array(arr) => format!("[{} items]", arr.len()),
        serde_json::Value::Object(_) => "{...}".to_string(),
    }
}
