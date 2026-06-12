# CYNIC — Multi-Agent Coordination

Three cortex agents, persistent organic agents, one kernel, one repo. The kernel is the coordination bus.

## Agents

| Agent | Runtime | Instructions | Coordination Config | L3 Enforcement |
|-------|---------|--------------|---------------------|----------------|
| Claude Code | `claude` | `CLAUDE.md` | `.claude/settings.json` | Explicit MCP (`cynic_coord_claim`) |
| Antigravity CLI | `antigravity` | `ANTIGRAVITY.md` | `.antigravity/settings.json` | Explicit MCP (`cynic_coord_claim`) |
| Codex CLI | `codex` | `CODEX.md` + `.codex/CODEX-PROTOCOL.md` | `.codex/config.toml` | Explicit MCP (`cynic_coord_claim`) |
| Hermes Agent | `hermes-agent-executor.service` → `hermes chat` | `HERMES_AGENT.md` + organ `SKILL.md` | REST `/coord/*` from executor | Executor claims declared task targets before launch |

## The Kernel Is the Bus

The CYNIC kernel runs on `${CYNIC_REST_ADDR}` (Tailscale). Agents access it through MCP or direct REST. Claude/Antigravity/Codex may use MCP when available; Hermes Agent uses REST from its executor because it is a persistent systemd service, not an interactive cortex.

## Document Hierarchy

Use this order when two files appear to disagree:

1. `AGENTS.md`
   Shared repo-wide protocol. This file is the stable source of truth for session lifecycle, claims, handoff semantics, and security rules.
2. Agent adapters
   - `CLAUDE.md`
   - `ANTIGRAVITY.md`
   - `CODEX.md`
   - `.codex/CODEX-PROTOCOL.md`
   - `docs/organs/HERMES_AGENT.md`
   These explain how each runtime executes the shared protocol.

Rule: Protocol beats journal, and dynamic Kernel state beats static files.

### MCP Coordination Tools

All agents have these tools via MCP:

| Tool | What | When |
|------|------|------|
| `cynic_coord_register` | Register yourself with the kernel | Session start |
| `cynic_coord_claim` | Claim a file before editing | Before every file write |
| `cynic_coord_who` | See active agents and claims (`GET /coord/who`) | Before starting work |
| `cynic_coord_release` | Release your claims | Session end or done with file |
| `cynic_observe` | Record a discovery/decision/blocker | When something material happens |
| `cynic_dispatch_agent_task` | Create a new task in the Kernel mempool | When delegating work to organs (`kind="hermes"`) or logging structural work (`kind="cortex"`) |
| `cynic_list_pending_agent_tasks` | Read pending tasks from the Kernel mempool | At session start, querying `kind="cortex"` to see if human has pending structural tasks |
| `cynic_claim_agent_task` | Atomically claim a task for execution | After reading pending tasks and deciding to execute one |
| `cynic_update_agent_task_result` | Mark a task as completed or failed | After finishing a claimed task |
| `cynic_health` | Check kernel health | When in doubt |
### Coordination Protocol

1. **JSON is King:** All machine-readable reports, discoveries, and signal extractions MUST be in JSON/JSONL.
2. **SESSION START**
   → cynic_coord_register(agent_id, intent, agent_type)
   → cynic_coord_who()             — see who else is working

3. **BEFORE EDITING A FILE**
   → cynic_coord_claim(agent_id, target)
   → If CONFLICT: STOP. Check cynic_coord_who(). Coordinate.

4. **DURING SESSION**
   → cynic_observe() on discoveries, decisions, blockers
   → cynic_coord_who() periodically if session is long

   REST note: `/coord/who` is the coordination view. `/agent-tasks` is the universal mempool.

5. **SESSION END**
   → cynic_coord_release(agent_id)  — release ALL claims

### Enforcement Layers

| Layer | Mechanism | Coverage | Deterministic |
|-------|-----------|----------|---------------|
| **L1** | Instructions (this file) | All agents | No — LLM may ignore |
| **L2** | Pre-commit hook | Git boundary | **Yes** — rejects commit on conflict |
| **L3** | Native MCP Tools | Explicit call | **Yes** — The proxy tracks and enforces. |

L2 is the safety net. Even if an agent fails to claim via L3, the commit will be rejected if another agent holds the file.

## Gate System (3-Level Sovereign CI)

No cloud CI, no GitHub Actions, no external dependency. The machine that pushes is the machine that verifies.

