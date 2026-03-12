# cynic-muscle Design Spec

**Date:** 2026-03-12
**Status:** Approved
**Scope:** LLM orchestration service connecting existing CYNIC kernel abstractions into a working distributed inference pipeline.

---

## Problem Statement

The CYNIC kernel has all the scaffolding for LLM orchestration — `InferencePort` trait, `MuscleHAL` gRPC stub, `probe.rs` discovery, `DaemonSupervisor` — but nothing is wired together. `hal.rs` returns hardcoded strings and ignores `backend.rs` entirely. The goal is to connect what exists without creating spaghetti.

## Architecture

### Topology

- **forge** (i5-6500T, 11GB RAM, Linux): Runs `cynic-kernel` (gRPC on `[::1]:50051`), SurrealDB, is the brain/orchestrator
- **Windows APU** (Ryzen 7 5700G, 32GB shared RAM, Vulkan): Runs `llama-server` as GPU compute worker
- **Network**: Tailscale VPN between nodes, Tailscale MCP for discovery/exec/transfer

### Design Principle

**Monolith modulaire** — one Cargo workspace, two binary entry points:
- `cynic-kernel`: The brain (gRPC services, storage, orchestration)
- `cynic-muscle`: Optional separate deployment for remote GPU workers

Both share the same crate; deployment topology is configuration, not compilation.

---

## Crystallized Truths

These 8 falsifiable statements define the design. Each survived recursive descent and metathinking.

### T1: hal.rs Must Become BackendRouter

**Current state:** `hal.rs` returns `"Reflexive thought simulated."` and never touches `InferencePort` or `backend.rs`.

**Target:** `MuscleHalService` holds a `BackendRouter` wrapping `Vec<Box<dyn InferencePort>>`. The gRPC `RequestInference` handler delegates to the router, which selects a backend by capability match (model hint, VRAM, health).

**Pre-requisite:** `InferencePort` currently uses RPITIT (`impl Future` returns), which is **not object-safe** — `Box<dyn InferencePort>` won't compile. Fix: add `async-trait` to `Cargo.toml` and convert to `#[async_trait]` with `-> BoxFuture<'_, Result<...>>` returns. This changes `backend.rs` (see change matrix).

**Confidence:** 55% — The wiring is clear, but selection heuristics may need iteration.

### T2: LlamaCppBackend Is the Single Most Valuable Addition

A struct implementing `InferencePort` that calls llama.cpp's OpenAI-compatible HTTP API (`/v1/chat/completions`). This is the bridge between the kernel's abstraction layer and real inference.

```rust
pub struct LlamaCppBackend {
    endpoint: Url,           // e.g., http://100.x.y.z:11435
    client: reqwest::Client,
    capability: BackendCapability,
}

impl InferencePort for LlamaCppBackend {
    // capability() → returns stored capability
    // infer() → POST /v1/chat/completions, map response
    // health() → GET /health, map status
}
```

**Model-agnostic:** The backend discovers what model is loaded at runtime via `/v1/models`. It never hardcodes a model name. Hot-swap is supported via llama.cpp's `--swap` endpoint or kernel-initiated restart.

**Cold-start handling:** `LlamaCppBackend::new()` must poll `GET /health` with exponential backoff (1s, 2s, 4s, max 60s total) before querying `/v1/models`. llama-server takes 5-30s to load a model. During cold-start, `health()` returns `BackendStatus::Unreachable` and `BackendRouter` skips this backend.

**Confidence:** 58% — The HTTP bridge is straightforward; edge cases are in timeout/retry behavior.

### T3: MemoryGuard Prevents OOM on Shared-Memory APU

The Ryzen 7 5700G shares 32GB between CPU and GPU (Vulkan). DWM needs ~2GB. Loading large models without guard causes DWM crashes and system instability.

**Design:**
- Query available memory before model load
- Reserve 4GB floor (DWM + system)
- Expose memory pressure in `BackendStatus::Degraded` when >80% used
- Delayed start on Windows: wait for DWM initialization before loading model

**Linux behavior:** On Linux (forge), MemoryGuard still enforces the floor but `wait_for_dwm()` is a no-op. `available_mb()` uses `sysinfo::System::available_memory()` which accounts for cached/buffers correctly on both platforms.

**Confidence:** 45% — Memory contention on shared APU is empirical; thresholds need tuning.

### T4: One Workspace, Two Binaries

```toml
# Cargo.toml (workspace)
[workspace]
members = ["cynic-kernel"]

