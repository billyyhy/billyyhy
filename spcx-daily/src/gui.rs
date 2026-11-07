use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use chrono::Utc;
use serde_json::Value;
use spcx_daily::{fetch_open_markets, session_snapshot};

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
    let snapshot = match fetch_open_markets() {
        Ok(markets) => session_snapshot(Utc::now(), &markets),
        Err(err) => serde_json::json!({
            "ok": false,
            "note": err,
            "orderSent": false
        }),
    };
    if path.starts_with("/api/session") {
        let body = snapshot.to_string();
        write_response(&mut stream, "application/json; charset=utf-8", body.as_bytes())
    } else {
        let page = render_page(&snapshot);
        write_response(&mut stream, "text/html; charset=utf-8", page.as_bytes())
    }
}

fn render_page(snapshot: &Value) -> String {
    if snapshot["ok"].as_bool() != Some(true) {
        let note = escape(snapshot["note"].as_str().unwrap_or("No open SpaceX daily market."));
        return PAGE
            .replacen(
                "<h1 id=\"title\">Loading the session…</h1>",
                "<h1 id=\"title\">No session is open</h1>",
                1,
            )
            .replacen(
                "<div class=\"warn\" id=\"note\"></div>",
                &format!("<div class=\"warn\" id=\"note\">{note}</div>"),
                1,
            );
    }

    let title = escape(snapshot["title"].as_str().unwrap_or("SpaceX daily"));
    let url = snapshot["url"].as_str().unwrap_or("");
    let heading = if url.starts_with("https://polymarket.com/event/") {
        format!(
            "<h1 id=\"title\"><a href=\"{}\">{title}</a></h1>",
            escape(url)
        )
    } else {
        format!("<h1 id=\"title\">{title}</h1>")
    };
    let up = format_price(snapshot["up"].as_f64());
    let down = format_price(snapshot["down"].as_f64());
    let close = escape(snapshot["closesHongKong"].as_str().unwrap_or(""));
    let note = escape(snapshot["note"].as_str().unwrap_or("No order was sent."));
    let next = match (
        snapshot["nextUrl"].as_str(),
        snapshot["nextTitle"].as_str(),
    ) {
        (Some(next_url), Some(next_title)) if next_url.starts_with("https://polymarket.com/event/") => {
            format!(
                "Next session: <a href=\"{}\">{}</a>.",
                escape(next_url),
                escape(next_title)
            )
        }
        _ => "Next session is not listed yet.".to_string(),
    };
    let meta = format!(
        "This contract closes {close} Hong Kong time. After that, this page selects the next day.<br>{next}"
    );
    PAGE.replacen("<h1 id=\"title\">Loading the session…</h1>", &heading, 1)
        .replacen("<strong id=\"up\">—</strong>", &format!("<strong id=\"up\">{up}</strong>"), 1)
        .replacen(
            "<strong id=\"down\">—</strong>",
            &format!("<strong id=\"down\">{down}</strong>"),
            1,
        )
        .replacen(
            "<div class=\"meta\" id=\"meta\"></div>",
            &format!("<div class=\"meta\" id=\"meta\">{meta}</div>"),
            1,
        )
        .replacen(
            "<div class=\"warn\" id=\"note\"></div>",
            &format!("<div class=\"warn\" id=\"note\">{note}</div>"),
            1,
        )
}

fn format_price(price: Option<f64>) -> String {
    match price {
        Some(value) => format!("{value:.3}"),
        None => "—".to_string(),
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