### Levels

| Level | Time | When | Contents |
|-------|------|------|----------|
| **Level 0** | <2s | Every push | `cargo fmt --check` + 6 lint targets (grep) + security scan |
| **Level 1** | ~25s | Once per session | `cargo clippy --workspace` — marker invalidated only by Rust source changes |
| **Level 2** | ~1m30s | Before deploy only | `cargo test --lib --release` + `cargo audit` + integration tests |

### Markers

Each level writes its own marker on success :
- `.gate-0` — Level 0 passed
- `.gate-1` — Level 1 passed (clippy fresh)
- `.gate-2` — Level 2 passed (tests fresh)

Markers are in `.gitignore` — they are not committed. They persist on disk between pushes.

### Typical Workflow

```
# Light change (.md or config) → push
git push
# → Level 0 (<2s) + Level 1 skip = ~2s

# Rust source change → push (first time)
git push
# → Level 0 (<2s) + Level 1 (~25s) = ~27s

# Next push without Rust changes
git push
# → Level 0 (<2s) + Level 1 skip = ~2s

# Deploy → requires Level 2
make gate-2    # ~1m30s
make deploy    # verifies gate-2 fresh → ship → restart → healthcheck
```

### Makefile Targets

```
make gate-0    # Level 0 only
make gate-1    # Level 1 only (clippy)
make gate-2    # Level 2 only (tests + audit + integration)
make check     # All 3 levels (backward compatible)
make gate      # All 3 levels + markers written
make deploy    # Verifies gate-2 fresh → ship → backup → deploy → verify
```

### Leeroy Jenkins Fallback

When the gate itself is broken (SSH timeout, pipe broken, Makefile corruption) :

```
PUSH_FORCE=1 git push
```

Bypasses all gates. Every bypass is logged to `.leeroy-jenkins/log.txt` for post-mortem. Use only for gate infrastructure failures — not to skip legitimate validation failures.

### Pre-Push vs Pre-Commit

| Hook | Runs when | What it does |
|------|-----------|-------------|
| **Pre-commit** | `git commit` | Coord conflict check + security scan + Cargo.lock coherence + `cargo fmt --check` |
| **Pre-push** | `git push` | Level 0 (always) + Level 1 (if Rust changed) + notes Level 2 status |

