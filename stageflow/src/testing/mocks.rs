//! Mock stages for testing.

use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::context::StageContext;
use crate::core::StageOutput;
use crate::stages::Stage;

/// A mock stage that records calls and returns a configurable output.
#[derive(Debug)]
pub struct MockStage {
    name: String,
    output: Mutex<StageOutput>,
    call_count: Mutex<usize>,
    contexts: Mutex<Vec<String>>,
}

impl MockStage {
    /// Creates a new mock stage with a success output.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: Mutex::new(StageOutput::ok_empty()),
            call_count: Mutex::new(0),
            contexts: Mutex::new(Vec::new()),
        }
    }

    /// Sets the output to return.
    pub fn set_output(&self, output: StageOutput) {
        *self.output.lock() = output;
    }

    /// Returns the number of times the stage was called.
    #[must_use]
    pub fn call_count(&self) -> usize {
        *self.call_count.lock()
    }

    /// Returns the stage contexts from each call.
    #[must_use]
    pub fn recorded_contexts(&self) -> Vec<String> {
        self.contexts.lock().clone()
    }

    /// Resets call tracking.
    pub fn reset(&self) {
        *self.call_count.lock() = 0;
        self.contexts.lock().clear();
    }
}

#[async_trait]
impl Stage for MockStage {
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, ctx: &StageContext) -> StageOutput {
        *self.call_count.lock() += 1;
        self.contexts.lock().push(ctx.stage_name().to_string());
        self.output.lock().clone()
    }
}

/// A stage that always succeeds with optional data.
#[derive(Debug)]
pub struct SuccessStage {
    name: String,
    data: HashMap<String, serde_json::Value>,
}

impl SuccessStage {
    /// Creates a new success stage.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data: HashMap::new(),
        }
    }

    /// Creates a success stage with data.
    #[must_use]
    pub fn with_data(name: impl Into<String>, data: HashMap<String, serde_json::Value>) -> Self {
        Self {
            name: name.into(),
            data,
        }
    }
}

#[async_trait]
impl Stage for SuccessStage {
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, _ctx: &StageContext) -> StageOutput {
        if self.data.is_empty() {
            StageOutput::ok_empty()
        } else {
            StageOutput::ok(self.data.clone())
        }
    }
}

/// A stage that always fails.
#[derive(Debug)]
pub struct FailingStage {
    name: String,
    error: String,
    retryable: bool,
}

impl FailingStage {
    /// Creates a new failing stage.
    #[must_use]
    pub fn new(name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            error: error.into(),
            retryable: false,
        }
    }

    /// Creates a retryable failing stage.
    #[must_use]
    pub fn retryable(name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            error: error.into(),
            retryable: true,
        }
    }
}

#[async_trait]
impl Stage for FailingStage {
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, _ctx: &StageContext) -> StageOutput {
        if self.retryable {
            StageOutput::fail_retryable(&self.error)
        } else {
            StageOutput::fail(&self.error)
        }
    }
}

/// A stage that takes time to execute.
#[derive(Debug)]
pub struct SlowStage {
    name: String,
    delay: Duration,
}

impl SlowStage {
    /// Creates a new slow stage.
    #[must_use]
    pub fn new(name: impl Into<String>, delay: Duration) -> Self {
        Self {
            name: name.into(),
            delay,
        }
    }

    /// Creates a slow stage with delay in milliseconds.
    #[must_use]
    pub fn with_delay_ms(name: impl Into<String>, ms: u64) -> Self {
        Self::new(name, Duration::from_millis(ms))
    }
}

#[async_trait]
impl Stage for SlowStage {
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, _ctx: &StageContext) -> StageOutput {
        tokio::time::sleep(self.delay).await;
        StageOutput::ok_empty()
    }
}

/// A stage that records all inputs and outputs.
#[derive(Debug)]
pub struct RecordingStage {
    name: String,
    executions: Mutex<Vec<RecordedExecution>>,
}

/// A recorded execution.
#[derive(Debug, Clone)]
pub struct RecordedExecution {
    /// Stage name from context.
    pub stage_name: String,
    /// Output produced.
    pub output: StageOutput,
}

impl RecordingStage {
    /// Creates a new recording stage.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            executions: Mutex::new(Vec::new()),
        }
    }

    /// Returns all recorded executions.
    #[must_use]
    pub fn executions(&self) -> Vec<RecordedExecution> {
        self.executions.lock().clone()
    }

    /// Returns the number of executions.
    #[must_use]
    pub fn execution_count(&self) -> usize {
        self.executions.lock().len()
    }

    /// Clears recorded executions.
    pub fn clear(&self) {
        self.executions.lock().clear();
    }
}

#[async_trait]
impl Stage for RecordingStage {
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, ctx: &StageContext) -> StageOutput {
        let output = StageOutput::ok_empty();
        self.executions.lock().push(RecordedExecution {
            stage_name: ctx.stage_name().to_string(),
            output: output.clone(),
        });
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ContextSnapshot, PipelineContext, RunIdentity, StageInputs};

    fn test_context(name: &str) -> StageContext {
        let pipeline_ctx = Arc::new(PipelineContext::new(RunIdentity::new()));
        StageContext::new(
            pipeline_ctx,
            name,
            StageInputs::default(),
            ContextSnapshot::new(),
        )
    }

    #[tokio::test]
    async fn test_mock_stage() {
        let stage = MockStage::new("test");
        let ctx = test_context("test");

        let output = stage.execute(&ctx).await;
        assert!(output.is_success());
        assert_eq!(stage.call_count(), 1);

        stage.set_output(StageOutput::fail("error"));
        let output = stage.execute(&ctx).await;
        assert!(output.is_failure());
        assert_eq!(stage.call_count(), 2);
    }

    #[tokio::test]
    async fn test_success_stage() {
        let stage = SuccessStage::new("success");
        let ctx = test_context("success");

        let output = stage.execute(&ctx).await;
        assert!(output.is_success());
    }

    #[tokio::test]
    async fn test_success_stage_with_data() {
        let mut data = HashMap::new();
        data.insert("key".to_string(), serde_json::json!("value"));
        let stage = SuccessStage::with_data("success", data);
        let ctx = test_context("success");

        let output = stage.execute(&ctx).await;
        assert!(output.is_success());
        assert_eq!(output.get("key"), Some(&serde_json::json!("value")));
    }

    #[tokio::test]
    async fn test_failing_stage() {
        let stage = FailingStage::new("fail", "test error");
        let ctx = test_context("fail");

        let output = stage.execute(&ctx).await;
        assert!(output.is_failure());
        assert!(!output.is_retryable());
    }

    #[tokio::test]
    async fn test_failing_stage_retryable() {
        let stage = FailingStage::retryable("fail", "retry me");
        let ctx = test_context("fail");

        let output = stage.execute(&ctx).await;
        assert!(output.is_failure());
        assert!(output.is_retryable());
    }

    #[tokio::test]
    async fn test_slow_stage() {
        let stage = SlowStage::with_delay_ms("slow", 10);
        let ctx = test_context("slow");

        let start = std::time::Instant::now();
        let output = stage.execute(&ctx).await;
        let elapsed = start.elapsed();

        assert!(output.is_success());
        assert!(elapsed >= Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_recording_stage() {
        let stage = RecordingStage::new("record");
        let ctx = test_context("record");

        stage.execute(&ctx).await;
        stage.execute(&ctx).await;

        assert_eq!(stage.execution_count(), 2);
        
        let executions = stage.executions();
        assert_eq!(executions[0].stage_name, "record");
    }
}
