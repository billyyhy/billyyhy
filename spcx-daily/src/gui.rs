use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use spcx_daily::new_york_daily_max;

const PAGE: &str = include_str!("../index.html");

pub fn serve(port: u16) -> Result<(), String> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|err| format!("Could not open the page on port {port}: {err}"))?;
    println!("HFT dashboard: http://127.0.0.1:{port}");
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
    let _ = stream.read(&mut buffer);
    let page = PAGE.replacen("__WEATHER__", &escape(&weather_line()), 1);
    write_response(&mut stream, "text/html; charset=utf-8", page.as_bytes())
}

fn weather_line() -> String {
    match new_york_daily_max() {
        Ok((day, temperature)) => format!(
            "New York daily max {temperature:.1} C on {day} (Open-Meteo). Weather book stays on the ledger until a market is priced."
        ),
        Err(_) => "Weather book stays on the ledger until a market is priced.".to_string(),
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
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
