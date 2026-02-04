//! Core domain model types for stageflow.
//!
//! This module contains the fundamental types used throughout the framework:
//! - Stage status and kind enums
//! - Stage output type with factory methods
//! - Stage artifacts and events

mod artifact;
mod event;
mod output;
#[cfg(test)]
mod output_tests;
mod status;

pub use artifact::StageArtifact;
pub use event::StageEvent;
pub use output::StageOutput;
pub use status::{StageKind, StageStatus};
