//! Timestamp utilities matching Python's datetime behavior.

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use std::str::FromStr;
use thiserror::Error;

/// Represents a timestamp that can be serialized/deserialized.
pub type Timestamp = DateTime<Utc>;

/// Errors that can occur during timestamp parsing.
#[derive(Debug, Error)]
pub enum TimestampError {
    /// The input type is not supported.
    #[error("Unsupported timestamp type: expected string or number")]
    UnsupportedType,

    /// The timestamp string is empty.
    #[error("Empty timestamp string")]
    EmptyString,

    /// The timestamp value is invalid.
    #[error("Invalid timestamp: {0}")]
    InvalidFormat(String),

    /// Nanosecond precision is not supported.
    #[error("Nanosecond precision timestamps are not supported")]
    NanosecondPrecision,
}

/// Detected precision of a Unix timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnixPrecision {
    /// Seconds (<=10 digits)
    Seconds,
    /// Milliseconds (<=13 digits)
    Milliseconds,
    /// Microseconds (<=16 digits)
    Microseconds,
}

/// Returns the current UTC time as an ISO 8601 formatted string.
///
/// The format matches Python's `datetime.now(UTC).isoformat()`:
/// `YYYY-MM-DDTHH:MM:SS.ffffff+00:00`
///
/// # Examples
///
/// ```
/// use stageflow::utils::iso_timestamp;
///
/// let ts = iso_timestamp();
/// assert!(ts.contains('T'));
/// assert!(ts.ends_with("+00:00") || ts.ends_with("Z"));
/// ```
#[must_use]
pub fn iso_timestamp() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%S%.6f+00:00").to_string()
}

/// Returns the current UTC timestamp.
#[must_use]
pub fn now_utc() -> Timestamp {
    Utc::now()
}

/// Detects the precision of a Unix timestamp based on digit count.
///
/// # Arguments
///
/// * `timestamp` - A numeric timestamp value
///
/// # Returns
///
/// The detected precision, or an error if nanoseconds are detected.
///
/// # Errors
///
/// Returns `TimestampError::NanosecondPrecision` if the timestamp has more than 16 digits.
pub fn detect_unix_precision(timestamp: f64) -> Result<UnixPrecision, TimestampError> {
    let abs_ts = timestamp.abs();
    let int_part = abs_ts.trunc() as i64;
    let digit_count = if int_part == 0 {
        1
    } else {
        int_part.abs().to_string().len()
    };

    match digit_count {
        0..=10 => Ok(UnixPrecision::Seconds),
        11..=13 => Ok(UnixPrecision::Milliseconds),
        14..=16 => Ok(UnixPrecision::Microseconds),
        _ => Err(TimestampError::NanosecondPrecision),
    }
}

/// Parses a timestamp from various formats.
///
/// Supports:
/// - Unix timestamps (seconds, milliseconds, microseconds)
/// - ISO 8601 strings
/// - RFC 2822 strings
/// - Common human-readable formats
///
/// # Arguments
///
/// * `input` - The timestamp to parse (string or number)
///
/// # Returns
///
/// A UTC `DateTime` on success.
///
/// # Errors
///
/// Returns `TimestampError` if the input cannot be parsed.
pub fn parse_timestamp(input: &str) -> Result<Timestamp, TimestampError> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Err(TimestampError::EmptyString);
    }

    // Try parsing as a number first
    if let Ok(num) = trimmed.parse::<f64>() {
        return parse_unix_timestamp(num);
    }

    // Try ISO 8601
    if let Ok(dt) = parse_iso8601(trimmed) {
        return Ok(dt);
    }

    // Try RFC 2822
    if let Ok(dt) = DateTime::parse_from_rfc2822(trimmed) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Try common formats
    if let Ok(dt) = parse_human_readable(trimmed) {
        return Ok(dt);
    }

    Err(TimestampError::InvalidFormat(trimmed.to_string()))
}

