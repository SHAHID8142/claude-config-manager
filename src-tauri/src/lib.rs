//! Typed command-contract primitives shared by all Tauri commands.
//!
//! Contract rules (docs/architecture/MODULE-BOUNDARIES.md):
//! - Commands return `Result<T, CommandError>` so the UI always receives a
//!   structured, redaction-safe error instead of a raw panic string.
//! - `CommandError` messages are plain, user-mappable strings — handlers must
//!   pre-redact any secret-shaped content before constructing one (T23 adds
//!   the shared redaction guard).

use serde::{Deserialize, Serialize};

/// Structured error payload crossing the UI boundary (T11 contract).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum CommandError {
    /// Caller-supplied input failed validation (field name included).
    InvalidInput(String),
    /// Filesystem or OS interaction failed; message carries no secret material.
    Io(String),
    /// Capability not available in the current build/environment.
    NotAvailable(String),
}

impl CommandError {
    pub fn invalid_input(message: impl Into<String>) -> Self {
        CommandError::InvalidInput(message.into())
    }

    pub fn io(message: impl Into<String>) -> Self {
        CommandError::Io(message.into())
    }

    pub fn not_available(message: impl Into<String>) -> Self {
        CommandError::NotAvailable(message.into())
    }
}

#[tauri::command]
fn greet(name: &str) -> Result<String, CommandError> {
    if name.len() > 256 {
        // Negative guard: oversized names rejected with a typed error.
        return Err(CommandError::invalid_input("name exceeds 256 characters"));
    }
    Ok(format!("Hello, {}! You've been greeted from Rust!", name))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod contract_tests {
    use super::*;

    #[test]
    fn command_error_serializes_with_kind_and_message() {
        let err = CommandError::invalid_input("endpoint must be https");
        let json = serde_json::to_value(&err).expect("serialize");
        assert_eq!(json["kind"], "invalid_input");
        assert_eq!(json["message"], "endpoint must be https");
    }

    #[test]
    fn command_error_kind_names_are_snake_case() {
        for (err, expected) in [
            (CommandError::io("read failed"), "io"),
            (CommandError::not_available("x"), "not_available"),
            (CommandError::invalid_input("y"), "invalid_input"),
        ] {
            let json = serde_json::to_value(&err).expect("serialize");
            assert_eq!(json["kind"], expected);
        }
    }

    #[test]
    fn greet_rejects_oversized_name_with_typed_error() {
        let long = "a".repeat(257);
        assert!(matches!(greet(&long), Err(CommandError::InvalidInput(_))));
    }

    #[test]
    fn greet_accepts_normal_name() {
        let out = greet("fixture-user").expect("ok");
        assert!(out.contains("fixture-user"));
    }
}