# cynic-kernel/Cargo.toml
[[bin]]
name = "cynic-kernel"
path = "src/main.rs"

[[bin]]
name = "cynic-muscle"
path = "src/muscle_main.rs"
```

`cynic-muscle` is a headless binary that:
1. Runs `probe.rs` to discover local hardware
2. Registers with the kernel's `VascularSystem` as a compute node
3. Starts `DaemonSupervisor` managing `llama-server` lifecycle
4. Exposes `MuscleHAL` gRPC service for the kernel to call

When a single machine is powerful enough, `cynic-kernel` spawns the muscle in-process: `muscle_main` logic is a library function `run_muscle(config) -> JoinHandle` called from `main.rs` via `tokio::spawn`. No thread boundaries — same tokio runtime. The binary entry point `muscle_main.rs` just calls this same function after parsing config.

**Confidence:** 52% — Clean separation, but the in-process vs. separate-binary decision needs runtime validation.

### T5: num_branches IS Multi-Orchestration

`MCTSInferenceRequest.num_branches` already exists in the proto. When `num_branches > 1`, the `BackendRouter` fans out to multiple backends (or the same backend with different temperatures), collects hypotheses, and returns them as the response vector.

This means multi-orchestration is not a new feature — it's wiring the existing proto field to actual fan-out logic.

**Fan-out semantics:**
- If `num_branches > available_backends`: dispatch `num_branches` requests across available backends (round-robin, same backend gets multiple requests with varied temperature)
- **Partial failure**: collect all successful results; if at least 1 succeeds, return partial results with `hypotheses.len() < num_branches`. If all fail, return `BackendError::AllBackendsFailed`
- **Latency**: wait-for-all with per-branch timeout (not global). Timed-out branches are excluded from results, not retried.

**Confidence:** 50% — Fan-out is conceptually clean, but latency aggregation (wait-for-all vs. first-N) needs design.

### T6: Every Inference Result Publishes to VascularSystem

The kernel's `VascularSystem` (event bus via gRPC streaming) must receive inference telemetry:
- Model used, latency, token count
- Backend health transitions
- Memory pressure events

This closes the metabolic loop: probe discovers → muscle infers → vascular observes → kernel decides.

**Confidence:** 48% — The event schema needs iteration; over-publishing risks noise.

### T7: probe.rs Already Discovers Everything

`probe.rs` (500+ LOC) already:
- Detects GPU vendor/API (CUDA/ROCm/Vulkan/Metal/CPU)
- Scans GGUF models across filesystem tiers
- Finds running inference servers
- Computes optimal llama-server flags
- Generates `SovereigntyAdvice`

**Do not re-discover.** The muscle binary calls `probe.rs` once at startup, caches `NodeConfig`, and passes it to `LlamaCppBackend` and `DaemonSupervisor`.

**Confidence:** 55% — probe.rs works; the gap is that its output is never consumed downstream.

### T8: Implementation Order

```
Step 0: Fix forge environment (Rust in PATH ✓, swap file, SurrealDB auth)
Step 0.5: Make InferencePort object-safe (async-trait + BoxFuture) + add model_hint to InferenceRequest
Step 1: LlamaCppBackend — impl InferencePort for llama.cpp HTTP (with cold-start polling)
Step 2: hal.rs rewrite — BackendRouter dispatching via InferencePort (Arc<RwLock<Vec<...>>>)
Step 2.5: Supervisor refactor — add shutdown channel (tokio::sync::watch) + re-spawn API
Step 3: Proto extensions — HALProfile enrichment, model_hint, SwapModel, HealthStream
Step 4: cynic-muscle binary — headless worker with supervisor + probe
Step 5: MemoryGuard — shared-memory APU protection (cross-platform)
Step 6: VascularSystem integration — fix fanout (store tx), inference telemetry publishing
Step 7: Windows Service — proper service wrapper for cynic-muscle
```

Each step is independently testable and deployable. No step depends on a later step.

**Confidence:** 52% — Order is sound, but steps 5-7 may reorder based on runtime findings.

---

## Proto Extensions

Additions to `cynic.proto` (backwards-compatible):

### HALProfile Enrichment
```protobuf
message HALProfile {
  MessageMeta meta = 1;
  string backend = 2;
  string gpu_name = 3;
  float vram_used_gb = 4;
  float vram_total_gb = 5;
  // New fields
  string loaded_model = 6;
  repeated string available_models = 7;
  float memory_pressure = 8;     // 0.0-1.0
  BackendHealthStatus health = 9;
}

