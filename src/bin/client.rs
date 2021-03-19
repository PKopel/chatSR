use crossterm::{
    cursor::{Hide, MoveTo, MoveToColumn, Show},
    execute,
    style::Print,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use json;
use std::io::{self, Error, Read, Stdout, Write};
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::str;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

extern crate chatsr;
use crate::chatsr::{get_addr, get_char, get_message, get_string, show_msg, timestamp};

const SERVER_PORT: usize = 34254;
const SMALL_HELP: &str = "[t|u|h|q]";
const HELP: &str = r#"TCP/UDP chat controls:
 - 'q' - quit
 - 'h' - display this help
 - 't' - send message via TCP
 - 'u' - send message via UDP
"#;

struct Client {
    nick: String,
    stream: TcpStream,
    socket: UdpSocket,
    client_addr: SocketAddr,
    server_addr: SocketAddr,
    stdout: Arc<Mutex<Stdout>>,
}

impl Client {
    fn new() -> Result<Client, Error> {
        println!("starting client...");
        let server_addr = get_addr(SERVER_PORT);
        let stream = TcpStream::connect(server_addr)?;
        let addr = stream.local_addr()?;
        let socket = UdpSocket::bind(addr)?;
        let nick = get_string("your nick");
        let stdout = Arc::new(Mutex::new(io::stdout()));
        return Ok(Client {
            nick: nick,
            stream: stream.try_clone()?,
            socket: socket.try_clone()?,
            client_addr: addr,
            server_addr: server_addr,
            stdout: stdout,
        });
    }

    fn start_printer(&self, receiver: Receiver<String>) -> JoinHandle<()> {
        let stdout = self.stdout.clone();
        return thread::spawn(move || loop {
            match receiver.recv() {
                Ok(end) if end.as_str() == "end" => break,
                Ok(msg) => {
                    let _stdout = stdout.lock().unwrap();
                    print!("\r{}press {}", msg, SMALL_HELP);
                }
                _ => break,
            }
        });
    }

    fn start_tcp_receiver(&self, sender: Sender<String>) -> JoinHandle<()> {
        let mut stream = self.stream.try_clone().unwrap();
        return thread::spawn(move || {
            let mut msg_buff = [0 as u8; 1024];
            loop {
                match stream.read(&mut msg_buff) {
                    Ok(size) if size > 3 => {
                        let msg = show_msg(&msg_buff, size);
                        sender.send(msg).unwrap();
                    }
                    Ok(_) => break,
                    Err(err) => {
                        println!("{}", err);
                        break;
                    }
                }
            }
        });
    }

    fn start_udp_receiver(&self, sender: Sender<String>) -> JoinHandle<()> {
        let socket = self.socket.try_clone().unwrap();
        return thread::spawn(move || {
            let mut msg_buff = [0 as u8; 1024];
            loop {
                match socket.recv_from(&mut msg_buff) {
                    Ok((size, _addr)) if size > 3 => {
                        let msg = show_msg(&msg_buff, size);
                        sender.send(msg).unwrap();
                    }
                    Ok(_) => break,
                    Err(err) => {
                        println!("{}", err);
                        break;
                    }
                }
            }
        });
    }

    fn prepare_msg(&self, prompt: &str, sender: Sender<String>) -> String {
        let mut stdout = self.stdout.lock().unwrap();
        execute!(
            stdout,
            EnterAlternateScreen,
            MoveTo(0, 0),
            Show,
            Print(prompt)
        )
        .unwrap();
        let msg_text = get_message();
        let msg_time = timestamp();
        execute!(stdout, LeaveAlternateScreen, MoveToColumn(0), Hide).unwrap();
        let msg = format!("\r[{}]<you>: {}", msg_time, msg_text);
        sender.send(msg).unwrap();
        return json::stringify(json::object! {
            time: msg_time,
            nick: self.nick.as_str(),
            text: msg_text
        });
    }

    fn run(&mut self) -> crossterm::Result<()> {
        let (tx, rx): (Sender<String>, Receiver<String>) = mpsc::channel();
        let printer_handle = self.start_printer(rx);
        let tcp_handle = self.start_tcp_receiver(tx.clone());
        let udp_handle = self.start_udp_receiver(tx.clone());
        loop {
            print!("\rpress {}", SMALL_HELP);
            terminal::enable_raw_mode()?;
            execute!(io::stdout(), Hide)?;
            match get_char() {
                'u' => {
                    terminal::disable_raw_mode()?;
                    let msg = self.prepare_msg("udp message: ", tx.clone());
                    self.socket.send_to(msg.as_bytes(), self.server_addr)?;
                }
                't' => {
                    terminal::disable_raw_mode()?;
                    let msg = self.prepare_msg("tcp message: ", tx.clone());
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
                    tx.send("end".to_string()).unwrap();
                    break;
                }
                _ => {}
            };
        }
        println!("closing client...");
        tcp_handle.join().unwrap();
        udp_handle.join().unwrap();
        printer_handle.join().unwrap();
        Ok(())
    }
}

fn main() -> crossterm::Result<()> {
    print!("{}", HELP);
    let mut client = Client::new()?;
    client.run()
}
