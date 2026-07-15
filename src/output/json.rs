use std::io::Write;

use crate::error::CliError;

pub fn write_json(value: &serde_json::Value, writer: &mut dyn Write) -> Result<(), CliError> {
    let output = serde_json::to_string_pretty(value)?;
    writeln!(writer, "{output}")?;
    Ok(())
}
