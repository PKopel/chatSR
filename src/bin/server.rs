use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::str;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

extern crate chatsr;
use crate::chatsr::{get_string, timestamp};

const SERVER_PORT: usize = 34254;

struct Server {
    listener: TcpListener,
    socket: UdpSocket,
    clients: Arc<Mutex<Vec<TcpStream>>>,
}

impl Server {
    fn new() -> io::Result<Server> {
        let server_host = get_string("server host address");
        let server_addr: SocketAddr = format!("{}:{}", server_host, SERVER_PORT).parse().unwrap();
        let listener = TcpListener::bind(server_addr)?;
        let socket = UdpSocket::bind(server_addr)?;
        let clients = Arc::new(Mutex::new(vec![]));
        Ok(Server {
            listener: listener,
            socket: socket,
            clients: clients,
        })
    }

    fn start_client_stream(&self, mut stream: TcpStream) -> JoinHandle<()> {
        let clients = Arc::clone(&self.clients);
        thread::spawn(move || {
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
                                        client.write(&data[..size]).unwrap();
                                    }
                                }
                            }
                        };
                    }
                    Err(_) => {
                        println!(
                            "[{}] An error occurred, terminating connection with {}",
                            timestamp(),
                            addr
                        );
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
            println!("[{}] Closing connection: {}", timestamp(), addr);
        })
    }

    fn start_client_socket(&self) -> io::Result<JoinHandle<()>> {
        let clients = Arc::clone(&self.clients);
        let socket = self.socket.try_clone()?;
        Ok(thread::spawn(move || {
            let mut data = [0 as u8; 1024];
            loop {
                match socket.recv_from(&mut data) {
                    Ok((size, addr)) => {
                        match str::from_utf8(&data[..size]) {
                            Ok("end") => break,
                            _ => {
                                for client in clients.lock().unwrap().iter() {
                                    let client_addr = client.peer_addr().unwrap();
                                    if client_addr != addr {
                                        socket.send_to(&data[..size], client_addr).unwrap();
                                    }
                                }
                            }
                        };
                    }
                    Err(_) => {
                        println!(
                            "[{}] An error occurred while processing UDP message",
                            timestamp()
                        );
                        break;
                    }
                }
            }
        }))
    }

    fn run(&self) -> io::Result<()> {
        let mut handles: Vec<JoinHandle<()>> = vec![];
        handles.push(self.start_client_socket()?);
        println!("[{}] Server listening on port 34254", timestamp());
        loop {
            let stream = self.listener.accept();
            match stream {
                Ok((stream, addr)) => {
                    println!("[{}] New connection: {}", timestamp(), addr);
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
