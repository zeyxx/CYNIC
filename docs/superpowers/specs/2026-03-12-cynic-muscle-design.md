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

**Confidence:** 58% — The HTTP bridge is straightforward; edge cases are in timeout/retry behavior.

### T3: MemoryGuard Prevents OOM on Shared-Memory APU

The Ryzen 7 5700G shares 32GB between CPU and GPU (Vulkan). DWM needs ~2GB. Loading large models without guard causes DWM crashes and system instability.

**Design:**
- Query available memory before model load
- Reserve 4GB floor (DWM + system)
- Expose memory pressure in `BackendStatus::Degraded` when >80% used
- Delayed start on Windows: wait for DWM initialization before loading model

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

When a single machine is powerful enough, `cynic-kernel` spawns the muscle in-process (no separate binary needed).

**Confidence:** 52% — Clean separation, but the in-process vs. separate-binary decision needs runtime validation.

### T5: num_branches IS Multi-Orchestration

`MCTSInferenceRequest.num_branches` already exists in the proto. When `num_branches > 1`, the `BackendRouter` fans out to multiple backends (or the same backend with different temperatures), collects hypotheses, and returns them as the response vector.

This means multi-orchestration is not a new feature — it's wiring the existing proto field to actual fan-out logic.

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
Step 1: LlamaCppBackend — impl InferencePort for llama.cpp HTTP
Step 2: hal.rs rewrite — BackendRouter dispatching via InferencePort
Step 3: Proto extensions — HALProfile enrichment, model_hint, SwapModel, HealthStream
Step 4: cynic-muscle binary — headless worker with supervisor + probe
Step 5: MemoryGuard — shared-memory APU protection
Step 6: VascularSystem integration — inference telemetry publishing
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
  uint32 num_branches = 4;
  float temperature = 5;
  // New field
  string model_hint = 6;  // Optional: preferred model, empty = router decides
}
```

### New RPCs on MuscleHAL
```protobuf
service MuscleHAL {
  rpc RequestInference(MCTSInferenceRequest) returns (MCTSInferenceResponse);
  rpc GetActiveHAL(PulseRequest) returns (HALProfile);
  // New
  rpc SwapModel(SwapModelRequest) returns (SwapModelResponse);
  rpc HealthStream(PulseRequest) returns (stream HALProfile);
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
    ├── backends: Vec<Box<dyn InferencePort>>
    ├── route(req) → selects backend by:
    │   ├── model_hint match (if provided)
    │   ├── health filter (skip Unreachable/Degraded)
    │   ├── capability match (VRAM, context size)
    │   └── round-robin fallback
    ├── fan_out(req, n) → parallel dispatch to n backends
    └── register/deregister backends at runtime
```

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
| `backend.rs` | **No change** | InferencePort trait is correct as-is |
| `probe.rs` | **No change** | Discovery works, output just needs consumers |
| `supervisor.rs` | **Minor**: add llama-server config | Already has spawn/monitor/restart |
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

No hardcoded values. Configuration via environment variables with sane defaults:

```bash
# Kernel
CYNIC_GRPC_ADDR=[::1]:50051
CYNIC_SURREALDB_URL=ws://localhost:8000
CYNIC_SURREALDB_USER=root
CYNIC_SURREALDB_PASS=<secret>

# Muscle
CYNIC_LLAMA_ENDPOINT=http://127.0.0.1:11435
CYNIC_MEMORY_FLOOR_MB=4096
CYNIC_INFERENCE_TIMEOUT_S=120
CYNIC_GPU_LAYERS=0  # 0 = auto from probe
```

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
