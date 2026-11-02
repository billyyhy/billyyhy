//! The SpaceX daily market on Polymarket is one contract per US session.
//! `spcx-up-or-down-on-november-2-2026` stops trading at that session's close.
//! The next session is a different event. This crate picks the open contract
//! whose close is soonest, so the same command follows the series every day.

use chrono::{DateTime, Utc};
use serde_json::Value;

pub const SERIES_SLUG: &str = "spcx-daily-up-or-down";
pub const GAMMA_EVENTS: &str = "https://gamma-api.polymarket.com/events";

#[derive(Debug, Clone, PartialEq)]
pub struct DailyMarket {
    pub slug: String,
    pub title: String,
    pub end: DateTime<Utc>,
    pub up: f64,
    pub down: f64,
    pub accepting_orders: bool,
    pub closed: bool,
}

impl DailyMarket {
    pub fn url(&self) -> String {
        format!("https://polymarket.com/event/{}", self.slug)
    }
}

/// Current session: still open, still accepting orders, and the soonest close.
/// After that close, the same rule returns the next listed day.
pub fn select_session(now: DateTime<Utc>, markets: &[DailyMarket]) -> Option<DailyMarket> {
    markets
        .iter()
        .filter(|market| market.accepting_orders && !market.closed && market.end > now)
        .min_by_key(|market| market.end)
        .cloned()
}

/// The following listed session, whether or not it is accepting orders yet.
pub fn following_session(current: &DailyMarket, markets: &[DailyMarket]) -> Option<DailyMarket> {
    markets
        .iter()
        .filter(|market| !market.closed && market.end > current.end)
        .min_by_key(|market| market.end)
        .cloned()
}

pub fn fetch_open_markets() -> Result<Vec<DailyMarket>, String> {
    let url = format!(
        "{GAMMA_EVENTS}?series_slug={SERIES_SLUG}&closed=false&active=true&limit=20&order=endDate&ascending=true"
    );
    let response = ureq::get(&url)
        .set("Accept", "application/json")
        .set("User-Agent", "spcx-daily/0.1")
        .call()
        .map_err(|err| format!("Polymarket request failed: {err}"))?;
    let body = response
        .into_string()
        .map_err(|err| format!("Polymarket response was not text: {err}"))?;
    let value: Value = serde_json::from_str(&body)
        .map_err(|err| format!("Polymarket response was not JSON: {err}"))?;
    parse_events(&value)
}

pub fn parse_events(value: &Value) -> Result<Vec<DailyMarket>, String> {
    let events = value
        .as_array()
        .ok_or_else(|| "Polymarket events response was not a list".to_string())?;
    let mut markets = Vec::new();
    for event in events {
        if let Some(market) = parse_event(event) {
            markets.push(market);
        }
    }
    Ok(markets)
}

fn parse_event(event: &Value) -> Option<DailyMarket> {
    let slug = event.get("slug")?.as_str()?.to_string();
    let title = event
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or(slug.as_str())
        .to_string();
    let end = parse_time(event.get("endDate")?.as_str()?)?;
    let closed = event.get("closed").and_then(Value::as_bool).unwrap_or(false);
    let market = event.get("markets")?.as_array()?.first()?;
    let accepting_orders = market
        .get("acceptingOrders")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let (up, down) = up_down_prices(market)?;
    Some(DailyMarket {
        slug,
        title,
        end,
        up,
        down,
        accepting_orders,
        closed,
    })
}

fn up_down_prices(market: &Value) -> Option<(f64, f64)> {
    let outcomes = json_list(market.get("outcomes")?)?;
    let prices = json_list(market.get("outcomePrices")?)?;
    if outcomes.len() != prices.len() {
        return None;
    }
    let mut up = None;
    let mut down = None;
    for (outcome, price) in outcomes.iter().zip(prices.iter()) {
        let label = outcome.as_str()?.to_ascii_lowercase();
        let price = price.as_str()?.parse::<f64>().ok()?;
        if label == "up" {
            up = Some(price);
        } else if label == "down" {
            down = Some(price);
        }
    }
    Some((up?, down?))
}

fn json_list(value: &Value) -> Option<Vec<Value>> {
    if let Some(items) = value.as_array() {
        return Some(items.clone());
    }
    let text = value.as_str()?;
    serde_json::from_str::<Vec<Value>>(text).ok()
}

fn parse_time(text: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|time| time.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn market(slug: &str, end: &str, up: f64, accepting: bool, closed: bool) -> DailyMarket {
        DailyMarket {
            slug: slug.to_string(),
            title: slug.to_string(),
            end: parse_time(end).unwrap(),
            up,
            down: 1.0 - up,
            accepting_orders: accepting,
            closed,
        }
    }

    #[test]
    fn keeps_the_session_that_closes_next() {
        let markets = vec![
            market(
                "spcx-up-or-down-on-november-2-2026",
                "2026-11-02T21:00:00Z",
                0.52,
                true,
                false,
            ),
            market(
                "spcx-up-or-down-on-november-3-2026",
                "2026-11-03T21:00:00Z",
                0.50,
                true,
                false,
            ),
        ];
        let now = Utc.with_ymd_and_hms(2026, 11, 2, 14, 30, 0).unwrap();
        let selected = select_session(now, &markets).unwrap();
        assert_eq!(selected.slug, "spcx-up-or-down-on-november-2-2026");
        assert_eq!(
            following_session(&selected, &markets).unwrap().slug,
            "spcx-up-or-down-on-november-3-2026"
        );
    }

    #[test]
    fn rolls_after_the_session_close() {
        let markets = vec![
            market(
                "spcx-up-or-down-on-november-2-2026",
                "2026-11-02T21:00:00Z",
                0.52,
                true,
                false,
            ),
            market(
                "spcx-up-or-down-on-november-3-2026",
                "2026-11-03T21:00:00Z",
                0.50,
                true,
                false,
            ),
        ];
        let now = Utc.with_ymd_and_hms(2026, 11, 2, 21, 0, 0).unwrap();
        let selected = select_session(now, &markets).unwrap();
        assert_eq!(selected.slug, "spcx-up-or-down-on-november-3-2026");
    }

    #[test]
    fn skips_closed_and_weekend_gaps() {
        let markets = vec![market(
            "spcx-up-or-down-on-october-30-2026",
            "2026-10-30T20:00:00Z",
            0.0,
            false,
            true,
        )];
        let now = Utc.with_ymd_and_hms(2026, 10, 31, 2, 0, 0).unwrap();
        assert!(select_session(now, &markets).is_none());
    }

    #[test]
    fn parses_stringified_outcome_prices() {
        let body = r#"[
          {
            "slug": "spcx-up-or-down-on-november-2-2026",
            "title": "SpaceX (SPCX) Up or Down on November 2?",
            "endDate": "2026-11-02T21:00:00Z",
            "closed": false,
            "markets": [{
              "acceptingOrders": true,
              "outcomes": "[\"Up\", \"Down\"]",
              "outcomePrices": "[\"0.52\", \"0.48\"]"
            }]
          }
        ]"#;
        let value: Value = serde_json::from_str(body).unwrap();
        let markets = parse_events(&value).unwrap();
        assert_eq!(markets.len(), 1);
        assert!((markets[0].up - 0.52).abs() < 1e-9);
        assert!((markets[0].down - 0.48).abs() < 1e-9);
        assert!(markets[0].accepting_orders);
    }
}
