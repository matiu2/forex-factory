//! Basic example demonstrating the Forex Factory library.
//!
//! Run with: `cargo run --example basic`

use forex_factory::{CalendarService, Currency, EventQuery, Impact, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the calendar service
    let service = CalendarService::new()?;

    println!("=== Today's Events ===\n");
    let today_events = service.get_today_events().await?;
    for event in &today_events {
        println!(
            "[{}] {} - {} ({})",
            event.currency,
            event.name,
            event.impact,
            event.datetime.format("%H:%M")
        );
    }
    println!("\nTotal: {} events\n", today_events.len());

    println!("=== High Impact USD Events This Week ===\n");
    let query = EventQuery::new()
        .with_currency(Currency::USD)
        .with_min_impact(Impact::High);
    let usd_events = service.query_events(&query).await?;
    for event in &usd_events {
        println!(
            "[{}] {} - {} stars",
            event.datetime.format("%a %H:%M"),
            event.name,
            event.impact.stars()
        );
        if let Some(ref forecast) = event.forecast {
            println!("    Forecast: {forecast}");
        }
        if let Some(ref previous) = event.previous {
            println!("    Previous: {previous}");
        }
    }
    println!("\nTotal: {} events", usd_events.len());

    Ok(())
}
