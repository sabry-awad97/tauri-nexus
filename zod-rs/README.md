# zod-rs

A Rust crate for generating TypeScript [Zod](https://zod.dev/) schemas from Rust types.

[![Crates.io](https://img.shields.io/crates/v/zod-rs.svg)](https://crates.io/crates/zod-rs)
[![Documentation](https://docs.rs/zod-rs/badge.svg)](https://docs.rs/zod-rs)
[![License](https://img.shields.io/crates/l/zod-rs.svg)](LICENSE-MIT)

## Overview

`zod-rs` bridges the gap between Rust and TypeScript by generating type-safe Zod schemas from your Rust type definitions. This ensures your API contracts stay in sync between your Rust backend and TypeScript frontend.

### Features

- 🦀 **Derive macro** - Generate Zod schemas with `#[derive(ZodSchema)]`
- 🔄 **Serde compatible** - Respects `#[serde(...)]` attributes automatically
- ✅ **Validation** - Built-in support for common validations (email, url, min/max, regex, etc.)
- 📦 **Framework agnostic** - Works with any Rust project (web APIs, CLI tools, Tauri apps)
- 🔌 **Extensible** - Pluggable code generator architecture for future schema formats

## Installation

Add `zod-rs` to your `Cargo.toml`:

```toml
[dependencies]
zod-rs = "0.1"
```

Or with specific features:

```toml
[dependencies]
zod-rs = { version = "0.1", features = ["chrono", "uuid"] }
```

## Quick Start

### Basic Usage

```rust
use zod_rs::ZodSchema;

#[derive(ZodSchema)]
struct User {
    name: String,
    age: u32,
    email: Option<String>,
}

fn main() {
    // Get the Zod schema string
    println!("{}", User::zod_schema());
    // Output: z.object({ name: z.string(), age: z.number().int().nonnegative(), email: z.string().optional() })

    // Get the full TypeScript declaration
    println!("{}", User::ts_declaration());
    // Output:
    // export const UserSchema = z.object({ ... });
    // export type User = z.infer<typeof UserSchema>;
}
```

### With Validation

```rust
use zod_rs::ZodSchema;

#[derive(ZodSchema)]
#[zod(rename_all = "camelCase")]
struct CreateUser {
    #[zod(min_length = 1, max_length = 100)]
    user_name: String,

    #[zod(min = 0, max = 150)]
    age: u32,

    #[zod(email)]
    email_address: String,

    #[zod(url, optional)]
    website: Option<String>,
}
```

Generated TypeScript:

```typescript
import { z } from "zod";

export const CreateUserSchema = z.object({
  userName: z.string().min(1).max(100),
  age: z.number().int().nonnegative().min(0).max(150),
  emailAddress: z.string().email(),
  website: z.string().url().optional(),
});

export type CreateUser = z.infer<typeof CreateUserSchema>;
```

### Enums

```rust
use zod_rs::ZodSchema;

// Unit enum -> z.enum()
#[derive(ZodSchema)]
enum Status {
    Active,
    Inactive,
    Pending,
}
// Output: z.enum(["Active", "Inactive", "Pending"])

// Data enum -> z.discriminatedUnion()
#[derive(ZodSchema)]
#[zod(tag = "type")]
enum Message {
    Text { content: String },
    Image { url: String, alt: Option<String> },
    File { path: String, size: u64 },
}
// Output: z.discriminatedUnion("type", [...])
```

## Attributes Reference

### Container Attributes (struct/enum)

| Attribute     | Description                          | Example                            |
| ------------- | ------------------------------------ | ---------------------------------- |
| `rename`      | Rename the type in generated schema  | `#[zod(rename = "UserDTO")]`       |
| `rename_all`  | Rename all fields using a convention | `#[zod(rename_all = "camelCase")]` |
| `tag`         | Internal tagging for enums           | `#[zod(tag = "type")]`             |
| `content`     | Adjacent tagging for enums           | `#[zod(tag = "t", content = "c")]` |
| `description` | Add description to schema            | `#[zod(description = "A user")]`   |
| `deprecated`  | Mark as deprecated                   | `#[zod(deprecated)]`               |
| `strict`      | No extra properties allowed          | `#[zod(strict)]`                   |

#### Rename Conventions

The `rename_all` attribute supports:

- `camelCase` - firstName
- `snake_case` - first_name
- `PascalCase` - FirstName
- `SCREAMING_SNAKE_CASE` - FIRST_NAME
- `kebab-case` - first-name

### Field Attributes

| Attribute  | Description              | Example                           |
| ---------- | ------------------------ | --------------------------------- |
| `rename`   | Rename this field        | `#[zod(rename = "userName")]`     |
| `skip`     | Skip this field          | `#[zod(skip)]`                    |
| `optional` | Mark as optional         | `#[zod(optional)]`                |
| `nullable` | Mark as nullable         | `#[zod(nullable)]`                |
| `default`  | Set default value        | `#[zod(default = "\"default\"")]` |
| `flatten`  | Flatten nested object    | `#[zod(flatten)]`                 |
| `type`     | Custom Zod type override | `#[zod(type = "z.custom()")]`     |

### Validation Attributes

#### String Validations

| Attribute     | Zod Output           | Example                            |
| ------------- | -------------------- | ---------------------------------- |
| `min_length`  | `.min(N)`            | `#[zod(min_length = 1)]`           |
| `max_length`  | `.max(N)`            | `#[zod(max_length = 100)]`         |
| `length`      | `.length(N)`         | `#[zod(length = 10)]`              |
| `email`       | `.email()`           | `#[zod(email)]`                    |
| `url`         | `.url()`             | `#[zod(url)]`                      |
| `uuid`        | `.uuid()`            | `#[zod(uuid)]`                     |
| `cuid`        | `.cuid()`            | `#[zod(cuid)]`                     |
| `datetime`    | `.datetime()`        | `#[zod(datetime)]`                 |
| `ip`          | `.ip()`              | `#[zod(ip)]`                       |
| `regex`       | `.regex(/pattern/)`  | `#[zod(regex = r"^\d+$")]`         |
| `starts_with` | `.startsWith("...")` | `#[zod(starts_with = "https://")]` |
| `ends_with`   | `.endsWith("...")`   | `#[zod(ends_with = ".com")]`       |

#### Number Validations

| Attribute     | Zod Output       | Example               |
| ------------- | ---------------- | --------------------- |
| `min`         | `.min(N)`        | `#[zod(min = 0.0)]`   |
| `max`         | `.max(N)`        | `#[zod(max = 100.0)]` |
| `positive`    | `.positive()`    | `#[zod(positive)]`    |
| `negative`    | `.negative()`    | `#[zod(negative)]`    |
| `nonnegative` | `.nonnegative()` | `#[zod(nonnegative)]` |
| `nonpositive` | `.nonpositive()` | `#[zod(nonpositive)]` |
| `int`         | `.int()`         | `#[zod(int)]`         |
| `finite`      | `.finite()`      | `#[zod(finite)]`      |

#### Array Validations

| Attribute  | Zod Output    | Example            |
| ---------- | ------------- | ------------------ |
| `nonempty` | `.nonempty()` | `#[zod(nonempty)]` |

## Type Mappings

| Rust Type                   | Zod Schema                       |
| --------------------------- | -------------------------------- |
| `String`, `&str`            | `z.string()`                     |
| `bool`                      | `z.boolean()`                    |
| `i8`-`i128`, `isize`        | `z.number().int()`               |
| `u8`-`u128`, `usize`        | `z.number().int().nonnegative()` |
| `f32`, `f64`                | `z.number()`                     |
| `char`                      | `z.string().length(1)`           |
| `Option<T>`                 | `T.optional()`                   |
| `Vec<T>`                    | `z.array(T)`                     |
| `HashMap<K, V>`             | `z.record(K, V)`                 |
| `HashSet<T>`                | `z.set(T)`                       |
| `Box<T>`, `Arc<T>`, `Rc<T>` | `T` (unwrapped)                  |
| `Uuid` (with feature)       | `z.string().uuid()`              |
| `DateTime` (with feature)   | `z.string().datetime()`          |

## Feature Flags

| Feature        | Description                    | Default |
| -------------- | ------------------------------ | ------- |
| `std`          | Standard library support       | ✅      |
| `serde-compat` | Respect serde attributes       | ✅      |
| `chrono`       | Support for `chrono::DateTime` | ❌      |
| `uuid`         | Support for `uuid::Uuid`       | ❌      |
| `tauri`        | Tauri framework integration    | ❌      |

### Enabling Features

```toml
[dependencies]
zod-rs = { version = "0.1", features = ["chrono", "uuid"] }
```

## Serde Compatibility

When the `serde-compat` feature is enabled (default), `zod-rs` respects serde attributes:

```rust
use serde::{Serialize, Deserialize};
use zod_rs::ZodSchema;

#[derive(Serialize, Deserialize, ZodSchema)]
#[serde(rename_all = "camelCase")]
struct User {
    #[serde(rename = "userId")]
    id: u64,

    #[serde(skip)]
    internal_field: String,

    #[serde(default)]
    active: bool,
}
```

The `#[zod(...)]` attributes take precedence over `#[serde(...)]` when both are present.

## Advanced Usage

### Nested Types

```rust
use zod_rs::ZodSchema;

#[derive(ZodSchema)]
struct Address {
    street: String,
    city: String,
}

#[derive(ZodSchema)]
struct User {
    name: String,
    address: Address,  // References AddressSchema
}
```

### Recursive Types

```rust
use zod_rs::ZodSchema;

#[derive(ZodSchema)]
struct TreeNode {
    value: String,
    children: Vec<TreeNode>,  // Uses z.lazy() for recursion
}
```

### Custom Type Override

```rust
use zod_rs::ZodSchema;

#[derive(ZodSchema)]
struct User {
    #[zod(type = "z.string().brand<'UserId'>()")]
    id: String,
}
```

### Strict Mode

```rust
use zod_rs::ZodSchema;

#[derive(ZodSchema)]
#[zod(strict)]
struct Config {
    name: String,
    value: i32,
}
// Output: z.object({ ... }).strict()
```

## CLI Tool

Install the CLI for batch schema generation:

```bash
cargo install zod-rs-cli
```

### Usage

```bash
# Generate schemas from Rust source files
zod-rs generate --input ./src --output ./generated

# Watch mode for development
zod-rs generate --watch

# Initialize configuration
zod-rs init
```

### Configuration

Create a `zod-rs.toml` in your project root:

```toml
[output]
path = "./generated"
format = "typescript"

[options]
rename_all = "camelCase"
generate_types = true
```

## Examples

See the [examples](./examples) directory for more detailed examples:

- [`basic_usage.rs`](./examples/basic_usage.rs) - Basic struct and enum usage
- [`serde_integration.rs`](./examples/serde_integration.rs) - Serde attribute compatibility
- [`complex_types.rs`](./examples/complex_types.rs) - Nested types, recursion, and advanced patterns

Run examples with:

```bash
cargo run --example basic_usage
cargo run --example serde_integration
cargo run --example complex_types
```

## Crate Structure

The `zod-rs` workspace consists of three crates:

| Crate           | Description              |
| --------------- | ------------------------ |
| `zod-rs`        | Runtime traits and types |
| `zod-rs-macros` | Procedural derive macro  |
| `zod-rs-cli`    | Command-line tool        |

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Development

```bash
# Run all tests
cargo test --workspace

# Run examples
cargo run --example basic_usage

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy --workspace
```

### Snapshot Testing

This project uses [insta](https://insta.rs/) for snapshot testing. Snapshots capture the expected output of the macro and are stored in `zod-rs-macros/tests/snapshots/`.

```bash
# Install cargo-insta (one-time setup)
cargo install cargo-insta

# Run tests (creates .snap.new files if snapshots change)
cargo test -p zod-rs-macros

# Review pending snapshot changes interactively
cargo insta review

# Accept all pending snapshots
cargo insta accept

# Reject all pending snapshots
cargo insta reject

# Run tests and automatically update snapshots
cargo insta test --accept
```

#### Snapshot Workflow

1. **Run tests** - If output changes, new `.snap.new` files are created
2. **Review changes** - Use `cargo insta review` to see diffs and accept/reject
3. **Commit snapshots** - The `.snap` files should be committed to git
4. **CI validation** - CI will fail if there are pending `.snap.new` files

> **Note:** `.snap.new` files are gitignored. Only reviewed and accepted `.snap` files should be committed.

## Acknowledgments

- [Zod](https://zod.dev/) - TypeScript-first schema validation
- [serde](https://serde.rs/) - Rust serialization framework
- [syn](https://docs.rs/syn) and [quote](https://docs.rs/quote) - Rust procedural macro libraries
- [darling](https://docs.rs/darling) - Attribute parsing for derive macros
