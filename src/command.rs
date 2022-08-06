use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::resp::RESP;

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

pub fn process_command(resp: &RESP, store: &mut HashMap<String, (String, u128)>) -> RESP {
    let (command, args) = match extract_command_args(resp) {
        Ok((command, args)) => (command, args),
        Err(e) => return e,
    };

    match command.to_uppercase().as_str() {
        "PING" => RESP::SimpleString("PONG".to_string()),
        "ECHO" => {
            if args.len() != 1 {
                RESP::Error("ERR wrong number of arguments for command".to_string())
            } else {
                args[0].clone()
            }
        }
        "SET" => {
            let num_args = args.len();
            if num_args < 2 {
                return RESP::Error("ERR wrong number of arguments for command".to_string());
            }

            let mut args_iter = args.iter();
            let key = args_iter.next().unwrap().to_string();
            let value = args_iter.next().unwrap().to_string();
            let mut expiry = 0;

            loop {
                let arg = match args_iter.next() {
                    Some(arg) => arg,
                    None => break,
                };

                match arg.to_string().to_uppercase().as_str() {
                    "EX" => {
                        let ttl_s = args_iter
                            .next()
                            .unwrap()
                            .to_string()
                            .parse::<u128>()
                            .unwrap();
                        expiry = now() + ttl_s * 1000;
                    }
                    "PX" => {
                        let ttl_ms = args_iter
                            .next()
                            .unwrap()
                            .to_string()
                            .parse::<u128>()
                            .unwrap();
                        expiry = now() + ttl_ms;
                    }
                    _ => {
                        return RESP::Error("ERR syntax error".to_string());
                    }
                }
            }

            store.insert(key, (value, expiry));
            RESP::SimpleString("OK".to_string())
        }
        "GET" => {
            let key = match args.get(0) {
                Some(key) => key.to_string(),
                None => {
                    return RESP::Error("ERR wrong number of arguments for command".to_string())
                }
            };

            match store.get(&key) {
                Some((value, ttl)) => {
                    if (ttl > &0) && (ttl < &now()) {
                        store.remove(&key);
                        RESP::BulkString(None)
                    } else {
                        RESP::BulkString(Some(value.to_string()))
                    }
                }
                None => RESP::BulkString(None),
            }
        }
        _ => RESP::Error(format!("unknown command '{}'", command)),
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
