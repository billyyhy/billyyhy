use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

const PAGE: &str = include_str!("../index.html");

pub fn serve(port: u16) -> Result<(), String> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|err| format!("Could not open the page on port {port}: {err}"))?;
    println!("SpaceX daily page: http://127.0.0.1:{port}");
    for connection in listener.incoming() {
        match connection {
            Ok(stream) => {
                let _ = respond(stream);
            }
            Err(err) => eprintln!("Connection failed: {err}"),
        }
    }
    Ok(())
}

fn respond(mut stream: TcpStream) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|err| err.to_string())?;
    let mut buffer = [0_u8; 1024];
    let read = stream.read(&mut buffer).unwrap_or(0);
    let request = String::from_utf8_lossy(&buffer[..read]);
    let path = request.split_whitespace().nth(1).unwrap_or("/");
    if path.starts_with("/api/session") {
        let body = br#"{"ok":false,"note":"This page loads the session in the browser.","orderSent":false}"#;
        write_response(&mut stream, "application/json; charset=utf-8", body)
    } else {
        write_response(&mut stream, "text/html; charset=utf-8", PAGE.as_bytes())
    }
}

fn write_response(stream: &mut TcpStream, content_type: &str, body: &[u8]) -> Result<(), String> {
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n",
        body.len()
    );
    stream
        .write_all(header.as_bytes())
        .and_then(|_| stream.write_all(body))
        .map_err(|err| err.to_string())
}
