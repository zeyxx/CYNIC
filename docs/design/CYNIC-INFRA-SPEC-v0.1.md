# CYNIC Infrastructure Specification — v0.1

*Initiated 2026-03-28. Living document. Every claim is falsifiable.*

**Status:** DRAFT — 8/16 dimensions covered. Gaps marked explicitly.

---

## 1. Vision

A sovereign distributed inference system where:
- The CYNIC kernel orchestrates Dogs (AI validators) across a heterogeneous fleet
- Contributors lend GPU/CPU with zero configuration
- Works from Raspberry Pi to multi-GPU nodes
- No mandatory cloud dependency (total sovereignty)

## 2. Constraints (non-negotiable)

| ID | Constraint | Rationale |
|----|-----------|-----------|
| C1 | Platform-agnostic | RPi, phone, desktop, GPU server |
| C2 | Zero-config for contributors | "Install binary, done" |
| C3 | Sovereign | Self-hostable, no cloud lock-in |
| C4 | Survives disconnections | Home networks, mobile, unstable |
| C5 | Observable | Kernel knows who's up/down in real-time |
| C6 | Secure by default | Contributors isolated, least privilege |

## 3. Current State (probed 2026-03-28)

### Fleet
- **cynic-core** (Linux): Kernel + SurrealDB + llama-server (Gemma 4B CPU, 13 tok/s) + embedding server
- **cynic-gpu** (Windows→Debian tomorrow): llama-server (Qwen 3.5 9B GPU, 55 tok/s)
- Network: Tailscale mesh, 2 active nodes

### Existing Infrastructure
- **CYNIC Kernel** (Rust): REST API (Axum) + MCP (rmcp, stdio) + gRPC protos (partial)
- **MCP Tailscale** (Go): Fleet discovery, SSH exec, persistent shells, background commands, service management
- **MCP CYNIC** (Rust, in-kernel): 10 tools — judge, health, verdicts, crystals, infer, audit, coord

### What Works
- Judge pipeline: 5 Dogs, consensus under φ-bounded doubt
- Health loop: 30s probe, circuit breaker per Dog
- CCM: crystal lifecycle, observation aggregation
- Coord: multi-agent file claiming (MCP-only)
- Background commands: ts_exec timeout → async (P0, shipped today)
- Shell recovery: BUSY state instead of DEAD (P1, shipped today)

## 4. Gaps — Covered (8/16)

### G1 — Push Events (MCP Tailscale)
**Problem:** 100% pull. No node can proactively report "I'm dying."
**Evidence:** sshd zombie on cynic-gpu — discovered only when ts_exec failed.
**Target:** Nodes report health proactively.

### G2 — Agent-less Architecture (MCP Tailscale)
**Problem:** SSH one-shot per command. No persistent presence on nodes.
**Evidence:** Every `ts_exec` = new SSH session. Windows `start` via SSH doesn't detach.
**Target:** Lightweight agent on each node.
**Research:** tsnet (Tailscale embedded in Go) enables zero-config join. OpAMP pattern for agent management.

### G3 — Network Lock-in (MCP Tailscale)
**Problem:** Tailscale hardcoded as network layer.
**Evidence:** If Tailscale coordination plane goes down, no new connections.
**Target:** Abstract network layer. Tailscale = one adapter. Headscale/Nebula as alternatives.
**Research:** Headscale (self-hosted, compatible clients), NetBird (modern OSS), Nebula (Slack, cert-based).

### G4 — MCP CYNIC Incomplete
**Problem:** MCP is read-heavy, write-poor. Agents can judge but not observe, create crystals, or receive events.
**Evidence:** REST has POST /observe, POST /crystal, GET /events — MCP doesn't.
**Target:** Write parity for agent autonomy (Rule 28: agents use the platform).

### G5 — Contribution Model
**Problem:** All machines owned by T. No model for external contributors.
**Target:** Friends contribute GPU % over Tailscale with isolation.
**Status:** NOT RESEARCHED.

### G6 — Inference Routing
**Problem:** One Dog = one fixed machine. No intelligent routing.
**Target:** Route by latency, availability, cost, model capability.
**Status:** Partially exists (health-gated routing in pipeline.rs).

### G7 — Circuit Breaker (MCP Tailscale)
**Problem:** Failed node → 30s wait per attempt, no memory of failure.
**Evidence:** sshd zombie, every ts_exec waited full timeout.
**Target:** Fail-fast with auto-recovery.
**Research:** sony/gobreaker (simple, production-proven), failsafe-go (flexible), mercari/go-circuitbreaker (context-aware).
**Decision:** PENDING implementation.

