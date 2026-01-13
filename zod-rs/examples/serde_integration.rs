//! Serde Integration Example
//!
//! This example demonstrates how `zod-rs` integrates with serde attributes
//! when the `serde-compat` feature is enabled (default).
//!
//! Run with: `cargo run --example serde_integration`

// In a real project, you would use:
// use zod_rs::ZodSchema;
// use serde::{Serialize, Deserialize};

/// Trait for types that can generate Zod schemas.
pub trait ZodSchema {
    fn zod_schema() -> &'static str;
    fn ts_type_name() -> &'static str;
    fn schema_name() -> &'static str;
    fn ts_declaration() -> String {
        format!(
            "export const {} = {};\nexport type {} = z.infer<typeof {}>;",
            Self::schema_name(),
            Self::zod_schema(),
            Self::ts_type_name(),
            Self::schema_name()
        )
    }
}

// =============================================================================
// Example 1: Serde rename_all
// =============================================================================

/// A user struct using serde's rename_all attribute.
///
/// In a real project:
/// ```rust,ignore
/// #[derive(Serialize, Deserialize, ZodSchema)]
/// #[serde(rename_all = "camelCase")]
/// struct User {
///     user_name: String,
///     email_address: String,
///     created_at: String,
/// }
/// ```
///
/// The `serde-compat` feature makes zod-rs respect serde attributes,
/// so field names are automatically converted to camelCase.
struct User {
    #[allow(dead_code)]
    user_name: String,
    #[allow(dead_code)]
    email_address: String,
    #[allow(dead_code)]
    created_at: String,
}

impl ZodSchema for User {
    fn zod_schema() -> &'static str {
        "z.object({ userName: z.string(), emailAddress: z.string(), createdAt: z.string() })"
    }

    fn ts_type_name() -> &'static str {
        "User"
    }

    fn schema_name() -> &'static str {
        "UserSchema"
    }
}

// =============================================================================
// Example 2: Serde field rename
// =============================================================================

/// A struct with individual field renames.
///
/// In a real project:
/// ```rust,ignore
/// #[derive(Serialize, Deserialize, ZodSchema)]
/// struct ApiResponse {
///     #[serde(rename = "statusCode")]
///     status_code: u32,
///
///     #[serde(rename = "responseBody")]
///     body: String,
///
///     #[serde(rename = "isSuccess")]
///     success: bool,
/// }
/// ```
struct ApiResponse {
    #[allow(dead_code)]
    status_code: u32,
    #[allow(dead_code)]
    body: String,
    #[allow(dead_code)]
    success: bool,
}

impl ZodSchema for ApiResponse {
    fn zod_schema() -> &'static str {
        "z.object({ statusCode: z.number().int().nonnegative(), responseBody: z.string(), isSuccess: z.boolean() })"
    }

    fn ts_type_name() -> &'static str {
        "ApiResponse"
    }

    fn schema_name() -> &'static str {
        "ApiResponseSchema"
    }
}

// =============================================================================
// Example 3: Serde skip
// =============================================================================

/// A struct with skipped fields.
///
/// In a real project:
/// ```rust,ignore
/// #[derive(Serialize, Deserialize, ZodSchema)]
/// struct UserWithSecrets {
///     id: u64,
///     name: String,
///
///     #[serde(skip)]
///     password_hash: String,
///
///     #[serde(skip_serializing)]
///     internal_id: u64,
/// }
/// ```
///
/// Fields marked with `#[serde(skip)]` are excluded from the Zod schema.
struct UserWithSecrets {
    #[allow(dead_code)]
    id: u64,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    password_hash: String,
    #[allow(dead_code)]
    internal_id: u64,
}

impl ZodSchema for UserWithSecrets {
    fn zod_schema() -> &'static str {
        "z.object({ id: z.number().int().nonnegative(), name: z.string() })"
    }

    fn ts_type_name() -> &'static str {
        "UserWithSecrets"
    }

    fn schema_name() -> &'static str {
        "UserWithSecretsSchema"
    }
}

// =============================================================================
// Example 4: Serde default
// =============================================================================

/// A struct with default values.
///
/// In a real project:
/// ```rust,ignore
/// #[derive(Serialize, Deserialize, ZodSchema)]
/// struct Config {
///     name: String,
///
///     #[serde(default)]
///     enabled: bool,
///
///     #[serde(default = "default_timeout")]
///     timeout_ms: u32,
/// }
///
/// fn default_timeout() -> u32 { 5000 }
/// ```
///
/// Fields with `#[serde(default)]` are marked as optional in the Zod schema.
struct Config {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    enabled: bool,
    #[allow(dead_code)]
    timeout_ms: u32,
}

