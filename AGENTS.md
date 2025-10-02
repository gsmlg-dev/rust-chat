# Rust Chat Agent Guidelines

## Build/Lint/Test Commands
- Build: `cargo build`
- Run server: `cargo run server [-a ADDRESS] [-p PORT]`
- Run client: `cargo run client <name> [-a ADDRESS] [-p PORT]`
- Test: `cargo test`
- Lint: `cargo clippy`
- Format: `cargo fmt`
- Single test: `cargo test test_name`

## Code Style Guidelines
- Use Rust 2024 edition (as specified in Cargo.toml)
- Follow standard Rust formatting (cargo fmt)
- Use `extern crate` declarations for external dependencies (seen in main.rs)
- Prefer `unwrap()` for simple error handling in this codebase
- Use async/await with tokio runtime
- Name variables in snake_case
- Use descriptive error messages with color output via term crate
- Use serde for JSON serialization/deserialization with proper error handling
- Use thiserror for custom error types with proper error propagation
- Use UUID v4 for unique identifiers with `uuid::Uuid::new_v4()`
- Use Arc<Mutex<T>> for shared state across async tasks
- Use tokio::sync::mpsc for async channel communication
- Write comprehensive doc comments with examples for public functions