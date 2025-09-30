# Rust Chat Agent Guidelines

## Build/Lint/Test Commands
- Build: `cargo build`
- Run: `cargo run [name]`
- Test: `cargo test`
- Lint: `cargo clippy`
- Format: `cargo fmt`
- Single test: `cargo test test_name`

## Code Style Guidelines
- Use Rust 2018 edition
- Follow standard Rust formatting (cargo fmt)
- Use `extern crate` declarations for external dependencies
- Prefer `unwrap()` for simple error handling in this codebase
- Use async/await with tokio runtime
- Name variables in snake_case
- Use descriptive error messages with color output via term crate