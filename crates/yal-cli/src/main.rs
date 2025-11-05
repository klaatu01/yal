use std::time::Duration;

use clap::{Parser, Subcommand};
use yal_ipc::{YalIPCClient, connect, context, context_with_deadline};

async fn client() -> YalIPCClient {
    let client = connect().await.unwrap_or_else(|_| {
        eprintln!("Failed to connect to yal IPC socket, is yal running?");
        std::process::exit(1);
    });

    client
        .ping(context_with_deadline(Duration::from_secs(2)))
        .await
        .unwrap_or_else(|_| {
            eprintln!("Failed to communicate with yal IPC server, you may need to restart yal");
            std::process::exit(1);
        });

    client
}

#[derive(Subcommand)]
enum MetadataCommands {
    Tree,
}

#[derive(Subcommand)]
enum Commands {
    Metadata {
        #[command(subcommand)]
        command: MetadataCommands,
    },
}

#[derive(Parser)]
#[command(
    name = "yal",
    version = "0.1",
    about = "cli for interacting with yal backend"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, default_value_t = false)]
    pretty_print: bool,
}

fn print_json(value: &serde_json::Value, pretty: bool) {
    if pretty {
        println!("{}", serde_json::to_string_pretty(value).unwrap());
    } else {
        println!("{}", serde_json::to_string(value).unwrap());
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let client = client().await;

    match cli.command {
        Commands::Metadata { command } => match command {
            MetadataCommands::Tree => {
                let response = client.tree(context()).await.unwrap_or_else(|_| {
                    eprintln!("Failed to get metadata tree from yal IPC server");
                    std::process::exit(1);
                });
                print_json(&response, cli.pretty_print);
            }
        },
    }
}
