#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::TcpStream,
        process::{Command, Stdio},
        thread::sleep,
        time::Duration,
    };

    // #[test]
    fn test_user_exists() {
        // Start the server
        let mut server = Command::new("../target/release/server")
            .stdout(Stdio::null())
            .spawn()
            .expect("Failed to start server");

        // Wait until the server is ready
        let mut attempts = 0;
        while attempts < 10 {
            if TcpStream::connect("127.0.0.1:8090").is_ok() {
                println!("Server is ready.");
                break;
            }
            attempts += 1;
            sleep(Duration::from_secs(1));
        }

        // Run the client
        let mut client1 = Command::new("../target/release/cli-client")
            .args(["--username", "user1"])
            .args(["--host", "127.0.0.1"])
            .args(["--port", "8090"])
            .spawn()
            .expect("Failed to start client1");

        let mut client2 = Command::new("../target/release/cli-client")
            .args(["--username", "user1"])
            .args(["--host", "127.0.0.1"])
            .args(["--port", "8090"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to start client2");

        // Capture the output of client2
        let mut client2_stdout = String::new();
        if let Some(stdout) = client2.stdout.take() {
            let mut reader = BufReader::new(stdout);
            reader
                .read_to_string(&mut client2_stdout)
                .expect("Failed to read stdout");
        }

        // Assert the output
        assert_eq!(
            client2_stdout.trim(),
            "ERROR: Username already in use. Please try again with a different username."
        );

        server.kill().expect("Failed to kill server");
        client1.kill().expect("Failed to kill client1");
        // server.wait().expect("Failed to wait for server");
        client2.kill().expect("Failed to kill client2");
    }

    // Attempted but not working as expected.
    // #[test]
    fn _test_messaging() {
        // Start the server
        println!("Starting server");
        let mut server = Command::new("../target/release/server")
            .args(["--port", "8091"])
            .stdout(Stdio::null())
            .spawn()
            .expect("Failed to start server");

        // Wait until the server is ready
        let mut attempts = 0;
        while attempts < 10 {
            if TcpStream::connect("127.0.0.1:8091").is_ok() {
                println!("Server is ready.");
                break;
            }
            attempts += 1;
            sleep(Duration::from_secs(1));
        }

        // Run the client
        let mut client1 = Command::new("../target/release/cli-client")
            .args(["--username", "user1"])
            .args(["--host", "127.0.0.1"])
            .args(["--port", "8091"])
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()
            .expect("Failed to start client1");

        let mut client2 = Command::new("../target/release/cli-client")
            .args(["--username", "user2"])
            .args(["--host", "127.0.0.1"])
            .args(["--port", "8091"])
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()
            .expect("Failed to start client2");

        let mut client3 = Command::new("../target/release/cli-client")
            .args(["--username", "user3"])
            .args(["--host", "127.0.0.1"])
            .args(["--port", "8091"])
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()
            .expect("Failed to start client3");

        // Write to Stdins of the clients
        println!("Sending messages");
        let client1_stdin = client1
            .stdin
            .as_mut()
            .expect("Failed to open client1 stdin");
        client1_stdin
            .write_all(b"send Hey Everyone. How are you doing?\n")
            .expect("Failed to write to client1 stdin");

        let client2_stdin = client2
            .stdin
            .as_mut()
            .expect("Failed to open client2 stdin");
        client2_stdin
            .write_all(b"send Hey User1. I am fine. Thanks\n")
            .expect("Failed to write to client2 stdin");

        let client3_stdin = client3
            .stdin
            .as_mut()
            .expect("Failed to open client3 stdin");
        client3_stdin
            .write_all(b"send Hola User1. I am doing awesome.\n")
            .expect("Failed to write to client3 stdin");

        client1_stdin
            .write_all(b"send Cool!\n")
            .expect("Failed to write to client1 stdin");

        let mut client1_stdout: Vec<String> = Vec::new();
        let mut client2_stdout: Vec<String> = Vec::new();
        let mut client3_stdout: Vec<String> = Vec::new();

        println!("=> Reading stdouts1");

        if let Some(stdout) = client1.stdout.take() {
            let reader = BufReader::new(stdout);
            for (index, line) in reader.lines().enumerate() {
                let line = line.expect("Failed to read line from client1 stdout");
                client1_stdout.push(line);

                // We know that there are two lines so we need to break after the second line.
                if index == 1 {
                    break;
                }
            }
        }

        println!("=> Reading stdouts2");

        if let Some(stdout) = client2.stdout.take() {
            let reader = BufReader::new(stdout);
            for (index, line) in reader.lines().enumerate() {
                let line = line.expect("Failed to read line from client2 stdout");
                // println!("{}", &line);
                client2_stdout.push(line);

                // We know that there are two lines so we need to break after the second line.
                if index == 1 {
                    break;
                }
            }
        }

        if let Some(stdout) = client3.stdout.take() {
            let reader = BufReader::new(stdout);
            for (index, line) in reader.lines().enumerate() {
                let line = line.expect("Failed to read line from client3 stdout");
                println!("{}", &line);
                client3_stdout.push(line);

                // We know that there are two lines so we need to break after the second line.
                if index == 1 {
                    break;
                }
            }
        }

        // Assert the output for client1
        assert_eq!(client1_stdout[0], "user2> Hey User1. I am fine. Thanks");
        assert_eq!(client1_stdout[1], "user3> Hola User1. I am doing awesome.");

        // Assert the output for client2
        assert_eq!(client2_stdout[0], "user3> Hola User1. I am doing awesome.");
        assert_eq!(client2_stdout[1], "user1> Hey Everyone. How are you doing?");

        server.kill().expect("Failed to kill server");
        client1.kill().expect("Failed to kill client1");
        client2.kill().expect("Failed to kill client2");
        // client3.kill().expect("Failed to kill client3");
    }
}
