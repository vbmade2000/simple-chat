use std::env;

use clap::Parser;
use common::{extract_parts, messages};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::TcpStream,
    sync::mpsc,
};

static LEAVE: &str = "leave";
static SEND: &str = "send";

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
async fn main() {
    // Arg parsing. We give priority to command line arguments if provided.
    let args = Args::parse();

    let mut host = env::var("SIMPLE_CHAT_SERVER_HOST").unwrap_or_default();
    let mut port = env::var("SIMPLE_CHAT_SERVER_PORT").unwrap_or_default();

    host = args.host.unwrap_or(host);
    port = args.port.unwrap_or(port);
    let username = args.username;

    // Connect to the server
    let mut stream = TcpStream::connect(format!("{}:{}", host, port))
        .await
        .expect("ERROR: Unable to connect to server");

    // Disable Nagle's algorithm to send data immediately
    stream.set_nodelay(true).unwrap();
    let (reader, writer) = stream.split();

    // Create a buffered writer and reader for network communication
    let mut writer = BufWriter::new(writer);
    let mut reader = BufReader::new(reader);
    let mut line = String::new(); // Buffer to store received data

    // Join the default room using supplied username
    writer
        .write_all(format!("<{}> {}\n", messages::JOIN_USER, username).as_bytes())
        .await
        .expect("ERROR: Unable to write to server");

    writer.flush().await.expect("ERROR: Unable to flush writer");

    println!("Waiting for response from server");
    reader
        .read_line(&mut line)
        .await
        .expect("ERROR: Unable to read from server");

    println!("Received response from server");
    let (command, _, _) = extract_parts(&line);
    if command == messages::DUPLICATE_USER {
        println!("ERROR: Username already in use. Please try again with a different username.");
        return;
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
                    writer.write_all(format!("<{}> {}\n", messages::LEAVE_USER, username).as_bytes()).await.expect("Unable to write to server");
                    writer.flush().await.expect("Unable to write to server");
                    return;
                } else if command == SEND {
                    let usr_msg = &input[command.len() + 1..];
                    let usr_msg = format!("<{}> {} {}\n", messages::USER_MSG, username, usr_msg);
                    writer.write_all(usr_msg.as_bytes()).await.expect("Unable to write to server");
                    writer.flush().await.expect("Unable to write to server");
                }

                input.clear();
         }

            result = reader.read_line(&mut line) => {
                if result.expect("ERROR: Unable to read from server") == 0 {
                    println!("Server closed the connection.");
                    return;
                }
                let (command, username, data) = extract_parts(&line);
                if command == messages::USER_MSG {
                    tx.send(format!("{}> {}", username, data)).await.expect("ERROR: Unable to communicate internally");
                } else if command == messages::INVALID_CMD {
                    println!("ERROR: Invalid command received from server");
                }
                line.clear();
            }

            result = rx.recv() => {
                println!("{}", result.unwrap());
            }

        }
    }
}
