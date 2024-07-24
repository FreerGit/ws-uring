use std::{
    io::{self, Bytes},
    net::ToSocketAddrs,
    os::fd::AsRawFd,
};

use io_uring::{
    opcode::{self},
    types::Fd,
    IoUring,
};
use libc::user;
use os_socketaddr::OsSocketAddr;
use thiserror::Error;
use url::Url;

pub struct Client {
    ring: IoUring,
    addr: OsSocketAddr,
    // On issue_connect, we need to re-allocate socket and start over.
    sockfd: Option<i32>,
    // buffer: Vec<u8>, // buffer for uring to put bytes on
    bump_index: usize,
}

type Result<T> = std::result::Result<T, ClientError>;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("{0}")]
    IO(#[from] io::Error),
    #[error("Client disconnected")]
    Disconnected,
    #[error("Invalid URL, could not parse")]
    InvalidUrl,
    #[error("No host in URL")]
    NoHost,
    #[error("Failed to resolve host")]
    ResolveHost,
    #[error("Out of Memory, message was larger than your buffer")]
    OOM,
}

#[derive(Debug, PartialEq)]
pub enum State {
    Idle,
    Read(usize),
    Connect,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Operation {
    Read,
    Write,
    Connect,
    Close,
}

impl Client {
    pub fn new(url: String) -> Result<Client> {
        let (addr, port) = dns_lookup(url)?;

        if port == 443 {
            panic!("HTTPS is not supported");
        }

        // TODO user definable `entries`?
        return Ok(Client {
            ring: IoUring::new(32).unwrap(),
            addr,
            sockfd: None,
            // buffer: vec![0u8; 1024 * 64],
            bump_index: 0,
        });
    }

    pub fn step(&mut self, user_buffer: &mut [u8]) -> Result<State> {
        let next = self.ring.completion().next();
        match next {
            Some(cqe) => {
                let op = unsafe { Box::from_raw(cqe.user_data() as *mut Operation) };
                let res = cqe.result();

                println!("{}, {:#?}", res, *op);
                if res < 0 {
                    return Err(ClientError::IO(io::Error::from_raw_os_error(-res)));
                }
                match *op {
                    // https://man7.org/linux/man-pages/man2/recv.2.html
                    Operation::Read => {
                        if res == 0 {
                            self.bump_index = 0;
                            return Ok(State::Read(res as usize));
                        }

                        self.bump_index += res as usize;

                        if user_buffer[..self.bump_index as usize].ends_with(b"\r\n\r\n") {
                            let msg_len = self.bump_index;
                            self.bump_index = 0;
                            return Ok(State::Read(msg_len));
                        }

                        self.issue_read(user_buffer).unwrap();
                        return Ok(State::Idle);
                    }
                    // https://man7.org/linux/man-pages/man2/write.2.html
                    // TODO: There may be a case where fewer bytes than suggested is written
                    Operation::Write => Ok(State::Idle),
                    Operation::Connect => Ok(State::Connect),
                    Operation::Close => Ok(State::Idle),
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

    pub fn issue_read(&mut self, rb: &mut [u8]) -> Result<()> {
        if self.bump_index >= rb.len() {
            return Err(ClientError::OOM);
        }
        assert!(rb.len() as i32 - self.bump_index as i32 >= 0);
        println!("issuing read with sockfd: {:?}", self.sockfd);
        let read = opcode::Recv::new(
            Fd(self.sockfd.unwrap().as_raw_fd()),
            rb[self.bump_index..].as_mut_ptr(),
            (rb.len() - self.bump_index) as u32,
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

    pub fn issue_close(&mut self) -> Result<()> {
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
}

fn dns_lookup(url_str: String) -> Result<(OsSocketAddr, u16)> {
    let url = Url::parse(&url_str).or(Err(ClientError::InvalidUrl))?;
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

    Ok((socket_addr, port))
}

#[cfg(test)]
mod tests {
    use crate::client::dns_lookup;

    #[test]
    fn dns_lookup_success() {
        assert!(dns_lookup("http://www.example.com".to_string()).is_ok());
        assert!(dns_lookup("http://example.com".to_string()).is_ok());
        assert!(dns_lookup("https://www.example.com".to_string()).is_ok());
        assert!(dns_lookup("http://localhost".to_string()).is_ok());

        let (_, port) = dns_lookup("http://localhost:8080".to_string()).unwrap();
        assert!(port == 8080);
    }

    #[test]
    fn dns_lookup_fail() {
        assert!(dns_lookup("".to_string()).is_err());
        assert!(dns_lookup("example.com".to_string()).is_err());
        assert!(dns_lookup("www.example.com".to_string()).is_err());
    }
}
