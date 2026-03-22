# CYNIC Real-Time & Multi-Session — Crystallized Truths

*2026-03-21 — Based on 3-agent research + 2 empirical investigations*
*Covers: parallel sessions, event bus (WebSocket vs SSE), IPv6 strategy*

## The Problem

Two Claude Code sessions on the same repo create silent collisions. The CYNIC coord system (claim/release) was built to prevent this but doesn't work. The "fix-and-shift" pattern (each fix creates a new bug) is a symptom.

## Research Findings

### Claude Code Native
- **Worktrees** (`claude --worktree`) = official answer. Separate checkout per session, separate branch.
- **Session lock file** = feature request #19364, closed "not planned", locked.
- **Agent Teams** = experimental, Opus-only, coordinates tasks not files.
- **`CLAUDE_CODE_TASK_LIST_ID`** = shared JSON file for task-level coordination.
- **Known bugs**: `~/.claude.json` overwrite (#1035), plan file cross-contamination (#27311).

### CYNIC Coord Audit
- **Stop hook timeout: 10ms** but curl needs 2-5s. Claims never released by hooks.
- **Claim not atomic**: check-then-UPSERT without transaction. Race window.
- **Scope: only `cynic-kernel/src/*`** — UI, docs, configs unprotected.
- **Fail-open everywhere** — kernel down = zero enforcement, silently.
- **Cost: ~2-5s latency per edit** for zero real protection.
- **Conclusion: theater of security.**

### Ecosystem 2026
- **Consensus**: worktrees (isolation) + ownership zones (convention) + dry-run merge (detection).
- **mcp_agent_mail** (Rust port): advisory file leases via MCP — right concept, CYNIC's implementation is wrong.
- **Orchestrator pattern**: overkill for 2-person team.
- **AGENTS.md**: Linux Foundation standard, Claude reads CLAUDE.md instead.
- **Overstory admits**: "compounding error rates and merge conflicts are the normal case, not edge cases."

## Truth Statements

| T# | Truth | Conf. | Design impact |
|----|-------|-------|---------------|
| T1 | Coord claim/release is a distributed lock manager over HTTP for 2 devs on 1 laptop. Stop hook 10ms makes release impossible. Non-atomic claim makes locking fictional. Cost: ~5 min latency/session, benefit: 0%. | 60% | Disable `protect-files.sh` immediately. |
| T2 | Anthropic's answer to multi-session collision is worktrees, not file coordination. Explicit architectural decision (issue #19364 closed "not planned"). | 58% | Adopt `claude --worktree` as default for parallel sessions. |
| T3 | CYNIC should pivot coord from "lock manager" to "observability bus" — observe what each session does, don't prevent. Human is the circuit breaker. | 55% | Transform `/coord/claim` → `/coord/observe`. PostToolUse hooks feed the bus. |
| T4 | Same observability gap causes fix-and-shift (sessions), 2203 silent failures (Dogs), undetected model change (S. machine). ONE problem. | 58% | Single SSE `/events` endpoint streaming: session activity, Dog state changes, model changes, config drift. |
| T5 | Worktrees solve file collisions but not semantic collisions (2 sessions fix same bug differently). CCM is the semantic collision detector. | 45% | CCM session-aware: tag observations by session_id, detect domain overlap. |
| T6 | Dry-run merge (`git merge --no-commit --no-ff`) before integration is cheapest reliable gate. | 55% | Add to workflow: before merge, dry-run + `make check`. |

## Action Plan

### Immediate (today)
1. Disable `protect-files.sh` pre-edit hook — pure cost, zero benefit
2. Document worktree workflow in CLAUDE.md
3. Fix Stop hook timeout (10ms → 5000ms) if keeping any coord

### Short-term (this week)
4. Design `/ws` WebSocket endpoint — primary event bus for all observability
5. Add `/events` SSE fallback — lightweight stream for curl/scripts
6. Add backend model detection to health loop (probe `/api/tags` or `/v1/models`)
7. Add dry-run merge script to Makefile

### Medium-term
8. Pivot coord from lock manager to observability bus
9. CCM session-aware tagging
10. Explore `CLAUDE_CODE_TASK_LIST_ID` integration

---

## Event Bus — WebSocket vs SSE (Empirical, 2026-03-21)

### Decision: WebSocket primary + SSE fallback

| Criterion | SSE | WebSocket |
|---|---|---|
| Direction | Server → client only | Bidirectional |
| MCP spec 2025-03-26 | Deprecated as standalone | Proposed (SEP-1288) |
| OpenClaw | LLM streaming only | **Control plane** (port 18789) |
| Agent subscription | Impossible — can't filter | Agent sends `{"subscribe": ["verdicts","dogs"]}` |
| Axum | `axum::response::Sse` native | `axum::extract::ws` native |
| Browser | EventSource API | WebSocket API |

### Why WebSocket wins for an agent OS
- OpenClaw uses WebSocket as its control plane — CYNIC must speak the same language
- Agents need to subscribe to specific event types (requires client→server messages)
- Dashboard may need to send commands (not just receive)
- SSE is a strict subset of what WebSocket provides

### Architecture
- `/ws` — WebSocket primary: bidirectional, topic subscriptions, auth via first message
- `/events` — SSE fallback: unidirectionnel, stream all events, Bearer auth via header
- Both coexist natively in axum

### Truth correction
T2 from overnight crystallize-truth ("SSE suffit") was wrong. WebSocket is the right choice
for an OS-for-agents. SSE remains as a lightweight fallback for monitoring scripts.

---

## IPv6 Strategy (Empirical, 2026-03-21)

### Decision: Design for IPv6 now, activate with multi-node

**Current state:**
- Tailscale assigns IPv6 ULA (`fd7a:115c:a1e0::`) to all nodes
- IPv6 ping Ubuntu↔S. machine: 2.8ms (same as IPv4 via WireGuard)
- CYNIC kernel binds IPv4 only (`<TAILSCALE_UBUNTU>:3030`)
- Ollama on S. machine binds IPv4 only (`0.0.0.0:11434`)

**Why IPv6 matters for CYNIC's future:**
- Multi-node CYNIC mesh needs direct addressability — IPv6 eliminates CGNAT
- Each kernel node gets a unique, stable address without NAT
- ULA (`fd7a:...`) = natural private addressing for sovereign mesh
- Distributed consensus (what CYNIC is) requires direct node-to-node connectivity

**Why not urgent today:**
- WireGuard tunnel makes IPv4/IPv6 equivalent in performance
- Only 2 machines currently — no NAT traversal issue
- All the value comes when CYNIC scales to N nodes

### Action plan
1. When adding WebSocket, bind kernel on `[::]:3030` (dual-stack) instead of `<TAILSCALE_UBUNTU>:3030`
2. Configure Ollama on S. machine: `OLLAMA_HOST=[::]:11434` (via Tailscale SSH, T. manages infra)
3. Update `backends.toml` to prefer IPv6 addresses when available
4. Design node discovery for multi-node mesh using IPv6 ULA prefix

### Infra note
T. manages all infrastructure including S.'s machine. No coordination needed
for infra changes — Tailscale SSH provides full access. S. owns `cynic-ui/` code, not infra.

---

## Sources

- Claude Code issue #19364 (session lock file — closed "not planned")
- Claude Code issue #1035 (~/.claude.json overwrite — closed "not planned")
- Claude Code issue #27311 (plan file cross-contamination)
- Dicklesworthstone/mcp_agent_mail_rust (advisory file leases)
- jayminwest/overstory (orchestrator pattern — admits failure modes)
- tick.md (living specification pattern)
- AGENTS.md (Linux Foundation standard)
- Augment Code multi-agent workspace guide
- MCP spec 2025-03-26 (Streamable HTTP replaces SSE)
- SEP-1288 (WebSocket transport proposal for MCP)
- OpenClaw Gateway protocol (WebSocket control plane, port 18789)
- OpenClaw 2026.2.26 release notes (WebSocket-first transport)
- Tailscale IPv6 docs (fd7a:115c:a1e0::/48 ULA allocation)
- axum discussions #834, #1063 (dual-stack binding with [::])
