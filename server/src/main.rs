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
        if let Ok((mut socket, addr)) = server.accept() {
            // Store the client, with username
            clients.insert(
                addr.to_string(),
                Client {
                    stream: socket.try_clone().expect("Failed to clone client"),
                    username: format!("user_{}", &clients.len()).to_string(),
                },
            );

            let tx = tx.clone();

            // Log the new connection
            println!(
                "Client {} connected with username: {}",
                addr,
                &clients.get(&addr.to_string()).unwrap().username,
            );

            let username = String::from(&clients.get(&addr.to_string()).unwrap().username);
            thread::spawn(move || loop {
                let mut buff = vec![0; MSG_SIZE];
                match socket.read_exact(&mut buff) {
                    Ok(_) => {
                        // Transform the message
                        let msg = buff.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
                        let msg = String::from_utf8(msg).expect("Invalid utf8 message");
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
            clients = clients
                .into_iter()
                .filter_map(|mut client| {
                    let mut buff = msg.clone().into_bytes();
                    buff.resize(MSG_SIZE, 0);
                    client.1.stream.write_all(&buff).map(|_| client).ok()
                })
                .collect::<HashMap<_, _>>();
            messages.push(msg);
        }

        if count >= 100 {
            if Path::new(LOGS_PATH).exists() {
                let mut file = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open(Path::new(LOGS_PATH))
                    .expect(format!("Couldn't open {}", LOGS_PATH).as_str());

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
                // Write the pyramid string to `file`, returns `io::Result<()>`
                for msg in messages {
                    if let Err(e) = writeln!(file, "{}", msg) {
                        eprintln!("Couldn't write to: {}", e);
                    }
                }
            }
            count = 0;
            messages = Vec::new();
        }

        count += 1;
        sleep();
    }
}
