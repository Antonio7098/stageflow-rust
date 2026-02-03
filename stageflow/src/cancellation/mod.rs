//! Structured cancellation and cleanup utilities.
//!
//! This module provides:
//! - CancellationToken for cooperative cancellation
//! - CleanupRegistry for LIFO cleanup execution
//! - StructuredTaskGroup for managing related tasks

mod cleanup;
mod task_group;
mod token;

pub use cleanup::{cleanup_on_cancel, run_with_cleanup, CleanupRegistry};
pub use task_group::StructuredTaskGroup;
pub use token::CancellationToken;
