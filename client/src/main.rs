use ::std::net::TcpStream;
use std::io::{self, ErrorKind, Read, Write};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::Duration;

const LOCAL: &str = "127.0.0.1:7070";
const MSG_SIZE: usize = 32;
fn main() {
    // Instantiate stream connection
    let mut client = TcpStream::connect(LOCAL).expect("Stream failed to connect");
    client
        .set_nonblocking(true)
        .expect("Failed to initiate non-blocking");
    let (tx, rx) = mpsc::channel::<String>();

    // Spawn a thread to handle messages from the server
    thread::spawn(move || loop {
        let mut buff = vec![0; MSG_SIZE];
        match client.read_exact(&mut buff) {
            Ok(_) => {
                let msg = buff.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
                let msg = String::from_utf8(msg).expect("Invalid utf8 message");
                println!("{}", msg);
            }
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            Err(_) => {
                println!("Connection with server {} was severed", LOCAL);
                break;
            }
        }

        match rx.try_recv() {
            Ok(msg) => {
                let mut buff = msg.clone().into_bytes();
                buff.resize(MSG_SIZE, 0);
                client.write_all(&buff).expect("Writing too socket failed");
            }
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => break,
        }

        thread::sleep(Duration::from_millis(100));
    });

    // Ask for username

    let mut username = String::new();
    println!("Enter a username:");
    io::stdin()
        .read_line(&mut username)
        .expect("Reading from stdin failed");
    let username = username.trim().to_string();
    if tx.send(username.clone()).is_err() {
        println!("Couldn't send the username: {}", username);
    }

    println!("Connected to the server as: {}", username);

    // Wait for the user input and send messages
    loop {
        let mut buff = String::new();
        io::stdin()
            .read_line(&mut buff)
            .expect("Reading from stdin failed");
        let msg = buff.trim().to_string();
        if msg == ":quit" || tx.send(msg).is_err() {
            println!("Cya :)");
            break;
        }
    }
}
