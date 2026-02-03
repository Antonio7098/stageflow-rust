# Stageflow Rust Porting Checklist

This document is the authoritative, phased tracking checklist for porting the Python `stageflow` framework to Rust.

Goals:
- Preserve **behavioral parity** with Python (including edge cases, error types, and event payloads).
- Provide **acceptance criteria** for each item so implementation can be validated objectively.

Non-goals:
- This document does not prescribe Rust architecture choices beyond what is required to match semantics.

---

## How to use this checklist

- Treat each `- [ ]` item as an individually testable deliverable.
- Prefer writing Rust tests that mirror the Python tests/behaviors described in the acceptance criteria.
- Where the Python behavior is known to be inconsistent or legacy, this checklist calls it out explicitly.

---

## Phase 0 — Project foundations (build, lint, baseline harness)

- [ ] **Workspace setup**
  - **Acceptance criteria**
    - A `cargo` workspace exists under `sf-rust/`.
    - CI can run `cargo fmt`, `cargo clippy`, and `cargo test`.

- [ ] **Deterministic time + UUID utilities**
  - **Acceptance criteria**
    - Provide helpers for `uuid` generation and RFC3339/ISO timestamps (UTC) consistent with Python `.isoformat()` usage.

---

## Phase 1 — Core domain model parity

### 1.1 Stage primitives

- [ ] **Stage kind + status enums** (`StageKind`, `StageStatus`)
  - **Acceptance criteria**
    - Rust exposes enums corresponding to Python kinds/statuses.
    - `StageStatus` supports: OK, SKIP, CANCEL, FAIL, RETRY (and any additional Python statuses if present).

- [ ] **StageOutput**
  - **Acceptance criteria**
    - Factory helpers exist matching Python semantics:
      - `ok(data=..., artifacts=..., events=..., metadata=...)`
      - `skip(reason=...)`
      - `cancel(reason=...)`
      - `fail(error=..., retryable=...)`
      - `retry(reason=..., retryable=...)`
    - `StageOutput` is treated as immutable once created.

- [ ] **Stage artifacts + stage events** (`StageArtifact`, `StageEvent`)
  - **Acceptance criteria**
    - Structured artifact/event types exist and can be emitted/collected similarly to Python.

### 1.2 Context snapshots (immutable, serializable)

- [ ] **RunIdentity**
  - **Acceptance criteria**
    - Carries identifiers:
      - `pipeline_run_id`, `request_id`, `session_id`, `user_id`, `org_id`, `interaction_id`
    - All IDs serialize to string form (UUID string) or `null`.

- [ ] **Conversation model**
  - **Acceptance criteria**
    - Stores message history and routing decision (where applicable) with stable serialization.

- [ ] **Enrichments model**
  - **Acceptance criteria**
    - Supports typed enrichment groups (profile, memory, documents, web_results) with stable serialization.

- [ ] **ExtensionBundle system**
  - **Acceptance criteria**
    - Typed extensions can register (type name -> serializer/deserializer) and round-trip.
    - Unknown extensions do not crash deserialization (match Python back-compat behavior).

- [ ] **ContextSnapshot**
  - **Acceptance criteria**
    - Snapshot is immutable.
    - `to_dict` includes both:
      - Composed keys (`run_id`, `enrichments`, `conversation`, `extensions`)
      - Legacy flattened convenience keys (e.g. `pipeline_run_id`, `request_id`, …) for compatibility.
    - `with_*` methods return a new snapshot (structural sharing optional).

### 1.3 Mutable execution contexts

- [ ] **ExecutionContext trait/protocol**
  - **Acceptance criteria**
    - Unifies APIs used by pipeline/stage contexts (event emission, metadata access, run IDs, execution mode).

- [ ] **PipelineContext**
  - **Acceptance criteria**
    - Holds:
      - run identifiers
      - topology and execution mode
      - mutable context data / enrichments / outputs
      - event sink reference
      - cancellation flag
    - `mark_canceled()` sets a boolean canceled flag (no additional side effects).
    - Supports `fork_for_subpipeline(...)` (child context) retaining parent linkage.

- [ ] **StageContext**
  - **Acceptance criteria**
    - Per-stage derived view of PipelineContext.
    - Provides access to declared stage inputs via `StageInputs`.

- [ ] **DictContextAdapter** (legacy adapter)
  - **Acceptance criteria**
    - Adapts a plain key/value dictionary into ExecutionContext behavior.

### 1.4 Output stores (thread-safe conflict semantics)

- [ ] **ContextBag**
  - **Acceptance criteria**
    - Thread-safe writes.
    - Writing an existing key raises a `DataConflictError`.
    - `to_dict()` returns a copy.

- [ ] **OutputBag**
  - **Acceptance criteria**
    - Thread-safe append-only collection of per-stage outputs with attempt tracking.
    - Retry semantics allow conditional overwrite according to Python’s rules.
    - Conflicts raise `OutputConflictError`.

### 1.5 StageInputs strictness

- [ ] **StageInputs**
  - **Acceptance criteria**
    - Provides immutable view of prior outputs.
    - Strict mode rejects undeclared dependencies:
      - Accessing missing/undeclared key raises `UndeclaredDependencyError`.

---

## Phase 2 — Pipeline specs, validation, and composition

### 2.1 PipelineSpec validation

- [ ] **PipelineSpec**
  - **Acceptance criteria**
    - Spec is immutable/frozen.
    - Name validation:
      - cannot be empty
      - cannot be whitespace-only
    - Cannot depend on itself.

### 2.2 Contract error metadata

- [ ] **ContractErrorInfo**
  - **Acceptance criteria**
    - Fields:
      - `code`, `summary`, `fix_hint`, `doc_url`, `context`
    - `with_context(...)` merges context maps.
    - `to_dict()` stable.

- [ ] **PipelineValidationError**
  - **Acceptance criteria**
    - Carries message, list of stages, optional `ContractErrorInfo`.
    - `to_dict()` matches Python shape.

- [ ] **Contract suggestions registry**
  - **Acceptance criteria**
    - Suggestion lookup by `code` exists.
    - Default suggestions exist for common builder failures (cycle, missing dep, etc.).

### 2.3 PipelineBuilder

- [ ] **PipelineBuilder validation on init**
  - **Acceptance criteria**
    - Missing dependency raises `PipelineValidationError` with:
      - code `CONTRACT-004-MISSING_DEP`
    - Cycle detection raises `CycleDetectedError` with:
      - code `CONTRACT-004-CYCLE`
      - a `cycle_path` list.

- [ ] **PipelineBuilder.compose semantics**
  - **Acceptance criteria**
    - New name is `"{left}+{right}"`.
    - Conflicting stage names with different runner/deps/conditional raise error:
      - code `CONTRACT-004-CONFLICT`
    - Identical stage specs are allowed.

- [ ] **PipelineBuilder.build semantics**
  - **Acceptance criteria**
    - Empty builder raises `PipelineValidationError` with:
      - code `CONTRACT-004-EMPTY`
    - Build returns an executable graph object.

### 2.4 Builder helper DSL

- [ ] **with_linear_chain**
  - **Acceptance criteria**
    - `count <= 0` returns the builder unchanged.
    - First stage depends on `first_depends_on` if provided.
    - Subsequent stages depend on the previous stage.

- [ ] **with_parallel_stages**
  - **Acceptance criteria**
    - `count <= 0` returns unchanged.
    - All stages share the same `depends_on` tuple (or empty).

- [ ] **with_fan_out_fan_in**
  - **Acceptance criteria**
    - Fan-out stage may depend on provided deps.
    - Parallel worker stages depend on fan-out.
    - Fan-in depends on all worker stage names.

- [ ] **with_conditional_branch**
  - **Acceptance criteria**
    - Router stage may depend on provided deps.
    - Each branch stage depends on router and is marked `conditional=True`.
    - Merge stage depends on all branches.

- [ ] **FluentPipelineBuilder**
  - **Acceptance criteria**
    - Tracks `_last_stage` for helper composition.
    - `.stage()` does not auto-add dependencies unless explicitly provided.

---

## Phase 3 — DAG execution engines (legacy and unified)

### 3.1 Common requirements

- [ ] **Topological execution & dependency enforcement**
  - **Acceptance criteria**
    - Stages run only when dependencies complete.
    - Missing dependencies and deadlocks error out deterministically.

- [ ] **Event emission for stage lifecycle**
  - **Acceptance criteria**
    - Emits `stage.started`, `stage.completed`, `stage.failed`, `stage.skipped` (and any others used by Python) with stable payloads.

### 3.2 Legacy StageGraph

- [ ] **Legacy StageGraph execution parity**
  - **Acceptance criteria**
    - Executes declared stages in DAG order.
    - `spec.conditional` is effectively a no-op (legacy graph does not implement conditional skipping based on it).

### 3.3 UnifiedStageGraph

- [ ] **UnifiedStageGraph execution parity**
  - **Acceptance criteria**
    - Supports conditional stages:
      - If inputs contain `skip_reason`, stage is skipped and `stage.skipped` is emitted.
    - Stores completed outputs in a stage-name-keyed map (Python does not use OutputBag here).

