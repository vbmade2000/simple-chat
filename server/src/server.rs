use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use common::{extract_parts, messages};
use tokio::{
    io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter},
    net::{tcp::WriteHalf, TcpListener, TcpStream},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Mutex,
    },
};

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

    // BufWriter<WriteHalf<'_>>
    // Handles joining of a new user. Returns true/false based on user joining success.
    async fn handle_join_command<T>(
        users: Arc<Mutex<HashMap<String, Sender<String>>>>,
        writer: &mut T,
        line: &mut String,
        my_username: &mut String,
        data: &str,
        client_address: SocketAddr,
        tx: Sender<String>,
    ) -> bool
    where
        T: AsyncWrite + std::marker::Unpin,
    {
        if users.lock().await.contains_key(data) {
            let _ = writer
                .write_all(format!("<{}>\n", messages::DUPLICATE_USER).as_bytes())
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

        // Send ACK for user joining.
        let _ = writer
            .write_all(format!("<{}> {}\n", messages::USER_JOINED, &data).as_bytes())
            .await;
        let _ = writer.flush().await;

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
        sender_username: &str,
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

        println!("- Sending message to all users: {:?}", &line);
        for (username, sender) in users.lock().await.iter() {
            if username != my_username {
                // println!("- Sending message to user: {:?}", &username);
                println!("User: {}", &sender_username);
                println!("data: {}", &data);
                let usr_msg = format!("<{}> {} {}\n", messages::USER_MSG, sender_username, data);
                println!("Sending message: {:?}", &usr_msg);
                let _ = sender.send(usr_msg).await;
            }
        }
    }

    async fn handle_connection(
        mut stream: TcpStream,
        client_address: SocketAddr,
        users: Arc<Mutex<HashMap<String, Sender<String>>>>,
    ) {
        println!("Received connection from: {:?}", &client_address);
        stream
            .set_nodelay(true)
            .expect("ERROR: Unable to set TCP_NODELAY");
        let (reader, writer) = stream.split();
        let mut reader = BufReader::new(reader);
        let mut writer = BufWriter::new(writer);

        // Send welcome message to the client
        // let _ = writer
        //     .write_all(format!("{} <server>\n", messages::WELCOME_MSG).as_bytes())
        //     .await;
        // let _ = writer.flush().await;

        let (tx, mut rx) = channel::<String>(1000);

        let mut line = String::new();
        let mut my_username = String::new();

        /*
           IMP:
           reader.read_line() method expects \n at the end of the message to mark it as line. Without it, it will wait indefinitely.
        */
        println!("Starting loop");
        loop {
            tokio::select! {
                result = reader.read_line(&mut line) => {
                    match result {
                        Ok(n) => {
                            println!("Line received from {:?}: {:?}", &client_address, &line);
                            if n == 0 {
                                println!("Connection closed by client {:?}", &client_address);
                                break;
                            }

                            if line.trim() == "\n" {
                                println!("Received new line character. Ignoring.");
                                continue;
                            }

                            println!("Received network message from {:?}: {:?}", &client_address, &line);

                            let (command, username, data) = extract_parts(&line);

                            // Handle joining of the user
                            if command == messages::JOIN_USER {
                                Self::handle_join_command(users.clone(), &mut writer, &mut line, &mut my_username, &data, client_address, tx.clone()).await;
                            }
                            // Handle leaving of the user
                            else if command == messages::LEAVE_USER {
                                Self::handle_leave_command(users.clone(), &mut rx, &data, &client_address).await;
                            }
                            // Handle user messages
                            else if command ==  messages::USER_MSG {
                                Self::handle_user_messages(users.clone(), &my_username, &mut writer, &mut line, &data, &client_address, &username).await;
                            } else {
                                println!("Invalid command received: {:?}", &line);
                                let _ = writer.write_all(format!("{}", messages::INVALID_CMD).as_bytes()).await;
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

#[cfg(test)]
mod tests {

    use tokio::io::{self, AsyncReadExt};

    use super::*;

    #[tokio::test]
    async fn test_handle_leave_command() {
        // Input
        let data: &str = "testuser";
        let (tx, mut rx) = channel::<String>(1);
        let mut users_map = HashMap::new();
        users_map.insert(data.to_string(), tx);

        let client_address = SocketAddr::new("127.0.0.1".parse().unwrap(), 9875);
        let users = Arc::new(Mutex::new(users_map));

        // Call
        SimpleChatServer::handle_leave_command(users.clone(), &mut rx, data, &client_address).await;
        let users = users.clone();

        // Assertion
        assert!(rx.is_closed());
        assert_eq!(users.lock().await.len(), 0);
    }

    // async fn test_handle_join_command_user_exists() {
    //     // async fn handle_join_command(
    //     //     users: Arc<Mutex<HashMap<String, Sender<String>>>>,
    //     //     writer: &mut BufWriter<WriteHalf<'_>>,
    //     //     line: &mut String,
    //     //     my_username: &mut String,
    //     //     data: &str,
    //     //     client_address: SocketAddr,
    //     //     tx: Sender<String>,
    //     // )
    //     use std::io::Cursor;

    //     // Input
    //     let existing_username: &str = "testuser";
    //     let (tx, _rx) = channel::<String>(1);

    //     let mut users_map = HashMap::new();
    //     users_map.insert(existing_username.to_string(), tx.clone());

    //     let client_address = SocketAddr::new("127.0.0.1".parse().unwrap(), 9875);

    //     // let buffer = Vec::new();
    //     let (client, mut server) = io::duplex(64);
    //     let (reader, writer) = tokio::io::split(server);
    //     let mut buf_writer = BufWriter::new(writer);
    //     let mut line = String::from("test line from user");
    //     let mut my_username = String::from("myuser");
    //     let users =  Arc::new(Mutex::new(users_map));

    //     // Call

    //     SimpleChatServer::handle_join_command(users, &mut buf_writer, &mut line, &mut my_username, existing_username, client_address, tx).await;
    //     let mut output = String::new();
    //     writer.read_to_string(&mut output).await.unwrap();

    //     // Assertion
    //     assert!(line.is_empty());
    //     assert_eq!(output, "Hello");
    // }
}