/// Parses a Unix timestamp with automatic precision detection.
pub fn parse_unix_timestamp(value: f64) -> Result<Timestamp, TimestampError> {
    // If the value has a fractional part, treat it as seconds
    let has_fraction = (value - value.trunc()).abs() > f64::EPSILON;

    if has_fraction {
        // Treat as seconds with fractional part
        let secs = value.trunc() as i64;
        let nanos = ((value.fract().abs()) * 1_000_000_000.0) as u32;
        return Utc
            .timestamp_opt(secs, nanos)
            .single()
            .ok_or_else(|| TimestampError::InvalidFormat(value.to_string()));
    }

    let precision = detect_unix_precision(value)?;
    let timestamp_i64 = value as i64;

    match precision {
        UnixPrecision::Seconds => Utc
            .timestamp_opt(timestamp_i64, 0)
            .single()
            .ok_or_else(|| TimestampError::InvalidFormat(value.to_string())),
        UnixPrecision::Milliseconds => {
            let secs = timestamp_i64 / 1000;
            let nanos = ((timestamp_i64 % 1000) * 1_000_000) as u32;
            Utc.timestamp_opt(secs, nanos)
                .single()
                .ok_or_else(|| TimestampError::InvalidFormat(value.to_string()))
        }
        UnixPrecision::Microseconds => {
            let secs = timestamp_i64 / 1_000_000;
            let nanos = ((timestamp_i64 % 1_000_000) * 1000) as u32;
            Utc.timestamp_opt(secs, nanos)
                .single()
                .ok_or_else(|| TimestampError::InvalidFormat(value.to_string()))
        }
    }
}

fn parse_iso8601(s: &str) -> Result<Timestamp, TimestampError> {
    // Handle 'Z' suffix by replacing with +00:00
    let normalized = s.replace('Z', "+00:00");

    // Try parsing with timezone
    if let Ok(dt) = DateTime::parse_from_rfc3339(&normalized) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Try various ISO formats
    let formats = [
        "%Y-%m-%dT%H:%M:%S%.f%:z",
        "%Y-%m-%dT%H:%M:%S%:z",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d",
    ];

    for fmt in formats {
        if let Ok(naive) = NaiveDateTime::parse_from_str(&normalized, fmt) {
            return Ok(Utc.from_utc_datetime(&naive));
        }
        // Try date-only formats
        if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(&normalized, fmt) {
            let naive_dt = naive_date.and_hms_opt(0, 0, 0).unwrap();
            return Ok(Utc.from_utc_datetime(&naive_dt));
        }
    }

    Err(TimestampError::InvalidFormat(s.to_string()))
}

fn parse_human_readable(s: &str) -> Result<Timestamp, TimestampError> {
    let formats = [
        "%B %d, %Y",           // October 5, 2023
        "%b %d, %Y",           // Oct 5, 2023
        "%d %B %Y",            // 5 October 2023
        "%d %b %Y",            // 5 Oct 2023
        "%m/%d/%Y",            // 10/05/2023
        "%d/%m/%Y",            // 05/10/2023
        "%Y/%m/%d",            // 2023/10/05
        "%B %d, %Y %H:%M:%S",  // October 5, 2023 14:30:00
        "%b %d, %Y %H:%M:%S",  // Oct 5, 2023 14:30:00
    ];

    for fmt in formats {
        if let Ok(naive) = NaiveDateTime::parse_from_str(s, fmt) {
            return Ok(Utc.from_utc_datetime(&naive));
        }
        if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(s, fmt) {
            let naive_dt = naive_date.and_hms_opt(0, 0, 0).unwrap();
            return Ok(Utc.from_utc_datetime(&naive_dt));
        }
    }

    Err(TimestampError::InvalidFormat(s.to_string()))
}

/// Formats a timestamp as ISO 8601 string.
#[must_use]
pub fn format_iso8601(dt: &Timestamp) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S%.6f+00:00").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_detect_unix_precision_seconds() {
        assert_eq!(
            detect_unix_precision(1696512000.0).unwrap(),
            UnixPrecision::Seconds
        );
    }

    #[test]
    fn test_detect_unix_precision_milliseconds() {
        assert_eq!(
            detect_unix_precision(1696512000000.0).unwrap(),
            UnixPrecision::Milliseconds
        );
    }

    #[test]
    fn test_detect_unix_precision_microseconds() {
        assert_eq!(
            detect_unix_precision(1696512000000000.0).unwrap(),
            UnixPrecision::Microseconds
        );
    }

    #[test]
    fn test_parse_iso8601() {
        let dt = parse_timestamp("2023-10-05T14:30:00Z").unwrap();
        assert_eq!(dt.year(), 2023);
        assert_eq!(dt.month(), 10);
        assert_eq!(dt.day(), 5);
    }

    #[test]
    fn test_parse_unix_seconds() {
        let dt = parse_timestamp("1696512000").unwrap();
        assert_eq!(dt.year(), 2023);
    }

    #[test]
    fn test_parse_empty_string() {
        assert!(matches!(
            parse_timestamp(""),
            Err(TimestampError::EmptyString)
        ));
    }

    #[test]
    fn test_iso_timestamp_format() {
        let ts = iso_timestamp();
        assert!(ts.contains('T'));
        assert!(ts.ends_with("+00:00"));
    }
}