- [ ] **Cancellation behavior**
  - **Acceptance criteria**
    - If a stage returns `StageStatus.CANCEL`, graph raises a `UnifiedPipelineCancelled`-equivalent error.
    - Pipeline cancellation emits a pipeline-level cancellation event (Python uses `pipeline_cancelled`).

- [ ] **Guard-retry runtime**
  - **Acceptance criteria**
    - Implements `GuardRetryPolicy` and runtime state tracking:
      - attempts
      - stagnation (based on hashed payload)
      - timeout
    - Emits the full guard-retry event family:
      - `guard_retry.attempt`
      - `guard_retry.scheduled`
      - `guard_retry.exhausted`
      - `guard_retry.recovered`
    - Overwrite semantics:
      - guard outputs may be overwritten by newer attempts until finalized.

---

## Phase 4 — Structured cancellation and cleanup

- [ ] **CancellationToken**
  - **Acceptance criteria**
    - `cancel(reason)` is idempotent (first reason wins).
    - `on_cancel(cb)` calls immediately if already cancelled; otherwise stores.
    - Exceptions in cancel callbacks are suppressed (logged).

- [ ] **CleanupRegistry**
  - **Acceptance criteria**
    - `register(cb, name?)` stores callback; if name provided, also sets attribute `__cleanup_name__` on the callback.
    - `run_all(timeout=10.0)`:
      - executes callbacks in LIFO order
      - splits timeout across callbacks with min per-callback timeout 0.01s
      - continues after errors, returns aggregated failures for reporting
      - clears registry after completion
    - `unregister(cb)` removes it if present.

- [ ] **StructuredTaskGroup**
  - **Acceptance criteria**
    - Cancels remaining tasks if one errors.
    - Sets cancel_token reason on failure.
    - Always runs cleanup registry in `__aexit__` (even on success).

- [ ] **cleanup_on_cancel / run_with_cleanup**
  - **Acceptance criteria**
    - Cleanup runs in `finally` on normal exit, exception, or cancellation.
    - `run_with_cleanup` wraps cleanup in timeout and suppresses timeout errors.

---

## Phase 5 — Observability: wide events

- [ ] **WideEventEmitter**
  - **Acceptance criteria**
    - Emits through `ctx.event_sink.try_emit`.
    - Default event types:
      - stage: `stage.wide`
      - pipeline: `pipeline.wide`

- [ ] **stage.wide payload**
  - **Acceptance criteria**
    - Includes context metadata:
      - pipeline_run_id/request_id/session_id/user_id/org_id (string or null)
      - topology, execution_mode, service
    - Includes stage summary:
      - stage name
      - status
      - started_at/ended_at (iso)
      - duration_ms
      - error
      - data_keys sorted
    - Supports optional `extra` dict.

- [ ] **pipeline.wide payload**
  - **Acceptance criteria**
    - `pipeline_name` defaults to provided else ctx.topology else `"pipeline"`.
    - Status defaults to `failed` if any stage failed else `completed`.
    - Includes `stage_counts` and `stage_details`.

---

## Phase 6 — Interceptors (middleware semantics)

- [ ] **Interceptor chain execution**
  - **Acceptance criteria**
    - Ordered by priority.
    - Can short-circuit stage execution.
    - Can observe/transform errors.

- [ ] **IdempotencyInterceptor**
  - **Acceptance criteria**
    - Enforces idempotent execution of WORK stages.
    - Uses an idempotency store; handles concurrent requests with locking.

- [ ] **RetryInterceptor**
  - **Acceptance criteria**
    - Supports backoff strategies: exponential, linear, constant.
    - Supports jitter strategies: none, full, equal, decorrelated.
    - Emits retry events:
      - `stage.retry_scheduled`
      - `stage.retry_exhausted`

- [ ] **Hardening interceptors**
  - **Acceptance criteria**
    - Immutability interceptor detects snapshot mutation.
    - Context size interceptor warns on large or growing contexts and records metrics.

---

## Phase 7 — Subpipelines

- [ ] **ChildRunTracker**
  - **Acceptance criteria**
    - Thread-safe registration/unregistration.
    - Supports traversal and cleanup.

- [ ] **SubpipelineSpawner**
  - **Acceptance criteria**
    - Enforces max depth (Python default: 5).
    - Emits lifecycle events:
      - `pipeline.spawned_child`
      - `pipeline.child_completed`
      - `pipeline.child_failed`
      - `pipeline.canceled`
    - Cascades cancellation from parent to children.

- [ ] **SubpipelineResult**
  - **Acceptance criteria**
    - Captures child pipeline completion state, outputs, and errors.

---

## Phase 8 — Tools subsystem (registry, gating, undo, approval)

### 8.1 Tool registry and definitions

- [ ] **ToolDefinition (v2) semantics** (`stageflow/tools/definitions.py`)
  - **Acceptance criteria**
    - `ToolDefinition` fields exist and serialize (handlers excluded):
      - `name`, `action_type`, `description`, `input_schema`
      - `allowed_behaviors: tuple[str, ...]` (empty tuple means allow all)
      - `requires_approval: bool`, `approval_message: str | None`
      - `undoable: bool`, `undo_handler: Option`
      - `artifact_type: Option[str]`
    - `ToolDefinition.is_behavior_allowed(behavior)`:
      - returns `True` when `allowed_behaviors` is empty
      - otherwise returns `behavior in allowed_behaviors`

- [ ] **ToolInput.from_action mapping** (`ToolInput`)
  - **Acceptance criteria**
    - Input includes:
      - `action_id`, `tool_name`, `payload`
      - `behavior == ctx.execution_mode` (or `None` if ctx is `None`)
      - `pipeline_run_id == ctx.pipeline_run_id` (or `None`)
      - `request_id == ctx.request_id` (or `None`)
    - `ToolInput.to_dict()` serializes IDs to strings or `null`.

- [ ] **ToolOutput semantics**
  - **Acceptance criteria**
    - `ToolOutput.ok(data?, artifacts?, undo_metadata?)` produces `success=True`.
    - `ToolOutput.fail(error)` produces `success=False` and sets `error`.
    - `ToolOutput.to_dict()` omits absent optional fields (matches Python conditional inclusion).

- [ ] **UndoMetadata schema + (de)serialization**
  - **Acceptance criteria**
    - Fields:
      - `action_id`, `tool_name`, `undo_data`, `created_at`
    - `created_at` defaults to `datetime.now(UTC).isoformat()`.
    - `UndoMetadata.from_dict`:
      - parses `action_id` from string
      - if `created_at` missing, uses current time.

### 8.1b Legacy ToolRegistry (v1) and tool-call parsing

- [ ] **ToolRegistry instance + global singleton** (`stageflow/tools/registry.py`)
  - **Acceptance criteria**
    - Registry can:
      - `register(tool_instance)`
      - `register_factory(action_type, factory)` for lazy construction
      - `get_tool(action_type)`:
        - returns existing instance if registered
        - if only a factory is registered, constructs tool on first get and memoizes
      - `can_execute(action_type)` is true if instance or factory exists
      - `list_tools()` returns instances that have been realized/registered
      - `__contains__` delegates to `can_execute`
    - Global functions:
      - `get_tool_registry()` returns singleton
      - `clear_tool_registry()` resets singleton
      - `register_tool(tool_instance)` registers into the global registry and returns `None`

- [ ] **`@tool(...)` decorator side effects**
  - **Acceptance criteria**
    - Decorator stores metadata on the class:
      - `_tool_action_type`, `_tool_name`, `_tool_description`
    - Decorator registers the class as a factory in the global registry.
    - Decorator returns the class unchanged (class remains instantiable).

- [ ] **Tool call parsing + resolution** (`ToolRegistry.parse_and_resolve`)
  - **Acceptance criteria**
    - Supports OpenAI-style wrapper by default (`function_wrapper="function"`):
      - reads `call[id_field]` and `call[function_wrapper][name_field]` / `[arguments_field]`
    - Supports direct format when `function_wrapper=None`.
    - Supports custom field names via `id_field`, `name_field`, `arguments_field`.
    - Arguments parsing:
      - if string: JSON-decode; empty string becomes `{}`
      - invalid JSON produces an `UnresolvedToolCall` with `error` containing "Invalid JSON"
      - if dict: uses as-is
      - otherwise: `{}`
    - Resolution:
      - resolves by **action_type** equal to parsed `name`
      - unknown tool produces `UnresolvedToolCall` with message containing "No tool registered"
    - `raw` field preserves the original call dict.
    - `ResolvedToolCall` and `UnresolvedToolCall` are immutable (frozen).

### 8.2 Tool errors

- [ ] **Tool error taxonomy**
  - **Acceptance criteria**
    - Rust has distinct error types analogous to:
      - ToolNotFound
      - ToolDenied (behavior gating)
      - ToolApprovalDenied
      - ToolApprovalTimeout
      - ToolUndoError
      - ToolExecutionError
    - Each error serializes to a dictionary/map with stable keys.

### 8.3 HITL approval service

- [ ] **ApprovalService parity**
  - **Acceptance criteria**
    - Matches request lifecycle, timeout expiry semantics, cancel semantics, and singleton helpers.

### 8.4 Undo store

- [ ] **UndoStore**
  - **Acceptance criteria**
    - TTL-based storage; expired entries return None and are removed.
    - Global singleton helpers exist (get/set/clear).

