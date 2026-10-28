//! Thin `bench` CLI binary.
//!
//! Every command starts, does one thing, prints a JSON envelope to stdout, and
//! exits. The binary delegates all rules/state work to `bench-core`.

use std::process;

use bench_core::{
    error::{CodedError, ErrorCode},
    schema::{schema_json, validate_value},
    Envelope, ENGINE_VERSION,
};
use clap::{Parser, Subcommand};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "bench")]
#[command(version = ENGINE_VERSION)]
#[command(about = "One-shot CLI for homebrew-bench combat testing")]
#[command(disable_help_subcommand = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show help. Use `--json` for the machine-readable command catalog.
    Help {
        /// Output machine-readable command catalog.
        #[arg(long)]
        json: bool,
    },
    /// Print JSON Schema or validate a monster stat block file.
    Schema {
        #[command(subcommand)]
        action: SchemaAction,
    },
}

#[derive(Subcommand)]
enum SchemaAction {
    /// Print the JSON Schema for the monster stat block.
    Monster,
    /// Validate a stat block JSON file.
    Validate {
        /// Path to the stat block JSON file.
        path: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Help { json }) => {
            if json {
                print_and_exit_ok(Envelope::ok(command_catalog()));
            }
            Err(CodedError::new(
                ErrorCode::InvalidJson,
                "use --help for usage, or help --json for the command catalog",
            ))
        }
        Some(Commands::Schema { action }) => run_schema(action),
        None => Err(CodedError::new(ErrorCode::InvalidJson, "no command given; use --help or help --json")),
    };

    match result {
        Ok(output) => {
            println!("{}", output);
            process::exit(0);
        }
        Err(err) => {
            let envelope: Envelope<serde_json::Value> = err.envelope();
            println!("{}", serde_json::to_string_pretty(&envelope).unwrap());
            process::exit(err.code.exit_code());
        }
    }
}

fn run_schema(action: SchemaAction) -> Result<String, CodedError> {
    let data = match action {
        SchemaAction::Monster => {
            let schema = schema_json::<bench_core::stat_block::StatBlock>()
                .map_err(|e| CodedError::new(ErrorCode::Internal, format!("failed to generate monster schema: {e:#}")))?;
            serde_json::from_str(&schema)
                .map_err(|e| CodedError::new(ErrorCode::Internal, format!("generated schema is not valid json: {e}")))?
        }
        SchemaAction::Validate { path } => {
            let text = std::fs::read_to_string(&path)
                .map_err(|e| CodedError::new(ErrorCode::NotFound, format!("failed to read {path}: {e}")))?;
            let value: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| CodedError::new(ErrorCode::InvalidJson, format!("invalid json in {path}: {e}")))?;
            let errors = validate_value::<bench_core::stat_block::StatBlock>(&value)
                .map_err(|e| CodedError::new(ErrorCode::Internal, format!("failed to validate {path}: {e}")))?;
            if errors.is_empty() {
                serde_json::json!({"valid": true, "path": path})
            } else {
                let details = serde_json::to_value(errors)
                    .map_err(|e| CodedError::new(ErrorCode::Internal, format!("failed to serialize errors: {e}")))?;
                return Err(CodedError::new(
                    ErrorCode::SchemaValidationFailed,
                    format!("{path} failed schema validation"),
                )
                .with_details(details));
            }
        }
    };
    serde_json::to_string_pretty(&Envelope::ok(data))
        .map_err(|e| CodedError::new(ErrorCode::Internal, format!("failed to serialize envelope: {e}")))
}

fn command_catalog() -> serde_json::Value {
    serde_json::json!({
        "commands": [
            {
                "name": "help",
                "args": ["--json"],
                "description": "Print machine-readable command catalog."
            },
            {
                "name": "schema",
                "subcommands": [
                    {
                        "name": "monster",
                        "description": "Print the JSON Schema for the monster stat block."
                    },
                    {
                        "name": "validate",
                        "args": ["path"],
                        "description": "Validate a stat block JSON file."
                    }
                ]
            }
        ]
    })
}

fn print_and_exit_ok<T: Serialize>(data: T) {
    println!("{}", serde_json::to_string_pretty(&data).unwrap());
    process::exit(0);
}
