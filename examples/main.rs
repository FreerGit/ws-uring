use ws_uring::client::{Client, State};

use std::io::{self};

fn main() -> io::Result<()> {
    let mut rb = vec![0u8; 1024 * 1000];
    let mut client = Client::new("http://www.google.com".to_owned()).unwrap();

    client.issue_connect().unwrap();
    let begin = std::time::Instant::now();
    loop {
        match client.step(&mut rb) {
            Ok(State::Read(0)) => {
                println!("Read with 0 bytes, this _most_ likely means a graceful disconnect");
                // client.issue_connect().unwrap();
            }
            Ok(State::Read(bytes)) => {
                println!("{:?}", String::from_utf8_lossy(&rb[..bytes]));
                client.issue_read(&mut rb).unwrap();
            }
            Ok(State::Connect) => {
                println!("in connect");
                let plaintext =
                    b"GET / HTTP/1.1\r\nHost: www.google.com\r\nConnection: close\r\n\r\n"
                        .to_owned();
                client.issue_write(&plaintext).unwrap();
                client.issue_read(&mut rb).unwrap();
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
