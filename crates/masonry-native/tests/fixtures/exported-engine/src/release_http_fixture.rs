use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

use masonry::{ClientMessage, Connect, CoreErrorCode, messagepack};
use masonry_native::Engine;
use masonry_rules::{FlashPayload, create_engine};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind release fixture");
    println!("http://{}", listener.local_addr().expect("fixture address"));
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(|| serve(stream));
            }
            Err(error) => eprintln!("release fixture accept failed: {error}"),
        }
    }
}

fn serve(mut stream: TcpStream) {
    let mut engine = create_engine().expect("create fixture engine");
    while let Some(request) = read_request(&mut stream) {
        let response = match (request.method.as_str(), request.path.as_str()) {
            ("POST", "/connect") => messagepack::from_slice::<Connect>(&request.body)
                .map_err(|error| error.to_string())
                .and_then(|message| engine.connect(message).map_err(|error| error.to_string()))
                .and_then(|response| messagepack::to_vec(&response).map_err(|e| e.to_string())),
            ("POST", "/messages") => {
                messagepack::from_slice::<ClientMessage<FlashPayload, CoreErrorCode>>(&request.body)
                    .map_err(|error| error.to_string())
                    .and_then(|message| engine.submit(message).map_err(|error| error.to_string()))
                    .and_then(|response| messagepack::to_vec(&response).map_err(|e| e.to_string()))
            }
            ("GET", "/poll") => match engine.poll() {
                Ok(Some(response)) => messagepack::to_vec(&response).map_err(|e| e.to_string()),
                Ok(None) => {
                    write_response(&mut stream, 204, "application/msgpack", &[]);
                    continue;
                }
                Err(error) => Err(error.to_string()),
            },
            _ => {
                write_response(&mut stream, 404, "text/plain", b"unknown fixture route");
                continue;
            }
        };
        match response {
            Ok(body) => write_response(&mut stream, 200, "application/msgpack", &body),
            Err(error) => write_response(&mut stream, 500, "text/plain", error.as_bytes()),
        }
    }
}

struct Request {
    method: String,
    path: String,
    body: Vec<u8>,
}

fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).ok()? == 0 {
        return None;
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_owned();
    let path = parts.next()?.to_owned();
    let mut content_length = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        if line == "\r\n" {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            content_length = value.trim().parse().ok()?;
        }
    }
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body).ok()?;
    Some(Request { method, path, body })
}

fn write_response(stream: &mut TcpStream, status: u16, content_type: &str, body: &[u8]) {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nContent-Type: {content_type}\r\nConnection: keep-alive\r\n\r\n",
        body.len()
    )
    .expect("write fixture headers");
    stream.write_all(body).expect("write fixture body");
    stream.flush().expect("flush fixture response");
}
