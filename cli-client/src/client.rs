/*
    Pending work:
    1. Appropriate error handling. Remove use of expect/unwrap and return proper error.
    2. Make functions more testable by using generic types.
    3. Return proper return code as per Unix standards.
    4. Make use of logging instead of println.
    5. Use of struct with serde serialization instead of raw text messages.
    6. Use of enums instead of constants for cohesiveness.
    7. Handle Ctrl + C signal to stop the server.
*/

use std::io;

use common::{extract_parts, messages};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::TcpStream,
    sync::mpsc,
};

pub struct Client {
    pub host: String,
    pub port: String,
    pub username: String,
}

static LEAVE: &str = "leave";
static SEND: &str = "send";

impl Client {
    pub fn new(host: String, port: String, username: String) -> Self {
        Client {
            host,
            port,
            username,
        }
    }
    pub async fn start(&self) -> io::Result<()> {
        // Connect to the server
        let mut stream = TcpStream::connect(format!("{}:{}", self.host, self.port)).await?;

        // Disable Nagle's algorithm to send data immediately
        stream.set_nodelay(true).unwrap();
        let (reader, writer) = stream.split();

        // Create a buffered writer and reader for network communication
        let mut writer = BufWriter::new(writer);
        let mut reader = BufReader::new(reader);
        let mut line = String::new(); // Buffer to store received data

        // Join the default room using supplied username
        writer
            .write_all(format!("<{}> {}\n", messages::JOIN_USER, self.username).as_bytes())
            .await
            .expect("ERROR: Unable to write to server");

        writer.flush().await.expect("ERROR: Unable to flush writer");

        reader
            .read_line(&mut line)
            .await
            .expect("ERROR: Unable to read from server");

        let (command, _, _) = extract_parts(&line);
        if command == messages::DUPLICATE_USER {
            eprintln!(
                "ERROR: Username already in use. Please try again with a different username."
            );
            return Ok(());
        }

        let (tx, mut rx) = mpsc::channel::<String>(1000);
        let mut input = String::new();

        // Create a buffered writer and reader for stdin/stdout communication
        let mut console_reader = BufReader::new(tokio::io::stdin());

        line.clear();
        loop {
            tokio::select! {
                _result = console_reader.read_line(&mut input) => {

                    // Handle sending here
                    input = input.trim().to_string();
                    if input.is_empty() || input == "\n" {
                        continue;
                    }

                    let user_input = input.split(" ").collect::<Vec<&str>>();

                    // Extract command from user input
                    let command = user_input[0].to_lowercase();


                    if command == LEAVE {
                        writer.write_all(format!("<{}> {}\n", messages::LEAVE_USER, self.username).as_bytes()).await.expect("Unable to write to server");
                        writer.flush().await.expect("Unable to write to server");
                        return Ok(());
                    } else if command == SEND {
                        let usr_msg = &input[command.len() + 1..];
                        let usr_msg = format!("<{}> {} {}\n", messages::USER_MSG, self.username, usr_msg);
                        writer.write_all(usr_msg.as_bytes()).await.expect("Unable to write to server");
                        writer.flush().await.expect("Unable to write to server");
                    }

                    input.clear();
             }

                result = reader.read_line(&mut line) => {
                    if result.expect("ERROR: Unable to read from server") == 0 {
                        eprintln!("Server closed the connection.");
                        return Ok(());
                    }
                    let (command, username, data) = extract_parts(&line);
                    if command == messages::USER_MSG {
                        tx.send(format!("{}> {}", username, data)).await.expect("ERROR: Unable to communicate internally");
                    } else if command == messages::INVALID_CMD {
                        eprintln!("ERROR: Invalid command received from server");
                    }
                    line.clear();
                }

                result = rx.recv() => {
                    println!("{}", result.unwrap());
                }

            }
        }
    }
}