enum BackendHealthStatus {
  HEALTHY = 0;
  DEGRADED = 1;
  UNREACHABLE = 2;
}
```

### MCTSInferenceRequest Addition
```protobuf
message MCTSInferenceRequest {
  MessageMeta meta = 1;
  string system_prompt = 2;
  string context = 3;
  int32 num_branches = 4;   // Keep int32 — matches existing proto, no type change
  float temperature = 5;
  // New field
  string model_hint = 6;    // Optional: preferred model, empty = router decides
}
```

**Note:** `num_branches` stays `int32` (not `uint32`) to maintain wire compatibility. Domain code clamps to `max(1, num_branches)` at the gRPC boundary.

### New RPCs on MuscleHAL
```protobuf
service MuscleHAL {
  rpc RequestInference(MCTSInferenceRequest) returns (MCTSInferenceResponse);
  rpc GetActiveHAL(PulseRequest) returns (HALProfile);
  // New
  rpc SwapModel(SwapModelRequest) returns (SwapModelResponse);
  rpc HealthStream(PulseRequest) returns (stream HALProfile);  // Push cadence: config.health_push_interval_s (default 5s), buffer 4, drop oldest on slow subscriber
}

message SwapModelRequest {
  MessageMeta meta = 1;
  string model_path = 2;
  uint32 gpu_layers = 3;    // 0 = auto from probe
  uint32 context_size = 4;  // 0 = default
}

message SwapModelResponse {
  MessageMeta meta = 1;
  bool success = 2;
  string error = 3;
  string loaded_model = 4;
  float load_time_ms = 5;
}
```

---

## Key Components

### 1. LlamaCppBackend (`backend_llamacpp.rs`)

```
InferencePort trait
    ├── capability() → BackendCapability { kind: Local, models, vram }
    ├── infer(req) → POST /v1/chat/completions → InferenceResponse
    └── health() → GET /health → BackendStatus
```

- Uses `reqwest` (already in Cargo.toml via tonic dependencies)
- Discovers loaded model via `GET /v1/models` on init
- Timeout: 120s default (configurable), with early-cancel on client disconnect
- Maps llama.cpp errors to `BackendError::InferenceFailed`

### 2. BackendRouter (`router.rs`)

```
BackendRouter
    ├── backends: Arc<RwLock<Vec<Arc<dyn InferencePort>>>>
    ├── inflight: Arc<AtomicUsize>  // active inference count for drain
    ├── route(req) → selects backend by:
    │   ├── model_hint match (if provided)
    │   ├── health filter (skip Unreachable/Degraded)
    │   ├── capability match (VRAM, context size)
    │   └── round-robin fallback (AtomicUsize counter)
    ├── fan_out(req, n) → parallel dispatch to n backends
    ├── register(backend) / deregister(id) → runtime add/remove
    └── drain() → wait for inflight to reach 0 (used before SwapModel)
