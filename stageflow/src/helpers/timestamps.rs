//! Timestamp parsing utilities.

pub use crate::utils::{
    parse_timestamp, UnixPrecision,
};

/// Re-export detect_unix_precision from utils
pub fn detect_unix_precision(timestamp: f64) -> Result<UnixPrecision, crate::utils::timestamps::TimestampError> {
    crate::utils::timestamps::detect_unix_precision(timestamp)
}

use chrono::{DateTime, FixedOffset, Utc};

/// Normalizes a datetime to UTC.
#[must_use]
pub fn normalize_to_utc(dt: DateTime<FixedOffset>, default_timezone: Option<FixedOffset>) -> DateTime<Utc> {
    dt.with_timezone(&Utc)
}
