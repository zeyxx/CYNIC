# CYNIC Kernel — Crystallized Architecture Truths

*Produced via Crystallize Truth framework. 3 layers applied. Conclusions stable.*
*Last validated: 2026-03-12 — Forgejo pipeline active.*

---

## TRUTH TABLE

| T# | Truth | Confidence | Design Impact |
|----|-------|------------|---------------|
| T1 | The domain core must contain ZERO platform-specific code — every OS/hardware call lives in an adapter behind a trait | 58% | 4 driven port traits: `GpuDetector`, `PlatformPaths`, `ProcessSpawner`, `SystemMetrics`. Plus existing `InferencePort`. |
| T2 | NodeConfig is a boot-time value object, not a runtime dependency — it seeds the composition root then the event bus handles runtime state | 55% | main.rs destructures NodeConfig into per-service config slices. No God Object passing. |
| T3 | probe.rs (897 LOC, 12 responsibilities) must split by SRP — only GPU detection needs platform-specific adapters behind a trait | 52% | 6 modules: gpu_detect, model_scanner, server_scanner, system_info, sovereignty_advisor, config_persistence |
| T4 | Health is a first-class domain concept with 3 states: HEALTHY, DEGRADED, CRITICAL — not a boolean | 56% | Circuit breaker on BackendRouter. Boot with degraded state. HeresyNotice for every degradation. Refuse inference only at CRITICAL. |
| T5 | The VascularSystem (event bus) is the integration backbone, not a nice-to-have — without it, services cannot communicate and telemetry doesn't flow | 50% | Fix the dropped sender. Implement real pub/sub fanout. Every service publishes state changes. |
| T6 | Hexagonal rigor must be proportional to rate-of-change — hot paths (inference pipeline) get full ports, cold paths (config persistence) get good functions | 48% | Prevents over-engineering. 5-6 port traits total, not 15+. |
| T7 | `BackendRouter` must not health-check on every request — circuit breaker pattern with periodic probe is O(N) vs current O(N*M) | 54% | Add circuit breaker state per backend: Closed (healthy), Open (failed, skip), HalfOpen (probing recovery) |
| T8 | Every driven port must have a mock adapter that passes the same test suite as the real adapter — this is how hexagonal enables testing | 52% | Mock adapters are not test scaffolding — they are first-class implementations. Test suite is port-level, not adapter-level. |
| T9 | The kernel must boot with whatever is available and CLEARLY REPORT what's missing — not panic, not silently skip | 55% | Boot sequence tries each service, reports HEALTHY/DEGRADED/CRITICAL via HeresyNotice, refuses work only when CRITICAL. |
| T10 | `dirs` crate replaces ALL `$HOME`/`/tmp` fallbacks — eliminates the #1 Windows blocker in one dependency | 58% | Replace every `std::env::var("HOME")` with `dirs::home_dir()`. Works on Windows, Linux, macOS without `#[cfg]`. |

---

## TARGET HEXAGONAL ARCHITECTURE

```
                    DRIVING ADAPTERS (in)
                    =====================

    gRPC MuscleHAL ─────┐
    gRPC KPulse ─────────┤
    gRPC Vascular ───────┤        ┌──────────────────────────────┐
    gRPC CogMemory ──────┼───────▶│                              │
    CLI (future) ────────┘        │       DOMAIN CORE            │
                                  │                              │
                                  │  InferenceRequest/Response   │
                                  │  BackendCapability           │
                                  │  BackendStatus (3-state)     │
                                  │  NodeConfig (value object)   │
                                  │  HealthState enum            │
                                  │  SovereigntyAdvice (pure fn) │
                                  │                              │
                                  │       PORTS (traits)         │
                                  │  ┌─────────────────────┐     │
                                  │  │ InferencePort       │◀────┼──── LlamaCppBackend
                                  │  │ StoragePort         │◀────┼──── SurrealDbAdapter
                                  │  │ GpuDetector         │◀────┼──── SysfsDetector / WmiDetector / NvidiaSmiDetector
                                  │  │ ProcessSpawner      │◀────┼──── TokioProcessAdapter
                                  │  │ EventBus            │◀────┼──── VascularAdapter (mpsc channels)
                                  │  └─────────────────────┘     │
                                  │                              │
                                  └──────────────────────────────┘

                    DRIVEN ADAPTERS (out)
                    ====================

    LlamaCppBackend ──────── HTTP ──────── llama-server (any host)
    SurrealDbAdapter ─────── WS ────────── SurrealDB
    SysfsDetector ────────── /sys ──────── Linux kernel
    WmiDetector ──────────── PowerShell ── Windows WMI
    NvidiaSmiDetector ────── subprocess ── nvidia-smi
    TokioProcessAdapter ──── tokio ─────── OS process table
    VascularAdapter ──────── mpsc ──────── in-process event bus
```

