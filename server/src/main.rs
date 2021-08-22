use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;

const LOCAL: &str = "127.0.0.1:7070";
const MSG_SIZE: usize = 32;

fn sleep() {
    thread::sleep(::std::time::Duration::from_millis(100));
}
fn main() {
    // Initialize the server
    let server = TcpListener::bind(LOCAL).expect("Listener failed to bind");
    server
        .set_nonblocking(true)
        .expect("Failed to initialize non-blocking");
    println!("Starting the server at: {}", LOCAL);

    let mut clients = vec![];
    let mut usernames = HashMap::new();
    let (tx, rx) = mpsc::channel::<String>();
    loop {
        if let Ok((mut socket, addr)) = server.accept() {
            usernames.insert(
                addr.to_string(),
                format!("user{}", &usernames.len()).to_string(),
            );
            println!(
                "Client {} connected with username: {}",
                addr,
                usernames.get(&addr.to_string()).unwrap()
            );

            let tx = tx.clone();
            clients.push(socket.try_clone().expect("Failed to clone client"));

            let names = usernames.clone();
            thread::spawn(move || loop {
                let mut buff = vec![0; MSG_SIZE];
                match socket.read_exact(&mut buff) {
                    Ok(_) => {
                        // Transform the message
                        let msg = buff.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
                        let msg = String::from_utf8(msg).expect("Invalid utf8 message");
                        let msg = format!("{} : {}", &names.get(&addr.to_string()).unwrap(), msg);

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
                    // Resize the buffer so it doesn't crash (may cut some of the message data)
                    let mut buff = msg.clone().into_bytes();
                    buff.resize(MSG_SIZE, 0);

                    client.write_all(&buff).map(|_| client).ok()
                })
                .collect::<Vec<_>>();
        }

        sleep();
    }
}
