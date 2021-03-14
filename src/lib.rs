use chrono::{Timelike, Utc};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use json;
use std::io::{self, Write};
use std::str;

pub fn get_char() -> char {
    loop {
        if let Ok(Event::Key(KeyEvent {
            code: KeyCode::Char(c),
            ..
        })) = event::read()
        {
            return c;
        }
    }
}

pub fn get_string(prompt: &str) -> String {
    print!("{}: ", prompt);
    let mut nick = String::new();
    io::stdout().flush().unwrap();
    match io::stdin().read_line(&mut nick) {
        Ok(_) => return String::from(nick.trim_end()),
        _ => {
            println!("try again");
            return get_string(prompt);
        }
    }
}

pub fn timestamp() -> String {
    let now = Utc::now();
    let (_pm, hour) = now.hour12();
    format!("{:02}:{:02}:{:02}", hour + 1, now.minute(), now.second())
}

pub fn show_msg(buff: &[u8], size: usize) {
    match str::from_utf8(&buff[..size]) {
        Ok(msg) => match json::parse(msg) {
            Ok(msg_obj) => {
                print!(
                    "\r[{}]<{}>: {}",
                    msg_obj["time"], msg_obj["nick"], msg_obj["text"]
                );
                io::stdout().flush().unwrap()
            }
            _ => return,
        },
        Err(err) => println!("{}", err),
    }
}
