//! Basic Usage Example
//!
//! This example demonstrates the fundamental usage of the `zod-rs` crate
//! for generating TypeScript Zod schemas from Rust types.
//!
//! Run with: `cargo run --example basic_usage`

// In a real project, you would use:
// use zod_rs::ZodSchema;

// For this example, we'll simulate the trait and its implementation
// since we can't actually run the derive macro in a standalone example.

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
// Example 1: Basic Struct
// =============================================================================

/// A simple user struct.
///
/// In a real project, you would derive ZodSchema:
/// ```rust,ignore
/// #[derive(ZodSchema)]
/// struct User {
///     name: String,
///     age: u32,
///     email: Option<String>,
/// }
/// ```
struct User {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    age: u32,
    #[allow(dead_code)]
    email: Option<String>,
}

impl ZodSchema for User {
    fn zod_schema() -> &'static str {
        "z.object({ name: z.string(), age: z.number().int().nonnegative(), email: z.string().optional() })"
    }

    fn ts_type_name() -> &'static str {
        "User"
    }

    fn schema_name() -> &'static str {
        "UserSchema"
    }
}

// =============================================================================
// Example 2: Struct with Validation
// =============================================================================

/// A user creation request with validation rules.
///
/// In a real project:
/// ```rust,ignore
/// #[derive(ZodSchema)]
/// #[zod(rename_all = "camelCase")]
/// struct CreateUser {
///     #[zod(min_length = 1, max_length = 100)]
///     user_name: String,
///
///     #[zod(min = 0, max = 150)]
///     age: u32,
///
///     #[zod(email)]
///     email_address: String,
/// }
/// ```
struct CreateUser {
    #[allow(dead_code)]
    user_name: String,
    #[allow(dead_code)]
    age: u32,
    #[allow(dead_code)]
    email_address: String,
}

impl ZodSchema for CreateUser {
    fn zod_schema() -> &'static str {
        "z.object({ userName: z.string().min(1).max(100), age: z.number().int().nonnegative().min(0).max(150), emailAddress: z.string().email() })"
    }

    fn ts_type_name() -> &'static str {
        "CreateUser"
    }

    fn schema_name() -> &'static str {
        "CreateUserSchema"
    }
}

// =============================================================================
// Example 3: Unit Enum
// =============================================================================

/// A status enum with unit variants.
///
/// In a real project:
/// ```rust,ignore
/// #[derive(ZodSchema)]
/// enum Status {
///     Active,
///     Inactive,
///     Pending,
/// }
/// ```
#[allow(dead_code)]
enum Status {
    Active,
    Inactive,
    Pending,
}

impl ZodSchema for Status {
    fn zod_schema() -> &'static str {
        r#"z.enum(["Active", "Inactive", "Pending"])"#
    }

    fn ts_type_name() -> &'static str {
        "Status"
    }

    fn schema_name() -> &'static str {
        "StatusSchema"
    }
}

// =============================================================================
// Example 4: Nested Struct
// =============================================================================

/// An address struct.
struct Address {
    #[allow(dead_code)]
    street: String,
    #[allow(dead_code)]
    city: String,
    #[allow(dead_code)]
    country: String,
}

impl ZodSchema for Address {
    fn zod_schema() -> &'static str {
        "z.object({ street: z.string(), city: z.string(), country: z.string() })"
    }

    fn ts_type_name() -> &'static str {
        "Address"
    }

    fn schema_name() -> &'static str {
        "AddressSchema"
    }
}

/// A user profile with nested address.
///
/// In a real project:
/// ```rust,ignore
/// #[derive(ZodSchema)]
/// struct UserProfile {
///     user: User,
///     address: Address,
///     bio: Option<String>,
/// }
/// ```
struct UserProfile {
    #[allow(dead_code)]
    user: User,
    #[allow(dead_code)]
    address: Address,
    #[allow(dead_code)]
    bio: Option<String>,
}

impl ZodSchema for UserProfile {
    fn zod_schema() -> &'static str {
        "z.object({ user: UserSchema, address: AddressSchema, bio: z.string().optional() })"
    }

    fn ts_type_name() -> &'static str {
        "UserProfile"
    }

    fn schema_name() -> &'static str {
        "UserProfileSchema"
    }
}

fn main() {
    println!("=== zod-rs Basic Usage Examples ===\n");

    // Example 1: Basic struct
    println!("1. Basic Struct (User):");
    println!("   Schema: {}", User::zod_schema());
    println!("   Type name: {}", User::ts_type_name());
    println!("   Schema name: {}", User::schema_name());
    println!();

    // Example 2: Struct with validation
    println!("2. Struct with Validation (CreateUser):");
    println!("   Schema: {}", CreateUser::zod_schema());
    println!();

    // Example 3: Unit enum
    println!("3. Unit Enum (Status):");
    println!("   Schema: {}", Status::zod_schema());
    println!();

    // Example 4: Nested struct
    println!("4. Nested Struct (UserProfile):");
    println!("   Schema: {}", UserProfile::zod_schema());
    println!();

    // Full TypeScript declaration
    println!("5. Full TypeScript Declaration:");
    println!("{}", User::ts_declaration());
    println!();

    // Generate a complete TypeScript file
    println!("6. Complete TypeScript Contract:");
    println!("---");
    println!("import {{ z }} from 'zod';");
    println!();
    println!("{}", User::ts_declaration());
    println!();
    println!("{}", CreateUser::ts_declaration());
    println!();
    println!("{}", Status::ts_declaration());
    println!();
    println!("{}", Address::ts_declaration());
    println!();
    println!("{}", UserProfile::ts_declaration());
    println!("---");
}
