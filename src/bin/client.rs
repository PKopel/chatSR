use std::io::{self, Error, Read, Write};
use std::net::TcpStream;
use std::net::UdpSocket;
use std::str;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

struct Client {
    nick: String,
    stream: Arc<Mutex<TcpStream>>,
    socket: Arc<UdpSocket>,
}

fn get_nick() -> String {
    print!("your nick: ");
    let mut nick = String::new();
    io::stdout().flush().unwrap();
    match io::stdin().read_line(&mut nick) {
        Ok(_) => return nick,
        _ => {
            println!("try again");
            return get_nick();
        }
    }
}

fn start_client() -> Result<(Client, JoinHandle<()>, JoinHandle<()>), Error> {
    println!("starting client...");
    let stream = Arc::new(Mutex::new(TcpStream::connect("127.0.0.1:34254")?));
    let socket = Arc::new(UdpSocket::bind("127.0.0.1:34254")?);
    let nick = get_nick();
    let client = Client {
        nick: nick,
        stream: stream.clone(),
        socket: socket.clone(),
    };
    let tcp_handle = start_tcp_receiver(stream);
    let udp_handle = start_udp_receiver(socket);
    Ok((client, tcp_handle, udp_handle))
}

fn show_msg(buff: [u8; 1024]) {
    match str::from_utf8(&buff) {
        Ok(text) => println!("{}", text),
        Err(err) => println!("{}", err),
    }
}

fn start_tcp_receiver(stream: Arc<Mutex<TcpStream>>) -> JoinHandle<()> {
    return thread::spawn(move || {
        let mut msg_buff = [0 as u8; 1024];
        loop {
            match stream.lock().unwrap().read(&mut msg_buff) {
                Ok(_) => show_msg(msg_buff),
                Err(err) => {
                    println!("{}", err);
                    break;
                }
            }
        }
    });
}

fn start_udp_receiver(socket: Arc<UdpSocket>) -> JoinHandle<()> {
    return thread::spawn(move || {
        let mut msg_buff = [0 as u8; 1024];
        loop {
            match socket.recv_from(&mut msg_buff) {
                Ok(_) => show_msg(msg_buff),
                Err(err) => {
                    println!("{}", err);
                    break;
                }
            }
        }
    });
}

fn send_tcp(msg_text: &str, client: Client) -> std::io::Result<()> {
    Ok(())
}

fn send_udp(msg_text: &str, client: Client) -> std::io::Result<()> {
    Ok(())
}

fn main() -> std::io::Result<()> {
    let client = match start_client() {
        Ok(client) => client,
        Err(err) => return Err(err),
    };
    println!("bye!");
    Ok(())
}
