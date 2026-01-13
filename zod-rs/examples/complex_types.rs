//! Complex Types Example
//!
//! This example demonstrates advanced usage of `zod-rs` with complex
//! type structures including nested types, generics, and various
//! validation scenarios.
//!
//! Run with: `cargo run --example complex_types`

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
// Example 1: Deeply Nested Structures
// =============================================================================

/// Geographic coordinates.
///
/// ```rust,ignore
/// #[derive(ZodSchema)]
/// struct Coordinates {
///     #[zod(min = -90.0, max = 90.0)]
///     latitude: f64,
///     #[zod(min = -180.0, max = 180.0)]
///     longitude: f64,
/// }
/// ```
struct Coordinates {
    #[allow(dead_code)]
    latitude: f64,
    #[allow(dead_code)]
    longitude: f64,
}

impl ZodSchema for Coordinates {
    fn zod_schema() -> &'static str {
        "z.object({ latitude: z.number().min(-90).max(90), longitude: z.number().min(-180).max(180) })"
    }

    fn ts_type_name() -> &'static str {
        "Coordinates"
    }

    fn schema_name() -> &'static str {
        "CoordinatesSchema"
    }
}

/// A physical address.
///
/// ```rust,ignore
/// #[derive(ZodSchema)]
/// #[zod(rename_all = "camelCase")]
/// struct Address {
///     #[zod(min_length = 1)]
///     street_line1: String,
///     street_line2: Option<String>,
///     city: String,
///     #[zod(min_length = 2, max_length = 2)]
///     state_code: String,
///     #[zod(regex = r"^\d{5}(-\d{4})?$")]
///     postal_code: String,
///     coordinates: Option<Coordinates>,
/// }
/// ```
struct Address {
    #[allow(dead_code)]
    street_line1: String,
    #[allow(dead_code)]
    street_line2: Option<String>,
    #[allow(dead_code)]
    city: String,
    #[allow(dead_code)]
    state_code: String,
    #[allow(dead_code)]
    postal_code: String,
    #[allow(dead_code)]
    coordinates: Option<Coordinates>,
}

impl ZodSchema for Address {
    fn zod_schema() -> &'static str {
        r#"z.object({ streetLine1: z.string().min(1), streetLine2: z.string().optional(), city: z.string(), stateCode: z.string().min(2).max(2), postalCode: z.string().regex(/^\d{5}(-\d{4})?$/), coordinates: CoordinatesSchema.optional() })"#
    }

    fn ts_type_name() -> &'static str {
        "Address"
    }

    fn schema_name() -> &'static str {
        "AddressSchema"
    }
}

/// Contact information.
///
/// ```rust,ignore
/// #[derive(ZodSchema)]
/// struct ContactInfo {
///     #[zod(email)]
///     email: String,
///     #[zod(regex = r"^\+?[1-9]\d{1,14}$")]
///     phone: Option<String>,
///     #[zod(url)]
///     website: Option<String>,
/// }
/// ```
struct ContactInfo {
    #[allow(dead_code)]
    email: String,
    #[allow(dead_code)]
    phone: Option<String>,
    #[allow(dead_code)]
    website: Option<String>,
}

impl ZodSchema for ContactInfo {
    fn zod_schema() -> &'static str {
        r#"z.object({ email: z.string().email(), phone: z.string().regex(/^\+?[1-9]\d{1,14}$/).optional(), website: z.string().url().optional() })"#
    }

    fn ts_type_name() -> &'static str {
        "ContactInfo"
    }

    fn schema_name() -> &'static str {
        "ContactInfoSchema"
    }
}

/// A complete organization profile.
///
/// ```rust,ignore
/// #[derive(ZodSchema)]
/// #[zod(rename_all = "camelCase")]
/// struct Organization {
///     #[zod(uuid)]
///     id: String,
///     #[zod(min_length = 1, max_length = 200)]
///     name: String,
///     #[zod(max_length = 1000)]
///     description: Option<String>,
///     headquarters: Address,
///     contact: ContactInfo,
///     #[zod(nonempty)]
///     branch_addresses: Vec<Address>,
/// }
/// ```
struct Organization {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    description: Option<String>,
    #[allow(dead_code)]
    headquarters: Address,
    #[allow(dead_code)]
    contact: ContactInfo,
    #[allow(dead_code)]
    branch_addresses: Vec<Address>,
}

impl ZodSchema for Organization {
    fn zod_schema() -> &'static str {
        "z.object({ id: z.string().uuid(), name: z.string().min(1).max(200), description: z.string().max(1000).optional(), headquarters: AddressSchema, contact: ContactInfoSchema, branchAddresses: z.array(AddressSchema).nonempty() })"
    }

    fn ts_type_name() -> &'static str {
        "Organization"
    }

    fn schema_name() -> &'static str {
        "OrganizationSchema"
    }
}

// =============================================================================
// Example 2: Complex Enum with Multiple Variant Types
// =============================================================================

