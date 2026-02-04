//! Contract validation for stage outputs.
//!
//! This module provides:
//! - Typed output validation
//! - Schema-based validation
//! - Output field extraction

mod typed_output;

pub use typed_output::{
    IntoStageOutput, TypedOutputConfig, TypedStageOutput, ValidationError,
    extract_field, validate_output_fields,
};
