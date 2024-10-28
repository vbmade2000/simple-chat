use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{tcp::WriteHalf, TcpListener, TcpStream},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Mutex,
    },
};

/// A simple chat server
/*
    Protocol:
    1. <JOIN><SPACE><USERNAME>
       Join the room with the given username. Username has to be unique.
    2. <LEAVE>
       Leave the room.
    3. <MSG><SPACE><MESSAGE TEXT>
       Send regular message to all users in the room.
    4. <ERROR><SPACE><ERROR-TEXT>

*/
pub struct SimpleChatServer {
    /// The address the server is listening on
    pub address: Arc<String>,
    /// The users connected to the server
    pub users: Arc<Mutex<HashMap<String, Sender<String>>>>,
}

impl SimpleChatServer {
    /// Create a new server instance
    pub fn new(address: String) -> SimpleChatServer {
        let address = Arc::new(address);

        SimpleChatServer {
            address,
            users: Arc::new(Mutex::new(HashMap::new())),
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
            let users = self.users.clone();

            tokio::spawn({
                async move {
                    Self::handle_connection(stream, client_address, users).await;
                }
            });
        }
    }

    /// Stop the server
    pub fn stop(&self) {
        println!("Stopping server on {}", self.address);
    }

    // Handles joining of a new user. Returns true/false based on user joining success.
    async fn handle_join_command(
        users: Arc<Mutex<HashMap<String, Sender<String>>>>,
        writer: &mut BufWriter<WriteHalf<'_>>,
        line: &mut String,
        my_username: &mut String,
        data: &str,
        client_address: SocketAddr,
        tx: Sender<String>,
    ) -> bool {
        if users.lock().await.contains_key(data) {
            let _ = writer
                .write_all("Username already taken. Please try another username.\n".as_bytes())
                .await;
            let _ = writer.flush().await;
            println!("Duplicate username received: {}", data);
            line.clear(); // Clear the buffer for safety.
            return false;
        }
        users.lock().await.insert(data.to_string(), tx.clone());
        *my_username = data.to_string();
        println!(
            "User {:?} from {:?} has joined the chat",
            &data, &client_address
        );
        line.clear(); // Clear the buffer for safety.
        true
    }

    // Handle leaving of the existing user
    async fn handle_leave_command(
        users: Arc<Mutex<HashMap<String, Sender<String>>>>,
        rx: &mut Receiver<String>,
        data: &str,
        client_address: &SocketAddr,
    ) {
        users.lock().await.remove(data);
        println!(
            "User {:?} from {:?} has left the chat",
            &data, client_address
        );
        rx.close();
    }

    // Handle user messages
    async fn handle_user_messages(
        users: Arc<Mutex<HashMap<String, Sender<String>>>>,
        my_username: &str,
        writer: &mut BufWriter<WriteHalf<'_>>,
        line: &mut String,
        data: &str,
        client_address: &SocketAddr,
    ) {
        // User should not be able to send message if not joined.
        if my_username.is_empty() {
            println!(
                "{:?} is trying to send message without joining the chat",
                client_address
            );
            let _ = writer
                .write_all("Please join the chat first.\n".as_bytes())
                .await;
            let _ = writer.flush().await;
            line.clear(); // Clear the buffer for safety.
            return;
        }

        println!("Sending message to all users: {:?}", &line);
        for (username, sender) in users.lock().await.iter() {
            println!("- Sending message to user: {:?}", &username);
            if username != my_username {
                let _ = sender
                    .send(format!("<{:?}> {:?}", my_username, data.to_string()))
                    .await;
            }
        }
    }

    async fn handle_connection(
        mut stream: TcpStream,
        client_address: SocketAddr,
        users: Arc<Mutex<HashMap<String, Sender<String>>>>,
    ) {
        println!("Received connection from: {:?}", &client_address);

        let (reader, writer) = stream.split();
        let mut reader = BufReader::new(reader);
        let mut writer = BufWriter::new(writer);

        // Send welcome message to the client
        let _ = writer
            .write_all("Welcome to the Simple Chat System\n".as_bytes())
            .await;
        let _ = writer.flush().await;
        let (tx, mut rx) = channel::<String>(1000);

        let mut line = String::new();
        let mut my_username = String::new();

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

                            // Split the received line to get command and rest of the text.
                            // Check Protocol section at the top of the file.
                            let lines = line.split(" ").collect::<Vec<&str>>();
                            let command = lines[0].trim().to_lowercase();
                            let data = line[command.len()..].trim().to_string();

                            // Handle joining of the user
                            if command == "<join>" {
                                Self::handle_join_command(users.clone(), &mut writer, &mut line, &mut my_username, &data, client_address, tx.clone()).await;
                            }
                            // Handle leaving of the user
                            else if command == "<leave>" {
                                Self::handle_leave_command(users.clone(), &mut rx, &data, &client_address).await;
                            }
                            // Handle user messages
                            else if command ==  "<msg>" {
                                Self::handle_user_messages(users.clone(), &my_username, &mut writer, &mut line, &data, &client_address).await;
                            } else {
                                println!("Invalid command received: {:?}", &line);
                                let _ = writer.write_all("Invalid command received. Please try again.\n".as_bytes()).await;
                                let _ = writer.flush().await;
                            }

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
                        Some(msg) => {
                            println!("Received channel message from other users: {:?}", &msg);
                            // Received message from other user(s). Send it to this user if this user is not the sender.
                            let _ = writer.write_all(msg.as_bytes()).await;
                            let _ = writer.flush().await;
                        }
                        None => {
                            println!("Channel closed");
                            break;
                        }
                    }
                }
            }
        }
    }
}
