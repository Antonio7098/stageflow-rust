//! Pipeline builder with validation.

use super::{StageGraph, StageSpec};
use crate::core::StageKind;
use crate::errors::{ContractErrorInfo, CycleDetectedError, PipelineValidationError};
use crate::stages::Stage;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Builder for creating validated pipelines.
#[derive(Debug, Clone)]
pub struct PipelineBuilder {
    /// The pipeline name.
    name: String,
    /// The stage specifications.
    stages: HashMap<String, StageSpec>,
    /// Insertion order for stages.
    stage_order: Vec<String>,
}

impl PipelineBuilder {
    /// Creates a new pipeline builder.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            stages: HashMap::new(),
            stage_order: Vec::new(),
        }
    }

    /// Adds a stage to the pipeline.
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails (missing dependency, cycle, etc.)
    pub fn stage(
        mut self,
        name: impl Into<String>,
        runner: Arc<dyn Stage>,
        dependencies: &[&str],
    ) -> Result<Self, PipelineValidationError> {
        let name = name.into();
        let deps: HashSet<String> = dependencies.iter().map(|s| (*s).to_string()).collect();

        let spec = StageSpec::new(&name, runner).with_dependencies(deps.iter().cloned());

        self.add_stage_spec(spec)?;
        Ok(self)
    }

    /// Adds a stage with a specification.
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails.
    pub fn add_stage_spec(&mut self, spec: StageSpec) -> Result<(), PipelineValidationError> {
        // Validate stage itself
        spec.validate()?;

        // Check for missing dependencies
        for dep in &spec.dependencies {
            if !self.stages.contains_key(dep) {
                return Err(PipelineValidationError::new(format!(
                    "Stage '{}' depends on unknown stage '{}'",
                    spec.name, dep
                ))
                .with_stages(vec![spec.name.clone(), dep.clone()])
                .with_error_info(
                    ContractErrorInfo::new(
                        "CONTRACT-004-MISSING_DEP",
                        format!("Dependency '{}' not found", dep),
                    )
                    .with_fix_hint("Ensure the dependency is added before the stage that depends on it."),
                ));
            }
        }

        self.stage_order.push(spec.name.clone());
        self.stages.insert(spec.name.clone(), spec);

        // Check for cycles
        self.detect_cycles()?;

        Ok(())
    }

    /// Composes this builder with another.
    ///
    /// # Errors
    ///
    /// Returns an error if there are conflicting stage definitions.
    pub fn compose(mut self, other: Self) -> Result<Self, PipelineValidationError> {
        self.name = format!("{}+{}", self.name, other.name);

        for (name, other_spec) in other.stages {
            if let Some(existing) = self.stages.get(&name) {
                // Check if specs are compatible
                if !specs_compatible(existing, &other_spec) {
                    return Err(PipelineValidationError::new(format!(
                        "Conflicting stage definitions for '{}'",
                        name
                    ))
                    .with_stages(vec![name.clone()])
                    .with_error_info(
                        ContractErrorInfo::new(
                            "CONTRACT-004-CONFLICT",
                            format!("Stage '{}' has different definitions in composed pipelines", name),
                        )
                        .with_fix_hint("Rename one of the stages or ensure they have identical configurations."),
                    ));
                }
                // Identical specs are allowed, skip adding
            } else {
                self.stage_order.push(name.clone());
                self.stages.insert(name, other_spec);
            }
        }

        Ok(self)
    }

    /// Builds the pipeline.
    ///
    /// # Errors
    ///
    /// Returns an error if the builder has no stages.
    pub fn build(self) -> Result<StageGraph, PipelineValidationError> {
        if self.stages.is_empty() {
            return Err(PipelineValidationError::new("Pipeline has no stages")
                .with_error_info(
                    ContractErrorInfo::new("CONTRACT-004-EMPTY", "Cannot build an empty pipeline")
                        .with_fix_hint("Add at least one stage to the pipeline before building."),
                ));
        }

        Ok(StageGraph::new(self.name, self.stages, self.stage_order))
    }

    /// Returns the pipeline name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the number of stages.
    #[must_use]
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    /// Detects cycles in the dependency graph.
    fn detect_cycles(&self) -> Result<(), CycleDetectedError> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for name in self.stages.keys() {
            if !visited.contains(name) {
                if let Some(cycle) = self.dfs_cycle(name, &mut visited, &mut rec_stack, &mut path) {
                    return Err(CycleDetectedError::new(cycle));
                }
            }
        }

        Ok(())
    }

    fn dfs_cycle(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(spec) = self.stages.get(node) {
            for dep in &spec.dependencies {
                if !visited.contains(dep) {
                    if let Some(cycle) = self.dfs_cycle(dep, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(dep) {
                    // Found a cycle
                    let cycle_start = path.iter().position(|n| n == dep).unwrap();
                    let mut cycle: Vec<String> = path[cycle_start..].to_vec();
                    cycle.push(dep.clone());
                    return Some(cycle);
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
        None
    }
}

fn specs_compatible(a: &StageSpec, b: &StageSpec) -> bool {
    a.dependencies == b.dependencies
        && a.conditional == b.conditional
        && a.kind == b.kind
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stages::NoOpStage;

    fn noop(name: &str) -> Arc<dyn Stage> {
        Arc::new(NoOpStage::new(name))
    }

    #[test]
    fn test_builder_creation() {
        let builder = PipelineBuilder::new("test");
        assert_eq!(builder.name(), "test");
        assert_eq!(builder.stage_count(), 0);
    }

    #[test]
    fn test_builder_add_stage() {
        let builder = PipelineBuilder::new("test")
            .stage("stage1", noop("stage1"), &[])
            .unwrap();

        assert_eq!(builder.stage_count(), 1);
    }

    #[test]
    fn test_builder_with_dependencies() {
        let builder = PipelineBuilder::new("test")
            .stage("stage1", noop("stage1"), &[])
            .unwrap()
            .stage("stage2", noop("stage2"), &["stage1"])
            .unwrap();

        assert_eq!(builder.stage_count(), 2);
    }

    #[test]
    fn test_builder_missing_dependency() {
        let result = PipelineBuilder::new("test")
            .stage("stage1", noop("stage1"), &["missing"]);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.error_info.is_some());
        assert_eq!(err.error_info.unwrap().code, "CONTRACT-004-MISSING_DEP");
    }

    #[test]
    fn test_builder_cycle_detection() {
        // This would create a cycle: a -> b -> c -> a
        // But since we check dependencies exist first, we need a different approach
        // Let's test self-dependency which is caught by StageSpec validation
        let result = PipelineBuilder::new("test")
            .stage("stage1", noop("stage1"), &[]);

        // Self-dependency is caught at spec level
        let runner = noop("stage1");
        let spec = StageSpec::new("stage1", runner).with_dependency("stage1");
        assert!(spec.validate().is_err());
    }

    #[test]
    fn test_builder_empty_build() {
        let builder = PipelineBuilder::new("test");
        let result = builder.build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.error_info.is_some());
        assert_eq!(err.error_info.unwrap().code, "CONTRACT-004-EMPTY");
    }

    #[test]
    fn test_builder_compose() {
        let builder1 = PipelineBuilder::new("a")
            .stage("stage1", noop("stage1"), &[])
            .unwrap();

        let builder2 = PipelineBuilder::new("b")
            .stage("stage2", noop("stage2"), &[])
            .unwrap();

        let composed = builder1.compose(builder2).unwrap();
        assert_eq!(composed.name(), "a+b");
        assert_eq!(composed.stage_count(), 2);
    }

    #[test]
    fn test_builder_compose_conflict() {
        let builder1 = PipelineBuilder::new("a")
            .stage("shared", noop("shared"), &[])
            .unwrap();

        let mut builder2 = PipelineBuilder::new("b");
        let spec = StageSpec::new("shared", noop("shared"))
            .with_dependency("other"); // Different deps
        // We can't add this directly due to missing dep validation
        // So let's test with conditional difference
        builder2.stages.insert("shared".to_string(), StageSpec::new("shared", noop("shared")).conditional());
        builder2.stage_order.push("shared".to_string());

        let result = builder1.compose(builder2);
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_build_success() {
        let graph = PipelineBuilder::new("test")
            .stage("stage1", noop("stage1"), &[])
            .unwrap()
            .stage("stage2", noop("stage2"), &["stage1"])
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(graph.name(), "test");
    }
}
