//! Utility functions for UUID generation and timestamp handling.
//!
//! This module provides deterministic helpers for generating UUIDs and
//! RFC3339/ISO timestamps consistent with Python's behavior.

pub mod timestamps;
mod uuid_utils;
pub mod validation;

pub use timestamps::{iso_timestamp, parse_timestamp, Timestamp, UnixPrecision};
pub use uuid_utils::{generate_uuid, generate_uuid_v7, UuidCollisionMonitor, UuidEvent};
pub use validation::{
    CycleError, InvalidNameError, MissingDependencyError, SelfDependencyError,
    ValidationError, validate_all, validate_dag, validate_dependencies_exist,
    validate_no_self_dependencies, validate_stage_name,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_uuid_is_valid() {
        let id = generate_uuid();
        assert_eq!(id.get_version_num(), 4);
    }

    #[test]
    fn test_iso_timestamp_format() {
        let ts = iso_timestamp();
        // Should be RFC3339 format: YYYY-MM-DDTHH:MM:SS.ssssss+00:00 or Z
        assert!(ts.contains('T'));
        assert!(ts.contains(':'));
    }
}
