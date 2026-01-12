//! Integration tests for the ZodSchema derive macro.
//!
//! These tests verify that the macro correctly generates ZodSchema implementations
//! for various struct and enum definitions.

use zod_rs::ZodSchema;
use zod_rs_macros::ZodSchema;

// =============================================================================
// Basic Struct Tests
// =============================================================================

#[test]
fn test_basic_struct_derive() {
    #[derive(ZodSchema)]
    struct User {
        name: String,
        age: u32,
    }

    let schema = User::zod_schema();
    assert!(schema.contains("z.object"));
    assert!(schema.contains("name"));
    assert!(schema.contains("age"));
    assert!(schema.contains("z.string()"));
    assert!(schema.contains("z.number()"));
}

#[test]
fn test_unit_struct_derive() {
    #[derive(ZodSchema)]
    struct Empty;

    let schema = Empty::zod_schema();
    assert!(schema.contains("z.object"));
}

#[test]
fn test_tuple_struct_derive() {
    #[derive(ZodSchema)]
    struct Point(f64, f64);

    let schema = Point::zod_schema();
    assert!(schema.contains("z.tuple"));
    assert!(schema.contains("z.number()"));
}

// =============================================================================
// Struct with Attributes Tests
// =============================================================================

#[test]
fn test_struct_with_rename() {
    #[derive(ZodSchema)]
    #[zod(rename = "UserDTO")]
    struct User {
        name: String,
    }

    assert_eq!(User::schema_name(), "UserDTOSchema");
    assert_eq!(User::ts_type_name(), "UserDTO");
}

