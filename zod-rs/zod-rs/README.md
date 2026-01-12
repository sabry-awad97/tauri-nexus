# zod-rs

Runtime traits and types for generating TypeScript Zod schemas from Rust types.

This is the runtime crate that provides the `ZodSchema` trait and supporting types.
For the derive macro, see `zod-rs-macros`.

## Usage

```rust
use zod_rs::ZodSchema;

// The trait is typically derived using #[derive(ZodSchema)]
// but can also be implemented manually:

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
}
```

## Features

- `std` - Standard library support (enabled by default)
- `serde-compat` - Respect serde attributes (enabled by default)
- `chrono` - Support for `chrono::DateTime` types
- `uuid` - Support for `uuid::Uuid` type
- `tauri` - Tauri framework integration

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
