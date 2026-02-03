//! Hardening interceptors for context protection.

use super::Interceptor;
use crate::context::{ExecutionContext, StageContext};
use crate::core::StageOutput;
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::warn;

/// Interceptor that detects snapshot mutations.
pub struct ImmutabilityInterceptor {
    /// Number of violations detected.
    violations: AtomicUsize,
}

impl ImmutabilityInterceptor {
    /// Creates a new immutability interceptor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            violations: AtomicUsize::new(0),
        }
    }

    /// Returns the number of violations detected.
    #[must_use]
    pub fn violation_count(&self) -> usize {
        self.violations.load(Ordering::SeqCst)
    }
}

impl Default for ImmutabilityInterceptor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Interceptor for ImmutabilityInterceptor {
    fn priority(&self) -> i32 {
        -50
    }

    async fn before(&self, ctx: &StageContext) -> Option<StageOutput> {
        // Store snapshot hash for later comparison
        // In a real implementation, we'd compute a hash of the snapshot
        None
    }

    async fn after(&self, ctx: &StageContext, output: StageOutput) -> StageOutput {
        // Compare snapshot hash - if different, log violation
        // For now, just pass through
        output
    }
}

/// Interceptor that warns on large or growing contexts.
pub struct ContextSizeInterceptor {
    /// Maximum allowed size in bytes.
    max_size_bytes: usize,
    /// Warning threshold as a fraction of max size.
    warning_threshold: f64,
}

impl ContextSizeInterceptor {
    /// Creates a new context size interceptor.
    #[must_use]
    pub fn new(max_size_bytes: usize, warning_threshold: f64) -> Self {
        Self {
            max_size_bytes,
            warning_threshold: warning_threshold.clamp(0.0, 1.0),
        }
    }

    /// Estimates the size of the context data.
    fn estimate_size(&self, ctx: &StageContext) -> usize {
        // Approximate by serializing to JSON
        let snapshot = ctx.snapshot();
        serde_json::to_string(snapshot)
            .map(|s| s.len())
            .unwrap_or(0)
    }
}

impl Default for ContextSizeInterceptor {
    fn default() -> Self {
        Self::new(1024 * 1024, 0.8) // 1MB max, warn at 80%
    }
}

#[async_trait]
impl Interceptor for ContextSizeInterceptor {
    fn priority(&self) -> i32 {
        -40
    }

    async fn before(&self, ctx: &StageContext) -> Option<StageOutput> {
        let size = self.estimate_size(ctx);
        let threshold = (self.max_size_bytes as f64 * self.warning_threshold) as usize;

        if size > self.max_size_bytes {
            warn!(
                stage = %ctx.stage_name(),
                size_bytes = size,
                max_bytes = self.max_size_bytes,
                "Context size exceeds maximum"
            );
        } else if size > threshold {
            warn!(
                stage = %ctx.stage_name(),
                size_bytes = size,
                threshold_bytes = threshold,
                "Context size approaching maximum"
            );
        }

        None
    }

    async fn after(&self, ctx: &StageContext, output: StageOutput) -> StageOutput {
        // Record metrics about context size growth
        let size = self.estimate_size(ctx);

        ctx.try_emit_event(
            "context.size_recorded",
            Some(serde_json::json!({
                "stage": ctx.stage_name(),
                "size_bytes": size,
            })),
        );

        output
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
    async fn test_immutability_interceptor() {
        let interceptor = ImmutabilityInterceptor::new();
        let ctx = test_stage_context();

        let before_result = interceptor.before(&ctx).await;
        assert!(before_result.is_none());

        let output = StageOutput::ok_empty();
        let after_result = interceptor.after(&ctx, output).await;
        assert!(after_result.is_success());
    }

    #[tokio::test]
    async fn test_context_size_interceptor() {
        let interceptor = ContextSizeInterceptor::new(10000, 0.8);
        let ctx = test_stage_context();

        let before_result = interceptor.before(&ctx).await;
        assert!(before_result.is_none());

        let output = StageOutput::ok_empty();
        let after_result = interceptor.after(&ctx, output).await;
        assert!(after_result.is_success());
    }
}
