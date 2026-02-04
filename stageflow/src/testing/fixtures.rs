//! Test fixtures for pipeline testing.

use std::collections::HashMap;
use std::sync::Arc;

use crate::context::{ContextSnapshot, PipelineContext, RunIdentity, StageContext, StageInputs};
use crate::core::StageOutput;
use crate::pipeline::{PipelineSpec, StageSpec};

/// A test context builder.
#[derive(Debug, Default)]
pub struct TestContext {
    /// Data to inject into the context.
    pub data: HashMap<String, serde_json::Value>,
    /// Metadata for the context.
    pub metadata: HashMap<String, serde_json::Value>,
    /// Pipeline run ID.
    pub pipeline_run_id: Option<String>,
    /// Request ID.
    pub request_id: Option<String>,
}

impl TestContext {
    /// Creates a new test context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds data to the context.
    #[must_use]
    pub fn with_data(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.data.insert(key.into(), value);
        self
    }

    /// Adds metadata to the context.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Sets the pipeline run ID.
    #[must_use]
    pub fn with_run_id(mut self, id: impl Into<String>) -> Self {
        self.pipeline_run_id = Some(id.into());
        self
    }

    /// Sets the request ID.
    #[must_use]
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    /// Builds a StageContext.
    #[must_use]
    pub fn build_stage_context(&self, stage_name: &str) -> StageContext {
        let identity = RunIdentity::new();
        let pipeline_ctx = Arc::new(PipelineContext::new(identity));
        let inputs = StageInputs::default();
        let snapshot = ContextSnapshot::new();

        StageContext::new(pipeline_ctx, stage_name, inputs, snapshot)
    }
}

/// A test fixture for running stage tests.
pub struct TestFixture {
    /// Test context.
    pub context: TestContext,
    /// Captured outputs.
    pub outputs: HashMap<String, StageOutput>,
}

impl TestFixture {
    /// Creates a new test fixture.
    #[must_use]
    pub fn new() -> Self {
        Self {
            context: TestContext::new(),
            outputs: HashMap::new(),
        }
    }

    /// Creates a fixture with context.
    #[must_use]
    pub fn with_context(context: TestContext) -> Self {
        Self {
            context,
            outputs: HashMap::new(),
        }
    }

    /// Records an output for a stage.
    pub fn record_output(&mut self, stage_name: impl Into<String>, output: StageOutput) {
        self.outputs.insert(stage_name.into(), output);
    }

    /// Gets an output for a stage.
    #[must_use]
    pub fn get_output(&self, stage_name: &str) -> Option<&StageOutput> {
        self.outputs.get(stage_name)
    }

    /// Returns true if all recorded outputs succeeded.
    #[must_use]
    pub fn all_succeeded(&self) -> bool {
        self.outputs.values().all(|o| o.is_success())
    }

    /// Returns true if any recorded output failed.
    #[must_use]
    pub fn any_failed(&self) -> bool {
        self.outputs.values().any(|o| o.is_failure())
    }
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new()
    }
}

/// A test pipeline builder.
pub struct TestPipeline {
    /// Pipeline name.
    pub name: String,
    /// Stage names in order.
    pub stages: Vec<String>,
}

impl TestPipeline {
    /// Creates a new test pipeline.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            stages: Vec::new(),
        }
    }

    /// Adds a stage to the pipeline.
    #[must_use]
    pub fn with_stage(mut self, name: impl Into<String>) -> Self {
        self.stages.push(name.into());
        self
    }

    /// Creates a linear pipeline with numbered stages.
    #[must_use]
    pub fn linear(name: impl Into<String>, count: usize) -> Self {
        let mut pipeline = Self::new(name);
        for i in 0..count {
            pipeline.stages.push(format!("stage_{}", i));
        }
        pipeline
    }

    /// Returns the stage names.
    #[must_use]
    pub fn stage_names(&self) -> &[String] {
        &self.stages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_context_builder() {
        let ctx = TestContext::new()
            .with_data("key", serde_json::json!("value"))
            .with_metadata("meta", serde_json::json!(true))
            .with_run_id("run-123")
            .with_request_id("req-456");

        assert_eq!(ctx.data.get("key"), Some(&serde_json::json!("value")));
        assert_eq!(ctx.metadata.get("meta"), Some(&serde_json::json!(true)));
        assert_eq!(ctx.pipeline_run_id, Some("run-123".to_string()));
        assert_eq!(ctx.request_id, Some("req-456".to_string()));
    }

    #[test]
    fn test_test_context_build_stage_context() {
        let ctx = TestContext::new()
            .with_run_id("run-123");

        let stage_ctx = ctx.build_stage_context("test_stage");
        assert_eq!(stage_ctx.stage_name(), "test_stage");
    }

    #[test]
    fn test_test_fixture() {
        let mut fixture = TestFixture::new();
        
        fixture.record_output("stage1", StageOutput::ok_empty());
        fixture.record_output("stage2", StageOutput::ok_empty());

        assert!(fixture.all_succeeded());
        assert!(!fixture.any_failed());
        assert!(fixture.get_output("stage1").is_some());
    }

    #[test]
    fn test_test_fixture_with_failure() {
        let mut fixture = TestFixture::new();
        
        fixture.record_output("stage1", StageOutput::ok_empty());
        fixture.record_output("stage2", StageOutput::fail("error"));

        assert!(!fixture.all_succeeded());
        assert!(fixture.any_failed());
    }

    #[test]
    fn test_test_pipeline() {
        let pipeline = TestPipeline::new("test")
            .with_stage("fetch")
            .with_stage("process")
            .with_stage("store");

        assert_eq!(pipeline.name, "test");
        assert_eq!(pipeline.stages.len(), 3);
    }

    #[test]
    fn test_test_pipeline_linear() {
        let pipeline = TestPipeline::linear("linear", 5);

        assert_eq!(pipeline.stages.len(), 5);
        assert_eq!(pipeline.stages[0], "stage_0");
        assert_eq!(pipeline.stages[4], "stage_4");
    }
}
