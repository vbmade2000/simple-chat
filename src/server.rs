use std::{net::SocketAddr, sync::Arc};

use futures_util::{StreamExt, TryStreamExt};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{TcpListener, TcpStream},
    stream,
    sync::broadcast::{self, Receiver, Sender},
};
// use tokio_tungstenite::accept_async;

/// A simple chat server
pub struct SimpleChatServer {
    /// The address the server is listening on
    pub address: Arc<String>,
    /// The users connected to the server
    pub users: Vec<String>,
    /// Broadcast channel sender. This along with the rx (Receiver) is used to broadcast messages to all connected users.
    tx: Sender<String>,
    /// Broadcast channel receiver.
    rx: Receiver<String>,
}

impl SimpleChatServer {
    /// Create a new server instance
    pub fn new(address: String) -> SimpleChatServer {
        let address = Arc::new(address);

        let (tx, rx) = broadcast::channel::<String>(1000);
        SimpleChatServer {
            address,
            users: Vec::new(),
            tx,
            rx,
        }
    }

    /// Start the server
    pub async fn start(&self) {
        let address = self.address.clone();
        let listener = TcpListener::bind(address.to_string())
            .await
            .unwrap_or_else(|_| panic!("ERROR: Failed to bind to {}", &self.address));

        println!("Starting server on {}", self.address);

        while let Ok((stream, client_address)) = listener.accept().await {
            let tx = self.tx.clone();

            tokio::spawn({
                async move {
                    Self::handle_connection(stream, client_address, tx).await;
                }
            });
        }
    }

    /// Stop the server
    pub fn stop(&self) {
        println!("Stopping server on {}", self.address);
    }

    pub async fn handle_connection(
        mut stream: TcpStream,
        client_address: SocketAddr,
        tx: Sender<String>,
    ) {
        println!("Received connection from: {:?}", &client_address);

        let (reader, writer) = stream.split();
        let mut reader = BufReader::new(reader);
        let mut writer = BufWriter::new(writer);
        let _ = writer
            .write_all("Welcome to the Simple Chat System\n".as_bytes())
            .await;
        let _ = writer.flush().await;

        let mut line = String::new();
        let mut rx = tx.subscribe();

        loop {
            tokio::select! {
                result = reader.read_line(&mut line) => {
                    match result {
                        Ok(n) => {
                            if n == 0 {
                                println!("Connection closed by client {:?}", &client_address);
                                break;
                            }
                            println!("Received network message from {:?}: {:?}", &client_address, &line);
                            let _ = tx.send(line.clone());
                            println!("Sent message to all users: {:?}", &line);
                            line.clear(); // Clear the buffer for safety.

                        }
                        Err(e) => {
                            println!("ERROR: Failed to read from socket; error={:?}", e);
                            break;
                        }
                    }
                }
                result = rx.recv() => {
                    match result {
                        Ok(msg) => {
                            println!("Received channel message from other users: {:?}", &msg);
                            // Received message from other user(s). Send it to this user if this user is not the sender.
                            let _ = writer.write_all(msg.as_bytes()).await;
                            let _ = writer.flush().await;
                        }
                        Err(e) => {
                            println!("ERROR: Failed to read from socket; error={:?}", e);
                            break;
                        }
                    }
                }
            }
        }
    }
}
