# Stageflow (Rust)

A Rust implementation of the stageflow pipeline framework.

## Overview

Stageflow provides a structured approach to building data processing pipelines with support for:

- **Stage-based execution**: Define discrete processing stages with dependencies
- **Context management**: Immutable snapshots and mutable execution contexts
- **Event-driven observability**: Comprehensive event emission for monitoring
- **Tool integration**: Extensible tool registry with approval workflows
- **Cancellation handling**: Structured cancellation with cleanup guarantees

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
stageflow = "0.1"
```

## Quick Start

```rust
use stageflow::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a pipeline
    let graph = PipelineBuilder::new("my-pipeline")
        .stage("fetch", Arc::new(FetchStage::new()), &[])?
        .stage("process", Arc::new(ProcessStage::new()), &["fetch"])?
        .stage("store", Arc::new(StoreStage::new()), &["process"])?
        .build()?;

    // Create execution context
    let ctx = Arc::new(PipelineContext::new(RunIdentity::new()));
    let snapshot = ContextSnapshot::new();

    // Execute the pipeline
    let result = graph.execute(ctx, snapshot).await?;
    
    println!("Pipeline completed: {:?}", result.success);
    Ok(())
}
```

## Features

- `full` (default): Enables all features
- `websearch`: Web search and content extraction

## Architecture

The framework is organized into these main modules:

- **core**: Stage primitives (StageOutput, StageStatus, StageKind)
- **context**: Execution contexts and snapshots
- **pipeline**: Pipeline builder and DAG execution
- **events**: Event sink system for observability
- **tools**: Tool registry and execution
- **cancellation**: Structured cancellation and cleanup
- **interceptors**: Middleware for stage execution
- **helpers**: Analytics, streaming, guardrails, and more

## Development

```bash
# Run tests
cargo test

# Run clippy
cargo clippy --all-features

# Format code
cargo fmt

# Build docs
cargo doc --open
```

## License

MIT
