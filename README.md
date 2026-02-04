# Stageflow (Rust)

A complete Rust implementation of the stageflow pipeline framework with **436 tests passing**.

## Overview

Stageflow provides a structured approach to building data processing pipelines with support for:

- **Stage-based execution**: Define discrete processing stages with dependencies
- **Parallel DAG execution**: Automatic parallel execution based on dependency graph
- **Context management**: Immutable snapshots and mutable execution contexts
- **Failure tolerance**: FailFast, ContinueOnFailure, and BestEffort modes
- **Retry with backoff**: Exponential, linear, and constant backoff strategies with jitter
- **Idempotency**: Cache results and prevent duplicate executions
- **Cancellation handling**: Structured cancellation with cleanup guarantees
- **Contracts**: Typed output validation and schema registry
- **Event-driven observability**: Wide events, tracing, and OpenTelemetry integration
- **Tool integration**: Extensible tool registry with approval workflows
- **Python bindings**: PyO3-based bindings for Python interop

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

- **core**: Stage primitives (`StageOutput`, `StageStatus`, `StageKind`, `StageArtifact`, `StageEvent`)
- **context**: Execution contexts (`PipelineContext`, `StageContext`, `RunIdentity`, `ContextSnapshot`)
- **pipeline**: Pipeline builder and execution
  - `PipelineBuilder`, `StageSpec`, `StageGraph`
  - `FailureMode`, `FailureCollector`, `BackpressureTracker`
  - `RetryConfig`, `BackoffStrategy`, `JitterStrategy`
  - `IdempotencyStore`, `CachedResult`
  - `CancellationToken`, `CleanupRegistry`
- **contracts**: Typed output validation
  - `TypedStageOutput`, `ValidationError`
  - `ContractRegistry`, `ContractMetadata`
  - `ContractSuggestion`, `ContractErrorInfo`
- **stages**: Stage traits and implementations
  - `Stage` trait, `NoOpStage`, `FnStage`
  - `StageResult`, `StageError`
  - `StagePorts`, `CorePorts`, `LLMPorts`, `AudioPorts`
- **observability**: Tracing and wide events
  - `WideEventEmitter`, `TracingEmitter`
  - `PipelineSpanAttributes`, `StageSpanAttributes`
- **events**: Event sink system
- **tools**: Tool registry and execution
- **helpers**: Analytics, streaming, guardrails, UUID utilities
- **websearch**: Web content fetching and extraction
- **testing**: Mock stages, assertions, fixtures

## Python Bindings

The `stageflow-py` crate provides PyO3 bindings:

```python
from stageflow_py import StageOutput, RunIdentity, RetryConfig

# Create outputs
output = StageOutput.ok_empty()
failed = StageOutput.fail("Something went wrong")

# Configure retry
retry = RetryConfig(max_attempts=5, base_delay_ms=1000)

# Run identity
identity = RunIdentity().with_user_id("user-123")
```

Build the Python wheel:

```bash
cd stageflow-py
maturin build --release
pip install target/wheels/*.whl
```

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
