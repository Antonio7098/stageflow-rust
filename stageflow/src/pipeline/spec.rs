//! Pipeline and stage specifications.

use crate::core::StageKind;
use crate::errors::PipelineValidationError;
use crate::stages::Stage;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;

/// Specification for a single stage in a pipeline.
#[derive(Debug, Clone)]
pub struct StageSpec {
    /// The unique name of the stage.
    pub name: String,
    /// The stage implementation.
    pub runner: Arc<dyn Stage>,
    /// Names of stages this stage depends on.
    pub dependencies: HashSet<String>,
    /// Whether this stage is conditional.
    pub conditional: bool,
    /// The kind of stage.
    pub kind: StageKind,
}

impl StageSpec {
    /// Creates a new stage specification.
    #[must_use]
    pub fn new(name: impl Into<String>, runner: Arc<dyn Stage>) -> Self {
        Self {
            name: name.into(),
            runner,
            dependencies: HashSet::new(),
            conditional: false,
            kind: StageKind::Work,
        }
    }

    /// Sets the dependencies.
    #[must_use]
    pub fn with_dependencies(mut self, deps: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.dependencies = deps.into_iter().map(Into::into).collect();
        self
    }

    /// Adds a dependency.
    #[must_use]
    pub fn with_dependency(mut self, dep: impl Into<String>) -> Self {
        self.dependencies.insert(dep.into());
        self
    }

    /// Marks the stage as conditional.
    #[must_use]
    pub fn conditional(mut self) -> Self {
        self.conditional = true;
        self
    }

    /// Sets the stage kind.
    #[must_use]
    pub fn with_kind(mut self, kind: StageKind) -> Self {
        self.kind = kind;
        self
    }

    /// Validates the stage specification.
    ///
    /// # Errors
    ///
    /// Returns an error if the stage depends on itself.
    pub fn validate(&self) -> Result<(), PipelineValidationError> {
        if self.dependencies.contains(&self.name) {
            return Err(PipelineValidationError::new(format!(
                "Stage '{}' cannot depend on itself",
                self.name
            ))
            .with_stages(vec![self.name.clone()]));
        }
        Ok(())
    }
}

/// Specification for an entire pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSpec {
    /// The pipeline name.
    pub name: String,
    /// Stage names in the pipeline.
    #[serde(default)]
    pub stages: Vec<String>,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl PipelineSpec {
    /// Creates a new pipeline specification.
    ///
    /// # Errors
    ///
    /// Returns an error if the name is empty or whitespace-only.
    pub fn new(name: impl Into<String>) -> Result<Self, PipelineValidationError> {
        let name = name.into();
        let trimmed = name.trim();

        if trimmed.is_empty() {
            return Err(PipelineValidationError::new(
                "Pipeline name cannot be empty or whitespace-only",
            ));
        }

        Ok(Self {
            name,
            stages: Vec::new(),
            metadata: std::collections::HashMap::new(),
        })
    }

    /// Adds stages to the specification.
    #[must_use]
    pub fn with_stages(mut self, stages: Vec<String>) -> Self {
        self.stages = stages;
        self
    }

    /// Adds metadata.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stages::NoOpStage;

    #[test]
    fn test_stage_spec_creation() {
        let runner = Arc::new(NoOpStage::new("test"));
        let spec = StageSpec::new("test", runner)
            .with_dependencies(["dep1", "dep2"])
            .with_kind(StageKind::Transform);

        assert_eq!(spec.name, "test");
        assert_eq!(spec.dependencies.len(), 2);
        assert_eq!(spec.kind, StageKind::Transform);
    }

    #[test]
    fn test_stage_spec_self_dependency() {
        let runner = Arc::new(NoOpStage::new("test"));
        let spec = StageSpec::new("test", runner).with_dependency("test");

        assert!(spec.validate().is_err());
    }

    #[test]
    fn test_pipeline_spec_creation() {
        let spec = PipelineSpec::new("my-pipeline").unwrap();
        assert_eq!(spec.name, "my-pipeline");
    }

    #[test]
    fn test_pipeline_spec_empty_name() {
        assert!(PipelineSpec::new("").is_err());
        assert!(PipelineSpec::new("   ").is_err());
    }

    #[test]
    fn test_conditional_stage() {
        let runner = Arc::new(NoOpStage::new("cond"));
        let spec = StageSpec::new("cond", runner).conditional();

        assert!(spec.conditional);
    }
}
