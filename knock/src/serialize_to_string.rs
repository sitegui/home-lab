use anyhow::bail;
use serde::Serialize;
use serde_json::Value;

/// Serialize a value into a string. Useful to serialize a simple enum without associated data
pub fn serialize_to_string<T: Serialize>(value: T) -> anyhow::Result<String> {
    match serde_json::to_value(value)? {
        Value::String(s) => Ok(s),
        _ => bail!("could not serialize value as a string"),
    }
}
