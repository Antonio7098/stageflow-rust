//! Pipeline building and execution.
//!
//! This module provides:
//! - Pipeline specifications
//! - Pipeline builder with validation
//! - DAG execution engines
//! - Failure tolerance modes

mod builder;
mod builder_helpers;
mod cancellation;
mod dag;
mod failure_tolerance;
mod guard_retry;
mod idempotency;
mod retry;
mod spec;
mod unified;

pub use builder::PipelineBuilder;
pub use builder_helpers::FluentPipelineBuilder;
pub use cancellation::{
    CancellationToken, CleanupGuard, CleanupRegistry, run_with_cleanup,
};
pub use dag::{GraphExecutionResult, StageGraph};
pub use failure_tolerance::{
    BackpressureConfig, BackpressureTracker, FailureCollector, FailureMode,
    FailureRecord, FailureSummary,
};
pub use guard_retry::{
    GuardRetryPolicy, GuardRetryRuntimeState, GuardRetryStrategy, hash_retry_payload,
};
pub use idempotency::{
    CachedResult, IdempotencyCheckResult, IdempotencyConfig, IdempotencyParamMismatch,
    IdempotencyStore, InMemoryIdempotencyStore, check_idempotency, generate_idempotency_key,
    hash_parameters,
};
pub use retry::{
    BackoffStrategy, JitterStrategy, RetryConfig, RetryDecision, RetryState,
    should_retry, with_retry,
};
pub use spec::{PipelineSpec, StageSpec};
pub use unified::UnifiedStageGraph;
