//! Stable error codes and exit codes.

use serde::{Deserialize, Serialize};

/// Error detail carried inside the JSON stdout envelope.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ErrorEnvelope {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Stable error codes. Each code maps to one exit code and one class of problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// JSON input could not be parsed.
    InvalidJson,
    /// Input failed schema validation.
    SchemaValidationFailed,
    /// A stat block contains an unmodeled ability that batch mode cannot resolve.
    UnmodeledAbilityRefused,
    /// Encounter state file has been tampered with or corrupted.
    StateTampered,
    /// A command asked for an encounter or resource that does not exist.
    NotFound,
    /// A command line argument is malformed or missing.
    InvalidArgument,
    /// An override path or value does not fit the state schema.
    InvalidOverride,
    /// The rules forbid the command right now.
    IllegalAction,
    /// A pending prompt must be answered before this command.
    PendingPrompt,
    /// The encounter has ended.
    EncounterEnded,
    /// The state file was written by an incompatible engine version.
    EngineVersionMismatch,
    /// A `--rule` slug names no house-rule file.
    UnknownHouseRule,
    /// An internal invariant failed.
    Internal,
}

impl ErrorCode {
    /// Stable machine-readable code string.
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::InvalidJson => "invalid_json",
            ErrorCode::SchemaValidationFailed => "schema_validation_failed",
            ErrorCode::UnmodeledAbilityRefused => "unmodeled_ability_refused",
            ErrorCode::StateTampered => "state_tampered",
            ErrorCode::NotFound => "not_found",
            ErrorCode::InvalidArgument => "invalid_argument",
            ErrorCode::InvalidOverride => "invalid_override",
            ErrorCode::IllegalAction => "illegal_action",
            ErrorCode::PendingPrompt => "pending_prompt",
            ErrorCode::EncounterEnded => "encounter_ended",
            ErrorCode::EngineVersionMismatch => "engine_version_mismatch",
            ErrorCode::UnknownHouseRule => "unknown_house_rule",
            ErrorCode::Internal => "internal",
        }
    }

    /// Process exit code.
    pub fn exit_code(&self) -> i32 {
        match self {
            ErrorCode::InvalidJson
            | ErrorCode::SchemaValidationFailed
            | ErrorCode::NotFound
            | ErrorCode::InvalidArgument
            | ErrorCode::InvalidOverride
            | ErrorCode::UnknownHouseRule => 1,
            ErrorCode::UnmodeledAbilityRefused
            | ErrorCode::IllegalAction
            | ErrorCode::PendingPrompt
            | ErrorCode::EncounterEnded => 2,
            ErrorCode::StateTampered | ErrorCode::EngineVersionMismatch | ErrorCode::Internal => 3,
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::error::Error for ErrorCode {}

/// An error tagged with a stable `ErrorCode` and optional structured details.
#[derive(Debug, Clone)]
pub struct CodedError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl CodedError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Render as a JSON stdout envelope.
    pub fn envelope<T: serde::Serialize>(&self) -> crate::Envelope<T> {
        crate::Envelope::error_with_details(
            self.code.as_str(),
            self.message.clone(),
            self.details.clone(),
        )
    }
}

impl From<anyhow::Error> for CodedError {
    fn from(err: anyhow::Error) -> Self {
        CodedError::new(ErrorCode::Internal, format!("{err:#}"))
    }
}
