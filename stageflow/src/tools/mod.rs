//! Tools subsystem for extensible tool execution.
//!
//! This module provides:
//! - Tool definitions and registry
//! - Tool input/output types
//! - Approval and undo workflows
//! - Advanced tool executor

mod approval;
mod definitions;
mod errors;
mod executor;
mod registry;
mod undo;

pub use approval::ApprovalService;
pub use definitions::{ToolDefinition, ToolInput, ToolOutput};
pub use errors::*;
pub use executor::AdvancedToolExecutor;
pub use registry::{
    clear_tool_registry, get_tool_registry, register_tool, ResolvedToolCall, Tool, ToolRegistry,
    UnresolvedToolCall,
};
pub use undo::{UndoMetadata, UndoStore};
