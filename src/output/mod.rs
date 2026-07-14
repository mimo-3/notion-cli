pub mod csv_out;
pub mod json;
pub mod plain;
pub mod table;
pub mod yaml;

use std::fmt;
use std::io::Write;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::CliError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Json,
    Yaml,
    Csv,
    Tsv,
    Plain,
    IdOnly,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Yaml => write!(f, "yaml"),
            OutputFormat::Csv => write!(f, "csv"),
            OutputFormat::Tsv => write!(f, "tsv"),
            OutputFormat::Plain => write!(f, "plain"),
            OutputFormat::IdOnly => write!(f, "id-only"),
        }
    }
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "yaml" | "yml" => Ok(OutputFormat::Yaml),
            "csv" => Ok(OutputFormat::Csv),
            "tsv" => Ok(OutputFormat::Tsv),
            "plain" | "text" => Ok(OutputFormat::Plain),
            "id-only" | "id" | "ids" => Ok(OutputFormat::IdOnly),
            _ => Err(format!("Unknown output format: {s}")),
        }
    }
}

/// Trait for types that can render themselves in plain-text format.
pub trait Displayable {
    fn display_plain(&self, writer: &mut dyn Write) -> Result<(), CliError>;
    fn display_id(&self, writer: &mut dyn Write) -> Result<(), CliError>;
}

/// Format any serializable value to the given output format.
pub fn format_value(
    value: &serde_json::Value,
    format: OutputFormat,
    writer: &mut dyn Write,
) -> Result<(), CliError> {
    match format {
        OutputFormat::Json => json::write_json(value, writer),
        OutputFormat::Yaml => yaml::write_yaml(value, writer),
        OutputFormat::Csv => csv_out::write_csv(value, writer, b','),
        OutputFormat::Tsv => csv_out::write_csv(value, writer, b'\t'),
        OutputFormat::Plain => plain::write_plain(value, writer),
        OutputFormat::IdOnly => {
            if let Some(id) = value.get("id").and_then(|v| v.as_str()) {
                writeln!(writer, "{id}")?;
            } else if let Some(arr) = value.as_array() {
                for item in arr {
                    if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                        writeln!(writer, "{id}")?;
                    }
                }
            }
            Ok(())
        }
    }
}
