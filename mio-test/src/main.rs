use mio::net::TcpStream;
use mio::{Events, Interest, Poll, Token};
use rustls::ClientConfig;
use rustls::OwnedTrustAnchor;
use rustls::RootCertStore;
use rustls::ServerName;
use rustls::Stream;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time;

const SERVER: Token = Token(0);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a root certificate store
    let mut root_cert_store = RootCertStore::empty();
    root_cert_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
        OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    }));

    // Create a rustls client config
    let config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();

    // Wrap it in an Arc
    let config = Arc::new(config);

    let mut begin = time::Instant::now();

    // Create a DNS name for the server
    let dns_name = ServerName::try_from("google.com").unwrap();

    // Create a TcpStream and set it to non-blocking
    let addr: SocketAddr = "216.58.207.238:443".parse().unwrap(); // example.com IP

    let mut stream = TcpStream::connect(addr)?;
    // stream.
    // stream.set_nonblocking(true)?;
    // stream.set

    // Create a Poll instance
    let mut poll = Poll::new()?;
    poll.registry()
        .register(&mut stream, SERVER, Interest::READABLE | Interest::WRITABLE)?;

    // Create a buffer for handling events
    let mut events = Events::with_capacity(128);

    // Create a client session
    let mut client = rustls::ClientConnection::new(config, dns_name)?;
    let mut tls_stream = Stream::new(&mut client, &mut stream);

    let mut end = time::Instant::now();
    print!("handshake {:?}\n", end - begin);

    begin = time::Instant::now();

    // Handshake loop
    // let mut sb = false;
    'outer: loop {
        // if sb {
        //     break;
        // }
        // poll.poll(&mut events, None)?;

        // println!("hej");
        match tls_stream.conn.complete_io(tls_stream.sock) {
            Ok(_) => {
                println!("TLS handshake completed");
                break 'outer;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(e) => return Err(Box::new(e)),
        }
    }

    end = time::Instant::now();
    print!("TLS handshake {:?}\n", end - begin);

    // Send a request
    tls_stream.write_all(b"GET / HTTP/1.0\r\n\r\n")?;
    println!("Sent request");

    begin = time::Instant::now();

    // Read the response
    // let mut response = Vec::new();
    let mut buf = [0; 4096];
    let mut x = 0;
    'outer: loop {
        // poll.poll(&mut events, None)?;

        for event in events.iter() {
            if event.token() == SERVER && event.is_readable() {
                match tls_stream.read(&mut buf) {
                    Ok(0) => {
                        println!("Connection closed");
                        // print!("{:#?}", response.str);
                        // return Ok(());
                        break 'outer;
                    }
                    Ok(m) => {
                        x = m;
                        // return Ok(());
                        break 'outer;
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(e) => return Err(Box::new(e)),
                }
            }
        }
    }
    end = time::Instant::now();
    print!("{:?}\n", end - begin);
    print!("{:?}\n", std::str::from_utf8(&buf[..x]).unwrap());
    return Ok(());
}
