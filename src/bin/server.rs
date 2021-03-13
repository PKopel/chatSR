use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;

fn handle_client(mut stream: TcpStream) {
    let mut data = [0 as u8; 1024]; // using 50 byte buffer
    loop {
        match stream.read(&mut data) {
            Ok(size) => {
                stream.write(&data[0..size]).unwrap();
            }
            Err(_) => {
                println!(
                    "An error occurred, terminating connection with {}",
                    stream.peer_addr().unwrap()
                );
                stream.shutdown(Shutdown::Both).unwrap();
                break;
            }
        }
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:34254").unwrap();
    // accept connections and process them, spawning a new thread for each one
    println!("Server listening on port 34254");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap());
                thread::spawn(move || {
                    // connection succeeded
                    handle_client(stream)
                });
            }
            Err(e) => {
                println!("Error: {}", e);
                /* connection failed */
            }
        }
    }
    // close the socket server
    drop(listener);
}