Pre-commit blocks bad commits. Pre-push blocks bad pushes. They complement each other — pre-commit is fast (<2s), pre-push is smarter (markers).

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
command = ["./.cortex/mcp/run-cynic-mcp.sh"]
env = { CYNIC_REST_ADDR = "<TAILSCALE_CORE>:3030", CYNIC_API_KEY = "..." }
```

### Sandbox

For coordination to work, Codex needs network access to the kernel:
- `sandbox_mode = "full-access"` — or ensure Tailscale IP is reachable from sandbox

### Workflow for Codex

Read `CODEX.md` first, then `.codex/CODEX-PROTOCOL.md`. Codex has no guaranteed automatic write hook, so treat coordination as manual unless MCP tools are visibly available in the session.

1. At session start: call `cynic_coord_register` with your agent ID and intent.
2. Before editing any file: call `cynic_coord_claim` or REST `/coord/claim`.
3. If the response includes conflicts: do NOT proceed. Call `cynic_coord_who` / `GET /coord/who`.
4. Record material discoveries with `cynic_observe`.
5. At session end: call `cynic_coord_release`.

## Hermes Agent — Specific Instructions

Hermes Agent is an organic persistent agent, not a cortex CLI. Read `HERMES_AGENT.md` for the adapter contract. The executor must own repo coordination before launching `hermes chat`.

Required behavior for repo-affecting Hermes tasks:

1. Task declares `targets`, `repo_targets`, `coord_targets`, or `files`.
2. Executor registers `hermes-agent-{domain}-{task_id}` with `/coord/register`.
3. Executor claims all declared targets with `/coord/claim-batch`.
4. Any returned conflict blocks the task, even if the kernel records claims as signals.
5. Executor releases claims with `/coord/release` in a `finally` path.

A repo-affecting Hermes task without declared targets must fail or requeue before `hermes chat` starts.

## Organic Governance (Zero-Markdown Ledger)

The organism relies on an **Agent-Agnostic, Zero-Markdown** governance model.
There is no `TODO.md` and no `.handoff.md`.

1. **The Code is the Truth**: The current state of the repo, the Git commit history, and the files (`docs/DATA_CONSTITUTION.md`, etc.) are the only static context.
2. **The Kernel is the Mempool**: Any dynamic context, discoveries, or tasks are pushed to the Kernel via `cynic_observe` or queried from `/agent-tasks`.
3. **Askesis (Proof-of-History)**: Human and agent interactions log their semantic footprints in structured JSONL (`.cynic/memory/logs/human-kernel.jsonl`), never in narrative markdown files.

## Organism Taxonomy

**Five species of intelligence + supporting infrastructure.**

| Type | Where | Lifecycle | State | Naming |
|------|-------|-----------|-------|--------|
| **Cortex** | Claude Code / Antigravity CLI / Codex CLI | Session (episodic) | None — memory via `.cortex/memory/` or AGENTS.md | Derived from task, written to `.cortex-session` |
| **Agent (organic)** | Autonomous framework (e.g., Hermes Agent, organism observer) | Persistent | Owns SOUL.md + SKILL.md, self-improving loop; repo writes are mediated by an adapter | Named in infrastructure registry (e.g., `hermes-x-organ`, `organ-forge-hermes-agent`) |
| **Dog** | In-kernel validator | Persistent | Stateless inference; state via crystals | Named in `backends.toml` Dogs array (e.g., `qwen-7b-hf`, `antigravity-cli`) |
| **Organ** | Infrastructure subsystem (sensorial + reactive) | Persistent | Stateful (logs, datasets, cache) | Hierarchical: `organ-{role}` (family) → `organ-{role}-hermes-agent` (instance) → `organ-{role}-{tool}` (daemon) |
| **Infra** | Scripts, MCP servers, hooks | Persistent | Tooling | Namespaced: `cynic-skills:*`, `mcp-coord/*`, `scripts/*` |

### Organ Naming Convention

1. **Role-based naming** — not arbitrary
   - `organ-forge` — Repo lifecycle manager (perception + reactive)
   - `organ-keep` — Backup + persistence (durability)
   - `organ-twitter` — Twitter/X data pipeline (collection)
   - `organ-{role}` — Reflects the organ's function in the organism

2. **Instance naming** — agent + tool
   - `organ-forge-hermes-agent` — The Hermes instance running anvil
   - `organ-forge-cron` — The cron job scheduling anvil
   - `organ-keep-systemd` — The systemd service running keep

3. **Registry entry** — `infra/registry.json`
   ```json
   {
     "organs": [
       {
         "name": "organ-forge",
         "instance": "organ-forge-hermes-agent",
         "role": "repo_lifecycle_manager",
         "status": "active"
       }
     ]
   }
   ```

### Organ Architecture (Data-Centric)

Organs are not scripts — they are **sensorial organs** that perceive the repo/system and react adaptively, following `docs/DATA_CONSTITUTION.md`:

1. **Perception** — Sensors detect state (CSV, hooks, sessions, git status).
2. **Transformation** — Raw state → decision signal (Cleaning, grouping, summarizing).
3. **Structuration** — Data modeled via 3NF, stored in registry or database (ACID).
4. **Analysis** — Patterns emerge (Statistics, Visualization, Distributions).
5. **Apprentissage** — Discovery of hidden structures (Clustering, K-Means).
6. **Reliability** — Sovereignty through durability and security (COMMIT/ROLLBACK).
7. **Adaptation** — Gates intensity scales with organism activity.


**See** `organs/forge/README.md` for full architecture.

### Cortex Naming Convention

1. **Derive from conversation theme** — not arbitrary
   - Good: `cortex-phi-calibration`, `cortex-wallet-judgment`, `cortex-organ-twitter-rework`
   - Bad: `cortex-1`, `cortex-session-3`

2. **Write to `.cortex-session`** — single identity point
   ```bash
   echo "wallet-judgment" > .cortex-session
   ```
   This file propagates to:
   - Git branch name (`git branch -l | grep $(cat .cortex-session)`)
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
   - Partition tasks: "Claude handles AGENTS.md § Taxonomy, Antigravity handles Hermes Agent rework"
   - Post in #cynic with explicit scope per cortex
   - Prevents redundant work on same file

3. **If collision detected** (another cortex on your branch or same file):
   - Call `cynic_coord_who()` to see who holds the claim
   - Negotiate via #cynic: "Can you pause on AGENTS.md while I finish § Taxonomy?"
   - Do NOT force-push or reset --hard without coordination

4. **At session end:**
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
