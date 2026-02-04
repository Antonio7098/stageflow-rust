//! Pipeline building and execution.
//!
//! This module provides:
//! - Pipeline specifications
//! - Pipeline builder with validation
//! - DAG execution engines

mod builder;
mod builder_helpers;
mod dag;
mod guard_retry;
mod spec;
mod unified;

pub use builder::PipelineBuilder;
pub use builder_helpers::FluentPipelineBuilder;
pub use dag::{GraphExecutionResult, StageGraph};
pub use guard_retry::{
    GuardRetryPolicy, GuardRetryRuntimeState, GuardRetryStrategy, hash_retry_payload,
};
pub use spec::{PipelineSpec, StageSpec};
pub use unified::UnifiedStageGraph;
