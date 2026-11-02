use chrono::{FixedOffset, Utc};
use spcx_daily::{fetch_open_markets, following_session, select_session};

fn main() {
    let now = Utc::now();
    let markets = match fetch_open_markets() {
        Ok(markets) => markets,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };
    let Some(session) = select_session(now, &markets) else {
        println!("No open SpaceX daily market.");
        println!("Polymarket lists the next weekday session when it is published. Weekends have no contract.");
        std::process::exit(0);
    };

    let hong_kong = FixedOffset::east_opt(8 * 3600).expect("UTC+8");
    let close_hk = session.end.with_timezone(&hong_kong);
    println!("{}", session.title);
    println!("{}", session.url());
    println!("Up {:.3}    Down {:.3}", session.up, session.down);
    println!(
        "This contract closes {} Hong Kong time. After that, run this again and it selects the next day.",
        close_hk.format("%Y-%m-%d %H:%M")
    );
    match following_session(&session, &markets) {
        Some(next) => println!("Next session: {}", next.url()),
        None => println!("Next session is not listed yet."),
    }
    println!("No order was sent.");
}
