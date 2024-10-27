use server::SimpleChatServer;

mod server;

#[tokio::main]
async fn main() {
    let server = SimpleChatServer::new("127.0.0.1:8080".to_string());
    server.start().await;
}
