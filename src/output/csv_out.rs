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
        serde_json::Value::String(_) => {
            let mut csv_writer = csv::WriterBuilder::new()
                .delimiter(delimiter)
                .from_writer(writer);
            csv_writer
                .write_record([value_to_string(value)])
                .map_err(|e| CliError::Config(format!("CSV write error: {e}")))?;
            csv_writer
                .flush()
                .map_err(|e| CliError::Config(format!("CSV flush error: {e}")))?;
            return Ok(());
        }
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

    let safe_headers: Vec<String> = headers
        .iter()
        .map(|header| neutralize_formula(header))
        .collect();

    csv_writer
        .write_record(&safe_headers)
        .map_err(|e| CliError::Config(format!("CSV write error: {e}")))?;

    for row in &rows {
        let fields: Vec<String> = headers
            .iter()
            .map(|h| row.get(h).map(value_to_string).unwrap_or_default())
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
        serde_json::Value::String(s) => neutralize_formula(s),
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => serde_json::to_string(v).unwrap_or_default(),
    }
}

fn neutralize_formula(value: &str) -> String {
    let first_non_whitespace = value
        .chars()
        .find(|character| !matches!(character, ' ' | '\t' | '\r' | '\n'));
    if matches!(first_non_whitespace, Some('=' | '+' | '-' | '@')) {
        format!("'{value}")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use csv::StringRecord;
    use serde_json::json;

    use super::*;

    fn first_record(value: &serde_json::Value, delimiter: u8) -> (StringRecord, StringRecord) {
        let mut output = Vec::new();
        write_csv(value, &mut output, delimiter).unwrap();

        let mut reader = csv::ReaderBuilder::new()
            .delimiter(delimiter)
            .from_reader(output.as_slice());
        let headers = reader.headers().unwrap().clone();
        let record = reader.records().next().unwrap().unwrap();
        (headers, record)
    }

    fn field<'a>(headers: &StringRecord, record: &'a StringRecord, name: &str) -> &'a str {
        let index = headers.iter().position(|header| header == name).unwrap();
        record.get(index).unwrap()
    }

    #[test]
    fn csv_neutralizes_dangerous_string_prefixes() {
        let value = json!([{
            "at": "\r@SUM(A1:A2)",
            "equals": "=1+1",
            "minus": "\t-2",
            "plus": "  +cmd",
        }]);

        let (headers, record) = first_record(&value, b',');

        assert_eq!(field(&headers, &record, "at"), "'\r@SUM(A1:A2)");
        assert_eq!(field(&headers, &record, "equals"), "'=1+1");
        assert_eq!(field(&headers, &record, "minus"), "'\t-2");
        assert_eq!(field(&headers, &record, "plus"), "'  +cmd");
    }

    #[test]
    fn tsv_neutralizes_dangerous_string_prefixes() {
        let value = json!([{
            "direct": "@SUM(A1:A2)",
            "whitespace": " \t=1+1",
        }]);

        let (headers, record) = first_record(&value, b'\t');

        assert_eq!(field(&headers, &record, "direct"), "'@SUM(A1:A2)");
        assert_eq!(field(&headers, &record, "whitespace"), "' \t=1+1");
    }

    #[test]
    fn csv_neutralizes_dangerous_headers() {
        let value = json!([{"=total": "=1+1", "normal": "ok"}]);

        let (headers, record) = first_record(&value, b',');
        let index = headers
            .iter()
            .position(|header| header == "'=total")
            .unwrap();

        assert_eq!(record.get(index).unwrap(), "'=1+1");
    }

    #[test]
    fn normal_numeric_and_json_values_keep_their_output_contract() {
        let value = json!([{
            "already_safe": "'=1+1",
            "json": {"enabled": true},
            "normal": "quarterly report",
            "number": -42,
        }]);

        let (headers, record) = first_record(&value, b',');

        assert_eq!(field(&headers, &record, "already_safe"), "'=1+1");
        assert_eq!(field(&headers, &record, "json"), r#"{"enabled":true}"#);
        assert_eq!(field(&headers, &record, "normal"), "quarterly report");
        assert_eq!(field(&headers, &record, "number"), "-42");
    }

    #[test]
    fn scalar_strings_are_neutralized_without_changing_numbers() {
        let mut formula = Vec::new();
        write_csv(&json!("=1+1"), &mut formula, b',').unwrap();
        assert_eq!(String::from_utf8(formula).unwrap(), "'=1+1\n");

        let mut number = Vec::new();
        write_csv(&json!(-42), &mut number, b',').unwrap();
        assert_eq!(String::from_utf8(number).unwrap(), "-42\n");
    }

    #[test]
    fn scalar_formulas_with_delimiters_remain_one_neutralized_cell() {
        for (delimiter, value) in [(b'\t', "\t=1+1"), (b',', "\n@SUM(A1:A2)")] {
            let mut output = Vec::new();
            write_csv(&json!(value), &mut output, delimiter).unwrap();

            let mut reader = csv::ReaderBuilder::new()
                .delimiter(delimiter)
                .has_headers(false)
                .from_reader(output.as_slice());
            let records: Vec<StringRecord> =
                reader.records().map(|record| record.unwrap()).collect();

            assert_eq!(records.len(), 1);
            assert_eq!(records[0].len(), 1);
            assert_eq!(records[0].get(0).unwrap(), format!("'{value}"));
        }
    }
}
