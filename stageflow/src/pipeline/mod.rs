//! Pipeline building and execution.
//!
//! This module provides:
//! - Pipeline specifications
//! - Pipeline builder with validation
//! - DAG execution engines

mod builder;
mod builder_helpers;
mod dag;
mod spec;
mod unified;

pub use builder::PipelineBuilder;
pub use builder_helpers::FluentPipelineBuilder;
pub use dag::StageGraph;
pub use spec::{PipelineSpec, StageSpec};
pub use unified::UnifiedStageGraph;
