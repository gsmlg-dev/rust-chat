extern crate reqwest;
extern crate rustyline;
extern crate term;

use clap::{Parser, Subcommand};

mod client;
mod server;

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
        /// Your chat name
        name: String,

        /// Server address (default: 127.0.0.1)
        #[arg(short, long, default_value = "127.0.0.1")]
        address: String,

        /// Server port (default: 12345)
        #[arg(short, long, default_value_t = 12345)]
        port: u16,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Server { address, port, tui } => {
            server::run_server(&address, port, tui).await;
        }
        Commands::Client {
            name,
            address,
            port,
        } => {
            client::run_client(&name, &address, port).await;
        }
    }
}
