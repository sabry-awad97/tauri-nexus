# zod-rs-macros

Procedural macros for generating TypeScript Zod schemas from Rust types.

This crate provides the `#[derive(ZodSchema)]` macro.

## Usage

```rust
use zod_rs_macros::ZodSchema;

#[derive(ZodSchema)]
#[zod(rename_all = "camelCase")]
struct User {
    #[zod(min_length = 1)]
    name: String,

    #[zod(min = 0)]
    age: u32,

    #[zod(email)]
    email: Option<String>,
}
```

## Features

- `serde-compat` - Respect serde attributes (enabled by default)
- `chrono` - Support for `chrono::DateTime` types
- `uuid` - Support for `uuid::Uuid` type

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
