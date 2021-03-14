use crossterm::{
    cursor::{Hide, MoveTo, MoveToColumn, Show},
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::Print,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use json;
use rand::Rng;
use std::io::{self, Error, Read, Write};
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::str;
use std::thread::{self, JoinHandle};

const SERVER_ADDR: &str = "127.0.0.1:34254";
const HELP: &str = r#"TCP/UDP chat controls:
 - 'q' - quit
 - 'h' - display this help
 - 't' - send message via TCP
 - 'u' - send message via UDP
"#;

fn get_nick() -> String {
    print!("your nick: ");
    let mut nick = String::new();
    io::stdout().flush().unwrap();
    match io::stdin().read_line(&mut nick) {
        Ok(_) => return String::from(nick.trim_end()),
        _ => {
            println!("try again");
            return get_nick();
        }
    }
}

fn show_msg(buff: &[u8], size: usize) {
    match str::from_utf8(&buff[..size]) {
        Ok(msg) => match json::parse(msg) {
            Ok(msg_obj) => {
                print!("\r<{}>: {}", msg_obj["nick"], msg_obj["text"]);
                io::stdout().flush().unwrap()
            }
            _ => return,
        },
        Err(err) => println!("{}", err),
    }
}

fn read_char() -> io::Result<char> {
    loop {
        if let Ok(Event::Key(KeyEvent {
            code: KeyCode::Char(c),
            ..
        })) = event::read()
        {
            return Ok(c);
        }
    }
}

struct Client {
    nick: String,
    stream: TcpStream,
    socket: UdpSocket,
    addr: SocketAddr,
}

impl Client {
    fn new() -> Result<Client, Error> {
        println!("starting client...");
        let mut rng = rand::thread_rng();
        let addr = SocketAddr::from(([127, 0, 0, 1], rng.gen_range(35000..40000)));
        let stream = TcpStream::connect(SERVER_ADDR)?;
        let socket = UdpSocket::bind(addr)?;
        let nick = get_nick();
        let client = Client {
            nick: nick,
            stream: stream.try_clone()?,
            socket: socket.try_clone()?,
            addr: addr,
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
        let mut msg_text = String::new();
        io::stdin().read_line(&mut msg_text).unwrap();
        execute!(io::stdout(), LeaveAlternateScreen, MoveToColumn(0), Hide).unwrap();
        print!("\r<{}>: {}", self.nick, msg_text);
        json::stringify(json::object! {
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
            }
        }));
    }

    fn run(&mut self) -> crossterm::Result<()> {
        loop {
            terminal::enable_raw_mode()?;
            execute!(io::stdout(), Hide)?;
            match read_char()? {
                'u' => {
                    terminal::disable_raw_mode()?;
                    let msg = self.prepare_msg();
                    self.socket.send_to(msg.as_bytes(), SERVER_ADDR)?;
                }
                't' => {
                    terminal::disable_raw_mode()?;
                    let msg = self.prepare_msg();
                    self.stream.write(msg.as_bytes())?;
                }
                'h' => {
                    terminal::disable_raw_mode()?;
                    print!("{}", HELP)
                }
                'q' => {
                    terminal::disable_raw_mode()?;
                    execute!(io::stdout(), MoveToColumn(0), Show)?;
                    self.stream.write(b"end")?;
                    self.socket.send_to(b"end", self.addr)?;
                    break;
                }
                _ => {}
            };
        }
        println!("closing client...");
        Ok(())
    }
}

fn main() -> io::Result<()> {
    print!("{}", HELP);
    let mut client = match Client::new() {
        Ok(client) => client,
        Err(err) => return Err(err),
    };
    let tcp_handle = client.start_tcp_receiver()?;
    let udp_handle = client.start_udp_receiver()?;
    client.run().unwrap();
    tcp_handle.join().unwrap();
    udp_handle.join().unwrap();
    Ok(())
}
