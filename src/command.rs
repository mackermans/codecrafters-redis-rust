use std::collections::HashMap;

use crate::resp::RESP;

pub fn process_command(resp: &RESP, store: &mut HashMap<String, String>) -> RESP {
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
            let key = match args.get(0) {
                Some(key) => key.to_string(),
                None => {
                    return RESP::Error("ERR wrong number of arguments for command".to_string())
                }
            };

            let value = match args.get(1) {
                Some(value) => value.to_string(),
                None => {
                    return RESP::Error("ERR wrong number of arguments for command".to_string())
                }
            };

            store.insert(key, value);
            RESP::SimpleString("OK".to_string())
        }
        "GET" => match store.get(&args[0].to_string()) {
            Some(s) => RESP::BulkString(Some(s.to_string())),
            None => RESP::BulkString(None),
        },
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
