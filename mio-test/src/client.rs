use std::{net::ToSocketAddrs, os::fd::AsRawFd};

use io_uring::{
    opcode::{self, Connect},
    types::Fd,
    IoUring,
};
use libc::sockaddr;
use os_socketaddr::OsSocketAddr;
use thiserror::Error;

pub struct Client {
    ring: IoUring,
}

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

#[derive(Debug)]
pub enum ConnectState {
    Connecting,
    Connected,
}

impl Client {
    pub fn new() -> Client {
        // TODO user definable `entries`?
        return Client {
            ring: IoUring::new(32).unwrap(),
        };
    }
    // TODO docs
    // TODO test
    /// `dns_lookup(...)` is the only function that blocks, therefore it's seperate. You may cache the ip and use for later.
    /// This function makes sure that reconnects with `connect(...)` can be kept non-blocking.
    /// # Example
    /// ```
    /// use ws_uring::client::Client;
    /// let client = Client::new();
    /// // Supply domain name, not entire url!
    /// assert!(client.dns_lookup("example.com", 443).is_ok());
    /// assert!(client.dns_lookup("www.example.com", 80).is_ok());
    /// ```
    pub fn dns_lookup(&self, domain: &str, port: u16) -> Result<OsSocketAddr> {
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

    // TODO docs
    pub fn connect(&mut self, addr: OsSocketAddr) -> Result<ConnectState> {
        let mut sq = IoUring::submission(&mut self.ring);
        // TODO no-delay?
        let sockfd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, libc::IPPROTO_TCP) };
        let prep_connect = opcode::Connect::new(Fd(sockfd.as_raw_fd()), addr.as_ptr(), addr.len());
        unsafe { sq.push(&prep_connect.build()).unwrap() };
        return Ok(ConnectState::Connected);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dns_lookup() {
        let client = Client::new();
        let mut ip = client.dns_lookup("google.com", 80);
        assert!(ip.is_ok());
        ip = client.dns_lookup("example.com", 443);
        assert!(ip.is_ok());
        ip = client.dns_lookup("www.example.com", 443);
        assert!(ip.is_ok());
        ip = client.dns_lookup("http://example.com", 443);
        assert!(ip.is_err());
        ip = client.dns_lookup("http://www.example.com", 443);
        assert!(ip.is_err());
        ip = client.dns_lookup("http://www.example.com", 80);
        assert!(ip.is_err());
        ip = client.dns_lookup("https://example.com", 443);
        assert!(ip.is_err());
    }
}
