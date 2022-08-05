#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;

use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::task::spawn;

#[derive(Debug)]
enum RESP {
    SimpleString(String),
    Error(String),
    Integer(String),
    BulkString(String),
    Array(String),
}

#[derive(Debug)]
enum RESPParseError {
    InvalidEncoding(String),
}

impl RESP {
    fn into_bytes(&self) -> Vec<u8> {
        self.to_string().into_bytes()
    }
    fn from_string(s: &str) -> Result<Self, RESPParseError> {
        if s.ends_with("\r\n") {
            if s.starts_with("+") {
                return Ok(RESP::SimpleString(s[1..s.len() - 2].to_string()));
            } else if s.starts_with("-") {
                return Ok(RESP::Error(s[1..s.len() - 2].to_string()));
            } else if s.starts_with(":") {
                return Ok(RESP::Integer(s[1..s.len() - 2].to_string()));
            } else if s.starts_with("$") {
                return Ok(RESP::BulkString(s[1..s.len() - 2].to_string()));
            } else if s.starts_with("*") {
                return Ok(RESP::Array(s[1..s.len() - 2].to_string()));
            }
        }

        Err(RESPParseError::InvalidEncoding(s.to_string()))
    }
    fn to_string(&self) -> String {
        match self {
            RESP::SimpleString(s) => format!("+{}\r\n", s),
            RESP::Error(s) => format!("-{}\r\n", s),
            RESP::Integer(s) => format!(":{}\r\n", s),
            RESP::BulkString(s) => format!("${}\r\n{}\r\n", s.len(), s),
            RESP::Array(s) => format!("*{}\r\n", s.len()),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut listener = TcpListener::bind("127.0.0.1:6379").await?;

    loop {
        let (mut socket, _) = listener.accept().await?;

        spawn(async move {
            let mut buf = [0; 1024];

            // In a loop, read data from the socket and write the data back.
            loop {
                let num_bytes = match socket.read(&mut buf).await {
                    // socket closed
                    Ok(num_bytes) if num_bytes == 0 => return,
                    Ok(num_bytes) => num_bytes,
                    Err(e) => {
                        eprintln!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                println!("num_bytes: {}", num_bytes);
                let raw_message = String::from_utf8_lossy(&buf[..num_bytes]);
                println!("raw_message: {:?}", raw_message);

                // let response = match RESP::from_string(&raw_message) {
                //     Ok(message) => match message {
                //         RESP::SimpleString(_) => RESP::SimpleString("PONG".to_string()),
                //         RESP::Error(s) => RESP::Error(s),
                //         RESP::Integer(s) => RESP::Integer(s),
                //         RESP::BulkString(s) => RESP::BulkString(s),
                //         RESP::Array(s) => RESP::Array(s),
                //     },
                //     Err(e) => {
                //         eprintln!("failed to parse message; err = {:?}", e);
                //         RESP::Error(format!("ERR invalid message: {:?}", e))
                //     }
                // };

                // println!("response: {:?}", response.to_string());

                let response = RESP::SimpleString("PONG".to_string());

                // Write the data back
                if let Err(e) = socket.write_all(&response.into_bytes()).await {
                    eprintln!("failed to write to socket; err = {:?}", e);
                    return;
                }
            }
        });
    }
}
