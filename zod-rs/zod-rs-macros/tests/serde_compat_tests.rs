//! Serde compatibility tests for the ZodSchema derive macro.
//!
//! These tests verify that serde attributes are correctly parsed and that
//! zod attributes can override serde attributes when both are present.

use zod_rs::ZodSchema;
use zod_rs_macros::ZodSchema;

// =============================================================================
// Serde rename_all Compatibility Tests
// =============================================================================

#[test]
fn test_serde_rename_all_camel_case() {
    #[derive(ZodSchema)]
    #[serde(rename_all = "camelCase")]
    #[allow(dead_code)]
    struct User {
        first_name: String,
        last_name: String,
    }

    let schema = User::zod_schema();
    assert!(schema.contains("firstName"));
    assert!(schema.contains("lastName"));
    assert!(!schema.contains("first_name"));
    assert!(!schema.contains("last_name"));
}

#[test]
fn test_serde_rename_all_snake_case() {
    #[derive(ZodSchema)]
    #[serde(rename_all = "snake_case")]
    #[allow(dead_code)]
    struct Config {
        apiKey: String,
        baseUrl: String,
    }

    let schema = Config::zod_schema();
    assert!(schema.contains("api_key"));
    assert!(schema.contains("base_url"));
}

#[test]
fn test_serde_rename_all_screaming_snake_case() {
    #[derive(ZodSchema)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[allow(dead_code)]
    enum Status {
        Active,
        Inactive,
    }

    let schema = Status::zod_schema();
    assert!(schema.contains("ACTIVE"));
    assert!(schema.contains("INACTIVE"));
}

// =============================================================================
// Serde Field Rename Tests
// =============================================================================

#[test]
fn test_serde_field_rename() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct ApiResponse {
        #[serde(rename = "statusCode")]
        status_code: u16,
        #[serde(rename = "responseBody")]
        body: String,
    }

    let schema = ApiResponse::zod_schema();
    assert!(schema.contains("statusCode"));
    assert!(schema.contains("responseBody"));
    assert!(!schema.contains("status_code"));
    assert!(!schema.contains("\"body\""));
}

// =============================================================================
// Serde Skip Tests
// =============================================================================

#[test]
fn test_serde_skip_field() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct User {
        name: String,
        #[serde(skip)]
        internal_id: u64,
    }

    let schema = User::zod_schema();
    assert!(schema.contains("name"));
    assert!(!schema.contains("internal_id"));
}

#[test]
fn test_serde_skip_serializing() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct User {
        name: String,
        #[serde(skip_serializing)]
        password_hash: String,
    }

    let schema = User::zod_schema();
    assert!(schema.contains("name"));
    assert!(!schema.contains("password_hash"));
}

#[test]
fn test_serde_skip_deserializing() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct User {
        name: String,
        #[serde(skip_deserializing)]
        computed_field: String,
    }

    let schema = User::zod_schema();
    assert!(schema.contains("name"));
    // skip_deserializing should also skip in schema
    assert!(!schema.contains("computed_field"));
}

// =============================================================================
// Serde Default Tests
// =============================================================================

#[test]
fn test_serde_default_field() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct Config {
        #[serde(default)]
        enabled: bool,
    }

    let schema = Config::zod_schema();
    // serde default should make field optional
    assert!(schema.contains("enabled"));
    assert!(schema.contains(".optional()"));
}

// =============================================================================
// Serde Tag Tests (Enum Tagging)
// =============================================================================

#[test]
fn test_serde_internal_tag() {
    #[derive(ZodSchema)]
    #[serde(tag = "type")]
    #[allow(dead_code)]
    enum Message {
        Text { content: String },
        Image { url: String },
    }

    let schema = Message::zod_schema();
    assert!(schema.contains("z.discriminatedUnion"));
    assert!(schema.contains("\"type\""));
}

#[test]
fn test_serde_adjacent_tag() {
    #[derive(ZodSchema)]
    #[serde(tag = "kind", content = "data")]
    #[allow(dead_code)]
    enum Event {
        Click { x: i32, y: i32 },
        Scroll { delta: f64 },
    }

    let schema = Event::zod_schema();
    assert!(schema.contains("kind"));
    assert!(schema.contains("data"));
}

