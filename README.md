# rust-chat

A simple CLI chat application built with Rust, featuring WebSocket communication and optional TUI interface.

## Features

- WebSocket-based real-time chat
- Command-line interface with readline support
- Optional TUI (Terminal User Interface) for server
- Cross-platform support

## Installation

```bash
cargo build --release
```

## Usage

### Start Server

```bash
# Basic server
cargo run server

# Custom address and port
cargo run server -a 0.0.0.0 -p 8080

# Enable TUI interface
cargo run server --tui
```

### Connect Client

```bash
# Connect with default settings
cargo run client your_name

# Connect to custom server
cargo run client your_name -a 192.168.1.100 -p 8080
```

## Dependencies

- `tokio` - Async runtime
- `axum` - Web framework with WebSocket support
- `clap` - Command line argument parsing
- `rustyline` - Readline functionality
- `term` - Terminal output formatting
- `ratatui` - TUI framework (server)
- `crossterm` - Terminal handling

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Lint
cargo clippy

# Format code
cargo fmt
```