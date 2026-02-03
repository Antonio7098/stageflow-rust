//! Stage trait and implementations.
//!
//! Stages are the fundamental units of work in a stageflow pipeline.

use crate::context::StageContext;
use crate::core::StageOutput;
use async_trait::async_trait;
use std::fmt::Debug;

/// Trait for pipeline stages.
///
/// Stages represent discrete units of work that can be composed
/// into pipelines with dependencies between them.
#[async_trait]
pub trait Stage: Send + Sync + Debug {
    /// Returns the name of the stage.
    fn name(&self) -> &str;

    /// Executes the stage.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The stage execution context
    ///
    /// # Returns
    ///
    /// The stage output indicating success, failure, skip, etc.
    async fn execute(&self, ctx: &StageContext) -> StageOutput;
}

/// A simple function-based stage.
pub struct FnStage<F>
where
    F: Fn(&StageContext) -> StageOutput + Send + Sync,
{
    name: String,
    func: F,
}

impl<F> FnStage<F>
where
    F: Fn(&StageContext) -> StageOutput + Send + Sync,
{
    /// Creates a new function-based stage.
    pub fn new(name: impl Into<String>, func: F) -> Self {
        Self {
            name: name.into(),
            func,
        }
    }
}

impl<F> Debug for FnStage<F>
where
    F: Fn(&StageContext) -> StageOutput + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FnStage")
            .field("name", &self.name)
            .finish()
    }
}

#[async_trait]
impl<F> Stage for FnStage<F>
where
    F: Fn(&StageContext) -> StageOutput + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, ctx: &StageContext) -> StageOutput {
        (self.func)(ctx)
    }
}

/// An async function-based stage.
pub struct AsyncFnStage<F, Fut>
where
    F: Fn(StageContext) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = StageOutput> + Send,
{
    name: String,
    func: F,
    _phantom: std::marker::PhantomData<Fut>,
}

impl<F, Fut> AsyncFnStage<F, Fut>
where
    F: Fn(StageContext) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = StageOutput> + Send,
{
    /// Creates a new async function-based stage.
    pub fn new(name: impl Into<String>, func: F) -> Self {
        Self {
            name: name.into(),
            func,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<F, Fut> Debug for AsyncFnStage<F, Fut>
where
    F: Fn(StageContext) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = StageOutput> + Send,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncFnStage")
            .field("name", &self.name)
            .finish()
    }
}

/// A no-op stage for testing.
#[derive(Debug, Clone)]
pub struct NoOpStage {
    name: String,
}

impl NoOpStage {
    /// Creates a new no-op stage.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Stage for NoOpStage {
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, _ctx: &StageContext) -> StageOutput {
        StageOutput::ok_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ContextSnapshot, PipelineContext, RunIdentity, StageInputs};
    use std::sync::Arc;

    fn test_stage_context() -> StageContext {
        let pipeline_ctx = Arc::new(PipelineContext::new(RunIdentity::new()));
        StageContext::new(
            pipeline_ctx,
            "test",
            StageInputs::default(),
            ContextSnapshot::new(),
        )
    }

    #[tokio::test]
    async fn test_fn_stage() {
        let stage = FnStage::new("test", |_ctx| StageOutput::ok_value("result", serde_json::json!("done")));

        assert_eq!(stage.name(), "test");

        let ctx = test_stage_context();
        let output = stage.execute(&ctx).await;
        assert!(output.is_success());
    }

    #[tokio::test]
    async fn test_noop_stage() {
        let stage = NoOpStage::new("noop");

        assert_eq!(stage.name(), "noop");

        let ctx = test_stage_context();
        let output = stage.execute(&ctx).await;
        assert!(output.is_success());
    }
}
