use clap::{command, Parser};
use server::SimpleChatServer;

mod server;

/// Struct to represent command line args
#[derive(Clone, Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1")]
    ip: Option<String>,

    #[arg(short, long, default_value = "8090")]
    port: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let ip = args.ip.unwrap();
    let port = args.port.unwrap();

    let server = SimpleChatServer::new(format!("{}:{}", ip, port).to_string());
    server.start().await;
}
