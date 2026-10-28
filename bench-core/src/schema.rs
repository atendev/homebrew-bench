//! JSON Schema generation and validation helpers.

use schemars::{schema::RootSchema, JsonSchema};
use serde::Serialize;
use std::path::Path;

/// Generate a JSON Schema (as a `RootSchema`) for any `JsonSchema` type.
pub fn schema_for<T: JsonSchema>() -> RootSchema {
    schemars::schema_for!(T)
}

/// Serialize a schema to a pretty JSON string.
pub fn schema_json<T: JsonSchema>() -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(&schema_for::<T>())?)
}

/// Validate a JSON file against the schema for type `T`.
///
/// Returns a list of dotted field-path errors. The current implementation uses
/// deserialization as a first-line validator; later tickets can add a
/// JSON-Schema validator for richer, schema-only checks.
pub fn validate_file<T>(path: impl AsRef<Path>) -> anyhow::Result<Vec<FieldError>>
where
    T: JsonSchema + for<'de> serde::Deserialize<'de>,
{
    let path = path.as_ref();
    let text = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("invalid json in {}: {e}", path.display()))?;
    validate_value::<T>(&value)
}

/// Validate a JSON value against the schema for type `T`.
pub fn validate_value<T>(value: &serde_json::Value) -> anyhow::Result<Vec<FieldError>>
where
    T: JsonSchema + for<'de> serde::Deserialize<'de>,
{
    let text = serde_json::to_string(value)?;
    let mut deserializer = serde_json::Deserializer::from_str(&text);
    match serde_path_to_error::deserialize::<_, T>(&mut deserializer) {
        Ok(_) => Ok(Vec::new()),
        Err(e) => Ok(vec![FieldError {
            path: dotted_path(e.path()),
            message: e.inner().to_string(),
        }]),
    }
}

/// A single validation error with a dotted field path.
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct FieldError {
    pub path: String,
    pub message: String,
}

fn dotted_path(path: &serde_path_to_error::Path) -> String {
    use serde_path_to_error::Segment;
    let parts: Vec<String> = path
        .iter()
        .map(|segment| match segment {
            Segment::Map { key } => key.to_string(),
            Segment::Seq { index } => index.to_string(),
            Segment::Enum { variant } => variant.to_string(),
            Segment::Unknown => "?".into(),
        })
        .collect();
    if parts.is_empty() {
        "(root)".into()
    } else {
        parts.join(".")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    struct Sample {
        name: String,
        count: u32,
    }

    #[test]
    fn schema_is_valid_json() {
        let json = schema_json::<Sample>().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("title").is_some() || parsed.get("$schema").is_some());
    }

    #[test]
    fn validate_value_accepts_valid() {
        let value = serde_json::json!({ "name": "test", "count": 3 });
        let errors = validate_value::<Sample>(&value).unwrap();
        assert!(errors.is_empty());
    }

    #[test]
    fn validate_value_reports_missing_field() {
        let value = serde_json::json!({ "count": 3 });
        let errors = validate_value::<Sample>(&value).unwrap();
        assert!(!errors.is_empty());
    }
}
