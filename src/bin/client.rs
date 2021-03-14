use crossterm::{
    cursor::{Hide, MoveTo, MoveToColumn, Show},
    execute,
    style::Print,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use json;
use std::io::{self, Error, Read, Write};
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::str;
use std::thread::{self, JoinHandle};

extern crate chatsr;
use crate::chatsr::{get_char, get_string, show_msg, timestamp};

const SERVER_PORT: usize = 34254;
const SMALL_HELP: &str = "[t|u|m|h|q]";
const HELP: &str = r#"TCP/UDP chat controls:
 - 'q' - quit
 - 'h' - display this help
 - 't' - send message via TCP
 - 'u' - send message via UDP
 - 'm' - send message via UDP multicast
"#;

struct Client {
    nick: String,
    stream: TcpStream,
    socket: UdpSocket,
    client_addr: SocketAddr,
    server_addr: SocketAddr,
}

impl Client {
    fn new() -> Result<Client, Error> {
        println!("starting client...");
        let server_host = get_string("server host address");
        let server_addr = format!("{}:{}", server_host, SERVER_PORT).parse().unwrap();
        let stream = TcpStream::connect(server_addr)?;
        let addr = stream.local_addr()?;
        let socket = UdpSocket::bind(addr)?;
        let nick = get_string("your nick");
        let client = Client {
            nick: nick,
            stream: stream.try_clone()?,
            socket: socket.try_clone()?,
            client_addr: addr,
            server_addr: server_addr,
        };
        Ok(client)
    }

    fn prepare_msg(&self) -> String {
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            MoveTo(0, 0),
            Show,
            Print("your message: ")
        )
        .unwrap();
        let msg_time = timestamp();
        let mut msg_text = String::new();
        io::stdin().read_line(&mut msg_text).unwrap();
        execute!(io::stdout(), LeaveAlternateScreen, MoveToColumn(0), Hide).unwrap();
        print!("\r[{}]<you>: {}", msg_time, msg_text);
        json::stringify(json::object! {
            time: msg_time,
            nick: self.nick.as_str(),
            text: msg_text
        })
    }

    fn start_tcp_receiver(&self) -> io::Result<JoinHandle<()>> {
        let mut stream = self.stream.try_clone()?;
        return Ok(thread::spawn(move || {
            let mut msg_buff = [0 as u8; 1024];
            loop {
                match stream.read(&mut msg_buff) {
                    Ok(size) if size > 3 => show_msg(&msg_buff, size),
                    Ok(_) => break,
                    Err(err) => {
                        println!("{}", err);
                        break;
                    }
                }
                print!("\rpress {}", SMALL_HELP);
                io::stdout().flush().unwrap();
            }
        }));
    }

    fn start_udp_receiver(&self) -> io::Result<JoinHandle<()>> {
        let socket = self.socket.try_clone()?;
        return Ok(thread::spawn(move || {
            let mut msg_buff = [0 as u8; 1024];
            loop {
                match socket.recv_from(&mut msg_buff) {
                    Ok((size, _addr)) if size > 3 => show_msg(&msg_buff, size),
                    Ok(_) => break,
                    Err(err) => {
                        println!("{}", err);
                        break;
                    }
                }
                print!("\rpress {}", SMALL_HELP);
                io::stdout().flush().unwrap();
            }
        }));
    }

    fn run(&mut self) -> crossterm::Result<()> {
        let tcp_handle = self.start_tcp_receiver()?;
        let udp_handle = self.start_udp_receiver()?;
        loop {
            print!("\rpress {}", SMALL_HELP);
            terminal::enable_raw_mode()?;
            execute!(io::stdout(), Hide)?;
            match get_char() {
                'm' => {
                    terminal::disable_raw_mode()?;
                    let msg = self.prepare_msg();
                    self.socket.send_to(msg.as_bytes(), self.server_addr)?;
                }
                'u' => {
                    terminal::disable_raw_mode()?;
                    let msg = self.prepare_msg();
                    self.socket.send_to(msg.as_bytes(), self.server_addr)?;
                }
                't' => {
                    terminal::disable_raw_mode()?;
                    let msg = self.prepare_msg();
                    self.stream.write(msg.as_bytes())?;
                }
                'h' => {
                    terminal::disable_raw_mode()?;
                    print!("\r{}", HELP)
                }
                'q' => {
                    terminal::disable_raw_mode()?;
                    execute!(io::stdout(), MoveToColumn(0), Show)?;
                    self.stream.write(b"end")?;
                    self.socket.send_to(b"end", self.client_addr)?;
                    break;
                }
                _ => {}
            };
        }
        println!("closing client...");
        tcp_handle.join().unwrap();
        udp_handle.join().unwrap();
        Ok(())
    }
}

fn main() -> crossterm::Result<()> {
    print!("{}", HELP);
    let mut client = Client::new()?;
    client.run()
}
