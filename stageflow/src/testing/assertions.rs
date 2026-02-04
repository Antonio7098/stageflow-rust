//! Test assertions for stage outputs.

use crate::core::{StageOutput, StageStatus};

/// Asserts that the output indicates success.
pub fn assert_output_succeeded(output: &StageOutput) {
    assert!(
        output.is_success(),
        "Expected success, got status: {:?}",
        output.status
    );
}

/// Asserts that the output indicates failure.
pub fn assert_output_failed(output: &StageOutput) {
    assert!(
        output.is_failure(),
        "Expected failure, got status: {:?}",
        output.status
    );
}

/// Asserts that the output has the expected status.
pub fn assert_output_status(output: &StageOutput, expected: StageStatus) {
    assert_eq!(
        output.status, expected,
        "Expected status {:?}, got {:?}",
        expected, output.status
    );
}

/// Asserts that the output contains a specific key.
pub fn assert_output_contains(output: &StageOutput, key: &str) {
    assert!(
        output.get(key).is_some(),
        "Expected output to contain key '{}', but it doesn't. Keys: {:?}",
        key,
        output.data.as_ref().map(|d| d.keys().collect::<Vec<_>>())
    );
}

/// Asserts that the output has data (not empty).
pub fn assert_output_has_data(output: &StageOutput) {
    assert!(
        output.data.is_some() && !output.data.as_ref().unwrap().is_empty(),
        "Expected output to have data, but it's empty"
    );
}

/// Asserts that the output data contains a specific value.
pub fn assert_output_value(output: &StageOutput, key: &str, expected: &serde_json::Value) {
    let actual = output.get(key);
    assert_eq!(
        actual,
        Some(expected),
        "Expected value {:?} for key '{}', got {:?}",
        expected,
        key,
        actual
    );
}

/// Asserts that the output metadata contains a specific key.
pub fn assert_output_metadata(output: &StageOutput, key: &str) {
    assert!(
        output.metadata.contains_key(key),
        "Expected metadata to contain key '{}', but it doesn't",
        key
    );
}

/// Asserts that the output is retryable.
pub fn assert_output_retryable(output: &StageOutput) {
    assert!(
        output.is_retryable(),
        "Expected output to be retryable, but it isn't"
    );
}

/// Asserts that the output is not retryable.
pub fn assert_output_not_retryable(output: &StageOutput) {
    assert!(
        !output.is_retryable(),
        "Expected output to not be retryable, but it is"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_assert_output_succeeded() {
        let output = StageOutput::ok_empty();
        assert_output_succeeded(&output);
    }

    #[test]
    #[should_panic(expected = "Expected success")]
    fn test_assert_output_succeeded_fails() {
        let output = StageOutput::fail("error");
        assert_output_succeeded(&output);
    }

    #[test]
    fn test_assert_output_failed() {
        let output = StageOutput::fail("error");
        assert_output_failed(&output);
    }

    #[test]
    fn test_assert_output_status() {
        let output = StageOutput::skip("reason");
        assert_output_status(&output, StageStatus::Skip);
    }

    #[test]
    fn test_assert_output_contains() {
        let output = StageOutput::ok_value("key", serde_json::json!("value"));
        assert_output_contains(&output, "key");
    }

    #[test]
    fn test_assert_output_has_data() {
        let output = StageOutput::ok_value("key", serde_json::json!("value"));
        assert_output_has_data(&output);
    }

    #[test]
    fn test_assert_output_value() {
        let output = StageOutput::ok_value("count", serde_json::json!(42));
        assert_output_value(&output, "count", &serde_json::json!(42));
    }

    #[test]
    fn test_assert_output_retryable() {
        let output = StageOutput::fail_retryable("error");
        assert_output_retryable(&output);
    }

    #[test]
    fn test_assert_output_not_retryable() {
        let output = StageOutput::fail("error");
        assert_output_not_retryable(&output);
    }
}
