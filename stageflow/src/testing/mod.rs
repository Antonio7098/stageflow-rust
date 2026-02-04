//! Testing utilities for stageflow pipelines.
//!
//! This module provides:
//! - Mock stages and contexts
//! - Test assertions for stage outputs
//! - Pipeline test harness

mod assertions;
mod fixtures;
mod mocks;

pub use assertions::{
    assert_output_contains, assert_output_failed, assert_output_has_data,
    assert_output_status, assert_output_succeeded,
};
pub use fixtures::{TestContext, TestFixture, TestPipeline};
pub use mocks::{
    FailingStage, MockStage, RecordingStage, SlowStage, SuccessStage,
};
