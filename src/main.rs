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
    Integer(i64),
    BulkString(Option<String>),
    Array(Vec<RESP>),
}

impl Clone for RESP {
    fn clone(&self) -> Self {
        match self {
            RESP::SimpleString(s) => RESP::SimpleString(s.clone()),
            RESP::Error(s) => RESP::Error(s.clone()),
            RESP::Integer(i) => RESP::Integer(*i),
            RESP::BulkString(s) => RESP::BulkString(s.clone()),
            RESP::Array(a) => RESP::Array(a.clone()),
        }
    }
}

#[derive(Debug)]
enum RESPParseError {
    InvalidEncoding(String),
}

fn parse_resp(chars: &mut std::str::Chars) -> Result<RESP, String> {
    let encoding = match chars.next() {
        Some(c) => c,
        None => return Err("unexpected end of input".to_string()),
    };
    println!("encoding: {}", encoding);

    match encoding {
        '+' => parse_resp_simple_string(chars),
        '-' => parse_resp_error(chars),
        ':' => parse_resp_integer(chars),
        '$' => parse_resp_bulk_string(chars),
        '*' => parse_resp_array(chars),
        _ => Err("unknown encoding".to_string()),
    }
}

fn parse_resp_internal_string(chars: &mut std::str::Chars) -> Result<String, String> {
    let mut value = String::new();
    loop {
        match chars.next() {
            Some(c) if c == '\r' => match chars.next() {
                Some(c) if c == '\n' => break,
                _ => return Err("expected newline after carriage return".to_string()),
            },
            Some(c) => value.push(c),
            None => return Err("unexpected end of input".to_string()),
        }
    }
    Ok(value)
}

fn parse_resp_simple_string(chars: &mut std::str::Chars) -> Result<RESP, String> {
    match parse_resp_internal_string(chars) {
        Ok(v) => Ok(RESP::SimpleString(v)),
        Err(e) => Err(e),
    }
}

fn parse_resp_error(chars: &mut std::str::Chars) -> Result<RESP, String> {
    match parse_resp_internal_string(chars) {
        Ok(v) => Ok(RESP::Error(v)),
        Err(e) => Err(e),
    }
}

fn parse_resp_integer(chars: &mut std::str::Chars) -> Result<RESP, String> {
    let mut value = String::new();
    loop {
        match chars.next() {
            Some(c) if c == '\r' => match chars.next() {
                Some(c) if c == '\n' => break,
                _ => return Err("expected newline after carriage return".to_string()),
            },
            Some(c) => value.push(c),
            None => return Err("unexpected end of input".to_string()),
        }
    }
    match value.parse::<i64>() {
        Ok(i) => Ok(RESP::Integer(i)),
        Err(_) => Err("invalid integer".to_string()),
    }
}

fn parse_resp_bulk_string(chars: &mut std::str::Chars) -> Result<RESP, String> {
    let mut bytes: i32 = 0;
    bytes = loop {
        match chars.next() {
            Some(c) if c == '\r' => match chars.next() {
                Some(c) if c == '\n' => break bytes,
                _ => return Err("expected newline after carriage return".to_string()),
            },
            Some(c) if c == '-' => match chars.next() {
                Some(c) if c == '1' => match chars.next() {
                    Some(c) if c == '\r' => match chars.next() {
                        Some(c) if c == '\n' => return Ok(RESP::BulkString(None)),
                        _ => {
                            return Err("expected newline after nil bulk string carriage return"
                                .to_string())
                        }
                    },
                    _ => return Err("expected carriage return after nil bulk string".to_string()),
                },
                _ => return Err("expected digit 1 following minus for nil bulk string".to_string()),
            },
            Some(c) => {
                let d = match c.to_digit(10) {
                    Some(d) => d as i32,
                    None => return Err(format!("expected digit, found: {}", c)),
                };
                bytes = bytes * 10 + d;
            }
            None => return Err("unexpected end of input".to_string()),
        }
    };

    let mut value = String::new();
    while bytes > 0 {
        match chars.next() {
            Some(c) => {
                value.push(c);
                bytes -= c.len_utf8() as i32;
            }
            None => {
                return Err("unexpected end of input: bytes remaining in bulk string".to_string())
            }
        }
    }

    if bytes < 0 {
        return Err("unexpected input: less than 0 bytes remaining in bulk string".to_string());
    }

    if chars.next() != Some('\r') {
        return Err("expected carriage return after bulk string".to_string());
    }

    if chars.next() != Some('\n') {
        return Err("expected newline after bulk string carriage return".to_string());
    }

    Ok(RESP::BulkString(Some(value)))
}

