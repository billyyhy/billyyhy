use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use chrono::Utc;

use spcx_daily::{fetch_open_markets, session_snapshot};

const PAGE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>SpaceX daily</title>
<style>
  :root { color-scheme: dark; }
  body { margin: 0; font-family: Georgia, "Iowan Old Style", serif; background: #14120e; color: #f4efe4; }
  main { max-width: 720px; margin: 0 auto; padding: 48px 24px 64px; }
  p.kicker { letter-spacing: 0.14em; text-transform: uppercase; font-family: "Segoe UI", sans-serif; font-size: 12px; color: #b7a98a; margin: 0 0 8px; }
  h1 { font-weight: 500; font-size: 34px; line-height: 1.2; margin: 0 0 28px; }
  .prices { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
  .card { background: #221e18; border: 1px solid #3a3226; border-radius: 16px; padding: 20px 22px; }
  .card span { display: block; font-family: "Segoe UI", sans-serif; font-size: 13px; letter-spacing: 0.08em; text-transform: uppercase; color: #b7a98a; }
  .card strong { display: block; margin-top: 8px; font-size: 48px; font-weight: 500; }
  .up strong { color: #d7e7c8; }
  .down strong { color: #f0c7b0; }
  .meta { margin-top: 28px; font-family: "Segoe UI", sans-serif; line-height: 1.55; color: #d9d0c0; }
  a { color: #f0d48a; }
  .warn { margin-top: 22px; padding: 12px 14px; border-radius: 10px; background: #2a2418; color: #f0d48a; font-family: "Segoe UI", sans-serif; }
</style>
</head>
<body>
<main>
  <p class="kicker">SpaceX daily</p>
  <h1 id="title">Loading the session…</h1>
  <div class="prices">
    <div class="card up"><span>Up</span><strong id="up">—</strong></div>
    <div class="card down"><span>Down</span><strong id="down">—</strong></div>
  </div>
  <div class="meta" id="meta"></div>
  <div class="warn" id="note"></div>
</main>
<script>
async function refresh() {
  const response = await fetch("/api/session");
  const session = await response.json();
  const title = document.getElementById("title");
  const note = document.getElementById("note");
  note.textContent = session.note || "";
  if (!session.ok) {
    title.textContent = "No session is open";
    document.getElementById("up").textContent = "—";
    document.getElementById("down").textContent = "—";
    document.getElementById("meta").textContent = "";
    return;
  }
  title.innerHTML = '<a id="link"></a>';
  const link = document.getElementById("link");
  link.href = session.url;
  link.textContent = session.title;
  document.getElementById("up").textContent = Number(session.up).toFixed(3);
  document.getElementById("down").textContent = Number(session.down).toFixed(3);
  const next = session.nextUrl
    ? 'Next session: <a href="' + session.nextUrl + '">' + session.nextTitle + '</a>.'
    : "Next session is not listed yet.";
  document.getElementById("meta").innerHTML =
    "This contract closes " + session.closesHongKong + " Hong Kong time. After that, this page selects the next day.<br>" + next;
}
refresh();
setInterval(refresh, 30000);
</script>
</body>
</html>
"#;

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
        let body = match fetch_open_markets() {
            Ok(markets) => session_snapshot(Utc::now(), &markets).to_string(),
            Err(err) => serde_json::json!({
                "ok": false,
                "note": err,
                "orderSent": false
            })
            .to_string(),
        };
        write_response(&mut stream, "application/json; charset=utf-8", body.as_bytes())
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