```

**Thread-safety:** `Arc<RwLock<Vec<...>>>` — read lock for routing (hot path), write lock only for register/deregister (cold path). Individual backends are `Arc<dyn InferencePort>` so they can be shared across fan-out futures.

**SwapModel concurrency:** Before swapping, call `drain()` to wait for in-flight requests to complete (with 30s timeout). After timeout, remaining in-flight requests get `BackendError::BackendUnavailable`. This is a known limitation, not a silent failure.

### 3. MemoryGuard (`memory_guard.rs`)

```
MemoryGuard
    ├── available_mb() → system memory minus reserved floor
    ├── can_load(model_size_mb) → bool
    ├── pressure() → f32 (0.0-1.0)
    └── wait_for_dwm() → delays until desktop compositor stable (Windows only)
```

### 4. muscle_main.rs (cynic-muscle binary)

```
main()
    ├── probe::discover() → NodeConfig
    ├── MemoryGuard::new(floor: 4096MB)
    ├── DaemonSupervisor::new(llama-server, flags from NodeConfig)
    ├── LlamaCppBackend::new(endpoint, capability from probe)
    ├── MuscleHalService::new(BackendRouter::new(vec![backend]))
    └── tonic::Server → serve MuscleHAL on configured port
```

---

## Existing Code: What Changes, What Doesn't

| File | Action | Rationale |
|------|--------|-----------|
| `backend.rs` | **Modify**: make object-safe + add `model_hint` | RPITIT `impl Future` is not object-safe; `BackendRouter` needs `Vec<Box<dyn InferencePort>>` |
| `probe.rs` | **No change** | Discovery works, output just needs consumers |
| `supervisor.rs` | **Moderate refactor**: add shutdown channel + re-spawn API | Current `supervise()` is a blocking `loop{}` with no cancellation — `SwapModel` needs kill→restart with new args |
| `hal.rs` | **Rewrite** | Replace stub with BackendRouter dispatch |
| `main.rs` | **Extend**: wire probe→hal, add config, signal handler | Boot sequence needs completion |
| `pulse.rs` | **Extend**: use sysinfo for real metrics | Hardcoded values → real telemetry |
| `storage.rs` | **Fix**: SurrealDB auth credentials | Test uses wrong password |
| `cynic.proto` | **Extend**: HALProfile fields, SwapModel, HealthStream | Backwards-compatible additions |

### New Files

| File | Purpose |
|------|---------|
| `backend_llamacpp.rs` | LlamaCppBackend impl InferencePort |
| `router.rs` | BackendRouter selection logic |
| `memory_guard.rs` | Shared-memory APU protection |
| `muscle_main.rs` | cynic-muscle binary entry point |
| `config.rs` | Runtime configuration (endpoints, ports, thresholds) |

---

## Runtime Configuration

No hardcoded values. Configuration via environment variables with sane defaults.

### config.rs Shape

```rust
pub struct KernelConfig {
    pub grpc_addr: SocketAddr,        // CYNIC_GRPC_ADDR, default [::1]:50051
    pub surrealdb_url: String,        // CYNIC_SURREALDB_URL, default ws://localhost:8000
    pub surrealdb_user: String,       // CYNIC_SURREALDB_USER, default "root"
    pub surrealdb_pass: String,       // CYNIC_SURREALDB_PASS, required (no default)
    pub inline_muscle: bool,          // CYNIC_INLINE_MUSCLE, default false
}