fn parse_resp_array(chars: &mut std::str::Chars) -> Result<RESP, String> {
    let mut elements = 0;
    elements = loop {
        match chars.next() {
            Some(c) if c == '\r' => match chars.next() {
                Some(c) if c == '\n' => break elements,
                _ => return Err("expected newline after carriage return".to_string()),
            },
            Some(c) => {
                let d = match c.to_digit(10) {
                    Some(d) => d,
                    None => return Err("expected digit".to_string()),
                };
                elements = elements * 10 + d;
            }
            None => return Err("unexpected end of input".to_string()),
        }
    };

    println!("elements: {}", elements);
    let mut resp_array: Vec<RESP> = Vec::new();

    for _ in 0..elements {
        let resp = parse_resp(chars);
        println!("parsed array element: {:?}", resp);
        match resp {
            Ok(resp) => resp_array.push(resp),
            Err(e) => return Err(e),
        }
    }

    println!("array: {:?}", resp_array);
    return Ok(RESP::Array(resp_array));
}

impl RESP {
    fn from_string(s: &str) -> Result<Self, RESPParseError> {
        if !s.ends_with("\r\n") {
            return Err(RESPParseError::InvalidEncoding(format!(
                "missing newline at end of input: {}",
                s
            )));
        }

        let mut chars = s.chars();
        match parse_resp(&mut chars) {
            Ok(resp) => Ok(resp),
            Err(e) => Err(RESPParseError::InvalidEncoding(format!("{}: {}", e, s))),
        }
    }
    fn to_string(&self) -> String {
        match self {
            RESP::SimpleString(s) => format!("+{}\r\n", s),
            RESP::Error(s) => format!("-{}\r\n", s),
            RESP::Integer(s) => format!(":{}\r\n", s),
            RESP::BulkString(s) => match s {
                Some(s) => {
                    println!("bulk string: {:?}", s);
                    format!("${}\r\n{}\r\n", s.len(), s)
                }
                None => "$-1\r\n".to_string(),
            },
            RESP::Array(s) => format!("*{:?}\r\n", s),
        }
    }
}

fn process_command(resp: &RESP) -> RESP {
    let (command, args) = match extract_command_args(resp) {
        Ok((command, args)) => (command, args),
        Err(e) => return RESP::Error(e.to_string()),
    };

    match command.as_str() {
        "PING" => RESP::SimpleString("PONG".to_string()),
        "ECHO" => args[0].clone(),
        _ => RESP::Error("ERR unknown command".to_string()),
    }
}

fn extract_command_args(resp: &RESP) -> Result<(String, Vec<RESP>), RESP> {
    match resp {
        RESP::Array(a) => match a.get(0) {
            Some(RESP::BulkString(Some(command))) => {
                Ok((command.to_string(), a.get(1..).unwrap().to_vec()))
            }
            _ => Err(RESP::Error("ERR invalid command".to_string())),
        },
        _ => Err(RESP::Error("ERR invalid command".to_string())),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut listener = TcpListener::bind("127.0.0.1:6379").await?;

    loop {
        let (mut socket, _) = listener.accept().await?;

        spawn(async move {
            let mut buf = [0; 1024];

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

                let parsed_resp = RESP::from_string(&raw_message);
                println!("parsed_resp: {:?}", parsed_resp);

                let response = match parsed_resp {
                    Ok(r) => process_command(&r).to_string(),
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
