use std::{
    collections::HashMap,
    env, fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};
use flate2::{write::GzEncoder, Compression};
struct HTTPRequest {
    method: String,
    req_parts: Vec<String>,
    headers: HashMap<String, String>,
    body: Option<String>,
}
fn send_response(mut stream: TcpStream, resp: &[u8]) {
    stream.write_all(resp).unwrap();
}
fn handle_connection(mut stream: TcpStream) {
    let mut buf_reader = BufReader::new(&mut stream);
    let mut headers = HashMap::new();
    let mut content_length = None;

    let mut request_line = String::new();
    buf_reader.read_line(&mut request_line).unwrap();
    let request_line = request_line.trim();
    for line in buf_reader.by_ref().lines() {
        let line = line.unwrap();
        if line.is_empty() {
            break;
        }

        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() == 2 {
            let key = parts[0].trim().to_string();
            let value = parts[1].trim().to_string();
            if key.to_lowercase() == "content-length" {
                content_length = Some(value.parse::<usize>().unwrap());
            }
            headers.insert(key, value);
        }
    }
    // Read body if Content-Length is specified
    let body = if let Some(length) = content_length {
        let mut body = String::new();
        buf_reader
            .by_ref()
            .take(length as u64)
            .read_to_string(&mut body)
            .unwrap();
        Some(body)
    } else {
        None
    };
    // The first line of a request is something like GET / HTTP/1.1

    let request_parts: Vec<&str> = request_line.split(' ').collect();
    let req: Vec<&str> = request_parts[1].split('/').collect();

    let request = HTTPRequest {
        method: String::from(request_parts[0]),
        req_parts: req.iter().map(|x| x.to_string()).collect::<Vec<String>>()[1..].to_vec(),
        headers,
        body,
    };
    match request.req_parts[..]
        .iter()
        .map(|x| x.as_str())
        .collect::<Vec<&str>>()[..]
    {
        ["files", filename] => {
            let args: Vec<String> = env::args().collect();
            let dir = args[2].clone();
            match request.method.as_str() {
                "GET" => {
                    match fs::read_to_string(format!("{dir}{filename}")) {
                        Ok(file_content) => {
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
                                file_content.chars().count(),
                                file_content
                            );
                            send_response(stream, response.as_bytes());
                        }
                        Err(_) => {
                            send_response(stream, b"HTTP/1.1 404 Not Found\r\n\r\n");
                        }
                    };
                }
                "POST" => {
                    match fs::write(format!("{dir}{filename}"), request.body.unwrap()) {
                        Ok(_) => {
                            let response = "HTTP/1.1 201 Created\r\n\r\n";
                            send_response(stream, response.as_bytes());
                        }
                        Err(_) => {
                            send_response(stream, b"HTTP/1.1 404 Not Found\r\n\r\n");
                        }
                    };
                }
                _ => send_response(stream, b"HTTP/1.1 404 Not Found\r\n\r\n"),
            }
        }
        ["user-agent"] => {
            if let Some(user_agent) = request.headers.get("User-Agent") {
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    user_agent.chars().count(),
                    user_agent
                );
                send_response(stream, response.as_bytes());
            } else {
                send_response(stream, b"HTTP/1.1 400 Bad Request\r\n\r\n");
            }
        }
        ["echo", param] => match request.headers.get("Accept-Encoding") {
            Some(encoding) => {
                if encoding.contains("gzip") {
                    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                    encoder.write_all(param.as_bytes()).unwrap();
                    let compressed = encoder.finish().unwrap();
                    let status_line = format!("HTTP/1.1 200 OK\r\nContent-Encoding: {}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n", "gzip", compressed.len());
                    let mut response = Vec::new();
                    response.extend_from_slice(status_line.as_bytes());
                    response.extend_from_slice(&compressed);
                    stream.write_all(&response).unwrap();
                } else {
                    let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                            param.chars().count(),
                            param
                        );
                    send_response(stream, response.as_bytes());
                }
            }
            _ => {
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    param.chars().count(),
                    param
                );
                send_response(stream, response.as_bytes());
            }
        },
        [""] => {
            send_response(stream, b"HTTP/1.1 200 OK\r\n\r\n");
        }
        _ => {
            send_response(stream, b"HTTP/1.1 404 Not Found\r\n\r\n");
        }
    }
}
fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");
    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream);
    }
}
