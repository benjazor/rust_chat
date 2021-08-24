use regex::Regex;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::mpsc;
use std::thread;

const LOCAL: &str = "127.0.0.1:7070";
const MSG_SIZE: usize = 32;
const LOGS_PATH: &str = "logs.txt";

fn sleep() {
    thread::sleep(::std::time::Duration::from_millis(100));
}

struct Client {
    stream: TcpStream,
    username: String,
}
fn main() {
    // Initialize the server
    let server = TcpListener::bind(LOCAL).expect("Listener failed to bind");
    server
        .set_nonblocking(true)
        .expect("Failed to initialize non-blocking");
    println!("Starting the server at: {}", LOCAL);

    let mut clients: HashMap<String, Client> = HashMap::new();
    let mut messages: Vec<String> = Vec::new();
    let mut count: u8 = 0;

    let (tx, rx) = mpsc::channel::<String>();
    loop {
        // Happens for every connection
        if let Ok((mut socket, addr)) = server.accept() {
            // Store the client, with username
            clients.insert(
                addr.to_string(),
                Client {
                    stream: socket.try_clone().expect("Failed to clone client"),
                    username: String::new(), //format!("user_{}", &clients.len()).to_string(),
                },
            );

            // Clone the sender so every client can send messages to the receiver
            let tx = tx.clone();

            // Log the new connection
            println!("Client {} connected to the server", addr);

            // Store the username to format messages in the thread
            let mut username = String::new();

            // Make a thread to read each user messages
            thread::spawn(move || loop {
                let mut buff = vec![0; MSG_SIZE];
                match socket.read_exact(&mut buff) {
                    Ok(_) => {
                        // Transform the message
                        let msg = buff.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
                        let msg = String::from_utf8(msg).expect("Invalid utf8 message");

                        // Set the username (First message from client)
                        if username.eq("") {
                            let username_regex =
                                Regex::new(r"[a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?").unwrap();
                            if username_regex.is_match(&msg) {
                                username = msg;
                                println!("New user: {} ({})", username, addr);
                                if let Err(err) = tx.send(format!("#{}|{}", username, addr)) {
                                    panic!("Failed to send msg to rx: {}", err);
                                }
                            }
                            continue;
                        }
                        // Regular message
                        let msg = format!("{} : {}", username, msg);

                        // Log the message and the sender
                        println!("{}", msg);
                        tx.send(msg).expect("Failed to send msg to rx");
                    }
                    Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
                    Err(_) => {
                        println!("Closing connection with: {}", &addr);
                        break;
                    }
                }
                sleep();
            });
        }

        if let Ok(msg) = rx.try_recv() {
            // Add the message to the logs
            messages.push(msg.clone());

            // New user
            if msg[0..1].eq("#") {
                let username = &msg[1..msg.find('|').expect("Not a valid message")];
                let address = &msg[msg.find('|').expect("Not a valid message") + 1..];
                clients
                    .get_mut(address)
                    .expect("Client does not exists")
                    .username = username.to_string();
                continue;
            }

            // Format message to send to the clients
            let mut buff = msg.clone().into_bytes();
            buff.resize(MSG_SIZE, 0);

            let username = &msg[..msg.find(' ').expect("Not a valid message")];

            clients = clients
                .into_iter()
                .filter_map(|mut client| {
                    // Send the message to everyone except to the sender
                    if !username.eq(&client.1.username) {
                        client.1.stream.write_all(&buff).map(|_| client).ok()
                    } else {
                        client.1.stream.write_all(&[]).map(|_| client).ok()
                    }
                })
                .collect::<HashMap<_, _>>();
        }

        // Every 100 iteration save the messages to the logs file
        if count >= 100 {
            if Path::new(LOGS_PATH).exists() {
                // Open the existing file
                let mut file = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open(Path::new(LOGS_PATH))
                    .expect(format!("Couldn't open {}", LOGS_PATH).as_str());

                // Append the new messages at the end
                for msg in messages {
                    if let Err(e) = writeln!(file, "{}", msg) {
                        eprintln!("Couldn't write to file: {}", e);
                    }
                }
            } else {
                // Open a file in write-only mode, returns `io::Result<File>`
                let mut file = match File::create(Path::new(LOGS_PATH)) {
                    Err(why) => panic!("Couldn't create {}: {}", LOGS_PATH, why),
                    Ok(file) => file,
                };
                // Write the message string to `file`, returns `io::Result<()>`
                for msg in messages {
                    if let Err(e) = writeln!(file, "{}", msg) {
                        eprintln!("Couldn't write to: {}", e);
                    }
                }
            }
            // Reset the count and messages values
            count = 0;
            messages = Vec::new();
        }

        count += 1;
        sleep();
    }
}
