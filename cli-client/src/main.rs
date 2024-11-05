use std::{env, io, process::exit};

use clap::Parser;
use client::Client;

mod client;

/// Struct to represent command line args
#[derive(Clone, Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Username of the user. This should be unique
    #[arg(short, long)]
    username: String,

    /// Simple Chat Server host
    #[arg(short = 'o', long)]
    host: Option<String>,

    /// Simple Chat Server port number
    #[arg(short, long)]
    port: Option<String>,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Arg parsing. We give priority to command line arguments if provided.
    let args = Args::parse();

    let mut host = env::var("SIMPLE_CHAT_SERVER_HOST").unwrap_or_default();
    let mut port = env::var("SIMPLE_CHAT_SERVER_PORT").unwrap_or_default();

    host = args.host.unwrap_or(host);
    port = args.port.unwrap_or(port);
    let username = args.username;

    let client = Client::new(host, port, username);

    // Return the appripriate code based on error.
    match client.start().await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("ERROR: {}", e);
            exit(1);
        }
    };
    Ok(())
}
