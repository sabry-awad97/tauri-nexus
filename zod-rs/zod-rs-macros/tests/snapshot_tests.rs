//! Snapshot tests for generated Zod schemas.
//!
//! These tests use insta to capture and verify the exact output of generated schemas.
//! Run `cargo insta review` to review and accept snapshot changes.

use zod_rs::ZodSchema;

// =============================================================================
// Basic Struct Snapshots
// =============================================================================

#[test]
fn snapshot_basic_struct() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct User {
        name: String,
        age: u32,
        active: bool,
    }

    insta::assert_snapshot!("basic_struct_schema", User::zod_schema());
    insta::assert_snapshot!("basic_struct_declaration", User::ts_declaration());
}

#[test]
fn snapshot_unit_struct() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct Empty;

    insta::assert_snapshot!("unit_struct_schema", Empty::zod_schema());
}

#[test]
fn snapshot_tuple_struct() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct Point(f64, f64, f64);

    insta::assert_snapshot!("tuple_struct_schema", Point::zod_schema());
}

// =============================================================================
// Struct with Container Attributes
// =============================================================================

#[test]
fn snapshot_struct_with_rename_all() {
    #[derive(ZodSchema)]
    #[zod(rename_all = "camelCase")]
    #[allow(dead_code)]
    struct UserProfile {
        first_name: String,
        last_name: String,
        email_address: String,
        phone_number: Option<String>,
    }

    insta::assert_snapshot!("struct_rename_all_camel", UserProfile::zod_schema());
}

#[test]
fn snapshot_struct_with_strict() {
    #[derive(ZodSchema)]
    #[zod(strict)]
    #[allow(dead_code)]
    struct StrictConfig {
        host: String,
        port: u16,
    }

    insta::assert_snapshot!("struct_strict", StrictConfig::zod_schema());
}

#[test]
fn snapshot_struct_with_description() {
    /// A configuration object for the application
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct AppConfig {
        /// The application name
        name: String,
        /// The version number
        version: String,
    }

    insta::assert_snapshot!("struct_with_docs", AppConfig::zod_schema());
}

// =============================================================================
// Field Attributes Snapshots
// =============================================================================

#[test]
fn snapshot_field_with_validations() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct Registration {
        #[zod(min_length = 3, max_length = 50)]
        username: String,
        #[zod(email)]
        email: String,
        #[zod(min_length = 8)]
        password: String,
        #[zod(min = 18.0, max = 120.0)]
        age: u8,
    }

    insta::assert_snapshot!("field_validations", Registration::zod_schema());
}

#[test]
fn snapshot_field_with_optional_nullable() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct Profile {
        name: String,
        #[zod(optional)]
        bio: String,
        #[zod(nullable)]
        avatar_url: String,
        nickname: Option<String>,
    }

    insta::assert_snapshot!("field_optional_nullable", Profile::zod_schema());
}

#[test]
fn snapshot_field_with_default() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct Settings {
        #[zod(default = "\"en\"")]
        language: String,
        #[zod(default = "true")]
        notifications: bool,
        #[zod(default = "10")]
        page_size: u32,
    }

    insta::assert_snapshot!("field_defaults", Settings::zod_schema());
}

#[test]
fn snapshot_field_with_rename() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct ApiResponse {
        #[zod(rename = "statusCode")]
        status_code: u16,
        #[zod(rename = "responseBody")]
        body: String,
    }

    insta::assert_snapshot!("field_rename", ApiResponse::zod_schema());
}

// =============================================================================
// Enum Snapshots
// =============================================================================

#[test]
fn snapshot_unit_enum() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    enum Status {
        Pending,
        Active,
        Suspended,
        Deleted,
    }

    insta::assert_snapshot!("unit_enum", Status::zod_schema());
    insta::assert_snapshot!("unit_enum_declaration", Status::ts_declaration());
}

#[test]
fn snapshot_unit_enum_with_rename_all() {
    #[derive(ZodSchema)]
    #[zod(rename_all = "SCREAMING_SNAKE_CASE")]
    #[allow(dead_code)]
    enum Permission {
        ReadOnly,
        ReadWrite,
        Admin,
    }

    insta::assert_snapshot!("unit_enum_screaming_snake", Permission::zod_schema());
}

#[test]
fn snapshot_enum_internal_tag() {
    #[derive(ZodSchema)]
    #[zod(tag = "type")]
    #[allow(dead_code)]
    enum Message {
        Text { content: String },
        Image { url: String, alt: Option<String> },
        Video { url: String, duration: u32 },
    }

    insta::assert_snapshot!("enum_internal_tag", Message::zod_schema());
}

#[test]
fn snapshot_enum_adjacent_tag() {
    #[derive(ZodSchema)]
    #[zod(tag = "event", content = "payload")]
    #[allow(dead_code)]
    enum Event {
        UserCreated { user_id: String, email: String },
        UserDeleted { user_id: String },
        OrderPlaced { order_id: String, total: f64 },
    }

    insta::assert_snapshot!("enum_adjacent_tag", Event::zod_schema());
}

// =============================================================================
// Complex Type Snapshots
// =============================================================================

#[test]
fn snapshot_nested_types() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct Address {
        street: String,
        city: String,
        country: String,
    }

    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct Company {
        name: String,
        address: Address,
        employees: Vec<String>,
    }

    insta::assert_snapshot!("nested_struct", Company::zod_schema());
}

#[test]
fn snapshot_collection_types() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct DataContainer {
        items: Vec<String>,
        tags: Vec<String>,
        counts: Vec<u32>,
    }

    insta::assert_snapshot!("collection_types", DataContainer::zod_schema());
}

#[test]
fn snapshot_all_primitive_types() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct AllPrimitives {
        string_field: String,
        bool_field: bool,
        i8_field: i8,
        i16_field: i16,
        i32_field: i32,
        i64_field: i64,
        u8_field: u8,
        u16_field: u16,
        u32_field: u32,
        u64_field: u64,
        f32_field: f32,
        f64_field: f64,
    }

    insta::assert_snapshot!("all_primitives", AllPrimitives::zod_schema());
}

// =============================================================================
// Validation Combinations Snapshots
// =============================================================================

#[test]
fn snapshot_string_validations() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct StringValidations {
        #[zod(email)]
        email: String,
        #[zod(url)]
        website: String,
        #[zod(uuid)]
        id: String,
        #[zod(regex = "^[A-Z]{2,3}$")]
        country_code: String,
        #[zod(min_length = 1, max_length = 255)]
        description: String,
    }

    insta::assert_snapshot!("string_validations", StringValidations::zod_schema());
}

#[test]
fn snapshot_number_validations() {
    #[derive(ZodSchema)]
    #[allow(dead_code)]
    struct NumberValidations {
        #[zod(positive)]
        positive_num: i32,
        #[zod(int)]
        integer: f64,
        #[zod(min = 0.0, max = 100.0)]
        percentage: f64,
        #[zod(min = 1.0)]
        at_least_one: u32,
    }

    insta::assert_snapshot!("number_validations", NumberValidations::zod_schema());
}