### 8.5 AdvancedToolExecutor (v2)

- [ ] **Execution lifecycle**
  - **Acceptance criteria**
    - Emits:
      - `tool.invoked` before gating/approval
      - `tool.denied` when behavior not allowed or approval timeout
      - `approval.requested` when approval needed
      - `approval.decided` when decision arrives
      - `tool.started` before handler
      - `tool.completed` on success
      - `tool.failed` on handler exception
    - Behavior gating denial:
      - emits `tool.denied` with `reason="behavior_not_allowed"` and then raises ToolDenied.
    - Approval denial:
      - raises ToolApprovalDenied (note: Python does not include request_id in this exception).
    - Approval timeout:
      - emits `tool.denied` with `reason="approval_timeout"` and raises ToolApprovalTimeout including request_id + timeout.

- [ ] **ExecutionContext event enrichment in tool-related contexts**
  - **Acceptance criteria**
    - `StageContext.try_emit_event(type, data)` enriches payload with:
      - `pipeline_run_id`, `request_id` (as strings or `null`)
      - `execution_mode`
      - plus the provided `data`
    - If `event_sink` exists:
      - calls `event_sink.try_emit(type=..., data=enriched)`
      - suppresses exceptions (logs warning)
    - If no `event_sink`:
      - does not raise (debug logs only)
    - `PipelineContext.try_emit_event(type, data)` also enriches with `execution_mode` and includes `topology` when present (as validated by tests).
    - `DictContextAdapter.try_emit_event(...)` never raises; logs at debug level and enriches with pipeline/request IDs and execution_mode when present.

- [ ] **Undo semantics**
  - **Acceptance criteria**
    - Undo metadata stored only when:
      - tool is undoable
      - output.success
      - output.undo_metadata exists
    - Undo:
      - returns false if no metadata or no undo handler
      - emits `tool.undone` and deletes entry on success
      - emits `tool.undo_failed` and raises ToolUndoError on failure

---

## Phase 9 — Helper modules parity (analytics, streaming, mocks, memory, guardrails, compression)

This phase is intentionally split so each helper can be ported independently.

### 9.1 Analytics (`stageflow/helpers/analytics.py`)

- [ ] **AnalyticsEvent**
  - **Acceptance criteria**
    - Fields:
      - `event_type: str`
      - `timestamp: datetime` default `datetime.now(UTC)`
      - `data: dict` default `{}`
      - `pipeline_run_id: UUID | None`
      - `stage_name: str | None`
      - `duration_ms: float | None`
      - `metadata: dict` default `{}`
    - `to_dict()`:
      - always includes `event_type`, `timestamp` (isoformat), `data`
      - conditionally includes `pipeline_run_id` (string), `stage_name`, `duration_ms`, `metadata` only when present/non-empty
    - `from_dict()`:
      - parses `timestamp` from ISO string via `datetime.fromisoformat`
      - parses `pipeline_run_id` from string UUID when present

- [ ] **JSONFileExporter**
  - **Acceptance criteria**
    - Lazily opens file on first export.
    - Writes JSONL lines (`json.dumps(event.to_dict(), default=str)` + `\n`).
    - Creates parent directories.
    - Supports `append=True` (open `a`) and `append=False` (open `w`).
    - Tracks `event_count` for exported events.
    - `flush()` flushes file handle, `close()` closes and nulls file handle.

- [ ] **ConsoleExporter**
  - **Acceptance criteria**
    - `_format_event`:
      - uses ANSI colors when `colorize=True` based on event_type keywords
      - prints `[HH:MM:SS.mmm]` timestamp
      - includes stage name and duration when set
      - when `verbose=True`, prints JSON of `event.data`
    - Tracks `event_count`.
    - `flush()`/`close()` are no-ops.

- [ ] **BufferedExporter**
  - **Acceptance criteria**
    - Buffers events and calls `export_batch` on underlying exporter.
    - Flush triggers:
      - batch size reached
      - explicit `flush()`
      - `close()` (must flush remaining events)
      - background flush loop every `flush_interval_seconds` (task created on first export)
    - Overflow behavior:
      - if buffer is full (`len(buffer) >= max_buffer_size`), drop **oldest** item (`pop(0)`), increment `_dropped_count`
      - calls `on_overflow(dropped_count, buffer_size)` when dropping; callback exceptions are suppressed
    - High-water behavior:
      - when `fill_ratio >= high_water_mark` and not warned:
        - sets `_high_water_warned=True`
        - calls `on_overflow(-1, buffer_size)` as a high-water warning convention
      - warning resets when `fill_ratio < high_water_mark * 0.5`
    - `stats` property returns:
      - `buffer_size`, `max_buffer_size`, `fill_ratio`, `dropped_count`, `high_water_warned`
    - `close()`:
      - cancels flush task (suppresses CancelledError)
      - flushes
      - closes underlying exporter

- [ ] **CompositeExporter**
  - **Acceptance criteria**
    - `export`, `export_batch`, `flush`, `close` fan-out to all exporters using `asyncio.gather`.

- [ ] **AnalyticsSink (EventSink adapter)**
  - **Acceptance criteria**
    - Supports filtering:
      - `exclude_patterns`: if any substring is contained in event type, drop event
      - `include_patterns`: if provided, keep event only if any substring matches
    - Converts EventSink events to AnalyticsEvent:
      - `event_type=type`
      - `data=data or {}`
      - `pipeline_run_id = data.get("pipeline_run_id")` (note: Python stores as-is)
      - `stage_name = data.get("stage")`
      - `duration_ms = data.get("duration_ms")`
    - `try_emit` schedules export with `asyncio.create_task`.

### 9.2 Streaming primitives (`stageflow/helpers/streaming.py`)

- [ ] **AudioChunk**
  - **Acceptance criteria**
    - `duration_ms`:
      - uses `bytes_per_sample = 2` for `PCM_16`, else `4`
      - `samples = len(data) / (bytes_per_sample * channels)`
      - duration computed as `(samples / sample_rate) * 1000`
    - `to_dict()` base64 encodes `data` as ASCII string and includes:
      - sample_rate, channels, format.value, timestamp_ms, sequence, is_final, metadata
    - `from_dict()` base64 decodes `data` and applies defaults (16000 Hz, 1 channel, `pcm_16`).

- [ ] **BackpressureMonitor + BackpressureStats**
  - **Acceptance criteria**
    - `record_put(queue_size, max_size)`:
      - increments `total_items`
      - updates `max_queue_size`
      - updates `fill_percentage = (queue_size/max_size)*100`
      - throttling latch:
        - throttle when fill >= high_water_mark
        - release when fill <= low_water_mark
    - `record_blocked(blocked_ms)` increments `blocked_puts` and accumulates `total_blocked_ms`.
    - `record_drop()` increments `dropped_items`.
    - `BackpressureStats.to_dict()` includes `drop_rate = dropped_items / max(total_items, 1)`.

- [ ] **ChunkQueue**
  - **Acceptance criteria**
    - Supports bounded async queue with `max_size`.
    - `put(item)`:
      - if `closed`, returns `False`
      - if `drop_on_overflow=True` and queue is full:
        - drops oldest via `get_nowait()`
        - records drop
        - emits `stream.chunk_dropped` with `{queue_size,max_size,reason:"overflow"}`
      - if `drop_on_overflow=False`, blocks on `await queue.put(item)`
      - measures elapsed; if `elapsed_ms > 1`:
        - records blocked time
        - emits `stream.producer_blocked` with `{blocked_ms, queue_size}`
      - records `record_put(qsize, max_size)`
      - throttle telemetry:
        - on transition into throttling: emits `stream.throttle_started`
        - on transition out: emits `stream.throttle_ended`
      - returns `True` if enqueued
      - if `QueueFull` exception occurs: records drop and emits `stream.chunk_dropped` with reason `queue_full`
    - `get()`:
      - if `closed` and empty => returns `None`
      - otherwise returns next item; may return `None` sentinel pushed by close
    - `close()`:
      - sets closed
      - emits `stream.queue_closed` with final monitor stats
      - best-effort `put_nowait(None)` sentinel (suppresses QueueFull)
    - Async iteration yields until `None`, then stops.
    - Telemetry emitter errors are suppressed; queue continues functioning.

- [ ] **StreamingBuffer**
  - **Acceptance criteria**
    - Maintains byte buffer; uses `bytes_per_sample=2` (PCM_16 assumption).
    - `add_chunk(chunk)`:
      - while adding would exceed `max_duration_ms`, drops oldest audio in 50ms increments
      - if dropped > 0 emits `stream.buffer_overflow` with `{bytes_dropped, buffer_duration_ms, max_duration_ms}`
      - tracks `total_received` and `total_dropped`
    - `read(duration_ms)`:
      - reads up to requested bytes (may return less)
      - emits `stream.buffer_underrun` on transition into underrun state
      - emits `stream.buffer_recovered` on transition out of underrun state
      - updates `total_read`
    - `stats` includes:
      - `duration_ms`, `bytes_buffered`, `total_received`, `total_read`, `total_dropped`, `is_ready`, `underrun_active`
    - Telemetry emitter errors are suppressed.