### Dependency Rule
```
Adapters ──depend-on──▶ Ports (traits) ──live-in──▶ Domain Core
Domain Core depends on NOTHING external.
main.rs (composition root) wires adapters to ports.
```

---

## PORT DEFINITIONS (the contracts)

### Port 1: InferencePort (EXISTS — backend.rs)
```rust
#[async_trait]
pub trait InferencePort: Send + Sync {
    fn capability(&self) -> &BackendCapability;
    async fn infer(&self, req: InferenceRequest) -> Result<InferenceResponse, BackendError>;
    async fn health(&self) -> BackendStatus;
}
```
**Adapters:** LlamaCppBackend (exists), MockBackend (exists), future: OllamaBackend, VllmBackend, RemoteBackend

### Port 2: StoragePort (NEW)
```rust
#[async_trait]
pub trait StoragePort: Send + Sync {
    async fn store_fact(&self, fact: Fact) -> Result<(), StorageError>;
    async fn query_facts(&self, key: &str) -> Result<Vec<Fact>, StorageError>;
    async fn register_trust(&self, entry: TrustEntry) -> Result<(), StorageError>;
    async fn verify_trust(&self, entry: &TrustEntry) -> Result<bool, StorageError>;
}
```
**Adapters:** SurrealDbAdapter (from current storage.rs), MockStorageAdapter (new), future: SqliteAdapter

### Port 3: GpuDetector (NEW)
```rust
pub trait GpuDetector: Send + Sync {
    fn detect(&self) -> Option<ComputeInfo>;
    fn name(&self) -> &str;  // "sysfs", "nvidia-smi", "wmi", etc.
}
```
**Adapters:** SysfsDetector (Linux), WmiDetector (Windows), NvidiaSmiDetector (cross-platform), MetalDetector (macOS), CpuFallback (always)
**Selection:** Composition root picks adapters based on `cfg!(target_os)`. Domain runs them in priority order.

### Port 4: ProcessSpawner (NEW)
```rust
#[async_trait]
pub trait ProcessSpawner: Send + Sync {
    async fn spawn(&self, cmd: &str, args: &[&str]) -> Result<ProcessHandle, SpawnError>;
    async fn kill(&self, handle: &ProcessHandle) -> Result<(), SpawnError>;
    async fn wait(&self, handle: &ProcessHandle) -> Result<ExitStatus, SpawnError>;
}
```
**Adapters:** TokioProcessAdapter (real), MockProcessAdapter (tests)
**Used by:** DaemonSupervisor (which becomes domain logic using this port, not a concrete process spawner)

### Port 5: EventBus (NEW)
```rust
#[async_trait]
pub trait EventBus: Send + Sync {
    async fn publish(&self, topic: &str, payload: &str) -> Result<(), BusError>;
    async fn subscribe(&self, topics: &[&str]) -> Result<EventReceiver, BusError>;
}
```
**Adapters:** ChannelBus (in-process mpsc), future: NatsBus, RedisBus
**Role:** Integration backbone. Inference telemetry, health transitions, memory pressure events all flow through here.

### NOT a port (cold paths — good functions suffice):
- **Config persistence** — `save_config()` / `load_config()` using `dirs::home_dir()` + `toml`
- **IO benchmark** — pure function, runs at boot, no adapter needed
- **Sovereignty advisor** — pure function on NodeConfig, no I/O
- **Model scanner** — uses `walkdir` (cross-platform), no adapter needed
- **Server scanner** — uses `reqwest` (cross-platform), no adapter needed

---

## BOOT SEQUENCE (composition root)

