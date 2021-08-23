use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;

const LOCAL: &str = "127.0.0.1:7070";
const MSG_SIZE: usize = 32;

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
                        tx.send(msg).expect("failed to send msg to rx");
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
        }

        sleep();
    }
}
