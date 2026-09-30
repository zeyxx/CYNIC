# Debt Priorities and Agent Architecture

- **schema**: `cynic.design.debt_priorities.v1`
- **status**: `proposal` — read-only observation, not authority
- **written**: 2026-09-30
- **branch**: `agent/hermes/debt-priorities-architecture`

---

## 1. Current State — Measured Facts

### 1.1 LOC Census (src only, 2026-09-30)

| Crate | LOC (src) | God-object? | Sub-domains |
|---|---|---|---|
| cynic-host | 83,763 | YES (3.3x) | 37 internal dirs |
| cynic-node | 55,454 | YES (2.2x) | 17 internal dirs |
| cynic-protocol | 32,902 | YES (1.3x) | policy in L0 |
| cynic-cli | 25,158 | borderline | logic in projection |
| cynic-inference | 25,715 | borderline | sink unplugged |
| cynic-intel | 10,950 | no | — |
| cynic-build | 9,645 | no | — |
| cynic-observer | 8,560 | no | — |
| cynic-vcs + forgejo | 8,762 | no | — |
| cynic-kernel | 6,963 | no | purged |
| cynic-lifecycle | 6,333 | no | — |
| cynic-workload | 6,258 | no | — |
| cynic-node-protocol | 5,934 | no | — |
| cynic-eval | 4,404 | no | — |
| cynic-storage | 4,132 | no | — |
| cynic-network | 3,590 | no | — |
| cynic-pkg | 3,245 | no | — |
| cynic-domain-ops | 2,981 | no | — |
| cynic-consensus | 2,598 | no | — |
| cynic-vcs-forgejo | 1,779 | no | — |
| cynic-host-probes | 1,304 | no | — |
| cynic-forensics | 1,279 | no | — |
| cynic-lxc | 1,207 | no | — |
| cynic-agent-bus | 1,086 | no | stub |
| cynic-market | 1,079 | no | — |
| cynic-release | 1,072 | no | — |
| cynic-finance | 1,049 | no | — |
| cynic-secrets | 1,046 | no | — |
| cynic-sandbox | 1,045 | no | — |
| cynic-intel-escore | 983 | no | — |
| cynic-mcp | 948 | no | — |
| cynic-pool | 790 | no | — |
| cynic-intel-soc | 621 | no | — |
| cynic-domain-cloudflare | 621 | no | — |
| cynic-types | 518 | no | — |
| cynic-solana-pay | 511 | no | — |
| cynic-domain-porkbun | 498 | no | — |
| cynic-channel | 485 | no | — |
| cynic-agent-bus-coordinator | 479 | no | — |
| cynic-zeroclaw-solana | 443 | no | — |
| cynic-askesis | 443 | no | — |
| cynic-discovery | 413 | no | — |
| cynic-intel-futarchy | 252 | no | — |
| cynic-discovery-host | 242 | no | — |
| cynic-identity | 238 | no | — |
| cynic-domain-askesis | 124 | no | — |
| cynic-agent-bus-server | 113 | no | stub |
| cynic-inference-runtime | 28 | no | stub |

### 1.2 Hotspots (files >500 LOC)

```
1119  cynic-node/src/fleet_inventory.rs
1068  cynic-cli/src/build_surface.rs
1063  cynic-protocol/src/agent_coordination.rs
1037  cynic-cli/src/agent_surface/coordination.rs
1016  cynic-protocol/src/framework.rs
1000  cynic-protocol/src/error_catalog.rs
 999  cynic-vcs-forgejo/src/forgejo_token.rs
 954  cynic-host/src/usecases/tests.rs
 929  cynic-node-protocol/src/node_enrollment.rs
 928  cynic-node/src/usecases/network_surfaces_uc.rs
 924  cynic-host/src/network_actuation.rs
 919  cynic-node-protocol/src/node_activation.rs
 890  cynic-domain-ops/src/runner.rs
 883  cynic-host/src/runtime_layout.rs
 879  cynic-node/src/adapters/proxmox_control.rs
 878  cynic-inference/src/application/chat_service.rs
 872  cynic-cli/src/chat_surface/runtime.rs
 842  cynic-protocol/src/federation_membership.rs
 841  cynic-node/src/cli.rs
 839  cynic-host/src/transport.rs
 833  cynic-intel/src/ccm/engine.rs
 824  cynic-host/src/runtime_metrics.rs
 822  cynic-cli/src/source_surface.rs
 809  cynic-host/src/agent_host/observe.rs
```

