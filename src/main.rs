use server::SimpleChatServer;

mod server;

#[tokio::main]
async fn main() {
    let server = SimpleChatServer::new("127.0.0.1:8080".to_string());
    server.start().await;
}

//   // Websocket handshake. Avoid panic as it may take some time due to stack unwiding, calling destructors etc.
//   let ws_stream = match tokio_tungstenite::accept_async(stream).await {
//     Ok(ws_stream) => ws_stream,
//     Err(e) => {
//         eprintln!("Error during websocket handshake: {}", e);
//         return;
//     }
// };
