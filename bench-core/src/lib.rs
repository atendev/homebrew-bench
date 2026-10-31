//! Core library for homebrew-bench.
//!
//! Holds the canonical data models, effect vocabulary, encounter state,
//! ChaCha8 randomness wrapper, config/path layout, and schema helpers.

#![allow(dead_code)]

pub mod config;
pub mod digest;
pub mod effect;
pub mod engine;
pub mod encounter;
pub mod error;
pub mod live;
pub mod log;
pub mod override_path;
pub mod party;
pub mod paths;
pub mod rng;
pub mod schema;
pub mod spell;
pub mod store;
pub mod stat_block;

/// Engine version written into encounter state files.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Schema version emitted in JSON stdout envelopes.
pub const SCHEMA_VERSION: &str = "1";

/// JSON stdout envelope shared by every `bench` command.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Envelope<T: serde::Serialize> {
    pub schema_version: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<error::ErrorEnvelope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T: serde::Serialize> Envelope<T> {
    pub fn ok(data: T) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.into(),
            status: "ok".into(),
            error: None,
            data: Some(data),
        }
    }

    pub fn error(code: &str, message: impl Into<String>) -> Self {
        Self::error_with_details(code, message, None)
    }

    pub fn error_with_details(
        code: &str,
        message: impl Into<String>,
        details: Option<serde_json::Value>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.into(),
            status: "error".into(),
            error: Some(error::ErrorEnvelope {
                code: code.into(),
                message: message.into(),
                details,
            }),
            data: None,
        }
    }
}

/// Serialize an envelope to a pretty JSON string.
pub fn to_stdout_json<T: serde::Serialize>(envelope: &Envelope<T>) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(envelope)?)
}
