//! Subpipeline spawning and management.

mod result;
mod spawner;
mod tracker;

pub use result::SubpipelineResult;
pub use spawner::SubpipelineSpawner;
pub use tracker::{ChildRunInfo, ChildRunTracker};