### 1.3 Known Structural Violations

| Violation | Location | Contract |
|---|---|---|
| God-object | cynic-host 84k LOC | `strict_dag_and_single_responsibility` (25k ceiling) |
| God-object | cynic-node 55k LOC | same |
| God-object | cynic-protocol 33k LOC | same |
| Policy in L0 | cynic-protocol `agent_coordination.rs` (1063 LOC) | L0 = types only |
| Policy in L0 | cynic-protocol `framework.rs` (1016 LOC) | L0 = types only |
| Logic in projection | cynic-cli `build_surface.rs` (1068 LOC) | L4 = thin facade |
| Logic in projection | cynic-cli `agent_surface/coordination.rs` (1037 LOC) | L4 = thin facade |
| Stub coordination | cynic-agent-bus-server 113 LOC | `fractal_agent_recursion` |
| Stub runtime | cynic-inference-runtime 28 LOC | `runtime_proof_over_prose` |
| No TLS | all HTTP surfaces | production blocker |
| No mTLS | inter-node communication | production blocker |
| Sink unplugged | ChatTrace → JSONL | `explicit_over_implicit` |
| Sink unplugged | Finance meters → aggregator | `explicit_over_implicit` |
| node_host lie | `available: true` = `is_some()` | `runtime_proof_over_prose` |
| Builder stale | builder at fc7bcfe2, main at c83893e4 | `runtime_proof_over_prose` |

---

## 2. Priority Domains (ordered by compound)

### P1 — Extraction cynic-host (84k → ~8 crates)

**Why first**: largest structural lever. Each extraction forces explicit contracts.

**Target crates** (proposed):
- `cynic-host-ssh` — ssh_config, ssh_trust, host_exec_cli
- `cynic-host-toolchain` — toolchain, build_runner_harness
- `cynic-host-audit` — audit_engine, divergence rules
- `cynic-host-os-health` — os_health, runtime_metrics
- `cynic-host-coordination` — coordination_lease_store, one_shot_authority
- `cynic-host-source` — source_sync, space_gc
- `cynic-host-principal` — principal_prepare, privilege, credential_hardening
- `cynic-host-lab` — lab, profiles

**Proof**: `cargo check` green on builder per extracted crate. LOC per crate < 10k.

### P2 — Policy/Types separation in cynic-protocol

**Why**: L0 must be pure types. Policy logic belongs in L1+.

**Target**: extract `agent_coordination.rs` (1063 LOC) and `framework.rs` (1016 LOC) policy into `cynic-coordination-policy` (L2) or into existing `cynic-agent-bus`.

**Proof**: cynic-protocol < 15k LOC, zero `fn` with side effects in L0.

### P3 — Real coordination channel

**Why**: without HTTP coordination, all agents operate in file-local silos.

**Target**: cynic-agent-bus-server from 113 LOC stub → real HTTP server with:
- Bearer token auth (fix the `Missing Bearer token` error)
- Claim lifecycle over HTTP (not file)
- Fencing token as distributed CAS

**Proof**: `cynic agent claim` succeeds via HTTP, not file fallback.

### P4 — Data pipeline (meters → sink)

**Why**: without measurements, no data-centric approach is possible.

**Target**: wire existing meters to JSONL sinks:
- ChatTrace → `evidence/inference_traces.jsonl`
- Finance meters → `evidence/finance_*.jsonl`
- Host meters → `evidence/host_metrics.jsonl`

**Proof**: JSONL files grow with real data after a session.

### P5 — TLS/mTLS transport

**Why**: production blocker. All inter-node communication is plaintext.

**Target**: TLS for cynic-agent-bus-server, mTLS for inter-node.

**Proof**: `curl https://...` succeeds, not `http://...`.

---

## 3. Agent Architecture — Current vs Target

### 3.1 Current (linear, human-driven)

```
Human operator
    │
    ▼
Hermes-Agent (LLM)
    │
    ▼
cynic CLI (manual invocation)
    │
    ▼
CYNIC crates (compile, audit, build)
    │
    ▼
Git commits (human-reviewed)
```

