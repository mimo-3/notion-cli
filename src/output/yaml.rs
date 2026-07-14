use std::io::Write;

use crate::error::CliError;

pub fn write_yaml(value: &serde_json::Value, writer: &mut dyn Write) -> Result<(), CliError> {
    let output = serde_yaml::to_string(value)
        .map_err(|e| CliError::Config(format!("YAML serialization error: {e}")))?;
    write!(writer, "{output}")?;
    Ok(())
}