/// Payment method types.
///
/// ```rust,ignore
/// #[derive(ZodSchema)]
/// #[zod(tag = "type")]
/// enum PaymentMethod {
///     CreditCard {
///         #[zod(regex = r"^\d{16}$")]
///         card_number: String,
///         #[zod(regex = r"^\d{2}/\d{2}$")]
///         expiry: String,
///         #[zod(regex = r"^\d{3,4}$")]
///         cvv: String,
///     },
///     BankTransfer {
///         #[zod(min_length = 8, max_length = 34)]
///         iban: String,
///         bic: Option<String>,
///     },
///     PayPal {
///         #[zod(email)]
///         email: String,
///     },
///     Crypto {
///         currency: String,
///         #[zod(min_length = 26, max_length = 62)]
///         wallet_address: String,
///     },
/// }
/// ```
#[allow(dead_code)]
enum PaymentMethod {
    CreditCard {
        card_number: String,
        expiry: String,
        cvv: String,
    },
    BankTransfer {
        iban: String,
        bic: Option<String>,
    },
    PayPal {
        email: String,
    },
    Crypto {
        currency: String,
        wallet_address: String,
    },
}

impl ZodSchema for PaymentMethod {
    fn zod_schema() -> &'static str {
        r#"z.discriminatedUnion("type", [z.object({ type: z.literal("CreditCard"), cardNumber: z.string().regex(/^\d{16}$/), expiry: z.string().regex(/^\d{2}\/\d{2}$/), cvv: z.string().regex(/^\d{3,4}$/) }), z.object({ type: z.literal("BankTransfer"), iban: z.string().min(8).max(34), bic: z.string().optional() }), z.object({ type: z.literal("PayPal"), email: z.string().email() }), z.object({ type: z.literal("Crypto"), currency: z.string(), walletAddress: z.string().min(26).max(62) })])"#
    }

    fn ts_type_name() -> &'static str {
        "PaymentMethod"
    }

    fn schema_name() -> &'static str {
        "PaymentMethodSchema"
    }
}

// =============================================================================
// Example 3: Recursive/Self-Referential Types
// =============================================================================

/// A tree node structure (self-referential).
///
/// ```rust,ignore
/// #[derive(ZodSchema)]
/// struct TreeNode {
///     value: String,
///     children: Vec<TreeNode>,
/// }
/// ```
///
/// Note: Self-referential types use z.lazy() for the recursive reference.
struct TreeNode {
    #[allow(dead_code)]
    value: String,
    #[allow(dead_code)]
    children: Vec<TreeNode>,
}

impl ZodSchema for TreeNode {
    fn zod_schema() -> &'static str {
        "z.object({ value: z.string(), children: z.array(z.lazy(() => TreeNodeSchema)) })"
    }

    fn ts_type_name() -> &'static str {
        "TreeNode"
    }

    fn schema_name() -> &'static str {
        "TreeNodeSchema"
    }
}

/// A comment with nested replies.
///
/// ```rust,ignore
/// #[derive(ZodSchema)]
/// #[zod(rename_all = "camelCase")]
/// struct Comment {
///     #[zod(uuid)]
///     id: String,
///     #[zod(uuid)]
///     author_id: String,
///     #[zod(min_length = 1, max_length = 10000)]
///     content: String,
///     #[zod(datetime)]
///     created_at: String,
///     replies: Vec<Comment>,
///     #[zod(min = 0)]
///     like_count: u32,
/// }
/// ```
struct Comment {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    author_id: String,
    #[allow(dead_code)]
    content: String,
    #[allow(dead_code)]
    created_at: String,
    #[allow(dead_code)]
    replies: Vec<Comment>,
    #[allow(dead_code)]
    like_count: u32,
}

impl ZodSchema for Comment {
    fn zod_schema() -> &'static str {
        "z.object({ id: z.string().uuid(), authorId: z.string().uuid(), content: z.string().min(1).max(10000), createdAt: z.string().datetime(), replies: z.array(z.lazy(() => CommentSchema)), likeCount: z.number().int().nonnegative().min(0) })"
    }

    fn ts_type_name() -> &'static str {
        "Comment"
    }

    fn schema_name() -> &'static str {
        "CommentSchema"
    }
}

// =============================================================================
// Example 4: API Request/Response Types
// =============================================================================

/// Pagination parameters.
///
/// ```rust,ignore
/// #[derive(ZodSchema)]
/// struct Pagination {
///     #[zod(min = 1, default = "1")]
///     page: u32,
///     #[zod(min = 1, max = 100, default = "20")]
///     per_page: u32,
/// }
/// ```
struct Pagination {
    #[allow(dead_code)]
    page: u32,
    #[allow(dead_code)]
    per_page: u32,
}

impl ZodSchema for Pagination {
    fn zod_schema() -> &'static str {
        "z.object({ page: z.number().int().nonnegative().min(1).default(1), perPage: z.number().int().nonnegative().min(1).max(100).default(20) })"
    }

    fn ts_type_name() -> &'static str {
        "Pagination"
    }

    fn schema_name() -> &'static str {
        "PaginationSchema"
    }
}

