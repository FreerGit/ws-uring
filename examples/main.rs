use std::io::{self};
use ws_uring::client::{Client, State};

fn main() -> io::Result<()> {
    // Note that the read buffer is owned by you, the caller. This means that if you do not use/copy
    // the bytes from said buffer before issuing a new get/post request that memory will be overwritten.
    let mut rb = vec![0u8; 1024 * 1000];
    // There is a simple websocket echo server in test_server/server.js that you can run.
    let mut client = Client::new("http://localhost:8080".to_owned()).unwrap();

    // Start the connection
    client.issue_connect().unwrap();
    loop {
        match client.step(&mut rb) {
            Ok(State::Read(None)) => {
                println!("A empty message, this _most_ likely means a graceful disconnect");
                // Uncomment below to send the get request in a endless loop!
                // client.issue_connect().unwrap();
            }
            Ok(State::Read(Some(msg))) => {
                println!("Got message: {:?}", msg);

                // You can check what the opcode of the message is and act accordingly
                // match msg.opcode() {
                //     websocket_codec::Opcode::Text => todo!(),
                //     websocket_codec::Opcode::Binary => todo!(),
                //     websocket_codec::Opcode::Close => todo!(),
                //     websocket_codec::Opcode::Ping => todo!(),
                //     websocket_codec::Opcode::Pong => todo!(),
                // }

                // Uncomment below to send messages back and forth in a loop!
                // client.issue_write("Hello!").unwrap();
                client.issue_read(&mut rb).unwrap();
            }
            Ok(State::Connect) => {
                println!("Connected!");

                // Send of a frame and "wait" for the response
                client.issue_write("Hello!").unwrap();
                client.issue_read(&mut rb).unwrap();
            }
            Ok(State::Idle) => {}
            Err(e) => {
                println!("{:?}", e.to_string());
                break;
            }
        }
    }
    Ok(())
}
