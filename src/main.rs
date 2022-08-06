mod command;
mod resp;

use std::collections::HashMap;

use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::task::spawn;

use crate::resp::RESP;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut listener = TcpListener::bind("127.0.0.1:6379").await?;

    loop {
        let (mut socket, _) = listener.accept().await?;

        spawn(async move {
            let mut buf = [0; 1024];
            let mut store: HashMap<String, String> = HashMap::new();

            loop {
                let num_bytes = match socket.read(&mut buf).await {
                    // socket closed
                    Ok(num_bytes) if num_bytes == 0 => {
                        println!("socket closed");
                        return;
                    }
                    Ok(num_bytes) => num_bytes,
                    Err(e) => {
                        eprintln!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                println!("num_bytes: {}", num_bytes);
                let raw_message = String::from_utf8_lossy(&buf[..num_bytes]);
                println!("raw_message: {:?}", raw_message);

                let parsed_resp = RESP::decode(&raw_message);
                println!("parsed_resp: {:?}", parsed_resp);

                let response = match parsed_resp {
                    Ok(r) => command::process_command(&r, &mut store).encode(),
                    Err(e) => {
                        eprintln!("failed to parse resp; err = {:?}", e);
                        return;
                    }
                };

                println!("response: {:?}", response.to_string());

                // Write the data back
                if let Err(e) = socket.write_all(&response.into_bytes()).await {
                    eprintln!("failed to write to socket; err = {:?}", e);
                    return;
                }
            }
        });
    }
}
