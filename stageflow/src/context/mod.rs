//! Context management for pipeline execution.
//!
//! This module provides:
//! - Immutable context snapshots for capturing state
//! - Mutable execution contexts for stage execution
//! - Thread-safe data bags for storing outputs

mod bags;
#[cfg(test)]
mod context_tests;
mod execution;
mod identity;
mod inputs;
mod snapshot;

pub use bags::{ContextBag, OutputBag};
pub use execution::{DictContextAdapter, ExecutionContext, PipelineContext, StageContext};
pub use identity::RunIdentity;
pub use inputs::StageInputs;
pub use snapshot::{ContextSnapshot, Conversation, Enrichments, ExtensionBundle};