**Problems**:
- One human → one LLM → one CLI call at a time
- No parallel agent execution
- No agent-to-agent coordination
- No data feedback loop
- No autonomous task execution

### 3.2 Target (data-centric, multi-agent)

```
Human operator (sovereign)
    │
    ▼
Hermes-Agent (orchestrator LLM)
    │
    ├──► Agent A (extraction) ──► cynic CLI ──► Git commit ──► Build proof
    │
    ├──► Agent B (audit) ──────► cynic CLI ──► JSON metrics
    │
    └──► Agent C (data) ───────► cynic CLI ──► JSONL traces
              │
              ▼
         Coordination bus (HTTP claims, fencing)
              │
              ▼
         Data pipeline (meters → JSONL → aggregation)
              │
              ▼
         Feedback loop (metrics inform next agent task)
```

**Key properties**:
- Multiple agents run in parallel on independent scopes
- Each agent claims a scope, executes, releases with proof
- Coordination via HTTP claims (not file)
- Data pipeline feeds back into agent task selection
- Human operator sets priorities, agents execute

### 3.3 NousPortal Free Models via Hermes-Agent

**Current capability**: Hermes-Agent can use free models from NousPortal (e.g., `meituan/longcat-2.5-preview:free`).

**How CYNIC agents use this**:
- Hermes-Agent IS the agent runtime — it already has LLM access
- CYNIC provides the CLI surfaces, audit gates, and proof infrastructure
- Agents invoke `cynic` CLI commands, read JSON output, make decisions
- No separate inference runtime needed for agent orchestration

**What CYNIC inference adds** (future):
- Local model for offline/air-gapped operation
- ChatTrace data for model evaluation and routing
- Not required for agent orchestration — Hermes-Agent covers this

### 3.4 Agent Task Types

| Task Type | Trigger | Proof | Data |
|---|---|---|---|
| Extraction | God-object LOC threshold | `cargo check` green | LOC before/after |
| Audit | Per commit | `cynic audit all` exit 0 | violations count |
| Fix | Audit finding | `cargo check` green | fix LOC |
| Data collection | Per session | JSONL file grows | metrics count |
| Build | Per commit | builder exit 0 | build duration |

---

## 4. Feedback Loop — Data-Centric Approach

### 4.1 Metrics to Collect (per commit)

```json
{
  "schema": "cynic.metrics.commit.v1",
  "sha": "...",
  "timestamp": "...",
  "crates": {
    "total": 48,
    "god_objects": 3,
    "over_10k": 5,
    "orphans": 0
  },
  "build": {
    "exit_code": 0,
    "duration_ms": 12345,
    "builder_sha": "..."
  },
  "audit": {
    "secrets": 0,
    "boundary_violations": 0,
    "node_host_available": true
  },
  "agents": {
    "active_claims": 0,
    "expired_claims": 0
  }
}
```

### 4.2 Metrics to Collect (per inference turn)

```json
{
  "schema": "cynic.trace.chat_turn.v1",
  "ts_unix_ms": 0,
  "session_id": "...",
  "model_alias": "...",
  "status": "ok",
  "prompt_tokens": null,
  "gen_tokens": null,
  "elapsed_ms": 0
}
```

### 4.3 Feedback Loop

```
Commit → Build → Audit → Metrics JSON
                              │
                              ▼
                    Agent reads metrics
                              │
                              ▼
                    Agent selects next task
                    (highest compound first)
                              │
                              ▼
                    Agent executes → Commit
                              │
                              ▼
                         (repeat)
```

---

## 5. Immediate Next Steps

1. Fix `node_host.available` (1 commit, honest audit)
2. Build builder on `c83893e4` (proof main is green)
3. Start P1 extraction (cynic-host → first sub-crate)
4. Wire ChatTrace sink (first data pipeline)
5. Design cynic-api (coordination HTTP backend)

---

## 6. Open Questions

- Should `cynic-agent-bus-server` be L2 or L3? (currently ambiguous)
- Should policy extraction from cynic-protocol go to `cynic-agent-bus` or a new `cynic-coordination-policy`?
- Should TLS use `rustls` or `native-tls`? (rustls preferred for pure Rust)
- Should the data pipeline be a new crate or a module in `cynic-observer`?
- How do we prevent agent scope overlap without a central coordinator?
