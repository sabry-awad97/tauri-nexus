# Contributing to zod-rs

Thank you for your interest in contributing to zod-rs! This document provides guidelines and instructions for contributing.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/zod-rs.git`
3. Create a branch: `git checkout -b feature/your-feature-name`

## Development Setup

### Prerequisites

- Rust 1.70+ (stable)
- cargo-insta for snapshot testing: `cargo install cargo-insta`

### Building

```bash
# Build all crates
cargo build --workspace

# Build with all features
cargo build --workspace --all-features
```

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p zod-rs
cargo test -p zod-rs-macros

# Run tests with output
cargo test --workspace -- --nocapture
```

## Snapshot Testing with Insta

This project uses [insta](https://insta.rs/) for snapshot testing the macro output. This ensures that changes to the code generation are intentional and reviewed.

### How It Works

1. Snapshot tests capture the generated Zod schema output
2. Expected outputs are stored in `.snap` files in `zod-rs-macros/tests/snapshots/`
3. When tests run, actual output is compared against snapshots
4. If output differs, a `.snap.new` file is created for review

### Commands

```bash
# Install cargo-insta (required for snapshot management)
cargo install cargo-insta

# Run tests - creates .snap.new files if output changed
cargo test -p zod-rs-macros

# Interactive review of pending snapshots
cargo insta review

# Accept all pending snapshots
cargo insta accept

# Reject all pending snapshots
cargo insta reject

# Run tests and auto-accept new snapshots
cargo insta test --accept

# Run tests in CI mode (fails if pending snapshots exist)
cargo insta test
```

### Workflow for Changing Output

1. Make your code changes
2. Run `cargo test -p zod-rs-macros`
3. If snapshots changed, review with `cargo insta review`
4. Accept intentional changes, reject unintentional ones
5. Commit both your code and updated `.snap` files

### Adding New Snapshot Tests

```rust
use insta::assert_snapshot;

#[test]
fn test_my_new_feature() {
    let output = generate_schema_for_my_type();
    assert_snapshot!(output);
}
```

On first run, insta will create a new `.snap` file. Review and accept it with `cargo insta review`.

### CI Behavior

- CI runs `cargo insta test` which fails if any `.snap.new` files exist
- This ensures all snapshot changes are reviewed before merging
- `.snap.new` files are gitignored and should never be committed

## Code Style

### Formatting

```bash
# Check formatting
cargo fmt --check

# Apply formatting
cargo fmt
```

### Linting

```bash
# Run clippy
cargo clippy --workspace

# Run clippy with all features
cargo clippy --workspace --all-features
```

## Pull Request Process

1. Ensure all tests pass: `cargo test --workspace`
2. Ensure code is formatted: `cargo fmt`
3. Ensure no clippy warnings: `cargo clippy --workspace`
4. Update documentation if needed
5. Add snapshot tests for new macro features
6. Review and accept any snapshot changes
7. Create a pull request with a clear description

## Project Structure

```
zod-rs/
├── zod-rs/              # Runtime crate (traits, types)
│   └── src/
├── zod-rs-macros/       # Proc-macro crate
│   ├── src/
│   │   ├── parser/      # AST parsing
│   │   ├── ir/          # Intermediate representation
│   │   ├── generator/   # Code generation
│   │   └── codegen/     # Rust impl block generation
│   └── tests/
│       ├── snapshots/   # Snapshot files (.snap)
│       └── *.rs         # Test files
├── zod-rs-cli/          # CLI tool
└── examples/            # Example usage
```

## Adding New Features

### New Attribute

1. Add to `parser/attributes.rs`
2. Update IR types in `ir/` if needed
3. Update generator in `generator/zod/`
4. Add snapshot tests
5. Update documentation

### New Type Mapping

1. Update `parser/type_parser.rs`
2. Update `generator/zod/type_mapper.rs`
3. Add snapshot tests
4. Update type mapping table in README

## Questions?

Feel free to open an issue for questions or discussions about contributing.
