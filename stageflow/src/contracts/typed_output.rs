//! Typed output helper that validates StageOutput payloads.
//!
//! Provides compile-time and runtime validation for stage output data
//! using serde for serialization/deserialization.

use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::marker::PhantomData;

use crate::core::StageOutput;

/// Error during typed output validation.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error message.
    pub message: String,
    /// Field that caused the error, if applicable.
    pub field: Option<String>,
}

impl ValidationError {
    /// Creates a new validation error.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            field: None,
        }
    }

    /// Creates a validation error for a specific field.
    #[must_use]
    pub fn for_field(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            field: Some(field.into()),
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref field) = self.field {
            write!(f, "Field '{}': {}", field, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for ValidationError {}

/// Configuration for typed output validation.
#[derive(Debug, Clone, Default)]
pub struct TypedOutputConfig {
    /// Whether to use strict validation.
    pub strict: bool,
    /// Default version string for outputs.
    pub default_version: Option<String>,
    /// Additional context for error messages.
    pub context: HashMap<String, String>,
}

impl TypedOutputConfig {
    /// Creates a new config.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables strict validation.
    #[must_use]
    pub fn strict(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Sets the default version.
    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.default_version = Some(version.into());
        self
    }

    /// Adds context.
    #[must_use]
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
}

/// Typed stage output builder with validation.
#[derive(Debug)]
pub struct TypedStageOutput<T> {
    config: TypedOutputConfig,
    _marker: PhantomData<T>,
}

impl<T> TypedStageOutput<T>
where
    T: Serialize + DeserializeOwned,
{
    /// Creates a new typed output handler.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: TypedOutputConfig::default(),
            _marker: PhantomData,
        }
    }

    /// Creates a new typed output handler with config.
    #[must_use]
    pub fn with_config(config: TypedOutputConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    /// Validates and serializes a payload.
    pub fn validate(&self, payload: &T) -> Result<serde_json::Value, ValidationError> {
        serde_json::to_value(payload)
            .map_err(|e| ValidationError::new(format!("Serialization error: {}", e)))
    }

    /// Validates and returns a dictionary representation.
    pub fn serialize(&self, payload: &T) -> Result<HashMap<String, serde_json::Value>, ValidationError> {
        let value = self.validate(payload)?;
        
        match value {
            serde_json::Value::Object(map) => {
                Ok(map.into_iter().collect())
            }
            _ => Err(ValidationError::new("Payload must serialize to an object")),
        }
    }

    /// Validates payload and produces a successful StageOutput.
    pub fn ok(&self, payload: &T) -> Result<StageOutput, ValidationError> {
        let data = self.serialize(payload)?;
        let mut output = StageOutput::ok(data);
        
        if let Some(ref version) = self.config.default_version {
            output = output.add_metadata("version", serde_json::json!(version));
        }
        
        Ok(output)
    }

    /// Creates a typed output from a JSON value, validating the structure.
    pub fn from_json(&self, value: serde_json::Value) -> Result<T, ValidationError> {
        serde_json::from_value(value)
            .map_err(|e| ValidationError::new(format!("Deserialization error: {}", e)))
    }

    /// Creates a typed output from a dictionary.
    pub fn from_dict(&self, data: HashMap<String, serde_json::Value>) -> Result<T, ValidationError> {
        let value = serde_json::Value::Object(data.into_iter().collect());
        self.from_json(value)
    }
}

impl<T> Default for TypedStageOutput<T>
where
    T: Serialize + DeserializeOwned,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for types that can be converted to StageOutput.
pub trait IntoStageOutput {
    /// Converts to a StageOutput.
    fn into_stage_output(self) -> Result<StageOutput, ValidationError>;
}

impl<T: Serialize> IntoStageOutput for T {
    fn into_stage_output(self) -> Result<StageOutput, ValidationError> {
        let value = serde_json::to_value(&self)
            .map_err(|e| ValidationError::new(format!("Serialization error: {}", e)))?;
        
        match value {
            serde_json::Value::Object(map) => {
                let data: HashMap<String, serde_json::Value> = map.into_iter().collect();
                Ok(StageOutput::ok(data))
            }
            _ => Err(ValidationError::new("Payload must serialize to an object")),
        }
    }
}

/// Validates that a StageOutput contains expected fields.
pub fn validate_output_fields(
    output: &StageOutput,
    required_fields: &[&str],
) -> Result<(), ValidationError> {
    let data = output.data.as_ref().ok_or_else(|| {
        ValidationError::new("Output has no data")
    })?;

    for field in required_fields {
        if !data.contains_key(*field) {
            return Err(ValidationError::for_field(
                *field,
                "Required field is missing",
            ));
        }
    }

    Ok(())
}

/// Extracts a typed value from StageOutput data.
pub fn extract_field<T: DeserializeOwned>(
    output: &StageOutput,
    field: &str,
) -> Result<T, ValidationError> {
    let data = output.data.as_ref().ok_or_else(|| {
        ValidationError::new("Output has no data")
    })?;

    let value = data.get(field).ok_or_else(|| {
        ValidationError::for_field(field, "Field not found")
    })?;

    serde_json::from_value(value.clone())
        .map_err(|e| ValidationError::for_field(field, format!("Invalid type: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestPayload {
        text: String,
        count: i32,
    }

    #[test]
    fn test_validation_error() {
        let err = ValidationError::new("test error");
        assert_eq!(err.to_string(), "test error");

        let field_err = ValidationError::for_field("name", "is required");
        assert_eq!(field_err.to_string(), "Field 'name': is required");
    }

    #[test]
    fn test_typed_output_config() {
        let config = TypedOutputConfig::new()
            .strict()
            .with_version("1.0")
            .with_context("stage", "test");

        assert!(config.strict);
        assert_eq!(config.default_version, Some("1.0".to_string()));
        assert_eq!(config.context.get("stage"), Some(&"test".to_string()));
    }

    #[test]
    fn test_typed_stage_output_validate() {
        let typed: TypedStageOutput<TestPayload> = TypedStageOutput::new();
        let payload = TestPayload {
            text: "hello".to_string(),
            count: 42,
        };

        let result = typed.validate(&payload);
        assert!(result.is_ok());
    }

    #[test]
    fn test_typed_stage_output_serialize() {
        let typed: TypedStageOutput<TestPayload> = TypedStageOutput::new();
        let payload = TestPayload {
            text: "hello".to_string(),
            count: 42,
        };

        let result = typed.serialize(&payload);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.get("text"), Some(&serde_json::json!("hello")));
        assert_eq!(data.get("count"), Some(&serde_json::json!(42)));
    }

    #[test]
    fn test_typed_stage_output_ok() {
        let typed: TypedStageOutput<TestPayload> = TypedStageOutput::new();
        let payload = TestPayload {
            text: "done".to_string(),
            count: 100,
        };

        let result = typed.ok(&payload);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.is_success());
        assert_eq!(output.get("text"), Some(&serde_json::json!("done")));
    }

    #[test]
    fn test_typed_stage_output_with_version() {
        let config = TypedOutputConfig::new().with_version("2.0");
        let typed: TypedStageOutput<TestPayload> = TypedStageOutput::with_config(config);
        let payload = TestPayload {
            text: "test".to_string(),
            count: 1,
        };

        let result = typed.ok(&payload);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.metadata.get("version"), Some(&serde_json::json!("2.0")));
    }

    #[test]
    fn test_typed_stage_output_from_json() {
        let typed: TypedStageOutput<TestPayload> = TypedStageOutput::new();
        let json = serde_json::json!({
            "text": "from json",
            "count": 5
        });

        let result = typed.from_json(json);
        assert!(result.is_ok());

        let payload = result.unwrap();
        assert_eq!(payload.text, "from json");
        assert_eq!(payload.count, 5);
    }

    #[test]
    fn test_typed_stage_output_from_dict() {
        let typed: TypedStageOutput<TestPayload> = TypedStageOutput::new();
        let mut data = HashMap::new();
        data.insert("text".to_string(), serde_json::json!("from dict"));
        data.insert("count".to_string(), serde_json::json!(10));

        let result = typed.from_dict(data);
        assert!(result.is_ok());

        let payload = result.unwrap();
        assert_eq!(payload.text, "from dict");
        assert_eq!(payload.count, 10);
    }

    #[test]
    fn test_validate_output_fields() {
        let output = StageOutput::ok_value("name", serde_json::json!("test"));

        let result = validate_output_fields(&output, &["name"]);
        assert!(result.is_ok());

        let result = validate_output_fields(&output, &["missing"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_field() {
        let mut data = HashMap::new();
        data.insert("count".to_string(), serde_json::json!(42));
        data.insert("name".to_string(), serde_json::json!("test"));
        let output = StageOutput::ok(data);

        let count: i32 = extract_field(&output, "count").unwrap();
        assert_eq!(count, 42);

        let name: String = extract_field(&output, "name").unwrap();
        assert_eq!(name, "test");

        let missing: Result<String, _> = extract_field(&output, "missing");
        assert!(missing.is_err());
    }
}
