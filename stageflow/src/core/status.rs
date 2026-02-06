//! Stage status and kind enums.

use serde::{Deserialize, Serialize};
use std::fmt;

/// The kind of work a stage performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageKind {
    /// A stage that transforms data (e.g., STT, TTS, LLM - change input form).
    Transform,
    /// A stage that enriches context (e.g., Profile, Memory, Skills - add context).
    Enrich,
    /// A stage that routes or decides between paths (e.g., Router, Dispatcher).
    Route,
    /// A stage that guards execution (e.g., Guardrails, Policy - validate).
    Guard,
    /// A stage that performs actual work / side effects (e.g., Persist, Notify).
    Work,
    /// A stage that represents an agent / main interactor.
    Agent,
}

impl Default for StageKind {
    fn default() -> Self {
        Self::Work
    }
}

impl fmt::Display for StageKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transform => write!(f, "transform"),
            Self::Enrich => write!(f, "enrich"),
            Self::Route => write!(f, "route"),
            Self::Guard => write!(f, "guard"),
            Self::Work => write!(f, "work"),
            Self::Agent => write!(f, "agent"),
        }
    }
}

/// The execution status of a stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    /// Stage completed successfully.
    Ok,
    /// Stage was skipped.
    Skip,
    /// Stage was cancelled.
    Cancel,
    /// Stage failed.
    Fail,
    /// Stage should be retried.
    Retry,
    /// Stage is pending execution.
    Pending,
    /// Stage is currently running.
    Running,
}

impl Default for StageStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl fmt::Display for StageStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok => write!(f, "ok"),
            Self::Skip => write!(f, "skip"),
            Self::Cancel => write!(f, "cancel"),
            Self::Fail => write!(f, "fail"),
            Self::Retry => write!(f, "retry"),
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
        }
    }
}

impl StageStatus {
    /// Returns true if the status represents a terminal state.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Ok | Self::Skip | Self::Cancel | Self::Fail)
    }

    /// Returns true if the status indicates success.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Ok | Self::Skip)
    }

    /// Returns true if the status indicates failure.
    #[must_use]
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Fail | Self::Cancel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_kind_display() {
        assert_eq!(StageKind::Work.to_string(), "work");
        assert_eq!(StageKind::Route.to_string(), "route");
        assert_eq!(StageKind::Guard.to_string(), "guard");
        assert_eq!(StageKind::Transform.to_string(), "transform");
        assert_eq!(StageKind::Enrich.to_string(), "enrich");
        assert_eq!(StageKind::Agent.to_string(), "agent");
    }

    #[test]
    fn test_stage_status_display() {
        assert_eq!(StageStatus::Ok.to_string(), "ok");
        assert_eq!(StageStatus::Fail.to_string(), "fail");
        assert_eq!(StageStatus::Retry.to_string(), "retry");
    }

    #[test]
    fn test_stage_status_is_terminal() {
        assert!(StageStatus::Ok.is_terminal());
        assert!(StageStatus::Skip.is_terminal());
        assert!(StageStatus::Fail.is_terminal());
        assert!(!StageStatus::Pending.is_terminal());
        assert!(!StageStatus::Running.is_terminal());
    }

    #[test]
    fn test_stage_status_serialize() {
        let status = StageStatus::Ok;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""ok""#);

        let deserialized: StageStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, StageStatus::Ok);
    }

    #[test]
    fn test_stage_kind_serialize() {
        let kind = StageKind::Route;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, r#""route""#);
    }
}
