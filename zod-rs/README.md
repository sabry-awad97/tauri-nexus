# zod-rs

A Rust crate for generating TypeScript [Zod](https://zod.dev/) schemas from Rust types.

[![Crates.io](https://img.shields.io/crates/v/zod-rs.svg)](https://crates.io/crates/zod-rs)
[![Documentation](https://docs.rs/zod-rs/badge.svg)](https://docs.rs/zod-rs)
[![License](https://img.shields.io/crates/l/zod-rs.svg)](LICENSE-MIT)

## Features

- 🦀 **Derive macro** - Generate Zod schemas with `#[derive(ZodSchema)]`
- 🔄 **Serde compatible** - Respects `#[serde(...)]` attributes
- ✅ **Validation** - Built-in support for common validations (email, url, min/max, etc.)
- 📦 **Framework agnostic** - Works with any Rust project
- 🔌 **Extensible** - Pluggable code generator architecture

## Quick Start

Add `zod-rs` to your `Cargo.toml`:

```toml
[dependencies]
zod-rs = "0.1"
```

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
    name: String,

    #[zod(min = 0, max = 150)]
    age: u32,

    #[zod(email)]
    email: String,

    #[zod(url, optional)]
    website: Option<String>,
}
```

Generated TypeScript:

```typescript
import { z } from "zod";

export const CreateUserSchema = z.object({
  name: z.string().min(1).max(100),
  age: z.number().int().nonnegative().min(0).max(150),
  email: z.string().email(),
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

// Data enum -> z.discriminatedUnion()
#[derive(ZodSchema)]
#[zod(tag = "type")]
enum Message {
    Text { content: String },
    Image { url: String, alt: Option<String> },
    File { path: String, size: u64 },
}
```

## Attributes

### Container Attributes (struct/enum)

| Attribute                          | Description                                                                             |
| ---------------------------------- | --------------------------------------------------------------------------------------- |
| `#[zod(rename = "Name")]`          | Rename the type in generated schema                                                     |
| `#[zod(rename_all = "camelCase")]` | Rename all fields (camelCase, snake_case, PascalCase, SCREAMING_SNAKE_CASE, kebab-case) |
| `#[zod(tag = "type")]`             | Use internal tagging for enums                                                          |
| `#[zod(tag = "t", content = "c")]` | Use adjacent tagging for enums                                                          |
| `#[zod(description = "...")]`      | Add description to schema                                                               |
| `#[zod(deprecated)]`               | Mark as deprecated                                                                      |
| `#[zod(strict)]`                   | Use strict mode (no extra properties)                                                   |

### Field Attributes

| Attribute                     | Description                      |
| ----------------------------- | -------------------------------- |
| `#[zod(rename = "name")]`     | Rename this field                |
| `#[zod(skip)]`                | Skip this field in schema        |
| `#[zod(optional)]`            | Mark as optional (`.optional()`) |
| `#[zod(nullable)]`            | Mark as nullable (`.nullable()`) |
| `#[zod(default = "value")]`   | Set default value                |
| `#[zod(flatten)]`             | Flatten nested object fields     |
| `#[zod(type = "z.custom()")]` | Override with custom Zod type    |

### Validation Attributes

| Attribute                   | Applies To | Zod Output          |
| --------------------------- | ---------- | ------------------- |
| `#[zod(min = N)]`           | numbers    | `.min(N)`           |
| `#[zod(max = N)]`           | numbers    | `.max(N)`           |
| `#[zod(min_length = N)]`    | strings    | `.min(N)`           |
| `#[zod(max_length = N)]`    | strings    | `.max(N)`           |
| `#[zod(length = N)]`        | strings    | `.length(N)`        |
| `#[zod(email)]`             | strings    | `.email()`          |
| `#[zod(url)]`               | strings    | `.url()`            |
| `#[zod(uuid)]`              | strings    | `.uuid()`           |
| `#[zod(regex = "pattern")]` | strings    | `.regex(/pattern/)` |
| `#[zod(positive)]`          | numbers    | `.positive()`       |
| `#[zod(negative)]`          | numbers    | `.negative()`       |
| `#[zod(int)]`               | numbers    | `.int()`            |
| `#[zod(nonempty)]`          | arrays     | `.nonempty()`       |

## Feature Flags

| Feature        | Description                    | Default |
| -------------- | ------------------------------ | ------- |
| `std`          | Standard library support       | ✅      |
| `serde-compat` | Respect serde attributes       | ✅      |
| `chrono`       | Support for `chrono::DateTime` | ❌      |
| `uuid`         | Support for `uuid::Uuid`       | ❌      |
| `tauri`        | Tauri framework integration    | ❌      |

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
| `Box<T>`, `Arc<T>`, `Rc<T>` | `T` (unwrapped)                  |
| `Uuid` (with feature)       | `z.string().uuid()`              |
| `DateTime` (with feature)   | `z.string().datetime()`          |

## CLI Tool

Install the CLI for batch schema generation:

```bash
cargo install zod-rs-cli
```

```bash
# Generate schemas from Rust source files
zod-rs generate --input ./src --output ./generated

# Watch mode for development
zod-rs generate --watch

# Initialize configuration
zod-rs init
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