pub struct MuscleConfig {
    pub llama_endpoint: Url,          // CYNIC_LLAMA_ENDPOINT, default http://127.0.0.1:11435
    pub memory_floor_mb: u64,         // CYNIC_MEMORY_FLOOR_MB, default 4096
    pub inference_timeout_s: u64,     // CYNIC_INFERENCE_TIMEOUT_S, default 120
    pub gpu_layers: u32,              // CYNIC_GPU_LAYERS, 0 = auto from probe
    pub grpc_addr: SocketAddr,        // CYNIC_MUSCLE_ADDR, default [::1]:50052
    pub kernel_addr: String,          // CYNIC_KERNEL_ADDR, default http://[::1]:50051
    pub health_push_interval_s: u64,  // CYNIC_HEALTH_INTERVAL_S, default 5
}
```

Both configs are parsed from env vars at startup via a `Config::from_env()` method. No config files, no TOML — env vars only for 12-factor compliance and systemd `Environment=` compatibility.

---

## Testing Strategy

- **Unit**: MockBackend (already exists in `backend.rs`) for router logic, fan-out, selection
- **Integration**: LlamaCppBackend against real llama-server (requires running instance)
- **Contract**: Proto compatibility verified by compiling both old and new clients
- **Memory**: MemoryGuard tested with simulated pressure (mock sysinfo)

---

## Risks and Mitigations

| Risk | Probability | Mitigation |
|------|-------------|------------|
| llama.cpp API changes | Low | Pin to known-good version, abstract behind InferencePort |
| OOM on shared APU | Medium | MemoryGuard + conservative defaults |
| Cross-compile failures | Low | Validated: forge→Windows in 0.66s via `x86_64-pc-windows-gnu` |
| DaemonSupervisor restart loops | Medium | Max restarts cap (already in supervisor.rs) + exponential backoff |
| Proto breaking changes | Low | All additions are new fields/RPCs, no modifications |

---

## Success Criteria

1. `cargo test` passes on forge with MockBackend router tests
2. `LlamaCppBackend::infer()` returns real completions from llama-server over Tailscale
3. `hal.rs` dispatches via BackendRouter instead of hardcoded response
4. `num_branches > 1` fans out and returns multiple hypotheses
5. `cynic-muscle` binary starts, discovers hardware via probe, manages llama-server lifecycle
6. No hardcoded model names anywhere in the build

---

## Spec Review Findings (Addressed)

Issues found by automated spec review and resolved in this revision:

| # | Issue | Resolution |
|---|-------|------------|
| 1 | `InferencePort` uses RPITIT — not object-safe for `Box<dyn>` | Add `async-trait`, convert to `BoxFuture`. Step 0.5 added. |
| 2 | `model_hint` in proto but not in `InferenceRequest` domain struct | `backend.rs` now marked as "Modify", `model_hint: Option<String>` added |
| 3 | `DaemonSupervisor` has no cancellation — can't implement `SwapModel` | Marked as "Moderate refactor", Step 2.5 added for shutdown channel |
| 4 | `BackendRouter` thread-safety unspecified | `Arc<RwLock<Vec<Arc<dyn InferencePort>>>>` + `drain()` for swap |
| 5 | Cold-start race: llama-server not ready when backend connects | Exponential backoff health polling (1s-60s) at startup |
| 6 | Fan-out partial failure undefined | Collect successes, return partial if ≥1; `AllBackendsFailed` if 0 |
| 7 | `config.rs` shape unspecified | `KernelConfig` + `MuscleConfig` structs defined |
| 8 | MemoryGuard undefined on Linux | Same floor logic, `wait_for_dwm()` is no-op on Linux |
| 9 | `num_branches` type mismatch (proto `int32` vs spec `uint32`) | Keep `int32`, clamp at domain boundary |
| 10 | `HealthStream` backpressure undefined | 5s push cadence, buffer 4, drop oldest on slow subscriber |
| 11 | `SwapModel` concurrency: in-flight requests killed | `drain()` with 30s timeout before swap |
| 12 | VascularSystem sender dropped immediately in `main.rs` | Step 6 now includes "fix fanout (store tx)" |
