use bytes::BytesMut;
use io_uring::{
    opcode::{self},
    types::Fd,
    IoUring,
};
use os_socketaddr::OsSocketAddr;
use std::{
    io::{self},
    net::ToSocketAddrs,
    os::fd::AsRawFd,
};
use thiserror::Error;
use tokio_util::codec::{Decoder, Encoder};
use url::Url;
use websocket_codec::{Message, MessageCodec};

pub struct Client {
    ring: IoUring,
    addr: OsSocketAddr,
    host: String,
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
    #[error("Handshaked failed: {0}")]
    Handshake(String),
}

#[derive(Debug, PartialEq)]
pub enum State {
    Idle,
    Read(Option<Message>),
    Connect,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Operation {
    Read,
    Write,
    Connect,
    Close,
    Handshake,
}

fn create_request(uri: &str) -> String {
    let r: [u8; 16] = rand::random();
    let key = data_encoding::BASE64.encode(&r);

    let websocket_req = format!(
        "GET / HTTP/1.1\r\nHost: {}\r\nUpgrade: websocket\r\n\
        Connection: Upgrade\r\ncontent-length: 0\
        \r\nsec-websocket-version: 13\r\nsec-websocket-key: {}\r\n\r\n",
        uri, key
    );
    websocket_req
}

impl Client {
    pub fn new(url: String) -> Result<Client> {
        let (addr, host, port) = dns_lookup(url)?;

        if port == 443 {
            panic!("HTTPS is not supported");
        }

        // TODO user definable `entries`?
        Ok(Client {
            ring: IoUring::new(32).unwrap(),
            addr,
            host,
            sockfd: None,
            // buffer: vec![0u8; 1024 * 64],
            bump_index: 0,
        })
    }

    pub fn step(&mut self, user_buffer: &mut [u8]) -> Result<State> {
        let next = self.ring.completion().next();
        match next {
            Some(cqe) => {
                let op = unsafe { Box::from_raw(cqe.user_data() as *mut Operation) };
                let res = cqe.result();
                if res < 0 {
                    return Err(ClientError::IO(io::Error::from_raw_os_error(-res)));
                }
                match *op {
                    // https://man7.org/linux/man-pages/man2/recv.2.html
                    Operation::Read => {
                        if res == 0 {
                            self.bump_index = 0;
                            return Ok(State::Read(None));
                        }

                        self.bump_index += res as usize;
                        let mut payload = BytesMut::from(&user_buffer[..self.bump_index]);

                        let mut codec = MessageCodec::client();
                        let p = codec.decode(&mut payload).unwrap();
                        self.bump_index = 0;

                        Ok(State::Read(Some(p.unwrap())))
                    }
                    // https://man7.org/linux/man-pages/man2/write.2.html
                    // TODO: There may be a case where fewer bytes than suggested is written
                    Operation::Write => Ok(State::Idle),
                    Operation::Connect => {
                        let x = create_request(&self.host);
                        self.issue_handshake(user_buffer, x.as_bytes()).unwrap();
                        Ok(State::Idle)
                    }
                    Operation::Handshake => {
                        let mut headers = [httparse::EMPTY_HEADER; 16];
                        let mut response = httparse::Response::new(&mut headers);

                        httparse::ParserConfig::default()
                            .allow_obsolete_multiline_headers_in_responses(true)
                            .parse_response(&mut response, &user_buffer[..res as usize])
                            .unwrap();
                        if response.code != Some(101)
                            || response.reason != Some("Switching Protocols")
                        {
                            return Err(ClientError::Handshake("Switching protocol".to_string()));
                        }
                        Ok(State::Connect)
                    }
                    Operation::Close => Ok(State::Idle),
                }
            }
            None => Ok(State::Idle),
        }
    }

    pub fn issue_write(&mut self, bytes: &str) -> Result<()> {
        let mut payload = BytesMut::new();
        let mut codec = MessageCodec::client();
        codec.encode(Message::text(bytes), &mut payload).unwrap();
        self.issue_write_underlying(&payload).unwrap();
        Ok(())
    }

    fn issue_write_underlying(&mut self, bytes: &[u8]) -> Result<()> {
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
        self.ring.submit().map_err(ClientError::IO)?;
        Ok(())
    }

    pub fn issue_read(&mut self, rb: &mut [u8]) -> Result<()> {
        if self.bump_index >= rb.len() {
            return Err(ClientError::OOM);
        }
        assert!(rb.len() as i32 - self.bump_index as i32 >= 0);

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
        self.ring.submit().map_err(ClientError::IO)?;
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
        self.sockfd = Some(sockfd);

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
        self.ring.submit().map_err(ClientError::IO)?;
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
        self.ring.submit().map_err(ClientError::IO)?;
        self.sockfd = None;
        Ok(())
    }

    fn issue_handshake(&mut self, rb: &mut [u8], bytes: &[u8]) -> Result<()> {
        self.issue_write_underlying(bytes)?;

        if self.bump_index >= rb.len() {
            return Err(ClientError::OOM);
        }
        assert!(rb.len() as i32 - self.bump_index as i32 >= 0);

        let read = opcode::Recv::new(
            Fd(self.sockfd.unwrap().as_raw_fd()),
            rb[self.bump_index..].as_mut_ptr(),
            (rb.len() - self.bump_index) as u32,
        )
        .build()
        .user_data(Box::into_raw(Box::new(Operation::Handshake)) as u64);
        unsafe {
            self.ring
                .submission()
                .push(&read)
                .expect("submission queue is full");
        }
        self.ring.submit().map_err(ClientError::IO)?;
        Ok(())
    }
}

fn dns_lookup(url_str: String) -> Result<(OsSocketAddr, String, u16)> {
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
    Ok((socket_addr, host, port))
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

        let (_, _, port) = dns_lookup("http://localhost:8080".to_string()).unwrap();
        assert!(port == 8080);
    }

    #[test]
    fn dns_lookup_fail() {
        assert!(dns_lookup("".to_string()).is_err());
        assert!(dns_lookup("example.com".to_string()).is_err());
        assert!(dns_lookup("www.example.com".to_string()).is_err());
    }
}
