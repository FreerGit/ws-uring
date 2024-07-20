use std::{
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

pub struct Client {
    ring: IoUring,
    domain: OsSocketAddr,
    sockfd: i32,
    conn_state: ConnectState,
    tls: Option<TlsStream>,
}

type TlsStream = StreamOwned<ClientConnection, TcpStream>;

type Result<T> = std::result::Result<T, ClientError>;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Client disconnected")]
    Disconnected,
    // #[error("")]
    // BadUrl(#[from] url::ParseError),
    #[error("Invalid DNS lookup: `{0}` ")]
    DNSLookupError(&'static str),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ConnectState {
    Connecting,
    Connected,
    Idle,
}

impl Client {
    pub fn new(addr: OsSocketAddr, tls: bool) -> Client {
        let begin = std::time::Instant::now();

        let end = std::time::Instant::now();
        println!("{:?}", end - begin);

        // println!("{:?}", config);

        let sockfd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, libc::IPPROTO_TCP) };
        let mut tls_ctx = None;
        if tls {
            let root_store = RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.into(),
            };
            let config = rustls::ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();

            let config = Arc::new(config);
            let tcp_stream = unsafe { TcpStream::from_raw_fd(sockfd) };
            let server_name = ServerName::try_from(domain).unwrap();
            let conn = ClientConnection::new(config, server_name).unwrap();
            let tls_stream = StreamOwned::new(conn, tcp_stream);
            tls_ctx = Some(tls_stream)
        }

        // TODO user definable `entries`?
        return Client {
            ring: IoUring::new(32).unwrap(),
            conn_state: ConnectState::Idle,
            tls: tls_ctx,
            domain,
            sockfd,
        };
    }

    // TODO docs
    pub fn connect(&mut self) -> Result<ConnectState> {
        if self.conn_state == ConnectState::Idle {
            // TODO no-delay?

            let prep_connect =
                opcode::Connect::new(Fd(self.sockfd.as_raw_fd()), addr.as_ptr(), addr.len());
            unsafe { self.ring.submission().push(&prep_connect.build()).unwrap() };
            self.ring.submit().unwrap();
            self.conn_state = ConnectState::Connecting;
        }

        let mut peekable_cq = self.ring.completion().peekable();
        if let Some(_) = peekable_cq.peek() {
            let cqe = peekable_cq.next(); // advance the cq
            assert!(cqe.is_some()); // Has to hold true since we peeked it, if not there is certainly a bug.
            if self.conn_state == ConnectState::Connecting {
                if let Some(_) = self.tls {
                    self.conn_state = ConnectState::Connected;
                } else {
                    println!("{:?}", peekable_cq.len());
                    self.conn_state = ConnectState::Connected;
                }
            }
        }
        return Ok(self.conn_state);
    }
}

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
pub fn dns_lookup(domain: &'static str, port: u16) -> Result<OsSocketAddr> {
    let addr = (domain, port).to_socket_addrs();
    match addr {
        Ok(mut addr_iter) => {
            let first_addr = addr_iter
                .next()
                .ok_or(ClientError::DNSLookupError("No IPv4 found from DNS lookup"))?;
            let os_addr: OsSocketAddr = first_addr.into();
            Ok(os_addr)
        }
        Err(_) => Err(ClientError::DNSLookupError("Invalid domain name")),
    }
}

// fn prep_connect

#[cfg(test)]
mod tests {
    use super::*;

    fn do_lookup(s: &'static str, port: u16) -> bool {
        let client = Client::new(s, false);
        client.dns_lookup(port).is_ok()
    }

    #[test]
    fn dns_lookup() {
        assert!(do_lookup("example.com", 443));
        assert!(do_lookup("www.example.com", 443));
        assert!(!do_lookup("http://example.com", 443));
        assert!(!do_lookup("http://www.example.com", 443));
        assert!(!do_lookup("http://www.example.com", 80));
        assert!(!do_lookup("https://example.com", 443));
    }
}
