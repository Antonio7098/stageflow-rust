//! Subpipeline execution result.

use crate::core::StageOutput;
use std::collections::HashMap;
use uuid::Uuid;

/// Result of a subpipeline execution.
#[derive(Debug, Clone)]
pub struct SubpipelineResult {
    /// The child pipeline run ID.
    pub child_run_id: Uuid,
    /// Whether execution completed successfully.
    pub success: bool,
    /// Per-stage outputs.
    pub outputs: HashMap<String, StageOutput>,
    /// Error message if failed.
    pub error: Option<String>,
    /// Execution duration in milliseconds.
    pub duration_ms: f64,
}

impl SubpipelineResult {
    /// Creates a successful result.
    #[must_use]
    pub fn success(
        child_run_id: Uuid,
        outputs: HashMap<String, StageOutput>,
        duration_ms: f64,
    ) -> Self {
        Self {
            child_run_id,
            success: true,
            outputs,
            error: None,
            duration_ms,
        }
    }

    /// Creates a failed result.
    #[must_use]
    pub fn failure(
        child_run_id: Uuid,
        error: impl Into<String>,
        outputs: HashMap<String, StageOutput>,
        duration_ms: f64,
    ) -> Self {
        Self {
            child_run_id,
            success: false,
            outputs,
            error: Some(error.into()),
            duration_ms,
        }
    }

    /// Gets output from a specific stage.
    #[must_use]
    pub fn get_output(&self, stage: &str) -> Option<&StageOutput> {
        self.outputs.get(stage)
    }

    /// Converts to a dictionary representation.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert(
            "child_run_id".to_string(),
            serde_json::json!(self.child_run_id.to_string()),
        );
        map.insert("success".to_string(), serde_json::json!(self.success));
        map.insert("duration_ms".to_string(), serde_json::json!(self.duration_ms));

        if let Some(ref error) = self.error {
            map.insert("error".to_string(), serde_json::json!(error));
        }

        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_result() {
        let child_id = Uuid::new_v4();
        let result = SubpipelineResult::success(child_id, HashMap::new(), 100.0);

        assert!(result.success);
        assert!(result.error.is_none());
        assert_eq!(result.duration_ms, 100.0);
    }

    #[test]
    fn test_failure_result() {
        let child_id = Uuid::new_v4();
        let result = SubpipelineResult::failure(child_id, "Something went wrong", HashMap::new(), 50.0);

        assert!(!result.success);
        assert_eq!(result.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_to_dict() {
        let child_id = Uuid::new_v4();
        let result = SubpipelineResult::success(child_id, HashMap::new(), 100.0);

        let dict = result.to_dict();
        assert!(dict.contains_key("child_run_id"));
        assert!(dict.contains_key("success"));
    }
}