```
main.rs — THE ONLY FILE THAT KNOWS ABOUT CONCRETE TYPES

1. Parse config from env vars (ConfigPort unnecessary — env::var is cross-platform)
2. Select platform adapters:
     gpu_detectors = match cfg!(target_os) {
         "linux"   => vec![NvidiaSmiDetector, SysfsDetector, CpuFallback],
         "windows" => vec![NvidiaSmiDetector, WmiDetector, CpuFallback],
         "macos"   => vec![MetalDetector, CpuFallback],
     }
3. Run probe (using injected GpuDetectors):
     node_config = probe::run(gpu_detectors, force_reprobe).await
4. Connect storage (with circuit breaker):
     storage: Box<dyn StoragePort> = SurrealDbAdapter::connect(config).await
       → on failure: report HeresyNotice("SURREALDB_UNREACHABLE", CRITICAL)
       → boot continues, storage = DegradedStorageAdapter (logs + rejects)
5. Create inference pipeline:
     backends = discover_backends_from_probe(&node_config)
       → if node_config.llm.running_server_url.is_some():
           LlamaCppBackend::connect(url) → register in router
       → if nothing found: report HeresyNotice("NO_INFERENCE_BACKEND", DEGRADED)
     router = BackendRouter::new(backends) // with circuit breaker
6. Create event bus:
     bus: Box<dyn EventBus> = ChannelBus::new()
7. Wire gRPC services (driving adapters):
     MuscleHalService::new(router, bus)
     PulseService::new(node_config.hardware, node_config.compute, bus)
     CognitiveMemoryService::new(storage)
     VascularService::new(bus)
8. Start gRPC server
9. Report boot health:
     if all services healthy → HEALTHY
     if some services degraded → DEGRADED + HeresyNotice per service
     if critical services down → CRITICAL + refuse all inference
```

---

## PROBE REFACTOR (probe.rs → modules)

Current: 1 file, 897 LOC, 12 responsibilities
Target: 6 focused modules

| Module | Responsibility | Platform-specific? | Lines (est.) |
|--------|---------------|--------------------|-------------|
| `probe/mod.rs` | Orchestration: run all probes, assemble NodeConfig | No | ~80 |
| `probe/gpu.rs` | GPU detection via injected `GpuDetector` trait impls | **Yes — via trait** | ~50 (domain) |
| `probe/gpu_linux.rs` | SysfsDetector adapter | Linux only | ~80 |
| `probe/gpu_windows.rs` | WmiDetector adapter | Windows only | ~60 |
| `probe/gpu_nvidia.rs` | NvidiaSmiDetector adapter | Cross-platform | ~40 |
| `probe/models.rs` | GGUF + Ollama model discovery | No (walkdir) | ~150 |
| `probe/servers.rs` | Running inference server discovery | No (reqwest) | ~60 |
| `probe/system.rs` | Hardware info, CPU, RAM, env detection | No (sysinfo) | ~80 |
| `probe/advisor.rs` | SovereigntyAdvisor — pure logic | No | ~60 |
| `probe/persistence.rs` | Load/save NodeConfig to TOML | No (dirs + toml) | ~40 |

**Total:** ~700 LOC (reduced from 897 by removing dead code and duplication)

**Key change:** `probe/mod.rs::run()` takes `Vec<Box<dyn GpuDetector>>` as parameter. The composition root (main.rs) provides the platform-appropriate detectors. The probe module itself has ZERO `#[cfg]` gates and ZERO platform-specific imports.

---

## CIRCUIT BREAKER (per-backend health)

```
States: Closed ──▶ Open ──▶ HalfOpen ──▶ Closed
                     │                      │
                     └──── (timeout) ───────┘

Closed:    requests flow normally. Count consecutive failures.
           After N failures → transition to Open.
Open:      all requests immediately fail with BackendError::CircuitOpen.
           After cooldown_ms → transition to HalfOpen.
HalfOpen:  allow ONE probe request.
           If success → Closed. If failure → Open.
```

**Replaces:** the current `b.health().await` call on EVERY request in the routing loop.

---

## TESTING PYRAMID

