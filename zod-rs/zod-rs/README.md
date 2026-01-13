# zod-rs

Runtime traits and types for generating TypeScript Zod schemas from Rust types.

[![Crates.io](https://img.shields.io/crates/v/zod-rs.svg)](https://crates.io/crates/zod-rs)
[![Documentation](https://docs.rs/zod-rs/badge.svg)](https://docs.rs/zod-rs)

This is the runtime crate that provides the `ZodSchema` trait and supporting types.
For the derive macro, see [`zod-rs-macros`](https://crates.io/crates/zod-rs-macros).

## Installation

```toml
[dependencies]
zod-rs = "0.1"
```

## Usage

### With Derive Macro (Recommended)

```rust
use zod_rs::ZodSchema;

#[derive(ZodSchema)]
struct User {
    name: String,
    age: u32,
}

fn main() {
    // Get the Zod schema string
    println!("{}", User::zod_schema());
    // => z.object({ name: z.string(), age: z.number().int().nonnegative() })

    // Get the full TypeScript declaration
    println!("{}", User::ts_declaration());
    // => export const UserSchema = z.object({ ... });
    // => export type User = z.infer<typeof UserSchema>;
}
```

### Manual Implementation

```rust
use zod_rs::{ZodSchema, SchemaMetadata};

struct MyType {
    value: i32,
}

impl ZodSchema for MyType {
    fn zod_schema() -> &'static str {
        "z.object({ value: z.number().int() })"
    }

    fn ts_type_name() -> &'static str {
        "MyType"
    }

    fn schema_name() -> &'static str {
        "MyTypeSchema"
    }

    fn metadata() -> SchemaMetadata {
        SchemaMetadata {
            description: Some("A custom type".to_string()),
            ..Default::default()
        }
    }
}
```

## Features

| Feature        | Description                    | Default |
| -------------- | ------------------------------ | ------- |
| `std`          | Standard library support       | ✅      |
| `serde-compat` | Respect serde attributes       | ✅      |
| `chrono`       | Support for `chrono::DateTime` | ❌      |
| `uuid`         | Support for `uuid::Uuid`       | ❌      |
| `tauri`        | Tauri framework integration    | ❌      |

## Trait Methods

The `ZodSchema` trait provides:

| Method             | Description                             |
| ------------------ | --------------------------------------- |
| `zod_schema()`     | Returns the Zod schema string           |
| `ts_type_name()`   | Returns the TypeScript type name        |
| `schema_name()`    | Returns the schema variable name        |
| `ts_declaration()` | Returns the full TypeScript declaration |
| `metadata()`       | Returns schema metadata                 |

## Blanket Implementations

This crate provides implementations for common Rust types:

- **Primitives**: `String`, `bool`, `char`, integers, floats
- **Collections**: `Option<T>`, `Vec<T>`, `HashMap<K, V>`, `HashSet<T>`
- **Feature-gated**: `Uuid`, `DateTime<Tz>`

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
