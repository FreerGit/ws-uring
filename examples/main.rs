// #![feature(io_uring)]

use ws_uring::client::{self, Client, ConnectState, ReadState, WriteState};

use io_uring::squeue::Entry;
use io_uring::types::Fd;
use io_uring::{opcode, types, IoUring, SubmissionQueue};
use os_socketaddr::OsSocketAddr;
use rustls::StreamOwned;
use std::io::{self, Read, Write};
use std::net::{SocketAddrV4, ToSocketAddrs};
use std::os::fd::AsRawFd;
use std::sync::Arc;
use std::time::SystemTime;

const BUFFER_SIZE: usize = 4096;

// TODO nagles algorithm, probably expose to caller.

fn main() -> io::Result<()> {
    let mut client = Client::new("https://www.example.com".to_owned()).unwrap();
    {
        let begin = std::time::Instant::now();
        loop {
            let state = client.connect();
            if let Ok(ConnectState::Connected) = state {
                break;
            }
        }
        let end = std::time::Instant::now();
        println!("Connecting took {:?}", end - begin);
    }

    // {
    //     let begin = std::time::Instant::now();
    //     let mut plaintext =
    //         b"GET / HTTP/1.1\r\nHost: www.example.com\r\nConnection: close\r\n\r\n".to_owned();
    //     loop {
    //         match client.write(&mut plaintext) {
    //             Ok(WriteState::Disconnected) => todo!(),
    //             Ok(WriteState::WantsWrite) => continue,
    //             Ok(WriteState::Written) => {
    //                 break;
    //             }
    //             Err(e) => println!("{:?}", e.to_string()),
    //         }
    //     }
    //     let end = std::time::Instant::now();
    //     println!("Writing took {:?}", end - begin);
    // }

    // {
    //     let begin = std::time::Instant::now();
    //     let mut rb = vec![0u8; 1024 * 8];
    //     loop {
    //         match client.read(&mut rb) {
    //             Ok(ReadState::Disconnected) => todo!(),
    //             Ok(ReadState::WantsRead) => continue,
    //             Ok(ReadState::Read(n)) => {
    //                 println!("{:?} {:?}", n, String::from_utf8_lossy(&rb[..n]));
    //                 break;
    //             }
    //             Err(e) => println!("{:?}", e.to_string()),
    //         }
    //     }
    //     let end = std::time::Instant::now();
    //     println!("Reading took {:?}", end - begin);
    // }

    // {
    //     let mut rb = vec![0u8; 1024 * 8];
    //     let begin = std::time::Instant::now();
    //     for _ in 0..5 {
    //         match client.read(&mut rb).unwrap() {
    //             ReadState::Idle => continue,
    //             ReadState::Disconnected => todo!(),
    //             ReadState::WantsRead => continue,
    //             ReadState::Read(n) => {
    //                 println!(
    //                     "{:?} {:?} {:?}",
    //                     end - begin,
    //                     n,
    //                     String::from_utf8_lossy(&rb[..n])
    //                 );
    //                 break;
    //             }
    //         }
    //     }
    //     let end = std::time::Instant::now();
    //     println!("{:?}", end - begin);
    // }

    Ok(())
}
