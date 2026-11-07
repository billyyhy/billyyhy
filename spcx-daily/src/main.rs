mod gui;

use spcx_daily::new_york_daily_max;

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

    println!("HFT dashboard");
    println!("Cointegration  equity 50.00 (start 50 + pnl 0.00) · 0W/0L · 0 open");
    println!("Weather        equity 50.00 (start 50 + pnl 0.00) · 0W/0L · 32 open");
    match new_york_daily_max() {
        Ok((day, temperature)) => println!(
            "New York daily max {temperature:.1} C on {day} (Open-Meteo). Weather book stays on the ledger until a market is priced."
        ),
        Err(err) => eprintln!("{err}"),
    }
    println!("Options book   paper flat · session open · 0 open · pnl -344.00 · long premium off");
    println!("NFLX call 66   17 @ 0.22   resolved   -34.00");
    println!("NFLX call 67   22 @ 0.17   resolved   -121.00");
    println!("NFLX call 68   18 @ 0.20   resolved   -189.00");
}