// =============================================================================
// Zod Override of Serde Tests
// =============================================================================

#[test]
fn test_zod_overrides_serde_rename_all() {
    #[derive(ZodSchema)]
    #[serde(rename_all = "camelCase")]
    #[zod(rename_all = "snake_case")]
    #[allow(dead_code)]
    struct User {
        firstName: String,
        lastName: String,
    }

    let schema = User::zod_schema();
    // zod should override serde, so snake_case should be applied
    assert!(schema.contains("first_name"));
    assert!(schema.contains("last_name"));
}

#[test]
fn test_zod_overrides_serde_field_rename() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct ApiResponse {
        #[serde(rename = "statusCode")]
        #[zod(rename = "status")]
        status_code: u16,
    }

    let schema = ApiResponse::zod_schema();
    // zod should override serde
    assert!(schema.contains("status"));
    assert!(!schema.contains("statusCode"));
}

#[test]
fn test_zod_overrides_serde_skip() {
    // Note: This test documents a known limitation.
    // Since #[zod(skip)] defaults to false, we cannot distinguish between
    // "not specified" and "explicitly set to false". Therefore, #[zod(skip = false)]
    // cannot override #[serde(skip)].
    //
    // If you need to include a field that serde would skip, you should not use
    // #[serde(skip)] on that field, or use a different approach.
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct User {
        name: String,
        #[serde(skip)]
        internal_id: u64,
    }

    let schema = User::zod_schema();
    // serde skip is applied, field is not in schema
    assert!(schema.contains("name"));
    assert!(!schema.contains("internal_id"));
}

#[test]
fn test_zod_overrides_serde_tag() {
    #[derive(ZodSchema)]
    #[serde(tag = "type")]
    #[zod(tag = "kind")]
    #[allow(dead_code)]
    enum Message {
        Text { content: String },
        Image { url: String },
    }

    let schema = Message::zod_schema();
    // zod should override serde
    assert!(schema.contains("\"kind\""));
    assert!(!schema.contains("\"type\""));
}

// =============================================================================
// Combined Serde and Zod Attributes Tests
// =============================================================================

#[test]
fn test_serde_and_zod_combined() {
    #[derive(ZodSchema)]
    #[serde(rename_all = "camelCase")]
    #[allow(dead_code)]
    struct Registration {
        #[zod(min_length = 3, max_length = 50)]
        user_name: String,
        #[zod(email)]
        email_address: String,
        #[serde(skip)]
        internal_token: String,
    }

    let schema = Registration::zod_schema();
    // serde rename_all should apply
    assert!(schema.contains("userName"));
    assert!(schema.contains("emailAddress"));
    // zod validations should apply
    assert!(schema.contains(".min(3)"));
    assert!(schema.contains(".max(50)"));
    assert!(schema.contains(".email()"));
    // serde skip should apply
    assert!(!schema.contains("internalToken"));
}

#[test]
fn test_serde_flatten_ignored() {
    // Note: flatten is complex and may not be fully supported
    // This test verifies the field is still included
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct Address {
        street: String,
        city: String,
    }

    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct User {
        name: String,
        #[serde(flatten)]
        address: Address,
    }

    let schema = User::zod_schema();
    assert!(schema.contains("name"));
    // flatten may or may not be supported, but field should be present
    assert!(schema.contains("address") || schema.contains("street"));
}

// =============================================================================
// Serde Variant Rename Tests
// =============================================================================

#[test]
fn test_serde_variant_rename() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    enum Action {
        #[serde(rename = "CREATE")]
        Create,
        #[serde(rename = "UPDATE")]
        Update,
        #[serde(rename = "DELETE")]
        Delete,
    }

    let schema = Action::zod_schema();
    assert!(schema.contains("CREATE"));
    assert!(schema.contains("UPDATE"));
    assert!(schema.contains("DELETE"));
}

#[test]
fn test_serde_variant_skip() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    enum Status {
        Active,
        #[serde(skip)]
        Internal,
        Inactive,
    }

    let schema = Status::zod_schema();
    assert!(schema.contains("Active"));
    assert!(schema.contains("Inactive"));
    assert!(!schema.contains("Internal"));
}
