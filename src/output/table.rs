use std::io::Write;

use comfy_table::{ContentArrangement, Table};

use crate::error::CliError;

/// Render a JSON array of objects as an ASCII table.
pub fn write_table(value: &serde_json::Value, writer: &mut dyn Write) -> Result<(), CliError> {
    let rows: Vec<&serde_json::Value> = match value {
        serde_json::Value::Array(arr) => arr.iter().collect(),
        obj @ serde_json::Value::Object(_) => vec![obj],
        _ => {
            writeln!(writer, "{value}")?;
            return Ok(());
        }
    };

    if rows.is_empty() {
        writeln!(writer, "(no results)")?;
        return Ok(());
    }

    // Collect headers from all rows
    let mut headers: Vec<String> = Vec::new();
    for row in &rows {
        if let serde_json::Value::Object(map) = row {
            for key in map.keys() {
                if !headers.contains(key) {
                    headers.push(key.clone());
                }
            }
        }
    }

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(&headers);

    for row in &rows {
        let cells: Vec<String> = headers
            .iter()
            .map(|h| {
                row.get(h)
                    .map(|v| match v {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Null => String::new(),
                        other => other.to_string(),
                    })
                    .unwrap_or_default()
            })
            .collect();
        table.add_row(cells);
    }

    writeln!(writer, "{table}")?;
    Ok(())
}
