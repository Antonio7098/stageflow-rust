//! Failure tolerance utilities for DAG execution.
//!
//! Provides continue-on-failure mode that records failures but continues
//! executing unrelated branches, and backpressure management for burst loads.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

/// How to handle stage failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum FailureMode {
    /// Stop pipeline on first failure (default).
    #[default]
    FailFast,
    /// Record failure, continue unrelated branches.
    ContinueOnFailure,
    /// Continue all branches, collect all failures.
    BestEffort,
}

/// Record of a stage failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureRecord {
    /// Stage name.
    pub stage: String,
    /// Error message.
    pub error: String,
    /// Error type name.
    pub error_type: String,
    /// Whether the error is recoverable.
    pub recoverable: bool,
    /// Unix timestamp of the failure.
    pub timestamp: f64,
    /// Additional context.
    pub context: HashMap<String, serde_json::Value>,
}

impl FailureRecord {
    /// Creates a new failure record.
    #[must_use]
    pub fn new(stage: impl Into<String>, error: impl Into<String>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);

        Self {
            stage: stage.into(),
            error: error.into(),
            error_type: "Error".to_string(),
            recoverable: false,
            timestamp: now,
            context: HashMap::new(),
        }
    }

    /// Sets the error type.
    #[must_use]
    pub fn with_error_type(mut self, error_type: impl Into<String>) -> Self {
        self.error_type = error_type.into();
        self
    }

    /// Marks as recoverable.
    #[must_use]
    pub fn recoverable(mut self) -> Self {
        self.recoverable = true;
        self
    }

    /// Adds context.
    #[must_use]
    pub fn with_context(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.context.insert(key.into(), value);
        self
    }
}

/// Summary of failures during pipeline execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureSummary {
    /// Total number of stages.
    pub total_stages: usize,
    /// Number of completed stages.
    pub completed_stages: usize,
    /// Number of failed stages.
    pub failed_stages: usize,
    /// List of failure records.
    pub failures: Vec<FailureRecord>,
    /// Partial results from completed stages.
    pub partial_results: HashMap<String, serde_json::Value>,
}

impl FailureSummary {
    /// Creates a new failure summary.
    #[must_use]
    pub fn new(total_stages: usize) -> Self {
        Self {
            total_stages,
            completed_stages: 0,
            failed_stages: 0,
            failures: Vec::new(),
            partial_results: HashMap::new(),
        }
    }

    /// Returns the success rate.
    #[must_use]
    pub fn success_rate(&self) -> f64 {
        if self.total_stages == 0 {
            return 0.0;
        }
        self.completed_stages as f64 / self.total_stages as f64
    }

    /// Returns true if any failures occurred.
    #[must_use]
    pub fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }

    /// Converts to dictionary.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("total_stages".to_string(), serde_json::json!(self.total_stages));
        map.insert("completed_stages".to_string(), serde_json::json!(self.completed_stages));
        map.insert("failed_stages".to_string(), serde_json::json!(self.failed_stages));
        map.insert("success_rate".to_string(), serde_json::json!(self.success_rate()));
        map.insert("failures".to_string(), serde_json::json!(
            self.failures.iter().map(|f| {
                serde_json::json!({
                    "stage": f.stage,
                    "error": f.error,
                    "error_type": f.error_type,
                    "recoverable": f.recoverable,
                    "timestamp": f.timestamp,
                })
            }).collect::<Vec<_>>()
        ));
        map
    }
}

/// Collect and manage failures during pipeline execution.
#[derive(Debug)]
pub struct FailureCollector {
    /// Failure mode.
    pub mode: FailureMode,
    failures: Vec<FailureRecord>,
    failed_stages: HashSet<String>,
    completed_stages: HashSet<String>,
}

impl FailureCollector {
    /// Creates a new failure collector.
    #[must_use]
    pub fn new(mode: FailureMode) -> Self {
        Self {
            mode,
            failures: Vec::new(),
            failed_stages: HashSet::new(),
            completed_stages: HashSet::new(),
        }
    }

    /// Records a stage failure.
    pub fn record_failure(&mut self, record: FailureRecord) {
        self.failed_stages.insert(record.stage.clone());
        self.failures.push(record);
    }

    /// Records a stage completion.
    pub fn record_completion(&mut self, stage: &str) {
        self.completed_stages.insert(stage.to_string());
    }

    /// Returns true if the stage has failed.
    #[must_use]
    pub fn has_failed(&self, stage: &str) -> bool {
        self.failed_stages.contains(stage)
    }

    /// Returns true if execution should stop based on mode.
    #[must_use]
    pub fn should_stop(&self) -> bool {
        match self.mode {
            FailureMode::FailFast => !self.failures.is_empty(),
            FailureMode::ContinueOnFailure | FailureMode::BestEffort => false,
        }
    }

    /// Returns true if a dependent stage can run.
    #[must_use]
    pub fn can_run(&self, stage: &str, dependencies: &[String]) -> bool {
        match self.mode {
            FailureMode::FailFast => {
                // Can only run if no failures at all
                self.failures.is_empty()
            }
            FailureMode::ContinueOnFailure => {
                // Can run if none of its dependencies have failed
                !dependencies.iter().any(|dep| self.failed_stages.contains(dep))
            }
            FailureMode::BestEffort => {
                // Always try to run
                true
            }
        }
    }

    /// Creates a summary of the execution.
    #[must_use]
    pub fn summary(&self, total_stages: usize) -> FailureSummary {
        FailureSummary {
            total_stages,
            completed_stages: self.completed_stages.len(),
            failed_stages: self.failed_stages.len(),
            failures: self.failures.clone(),
            partial_results: HashMap::new(),
        }
    }

