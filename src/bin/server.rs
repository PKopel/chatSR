use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::str;
use std::sync::{Arc, Mutex};
use std::thread;

fn handle_client(mut stream: TcpStream, clients: Arc<Mutex<Vec<TcpStream>>>) {
    let mut data = [0 as u8; 1024];
    let addr = stream.peer_addr().unwrap();
    loop {
        match stream.read(&mut data) {
            Ok(size) => {
                match str::from_utf8(&data[..size]) {
                    Ok("end") => break,
                    _ => {
                        for mut client in clients.lock().unwrap().iter() {
                            if client.peer_addr().unwrap() != addr {
                                client.write(&data[0..size]).unwrap();
                            }
                        }
                    }
                };
            }
            Err(_) => {
                println!("An error occurred, terminating connection with {}", addr);
                break;
            }
        }
    }
    stream.shutdown(Shutdown::Both).unwrap();
    let mut clients = clients.lock().unwrap();
    clients
        .iter()
        .position(|n| n.peer_addr().unwrap() == addr)
        .map(|e| clients.remove(e));
    println!("Closing connection: {}", addr);
}

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:34254")?;
    let clients = Arc::new(Mutex::new(vec![]));
    println!("Server listening on port 34254");
    loop {
        let stream = listener.accept();
        match stream {
            Ok((stream, addr)) => {
                println!("New connection: {}", addr);
                clients.lock().unwrap().push(stream.try_clone().unwrap());
                let clients_clone = Arc::clone(&clients);
                thread::spawn(move || handle_client(stream, clients_clone));
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
        }
    }
    drop(listener);
    Ok(())
}
