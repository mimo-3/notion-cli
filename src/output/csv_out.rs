use std::io::Write;

use crate::error::CliError;

/// Write a JSON value as CSV (or TSV with a different delimiter).
/// For arrays of objects, each object becomes a row; keys become headers.
/// For single objects, output a single row.
pub fn write_csv(
    value: &serde_json::Value,
    writer: &mut dyn Write,
    delimiter: u8,
) -> Result<(), CliError> {
    let rows: Vec<&serde_json::Value> = match value {
        serde_json::Value::Array(arr) => arr.iter().collect(),
        obj @ serde_json::Value::Object(_) => vec![obj],
        _ => {
            // Scalar value: just print it
            writeln!(writer, "{}", value_to_string(value))?;
            return Ok(());
        }
    };

    if rows.is_empty() {
        return Ok(());
    }

    // Collect all unique keys in order of first appearance
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

    let mut csv_writer = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .from_writer(writer);

    csv_writer
        .write_record(&headers)
        .map_err(|e| CliError::Config(format!("CSV write error: {e}")))?;

    for row in &rows {
        let fields: Vec<String> = headers
            .iter()
            .map(|h| {
                row.get(h)
                    .map(|v| value_to_string(v))
                    .unwrap_or_default()
            })
            .collect();
        csv_writer
            .write_record(&fields)
            .map_err(|e| CliError::Config(format!("CSV write error: {e}")))?;
    }

    csv_writer
        .flush()
        .map_err(|e| CliError::Config(format!("CSV flush error: {e}")))?;

    Ok(())
}

fn value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => serde_json::to_string(v).unwrap_or_default(),
    }
}
