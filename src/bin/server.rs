use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

extern crate chatsr;
use crate::chatsr::{connection_error, timestamp};

const SERVER_PORT: usize = 34254;
const SERVER_IP: &str = "0.0.0.0";

struct Server {
    listener: TcpListener,
    socket: UdpSocket,
    clients: Arc<Mutex<Vec<TcpStream>>>,
}

impl Server {
    fn new() -> io::Result<Server> {
        let server_addr: SocketAddr = format!("{SERVER_IP}:{SERVER_PORT}").parse().unwrap();
        let listener = TcpListener::bind(server_addr)?;
        let socket = UdpSocket::bind(server_addr)?;
        let clients = Arc::new(Mutex::new(vec![]));
        return Ok(Server {
            listener: listener,
            socket: socket,
            clients: clients,
        });
    }

    fn start_client_stream(&self, mut stream: TcpStream) -> JoinHandle<()> {
        let clients = Arc::clone(&self.clients);
        return thread::spawn(move || {
            let mut data = [0 as u8; 1024];
            let addr = stream.peer_addr().unwrap();
            loop {
                match stream.read(&mut data) {
                    Ok(size) if size > 3 => {
                        for mut client in clients.lock().unwrap().iter() {
                            if let Ok(client_addr) = client.peer_addr() {
                                if client_addr != addr {
                                    match client.write(&data[..size]) {
                                        Err(_) => connection_error(client_addr),
                                        _ => continue,
                                    };
                                }
                            } else {
                                break;
                            }
                        }
                    }
                    Ok(_) => break,
                    Err(_) => {
                        connection_error(addr);
                        break;
                    }
                }
            }
            stream.shutdown(Shutdown::Both).unwrap();
            let mut clients = clients.lock().unwrap();
            clients.retain(|client| match client.peer_addr() {
                Ok(client_addr) if client_addr != addr => true,
                _ => false,
            });
            println!("[{time}] Closing connection: {addr}", time = timestamp());
        });
    }

    fn start_client_socket(&self) -> JoinHandle<()> {
        let clients = Arc::clone(&self.clients);
        let socket = self.socket.try_clone().unwrap();
        return thread::spawn(move || {
            let mut data = [0 as u8; 1024];
            loop {
                match socket.recv_from(&mut data) {
                    Ok((size, addr)) if size > 3 => {
                        for client in clients.lock().unwrap().iter() {
                            if let Ok(client_addr) = client.peer_addr() {
                                if client_addr != addr {
                                    match socket.send_to(&data[..size], client_addr) {
                                        Err(_) => connection_error(client_addr),
                                        _ => continue,
                                    };
                                }
                            } else {
                                break;
                            }
                        }
                    }
                    Ok(_) => break,
                    Err(_) => {
                        println!(
                            "[{time}] An error occurred while processing UDP message",
                            time = timestamp()
                        );
                        break;
                    }
                }
            }
        });
    }

    fn run(&self) -> io::Result<()> {
        let mut handles: Vec<JoinHandle<()>> = vec![];
        handles.push(self.start_client_socket());
        println!("[{time}] Server listening on port 34254", time = timestamp());
        loop {
            let stream = self.listener.accept();
            match stream {
                Ok((stream, addr)) => {
                    println!("[{time}] New connection: {addr}", time = timestamp());
                    self.clients
                        .lock()
                        .unwrap()
                        .push(stream.try_clone().unwrap());
                    handles.push(self.start_client_stream(stream));
                }
                Err(err) => {
                    handles.into_iter().for_each(|h| {
                        h.join().unwrap();
                    });
                    return Err(err);
                }
            }
        }
    }
}

fn main() -> io::Result<()> {
    let server = Server::new()?;
    server.run()
}
