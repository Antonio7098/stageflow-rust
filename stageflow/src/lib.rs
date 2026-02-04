//! # Stageflow
//!
//! A Rust implementation of the stageflow pipeline framework.
//!
//! Stageflow provides a structured approach to building data processing pipelines
//! with support for:
//!
//! - **Stage-based execution**: Define discrete processing stages with dependencies
//! - **Context management**: Immutable snapshots and mutable execution contexts
//! - **Event-driven observability**: Comprehensive event emission for monitoring
//! - **Tool integration**: Extensible tool registry with approval workflows
//! - **Cancellation handling**: Structured cancellation with cleanup guarantees
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use stageflow::prelude::*;
//!
//! // Define a pipeline
//! let pipeline = PipelineBuilder::new("my-pipeline")
//!     .stage("fetch", FetchStage::new())
//!     .stage("process", ProcessStage::new(), &["fetch"])
//!     .stage("store", StoreStage::new(), &["process"])
//!     .build()?;
//!
//! // Execute the pipeline
//! let result = pipeline.run(context).await?;
//! ```

#![forbid(unsafe_code)]
#![warn(
    clippy::all,
    clippy::pedantic,
    missing_docs,
    rust_2018_idioms
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]

pub mod cancellation;
pub mod compression;
pub mod context;
pub mod contracts;
pub mod core;
pub mod errors;
pub mod events;
pub mod helpers;
pub mod interceptors;
pub mod observability;
pub mod pipeline;
pub mod stages;
pub mod subpipeline;
pub mod tools;
pub mod utils;

#[cfg(feature = "websearch")]
pub mod websearch;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::cancellation::{
        CancellationToken, CleanupRegistry, StructuredTaskGroup,
    };
    pub use crate::context::{
        ContextBag, ContextSnapshot, DictContextAdapter, ExecutionContext,
        OutputBag, PipelineContext, RunIdentity, StageContext, StageInputs,
    };
    pub use crate::core::{
        StageArtifact, StageEvent, StageKind, StageOutput, StageStatus,
    };
    pub use crate::errors::{
        ContractErrorInfo, CycleDetectedError, DataConflictError,
        OutputConflictError, PipelineValidationError, StageflowError,
        UndeclaredDependencyError,
    };
    pub use crate::events::{EventSink, LoggingEventSink, NoOpEventSink};
    pub use crate::pipeline::{
        FluentPipelineBuilder, PipelineBuilder, PipelineSpec, StageGraph,
        StageSpec, UnifiedStageGraph,
    };
    pub use crate::stages::Stage;
    pub use crate::tools::{
        ToolDefinition, ToolInput, ToolOutput, ToolRegistry, UndoMetadata,
    };
    pub use crate::utils::{generate_uuid, iso_timestamp, Timestamp};
}

#[cfg(test)]
mod tests {
    #[test]
    fn library_compiles() {
        assert!(true);
    }
}
