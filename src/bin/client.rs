use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::Print,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use json;
use std::io::{self, Error, Read, Write};
use std::net::UdpSocket;
use std::net::{Shutdown, TcpStream};
use std::str;
use std::thread::{self, JoinHandle};

struct Client {
    nick: String,
    stream: TcpStream,
    socket: UdpSocket,
}

const MENU: &str = r#"TCP/UDP chat
Controls:
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

fn start_client() -> Result<(Client, JoinHandle<()>, JoinHandle<()>), Error> {
    println!("starting client...\n{}", MENU);
    let stream = TcpStream::connect("127.0.0.1:34254")?;
    let socket = UdpSocket::bind("127.0.0.1:34255")?;
    let nick = get_nick();
    let client = Client {
        nick: nick,
        stream: stream.try_clone()?,
        socket: socket.try_clone()?,
    };
    let tcp_handle = start_tcp_receiver(stream);
    let udp_handle = start_udp_receiver(socket);
    Ok((client, tcp_handle, udp_handle))
}

fn show_msg(buff: &[u8], size: usize) {
    match str::from_utf8(&buff[..size]) {
        Ok(msg) => {
            let msg_obj = json::parse(msg).unwrap();
            print!("\r{}> {}", msg_obj["nick"], msg_obj["text"]);
            io::stdout().flush().unwrap()
        }
        Err(err) => println!("{}", err),
    }
}

fn start_tcp_receiver(mut stream: TcpStream) -> JoinHandle<()> {
    return thread::spawn(move || {
        let mut msg_buff = [0 as u8; 1024];
        loop {
            match stream.read(&mut msg_buff) {
                Ok(size) => show_msg(&msg_buff, size),
                Err(err) => {
                    println!("{}", err);
                    break;
                }
            }
        }
    });
}

fn start_udp_receiver(socket: UdpSocket) -> JoinHandle<()> {
    return thread::spawn(move || {
        let mut msg_buff = [0 as u8; 1024];
        loop {
            match socket.recv_from(&mut msg_buff) {
                Ok((size, _addr)) => show_msg(&msg_buff, size),
                Err(err) => {
                    println!("{}", err);
                    break;
                }
            }
        }
    });
}

fn prepare_msg(nick: &str) -> String {
    execute!(io::stdout(), EnterAlternateScreen, Print("your message: ")).unwrap();
    let mut msg_text = String::new();
    io::stdin().read_line(&mut msg_text).unwrap();
    execute!(io::stdout(), LeaveAlternateScreen).unwrap();
    json::stringify(json::object! {
        nick: nick,
        text: msg_text
    })
}

fn read_char() -> std::io::Result<char> {
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

fn user_loop(mut client: Client) -> crossterm::Result<()> {
    let nick = client.nick;
    loop {
        terminal::enable_raw_mode()?;
        match read_char()? {
            'u' => {
                terminal::disable_raw_mode()?;
                let msg = prepare_msg(&nick);
                client.socket.send_to(msg.as_bytes(), "127.0.0.1:34254")?;
            }
            't' => {
                terminal::disable_raw_mode()?;
                let msg = prepare_msg(&nick);
                client.stream.write(msg.as_bytes())?;
            }
            'q' => break,
            _ => {}
        };
    }
    client.stream.shutdown(Shutdown::Both)?;
    println!("bye!");
    terminal::disable_raw_mode()
}

fn main() -> std::io::Result<()> {
    let (client, tcp_handle, udp_handle) = match start_client() {
        Ok(client) => client,
        Err(err) => return Err(err),
    };
    user_loop(client).unwrap();
    tcp_handle.join().unwrap();
    udp_handle.join().unwrap();
    Ok(())
}
