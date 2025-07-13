use chrono::{TimeZone, Utc};

// A couple of helper functions to format data for the API response.

const SATS_PER_BTC: f64 = 100_000_000.0;

/// Converts capacity from sats (i64) to a formatted BTC string.
pub fn format_capacity(sats: i64) -> String {
    let btc = sats as f64 / SATS_PER_BTC;
    format!("{:.8}", btc)
}

/// Converts a Unix timestamp (i64) into a readable date string (RFC3339 format).
pub fn format_timestamp(ts: i64) -> String {
    // Create a `DateTime<Utc>` object from the timestamp.
    let datetime = Utc.timestamp_opt(ts, 0).single();

    // Format it into a standard date string.
    if let Some(dt) = datetime {
        dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    } else {
        // Fallback for invalid timestamps.
        "Invalid Timestamp".to_string()
    }
} 