impl ZodSchema for Config {
    fn zod_schema() -> &'static str {
        "z.object({ name: z.string(), enabled: z.boolean().optional(), timeoutMs: z.number().int().nonnegative().optional() })"
    }

    fn ts_type_name() -> &'static str {
        "Config"
    }

    fn schema_name() -> &'static str {
        "ConfigSchema"
    }
}

// =============================================================================
// Example 5: Serde tagged enum
// =============================================================================

/// A tagged enum using serde's tag attribute.
///
/// In a real project:
/// ```rust,ignore
/// #[derive(Serialize, Deserialize, ZodSchema)]
/// #[serde(tag = "type")]
/// enum Event {
///     UserCreated { user_id: u64, name: String },
///     UserDeleted { user_id: u64 },
///     UserUpdated { user_id: u64, changes: Vec<String> },
/// }
/// ```
///
/// The `#[serde(tag = "type")]` attribute is respected, generating a
/// discriminated union in Zod.
#[allow(dead_code)]
enum Event {
    UserCreated { user_id: u64, name: String },
    UserDeleted { user_id: u64 },
    UserUpdated { user_id: u64, changes: Vec<String> },
}

impl ZodSchema for Event {
    fn zod_schema() -> &'static str {
        r#"z.discriminatedUnion("type", [z.object({ type: z.literal("UserCreated"), userId: z.number().int().nonnegative(), name: z.string() }), z.object({ type: z.literal("UserDeleted"), userId: z.number().int().nonnegative() }), z.object({ type: z.literal("UserUpdated"), userId: z.number().int().nonnegative(), changes: z.array(z.string()) })])"#
    }

    fn ts_type_name() -> &'static str {
        "Event"
    }

    fn schema_name() -> &'static str {
        "EventSchema"
    }
}

// =============================================================================
// Example 6: Zod overrides serde
// =============================================================================

/// A struct where zod attributes override serde attributes.
///
/// In a real project:
/// ```rust,ignore
/// #[derive(Serialize, Deserialize, ZodSchema)]
/// #[serde(rename_all = "camelCase")]
/// struct MixedAttrs {
///     // Uses serde's camelCase: firstName
///     first_name: String,
///
///     // Zod rename overrides serde: custom_name
///     #[zod(rename = "custom_name")]
///     last_name: String,
///
///     // Serde skips, but zod includes with validation
///     #[serde(skip)]
///     #[zod(skip = false, email)]
///     email: String,
/// }
/// ```
///
/// When both `#[serde(...)]` and `#[zod(...)]` are present, the zod
/// attribute takes precedence.
struct MixedAttrs {
    #[allow(dead_code)]
    first_name: String,
    #[allow(dead_code)]
    last_name: String,
    #[allow(dead_code)]
    email: String,
}

impl ZodSchema for MixedAttrs {
    fn zod_schema() -> &'static str {
        "z.object({ firstName: z.string(), custom_name: z.string(), email: z.string().email() })"
    }

    fn ts_type_name() -> &'static str {
        "MixedAttrs"
    }

    fn schema_name() -> &'static str {
        "MixedAttrsSchema"
    }
}

fn main() {
    println!("=== zod-rs Serde Integration Examples ===\n");

    // Example 1: rename_all
    println!("1. Serde rename_all (User):");
    println!("   Schema: {}", User::zod_schema());
    println!("   Note: Field names converted to camelCase");
    println!();

    // Example 2: field rename
    println!("2. Serde field rename (ApiResponse):");
    println!("   Schema: {}", ApiResponse::zod_schema());
    println!("   Note: Individual fields renamed");
    println!();

    // Example 3: skip
    println!("3. Serde skip (UserWithSecrets):");
    println!("   Schema: {}", UserWithSecrets::zod_schema());
    println!("   Note: password_hash and internal_id excluded");
    println!();

    // Example 4: default
    println!("4. Serde default (Config):");
    println!("   Schema: {}", Config::zod_schema());
    println!("   Note: enabled and timeoutMs are optional");
    println!();

    // Example 5: tagged enum
    println!("5. Serde tagged enum (Event):");
    println!("   Schema: {}", Event::zod_schema());
    println!("   Note: Uses discriminatedUnion with 'type' tag");
    println!();

    // Example 6: zod overrides serde
    println!("6. Zod overrides serde (MixedAttrs):");
    println!("   Schema: {}", MixedAttrs::zod_schema());
    println!("   Note: zod attributes take precedence");
    println!();

    // Generate TypeScript
    println!("=== Generated TypeScript ===");
    println!("import {{ z }} from 'zod';");
    println!();
    println!("{}", User::ts_declaration());
    println!();
    println!("{}", ApiResponse::ts_declaration());
    println!();
    println!("{}", UserWithSecrets::ts_declaration());
    println!();
    println!("{}", Config::ts_declaration());
    println!();
    println!("{}", Event::ts_declaration());
}