```
                    /\
                   /  \    End-to-End (grpcurl → kernel → llama-server)
                  /    \   1-2 tests, slow, require running infra
                 /──────\
                /        \  Integration (real adapter + real external system)
               /          \ LlamaCppBackend + real llama-server
              /            \ SurrealDbAdapter + real SurrealDB
             /──────────────\
            /                \  Port Contract Tests (trait test suite)
           /                  \ Each port trait has a test suite
          /                    \ MockBackend AND LlamaCppBackend both pass it
         /────────────────────── \
        /                        \  Unit Tests (pure domain logic)
       /                          \ BackendRouter selection, circuit breaker state machine
      /                            \ SovereigntyAdvisor, NodeConfig assembly
     /──────────────────────────────\
```

**Critical pattern:** Port contract tests. A set of tests that ANY implementation of a port must pass:
```rust
// This function tests ANY InferencePort implementation
async fn inference_port_contract(port: &dyn InferencePort) {
    // Must report capability
    let cap = port.capability();
    assert!(!cap.id.is_empty());

    // Must report health
    let health = port.health().await;
    // health is one of Healthy, Degraded, Unreachable

    // Must handle inference request
    let req = InferenceRequest { /* ... */ };
    let result = port.infer(req).await;
    // result is Ok or a well-formed BackendError
}
```

MockBackend, LlamaCppBackend, and any future backend ALL run this same test suite. This is how you guarantee Liskov Substitution.

---

## WINDOWS FIXES (concrete actions)

| # | Problem | Fix | Crate/Tool |
|---|---------|-----|------------|
| 1 | `$HOME` fallback to `/tmp` | `dirs::home_dir()` | `dirs` crate |
| 2 | Drive scan limited to C-F | `sysinfo::Disks::new_with_refreshed_list()` to enumerate all drives | `sysinfo` (already dep) |
| 3 | AVX2 hardcoded `true` on Windows | `std::arch::is_x86_feature_detected!("avx2")` | stdlib |
| 4 | `[::1]:50051` fails without IPv6 | Try IPv6, fallback to `127.0.0.1:50051` | stdlib |
| 5 | GPU detection on Windows | `WmiDetector` adapter behind `GpuDetector` trait | PowerShell (exists in probe.rs) |
| 6 | Process paths Unix-style | `Path::new()` everywhere (already cross-platform) | stdlib |
| 7 | `/proc/cpuinfo` for AVX2 | `#[cfg(target_os = "linux")]` gate + stdlib on Windows | stdlib |

---

## DEAD CODE TO REMOVE

| Item | Location | Action |
|------|----------|--------|
| `async-stream` | Cargo.toml | Remove dependency |
| `hal.rs` storage field | hal.rs | Already removed in rewrite |
| French strings in advisor | probe.rs | Standardize to English |
| `BackendKind::Remote` | backend.rs | Keep — will be used for Tailscale backends |
| IO benchmark at boot | probe.rs | Move to on-demand, not boot-blocking |
| prost 0.13 vs tonic 0.12 | Cargo.toml | Align prost to 0.12 to match tonic |

---

## PATTERNS CHECKLIST

| Pattern | Status | Where |
|---------|--------|-------|
| Hexagonal (ports & adapters) | Partial → Full | All driven dependencies |
| Circuit Breaker | Missing → Add | BackendRouter per-backend |
| Service Discovery | Partial (probe) → Formalize | probe detects, router registers |
| Health Checks | Fabricated → Real | PulseService from sysinfo |
| Graceful Degradation | Missing → Add | 3-state health at boot |
| Heartbeats | Missing → Add | HealthStream RPC (proto) |
| Dependency Inversion | 1/8 files → All | Port traits everywhere |
| Single Responsibility | 1/8 files → All | probe.rs split, pulse.rs real data |
| Open/Closed | Violated → Enforced | New backends = new adapter, zero existing code modified |
| Liskov Substitution | Untested → Contract tests | Port contract test suites |
| Interface Segregation | Violated → Fixed | Split fat interfaces into focused ports |
| Backpressure | Missing → Add | Bounded channels on EventBus, max concurrent inference |
| Event-Driven | Broken → Fixed | VascularSystem with real pub/sub fanout |
| Composition Root | Implicit → Explicit | main.rs is THE ONLY file with concrete types |
