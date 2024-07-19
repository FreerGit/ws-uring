// #![feature(io_uring)]

use io_uring::squeue::Entry;
use io_uring::types::Fd;
use io_uring::{opcode, types, IoUring, SubmissionQueue};
use mio::net::TcpStream;
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
    let addr = "example.com:443".to_socket_addrs()?.next().unwrap();
    let os_addr: OsSocketAddr = addr.into();
    // libc::sockaddr::into(addr);
    let outer_b = std::time::Instant::now();

    // SocketAddrV4::

    // TODO user spec.
    let mut ring = IoUring::new(32).unwrap();

    let mut sq = IoUring::submission(&mut ring);

    let sockfd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, libc::IPPROTO_TCP) };
    // io_uring::
    let prep_connect =
        opcode::Connect::new(Fd(sockfd.as_raw_fd()), os_addr.as_ptr(), os_addr.len());

    unsafe { sq.push(&prep_connect.build()).unwrap() };

    let outer_e = std::time::Instant::now();
    println!("{:?} ", outer_e - outer_b);
    Ok(())
}

// let root_store =
//     rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

// let config = rustls::ClientConfig::builder()
//     .with_root_certificates(root_store)
//     .with_no_client_auth();

// let server_name = "example.com".try_into().unwrap();
// let mut conn = rustls::ClientConnection::new(Arc::new(config), server_name).unwrap();
// // let mut tls_stream = StreamOwned::new(conn, stream);

// // Prepare the GET request
// let request = b"GET / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n";
// let mut write_buffer = request.to_vec();

// // Set up io_uring
// let mut ring = IoUring::new(8)?;

// let mut read_buffer = vec![0u8; BUFFER_SIZE];
// // let mut response = Vec::new();

// // unsafe { ring.submission().push(opcode::Connect) };

// ##########################

// use mio::net::TcpStream;
// use mio::Token;
// use rustls::ClientConfig;
// use rustls::OwnedTrustAnchor;
// use rustls::RootCertStore;
// use rustls::ServerName;
// use rustls::Stream;
// use std::io::{Read, Write};
// use std::net::SocketAddr;
// use std::sync::Arc;
// use std::time;

// // const SERVER: Token = Token(0);

// fn main() -> Result<(), Box<dyn std::error::Error>> {
// // Create a root certificate store
// let mut root_cert_store = RootCertStore::empty();
// root_cert_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
//     OwnedTrustAnchor::from_subject_spki_name_constraints(
//         ta.subject,
//         ta.spki,
//         ta.name_constraints,
//     )
// }));

// // Create a rustls client config
// let config = ClientConfig::builder()
//     .with_safe_defaults()
//     .with_root_certificates(root_cert_store)
//     .with_no_client_auth();

// // Wrap it in an Arc
// let config = Arc::new(config);

// let mut begin = time::Instant::now();

// // Create a DNS name for the server
// let dns_name = ServerName::try_from("google.com").unwrap();

// // Create a TcpStream and set it to non-blocking
// let addr: SocketAddr = "142.250.74.174:443".parse().unwrap(); // example.com IP

// // TODO blocks
// let mut stream = TcpStream::connect(addr)?;

// // stream.
// // stream.set_nonblocking(true)?;
// // stream.set

// // Create a Poll instance
// // let mut poll = Poll::new()?;
// // poll.registry()
// //     .register(&mut stream, SERVER, Interest::READABLE | Interest::WRITABLE)?;

// // Create a buffer for handling events
// // let mut events = Events::with_capacity(128);

// // Create a client session
// let mut client = rustls::ClientConnection::new(config, dns_name)?;
// let mut tls_stream = Stream::new(&mut client, &mut stream);

// let mut end = time::Instant::now();
// print!("handshake {:?}\n", end - begin);

// begin = time::Instant::now();

// // Handshake loop
// 'outer: loop {
//     // poll.poll(&mut events, None)?;

//     match tls_stream.conn.complete_io(tls_stream.sock) {
//         Ok(_) => {
//             println!("TLS handshake completed");
//             break 'outer;
//         }
//         Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
//         Err(e) => return Err(Box::new(e)),
//     }
// }

// end = time::Instant::now();
// print!("TLS handshake {:?}\n", end - begin);

// // Send a request
// tls_stream.write_all(b"GET / HTTP/1.0\r\n\r\n")?;
// println!("Sent requestx");

// begin = time::Instant::now();

// let mut buf = [0; 4096];
// let mut x = 0;
// 'outer: loop {
//     // poll.poll(&mut events, None)?;

//     // for event in events.iter() {
//     // if event.token() == SERVER && event.is_readable() {
//     match tls_stream.read(&mut buf) {
//         Ok(0) => {
//             println!("Connection closed");
//             break 'outer;
//         }
//         Ok(m) => {
//             x = m;
//             break 'outer;
//         }
//         Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
//         Err(e) => return Err(Box::new(e)),
//     }
// }
// // }
// // }
// end = time::Instant::now();
// print!("{:?}\n", end - begin);
// print!("{:?}\n", std::str::from_utf8(&buf[..x]).unwrap());
// return Ok(());
// }
