# zod-rs-macros

Procedural macros for generating TypeScript Zod schemas from Rust types.

[![Crates.io](https://img.shields.io/crates/v/zod-rs-macros.svg)](https://crates.io/crates/zod-rs-macros)
[![Documentation](https://docs.rs/zod-rs-macros/badge.svg)](https://docs.rs/zod-rs-macros)

This crate provides the `#[derive(ZodSchema)]` macro. For the runtime traits,
see [`zod-rs`](https://crates.io/crates/zod-rs).

## Installation

```toml
[dependencies]
zod-rs-macros = "0.1"
```

Or use the main `zod-rs` crate which re-exports the macro:

```toml
[dependencies]
zod-rs = "0.1"
```

## Basic Usage

```rust
use zod_rs_macros::ZodSchema;

#[derive(ZodSchema)]
#[zod(rename_all = "camelCase")]
struct User {
    #[zod(min_length = 1)]
    user_name: String,

    #[zod(min = 0)]
    age: u32,

    #[zod(email)]
    email: Option<String>,
}
```

## Attributes

### Container Attributes

| Attribute                          | Description                |
| ---------------------------------- | -------------------------- |
| `#[zod(rename = "Name")]`          | Rename the type            |
| `#[zod(rename_all = "camelCase")]` | Rename all fields          |
| `#[zod(tag = "type")]`             | Internal tagging for enums |
| `#[zod(tag = "t", content = "c")]` | Adjacent tagging           |
| `#[zod(description = "...")]`      | Add description            |
| `#[zod(deprecated)]`               | Mark as deprecated         |
| `#[zod(strict)]`                   | Strict mode                |

### Field Attributes

| Attribute                   | Description           |
| --------------------------- | --------------------- |
| `#[zod(rename = "name")]`   | Rename field          |
| `#[zod(skip)]`              | Skip field            |
| `#[zod(optional)]`          | Mark optional         |
| `#[zod(nullable)]`          | Mark nullable         |
| `#[zod(default = "value")]` | Set default           |
| `#[zod(flatten)]`           | Flatten nested object |

### Validation Attributes

**String**: `min_length`, `max_length`, `length`, `email`, `url`, `uuid`, `regex`

**Number**: `min`, `max`, `positive`, `negative`, `int`, `finite`

**Array**: `nonempty`

## Enum Support

```rust
// Unit enum
#[derive(ZodSchema)]
enum Status {
    Active,
    Inactive,
}
// => z.enum(["Active", "Inactive"])

// Tagged enum
#[derive(ZodSchema)]
#[zod(tag = "type")]
enum Message {
    Text { content: String },
    Image { url: String },
}
// => z.discriminatedUnion("type", [...])
```

## Features

| Feature        | Description                    | Default |
| -------------- | ------------------------------ | ------- |
| `serde-compat` | Respect serde attributes       | ✅      |
| `chrono`       | Support for `chrono::DateTime` | ❌      |
| `uuid`         | Support for `uuid::Uuid`       | ❌      |

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