    /// Returns all failures.
    #[must_use]
    pub fn failures(&self) -> &[FailureRecord] {
        &self.failures
    }
}

impl Default for FailureCollector {
    fn default() -> Self {
        Self::new(FailureMode::default())
    }
}

/// Backpressure configuration for burst load management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackpressureConfig {
    /// Maximum concurrent stage executions.
    pub max_concurrent: usize,
    /// Queue size before applying backpressure.
    pub queue_size: usize,
    /// Delay in milliseconds when backpressure is applied.
    pub delay_ms: u64,
    /// Whether to drop requests when queue is full.
    pub drop_on_full: bool,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            queue_size: 100,
            delay_ms: 100,
            drop_on_full: false,
        }
    }
}

/// Backpressure state tracker.
#[derive(Debug)]
pub struct BackpressureTracker {
    config: BackpressureConfig,
    current_concurrent: usize,
    current_queue_size: usize,
}

impl BackpressureTracker {
    /// Creates a new backpressure tracker.
    #[must_use]
    pub fn new(config: BackpressureConfig) -> Self {
        Self {
            config,
            current_concurrent: 0,
            current_queue_size: 0,
        }
    }

    /// Returns true if backpressure should be applied.
    #[must_use]
    pub fn should_apply_backpressure(&self) -> bool {
        self.current_concurrent >= self.config.max_concurrent
            || self.current_queue_size >= self.config.queue_size
    }

    /// Acquires a slot for execution.
    pub fn acquire(&mut self) -> bool {
        if self.current_concurrent >= self.config.max_concurrent {
            if self.config.drop_on_full {
                return false;
            }
            self.current_queue_size += 1;
        }
        self.current_concurrent += 1;
        true
    }

    /// Releases a slot after execution.
    pub fn release(&mut self) {
        if self.current_concurrent > 0 {
            self.current_concurrent -= 1;
        }
        if self.current_queue_size > 0 {
            self.current_queue_size -= 1;
        }
    }

    /// Returns the delay to apply.
    #[must_use]
    pub fn delay_ms(&self) -> u64 {
        if self.should_apply_backpressure() {
            self.config.delay_ms
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failure_mode_default() {
        assert_eq!(FailureMode::default(), FailureMode::FailFast);
    }

    #[test]
    fn test_failure_record_creation() {
        let record = FailureRecord::new("my_stage", "error message")
            .with_error_type("ValueError")
            .recoverable();

        assert_eq!(record.stage, "my_stage");
        assert_eq!(record.error, "error message");
        assert_eq!(record.error_type, "ValueError");
        assert!(record.recoverable);
    }

    #[test]
    fn test_failure_summary() {
        let mut summary = FailureSummary::new(10);
        summary.completed_stages = 7;
        summary.failed_stages = 3;

        assert!((summary.success_rate() - 0.7).abs() < 0.001);
        assert!(!summary.has_failures());

        summary.failures.push(FailureRecord::new("stage1", "error"));
        assert!(summary.has_failures());
    }

    #[test]
    fn test_failure_collector_fail_fast() {
        let mut collector = FailureCollector::new(FailureMode::FailFast);
        
        assert!(!collector.should_stop());
        
        collector.record_failure(FailureRecord::new("stage1", "error"));
        
        assert!(collector.should_stop());
        assert!(collector.has_failed("stage1"));
        assert!(!collector.has_failed("stage2"));
    }

    #[test]
    fn test_failure_collector_continue_on_failure() {
        let mut collector = FailureCollector::new(FailureMode::ContinueOnFailure);
        
        collector.record_failure(FailureRecord::new("stage1", "error"));
        
        assert!(!collector.should_stop());
        
        // Stage depending on failed stage cannot run
        assert!(!collector.can_run("stage2", &["stage1".to_string()]));
        
        // Stage not depending on failed stage can run
        assert!(collector.can_run("stage3", &["stage_other".to_string()]));
    }

    #[test]
    fn test_failure_collector_best_effort() {
        let mut collector = FailureCollector::new(FailureMode::BestEffort);
        
        collector.record_failure(FailureRecord::new("stage1", "error"));
        
        assert!(!collector.should_stop());
        
        // All stages can run in best effort mode
        assert!(collector.can_run("stage2", &["stage1".to_string()]));
    }

    #[test]
    fn test_failure_collector_summary() {
        let mut collector = FailureCollector::new(FailureMode::FailFast);
        
        collector.record_completion("stage1");
        collector.record_completion("stage2");
        collector.record_failure(FailureRecord::new("stage3", "error"));
        
        let summary = collector.summary(5);
        
        assert_eq!(summary.total_stages, 5);
        assert_eq!(summary.completed_stages, 2);
        assert_eq!(summary.failed_stages, 1);
    }

    #[test]
    fn test_backpressure_config_default() {
        let config = BackpressureConfig::default();
        assert_eq!(config.max_concurrent, 10);
        assert_eq!(config.queue_size, 100);
    }

    #[test]
    fn test_backpressure_tracker() {
        let config = BackpressureConfig {
            max_concurrent: 2,
            queue_size: 5,
            delay_ms: 50,
            drop_on_full: false,
        };
        
        let mut tracker = BackpressureTracker::new(config);
        
        assert!(!tracker.should_apply_backpressure());
        
        tracker.acquire();
        tracker.acquire();
        
        assert!(tracker.should_apply_backpressure());
        assert_eq!(tracker.delay_ms(), 50);
        
        tracker.release();
        
        assert!(!tracker.should_apply_backpressure());
    }
}
