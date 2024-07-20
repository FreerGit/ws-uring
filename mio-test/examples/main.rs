// #![feature(io_uring)]

use ws_uring::client::{self, Client, ConnectState};

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
    // TODO this blocks.
    // let addr = "example.com:443".to_socket_addrs()?.next().unwrap();
    // let os_addr: OsSocketAddr = addr.into();
    // // libc::sockaddr::into(addr);
    // let outer_b = std::time::Instant::now();

    // // SocketAddrV4::

    // // TODO user spec.
    // let mut ring = IoUring::new(32).unwrap();

    // let mut sq = IoUring::submission(&mut ring);

    // let sockfd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, libc::IPPROTO_TCP) };
    // // io_uring::
    // let prep_connect =
    //     opcode::Connect::new(Fd(sockfd.as_raw_fd()), os_addr.as_ptr(), os_addr.len());

    // unsafe { sq.push(&prep_connect.build()).unwrap() };

    // let outer_e = std::time::Instant::now();
    // println!("{:?} ", outer_e - outer_b);

    let addr = ws_uring::client::dns_lookup("example.com", 80).unwrap();
    let mut client = Client::new(addr, true);
    loop {
        let begin = std::time::Instant::now();
        let state = client.connect();
        let end = std::time::Instant::now();
        // println!("{:?} {:?}", end - begin, state);
        if let Ok(ConnectState::Connected) = state {
            break;
        }
    }
    Ok(())
}

// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     // Assume you have a connected socket
//     let mut socket = TcpStream::connect("example.com:443")?;
//     socket.set_nonblocking(true)?;

//     // Set up rustls
//     let mut root_store = rustls::RootCertStore::empty();
//     root_store.add_server_trust_anchors(&TLS_SERVER_ROOTS);
//     let config = ClientConfig::builder()
//         .with_safe_defaults()
//         .with_root_certificates(root_store)
//         .with_no_client_auth();
//     let config = Arc::new(config);
//     let server_name = ServerName::try_from("example.com")?;
//     let mut conn = ClientConnection::new(config, server_name)?;

//     // Create an IoUring instance
//     let mut ring = IoUring::new(256)?;

//     // Buffer to read/write data
//     let mut read_buf = [0; 4096];
//     let mut write_buf = Vec::new();

//     // Perform the TLS handshake
//     loop {
//         // Read data from socket and feed it to the TLS connection
//         let read_e = opcode::Read::new(types::Fd(socket.as_raw_fd()), read_buf.as_mut_ptr(), read_buf.len() as _)
//             .build()
//             .user_data(0x01);
//         unsafe {
//             ring.submission().push(&read_e).expect("submission queue is full");
//         }
//         ring.submit_and_wait(1)?;

//         let cqe = ring.completion().next().expect("completion queue is empty");
//         if cqe.result() > 0 {
//             let n = cqe.result() as usize;
//             let mut tls_stream = StreamOwned::new(conn, socket);
//             tls_stream.read_tls(&mut &read_buf[..n])?;
//             tls_stream.process_new_packets()?;
//         }

//         // Write data from the TLS connection to the socket
//         if conn.wants_write() {
//             write_buf.clear();
//             conn.write_tls(&mut write_buf)?;
//             let write_e = opcode::Write::new(types::Fd(socket.as_raw_fd()), write_buf.as_ptr(), write_buf.len() as _)
//                 .build()
//                 .user_data(0x02);
//             unsafe {
//                 ring.submission().push(&write_e).expect("submission queue is full");
//             }
//             ring.submit_and_wait(1)?;

//             let cqe = ring.completion().next().expect("completion queue is empty");
//             if cqe.result() < 0 {
//                 return Err(std::io::Error::from_raw_os_error(-cqe.result()).into());
//             }
//         }

//         // Check if the handshake is complete
//         if !conn.is_handshaking() {
//             break;
//         }
//     }

//     // Now you have a TLS connection, you can proceed with normal I/O operations
//     // Example: Reading data using io_uring
//     let mut buf = [0; 4096];
//     let read_e = opcode::Read::new(types::Fd(socket.as_raw_fd()), buf.as_mut_ptr(), buf.len() as _)
//         .build()
//         .user_data(0x03);
//     unsafe {
//         ring.submission().push(&read_e).expect("submission queue is full");
//     }
//     ring.submit_and_wait(1)?;

//     let cqe = ring.completion().next().expect("completion queue is empty");
//     if cqe.user_data() == 0x03 {
//         let n = cqe.result();
//         if n > 0 {
//             println!("Read {} bytes", n);
//         } else {
//             eprintln!("Read error: {}", n);
//         }
//     }

//     // Example: Writing data using io_uring
//     let write_data = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";
//     let write_e = opcode::Write::new(types::Fd(socket.as_raw_fd()), write_data.as_ptr(), write_data.len() as _)
//         .build()
//         .user_data(0x04);
//     unsafe {
//         ring.submission().push(&write_e).expect("submission queue is full");
//     }
//     ring.submit_and_wait(1)?;

//     let cqe = ring.completion().next().expect("completion queue is empty");
//     if cqe.user_data() == 0x04 {
//         let n = cqe.result();
//         if n > 0 {
//             println!("Wrote {} bytes", n);
//         } else {
//             eprintln!("Write error: {}", n);
//         }
//     }

//     Ok(())
// }
