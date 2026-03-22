# CYNIC-OS Research — Agent Operating System Architecture

*2026-03-21 — Based on 2-agent deep research + overnight analysis of 23 repos*

## 1. The OS Concept Mapping

| OS Concept | Traditional | CYNIC Today | Gap |
|---|---|---|---|
| Kernel | Privileged orchestration | `cynic-kernel` | - |
| Syscalls | Stable interface user→kernel | Port traits (InferencePort, StoragePort, EventBus, CoordPort) | No versioning policy |
| Process | Running program, isolated | Agent session | No isolation (shared FS) |
| Scheduler | Routes work to CPU | BackendRouter (routes to Dogs) | No preemption, no cost-aware priority |
| IPC | pipes, sockets, shared mem | VascularSystem (mpsc channels) | No external IPC (WebSocket needed) |
| Signals | Async notifications | HeresyNotice | - |
| Filesystem | Persistent storage | StoragePort + SurrealDB | - |
| Device drivers | Hardware adapters | InferencePort adapters (llama.cpp, Ollama, Gemini, HF) | - |
| Mutex | Synchronization | coord/claim + coord/release | Non-atomic, theater |
| Permissions/ACL | User/group/rwx | Bearer auth only | No per-agent capabilities |
| Capabilities (seL4) | Unforgeable tokens | - | Not implemented |
| Sandbox | Process isolation | - | No WASM/Docker sandboxing |
| Taint tracking | Info flow control | - | Not implemented |
| Cgroups | Resource limits per process | Rate limit (30/min global) | No per-agent token budgets |
| Namespaces | Isolated resource views | - | Sessions not namespace-isolated |
| Audit log | Immutable record | SurrealDB mcp_audit table | Not tamper-evident (no Merkle chain) |
| OOM killer | Evict under pressure | CCM decay (φ⁻² threshold) | - |
| Init system | Boot sequence | main.rs boot: probe→storage→backends→bus→health | - |
| Loadable modules | Extend kernel at runtime | MCP tools, Dog plugins via InferencePort | - |
| Graceful degradation | Boot with what's available | 3-state health: HEALTHY/DEGRADED/CRITICAL | - |

## 2. The Fractal Invariant

```
Agent:        Dog judges a chess move using 6 axioms
Node:         CYNIC judges a Dog's verdict using the same 6 axioms
Cluster:      Network judges a CYNIC node using the same 6 axioms
Federation:   Epistemic immune system filters truth using the same 6 axioms
```

The fractal invariant: **the same scheduling, isolation, and accountability primitives that apply to a single agent must also apply to a cluster of nodes and a federation of clusters.** If the abstraction breaks at a different scale, it was not truly fractal.

## 3. Four Agent Types — Integration Matrix

| Property | Claude Code | Gemini CLI | OpenClaw | KAIROS |
|---|---|---|---|---|
| Lifecycle | Session (ephemeral) | Session (ephemeral) | Persistent daemon | Continuous daemon (60s cycles) |
| Communication | stdio MCP + hooks + REST | stdio MCP + hooks + REST | WebSocket (18789) + HTTP + ACP bridge | HTTP REST only (POST /judge) |
| Hook system | 12 events, bash, sync/async | ~10 events, bash, sync | Webhooks out, tool policies | None (loop is code) |
| Session identity | CLAUDE_SESSION_ID | GEMINI_SESSION_ID | Binding tuple (channel,account,peer) | No session — process identity |
| Sandbox | Per-tool allow/deny | Docker sandbox | Docker per non-elevated session | Trusted systemd service |
| CYNIC integration | Deep (MCP + 4 hooks) | Minimal (GEMINI.md only) | None | HTTP adapter (CynicHttpAdapter) |
| Needs from OS | FS, git, shell, MCP | FS, git, shell, MCP, Docker | Node.js, port 18789, FS, Docker | Python, network, SurrealDB |

## 4. The Universal Interface (Already Exists)

```
POST /judge         # universal — any agent, any domain
POST /observe       # universal — feed CCM learning loop
GET  /health        # universal — liveness probe
WS   /ws            # universal — event bus (to build)
POST /coord/register  # session agents
POST /coord/claim     # session agents
POST /coord/release   # session agents
```

All 7 endpoints already exist (except /ws). The gap is wiring, not API.

## 5. What Makes an OS Industrial-Grade (7-year horizon)

From POSIX (50+ years), Linux (35), seL4 (15+):

1. **Stable interfaces with explicit contracts** — Port traits = POSIX syscalls. Breaking change = new major version + 18-24 month parallel support.
2. **Minimal trusted computing base** — Keep kernel small. Judgment logic, learning, crystallization outside kernel behind port traits.
3. **Mechanism without policy** (Unix principle) — EventBus doesn't know if events are verdicts or health checks. StoragePort doesn't know if facts are chess moves or market signals.
4. **Deprecation as first-class operation** — Semantic versioning, Deprecation headers, 2 major versions supported simultaneously.
5. **Observability built in** — Structured logging, metrics, distributed tracing, health endpoints. Non-negotiable interfaces.
6. **Failure modes designed, not discovered** — `BackendError::CircuitOpen`, `StorageError::Unreachable` are stable contracts. Changing an error variant is breaking.
7. **Graceful degradation over availability** — Boot with 2 Dogs instead of 5. Never panic on non-critical failure.
8. **Zero implicit global state** — Composition root pattern (only main.rs knows concrete types).

## 6. Existing Agent OS Projects

- **AIOS** (Rutgers, COLM 2025) — Academic. LLM = CPU core. Scheduler (FIFO/RR), context snapshot/restore, LRU-K memory eviction. 2.1x throughput, linear to 2000 agents.
- **OpenFang** (Rust, 15K stars) — Most architecturally complete. 14 crates, 137K LOC. WASM sandbox, Ed25519 manifests, taint tracking, Merkle audit, OFP protocol. Pre-1.0.
- **ACOS** (arxiv) — Research. 7 specialized agents replace OS components.
- **AgenticOS 2026 Workshop** — USENIX. Identifies "pre-OS era" — chaos of duplicated solutions lacking fundamental abstractions.

## 7. What CYNIC Has vs What It Needs

### Already Present
- Kernel as orchestration layer
- Port traits as stable interfaces
- Composition root pattern
- Circuit breaker (partial)
- HeresyNotice = signals
- VascularSystem = IPC
- 3-state health = graceful degradation
- Fractal axiom system
- CCM decay = knowledge OOM
- MCP tools = loadable modules

### Missing for True Agent OS
1. Per-agent capability manifests (RBAC)
2. Preemption (interrupt running inference)
3. WASM/Docker tool sandboxing
4. Taint tracking (secret flow labeling)
5. Token budget allocator (cgroup per agent)
6. Namespace isolation (per-session storage view)
7. Merkle audit chain (tamper-evident)
8. API versioning policy
9. LODController (degrade consciousness under pressure)
10. WebSocket event bus (/ws)
11. `cynic run` launcher (worktree + MCP + scope injection)

## Sources

- AIOS: arxiv.org/html/2403.16971v5
- OpenFang: github.com/RightNow-AI/openfang
- AgenticOS 2026: os-for-agent.github.io
- ACOS: arxiv 2411.17710
- seL4 Design Principles: microkerneldude.org
- Fractal Component Model: France Telecom + INRIA, 2001
- Claude Code hooks: code.claude.com/docs/en/hooks
- Gemini CLI hooks: geminicli.com/docs
- OpenClaw Gateway: docs.openclaw.ai/gateway/protocol
- MCP Streamable HTTP: modelcontextprotocol.io/specification/2025-03-26