/// Sort direction.
///
/// ```rust,ignore
/// #[derive(ZodSchema)]
/// #[zod(rename_all = "lowercase")]
/// enum SortDirection {
///     Asc,
///     Desc,
/// }
/// ```
#[allow(dead_code)]
enum SortDirection {
    Asc,
    Desc,
}

impl ZodSchema for SortDirection {
    fn zod_schema() -> &'static str {
        r#"z.enum(["asc", "desc"])"#
    }

    fn ts_type_name() -> &'static str {
        "SortDirection"
    }

    fn schema_name() -> &'static str {
        "SortDirectionSchema"
    }
}

/// A paginated API response.
///
/// ```rust,ignore
/// #[derive(ZodSchema)]
/// #[zod(rename_all = "camelCase")]
/// struct PaginatedResponse<T> {
///     data: Vec<T>,
///     total_count: u64,
///     page: u32,
///     per_page: u32,
///     has_next_page: bool,
///     has_prev_page: bool,
/// }
/// ```
///
/// Note: Generic types require special handling in the actual implementation.
struct PaginatedResponse {
    #[allow(dead_code)]
    data: Vec<Organization>,
    #[allow(dead_code)]
    total_count: u64,
    #[allow(dead_code)]
    page: u32,
    #[allow(dead_code)]
    per_page: u32,
    #[allow(dead_code)]
    has_next_page: bool,
    #[allow(dead_code)]
    has_prev_page: bool,
}

impl ZodSchema for PaginatedResponse {
    fn zod_schema() -> &'static str {
        "z.object({ data: z.array(OrganizationSchema), totalCount: z.number().int().nonnegative(), page: z.number().int().nonnegative(), perPage: z.number().int().nonnegative(), hasNextPage: z.boolean(), hasPrevPage: z.boolean() })"
    }

    fn ts_type_name() -> &'static str {
        "PaginatedResponse"
    }

    fn schema_name() -> &'static str {
        "PaginatedResponseSchema"
    }
}

// =============================================================================
// Example 5: Strict Mode and Passthrough
// =============================================================================

/// A strict configuration (no extra properties allowed).
///
/// ```rust,ignore
/// #[derive(ZodSchema)]
/// #[zod(strict)]
/// struct StrictConfig {
///     name: String,
///     value: i32,
/// }
/// ```
struct StrictConfig {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    value: i32,
}

impl ZodSchema for StrictConfig {
    fn zod_schema() -> &'static str {
        "z.object({ name: z.string(), value: z.number().int() }).strict()"
    }

    fn ts_type_name() -> &'static str {
        "StrictConfig"
    }

    fn schema_name() -> &'static str {
        "StrictConfigSchema"
    }
}

fn main() {
    println!("=== zod-rs Complex Types Examples ===\n");

    // Example 1: Deeply nested structures
    println!("1. Deeply Nested Structures:");
    println!("   Coordinates: {}", Coordinates::zod_schema());
    println!("   Address: {}", Address::zod_schema());
    println!("   ContactInfo: {}", ContactInfo::zod_schema());
    println!("   Organization: {}", Organization::zod_schema());
    println!();

    // Example 2: Complex enum
    println!("2. Complex Enum (PaymentMethod):");
    println!("   Schema: {}", PaymentMethod::zod_schema());
    println!();

    // Example 3: Recursive types
    println!("3. Recursive Types:");
    println!("   TreeNode: {}", TreeNode::zod_schema());
    println!("   Comment: {}", Comment::zod_schema());
    println!();

    // Example 4: API types
    println!("4. API Request/Response Types:");
    println!("   Pagination: {}", Pagination::zod_schema());
    println!("   SortDirection: {}", SortDirection::zod_schema());
    println!("   PaginatedResponse: {}", PaginatedResponse::zod_schema());
    println!();

    // Example 5: Strict mode
    println!("5. Strict Mode:");
    println!("   StrictConfig: {}", StrictConfig::zod_schema());
    println!();

    // Generate complete TypeScript file
    println!("=== Generated TypeScript Contract ===");
    println!("import {{ z }} from 'zod';");
    println!();
    println!("// Coordinates");
    println!("{}", Coordinates::ts_declaration());
    println!();
    println!("// Address");
    println!("{}", Address::ts_declaration());
    println!();
    println!("// ContactInfo");
    println!("{}", ContactInfo::ts_declaration());
    println!();
    println!("// Organization");
    println!("{}", Organization::ts_declaration());
    println!();
    println!("// PaymentMethod");
    println!("{}", PaymentMethod::ts_declaration());
    println!();
    println!("// TreeNode (recursive)");
    println!("{}", TreeNode::ts_declaration());
    println!();
    println!("// Comment (recursive)");
    println!("{}", Comment::ts_declaration());
    println!();
    println!("// Pagination");
    println!("{}", Pagination::ts_declaration());
    println!();
    println!("// SortDirection");
    println!("{}", SortDirection::ts_declaration());
    println!();
    println!("// PaginatedResponse");
    println!("{}", PaginatedResponse::ts_declaration());
    println!();
    println!("// StrictConfig");
    println!("{}", StrictConfig::ts_declaration());
}