- [ ] **Utility functions**
  - **Acceptance criteria**
    - `encode_audio_for_logging(data, max_bytes)`:
      - returns `<audio:{total}B,data:{b64}>` if not truncated
      - returns `<audio:{total}B,sample:{b64}...>` if truncated
    - `calculate_audio_duration_ms(byte_count, sample_rate, channels, bytes_per_sample)` matches formula used by tests.

### 9.3 Guardrails SDK (`stageflow/helpers/guardrails.py`)

- [ ] **Violation models**
  - **Acceptance criteria**
    - `ViolationType` enum includes (at least):
      - `pii_detected`, `profanity`, `toxicity`, `content_too_long`, `rate_limited`, `blocked_topic`, `injection_attempt`, `custom`
    - `PolicyViolation` fields:
      - `type`, `message`, `severity`, `metadata`, optional `location(start,end)`
    - `to_dict()` serializes type as string value and location as tuple or null.
    - `GuardrailResult` fields:
      - `passed`, `violations`, optional `transformed_content`, `metadata`
      - `to_dict()` includes violations as list of dicts

- [ ] **PIIDetector**
  - **Acceptance criteria**
    - Detects PII using regex patterns for:
      - email, phone, ssn, credit_card, ip_address
    - `detect_types` restricts which patterns run.
    - Emits violations with:
      - type `PII_DETECTED`, severity 0.8
      - metadata includes `pii_type`
      - location populated
    - Optional redaction:
      - replaces match with `redaction_char * len(match)`
      - returns `transformed_content` only if redaction enabled
    - Result metadata includes `pii_types_checked`.

- [ ] **ContentFilter**
  - **Acceptance criteria**
    - Leetspeak normalization is applied for detection (`_normalize_leetspeak`).
    - Profanity detection checks both original and normalized word sets.
    - Profanity violations:
      - type `PROFANITY`
      - metadata includes `word`
      - severity uses `max_severity` parameter
    - Blocked patterns:
      - regex search against original and normalized content
      - violations type `BLOCKED_TOPIC`, severity 0.9
      - metadata includes `pattern` and `normalized` boolean.

- [ ] **InjectionDetector**
  - **Acceptance criteria**
    - Checks a built-in list of injection regex patterns + `additional_patterns`.
    - On match emits violations:
      - type `INJECTION_ATTEMPT`, severity 1.0
      - metadata includes `matched_pattern`
      - location populated.

- [ ] **ContentLengthCheck**
  - **Acceptance criteria**
    - Uses:
      - `char_count = len(content)`
      - `token_count = char_count // 4` (approximation)
    - Emits violations of type `CONTENT_TOO_LONG` for max chars and max tokens.
    - Emits violation of type `CUSTOM` for min chars underflow.

- [ ] **GuardrailStage**
  - **Acceptance criteria**
    - Selects content:
      - if `content_key` set and `ctx.inputs` present: `ctx.inputs.get(content_key)`
      - else `ctx.snapshot.input_text`
    - If content missing/empty => `StageOutput.skip(reason="No content to check")`.
    - For each check:
      - passes a check context containing `user_id`/`session_id` as strings or null
      - filters violations by `violation_threshold`
      - applies `transformed_content` when `transform_content=True`
      - if `log_violations=True` and event sink present, emits:
        - `guardrail.violations_detected` with `{violations:[...], check:<CheckClassName>}`
    - Output data always includes:
      - `guardrail_passed: bool`, `violations: list`, `checks_run: int`
      - `transformed_content` included only when it changed and transformations enabled
    - If violations and `fail_on_violation=True`:
      - returns `StageOutput.fail(error="Guardrail violations: N found", data=output_data)`
    - If violations and `fail_on_violation=False`:
      - emits `guardrail.fail_open` audit event payload including:
        - stage name, pipeline_run_id, request_id, execution_mode
        - violation_count, fail_on_violation=false, violations list
      - returns `StageOutput.ok(**output_data)` (with `guardrail_passed=false`)

### 9.4 Memory helpers (`stageflow/helpers/memory.py`)

- [ ] **MemoryEntry**
  - **Acceptance criteria**
    - `timestamp` default is `datetime.now(UTC)`.
    - `to_dict()` serializes:
      - session_id as string
      - timestamp as ISO string
    - `from_dict()` parses session_id from string UUID and timestamp from ISO.

- [ ] **MemoryConfig + token approximation rules**
  - **Acceptance criteria**
    - Defaults:
      - `max_entries=20`, `max_tokens=4000`, `include_system=True`, `recency_window_seconds=0`
    - Token approximation is `len(content) // 4`.

- [ ] **InMemoryStore.fetch filtering order**
  - **Acceptance criteria**
    - Orders are oldest->newest in storage; fetch returns oldest->newest.
    - Filtering steps:
      - optional recency filter (`timestamp.timestamp() > now - recency_window_seconds`)
      - optional system filtering (`role != "system"` when `include_system=False`)
      - `max_entries` keeps the most recent N (`entries[-max_entries:]`)
      - `max_tokens` keeps most recent entries without exceeding max tokens:
        - iterates from newest backwards, inserts kept entries at front

