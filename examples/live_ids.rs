//! Parse a saved Forex Factory page and report how many events carry an id.
//!
//! A committed fixture can drift from the live site, so this checks the real
//! thing: `curl ... 'https://www.forexfactory.com/calendar?week=this' -o ff.html`
//! then `cargo run --example live_ids -- ff.html`.
fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/ff.html".into());
    let html = std::fs::read_to_string(&path).expect("read the saved page");
    let parser = forex_factory::CalendarParser::new().expect("parser");
    let base = chrono::NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();
    let events = parser.parse(&html, base);
    let with = events.iter().filter(|e| e.event_id.is_some()).count();
    println!("parsed {} events, {with} carry an id", events.len());
    for e in events.iter().take(4) {
        println!("  {:?}  {}  -> {:?}", e.event_id, e.name, e.detail_url());
    }
}