### G8 — File Transfer (MCP Tailscale)
**Problem:** SFTP not available on all platforms (Windows OpenSSH).
**Evidence:** ts_transfer failed with "subsystem request failed."
**Target:** Fallback chain: SFTP → SCP → base64 exec.
**Research:** bramvdbogaerde/go-scp (Go native), base64-over-exec (universal fallback).
**Decision:** PENDING implementation.

## 5. Gaps — NOT YET COVERED (8/16)

| ID | Dimension | Why it matters | Priority |
|----|-----------|---------------|----------|
| G9 | **Security model for contributors** | What can a contributor's node see/do? Trust boundaries? | HIGH — blocks G5 |
| G10 | **Model distribution** | How to push GGUFs to new nodes? 5-20GB files. | MEDIUM — blocks contributor onboarding |
| G11 | **A2A protocol** | Agent-to-agent communication (existing roadmap mentions it) | LOW — strategic, not blocking |
| G12 | **gRPC (existing in kernel)** | Proto + services partially exist. Dog-to-dog streaming. | MEDIUM — need inventory of what exists |
| G13 | **External monitoring** | Prometheus exposed but not consumed. No alerting. | MEDIUM — ops maturity |
| G14 | **Backup/recovery** | Single point of failure: cynic-core + SurrealDB | HIGH — data loss risk |
| G15 | **Edge inference** | Tiny model on phone/RPi as a Dog? | LOW — vision, not blocking |
| G16 | **Cost metering** | Track GPU-hours per contributor for fairness | LOW — blocks at scale only |

## 6. Architecture Target (sketch)

```
┌─────────────────────────────────────────────────┐
│  AI Agents (Claude, Gemini, future)             │
│  └─ MCP protocol (stdio/SSE)                   │
├─────────────────────────────────────────────────┤
│  MCP Orchestrator (Go, ex tailscale-mcp)        │
│  ├─ Fleet discovery + health                    │
│  ├─ Command dispatch (sync + async)             │
│  ├─ File transfer (SFTP/SCP/base64)             │
│  ├─ Circuit breaker per node                    │
│  └─ Inference routing                           │
├──────────────┬──────────────────────────────────┤
│ Network Layer│ Tailscale / Headscale / Nebula   │
│ (abstracted) │ WireGuard underneath             │
├──────────────┴──────────────────────────────────┤
│  cynic-agent (Go, lightweight, per node)        │
│  ├─ Health push                                 │
│  ├─ Command execution                           │
│  ├─ Inference serving (llama-server wrapper)     │
│  ├─ File transfer                               │
│  ├─ Self-update                                 │
│  └─ tsnet embedded (zero-config network join)   │
├─────────────────────────────────────────────────┤
│  CYNIC Kernel (Rust, central)                   │
│  ├─ Judge pipeline + Dog orchestration          │
│  ├─ Crystal/CCM memory                          │
│  ├─ Storage (SurrealDB)                         │
│  ├─ REST + MCP + gRPC APIs                      │
│  └─ A2A (future)                                │
└─────────────────────────────────────────────────┘
```

## 7. Implementation Sequence (compound organic)

| Phase | What | Prerequisite | Effort | Value |
|-------|------|-------------|--------|-------|
| **v0.1** | gobreaker + go-scp in tailscale-mcp | Nothing | 2-4h | Immediate resilience |
| **v0.2** | Abstract network layer | v0.1 | 4h | Tailscale no longer hardcoded |
| **v0.3** | MCP CYNIC write path (observe, crystal create) | Nothing | 4h | Agent autonomy |
| **v0.4** | cynic-agent spec + prototype (tsnet) | v0.2 + research G9 | 2 sessions | Contributor foundation |
| **v0.5** | Security model + contribution protocol | v0.4 + research G9 | 1 session | Friends can contribute |
| **v0.6** | Inventory gRPC existing + streaming events | v0.3 | 1 session | Push events, real-time |
| **v0.7** | Model distribution pipeline | v0.4 | 1 session | GGUF push to nodes |
| **v1.0** | External monitoring + backup/recovery | v0.6 | 1 session | Production maturity |

## 8. Research Debt (to investigate before building)

| Topic | Question | Blocks |
|-------|----------|--------|
| Contributor security | What trust model? Capability-based? Sandbox? | v0.5 |
| gRPC kernel inventory | What proto services exist? What's wired? | v0.6 |
| A2A protocol | Adopt Google A2A? Build CYNIC-native? | v1.0+ |
| Edge inference | Minimum viable model for RPi/phone? | v1.0+ |

---

*v0.1 — 2026-03-28 — 8/16 gaps covered, 8 pending research*
*Next: implement v0.1 (gobreaker + go-scp), research G9 (security model)*
