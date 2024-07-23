use std::{
    io::{self, ErrorKind, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    os::fd::{AsRawFd, FromRawFd},
    sync::Arc,
};

use io_uring::{
    opcode::{self},
    types::Fd,
    CompletionQueue, IoUring,
};
use os_socketaddr::OsSocketAddr;
use rustls::{pki_types::ServerName, ClientConnection, RootCertStore, StreamOwned};
use thiserror::Error;
use url::Url;

pub struct Client {
    ring: IoUring,
    addr: OsSocketAddr,
    domain: String,
    // On issue_connect, we need to re-allocate socket and start over.
    sockfd: Option<i32>,
    tls: Option<TlsStream>,
    tls_buffer: Vec<u8>, // TODO optional
}

type TlsStream = StreamOwned<ClientConnection, TcpStream>;

type Result<T> = std::result::Result<T, ClientError>;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("{0}")]
    IO(#[from] io::Error),
    #[error("{0}")]
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

// #[derive(Debug, PartialEq, Clone, Copy)]
// pub enum ConnectState {
//     Disconnected,
//     Connecting,
//     Connected,
// }

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum State {
    Idle,
    Read(usize),
    Write,
    Connect,
}

// #[derive(Debug, PartialEq, Clone, Copy)]
// pub enum ReadState {
//     Disconnected,
//     WantsRead,
//     Read(usize),
// }

// #[derive(Debug, PartialEq, Clone, Copy)]
// pub enum WriteState {
//     Disconnected,
//     WantsWrite,
//     Written,
// }

#[derive(Debug, PartialEq, Clone, Copy)]
enum Operation {
    Read,
    Write,
    Connect,
    Close,
}

impl Client {
    pub fn new(url: String) -> Result<Client> {
        // let begin = std::time::Instant::now();

        // let end = std::time::Instant::now();
        // println!("{:?}", end - begin);

        let (addr, domain, server_name, port) = dns_lookup(url)?;

        let mut tls_ctx = None;

        // TLS
        // if port == 443 {
        //     let root_store = RootCertStore {
        //         roots: webpki_roots::TLS_SERVER_ROOTS.into(),
        //     };
        //     let config = rustls::ClientConfig::builder()
        //         .with_root_certificates(root_store)
        //         .with_no_client_auth();

        //     let config = Arc::new(config);
        //     let tcp_stream = unsafe { TcpStream::from_raw_fd(sockfd) };
        //     let conn = ClientConnection::new(config, server_name).unwrap();
        //     let tls_stream = StreamOwned::new(conn, tcp_stream);
        //     tls_ctx = Some(tls_stream)
        // }

        // TODO user definable `entries`?
        return Ok(Client {
            ring: IoUring::new(32).unwrap(),
            addr,
            domain,
            tls: tls_ctx,
            sockfd: None,
            tls_buffer: vec![0u8; 16384], // TODO expose static size to user
        });
    }

    pub fn step(&mut self) -> Result<State> {
        match self.ring.completion().next() {
            Some(cqe) => {
                let op = unsafe { Box::from_raw(cqe.user_data() as *mut Operation) };
                let res = cqe.result();
                // io::Error::from_raw_os_error(
                //     io::Error::last_os_error().raw_os_error().unwrap()
                // )
                println!("{}, {:#?}", res, *op,);
                if res < 0 {
                    return Err(ClientError::IO(io::Error::from_raw_os_error(-res)));
                }
                match *op {
                    // https://man7.org/linux/man-pages/man2/recv.2.html
                    Operation::Read => Ok(State::Read(res as usize)),
                    // https://man7.org/linux/man-pages/man2/write.2.html
                    // TODO: There may be a case where fewer bytes than suggested is written
                    Operation::Write => Ok(State::Write),
                    Operation::Connect => Ok(State::Connect),
                    Operation::Close => {
                        return Ok(State::Idle);
                    }
                }
            }
            None => Ok(State::Idle),
        }
    }

    pub fn issue_write(&mut self, bytes: &[u8]) -> Result<()> {
        println!("issuing write with sockfd: {:?}", self.sockfd);
        let write = opcode::Write::new(
            Fd(self.sockfd.unwrap().as_raw_fd()),
            bytes.as_ptr(),
            bytes.len() as _,
        )
        .build()
        .user_data(Box::into_raw(Box::new(Operation::Write)) as u64);
        unsafe {
            self.ring
                .submission()
                .push(&write)
                .expect("submission queue is full");
        }
        self.ring.submit().or_else(|e| Err(ClientError::IO(e)))?;
        Ok(())
    }

    pub fn issue_read(&mut self, buf: &mut [u8]) -> Result<()> {
        println!("issuing read with sockfd: {:?}", self.sockfd);
        let read = opcode::Recv::new(
            Fd(self.sockfd.unwrap().as_raw_fd()),
            buf.as_mut_ptr(),
            buf.len() as _,
        )
        .build()
        .user_data(Box::into_raw(Box::new(Operation::Read)) as u64);
        unsafe {
            self.ring
                .submission()
                .push(&read)
                .expect("submission queue is full");
        }
        self.ring.submit().or_else(|e| Err(ClientError::IO(e)))?;
        Ok(())
    }

    pub fn issue_connect(&mut self) -> Result<()> {
        //
        if self.sockfd.is_some() {
            self.issue_close().unwrap();
        }
        let sockfd = unsafe {
            libc::socket(
                libc::AF_INET,
                libc::SOCK_STREAM | libc::O_NONBLOCK,
                libc::IPPROTO_TCP,
            )
        };
        println!("{}", sockfd);
        self.sockfd = Some(sockfd);

        println!("issuing connect with sockfd: {:?}", self.sockfd);
        let connect = opcode::Connect::new(
            Fd(self.sockfd.unwrap().as_raw_fd()),
            self.addr.as_ptr(),
            self.addr.len(),
        )
        .build()
        .user_data(Box::into_raw(Box::new(Operation::Connect)) as u64);
        unsafe {
            self.ring
                .submission()
                .push(&connect)
                .expect("submission queue is full")
        };
        self.ring.submit().or_else(|e| Err(ClientError::IO(e)))?;
        Ok(())
    }

    // Handled privately for now, perhaps expose to user??
    fn issue_close(&mut self) -> Result<()> {
        let close = opcode::Close::new(Fd(self.sockfd.unwrap().as_raw_fd()))
            .build()
            .user_data(Box::into_raw(Box::new(Operation::Close)) as u64);
        unsafe {
            self.ring
                .submission()
                .push(&close)
                .expect("submission queue is full");
        }
        self.ring.submit().or_else(|e| Err(ClientError::IO(e)))?;
        self.sockfd = None;
        Ok(())
    }

    pub fn handle_write(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn handle_read(&mut self, buf: &mut Vec<u8>) -> Result<()> {
        Ok(())
    }

    pub fn handle_connect(&mut self) -> Result<()> {
        Ok(())
    }

    // TODO docs
    // pub fn connect(&mut self) -> Result<ConnectState> {
    //     // assert!(self.tls.is_some());

    //     if !self.connect_submitted {
    //         let prep_connect = opcode::Connect::new(
    //             Fd(self.sockfd.as_raw_fd()),
    //             self.addr.as_ptr(),
    //             self.addr.len(),
    //         );
    //         unsafe {
    //             self.ring
    //                 .submission()
    //                 .push(
    //                     &prep_connect
    //                         .build()
    //                         .user_data(Box::into_raw(Box::new(OperationType::Connect)) as u64),
    //                 )
    //                 .expect("submission queue is full")
    //         };
    //         self.ring.submit().unwrap();
    //         self.connect_submitted = true; // todo set false when connect is done.
    //     }
    //     let a = self.ring.completion().len();
    //     println!("before {:?}", a);
    //     match get_cqe_by_op(self.ring.completion(), OperationType::Connect) {
    //         _ => {
    //             if self.tls.is_none() {
    //                 return Ok(ConnectState::Connected);
    //             }
    //         }
    //     }
    //     let b = self.ring.completion().len();
    //     println!("after {:?}", b);
    //     if a > 10 {
    //         panic!();
    //     }
    //     // let conn = &self. .tls.as_ref().unwrap().conn;
    //     // if !self.tls.as_ref().unwrap().conn.is_handshaking() {
    //     //     return Ok(ConnectState::Connected);
    //     // }

    //     if self.tls.as_ref().unwrap().conn.wants_write() {
    //         // println!("wants write");
    //         let mut write_buffer = vec![0u8; 16384];
    //         match self.write(&mut write_buffer) {
    //             Ok(WriteState::Written) => {}
    //             Ok(WriteState::WantsWrite) => {}
    //             Ok(WriteState::Disconnected) => return Ok(ConnectState::Disconnected),
    //             Err(e) => return Err(e),
    //         }
    //     }

    //     if self.tls.as_ref().unwrap().conn.wants_read() {
    //         // println!("wants read");

    //         let mut read_buffer = vec![0u8; 16384];
    //         match self.read(&mut read_buffer) {
    //             Ok(ReadState::Read(_)) => {}
    //             Ok(ReadState::WantsRead) => {}
    //             Ok(ReadState::Disconnected) => return Ok(ConnectState::Disconnected),
    //             Err(e) => return Err(e),
    //         }
    //     }
    //     // TODO .next the connect cqe

    //     return Ok(ConnectState::Connecting);
    // }

    // pub fn write(&mut self, write_buffer: &mut [u8]) -> Result<WriteState> {
    //     let conn = &mut self.tls.as_mut().unwrap().conn;
    //     let mut bytes_written = 0;
    //     if !self.write_submitted {
    //         bytes_written = conn.writer().write(&write_buffer).unwrap();

    //         match conn.write_tls(&mut &mut write_buffer[..]) {
    //             Ok(0) => return Ok(WriteState::Written),
    //             Ok(bytes_encrypted) => {
    //                 let write_op = opcode::Write::new(
    //                     Fd(self.sockfd.as_raw_fd()),
    //                     write_buffer.as_ptr(),
    //                     bytes_encrypted as _,
    //                 )
    //                 .build()
    //                 .user_data(Box::into_raw(Box::new(OperationType::Write)) as u64);
    //                 unsafe {
    //                     self.ring
    //                         .submission()
    //                         .push(&write_op)
    //                         .expect("submission queue is full");
    //                 }
    //                 self.write_submitted = true;
    //                 self.ring.submit().or_else(|e| Err(ClientError::IO(e)))?;
    //             }
    //             Err(e) => return Err(ClientError::IO(e)),
    //         }
    //     }

    //     match self.ring.completion().next() {
    //         Some(cqe) => todo!(),
    //         None => todo!(),
    //     }

    //     match get_cqe_by_op(self.ring.completion(), OperationType::Write) {
    //         Some(cqe) => {
    //             let res = cqe.result();
    //             if res >= 0 {
    //                 let bytes_sent = res as usize;
    //                 println!("sent: {} written: {}", bytes_sent, bytes_written);
    //                 assert!(bytes_sent == bytes_written); // Im unsure how this occurs but certainly a bug if it does.
    //                 return Ok(WriteState::Written);
    //             } else {
    //                 return Err(ClientError::IO(io::Error::from_raw_os_error(-res)));
    //             }
    //         }
    //         None => return Ok(WriteState::WantsWrite),
    //     }
    // }

    // /// Caller owns the read_buffer
    // ///
    // /// For performance reason, you may want to reuse buffer and set it to a "large enough" start size.
    // pub fn read(&mut self, mut read_buffer: &mut Vec<u8>) -> Result<ReadState> {
    //     // let mut tls_buffer = [0u8; 16384]; // Buffer for encrypted data
    //     let conn = &mut self.tls.as_mut().unwrap().conn;

    //     // Only check if we have submitted a read event
    //     if self.read_submitted {
    //         // println!("in first read");

    //         // Check for existing bytes from previous fn call
    //         match conn.reader().read(read_buffer) {
    //             Ok(0) => return Ok(ReadState::Disconnected),
    //             Ok(count) => {
    //                 // We have used up the read, make sure to issue next iteration
    //                 self.read_submitted = false;
    //                 return Ok(ReadState::Read(count));
    //             }
    //             Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
    //                 self.read_submitted = false;
    //                 return Ok(ReadState::WantsRead);
    //             }
    //             Err(e) => return Err(ClientError::IO(e)),
    //         }
    //     }

    //     let read_op = opcode::Read::new(
    //         Fd(self.sockfd.as_raw_fd()),
    //         self.tls_buffer.as_mut_ptr(),
    //         self.tls_buffer.len() as _,
    //     )
    //     .build()
    //     .user_data(Box::into_raw(Box::new(OperationType::Read)) as u64);
    //     unsafe {
    //         self.ring
    //             .submission()
    //             .push(&read_op)
    //             .expect("submission queue is full");
    //     }
    //     // Read event submitted, update state
    //     self.read_submitted = true;
    //     self.ring.submit().or_else(|e| Err(ClientError::IO(e)))?;
    //     // println!("submit");

    //     match get_cqe_by_op(self.ring.completion(), OperationType::Read) {
    //         Some(cqe) => {
    //             let res = cqe.result();
    //             if res >= 0 {
    //                 let bytes_read = res as usize;
    //                 // Pass the encrypted data to rustls
    //                 conn.read_tls(&mut &self.tls_buffer[..bytes_read])?;
    //                 // println!("{}", bytes_read);

    //                 // Process the new packets
    //                 match conn.process_new_packets() {
    //                     Ok(io_state) => println!("Processed new packets: {:?}", io_state),
    //                     Err(e) => return Err(ClientError::TLS(e)),
    //                 }

    //                 // let mut total_decrypted = 0;

    //                 // loop {
    //                 match conn.reader().read(&mut read_buffer) {
    //                     Ok(0) => return Ok(ReadState::Disconnected), // No more data
    //                     Ok(count) => {
    //                         // total_decrypted += count;
    //                         println!("Read {} bytes of decrypted data", count);
    //                         // if total_decrypted == read_buffer.len() {
    //                         // break; // Buffer is full
    //                         // }
    //                         self.read_submitted = false;
    //                         return Ok(ReadState::Read(count));
    //                     }
    //                     Err(e) if e.kind() == ErrorKind::WouldBlock => {
    //                         return Ok(ReadState::WantsRead)
    //                     }
    //                     Err(e) => return Err(ClientError::IO(e)),
    //                 }
    //             } else {
    //                 return Err(ClientError::IO(io::Error::from_raw_os_error(-res)));
    //             }
    //         }
    //         None => return Ok(ReadState::WantsRead),
    //     }
    // }
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
