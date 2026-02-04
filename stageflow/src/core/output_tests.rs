//! Comprehensive tests for StageOutput.

#[cfg(test)]
mod tests {
    use crate::core::{StageOutput, StageStatus, StageArtifact, StageEvent};
    use std::collections::HashMap;

    #[test]
    fn test_output_ok_empty() {
        let output = StageOutput::ok_empty();
        assert!(output.is_success());
        assert!(!output.is_failure());
        assert_eq!(output.status, StageStatus::Ok);
    }

    #[test]
    fn test_output_ok_with_data() {
        let mut data = HashMap::new();
        data.insert("key".to_string(), serde_json::json!("value"));
        data.insert("count".to_string(), serde_json::json!(42));

        let output = StageOutput::ok(data);
        assert!(output.is_success());
        assert_eq!(output.get("key"), Some(&serde_json::json!("value")));
        assert_eq!(output.get("count"), Some(&serde_json::json!(42)));
    }

    #[test]
    fn test_output_ok_value() {
        let output = StageOutput::ok_value("result", serde_json::json!({"nested": true}));
        assert!(output.is_success());
        assert_eq!(output.get("result"), Some(&serde_json::json!({"nested": true})));
    }

    #[test]
    fn test_output_fail() {
        let output = StageOutput::fail("Something went wrong");
        assert!(output.is_failure());
        assert!(!output.is_success());
        assert!(!output.is_retryable());
        assert_eq!(output.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_output_fail_retryable() {
        let output = StageOutput::fail_retryable("Transient error");
        assert!(output.is_failure());
        assert!(output.is_retryable());
    }

    #[test]
    fn test_output_skip() {
        let output = StageOutput::skip("Not needed");
        assert_eq!(output.status, StageStatus::Skip);
        // Skip is considered a success (stage completed without error)
        assert!(output.is_success());
        assert!(!output.is_failure());
    }

    #[test]
    fn test_output_cancel() {
        let output = StageOutput::cancel("User cancelled");
        assert_eq!(output.status, StageStatus::Cancel);
    }

    #[test]
    fn test_output_retry() {
        let output = StageOutput::retry("Will retry");
        assert_eq!(output.status, StageStatus::Retry);
        assert!(output.is_retryable());
    }

    #[test]
    fn test_output_with_metadata() {
        let output = StageOutput::ok_empty()
            .add_metadata("version", serde_json::json!("1.0"))
            .add_metadata("timestamp", serde_json::json!(1234567890));

        assert_eq!(output.metadata.get("version"), Some(&serde_json::json!("1.0")));
        assert_eq!(output.metadata.get("timestamp"), Some(&serde_json::json!(1234567890)));
    }

    #[test]
    fn test_output_with_artifacts() {
        let artifact = StageArtifact::new(
            "file",
            "artifact-1",
            "report.pdf",
            serde_json::json!({"content": "base64data"}),
        );

        let output = StageOutput::ok_empty().with_artifacts(vec![artifact]);
        assert_eq!(output.artifacts.len(), 1);
        assert_eq!(output.artifacts[0].name, "report.pdf");
    }

    #[test]
    fn test_output_with_events() {
        let event = StageEvent::new("progress")
            .add_data("percent", serde_json::json!(50));

        let output = StageOutput::ok_empty().with_events(vec![event]);
        assert_eq!(output.events.len(), 1);
        assert_eq!(output.events[0].event_type, "progress");
    }

    #[test]
    fn test_output_data_or_empty() {
        let empty_output = StageOutput::ok_empty();
        assert!(empty_output.data_or_empty().is_empty());

        let output_with_data = StageOutput::ok_value("key", serde_json::json!(123));
        assert!(!output_with_data.data_or_empty().is_empty());
    }

    #[test]
    fn test_output_get_nonexistent() {
        let output = StageOutput::ok_empty();
        assert!(output.get("nonexistent").is_none());
    }

    #[test]
    fn test_output_to_dict() {
        let output = StageOutput::ok_value("test", serde_json::json!("value"));
        let dict = output.to_dict();

        assert!(dict.contains_key("status"));
        assert!(dict.contains_key("data"));
    }

    #[test]
    fn test_output_serialization() {
        let output = StageOutput::ok_value("key", serde_json::json!("value"));
        let json = serde_json::to_string(&output).unwrap();
        let deserialized: StageOutput = serde_json::from_str(&json).unwrap();

        assert_eq!(output.status, deserialized.status);
        assert_eq!(output.get("key"), deserialized.get("key"));
    }

    #[test]
    fn test_status_display() {
        assert_eq!(format!("{}", StageStatus::Ok), "ok");
        assert_eq!(format!("{}", StageStatus::Fail), "fail");
        assert_eq!(format!("{}", StageStatus::Skip), "skip");
        assert_eq!(format!("{}", StageStatus::Cancel), "cancel");
        assert_eq!(format!("{}", StageStatus::Retry), "retry");
    }

    #[test]
    fn test_output_clone() {
        let original = StageOutput::ok_value("data", serde_json::json!(42));
        let cloned = original.clone();

        assert_eq!(original.status, cloned.status);
        assert_eq!(original.get("data"), cloned.get("data"));
    }

    #[test]
    fn test_output_debug() {
        let output = StageOutput::ok_empty();
        let debug_str = format!("{:?}", output);
        assert!(debug_str.contains("StageOutput"));
    }

    #[test]
    fn test_artifact_creation() {
        let artifact = StageArtifact::new(
            "text",
            "id-1",
            "test.txt",
            serde_json::json!("Hello, World!"),
        );

        assert_eq!(artifact.name, "test.txt");
        assert_eq!(artifact.artifact_type, "text");
    }

    #[test]
    fn test_event_creation() {
        let event = StageEvent::new("custom.event")
            .add_data("key", serde_json::json!("value"));

        assert_eq!(event.event_type, "custom.event");
    }

    #[test]
    fn test_output_multiple_values() {
        let mut data = HashMap::new();
        data.insert("a".to_string(), serde_json::json!(1));
        data.insert("b".to_string(), serde_json::json!(2));
        data.insert("c".to_string(), serde_json::json!(3));

        let output = StageOutput::ok(data);
        assert_eq!(output.get("a"), Some(&serde_json::json!(1)));
        assert_eq!(output.get("b"), Some(&serde_json::json!(2)));
        assert_eq!(output.get("c"), Some(&serde_json::json!(3)));
    }

    #[test]
    fn test_output_nested_json() {
        let nested = serde_json::json!({
            "level1": {
                "level2": {
                    "level3": "deep value"
                }
            }
        });

        let output = StageOutput::ok_value("nested", nested.clone());
        assert_eq!(output.get("nested"), Some(&nested));
    }
}
