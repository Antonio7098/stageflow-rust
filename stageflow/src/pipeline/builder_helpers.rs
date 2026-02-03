//! Helper DSL for building pipelines.

use super::{PipelineBuilder, StageSpec};
use crate::errors::PipelineValidationError;
use crate::stages::{NoOpStage, Stage};
use std::sync::Arc;

/// A fluent pipeline builder that tracks the last added stage.
#[derive(Debug)]
pub struct FluentPipelineBuilder {
    /// The underlying builder.
    inner: PipelineBuilder,
    /// The name of the last added stage.
    last_stage: Option<String>,
}

impl FluentPipelineBuilder {
    /// Creates a new fluent builder.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            inner: PipelineBuilder::new(name),
            last_stage: None,
        }
    }

    /// Adds a stage. Does NOT auto-add dependencies unless explicitly provided.
    pub fn stage(
        mut self,
        name: impl Into<String>,
        runner: Arc<dyn Stage>,
        dependencies: &[&str],
    ) -> Result<Self, PipelineValidationError> {
        let name = name.into();
        self.inner = self.inner.stage(&name, runner, dependencies)?;
        self.last_stage = Some(name);
        Ok(self)
    }

    /// Adds a linear chain of stages.
    ///
    /// - `count <= 0` returns the builder unchanged.
    /// - First stage depends on `first_depends_on` if provided.
    /// - Subsequent stages depend on the previous stage.
    pub fn with_linear_chain(
        mut self,
        prefix: &str,
        count: usize,
        first_depends_on: Option<&str>,
    ) -> Result<Self, PipelineValidationError> {
        if count == 0 {
            return Ok(self);
        }

        for i in 0..count {
            let name = format!("{}{}", prefix, i + 1);
            let runner: Arc<dyn Stage> = Arc::new(NoOpStage::new(&name));

            let deps: Vec<&str> = if i == 0 {
                first_depends_on.into_iter().collect()
            } else {
                vec![&*Box::leak(format!("{}{}", prefix, i).into_boxed_str())]
            };

            // Build deps properly
            let deps: Vec<String> = if i == 0 {
                first_depends_on.iter().map(|s| s.to_string()).collect()
            } else {
                vec![format!("{}{}", prefix, i)]
            };

            let spec = StageSpec::new(&name, runner)
                .with_dependencies(deps);

            self.inner.add_stage_spec(spec)?;
            self.last_stage = Some(name);
        }

        Ok(self)
    }

    /// Adds parallel stages that share the same dependencies.
    ///
    /// - `count <= 0` returns unchanged.
    /// - All stages depend on the provided `depends_on`.
    pub fn with_parallel_stages(
        mut self,
        prefix: &str,
        count: usize,
        depends_on: &[&str],
    ) -> Result<Self, PipelineValidationError> {
        if count == 0 {
            return Ok(self);
        }

        let deps: Vec<String> = depends_on.iter().map(|s| s.to_string()).collect();

        for i in 0..count {
            let name = format!("{}{}", prefix, i + 1);
            let runner: Arc<dyn Stage> = Arc::new(NoOpStage::new(&name));

            let spec = StageSpec::new(&name, runner)
                .with_dependencies(deps.clone());

            self.inner.add_stage_spec(spec)?;
            self.last_stage = Some(name);
        }

        Ok(self)
    }

    /// Adds a fan-out/fan-in pattern.
    ///
    /// - Fan-out stage may depend on provided deps.
    /// - Worker stages depend on fan-out.
    /// - Fan-in depends on all workers.
    pub fn with_fan_out_fan_in(
        mut self,
        fan_out_name: &str,
        worker_prefix: &str,
        worker_count: usize,
        fan_in_name: &str,
        depends_on: &[&str],
    ) -> Result<Self, PipelineValidationError> {
        // Fan-out stage
        let runner: Arc<dyn Stage> = Arc::new(NoOpStage::new(fan_out_name));
        let spec = StageSpec::new(fan_out_name, runner)
            .with_dependencies(depends_on.iter().map(|s| s.to_string()));
        self.inner.add_stage_spec(spec)?;

        // Worker stages
        let mut worker_names = Vec::new();
        for i in 0..worker_count {
            let name = format!("{}{}", worker_prefix, i + 1);
            worker_names.push(name.clone());

            let runner: Arc<dyn Stage> = Arc::new(NoOpStage::new(&name));
            let spec = StageSpec::new(&name, runner)
                .with_dependency(fan_out_name);
            self.inner.add_stage_spec(spec)?;
        }

        // Fan-in stage
        let runner: Arc<dyn Stage> = Arc::new(NoOpStage::new(fan_in_name));
        let spec = StageSpec::new(fan_in_name, runner)
            .with_dependencies(worker_names);
        self.inner.add_stage_spec(spec)?;

        self.last_stage = Some(fan_in_name.to_string());
        Ok(self)
    }

    /// Adds a conditional branch pattern.
    ///
    /// - Router stage may depend on provided deps.
    /// - Each branch stage depends on router and is marked conditional.
    /// - Merge stage depends on all branches.
    pub fn with_conditional_branch(
        mut self,
        router_name: &str,
        branch_names: &[&str],
        merge_name: &str,
        depends_on: &[&str],
    ) -> Result<Self, PipelineValidationError> {
        // Router stage
        let runner: Arc<dyn Stage> = Arc::new(NoOpStage::new(router_name));
        let spec = StageSpec::new(router_name, runner)
            .with_dependencies(depends_on.iter().map(|s| s.to_string()));
        self.inner.add_stage_spec(spec)?;

        // Branch stages (conditional)
        for branch in branch_names {
            let runner: Arc<dyn Stage> = Arc::new(NoOpStage::new(*branch));
            let spec = StageSpec::new(*branch, runner)
                .with_dependency(router_name)
                .conditional();
            self.inner.add_stage_spec(spec)?;
        }

        // Merge stage
        let runner: Arc<dyn Stage> = Arc::new(NoOpStage::new(merge_name));
        let spec = StageSpec::new(merge_name, runner)
            .with_dependencies(branch_names.iter().map(|s| s.to_string()));
        self.inner.add_stage_spec(spec)?;

        self.last_stage = Some(merge_name.to_string());
        Ok(self)
    }

    /// Returns the last added stage name.
    #[must_use]
    pub fn last_stage(&self) -> Option<&str> {
        self.last_stage.as_deref()
    }

    /// Returns the underlying builder.
    #[must_use]
    pub fn into_inner(self) -> PipelineBuilder {
        self.inner
    }

    /// Builds the pipeline.
    pub fn build(self) -> Result<super::StageGraph, PipelineValidationError> {
        self.inner.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fluent_builder_creation() {
        let builder = FluentPipelineBuilder::new("test");
        assert!(builder.last_stage().is_none());
    }

    #[test]
    fn test_linear_chain_empty() {
        let builder = FluentPipelineBuilder::new("test")
            .with_linear_chain("stage", 0, None)
            .unwrap();

        assert!(builder.last_stage().is_none());
    }

    #[test]
    fn test_linear_chain() {
        let builder = FluentPipelineBuilder::new("test")
            .with_linear_chain("stage", 3, None)
            .unwrap();

        assert_eq!(builder.last_stage(), Some("stage3"));
        assert_eq!(builder.inner.stage_count(), 3);
    }

    #[test]
    fn test_parallel_stages() {
        let builder = FluentPipelineBuilder::new("test")
            .with_linear_chain("init", 1, None)
            .unwrap()
            .with_parallel_stages("worker", 3, &["init1"])
            .unwrap();

        assert_eq!(builder.inner.stage_count(), 4);
    }

    #[test]
    fn test_fan_out_fan_in() {
        let builder = FluentPipelineBuilder::new("test")
            .with_fan_out_fan_in("scatter", "worker", 3, "gather", &[])
            .unwrap();

        // scatter + 3 workers + gather = 5
        assert_eq!(builder.inner.stage_count(), 5);
        assert_eq!(builder.last_stage(), Some("gather"));
    }

    #[test]
    fn test_conditional_branch() {
        let builder = FluentPipelineBuilder::new("test")
            .with_conditional_branch("router", &["branch_a", "branch_b"], "merge", &[])
            .unwrap();

        // router + 2 branches + merge = 4
        assert_eq!(builder.inner.stage_count(), 4);
        assert_eq!(builder.last_stage(), Some("merge"));
    }
}
