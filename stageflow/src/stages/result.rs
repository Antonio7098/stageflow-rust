//! Unified stage result types for substrate architecture.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Legacy stage status type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LegacyStageStatus {
    /// Stage started.
    Started,
    /// Stage completed successfully.
    Completed,
    /// Stage failed.
    Failed,
}

impl std::fmt::Display for LegacyStageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Started => write!(f, "started"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

/// Typed result returned by a stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    /// Stage name.
    pub name: String,
    /// Stage status.
    pub status: LegacyStageStatus,
    /// When the stage started.
    pub started_at: DateTime<Utc>,
    /// When the stage ended.
    pub ended_at: DateTime<Utc>,
    /// Result data.
    #[serde(default)]
    pub data: HashMap<String, serde_json::Value>,
    /// Error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl StageResult {
    /// Creates a new stage result.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        status: LegacyStageStatus,
        started_at: DateTime<Utc>,
        ended_at: DateTime<Utc>,
    ) -> Self {
        Self {
            name: name.into(),
            status,
            started_at,
            ended_at,
            data: HashMap::new(),
            error: None,
        }
    }

    /// Creates a completed stage result.
    #[must_use]
    pub fn completed(
        name: impl Into<String>,
        started_at: DateTime<Utc>,
        data: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            name: name.into(),
            status: LegacyStageStatus::Completed,
            started_at,
            ended_at: Utc::now(),
            data,
            error: None,
        }
    }

    /// Creates a failed stage result.
    #[must_use]
    pub fn failed(
        name: impl Into<String>,
        started_at: DateTime<Utc>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            status: LegacyStageStatus::Failed,
            started_at,
            ended_at: Utc::now(),
            data: HashMap::new(),
            error: Some(error.into()),
        }
    }

    /// Returns the duration in milliseconds.
    #[must_use]
    pub fn duration_ms(&self) -> f64 {
        (self.ended_at - self.started_at).num_milliseconds() as f64
    }

    /// Returns true if the stage succeeded.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self.status, LegacyStageStatus::Completed)
    }

    /// Returns true if the stage failed.
    #[must_use]
    pub fn is_failure(&self) -> bool {
        matches!(self.status, LegacyStageStatus::Failed)
    }
}

/// Error raised when a stage fails.
#[derive(Debug, Clone)]
pub struct StageError {
    /// Stage that failed.
    pub stage: String,
    /// Original error message.
    pub message: String,
}

impl StageError {
    /// Creates a new stage error.
    #[must_use]
    pub fn new(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for StageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stage {} failed: {}", self.stage, self.message)
    }
}

impl std::error::Error for StageError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_result_completed() {
        let started = Utc::now();
        let mut data = HashMap::new();
        data.insert("key".to_string(), serde_json::json!("value"));

        let result = StageResult::completed("test_stage", started, data);

        assert_eq!(result.name, "test_stage");
        assert!(result.is_success());
        assert!(!result.is_failure());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_stage_result_failed() {
        let started = Utc::now();
        let result = StageResult::failed("test_stage", started, "Something went wrong");

        assert_eq!(result.name, "test_stage");
        assert!(!result.is_success());
        assert!(result.is_failure());
        assert_eq!(result.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_stage_result_duration() {
        let started = Utc::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let result = StageResult::completed("test", started, HashMap::new());

        assert!(result.duration_ms() >= 10.0);
    }

    #[test]
    fn test_legacy_status_display() {
        assert_eq!(format!("{}", LegacyStageStatus::Started), "started");
        assert_eq!(format!("{}", LegacyStageStatus::Completed), "completed");
        assert_eq!(format!("{}", LegacyStageStatus::Failed), "failed");
    }

    #[test]
    fn test_stage_error() {
        let error = StageError::new("my_stage", "Connection timeout");
        assert_eq!(error.stage, "my_stage");
        assert!(error.to_string().contains("my_stage"));
        assert!(error.to_string().contains("Connection timeout"));
    }

    #[test]
    fn test_stage_result_serialization() {
        let started = Utc::now();
        let result = StageResult::completed("test", started, HashMap::new());

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: StageResult = serde_json::from_str(&json).unwrap();

        assert_eq!(result.name, deserialized.name);
        assert_eq!(result.status, deserialized.status);
    }
}
