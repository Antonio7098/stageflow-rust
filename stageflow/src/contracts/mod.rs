//! Contract validation for stage outputs.
//!
//! This module provides:
//! - Typed output validation
//! - Schema-based validation
//! - Output field extraction
//! - Contract error metadata
//! - Contract registry for versioning

mod errors;
mod registry;
mod suggestions;
mod typed_output;

pub use errors::{ContractErrorInfo, codes};
pub use registry::{
    ContractCompatibilityReport, ContractMetadata, ContractRegistry, REGISTRY,
};
pub use suggestions::{
    ContractSuggestion, get_contract_suggestion, list_suggestions, register_suggestion,
};
pub use typed_output::{
    IntoStageOutput, TypedOutputConfig, TypedStageOutput, ValidationError,
    extract_field, validate_output_fields,
};