- [ ] **MemoryFetchStage**
  - **Acceptance criteria**
    - If `snapshot.session_id is None` => `StageOutput.skip(reason="No session_id in context")`.
    - Otherwise outputs:
      - `memory_entries`: list of `MemoryEntry.to_dict()`
      - `memory_count`: len(entries)
      - `memory_tokens`: sum(len(content)//4)

- [ ] **MemoryWriteStage**
  - **Acceptance criteria**
    - If `snapshot.session_id is None` => `StageOutput.skip(reason="No session_id in context")`.
    - Writes user entry when `snapshot.input_text` is truthy:
      - `id = f"{interaction_id or 'unknown'}_user"`
      - `role="user"`
    - Writes assistant entry when `ctx.inputs` exists and `get_from(response_stage,response_key)` returns truthy:
      - `id = f"{interaction_id or 'unknown'}_assistant"`
      - `role="assistant"`
      - `content = str(response)`
    - Returns `StageOutput.ok(entries_written, session_id=str(session_id))`.

### 9.5 Mock providers (`stageflow/helpers/mocks.py`)

- [ ] **MockLLMProvider**
  - **Acceptance criteria**
    - Response selection order:
      - first regex `patterns` match (case-insensitive)
      - else echo mode (`"Echo: {prompt}"`) if enabled
      - else cycles through `responses` list
    - Latency simulation:
      - `latency_ms` plus uniform jitter in `[-latency_jitter_ms, +latency_jitter_ms]`
    - Failure simulation:
      - with probability `fail_rate`, raises `Exception(fail_error)`
    - Call history:
      - increments `call_count`
      - appends `{prompt, messages, timestamp}` (timestamp ISO string)
    - Token counting:
      - default `len(s)//4` unless custom counter
      - usage computed for prompt/completion/total
    - Streaming:
      - yields string chunks of `chunk_size` characters
      - sleeps 0.01s between chunks
    - `reset()` clears counts/history and resets response index.

- [ ] **MockSTTProvider**
  - **Acceptance criteria**
    - Deterministic mapping:
      - hashes audio via `md5(audio).hexdigest()[:16]` for lookup in `audio_map`
      - if not mapped, cycles through `transcriptions`
    - Failure simulation:
      - with probability `fail_rate`, raises `Exception("Mock STT error")`
    - Confidence:
      - if `simulate_confidence=True`, random uniform in [0.85, 0.99]
      - else 0.95
    - Duration estimate:
      - `(len(audio) / (16000*2)) * 1000`
    - `reset()` resets call count and cycle index.

- [ ] **MockTTSProvider**
  - **Acceptance criteria**
    - Deterministic audio bytes based on md5 seed; size = `len(text) * bytes_per_char`.
    - Failure simulation:
      - with probability `fail_rate`, raises `Exception("Mock TTS error")`
    - Streaming:
      - chunk size derived from `chunk_duration_ms` using `bytes_per_ms = sample_rate*2/1000`
      - yields `MockAudioChunk` with `data` and `sample_rate`
      - sleeps `chunk_duration_ms/1000` between chunks

- [ ] **MockAuthProvider**
  - **Acceptance criteria**
    - `validate(token)`:
      - increments `validation_count` and records history with token truncated to 20 chars + `...` when needed
      - with probability `fail_rate`, raises `ValueError("Mock auth failure")`
      - if token is known:
        - checks expiry and raises `ValueError("Token expired")` if expired
        - otherwise returns claims
      - if `accept_any=True`, returns `default_claims`
      - else raises `ValueError("Invalid token")`
    - `create_token(...)` returns `(token_string, claims)` and stores in valid_tokens.

- [ ] **MockToolExecutor**
  - **Acceptance criteria**
    - Tracks `execution_count` and `execution_history` entries with `{tool, arguments, timestamp}`.
    - Simulates latency via `await sleep(latency_ms/1000)`.
    - Failure simulation:
      - with probability `fail_rate`, returns `success=false`, error `"Mock tool execution failed"`
    - Unknown tool returns `default_output` (default `{status:"ok"}`).
    - Exceptions in tool handler return `success=false` and `error=str(exc)`.
    - `register_tool` supports dynamic registration; `reset()` clears counters/history.

### 9.6 Compression utilities (`stageflow/compression/__init__.py`)

- [ ] **Shallow dict delta format**
  - **Acceptance criteria**
    - `compute_delta(base, current)`:
      - `delta["set"]` contains keys that are new or value-changed (`base[key] != value`)
      - `delta["remove"]` contains keys present in base but absent in current
      - omits `set` and/or `remove` keys when empty
    - `apply_delta(base, delta)`:
      - begins from `dict(base)`
      - removes keys listed in `remove` (missing removals are ignored)
      - applies `set` assignments
      - roundtrips: `apply_delta(base, compute_delta(base,current)) == current`

- [ ] **CompressionMetrics + byte estimation**
  - **Acceptance criteria**
    - `compress(base,current)` returns `(delta, metrics)` where:
      - `metrics.original_bytes` is UTF-8 byte length of JSON dump of `current`
      - `metrics.delta_bytes` is UTF-8 byte length of JSON dump of `delta`
    - For JSON-unsafe values:
      - falls back to `_json_safe` converting unknown types to `str`, recursively for lists/dicts
    - Metrics:
      - `reduction_bytes = max(original_bytes - delta_bytes, 0)`
      - `ratio = delta_bytes/original_bytes` (or 1.0 when original_bytes==0)

### 9.7 Runtime helpers (uuid collision + memory tracking + runner)

- [ ] **MemoryTracker + track_memory decorator** (`stageflow/helpers/memory_tracker.py`)
  - **Acceptance criteria**
    - Uses `tracemalloc` only (no psutil dependency).
    - `MemoryTracker(auto_start=True)` starts tracing in `__post_init__`.
    - `observe(label)`:
      - requires tracker active else raises `RuntimeError`
      - records `MemorySample(timestamp=datetime.now(UTC), current_kb, peak_kb, label)`
      - appends to `samples` and notifies listeners
    - `track_memory(label, tracker?)`:
      - wraps sync and async functions
      - emits `label:start` and `label:end` observations

- [ ] **UuidCollisionMonitor** (`stageflow/helpers/uuid_utils.py`)
  - **Acceptance criteria**
    - Sliding window defined by:
      - `ttl_seconds` (min 1.0s)
      - `max_entries` hard cap
    - `observe(uuid)`:
      - returns `True` if UUID string already present in window else `False`
      - appends entry and trims expired/excess entries
      - notifies listeners with `UuidEvent(value, collision, category, observed_at, skew_ms?)`
    - Optional UUIDv7 skew detection exists; when enabled and skew exceeds threshold, logs warning.

- [ ] **PipelineRunner utilities** (`stageflow/helpers/run_utils.py`)
  - **Acceptance criteria**
    - `PipelineRunner.create_snapshot(...)`:
      - generates missing IDs with `uuid4()`
      - sets `metadata={"channel": channel}`
    - `PipelineRunner.run(...)`:
      - always installs an `ObservableEventSink` via global `set_event_sink`
      - observes pipeline_run_id via uuid monitor when enabled
      - emits memory tracker labels `pipeline:start` and `pipeline:end` (or `pipeline:cancelled` / `pipeline:error`)
      - cancellation:
        - catches unified cancellation exception and returns `RunResult(success=True, cancelled=True, cancel_reason=...)`
    - `RunResult` includes:
      - `success`, `stages`, `duration_ms`, optional error fields, cancellation fields, events, and pipeline_run_id
      - `to_dict()` stringifies pipeline_run_id

### 9.8 Global event sink system (`stageflow/events/*`)

- [ ] **EventSink protocol + global sink context**
  - **Acceptance criteria**
    - `EventSink` protocol supports:
      - `async emit(type, data)`
      - `try_emit(type, data)`
    - Global sink is stored in a context variable (task-local inheritance).
    - `set_event_sink(sink)` sets current sink.
    - `clear_event_sink()` resets to None.
    - `get_event_sink()` returns:
      - current sink when set
      - otherwise returns a new `NoOpEventSink` instance.

- [ ] **NoOpEventSink**
  - **Acceptance criteria**
    - `emit` and `try_emit` are total no-ops.
    - Both ignore parameters and never raise (including when `type=None`, `data=None`).
    - `emit` returns `None`.

- [ ] **LoggingEventSink**
  - **Acceptance criteria**
    - Constructor supports `level` (default `INFO`).
    - `emit` and `try_emit` log with:
      - message template `"Event: %s"`
      - `extra={"event_type": type, "event_data": data}`
    - Works when `data=None`.

- [ ] **BackpressureMetrics**
  - **Acceptance criteria**
    - Fields:
      - `emitted`, `dropped`, `queue_full_count`, `last_emit_time`, `last_drop_time`
    - `record_emit()` increments emitted and sets `last_emit_time=time.monotonic()`.
    - `record_drop()` increments dropped and queue_full_count and sets `last_drop_time=time.monotonic()`.
    - `drop_rate` is percent: `(dropped/(emitted+dropped))*100` (0 when total==0).
    - `to_dict()` includes `drop_rate_percent` rounded to 2 decimals.

- [ ] **BackpressureAwareEventSink**
  - **Acceptance criteria**
    - Wraps a downstream sink (default: `LoggingEventSink`).
    - Uses bounded `asyncio.Queue` with `max_queue_size`.
    - `start()`:
      - idempotent
      - spawns background worker task.
    - `emit(...)`:
      - auto-starts worker if not running
      - blocks on queue put
      - records emit metrics
    - `try_emit(...)`:
      - returns `True` if queued, `False` if dropped due to `QueueFull`
      - auto-starts via `asyncio.create_task(start())` when not running
      - on drop:
        - records drop metrics
        - logs warning including event_type, queue_size, dropped_total
        - calls optional `on_drop(event_type, data)` callback
    - Worker behavior:
      - pulls events with `wait_for(queue.get(), timeout=0.1)`
      - calls downstream `emit`
      - downstream errors are logged but do not crash the worker
    - `stop(drain=True, timeout=5.0)`:
      - idempotent
      - if drain and queue not empty, drains by emitting remaining items; wait bounded by timeout
      - cancels worker task and suppresses CancelledError
    - Exposes:
      - `metrics`, `queue_size`, `is_running`.

- [ ] **wait_for_event_sink_tasks**
  - **Acceptance criteria**
    - If there are pending tracked tasks, awaits them via `asyncio.gather(..., return_exceptions=True)` and clears them from the set.
    - If none pending, returns quickly.

### 9.9 Timestamp parsing utilities (`stageflow/helpers/timestamps.py`)

- [ ] **detect_unix_precision**
  - **Acceptance criteria**
    - Uses digit count of the integer part of `abs(timestamp)`:
      - <=10 digits => `seconds`
      - <=13 digits => `milliseconds`
      - <=16 digits => `microseconds`
      - otherwise raises `ValueError` (nanoseconds unsupported)

- [ ] **normalize_to_utc**
  - **Acceptance criteria**
    - If datetime is naive:
      - if `default_timezone is None`, returns as-is
      - else assigns tzinfo=default_timezone
    - If `default_timezone is None`, returns dt unchanged (does not force UTC)
    - Otherwise converts to UTC via `astimezone(UTC)`

- [ ] **parse_timestamp**
  - **Acceptance criteria**
    - Accepts `str | int | float` else raises `TypeError`.
    - Numeric inputs:
      - parse as unix timestamp using detected precision unless float with fractional part (treated as seconds)
    - String inputs:
      - trims; empty => `ValueError`
      - if numeric string => parse as unix timestamp
      - else tries RFC 2822 via `email.utils.parsedate_to_datetime`
      - else tries ISO 8601 via `datetime.fromisoformat` with `Z` => `+00:00`
      - else tries a fixed set of human-readable formats (e.g. `October 5, 2023`)
      - else raises `ValueError`
    - Returns timezone-aware UTC datetime by default.

### 9.10 Provider response types (`stageflow/helpers/providers.py`)

- [ ] **LLMResponse**
  - **Acceptance criteria**
    - Frozen dataclass (immutable) with slots.
    - Fields:
      - required: `content`, `model`, `provider`
      - optional: `input_tokens`, `output_tokens`, `latency_ms`, `finish_reason`, `tool_calls`, `cached_tokens`, `raw_response`
    - `total_tokens = input_tokens + output_tokens`.
    - `to_dict()` includes token counts, latency, finish_reason, tool_calls, cached_tokens (raw_response excluded).
    - `to_otel_attributes()` exports keys:
      - `llm.model`, `llm.provider`, `llm.input_tokens`, `llm.output_tokens`, `llm.total_tokens`, `llm.latency_ms`, `llm.finish_reason`, `llm.cached_tokens`

- [ ] **STTResponse**
  - **Acceptance criteria**
    - Frozen dataclass (immutable) with slots.
    - Defaults:
      - `confidence=1.0`, `language="en"`, `is_final=True`
    - Fields include `duration_ms`, `provider`, `model`, `latency_ms`, `words`, `raw_response`.
    - `to_dict()` includes all fields except raw_response.
    - `to_otel_attributes()` exports keys:
      - `stt.provider`, `stt.model`, `stt.confidence`, `stt.duration_ms`, `stt.language`, `stt.latency_ms`, `stt.is_final`

- [ ] **TTSResponse**
  - **Acceptance criteria**
    - Frozen dataclass (immutable) with slots.
    - `byte_count = len(audio)`.
    - `to_dict()` excludes raw audio bytes and includes:
      - `byte_count`, `duration_ms`, `sample_rate`, `format`, `provider`, `model`, `latency_ms`, `channels`, `characters_processed`
    - `to_otel_attributes()` exports keys:
      - `tts.provider`, `tts.model`, `tts.duration_ms`, `tts.sample_rate`, `tts.format`, `tts.latency_ms`, `tts.byte_count`, `tts.characters_processed`

### 9.11 Websearch helpers (`stageflow/websearch/*`)

- [ ] **Data models (`stageflow/websearch/models.py`)**
  - **Acceptance criteria**
    - `PageMetadata`
      - Frozen dataclass (immutable) with slots.
      - Defaults:
        - `title=None`, `description=None`, `language=None`, `author=None`, `published_date=None`, `canonical_url=None`, `og_image=None`, `content_type=None`, `keywords=[]`
      - `to_dict()` includes all fields.
      - `from_dict()`:
        - missing keys default to dataclass defaults
        - `keywords` defaults to `[]` when missing
      - Roundtrip `from_dict(to_dict(x)) == x`.
    - `ExtractedLink`
      - Frozen dataclass (immutable) with slots.
      - Defaults:
        - `text=""`, `title=None`, `rel=None`, `is_internal=False`, `context=None`
      - `from_element(href, text, base_url=None, title=None, rel=None, context=None)`:
        - if `base_url` is set and `href` does not start with `http://`, `https://`, `//`, then resolves via `urljoin(base_url, href)`
        - if `href` starts with `//`, prefixes with `"https:"`
        - `is_internal` is `True` iff `urlparse(base_url).netloc == urlparse(resolved_url).netloc`
        - `text` is `.strip()`
      - `to_dict()` and `from_dict()` preserve all fields.
    - `NavigationAction`
      - Frozen dataclass (immutable) with slots.
      - Required: `action_type`, `label`
      - Defaults: `url=None`, `selector=None`, `priority=5`, `metadata={}`
      - `to_dict()` and `from_dict()` preserve all fields.
    - `PaginationInfo`
      - Frozen dataclass (immutable) with slots.
      - Defaults: `current_page=1`, `total_pages=None`, `next_url=None`, `prev_url=None`, `page_urls=[]`
      - `has_next` iff `next_url is not None`; `has_prev` iff `prev_url is not None`.
      - `to_dict()` and `from_dict()` preserve all fields.
    - `WebPage`
      - Frozen dataclass (immutable) with slots.
      - Defaults:
        - `final_url=None`, `status_code=0`, `markdown=""`, `plain_text=""`, `metadata=PageMetadata()`, `links=[]`, `navigation_actions=[]`, `pagination=None`, `fetch_duration_ms=0.0`, `extract_duration_ms=0.0`, `fetched_at=None`, `word_count=0`, `error=None`
      - `success` is `True` iff `error is None` and `200 <= status_code < 400`.
      - `title` returns `metadata.title`; `description` returns `metadata.description`.
      - `internal_links` filters links where `is_internal=True`; `external_links` filters where `is_internal=False`.
      - `extract_links(selector=None, internal_only=False, external_only=False, limit=None)`:
        - ignores `selector` (API compatibility)
        - applies internal/external filters
        - if `limit` set, returns prefix `links[:limit]`
      - `to_dict()` includes:
        - serialized `metadata`, `links`, `navigation_actions`, and `pagination` (or `None`)
      - `from_dict()`:
        - rebuilds `metadata`, `links`, `navigation_actions`, `pagination`
        - missing keys default to the same defaults as constructor
      - `error_result(url, error, duration_ms=0.0)`:
        - returns `WebPage` with `status_code=0`, `error=error`, `fetch_duration_ms=duration_ms`, `fetched_at=now(UTC).isoformat()`
      - `truncate(max_chars=10000)`:
        - if `len(markdown) <= max_chars`, returns `self` (same instance semantics)
        - otherwise returns a new page with:
          - `markdown` truncated to `max_chars` (optionally trimmed to last paragraph break `\n\n` if past half) and appends `"\n\n[Content truncated...]"`
          - `plain_text` truncated to `max_chars` (optionally trimmed to last sentence boundary `. ` / `! ` / `? ` if past half)
          - all other fields preserved

- [ ] **Fetcher layer (`stageflow/websearch/fetcher.py`)**
  - **Acceptance criteria**
    - Module-level constant `HTTPX_AVAILABLE` reflects import availability of `httpx`.
    - `FetchConfig`
      - Frozen dataclass (immutable) with slots.
      - Defaults:
        - `timeout=30.0`, `max_retries=3`, `retry_delay=1.0`, `follow_redirects=True`, `max_redirects=10`, `verify_ssl=True`, `max_content_length=10_000_000`
        - `user_agent` default is StageflowBot UA string
        - `default_headers` defaults to `{}`
    - `FetchResult`
      - Mutable dataclass with slots.
      - Defaults: `status_code=0`, `content=b""`, `content_type=""`, `headers={}`, `duration_ms=0.0`, `retry_count=0`, `error=None`, `final_url=None`
      - `request_id` defaults to `str(uuid4())`.
      - `success` is `True` iff `error is None` and `200 <= status_code < 400`.
      - `text` decoding:
        - if content empty, returns empty string
        - if `content_type` includes `charset=...`, use that encoding
        - otherwise default `utf-8`
        - on `UnicodeDecodeError`/unknown codec, decode with `utf-8` and `errors="replace"`
      - `is_html` is `True` iff content-type contains `text/html` or `application/xhtml`.
      - `to_dict()` excludes raw bytes, includes `content_length=len(content)` and includes `request_id`.
    - `Fetcher`
      - `fetch(url, timeout=None, headers=None, follow_redirects=None)`:
        - assigns a new `request_id=str(uuid4())` and emits `on_fetch_start(url, request_id)` (exceptions suppressed)
        - merges headers:
          - `{"User-Agent": config.user_agent, **config.default_headers, **headers}`
        - uses `effective_timeout = timeout or config.timeout`
        - uses `effective_follow = follow_redirects if not None else config.follow_redirects`
        - retry loop attempts `range(config.max_retries + 1)`
        - on each attempt:
          - awaits `_do_fetch(url, timeout=effective_timeout, headers=merged_headers, follow_redirects=effective_follow)`
          - then overwrites `result.request_id = request_id` and `result.retry_count = retry_count`
          - sets `result.duration_ms` from `time.perf_counter()` delta
          - emits `on_fetch_complete(url, request_id, status, duration_ms, size, from_cache=False)` if `result.success`
          - otherwise emits `on_fetch_error(url, request_id, error_string, duration_ms, retryable=(status_code>=500))`
          - and returns the `result` immediately (no retry-on-status-code behavior)
        - on exception:
          - determines retryable via `_is_retryable_error()` (substring match on lowercased exception string)
          - if retryable and attempts remain, sleeps `config.retry_delay * (2**attempt)` and continues
          - else breaks and returns `FetchResult(url=url, error=last_error or "Unknown error", duration_ms=..., retry_count=retry_count, request_id=request_id)` after emitting `on_fetch_error(..., retryable=False)`
      - `_is_retryable_error()` matches patterns:
        - `timeout`, `connection`, `reset`, `refused`, `temporary`, `503`, `502`, `504`, `429`
      - `fetch_many(urls, concurrency=5, timeout=None, headers=None)`:
        - returns `[]` for empty input
        - uses `asyncio.Semaphore(concurrency)` and `asyncio.gather` to preserve input order
    - `HttpFetcher`
      - Raises `ImportError` if `HTTPX_AVAILABLE` is `False`.
      - Supports async context manager:
        - `__aenter__` ensures client created
        - `__aexit__` calls `close()`
      - Lazily creates `httpx.AsyncClient` with:
        - `timeout=httpx.Timeout(config.timeout)`, `follow_redirects=config.follow_redirects`, `max_redirects=config.max_redirects`, `verify=config.verify_ssl`
      - `_do_fetch` performs `client.get(url, ...)`, sets `final_url=str(response.url)` and `content_type` from headers.
      - Enforces `max_content_length` truncation if `>0`.
    - `MockFetcher`
      - Accepts `responses: {url: (status_code, content(str|bytes), headers)}` and optional `default_response`.
      - Default response when missing:
        - `(404, "Not Found", {"content-type": "text/plain"})`
      - Records `fetch_history` in call order.
      - `_do_fetch` ignores `timeout/headers/follow_redirects`, returns `FetchResult` with `final_url=url`.
      - If response content is `str`, encodes as UTF-8 bytes.

- [ ] **Extractor layer (`stageflow/websearch/extractor.py`)**
  - **Acceptance criteria**
    - Module-level constant `SELECTOLAX_AVAILABLE` reflects import availability of `selectolax`.
    - `ExtractionConfig`
      - Frozen dataclass (immutable) with slots.
      - Defaults:
        - preserve flags all `True` (`preserve_headings`, `preserve_lists`, `preserve_links`, `preserve_emphasis`, `preserve_code`, `preserve_blockquotes`, `preserve_tables`)
        - `max_link_text_length=100`, `max_heading_length=200`, `include_link_urls=True`, `min_text_length=1`
        - `remove_selectors` includes `script`, `style`, `noscript`, `iframe`, `svg`, `nav`, `footer`, `header`, `aside` and various ad/sidebar/cookie selectors
        - `main_content_selectors` contains `article`, `main`, `[role="main"]`, and common content containers
    - `ExtractionResult`
      - Mutable dataclass with slots.
      - Defaults: `markdown=""`, `plain_text=""`, `metadata=PageMetadata()`, `links=[]`, `word_count=0`, `heading_outline=[]`
      - `to_dict()` serializes `metadata` via `to_dict()` and links via `to_dict()`.
    - `DefaultContentExtractor`
      - Raises `ImportError` if `SELECTOLAX_AVAILABLE` is `False`.
      - `extract(html, base_url=None, selector=None)`:
        - decompose nodes matching all `remove_selectors`
        - chooses content node:
          - if `selector` provided and matches, use first match
          - else first match from `main_content_selectors`
          - else falls back to `tree.body` else `tree.root`
        - extracts metadata from full document:
          - `og:title` overrides `<title>`
          - `og:description` overrides `meta[name=description]`
          - `language` from `<html lang=...>`
          - `author` from meta `author`
          - `published_date` from meta `article:published_time`
          - `canonical_url` from `link[rel="canonical"]`
          - `og_image` from `og:image`
          - `keywords` from comma-splitting meta `keywords`
          - `content_type` from `og:type`
        - extracts links from the content node:
          - skips `href` starting with `#`, `javascript:`, `mailto:`, `tel:`
          - resolves relative URLs using `ExtractedLink.from_element` with `base_url`
          - de-duplicates by final URL
        - builds markdown/plain_text preserving structure:
          - headings map to `#`..`######` with `heading_outline` tracking
          - lists preserve `-` and numbered `1.` format (with indentation)
          - links become `[text](url)` when `include_link_urls=True`
          - bold `**text**`, italic `_text_`, inline code `` `code` ``, fenced code blocks using language from `class="language-..."`
          - blockquotes prefix `> `
          - tables converted to markdown with header separator row
          - `br` -> hard break (`"  \n"`), `hr` -> `---`
        - cleans markdown:
          - collapses `\n{3,}` to `\n\n`, collapses multiple spaces, unescapes HTML entities, trims
        - cleans plain text by collapsing whitespace and unescaping entities
        - `word_count=len(plain_text.split())`
      - `extract_metadata(html)` works on raw html without removal/selectors.
      - `extract_links(html, base_url=None, selector=None)`:
        - decompose nodes matching `remove_selectors`
        - uses `tree.body`/`tree.root` unless `selector` matched
    - `FallbackContentExtractor`
      - Selector is not supported; `extract(..., selector=...)` ignores selector.
      - Removes scripts/styles/comments via regex.
      - Extracts headings/paragraphs via regex; decodes HTML entities.
      - `extract_links` skips `#`/`javascript:`/`mailto:` and resolves relative URLs when `base_url` provided.
      - `extract_metadata` extracts `<title>` and meta description via regex.
    - `get_default_extractor(config=None)`:
      - returns `DefaultContentExtractor` if selectolax available, else `FallbackContentExtractor`.

- [ ] **Navigator layer (`stageflow/websearch/navigator.py`)**
  - **Acceptance criteria**
    - Module-level constant `SELECTOLAX_AVAILABLE` reflects import availability of `selectolax`.
    - `NavigationConfig`
      - Frozen dataclass (immutable) with slots.
      - Defaults:
        - `pagination_selectors` contains `.pagination` and other common selectors
        - `pagination_link_patterns` includes patterns like `page=\d+` and `offset=\d+`
        - `next_link_texts` includes `next`, arrows, `older`, `more`
        - `prev_link_texts` includes `prev`, `previous`, arrows, `newer`, `back`
        - `nav_link_selectors` contains `nav a`, `.menu a`, and role-based selectors
        - `content_selectors` contains `article`, `main`, `[role="main"]`, `#content`, `.content`, etc.
        - `min_nav_links=3`, `max_actions=20`
    - `NavigationResult`
      - Mutable dataclass with slots.
      - Defaults: `actions=[]`, `pagination=None`, `main_content_selector=None`, `nav_links=[]`, `breadcrumbs=[]`
      - `to_dict()` serializes `actions`, `pagination`, `nav_links`, `breadcrumbs`.
    - `PageNavigator`
      - Raises `ImportError` if `SELECTOLAX_AVAILABLE` is `False`.
      - `analyze(html, base_url=None)`:
        - extracts `pagination`, `main_content_selector`, `nav_links`, `breadcrumbs`
        - builds `actions` with priority ordering:
          - pagination next => `priority=1`, label `"Next page"`, metadata `{"direction":"next"}`
          - pagination prev => `priority=2`, label `"Previous page"`, metadata `{"direction":"prev"}`
          - nav links => `action_type="nav_link"`, `priority=3`
          - content links => `action_type="content_link"`, `priority=4`
        - respects `max_actions` cap
        - does not add duplicate action URLs (content links skipped when URL already present)
      - Pagination detection:
        - first checks `pagination_selectors`; if found parses container:
          - recognizes next/prev by link text tokens or class contains `next`/`prev`
          - collects other link hrefs as `page_urls`
          - extracts `current_page` from `.current, .active, [aria-current='page']` if digit
          - sets `total_pages=len(page_urls)+1` when page_urls non-empty else `None`
        - if no container, falls back to scanning all `a[href]` for URL pattern matches; if >=2 links:
          - sorts by extracted page number (missing => 0)
          - returns `PaginationInfo(current_page=1, page_urls=[...], next_url=first_sorted_url)`
      - Main content selector detection:
        - returns first `content_selectors` match with `text_len > 200`
        - else searches `div[class]`/`div[id]` candidates with keywords, choosing max text length and returns `".<class>"` or `"#<id>"`
      - Nav links extracted from `nav_link_selectors` and de-duplicated by URL; skips `#` and `javascript:`.
      - Breadcrumbs extracted from a fixed selector list and stops after first selector yielding any links.
      - Content links extracted from detected main content node (or body/root), skipping `#`/`javascript:`/`mailto:`/`tel:` and skipping link texts shorter than 3 chars.
    - `FallbackNavigator`
      - Uses regex to find anchors, identifies next/prev via text tokens and class string.
      - Produces `PaginationInfo(current_page=1, next_url=?, prev_url=?)` when found.
      - Produces pagination actions with priorities 1/2.
      - Does not attempt main content selector, nav links, or breadcrumbs (all empty/None).
    - `get_default_navigator(config=None)`:
      - returns `PageNavigator` if selectolax available, else `FallbackNavigator`.

- [ ] **Client layer (`stageflow/websearch/client.py`)**
  - **Acceptance criteria**
    - `WebSearchConfig`
      - Frozen dataclass (immutable) with slots.
      - Defaults:
        - `max_concurrent=5`, `auto_extract=True`, `auto_navigate=True`
        - `fetch=FetchConfig()`, `extraction=ExtractionConfig()`, `navigation=NavigationConfig()`
    - `WebSearchClient.__init__` wiring
      - If `fetcher` provided, client does not own it and does not auto-close it.
      - Else:
        - if `HTTPX_AVAILABLE`, uses `HttpFetcher(config.fetch, on_fetch_start, on_fetch_complete, on_fetch_error)` and marks it owned
        - otherwise raises `ImportError("httpx is required for WebSearchClient...")`
      - Chooses extractor:
        - provided `extractor` else `DefaultContentExtractor(config.extraction)` when selectolax available else `FallbackContentExtractor(config.extraction)`
      - Chooses navigator:
        - provided `navigator` else `PageNavigator(config.navigation)` when selectolax available else `FallbackNavigator(config.navigation)`
    - Async context management:
      - `__aenter__` delegates to fetcher `__aenter__` if present
      - `close()` closes owned fetcher if it has `close()`
      - `__aexit__` calls `close()`
    - `fetch(url, timeout=None, headers=None, selector=None, extract=None, navigate=None)`:
      - generates a local `request_id=str(uuid4())` used only for `on_extract_complete`
      - `should_extract` defaults from `config.auto_extract` unless overridden
      - `should_navigate` defaults from `config.auto_navigate` unless overridden
      - calls `fetcher.fetch(url, timeout=..., headers=...)`
      - on fetch failure returns `WebPage.error_result(url, error_or_http_status, fetch_duration_ms)`
      - on non-HTML fetch result:
        - returns `WebPage` with `markdown/plain_text = fetch_result.text[:10000]`, `word_count=len(text.split())`, `fetched_at=now(UTC).isoformat()`
      - on HTML:
        - if extracting:
          - uses extractor `extract(html, base_url=final_url_or_url, selector=selector)`
        - else:
          - `markdown=""`, `plain_text=""`, `links=[]`, `word_count=0`, but still extracts metadata via `extract_metadata(html)`
        - if navigating:
          - uses navigator `analyze(html, base_url=final_url_or_url)` and attaches actions + pagination
        - computes `extract_duration_ms` from perf counter around extraction/navigation section
        - calls optional `on_extract_complete(url, request_id, extract_duration_ms, len(markdown), len(links))` with exceptions suppressed
    - `fetch_many(urls, concurrency=None, ...)`:
      - returns `[]` for empty input
      - uses `asyncio.Semaphore(concurrency or config.max_concurrent)` and `asyncio.gather` to preserve order
    - `fetch_with_navigation(url, max_pages=10, follow_pagination=True, ...)`:
      - iteratively fetches pages, tracking `seen_urls` to avoid loops
      - stops when:
        - `len(pages) == max_pages`, or
        - next URL is `None`, or
        - `follow_pagination` is `False`, or
        - URL repeats
      - uses `page.pagination.next_url` to advance
    - `crawl(start_url, max_pages=10, max_depth=2, same_domain_only=True, link_filter=None, ...)`:
      - BFS-like loop over a list queue of `(url, depth)`
      - `seen_urls` prevents repeats
      - `start_domain=urlparse(start_url).netloc`
      - if `same_domain_only`, skips links whose `netloc` differs
      - if `link_filter` provided, only enqueues links where `link_filter(link)` is truthy
      - does not special-case failed pages when extracting links (enqueues based on `page.links`)
    - `extract_content(html, base_url=None, selector=None)`:
      - extracts with extractor + navigator without fetching
      - returns `WebPage(url=base_url or "", status_code=200, fetched_at=now(UTC).isoformat(), extract_duration_ms>0)`
    - `create_mock_client(responses=None, config=None)`:
      - constructs `MockFetcher(responses, config=config.fetch if config else None)` and returns `WebSearchClient(config, fetcher=mock_fetcher)`

- [ ] **Run utilities (`stageflow/websearch/run_utils.py`)**
  - **Acceptance criteria**
    - `FetchProgress`:
      - mutable dataclass with defaults: `completed=0`, `total=0`, `current_url=None`, `success_count=0`, `error_count=0`, `elapsed_ms=0.0`
      - `percent` is `(completed/total*100)` else `0.0`
    - `SearchResult`:
      - includes: `query`, `pages`, `relevant_pages`, `total_words`, `duration_ms`
      - `to_dict()` includes counts (`pages_fetched`, `relevant_pages`) and totals (not raw pages)
    - `SiteMap`:
      - includes: `start_url`, `pages`, `internal_links`, `external_links`, `depth_reached`, `duration_ms`
      - `to_dict()` includes counts (`pages_crawled`, internal/external counts) and totals
    - `fetch_page(url, ...)`:
      - uses internal async context manager that creates `WebSearchClient(config)` unless `client_factory` provided
    - `fetch_pages(urls, concurrency=5, ..., on_progress=None, parallel_extraction=True)`:
      - returns `[]` for empty input
      - results preserve input order
      - uses `asyncio.Semaphore(concurrency)`
      - maintains a single `FetchProgress(total=len(urls))` updated on each completion:
        - increments `completed`, increments `success_count` or `error_count`, updates `elapsed_ms` from wall-clock UTC time
        - calls `on_progress(progress)` if provided
      - if `parallel_extraction=True`:
        - uses `client._fetcher.fetch(...)` to fetch raw HTML
        - on success + HTML, runs extraction + navigation via thread pool executor (`max_workers=4`, thread_name_prefix=`extract`)
        - on success + non-HTML, returns `WebPage` with markdown/plain_text truncated to 10000 and `word_count=len(text.split())`
        - on failure, returns `WebPage.error_result(url, error_or_http_status, duration_ms)`
      - if `parallel_extraction=False`, delegates to `client.fetch(...)`
      - returns only non-None results (should be all slots filled)
    - `search_and_extract(start_url, query, ...)`:
      - crawls via `client.crawl(...)`
      - relevance scoring:
        - `query_terms=set(query.lower().split())`
        - `content=(title + plain_text).lower()`
        - `score = matches/len(query_terms)` (0 if query_terms empty)
        - includes pages where `score >= relevance_threshold`
      - sorts `relevant_pages` by number of matching terms desc
      - computes `total_words=sum(word_count)` across `relevant_pages`
      - `duration_ms` from UTC wall clock
    - `map_site(start_url, ...)`:
      - crawls via `client.crawl(...)` with optional `link_filter`
      - collects unique links across all pages by URL:
        - `internal_links` when `link.is_internal`
        - `external_links` when `include_external=True` and `not is_internal`
      - sets `depth_reached=max_depth` (requested depth, not observed depth)
      - `duration_ms` from UTC wall clock
    - `fetch_with_retry(url, max_retries=3, retry_delay=1.0, ...)`:
      - attempts `max_retries + 1` times
      - returns on first success
      - exponential backoff sleep: `retry_delay * (attempt + 1)`
      - on exhaustion returns `WebPage.error_result(url, f"Failed after {max_retries + 1} attempts: {last_error}", 0.0)`
    - `extract_all_links(urls, concurrency=10, internal_only=False, external_only=False)`:
      - fetches pages via `fetch_pages`
      - skips failed pages
      - de-duplicates by URL
      - enforces internal/external filters
    - `shutdown_extraction_pool()`:
      - calls executor `.shutdown(wait=True)` and sets global executor to `None`

- [ ] **Protocols (`stageflow/websearch/protocols.py`)**
  - **Acceptance criteria**
    - `FetcherProtocol`, `ContentExtractorProtocol`, `NavigatorProtocol`, `ObservabilityProtocol` are `@runtime_checkable` protocols defining the method signatures used by the websearch implementation.

---

## Phase 10 — Parity test suite and completeness gates

- [ ] **Golden payload fixtures (event-level JSON parity)**
  - **Acceptance criteria**
    - For each emitted event type covered by the Python test suite, Rust produces payloads that match a canonical JSON fixture after normalization:
      - stable key ordering (object keys sorted, lists sorted only where Python semantics sort)
      - stable types (`str`/`null` where Python uses `str(x)` or `None`)
      - stable timestamp formatting (ISO 8601; timezone handling matches Python)
      - stable `data_keys` sorting rules and other explicitly sorted fields
    - Fixture generation process is deterministic:
      - fixtures are derived from running the Python pipeline/tests with controlled clocks/UUIDs where necessary
      - any intentionally variable fields are explicitly normalized (or removed) in both Python and Rust harnesses

- [ ] **Behavioral parity matrix (Python tests -> Rust tests)**
  - **Acceptance criteria**
    - Every Python test module has a Rust analog (by module name or by explicit mapping) that asserts the same behavioral contract.
    - At minimum, include explicit mapping rows for:
      - `tests/unit/test_execution_context.py`
      - `tests/execution/test_dag.py`
      - `tests/unit/pipeline/test_builder.py`
      - `tests/pipeline/test_builder_helpers.py`
      - `tests/unit/pipeline/test_cancellation.py`
      - `tests/unit/test_advanced_tool_executor.py`
      - `tests/unit/tools/test_registry.py`
      - `tests/unit/test_tool_registry_parse.py`
      - `tests/unit/tools/test_approval.py`
      - `tests/unit/tools/test_errors.py`
      - `tests/unit/test_helpers_analytics.py`
      - `tests/unit/test_helpers_streaming.py`
      - `tests/unit/test_helpers_guardrails.py`
      - `tests/unit/test_helpers_memory.py`
      - `tests/unit/test_helpers_mocks.py`
      - `tests/unit/test_helpers_runtime.py`
      - `tests/unit/test_helpers_runtime_integrated.py`
      - `tests/unit/test_events.py`
      - `tests/unit/events/test_backpressure_sink.py`
      - `tests/unit/test_helpers_timestamps.py`
      - `tests/unit/test_helpers_providers.py`
      - `tests/websearch/test_models.py`
      - `tests/websearch/test_fetcher.py`
      - `tests/websearch/test_extractor.py`
      - `tests/websearch/test_navigator.py`
      - `tests/websearch/test_client.py`
      - `tests/websearch/test_run_utils.py`
      - `tests/websearch/test_exports.py`

- [ ] **Export-surface parity gates**
  - **Acceptance criteria**
    - Rust crate exposes the same conceptual “public surface” as Python exports:
      - core pipeline types, contexts, events/sinks, tools, helpers, and testing utilities
    - Any intentional deviations are captured as checklist exceptions with:
      - rationale
      - migration guidance
      - explicit acceptance tests proving equivalence at the behavior level

- [ ] **Determinism + concurrency invariants**
  - **Acceptance criteria**
    - Parity tests include at least one run that validates deterministic behavior for:
      - stable ordering of stage execution where DAG allows non-determinism (must match Python strategy)
      - stable event emission ordering for stage lifecycle and tool lifecycle
      - bounded-queue backpressure behavior (drop vs block semantics) matching Python sink(s)
    - No test is allowed to pass “by luck” due to timing; concurrency-sensitive tests must use synchronization points.

- [ ] **Porting completion definition**
  - **Acceptance criteria**
    - Every item in Phases 1–9 is checked.
    - Phase 10 gates are green:
      - golden fixtures match
      - parity matrix complete
      - export-surface parity validated
      - determinism/concurrency invariants validated
    - Rust test suite passes and covers the same edge cases described here.
