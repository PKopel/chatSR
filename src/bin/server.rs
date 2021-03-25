use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};

extern crate chatsr;
use crate::chatsr::{connection_error, get_string, timestamp};

const SERVER_PORT: usize = 34254;

struct Server {
    listener: TcpListener,
    socket: UdpSocket,
    clients: Arc<Mutex<Vec<TcpStream>>>,
    running: Arc<AtomicBool>,
}

impl Server {
    fn new() -> io::Result<Server> {
        let server_host = get_string("server host address");
        let server_addr: SocketAddr = format!("{}:{}", server_host, SERVER_PORT).parse().unwrap();
        let listener = TcpListener::bind(server_addr)?;
        let socket = UdpSocket::bind(server_addr)?;
        let clients = Arc::new(Mutex::new(vec![]));
        let running = Arc::new(AtomicBool::new(true));
        return Ok(Server {
            listener: listener,
            socket: socket,
            clients: clients,
            running: running,
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
            println!("[{}] Closing connection: {}", timestamp(), addr);
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
                            "[{}] An error occurred while processing UDP message",
                            timestamp()
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
        println!("[{}] Server listening on port 34254", timestamp());
        while self.running.load(Ordering::Relaxed) {
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
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let server = Server::new()?;
    let running = server.running.clone();
    let server = thread::spawn(move || server.run());
    io::stdin().read(&mut [0u8]).unwrap();
    running.store(false, Ordering::Relaxed);
    server.join().unwrap()
}
