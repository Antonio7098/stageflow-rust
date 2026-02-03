//! Interceptor chain for ordered middleware execution.

use crate::context::StageContext;
use crate::core::StageOutput;
use async_trait::async_trait;
use std::sync::Arc;

/// Trait for stage execution interceptors.
#[async_trait]
pub trait Interceptor: Send + Sync {
    /// Returns the interceptor's priority (lower = earlier execution).
    fn priority(&self) -> i32 {
        0
    }

    /// Called before stage execution.
    ///
    /// Return `Some(output)` to short-circuit execution.
    /// Return `None` to continue to the next interceptor or stage.
    async fn before(&self, _ctx: &StageContext) -> Option<StageOutput> {
        None
    }

    /// Called after stage execution.
    ///
    /// Can observe or transform the output.
    async fn after(&self, _ctx: &StageContext, output: StageOutput) -> StageOutput {
        output
    }

    /// Called when an error occurs during stage execution.
    async fn on_error(&self, _ctx: &StageContext, error: &str) -> Option<StageOutput> {
        None
    }
}

/// A chain of interceptors for stage execution.
pub struct InterceptorChain {
    interceptors: Vec<Arc<dyn Interceptor>>,
}

impl InterceptorChain {
    /// Creates a new empty chain.
    #[must_use]
    pub fn new() -> Self {
        Self {
            interceptors: Vec::new(),
        }
    }

    /// Adds an interceptor to the chain.
    pub fn add(&mut self, interceptor: Arc<dyn Interceptor>) {
        self.interceptors.push(interceptor);
        self.interceptors.sort_by_key(|i| i.priority());
    }

    /// Executes the chain before stage execution.
    ///
    /// Returns `Some(output)` if any interceptor short-circuits.
    pub async fn run_before(&self, ctx: &StageContext) -> Option<StageOutput> {
        for interceptor in &self.interceptors {
            if let Some(output) = interceptor.before(ctx).await {
                return Some(output);
            }
        }
        None
    }

    /// Executes the chain after stage execution.
    pub async fn run_after(&self, ctx: &StageContext, mut output: StageOutput) -> StageOutput {
        // Run in reverse order
        for interceptor in self.interceptors.iter().rev() {
            output = interceptor.after(ctx, output).await;
        }
        output
    }

    /// Handles an error through the chain.
    pub async fn handle_error(&self, ctx: &StageContext, error: &str) -> Option<StageOutput> {
        for interceptor in &self.interceptors {
            if let Some(output) = interceptor.on_error(ctx, error).await {
                return Some(output);
            }
        }
        None
    }

    /// Returns the number of interceptors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.interceptors.len()
    }

    /// Returns true if the chain is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.interceptors.is_empty()
    }
}

impl Default for InterceptorChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ContextSnapshot, PipelineContext, RunIdentity, StageInputs};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingInterceptor {
        before_count: AtomicUsize,
        after_count: AtomicUsize,
        priority: i32,
    }

    impl CountingInterceptor {
        fn new(priority: i32) -> Self {
            Self {
                before_count: AtomicUsize::new(0),
                after_count: AtomicUsize::new(0),
                priority,
            }
        }
    }

    #[async_trait]
    impl Interceptor for CountingInterceptor {
        fn priority(&self) -> i32 {
            self.priority
        }

        async fn before(&self, _ctx: &StageContext) -> Option<StageOutput> {
            self.before_count.fetch_add(1, Ordering::SeqCst);
            None
        }

        async fn after(&self, _ctx: &StageContext, output: StageOutput) -> StageOutput {
            self.after_count.fetch_add(1, Ordering::SeqCst);
            output
        }
    }

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
    async fn test_chain_creation() {
        let chain = InterceptorChain::new();
        assert!(chain.is_empty());
    }

    #[tokio::test]
    async fn test_chain_ordering() {
        let mut chain = InterceptorChain::new();

        let i1 = Arc::new(CountingInterceptor::new(10));
        let i2 = Arc::new(CountingInterceptor::new(5));
        let i3 = Arc::new(CountingInterceptor::new(15));

        chain.add(i1);
        chain.add(i2);
        chain.add(i3);

        assert_eq!(chain.len(), 3);

        let ctx = test_stage_context();
        chain.run_before(&ctx).await;

        // All interceptors should have been called
    }

    #[tokio::test]
    async fn test_chain_short_circuit() {
        struct ShortCircuitInterceptor;

        #[async_trait]
        impl Interceptor for ShortCircuitInterceptor {
            async fn before(&self, _ctx: &StageContext) -> Option<StageOutput> {
                Some(StageOutput::skip("Short-circuited"))
            }
        }

        let mut chain = InterceptorChain::new();
        chain.add(Arc::new(ShortCircuitInterceptor));

        let ctx = test_stage_context();
        let result = chain.run_before(&ctx).await;

        assert!(result.is_some());
    }
}
