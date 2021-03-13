use crossterm::{
    cursor::{Hide, MoveToColumn, Show},
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::Print,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use json;
use std::io::{self, Error, Read, Write};
use std::net::TcpStream;
use std::net::UdpSocket;
use std::str;
use std::thread::{self, JoinHandle};

const MENU: &str = r#"TCP/UDP chat
controls:
 - 'q' - quit
 - 't' - send TCP message
 - 'u' - send UDP message
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
                print!("{}> {}", msg_obj["nick"], msg_obj["text"]);
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
}

impl Client {
    fn new() -> Result<Client, Error> {
        let stream = TcpStream::connect("127.0.0.1:34254")?;
        let socket = UdpSocket::bind("127.0.0.1:34255")?;
        let nick = get_nick();
        let client = Client {
            nick: nick,
            stream: stream.try_clone()?,
            socket: socket.try_clone()?,
        };
        Ok(client)
    }

    fn prepare_msg(&self) -> String {
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            MoveToColumn(0),
            Show,
            Print("your message: ")
        )
        .unwrap();
        let mut msg_text = String::new();
        io::stdin().read_line(&mut msg_text).unwrap();
        execute!(io::stdout(), LeaveAlternateScreen, MoveToColumn(0), Hide).unwrap();
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
            println!("TCP stream closed")
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
                    self.socket.send_to(msg.as_bytes(), "127.0.0.1:34254")?;
                }
                't' => {
                    terminal::disable_raw_mode()?;
                    let msg = self.prepare_msg();
                    self.stream.write(msg.as_bytes())?;
                }
                'q' => {
                    terminal::disable_raw_mode()?;
                    execute!(io::stdout(), MoveToColumn(0), Show)?;
                    self.stream.write(b"end")?;
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
    print!("{}", MENU);
    println!("starting client...");
    let mut client = match Client::new() {
        Ok(client) => client,
        Err(err) => return Err(err),
    };
    let tcp_handle = client.start_tcp_receiver()?;
    //let udp_handle = client.start_udp_receiver()?;
    client.run().unwrap();
    tcp_handle.join().unwrap();
    //udp_handle.join().unwrap();
    Ok(())
}
