// #![feature(io_uring)]

use ws_uring::client::{Client, State};

use std::io::{self};

// TODO nagles algorithm, probably expose to caller.

fn main() -> io::Result<()> {
    let mut client = Client::new("http://www.example.com".to_owned()).unwrap();

    client.issue_connect().unwrap();
    let mut read_buffer = vec![0u8; 1024 * 16];
    let begin = std::time::Instant::now();
    loop {
        match client.step() {
            Ok(State::Read(0)) => {
                client.issue_connect().unwrap();
            }
            Ok(State::Read(bytes)) => {
                println!("{:?}", String::from_utf8_lossy(&read_buffer[..bytes]));
                client.issue_read(&mut read_buffer).unwrap();
            }
            Ok(State::Write) => {
                println!("wants to write");
            }
            Ok(State::Connect) => {
                println!("in connect");
                let plaintext =
                    b"GET / HTTP/1.1\r\nHost: www.example.com\r\nConnection: close\r\n\r\n"
                        .to_owned();
                client.issue_write(&plaintext).unwrap();
                client.issue_read(&mut read_buffer).unwrap();
            }
            Ok(State::Idle) => {}
            Err(e) => {
                println!("{:?}", e.to_string());
                break;
            }
        }
    }
    let end = std::time::Instant::now();
    println!("Connection took: {:?}", end - begin);

    Ok(())
}