#[test]
fn test_struct_with_rename_all_camel_case() {
    #[derive(ZodSchema)]
    #[zod(rename_all = "camelCase")]
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
fn test_struct_with_strict() {
    #[derive(ZodSchema)]
    #[zod(strict)]
    struct StrictUser {
        name: String,
    }

    let schema = StrictUser::zod_schema();
    assert!(schema.contains(".strict()"));
}

#[test]
fn test_struct_with_description() {
    #[derive(ZodSchema)]
    #[zod(description = "A user in the system")]
    struct User {
        name: String,
    }

    let metadata = User::metadata();
    assert_eq!(
        metadata.description,
        Some("A user in the system".to_string())
    );
}

#[test]
fn test_struct_with_deprecated() {
    #[derive(ZodSchema)]
    #[zod(deprecated)]
    struct OldUser {
        name: String,
    }

    let metadata = OldUser::metadata();
    assert!(metadata.deprecated);
}

// =============================================================================
// Field Attribute Tests
// =============================================================================

#[test]
fn test_field_with_rename() {
    #[derive(ZodSchema)]
    struct User {
        #[zod(rename = "userName")]
        name: String,
    }

    let schema = User::zod_schema();
    assert!(schema.contains("userName"));
    assert!(!schema.contains("\"name\""));
}

#[test]
fn test_field_with_skip() {
    #[derive(ZodSchema)]
    struct User {
        name: String,
        #[zod(skip)]
        internal_id: u64,
    }

    let schema = User::zod_schema();
    assert!(schema.contains("name"));
    assert!(!schema.contains("internal_id"));
}

#[test]
fn test_field_with_optional() {
    #[derive(ZodSchema)]
    struct User {
        name: String,
        #[zod(optional)]
        nickname: String,
    }

    let schema = User::zod_schema();
    assert!(schema.contains(".optional()"));
}

#[test]
fn test_field_with_nullable() {
    #[derive(ZodSchema)]
    struct User {
        name: String,
        #[zod(nullable)]
        middle_name: String,
    }

    let schema = User::zod_schema();
    assert!(schema.contains(".nullable()"));
}

#[test]
fn test_field_with_default() {
    #[derive(ZodSchema)]
    struct Config {
        #[zod(default = "\"default_value\"")]
        setting: String,
    }

    let schema = Config::zod_schema();
    assert!(schema.contains(".default("));
}

// =============================================================================
// Validation Attribute Tests
// =============================================================================

#[test]
fn test_string_min_max_length() {
    #[derive(ZodSchema)]
    struct User {
        #[zod(min_length = 1, max_length = 100)]
        name: String,
    }

    let schema = User::zod_schema();
    assert!(schema.contains(".min(1)"));
    assert!(schema.contains(".max(100)"));
}

#[test]
fn test_number_min_max() {
    #[derive(ZodSchema)]
    struct Product {
        #[zod(min = 0.0, max = 1000.0)]
        price: f64,
    }

    let schema = Product::zod_schema();
    assert!(schema.contains(".min(0)"));
    assert!(schema.contains(".max(1000)"));
}

#[test]
fn test_email_validation() {
    #[derive(ZodSchema)]
    struct Contact {
        #[zod(email)]
        email: String,
    }

    let schema = Contact::zod_schema();
    assert!(schema.contains(".email()"));
}

#[test]
fn test_url_validation() {
    #[derive(ZodSchema)]
    struct Website {
        #[zod(url)]
        homepage: String,
    }

    let schema = Website::zod_schema();
    assert!(schema.contains(".url()"));
}

#[test]
fn test_uuid_validation() {
    #[derive(ZodSchema)]
    struct Entity {
        #[zod(uuid)]
        id: String,
    }

    let schema = Entity::zod_schema();
    assert!(schema.contains(".uuid()"));
}

#[test]
fn test_regex_validation() {
    #[derive(ZodSchema)]
    struct Code {
        #[zod(regex = "^[A-Z]{3}$")]
        country_code: String,
    }

    let schema = Code::zod_schema();
    assert!(schema.contains(".regex("));
}

#[test]
fn test_positive_validation() {
    #[derive(ZodSchema)]
    struct Quantity {
        #[zod(positive)]
        count: i32,
    }

    let schema = Quantity::zod_schema();
    assert!(schema.contains(".positive()"));
}

#[test]
fn test_int_validation() {
    #[derive(ZodSchema)]
    struct Counter {
        #[zod(int)]
        value: f64,
    }

    let schema = Counter::zod_schema();
    assert!(schema.contains(".int()"));
}

// =============================================================================
// Enum Tests
// =============================================================================

#[test]
fn test_unit_enum_derive() {
    #[derive(ZodSchema)]
    enum Status {
        Active,
        Inactive,
        Pending,
    }

    let schema = Status::zod_schema();
    assert!(schema.contains("z.enum"));
    assert!(schema.contains("Active"));
    assert!(schema.contains("Inactive"));
    assert!(schema.contains("Pending"));
}

#[test]
fn test_unit_enum_with_rename_all() {
    #[derive(ZodSchema)]
    #[zod(rename_all = "SCREAMING_SNAKE_CASE")]
    enum Status {
        Active,
        Inactive,
    }

    let schema = Status::zod_schema();
    assert!(schema.contains("ACTIVE"));
    assert!(schema.contains("INACTIVE"));
}

#[test]
fn test_enum_with_internal_tag() {
    #[derive(ZodSchema)]
    #[zod(tag = "type")]
    enum Message {
        Text { content: String },
        Image { url: String },
    }

    let schema = Message::zod_schema();
    assert!(schema.contains("z.discriminatedUnion"));
    assert!(schema.contains("\"type\""));
}

#[test]
fn test_enum_with_adjacent_tag() {
    #[derive(ZodSchema)]
    #[zod(tag = "kind", content = "data")]
    enum Event {
        Click { x: i32, y: i32 },
        Scroll { delta: f64 },
    }

    let schema = Event::zod_schema();
    assert!(schema.contains("kind"));
    assert!(schema.contains("data"));
}

#[test]
fn test_enum_variant_with_rename() {
    #[derive(ZodSchema)]
    enum Action {
        #[zod(rename = "CREATE")]
        Create,
        #[zod(rename = "UPDATE")]
        Update,
    }

    let schema = Action::zod_schema();
    assert!(schema.contains("CREATE"));
    assert!(schema.contains("UPDATE"));
}

#[test]
fn test_enum_variant_with_skip() {
    #[derive(ZodSchema)]
    enum Status {
        Active,
        #[zod(skip)]
        Internal,
        Inactive,
    }

    let schema = Status::zod_schema();
    assert!(schema.contains("Active"));
    assert!(schema.contains("Inactive"));
    assert!(!schema.contains("Internal"));
}

// =============================================================================
// Complex Type Tests
// =============================================================================

#[test]
fn test_option_type() {
    #[derive(ZodSchema)]
    struct User {
        name: String,
        nickname: Option<String>,
    }

    let schema = User::zod_schema();
    assert!(schema.contains(".optional()"));
}

#[test]
fn test_vec_type() {
    #[derive(ZodSchema)]
    struct Group {
        members: Vec<String>,
    }

    let schema = Group::zod_schema();
    assert!(schema.contains("z.array"));
}

#[test]
fn test_nested_struct() {
    #[derive(ZodSchema)]
    struct Address {
        street: String,
        city: String,
    }

    #[derive(ZodSchema)]
    struct User {
        name: String,
        address: Address,
    }

    let schema = User::zod_schema();
    assert!(schema.contains("AddressSchema"));
}

#[test]
fn test_generic_struct() {
    #[derive(ZodSchema)]
    struct Container<T: ZodSchema> {
        value: T,
    }

    // Generic structs should compile
    let schema = Container::<String>::zod_schema();
    assert!(schema.contains("z.object"));
}

// =============================================================================
// Doc Comment Tests
// =============================================================================

#[test]
fn test_doc_comment_extraction() {
    /// A user in the system
    #[derive(ZodSchema)]
    struct User {
        /// The user's display name
        name: String,
    }

    let metadata = User::metadata();
    assert!(metadata.description.is_some());
    assert!(metadata.description.unwrap().contains("user in the system"));
}

// =============================================================================
// Schema Name and Type Name Tests
// =============================================================================

#[test]
fn test_schema_name() {
    #[derive(ZodSchema)]
    struct MyType {
        field: String,
    }

    assert_eq!(MyType::schema_name(), "MyTypeSchema");
}

#[test]
fn test_ts_type_name() {
    #[derive(ZodSchema)]
    struct MyType {
        field: String,
    }

    assert_eq!(MyType::ts_type_name(), "MyType");
}

#[test]
fn test_ts_declaration() {
    #[derive(ZodSchema)]
    struct User {
        name: String,
    }

    let declaration = User::ts_declaration();
    assert!(declaration.contains("export const UserSchema"));
    assert!(declaration.contains("export type User"));
}
