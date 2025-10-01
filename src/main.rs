extern crate reqwest;
extern crate rustyline;
extern crate term;

use clap::{Parser, Subcommand};

mod client;
mod server;
mod shared;

#[derive(Parser)]
#[command(name = "chat")]
#[command(about = "A simple CLI chat tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start chat server
    Server {
        /// Listen address (default: 127.0.0.1)
        #[arg(short, long, default_value = "127.0.0.1")]
        address: String,

        /// Listen port (default: 12345)
        #[arg(short, long, default_value_t = 12345)]
        port: u16,

        /// Enable TUI interface
        #[arg(long, default_value_t = false)]
        tui: bool,
    },
    /// Connect to chat server
    Client {
        /// Server address (default: 127.0.0.1)
        #[arg(short, long, default_value = "127.0.0.1")]
        address: String,

        /// Server port (default: 12345)
        #[arg(short, long, default_value_t = 12345)]
        port: u16,

        /// Your chat name (optional, random if not provided)
        #[arg(long)]
        name: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Server { address, port, tui } => {
            if let Err(e) = server::run_server(&address, port, tui).await {
                eprintln!("Server error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Client {
            address,
            port,
            name,
        } => {
            client::run_client(&address, port, name).await;
        }
    }
}
