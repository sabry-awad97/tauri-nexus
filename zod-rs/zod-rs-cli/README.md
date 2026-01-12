# zod-rs-cli

CLI tool for generating TypeScript Zod schemas from Rust source files.

## Installation

```bash
cargo install zod-rs-cli
```

## Usage

```bash
# Generate schemas from Rust source files
zod-rs generate --input ./src --output ./generated

# Watch mode for development
zod-rs generate --watch

# Initialize configuration
zod-rs init

# Validate existing schemas
zod-rs validate --path ./generated
```

## Configuration

Create a `zod-rs.toml` file in your project root:

```toml
[output]
dir = "./generated"
file = "schemas.ts"
generate_types = true
generate_docs = true

[naming]
rename_all = "camelCase"
schema_suffix = "Schema"

[features]
serde_compat = true
chrono = false
uuid = false
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
