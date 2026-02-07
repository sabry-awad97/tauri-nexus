use std::{collections::HashSet, time::Duration};

use crate::{
    LogConfig, LogEntry, LogLevel, ProcedureType, RequestId, RequestMeta, TracingConfig,
    redact_value,
};

use proptest::prelude::*;
use serde_json::json;

#[test]
fn test_request_id_uniqueness() {
    let ids: Vec<RequestId> = (0..100).map(|_| RequestId::new()).collect();
    let unique: HashSet<_> = ids.iter().map(|id| id.to_string()).collect();
    assert_eq!(ids.len(), unique.len());
}

#[test]
fn test_request_id_from_string() {
    let id: RequestId = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
    assert_eq!(id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
}

#[test]
fn test_request_id_display() {
    let id: RequestId = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
    assert_eq!(format!("{}", id), "550e8400-e29b-41d4-a716-446655440000");
}

#[test]
fn test_request_id_short() {
    let id: RequestId = "12345678-1234-4567-89ab-123456789abc".parse().unwrap();
    assert_eq!(id.short(), "12345678");
}

#[test]
fn test_log_level_should_log() {
    assert!(LogLevel::Info.should_log(LogLevel::Error));
    assert!(LogLevel::Info.should_log(LogLevel::Warn));
    assert!(LogLevel::Info.should_log(LogLevel::Info));
    assert!(!LogLevel::Info.should_log(LogLevel::Debug));
    assert!(!LogLevel::Info.should_log(LogLevel::Trace));

    assert!(!LogLevel::Off.should_log(LogLevel::Error));
    assert!(LogLevel::Trace.should_log(LogLevel::Error));
}

#[test]
fn test_request_meta_creation() {
    let meta = RequestMeta::new("users.get", ProcedureType::Query);
    assert_eq!(meta.path, "users.get");
    assert_eq!(meta.procedure_type, ProcedureType::Query);
    assert!(meta.timestamp > 0);
    assert!(meta.client_id.is_none());
    assert!(meta.parent_request_id.is_none());
}

#[test]
fn test_request_meta_with_client_id() {
    let meta = RequestMeta::new("test", ProcedureType::Mutation).with_client_id("client-123");
    assert_eq!(meta.client_id, Some("client-123".to_string()));
}

#[test]
fn test_request_meta_with_parent_id() {
    let parent: RequestId = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
    let meta = RequestMeta::new("test", ProcedureType::Query).with_parent_request_id(parent);
    assert_eq!(meta.parent_request_id, Some(parent));
}

#[test]
fn test_log_entry_creation() {
    let meta = RequestMeta::new("test", ProcedureType::Query);
    let entry = LogEntry::new(meta);
    assert!(entry.success);
    assert!(entry.duration_ms.is_none());
    assert!(entry.error_code.is_none());
}

#[test]
fn test_log_entry_with_duration() {
    let meta = RequestMeta::new("test", ProcedureType::Query);
    let entry = LogEntry::new(meta).with_duration(Duration::from_millis(150));
    assert_eq!(entry.duration_ms, Some(150));
    assert!(entry.duration_us.is_some());
}

#[test]
fn test_log_entry_with_error() {
    let meta = RequestMeta::new("test", ProcedureType::Query);
    let entry = LogEntry::new(meta).with_error("NOT_FOUND", "User not found");
    assert!(!entry.success);
    assert_eq!(entry.error_code, Some("NOT_FOUND".to_string()));
    assert_eq!(entry.error_message, Some("User not found".to_string()));
}

#[test]
fn test_log_config_defaults() {
    let config = LogConfig::new();
    assert_eq!(config.level, LogLevel::Info);
    assert!(config.log_timing);
    assert!(!config.log_input);
    assert!(!config.log_output);
    assert!(config.log_success);
    assert!(config.log_errors);
    assert!(config.redacted_fields.contains("password"));
    assert!(config.redacted_fields.contains("token"));
}

#[test]
fn test_log_config_builder() {
    let config = LogConfig::new()
        .with_level(LogLevel::Debug)
        .with_timing(false)
        .with_input_logging(true)
        .with_output_logging(true)
        .redact_field("custom_secret")
        .exclude_path("health");

    assert_eq!(config.level, LogLevel::Debug);
    assert!(!config.log_timing);
    assert!(config.log_input);
    assert!(config.log_output);
    assert!(config.redacted_fields.contains("custom_secret"));
    assert!(config.excluded_paths.contains("health"));
}

#[test]
fn test_log_config_procedure_level() {
    let config = LogConfig::new()
        .with_level(LogLevel::Info)
        .with_procedure_level("debug.test", LogLevel::Debug);

    assert_eq!(config.get_level_for_path("normal"), LogLevel::Info);
    assert_eq!(config.get_level_for_path("debug.test"), LogLevel::Debug);
}

#[test]
fn test_log_config_clear_redacted_fields() {
    let config = LogConfig::new().clear_redacted_fields();
    assert!(config.redacted_fields.is_empty());
}

#[test]
fn test_log_config_should_log_path() {
    let config = LogConfig::new().exclude_path("health").exclude_path("ping");

    assert!(!config.should_log_path("health"));
    assert!(!config.should_log_path("ping"));
    assert!(config.should_log_path("users.get"));
}

#[test]
fn test_redact_value_simple() {
    let config = LogConfig::new();
    let input = json!({
        "username": "john",
        "password": "secret123"
    });

    let redacted = redact_value(&input, &config);
    assert_eq!(redacted["username"], "john");
    assert_eq!(redacted["password"], "[REDACTED]");
}

#[test]
fn test_redact_value_nested() {
    let config = LogConfig::new();
    let input = json!({
        "user": {
            "name": "john",
            "auth": {
                "password": "secret",
                "api_key": "key123"
            }
        }
    });

    let redacted = redact_value(&input, &config);
    assert_eq!(redacted["user"]["name"], "john");
    assert_eq!(redacted["user"]["auth"], "[REDACTED]");
}

#[test]
fn test_redact_value_nested_deep() {
    let config = LogConfig::new()
        .clear_redacted_fields()
        .redact_field("password")
        .redact_field("api_key");
    let input = json!({
        "user": {
            "name": "john",
            "settings": {
                "password": "secret",
                "api_key": "key123"
            }
        }
    });

    let redacted = redact_value(&input, &config);
    assert_eq!(redacted["user"]["name"], "john");
    assert_eq!(redacted["user"]["settings"]["password"], "[REDACTED]");
    assert_eq!(redacted["user"]["settings"]["api_key"], "[REDACTED]");
}

#[test]
fn test_redact_value_array() {
    let config = LogConfig::new();
    let input = json!([
        {"name": "john", "token": "abc"},
        {"name": "jane", "token": "xyz"}
    ]);

    let redacted = redact_value(&input, &config);
    assert_eq!(redacted[0]["name"], "john");
    assert_eq!(redacted[0]["token"], "[REDACTED]");
    assert_eq!(redacted[1]["name"], "jane");
    assert_eq!(redacted[1]["token"], "[REDACTED]");
}

#[test]
fn test_redact_value_case_insensitive() {
    let config = LogConfig::new();
    let input = json!({
        "PASSWORD": "secret1",
        "Password": "secret2",
        "user_password": "secret3"
    });

    let redacted = redact_value(&input, &config);
    assert_eq!(redacted["PASSWORD"], "[REDACTED]");
    assert_eq!(redacted["Password"], "[REDACTED]");
    assert_eq!(redacted["user_password"], "[REDACTED]");
}

#[test]
fn test_redact_value_custom_replacement() {
    let config = LogConfig::new().with_redaction_replacement("***");
    let input = json!({"password": "secret"});

    let redacted = redact_value(&input, &config);
    assert_eq!(redacted["password"], "***");
}

#[test]
fn test_redact_value_primitives_unchanged() {
    let config = LogConfig::new();

    assert_eq!(redact_value(&json!(42), &config), json!(42));
    assert_eq!(redact_value(&json!("hello"), &config), json!("hello"));
    assert_eq!(redact_value(&json!(true), &config), json!(true));
    assert_eq!(redact_value(&json!(null), &config), json!(null));
}

#[test]
fn test_tracing_config_defaults() {
    let config = TracingConfig::default();
    assert!(config.create_spans);
    assert!(!config.record_input);
    assert!(!config.record_output);
    assert_eq!(config.max_attribute_size, 1024);
    assert_eq!(config.service_name, "tauri-rpc");
}

#[test]
fn test_tracing_config_builder() {
    let config = TracingConfig::new()
        .with_spans(false)
        .with_input_recording(true)
        .with_output_recording(true)
        .with_max_attribute_size(2048)
        .with_service_name("my-app");

    assert!(!config.create_spans);
    assert!(config.record_input);
    assert!(config.record_output);
    assert_eq!(config.max_attribute_size, 2048);
    assert_eq!(config.service_name, "my-app");
}

proptest! {
    /// Property: All generated request IDs should be unique.
    #[test]
    fn prop_request_id_uniqueness(count in 10usize..100) {
        let ids: Vec<RequestId> = (0..count).map(|_| RequestId::new()).collect();
        let unique: std::collections::HashSet<_> = ids.iter().copied().collect();
        prop_assert_eq!(ids.len(), unique.len());
    }

    /// Property: Request IDs should be valid UUIDs.
    #[test]
    fn prop_request_id_is_valid_uuid(_seed in 0u64..1000) {
        let id = RequestId::new();
        // The UUID is already valid internally, just verify it can be converted to string
        let uuid_str = id.to_string();
        let parsed = uuid::Uuid::parse_str(&uuid_str);
        prop_assert!(parsed.is_ok(), "Request ID should be a valid UUID");
    }

    /// Property: Redaction should preserve non-sensitive fields.
    #[test]
    fn prop_redaction_preserves_non_sensitive(
        name in "[a-z]{3,10}",
        age in 1i32..100
    ) {
        let config = LogConfig::new();
        let input = json!({
            "name": name.clone(),
            "age": age
        });

        let redacted = redact_value(&input, &config);
        prop_assert_eq!(&redacted["name"], &json!(name));
        prop_assert_eq!(&redacted["age"], &json!(age));
    }

    /// Property: Redaction should always redact sensitive fields.
    #[test]
    fn prop_redaction_always_redacts_sensitive(
        password in "[a-zA-Z0-9]{5,20}",
        token in "[a-zA-Z0-9]{10,30}"
    ) {
        let config = LogConfig::new();
        let input = json!({
            "password": password,
            "token": token,
            "api_key": "some-key"
        });

        let redacted = redact_value(&input, &config);
        prop_assert_eq!(&redacted["password"], &json!("[REDACTED]"));
        prop_assert_eq!(&redacted["token"], &json!("[REDACTED]"));
        prop_assert_eq!(&redacted["api_key"], &json!("[REDACTED]"));
    }

    /// Property: Log level ordering should be consistent.
    #[test]
    fn prop_log_level_ordering_consistent(level_idx in 0usize..5) {
        let levels = [LogLevel::Trace, LogLevel::Debug, LogLevel::Info, LogLevel::Warn, LogLevel::Error];
        let level = levels[level_idx];

        // Error level should always be logged (except for Off)
        if level != LogLevel::Off {
            prop_assert!(level.should_log(LogLevel::Error));
        }

        // Off should never log anything
        prop_assert!(!LogLevel::Off.should_log(level));
    }

    /// Property: Excluded paths should never be logged.
    #[test]
    fn prop_excluded_paths_not_logged(
        path in "[a-z]{3,10}"
    ) {
        let config = LogConfig::new().exclude_path(path.clone());
        prop_assert!(!config.should_log_path(&path));
    }

    /// Property: Procedure-specific log levels should override global level.
    #[test]
    fn prop_procedure_level_overrides_global(
        path in "[a-z]{3,10}",
        global_level_idx in 0usize..5,
        proc_level_idx in 0usize..5
    ) {
        let levels = [LogLevel::Trace, LogLevel::Debug, LogLevel::Info, LogLevel::Warn, LogLevel::Error];
        let global_level = levels[global_level_idx];
        let proc_level = levels[proc_level_idx];

        let config = LogConfig::new()
            .with_level(global_level)
            .with_procedure_level(path.clone(), proc_level);

        prop_assert_eq!(config.get_level_for_path(&path), proc_level);
        prop_assert_eq!(config.get_level_for_path("other"), global_level);
    }
}
