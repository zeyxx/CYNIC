# CYNIC — Multi-Agent Coordination

Three cortex agents, one kernel, one repo. The kernel is the coordination bus.

## Agents

| Agent | CLI | Instructions | MCP Config | L3 Enforcement |
|-------|-----|-------------|------------|----------------|
| Claude Code | `claude` | `CLAUDE.md` | `.claude/settings.json` | Hooks (PreToolUse → coord-claim.sh) |
| Gemini CLI | `gemini` | `GEMINI.md` | `.gemini/settings.json` | Hooks (BeforeTool → coord-claim.sh) |
| Codex CLI | `codex` | `AGENTS.md` (this file) | `.codex/config.toml` | MCP proxy (apply_patch lacks hooks — [#16732](https://github.com/openai/codex/issues/16732)) |

## The Kernel Is the Bus

The CYNIC kernel runs on `${CYNIC_REST_ADDR}` (Tailscale). All agents access it — Claude via hooks, Gemini/Codex via the `cynic-coord` MCP proxy (`mcp-coord/cynic-coord`).

## Document Hierarchy

Use this order when two files appear to disagree:

1. `AGENTS.md`
   Shared repo-wide protocol. This file is the stable source of truth for session lifecycle, claims, handoff semantics, and security rules.
2. Agent adapters
   - `CLAUDE.md`
   - `GEMINI.md`
   - `.codex/CODEX-PROTOCOL.md`
   These explain how each client executes the shared protocol and what automation or fallback paths it really has.
3. `.handoff.md`
   Append-only session journal. Useful context, but non-normative by design. Older entries may be superseded by newer reality.
4. Roadmap / backlog files
   `TODO.md` is the live work ledger. Historical sprint snapshots such as `TODO-ROBUSTNESS.md` and `docs/TODO.md` are not protocol sources.

Rule: protocol beats journal, and live ledger beats historical snapshot.

### MCP Coordination Tools

All agents have these tools via MCP:

| Tool | What | When |
|------|------|------|
| `cynic_coord_register` | Register yourself with the kernel | Session start |
| `cynic_coord_claim` | Claim a file before editing | Before every file write |
| `cynic_coord_who` | See active agents and claims | Before starting work |
| `cynic_coord_release` | Release your claims | Session end or done with file |
| `cynic_observe` | Record a discovery/decision/blocker | When something material happens |
| `cynic_health` | Check kernel health | When in doubt |
| `cynic_handoff` | Read/write `.handoff.md` | Session start (read) and end (append) |

### Coordination Protocol

```
1. SESSION START
   → cynic_coord_register(agent_id, intent, agent_type)
   → cynic_handoff(action="read")  — load context from previous sessions
   → cynic_coord_who()             — see who else is working

2. BEFORE EDITING A FILE
   → cynic_coord_claim(agent_id, target)
   → If CONFLICT: STOP. Check cynic_coord_who(). Coordinate.

3. DURING SESSION
   → cynic_observe() on discoveries, decisions, blockers
   → cynic_coord_who() periodically if session is long

4. SESSION END
   → cynic_handoff(action="append", message="what I did, what's next")
   → cynic_coord_release(agent_id)  — release ALL claims
```

### Enforcement Layers

| Layer | Mechanism | Coverage | Deterministic |
|-------|-----------|----------|---------------|
| **L1** | Instructions (this file) | All agents | No — LLM may ignore |
| **L2** | Pre-commit hook | Git boundary | **Yes** — rejects commit on conflict |
| **L3** | Per-agent hooks | File write time | **Yes** for Claude/Gemini. MCP-dependent for Codex. |

L2 is the safety net. Even if an agent skips L1 and L3, the commit will be rejected if another agent holds the file.

## Codex CLI — Specific Instructions

### Environment

Source `~/.cynic-env` before launching Codex, or configure env forwarding:
```bash
source ~/.cynic-env && codex
```

### MCP Setup

The `cynic-coord` MCP server is configured in `.codex/config.toml`:
```toml
[mcp.cynic-coord]
type = "stdio"
command = ["./mcp-coord/cynic-coord"]
env = { CYNIC_REST_ADDR = "<TAILSCALE_CORE>:3030", CYNIC_API_KEY = "..." }
```

### Sandbox

For coordination to work, Codex needs network access to the kernel:
- `sandbox_mode = "full-access"` — or ensure Tailscale IP is reachable from sandbox

### Workflow for Codex

1. At session start: call `cynic_coord_register` with your agent ID and intent
2. Before editing any file in `cynic-kernel/`: call `cynic_coord_claim`
3. If you get CONFLICT: do NOT proceed. Call `cynic_coord_who` to see who has the file
4. Record material discoveries with `cynic_observe`
5. At session end: call `cynic_handoff(action="append")` with a summary, then `cynic_coord_release`

## Handoff Protocol

`.handoff.md` is the semantic context bus — what was discovered, what's hot, what's next. The kernel coord system handles file-level mutex; handoff carries meaning.

Important: `.handoff.md` is a journal, not a spec. Do not treat old entries as stable protocol. If a handoff entry conflicts with this file or an adapter file, the handoff entry is historical context only.

Format:
```markdown
## [agent-id] 2026-04-15T18:30:00Z
STATUS: what I did
TOUCHED: files modified
NEXT: what should happen next
BLOCKS: any blockers
FOR_OTHER_AGENT: specific messages
```

## Organism Taxonomy

Three species of intelligence + supporting infrastructure:

| Type | Where | Lifecycle | State | Naming |
|------|-------|-----------|-------|--------|
| **Cortex** | Claude Code / Gemini CLI | Session (episodic) | None — memory via `.claude/memory/` or AGENTS.md | Derived from task, written to `.cortex-session` |
| **Agent (organic)** | Autonomous framework (e.g., Hermes Agent, organism observer) | Persistent | Owns SOUL.md + SKILL.md, self-improving loop | Named in infrastructure registry (e.g., `hermes-x-organ`, `meta-cortex-observer`) |
| **Dog** | In-kernel validator | Persistent | Stateless inference; state via crystals | Named in `backends.toml` Dogs array (e.g., `qwen-7b-hf`, `gemini-cli`) |
| **Organ** | Infrastructure subsystem (passive or active) | Persistent | Stateful (logs, datasets, cache) | Hierarchical: `organ-X` (family) → `organ-X-hermes-agent` (agent) → `organ-X-mitmproxy` (daemon) |
| **Infra** | Scripts, MCP servers, hooks | Persistent | Tooling | Namespaced: `cynic-skills:*`, `mcp-coord/*`, `scripts/*` |

### Cortex Naming Convention

1. **Derive from conversation theme** — not arbitrary
   - Good: `cortex-phi-calibration`, `cortex-wallet-judgment`, `cortex-organ-x-rework`
   - Bad: `cortex-1`, `cortex-session-3`

2. **Write to `.cortex-session`** — single identity point
   ```bash
   echo "wallet-judgment" > .cortex-session
   ```
   This file propagates to:
   - Git branch name (`git branch -l | grep $(cat .cortex-session)`)
   - Handoff entry signature (`## [cortex-wallet-judgment] 2026-04-27T...`)
   - Coord claims (`cynic_coord_claim` tagged with cortex ID)

3. **Branch-per-cortex mandatory** — prevents HEAD collision
   ```bash
   git checkout -b feat/$(cat .cortex-session)-$(date +%Y-%m-%d)
   # Example: feat/wallet-judgment-2026-04-27
   ```
   - Never push directly to main
   - Always PR from cortex branch
   - If another cortex is on main, branch name disambiguates blame

### Multi-Cortex Coordination Protocol

**Problem 1: Git isolation** — two cortex sessions both editing main → detached HEAD, orphaned work.
**Problem 2: Task duplication** — human dispatch must partition work or cortices execute redundant tasks.

**Solution (Level 0: Convention-Only, no locks):**

1. **At session start:**
   - Derive cortex name from your task (e.g., "wallet judgment")
   - Write to `.cortex-session`
   - Create a branch: `git checkout -b <type>/<cortex-session>-YYYY-MM-DD`
   - Confirm with `cynic_coord_who()` — you should be the only cortex on that branch

2. **Human dispatch:**
   - Partition tasks: "Claude handles AGENTS.md § Taxonomy, Gemini handles Hermes Agent rework"
   - Post in #cynic with explicit scope per cortex
   - Prevents redundant work on same file

3. **If collision detected** (another cortex on your branch or same file):
   - Call `cynic_coord_who()` to see who holds the claim
   - Negotiate via #cynic: "Can you pause on AGENTS.md while I finish § Taxonomy?"
   - Do NOT force-push or reset --hard without coordination

4. **At session end:**
   - Append to `.handoff.md` with cortex ID + timestamp
   - Run `cynic_coord_release()` to free all claims
   - Leave `.cortex-session` file in-tree (useful for tracking which cortex touched what)

**Escalation to Level 1 (locking):**
- Measure collision rate over 5 sessions
- If collisions > 1 per session: escalate to distributed lock (Redis/SurrealDB-based `cynic_coord_lock`)
- Until then: convention + human coordination is sufficient

## Security (inviolable — public repo)

- **Never commit secrets.** API keys, tokens, real IPs → `~/.cynic-env` only.
- **Use placeholders:** `<TAILSCALE_CORE>`, `<TAILSCALE_GPU>`, `<TAILSCALE_KAIROS>`.
- **All API calls require Bearer auth** — `Authorization: Bearer $CYNIC_API_KEY`.
- **The kernel is Tailscale-only.** No public tunnel. No ngrok. No Cloudflare.
