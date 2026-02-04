//! Stage interfaces for ISP (Interface Segregation Principle).
//!
//! This module defines minimal interfaces for different stage capabilities,
//! allowing stages to implement only what they need.

use crate::context::StageContext;
use crate::core::StageOutput;
use async_trait::async_trait;
use std::collections::HashMap;

/// Interface for stages that support retry on failure.
#[async_trait]
pub trait RetryableStage: Send + Sync {
    /// Maximum number of retry attempts (default: 0, no retries).
    fn max_retries(&self) -> usize {
        0
    }

    /// Determine if an error is retryable.
    fn should_retry(&self, error: &str) -> bool;

    /// Execute the stage with retry support.
    async fn execute(&self, ctx: &StageContext) -> StageOutput;
}

/// Interface for stages that can be conditionally skipped.
#[async_trait]
pub trait ConditionalStage: Send + Sync {
    /// Determine if this stage should execute.
    fn should_run(&self, ctx: &StageContext) -> bool;

    /// Execute the stage or skip based on condition.
    async fn execute(&self, ctx: &StageContext) -> StageOutput;
}

/// Interface for stages that emit detailed events.
#[async_trait]
pub trait ObservableStage: Send + Sync {
    /// Execute the stage with event emission.
    async fn execute(&self, ctx: &StageContext) -> StageOutput;
}

/// Interface for stages with runtime configuration.
pub trait ConfigurableStage: Send + Sync {
    /// Apply runtime configuration to the stage.
    fn configure(&mut self, config: HashMap<String, serde_json::Value>);

    /// Get the current stage configuration.
    fn get_config(&self) -> HashMap<String, serde_json::Value>;
}

/// Interface for stages that require external dependencies.
pub trait DependentStage: Send + Sync {
    /// Type of dependencies this stage requires.
    type Dependencies;

    /// Initialize stage with required dependencies.
    fn with_dependencies(deps: Self::Dependencies) -> Self
    where
        Self: Sized;
}

/// Marker trait for stages that are idempotent.
pub trait IdempotentStage: Send + Sync {}

/// Marker trait for stages that are safe to run in parallel.
pub trait ParallelSafeStage: Send + Sync {}

/// Stage capability flags.
#[derive(Debug, Clone, Default)]
pub struct StageCapabilities {
    /// Whether the stage supports retries.
    pub retryable: bool,
    /// Maximum retry attempts if retryable.
    pub max_retries: usize,
    /// Whether the stage is conditional.
    pub conditional: bool,
    /// Whether the stage emits events.
    pub observable: bool,
    /// Whether the stage is configurable.
    pub configurable: bool,
    /// Whether the stage is idempotent.
    pub idempotent: bool,
    /// Whether the stage is safe to run in parallel.
    pub parallel_safe: bool,
}

impl StageCapabilities {
    /// Creates new default capabilities.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets retryable capability.
    #[must_use]
    pub fn with_retryable(mut self, max_retries: usize) -> Self {
        self.retryable = true;
        self.max_retries = max_retries;
        self
    }

    /// Sets conditional capability.
    #[must_use]
    pub fn with_conditional(mut self) -> Self {
        self.conditional = true;
        self
    }

    /// Sets observable capability.
    #[must_use]
    pub fn with_observable(mut self) -> Self {
        self.observable = true;
        self
    }

    /// Sets configurable capability.
    #[must_use]
    pub fn with_configurable(mut self) -> Self {
        self.configurable = true;
        self
    }

    /// Sets idempotent capability.
    #[must_use]
    pub fn with_idempotent(mut self) -> Self {
        self.idempotent = true;
        self
    }

    /// Sets parallel safe capability.
    #[must_use]
    pub fn with_parallel_safe(mut self) -> Self {
        self.parallel_safe = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_capabilities_default() {
        let caps = StageCapabilities::new();
        assert!(!caps.retryable);
        assert!(!caps.conditional);
        assert!(!caps.observable);
        assert!(!caps.configurable);
        assert!(!caps.idempotent);
        assert!(!caps.parallel_safe);
    }

    #[test]
    fn test_stage_capabilities_builder() {
        let caps = StageCapabilities::new()
            .with_retryable(3)
            .with_conditional()
            .with_observable()
            .with_idempotent()
            .with_parallel_safe();

        assert!(caps.retryable);
        assert_eq!(caps.max_retries, 3);
        assert!(caps.conditional);
        assert!(caps.observable);
        assert!(caps.idempotent);
        assert!(caps.parallel_safe);
    }
}
