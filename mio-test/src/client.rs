use std::{
    io::{self, ErrorKind, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    os::fd::{AsRawFd, FromRawFd},
    sync::Arc,
};

use io_uring::{
    opcode::{self},
    types::Fd,
    IoUring,
};
use os_socketaddr::OsSocketAddr;
use rustls::{pki_types::ServerName, ClientConnection, RootCertStore, StreamOwned};
use thiserror::Error;
use url::Url;

pub struct Client {
    ring: IoUring,
    addr: OsSocketAddr,
    domain: String,
    sockfd: i32,
    conn_state: ConnectState,
    read_submitted: bool,
    tls: Option<TlsStream>,
    tls_buffer: Vec<u8>,
    // wb: Vec<u8>,
}

type TlsStream = StreamOwned<ClientConnection, TcpStream>;

type Result<T> = std::result::Result<T, ClientError>;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Internal error, most likely a bug :( {0}")]
    IO(#[from] io::Error),
    #[error("Internal error, most likely a bug :( {0}")]
    TLS(#[from] rustls::Error),
    #[error("Client disconnected")]
    Disconnected,
    #[error("Invalid URL, could not parse")]
    InvalidUrl,
    #[error("No host in URL")]
    NoHost,
    #[error("Failed to resolve host")]
    ResolveHost,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ConnectState {
    Connecting,
    Connected,
    TlsHandshakeInit,
    TlsHandshakeRead,
    Idle,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ReadState {
    Idle,
    Disconnected,
    WantsRead,
    Read(usize),
}

impl Client {
    pub fn new(url: String) -> Result<Client> {
        // let begin = std::time::Instant::now();

        // let end = std::time::Instant::now();
        // println!("{:?}", end - begin);

        let (addr, domain, server_name, port) = dns_lookup(url)?;

        let sockfd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, libc::IPPROTO_TCP) };
        let mut tls_ctx = None;
        // TLS
        if port == 443 {
            let root_store = RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.into(),
            };
            let config = rustls::ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();

            let config = Arc::new(config);
            let tcp_stream = unsafe { TcpStream::from_raw_fd(sockfd) };
            let conn = ClientConnection::new(config, server_name).unwrap();
            let tls_stream = StreamOwned::new(conn, tcp_stream);
            tls_ctx = Some(tls_stream)
        }

        // TODO user definable `entries`?
        return Ok(Client {
            ring: IoUring::new(32).unwrap(),
            conn_state: ConnectState::Idle,
            read_submitted: false,
            addr,
            domain,
            tls: tls_ctx,
            sockfd,
            tls_buffer: vec![0u8; 16384],
        });
    }

    // TODO docs
    pub fn connect(&mut self) -> Result<ConnectState> {
        if self.conn_state == ConnectState::Idle {
            // TODO no-delay?

            let prep_connect = opcode::Connect::new(
                Fd(self.sockfd.as_raw_fd()),
                self.addr.as_ptr(),
                self.addr.len(),
            );
            unsafe { self.ring.submission().push(&prep_connect.build()).unwrap() };
            self.ring.submit().unwrap();
            self.conn_state = ConnectState::Connecting;
        }

        if self.conn_state == ConnectState::TlsHandshakeInit {
            // let read_e = opcode::Read::new(
            //     Fd(self.tls.unwrap().get_ref().as_raw_fd()),
            //     // self.tls.unwrap().sock.
            //     self.rb.as_mut_ptr(),
            //     self.rb.len() as _,
            // );
            // unsafe {
            //     self.ring
            //         .submission()
            //         .push(&read_e.build())
            //         .expect("SQ is full")
            // }
            // self.ring.submit().unwrap();
            self.conn_state = ConnectState::TlsHandshakeRead;
        }

        // TODO just check len instead.
        // let mut peekable_cq = self.ring.completion().peekable();
        if self.ring.completion().len() > 0 {
            // Has to hold true since we peeked it, if not there is certainly a bug.
            match self.conn_state {
                ConnectState::Connecting => {
                    if let Some(_) = self.tls {
                        self.conn_state = ConnectState::TlsHandshakeRead;
                    } else {
                        let _ = self.ring.completion().next().unwrap(); // advance the cq
                        self.conn_state = ConnectState::Connected;
                    }
                }
                ConnectState::TlsHandshakeRead => {
                    let _ = self.ring.completion().next().unwrap(); // advance the cq
                                                                    // ;
                    let mut read_buf = vec![0u8; 16384];
                    let mut write_buf = vec![0u8; 16384];
                    let conn = &mut self.tls.as_mut().unwrap().conn;
                    while conn.is_handshaking() {
                        // panic!("hejs");
                        // Try to progress the handshake
                        if conn.wants_write() {
                            match conn.write_tls(&mut &mut write_buf[..]) {
                                Ok(written) => {
                                    if written > 0 {
                                        let write_op = opcode::Write::new(
                                            Fd(self.sockfd.as_raw_fd()),
                                            write_buf.as_ptr(),
                                            written as _,
                                        )
                                        .build();
                                        unsafe {
                                            self.ring
                                                .submission()
                                                .push(&write_op)
                                                .expect("submission queue is full");
                                        }
                                        self.ring.submit_and_wait(1).unwrap();
                                        // Process completion
                                        self.ring.completion().for_each(|cqe| {
                                            let res = cqe.result();
                                            if res < 0 {
                                                eprintln!(
                                                    "Write failed: {}",
                                                    io::Error::from_raw_os_error(-res)
                                                );
                                            }
                                        });
                                    }
                                }
                                // Err(WantWrite) => {}
                                Err(e) => panic!("{}", format!("{:?}", e)),
                            }
                        }

                        if conn.wants_read() {
                            let read_op = opcode::Read::new(
                                Fd(self.sockfd.as_raw_fd()),
                                read_buf.as_mut_ptr(),
                                read_buf.len() as _,
                            )
                            .build();
                            unsafe {
                                self.ring
                                    .submission()
                                    .push(&read_op)
                                    .expect("submission queue is full");
                            }
                            self.ring.submit_and_wait(1).unwrap();

                            let mut bytes_read = 0;
                            self.ring.completion().for_each(|cqe| {
                                let res = cqe.result();
                                if res >= 0 {
                                    bytes_read = res as usize;
                                } else {
                                    eprintln!(
                                        "Read failed: {}",
                                        io::Error::from_raw_os_error(-res)
                                    );
                                }
                            });

                            if bytes_read > 0 {
                                match conn.read_tls(&mut &read_buf[..bytes_read]) {
                                    Ok(_) => {
                                        if let Err(e) = conn.process_new_packets() {
                                            panic!("{}", format!("{:?}", e));
                                        }
                                    }
                                    // Err(WantRead) => {}
                                    Err(e) => panic!("{}", format!("{:?}", e)),
                                }
                            }
                        }
                    }
                    // panic!("hej");
                    return Ok(ConnectState::Connected);
                }
                ConnectState::TlsHandshakeInit | ConnectState::Idle | ConnectState::Connected => {
                    todo!()
                }
            }
        }
        return Ok(self.conn_state);
    }

    pub fn write(&mut self, plaintext: &[u8]) -> Result<()> {
        let mut total_written = 0;
        let mut write_buf = vec![0u8; 16384];
        let conn = &mut self.tls.as_mut().unwrap().conn;

        while total_written < plaintext.len() {
            // Write plaintext into the TLS connection
            let bytes_processed = conn.writer().write(&plaintext[total_written..]).unwrap();
            total_written += bytes_processed;

            // Encrypt and send the data
            loop {
                match conn.write_tls(&mut &mut write_buf[..]) {
                    Ok(0) => break, // No more data to write
                    Ok(bytes_encrypted) => {
                        let write_op = opcode::Write::new(
                            Fd(self.sockfd.as_raw_fd()),
                            write_buf.as_ptr(),
                            bytes_encrypted as _,
                        )
                        .build();
                        unsafe {
                            self.ring
                                .submission()
                                .push(&write_op)
                                .expect("submission queue is full");
                        }
                        self.ring.submit_and_wait(1).unwrap();

                        let mut bytes_sent = 0;
                        self.ring.completion().for_each(|cqe| {
                            let res = cqe.result();
                            if res >= 0 {
                                bytes_sent = res as usize;
                            } else {
                                eprintln!("Write failed: {}", io::Error::from_raw_os_error(-res));
                            }
                        });

                        if bytes_sent != bytes_encrypted {
                            panic!("failed to write data");
                        }
                    }
                    // Err(rustls::Error::WantWrite) => continue,
                    Err(e) => panic!("{}", format!("{:?}", e)),
                }
            }
        }

        Ok(())
    }

    /// Caller owns the read_buffer
    ///
    /// For performance reason, you may want to reuse buffer and set it to a "large enough" start size.
    pub fn read(&mut self, mut read_buffer: &mut Vec<u8>) -> Result<ReadState> {
        // let mut tls_buffer = [0u8; 16384]; // Buffer for encrypted data
        let conn = &mut self.tls.as_mut().unwrap().conn;

        // Only check if we have submitted a read event
        if self.read_submitted {
            println!("in first read");

            // Check for existing bytes from previous fn call
            match conn.reader().read(read_buffer) {
                Ok(0) => return Ok(ReadState::Disconnected),
                Ok(count) => {
                    // We have used up the read, make sure to issue next iteration
                    self.read_submitted = false;
                    return Ok(ReadState::Read(count));
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    self.read_submitted = false;
                    return Ok(ReadState::WantsRead);
                }
                Err(e) => return Err(ClientError::IO(e)),
            }
        }

        let read_op = opcode::Read::new(
            Fd(self.sockfd.as_raw_fd()),
            self.tls_buffer.as_mut_ptr(),
            self.tls_buffer.len() as _,
        )
        .build();
        unsafe {
            self.ring
                .submission()
                .push(&read_op)
                .expect("submission queue is full");
        }
        // Read event submitted, update state
        self.read_submitted = true;
        self.ring.submit().or_else(|e| Err(ClientError::IO(e)))?;
        println!("submit");

        // No reads complete, early exit. Likely branch.
        if self.ring.completion().len() == 0 {
            println!("len == 0");
            return Ok(ReadState::WantsRead);
        } else {
            let cqe = self.ring.completion().next().unwrap();
            let res = cqe.result();
            if res < 0 {
                return Err(ClientError::IO(io::Error::from_raw_os_error(-res)));
            }
            let bytes_read = res as usize;
            // Pass the encrypted data to rustls
            conn.read_tls(&mut &self.tls_buffer[..bytes_read])?;
            println!("{}", bytes_read);

            // Process the new packets
            let begin = std::time::Instant::now();
            match conn.process_new_packets() {
                Ok(io_state) => println!("Processed new packets: {:?}", io_state),
                Err(e) => return Err(ClientError::TLS(e)),
            }
            let end = std::time::Instant::now();
            println!("{:?} loop", end - begin);
            // panic!("now");

            let mut total_decrypted = 0;

            // loop {
            match conn.reader().read(&mut read_buffer) {
                Ok(0) => return Ok(ReadState::Disconnected), // No more data
                Ok(count) => {
                    // total_decrypted += count;
                    println!("Read {} bytes of decrypted data", count);
                    // if total_decrypted == read_buffer.len() {
                    // break; // Buffer is full
                    // }
                    self.read_submitted = false;
                    return Ok(ReadState::Read(count));
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(ReadState::WantsRead),
                Err(e) => return Err(ClientError::IO(e)),
            }
            // }

            // Ok(total_decrypted)
        }
    }
}

// fn perform_non_blocking_tls_handshake(
//     ring: &mut IoUring,
//     mut conn: ClientConnection,
//     stream: TcpStream,
// ) -> io::Result<()> {
//     // stream.set_nonblocking(true)?;
//     let fd = Fd(stream.as_raw_fd());

//     Ok(())
// }

// TODO docs
// TODO test
/// `dns_lookup(...)` is the only function that blocks, therefore it's seperate. You may cache the ip and use for later.
/// This function makes sure that reconnects with `connect(...)` can be kept non-blocking.
/// # Example
/// ```
/// use ws_uring::client::Client;
/// // Supply domain name, not entire url!
/// let client_1 = Client::new("example.com", true);
/// assert!(client_1.dns_lookup(443).is_ok());
/// // This will `fail`!
/// let client_2 = Client::new("http://www.example.com", true);
/// assert!(client_2.dns_lookup(443).is_err());
/// ```
// fn dns_lookup(domain: &str, port: u16) -> Result<OsSocketAddr> {
//     let addr = (domain, port).to_socket_addrs();
//     match addr {
//         Ok(mut addr_iter) => {
//             let first_addr = addr_iter
//                 .next()
//                 .ok_or(ClientError::DNSLookupError("No IPv4 found from DNS lookup"))?;
//             let os_addr: OsSocketAddr = first_addr.into();
//             println!("{:?}", first_addr.to_string());
//             Ok(os_addr)
//         }
//         Err(_) => Err(ClientError::DNSLookupError("Invalid domain name")),
//     }
// }

fn dns_lookup(url_str: String) -> Result<(OsSocketAddr, String, ServerName<'static>, u16)> {
    // Parse the URL
    let url = Url::parse(&url_str).or(Err(ClientError::InvalidUrl))?;

    // 1. Lookup the IP to use for the socket
    let host = url.host_str().ok_or(ClientError::NoHost)?.to_string();
    let port = url
        .port()
        .unwrap_or(if matches!(url.scheme(), "https" | "wss") {
            443
        } else {
            80
        });
    let socket_addr: OsSocketAddr = (host.to_owned(), port)
        .to_socket_addrs()
        .or(Err(ClientError::ResolveHost))?
        .next()
        .into();
    // 2. Convert the URL to a domain name for the GET request
    let domain = url.host_str().ok_or(ClientError::NoHost)?.to_string();

    // 3. Convert the URL to a server name for ClientConnection::new
    let server_name = ServerName::try_from(host.clone()).unwrap();

    Ok((socket_addr, domain, server_name, port))
}

// fn prep_connect

// #[cfg(test)]
// mod tests {
//     use super::*;

//     fn do_lookup(s: &'static str, port: u16) -> bool {
//         let client = Client::new(s, false);
//         client.dns_lookup(port).is_ok()
//     }

//     #[test]
//     fn dns_lookup() {
//         assert!(do_lookup("example.com", 443));
//         assert!(do_lookup("www.example.com", 443));
//         assert!(!do_lookup("http://example.com", 443));
//         assert!(!do_lookup("http://www.example.com", 443));
//         assert!(!do_lookup("http://www.example.com", 80));
//         assert!(!do_lookup("https://example.com", 443));
//     }
// }
