mod gui;

use chrono::Utc;
use spcx_daily::{fetch_open_markets, session_snapshot};

fn main() {
    if std::env::args().any(|arg| arg == "--gui") {
        let port = std::env::args()
            .position(|arg| arg == "--port")
            .and_then(|index| std::env::args().nth(index + 1))
            .and_then(|value| value.parse().ok())
            .unwrap_or(8734);
        if let Err(err) = gui::serve(port) {
            eprintln!("{err}");
            std::process::exit(1);
        }
        return;
    }

    let now = Utc::now();
    let markets = match fetch_open_markets() {
        Ok(markets) => markets,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };
    let snapshot = session_snapshot(now, &markets);
    if snapshot["ok"].as_bool() != Some(true) {
        println!("{}", snapshot["note"].as_str().unwrap_or("No open SpaceX daily market."));
        return;
    }

    let close_hk = snapshot["closesHongKong"].as_str().unwrap_or("");
    println!("{}", snapshot["title"].as_str().unwrap_or(""));
    println!("{}", snapshot["url"].as_str().unwrap_or(""));
    println!(
        "Up {:.3}    Down {:.3}",
        snapshot["up"].as_f64().unwrap_or(0.0),
        snapshot["down"].as_f64().unwrap_or(0.0)
    );
    println!(
        "This contract closes {close_hk} Hong Kong time. After that, run this again and it selects the next day."
    );
    match snapshot["nextUrl"].as_str() {
        Some(next) => println!("Next session: {next}"),
        None => println!("Next session is not listed yet."),
    }
    println!("{}", snapshot["note"].as_str().unwrap_or("No order was sent."));
}
