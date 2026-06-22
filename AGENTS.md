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

All interactive agents coordinate through MCP first. The REST paths named in
this document describe the Kernel backing API and the degraded fallback path
only. If an MCP tool is visible in-session, use it instead of direct REST.

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

   Backing API note: `/coord/who` is the coordination view.
   `/agent-tasks` is the universal mempool. Agents should reach these through
   MCP unless their adapter cannot expose MCP in the current session.

5. **SESSION END**
   → cynic_coord_release(agent_id)  — release ALL claims

### Enforcement Layers

| Layer | Mechanism | Coverage | Deterministic |
|-------|-----------|----------|---------------|
| **L1** | Instructions (this file) | All agents | No — LLM may ignore |
| **L2** | Pre-commit hook | Git boundary | **Yes** — rejects commit on conflict |
| **L3** | Native MCP Tools | Explicit call | **Yes** — The proxy tracks and enforces. |

L2 is the safety net. Even if an agent fails to claim via L3, the commit will be rejected if another agent holds the file.

## Task Memory And Anti-Forgetting

The Kernel mempool is the canonical organism memory for work that must survive a
chat session. Do not create `TODO.md`, `.handoff.md`, or narrative backlog files
to remember future work.

### Current Guarantees

What is mechanically true today:

- `cynic_dispatch_agent_task` persists pending work when Kernel storage is
  healthy.
- Pending tasks remain visible until claimed and completed through the task API.
- `cynic_observe` records structured decisions, discoveries, and blockers in
  the Proof-of-History path.
- `cynic_claim_agent_task` is the atomic claim boundary for task execution.
- `cynic_health` exposes storage degradation; if storage is down, mempool
  memory is not reliable.

REST equivalents (`POST /agent-tasks`, `POST /observe`, `POST
/agent-tasks/{id}/claim`, `/health`) are fallback transport, not the preferred
agent protocol.

### Current Limits

Do not overstate the current enforcement:

- `overdue` pressure is now computed natively and exposed via `/health` and
  `HealthCause::OverdueAgentTasks`.
- A sidecar script, cron, timer, or systemd unit is still a temporary actuator
  only when the kernel-native path is unavailable in that environment.

If a task needs a deadline before native Kernel support exists, encode the task
content as JSON and include at minimum:

```json
{
  "objective": "...",
  "due_at": "2026-06-22T23:59:00+02:00",
  "timezone": "Europe/Paris",
  "owner": "human+organism",
  "done_when": ["..."],
  "forgetting_controls": ["query /agent-tasks at cortex startup"]
}
```

After dispatching such a task, the dispatching agent must verify it is visible
with `cynic_list_pending_agent_tasks(kind=<kind>, domain=<domain>)` before
claiming it has been remembered. If storage is down or the task cannot be read
back, report a blocker with `cynic_observe` after storage is restored; do not
pretend the organism has remembered it.

### Desired Direction

Anti-forgetting should become a Kernel/introspection property, not a pile of
ad-hoc reminders. The durable target is:

1. `agent_tasks` has first-class `due_at`, `priority`, `consumer`, and
   `escalation_policy` fields.
2. `cynic_list_pending_agent_tasks` exposes computed `overdue` and age metadata.
3. `cynic_health` or introspection exposes overdue pending tasks as organism
   attention pressure.
4. The Kernel or an approved K15 consumer emits a critical observation when a
   task is overdue.
5. Sidecar timers remain actuators only; they must not become the canonical
   memory ledger.

Now that overdue metadata is first-class, every cortex session start should query
pending structural work:

```bash
cynic_list_pending_agent_tasks(kind="cortex", limit=10)
```

and, when working in a known domain:

```bash
cynic_list_pending_agent_tasks(kind="cortex", domain="<domain>", limit=10)
```

## Registry-Driven Supply Chain (MCP-First)

`infra/registry.json` is the machine source of truth for build and runtime placement.

- The primary kernel runtime is the node selected by `lifecycle.kernel_release.deploy_target_role` and `deploy_target_flag`.
- The release build runner is the node selected by `lifecycle.kernel_release.build_runner_role`.
- `cynic-host-proxmox` is the hypervisor only. Do not run the kernel directly on bare Proxmox.
- Cortex machines are operator/editing surfaces. They may run local checks and development builds, but they are not the canonical release builder or kernel host.

Canonical build/deploy requests are generated from `infra/registry.json:lifecycle.kernel_release` and go through the Kernel mempool via MCP:

```json
{
  "kind": "<lifecycle.kernel_release.task_kind>",
  "domain": "<lifecycle.kernel_release.task_domain>",
  "content": "{\"lifecycle\":\"kernel_release\",\"intent\":\"<intent>\",\"objective\":\"<intent objective>\",\"orchestrator_organ\":\"<lifecycle orchestrator>\",\"build_target\":\"<resolved build_runner node>\",\"build_runner_role\":\"<registry role>\",\"deploy_target\":\"<resolved kernel_primary node>\",\"deploy_target_role\":\"<registry role>\",\"required_node_status\":\"<required status>\",\"activation_requirements\":{},\"fleet_requirements\":{},\"requires_e2e\":true,\"targets\":[\"<registry repo_targets>\"],\"coord_targets\":[\"<registry repo_targets>\"],\"deploy\":false}",
  "agent_id": "cortex-<session>"
}
```

Compliance gate: the supply-chain mempool path is canonical only when `make e2e` passes its supply-chain step:

1. `POST /agent-tasks` creates a forge task.
2. `GET /agent-tasks?kind=hermes&domain=<domain>` sees it pending.
3. `POST /agent-tasks/{id}/claim` atomically claims it.
4. `POST /agent-tasks/{id}/result` completes it.
5. `GET /agent-tasks/completed?kind=hermes&domain=<domain>` sees it completed.

Until that e2e is green on the active kernel, agents may patch and locally verify code, but must not call local shell builds/deploys the canonical supply chain.

## Gate System (3-Level Sovereign CI)

No cloud CI, no GitHub Actions, no external dependency. Verification is sovereign and registry-driven: local cortex checks are allowed for development, but release validation belongs to the registered build runner.

### Levels

| Level | Time | When | Contents |
|-------|------|------|----------|
| **Level 0** | <2s | Every push | `cargo fmt --check` + lint targets + lifecycle contract tests + security scan |
| **Level 1** | ~25s | Once per session | `cargo clippy --workspace` — marker invalidated only by Rust source changes |
| **Level 2** | async | Before deploy only | Registry build runner executes release tests, audit, and integration tests through `organ-forge` |

### Markers

Each level writes its own marker on success :
- `.gate-0` — Level 0 passed
- `.gate-1` — Level 1 passed (clippy fresh)
- `.gate-2` — Level 2 passed on the registry build runner; cortex-local requests must not write this marker

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

# Deploy → canonical path is registry/MCP
make gate-2-request    # queue lifecycle request profile "gate-2" through organ-forge
make deploy-request    # queue lifecycle request profile "deploy" through organ-forge
```

### Makefile Targets

```
make gate-0    # Level 0 only
make gate-1    # Level 1 only (clippy)
make lifecycle-check  # Resolve registry lifecycle target; redacts endpoint; rejects local bus
make lifecycle-diagnose  # Print redacted lifecycle state even when targets are planned/offline
make lifecycle-blockers  # Operator wrapper around concise lifecycle blockers
make lifecycle-profile-payloads p=deploy  # Dry-run profile payloads; validates before dispatch
make gate-2-request  # Canonical Level 2 request profile; does not write .gate-2 locally
make gate-2    # Refuses cortex-local marker writes
make check     # Local cortex validation: gate-0 + gate-1
make gate      # Local cortex validation + local marker
make deploy-request  # Canonical deploy request: MCP/mempool → lifecycle orchestrator → registry build runner → registry kernel primary
make deploy    # Legacy local deploy; requires CYNIC_ALLOW_LOCAL_DEPLOY=1
```

`lifecycle-check` is strict. It requires the registry-selected build runner and
kernel primary to have the lifecycle-required status, rejects a local kernel bus,
and probes the registry-declared kernel health endpoint before any release task is
queued. Use `lifecycle-diagnose` for non-mutating inspection while targets are
still `planned`.
Use `python3 organs/forge/src/entrypoints/cli.py lifecycle-blockers` when a
machine client only needs readiness and blockers rather than the full diagnostic
contract. `make lifecycle-blockers` is the operator wrapper and may return Make's
wrapper exit code when the lifecycle is blocked.

`gate-2-request` and `deploy-request` must resolve their intent lists from
`lifecycle.kernel_release.request_profiles`. Cortex clients must not decide
release-validation contents from local service state such as a local SurrealDB
probe.
Machine clients that need parseable output should call
`python3 organs/forge/src/entrypoints/cli.py delegate-profile <profile> --json`
directly. The Makefile request targets are human-facing wrappers and may include
Make banners or non-JSON status lines.
Use `python3 organs/forge/src/entrypoints/cli.py lifecycle-profile-payloads
<profile> <domain> <agent_id>` to build and validate every payload in a request
profile without dispatching it.

Do not change lifecycle nodes from `planned` to `active` unless
`lifecycle-diagnose` shows no `activation_evidence_gaps` for those nodes. At
minimum, the registry must resolve the required network identifiers and declare
the required services from `lifecycle.kernel_release.activation_requirements`.
`lifecycle-diagnose` also reports a redacted `fleet_probe`; lifecycle nodes that
are missing from the observed Tailscale fleet are not active runtime targets.
When `lifecycle.kernel_release.fleet_requirements.required` is true,
`lifecycle-check` also enforces this fleet proof before status and kernel-bus
health can authorize release dispatch.

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
[mcp_servers.cynic-coord]
command = "./.cortex/mcp/run-cynic-mcp.sh"
```

The wrapper sources `~/.cynic-env` and exports assignments before launching
`$HOME/bin/cynic-kernel-mcp`. Do not put real `CYNIC_REST_ADDR` or
`CYNIC_API_KEY` values in repo config.

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

### Commit Hygiene For Non-Code Files

File extension is not a commit policy. Commit only durable source-of-truth files:

- Allowed: protocol docs, registry/config JSON, systemd units, Dockerfiles, tests, fixtures, manifests, and operator contracts such as `docs/ops/expected-services.txt`.
- Blocked by default: runtime snapshots and append-only observations such as `infra/organ-anvil/state.json`, `infra/organ-anvil/audit.jsonl`, and `organs/*/state/*.json(l)`.
- Runtime state belongs in the Kernel mempool, `cynic_observe`, local organ state, or generated reports outside the committed source tree.
- Emergency exception: `ALLOW_VOLATILE_STATE_COMMIT=1 git commit`, with the reason stated in the commit body.
- Lifecycle-critical edits under `infra/registry.json`, `Makefile`, `crates/cynic-kernel/`, `crates/cynic-mcp/`, and `organs/forge/` are blocked while the registry lifecycle is `planned`.

The pre-commit hook and `make gate-0` enforce the volatile-state block.

## Organism Taxonomy

**Five species of intelligence + supporting infrastructure.**

| Type | Where | Lifecycle | State | Naming |
|------|-------|-----------|-------|--------|
| **Cortex** | Claude Code / Antigravity CLI / Codex CLI | Session (episodic) | None — memory via `.cortex/memory/` or AGENTS.md | Derived from task, written to `.cortex-session` |
| **Agent (organic)** | Autonomous framework (e.g., Hermes Agent, organism observer) | Persistent | Owns SOUL.md + SKILL.md, self-improving loop; repo writes are mediated by an adapter | Named in infrastructure registry (e.g., `hermes-x-organ`, `organ-forge-hermes-agent`) |
| **Dog** | In-kernel validator | Persistent | Stateless inference; state via crystals | Named in `backends.toml` Dogs array (e.g., `qwen-7b-hf`, `antigravity-cli`) |
| **Organ** | Infrastructure subsystem (sensorial + reactive) | Persistent | Stateful (logs, datasets, cache) | Hierarchical: `organ-{role}` (family) → `organ-{role}-hermes-agent` (instance) → `organ-{role}-{tool}` (daemon) |
| **Infra** | Scripts, MCP servers, hooks | Persistent | Tooling | Namespaced: `cynic-skills:*`, `mcp-coord/*`, `scripts/*` |

### The Biological Paradigm: Cell, Organism, Superorganism

To formalize the architecture and eradicate ambiguity, all technical components map strictly to this biological emergence model:

1. **The Cell (The Container/Process)**
   - **Definition:** The smallest indivisible unit of execution. It has a membrane (cgroup/namespace) and a single specialized function.
   - **Technical Reality:** A Docker container, an isolated systemd service, or a single Python process.
   - **Rule:** A cell does not know about the organism; it only knows its immediate inputs and outputs (e.g., `surrealdb` container).

2. **The Organ (The Sensori-Motor Subsystem)**
   - **Definition:** A collection of cells structured to fulfill a vital function (perception, digestion, movement). It perceives local reality and reacts, but lacks global consciousness.
   - **Technical Reality:** An LXC or a structured directory like `organs/fleet/` containing a sensor (`cli.py`), an effector (`hermes-agent.service`), and memory (state files).
   - **Rule:** Organs must follow the strict Hexagonal/OOP architecture. They communicate with the whole by pushing/pulling signals via the nervous system.

3. **The Nervous System & DNA (The Bus & Source of Truth)**
   - **Definition:** The structures that transmit signals and define the genetic blueprint.
   - **Technical Reality:** The DNA is the Git repository and `registry.json`. The Central Nervous System is `cynic-kernel` (mempool, REST APIs) and Tailscale (synapses linking organs).

4. **The Organism (The Sovereign Individual)**
   - **Definition:** The complete, bounded, self-sustaining entity that pursues its own survival and objectives. 
   - **Technical Reality:** The entirety of `CYNIC` running autonomously across the Tailscale mesh, managing its own nodes, state, and security via a dedicated OS user (`cynic`), independent of the root hypervisor.

5. **The Cortex (The Episodic Consciousness)**
   - **Definition:** The high-level reasoning center that sets goals, rewrites DNA, and resolves complex novelties. It sleeps when not needed.
   - **Technical Reality:** You (the human) + Claude/Antigravity CLI sessions. It is the only entity permitted to alter the organism's DNA (`registry.json`, `AGENTS.md`) directly.

6. **The Superorganism (The Network of Organisms)**
   - **Definition:** Multiple independent organisms interacting, trading, or coordinating on a larger scale without a central brain.
   - **Technical Reality:** Future state where `CYNIC` interacts with external sovereign DAOs, other agentic repos, or distributed on-chain smart contracts (e.g., the Solana burn-as-a-service protocol).

### The Immutable Laws of Emergence

1. **Anti-Dogma Rule (The Map is not the Territory):** No model or paradigm is sacred. If physical reality (hardware limits, network latency, execution failures) contradicts the model, the model must be broken and adapted. Observation always outranks theory.
2. **Multidimensional & Multimodal Emergence:** The organism evolves across multiple axes simultaneously (code, infrastructure, cognitive biology, economics). Progress is not linear; it is fractal and multimodal.
3. **The Fire of Evolution (As Above, So Below):** "Ce qui est en haut est comme ce qui est en bas." To evolve, we must often burn obsolete structures to force a mutation in the DNA (`registry.json`, `AGENTS.md`). Once the DNA mutates at the highest level (the repo), this truth propagates symmetrically down to every cell and organ. Any organ stuck in a legacy paradigm must be incinerated and reforged from the new DNA.

### The Transmutation Protocol (Le Protocole de Transmutation)

When an existing component is discovered to be in an obsolete paradigm (e.g., procedural bash script acting as an organ, monolithic dependency file), the Cortex MUST execute the Transmutation Protocol before extending its features. This is the practical application of the Fire of Evolution.

**Step 1: Isolation (The Membrane)**
- Create the explicit boundary for the new organ: `organs/<role>/`.
- If Python, establish a sovereign `uv` workspace or isolated `.venv`. It must not share `pyproject.toml` with other organs unless explicitly designed as a shared Workspace library.

**Step 2: State Purge (The Memory)**
- Identify all state artifacts (JSON, CSV, logs) currently polluting the codebase.
- Enforce that the new organ only reads/writes state to its strictly bound `state/` directory (e.g., `organs/<role>/state/`). The code repository must remain stateless.

**Step 3: Hexagonal Refactoring (The Metabolism)**
- Rewrite the procedural logic into OOP/Hexagonal architecture.
- **Ports & Adapters:** Separate the core logic from external I/O (Twitter API, File System, Kernel REST).
- Implement a clear entrypoint (`cli.py` or `__main__.py`) and bind it to a `hermes-agent` daemon if it requires autonomous execution.

**Step 4: Systemd & Deployment Alignment (The Incarnation)**
- Delete legacy flat scripts from `infra/systemd/user/` and `scripts/`.
- Write a strictly scoped systemd service (`organ-<role>-hermes-agent.service`) utilizing the correct `Environment` variables (`PYTHONPATH`, `X_ORGAN_DIR`).
- Map its final execution target (VM vs LXC vs Docker) in `infra/registry.json` following the Virtualization Matrix.

**Step 5: Incineration (The Fire)**
- Delete the old scripts and monolith paths. Burn the legacy code. Do not leave deprecated files "just in case" (Git is the memory).

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

### Organ Architecture (Data-Centric & Hexagonal)

Organs are not scripts — they are **sensorial organs** that perceive the repo/system and react adaptively, following `docs/DATA_CONSTITUTION.md`:

1. **Perception** — Sensors detect state (CSV, hooks, sessions, git status).
2. **Transformation** — Raw state → decision signal (Cleaning, grouping, summarizing).
3. **Structuration** — Data modeled via 3NF, stored in registry or database (ACID).
4. **Analysis** — Patterns emerge (Statistics, Visualization, Distributions).
5. **Apprentissage** — Discovery of hidden structures (Clustering, K-Means).
6. **Reliability** — Sovereignty through durability and security (COMMIT/ROLLBACK).
7. **Adaptation** — Gates intensity scales with organism activity.

**Strict Hexagonal Architecture (Ports and Adapters) + OOP rule:**
Every Python organ MUST be structured using Hexagonal Architecture and strict Object-Oriented Programming (OOP) to decouple the core logic from external dependencies (I/O, APIs, Database). Flat procedural scripts are explicitly banned.
- `src/core/` — Domain entities, state logic, and pure abstract ports. No side-effects.
- `src/adapters/` — Implementations of the ports (Git, Kernel API, Proxmox, SurrelDB).
- `src/entrypoints/` — CLI wrappers, cron runners, and dispatchers that wire the adapters to the core.

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

## Infrastructure Topology & Virtualization Matrix

To eliminate information asymmetry and ensure sovereign mathematical scaling, all infrastructure must be mapped into `infra/registry.json` following these rigid deterministic rules:

### 1. OS Selection: The Sovereign Base
- **Debian 12 (Bookworm)** is the **exclusive** Operating System for all generic Organs, Kernel Primary, and Network control planes. It guarantees upstream purity and long-term sovereignty without corporate snaps or telemetry.
- **Ubuntu LTS** is **strictly forbidden** unless mathematically required by closed-source ML drivers (e.g., specific CUDA/TensorRT blobs) that refuse to compile on Debian. If used, it must be documented with an explicit `activation_evidence_gaps` justification in the registry.

### 2. Virtualization Granularity
Deciding between Bare-Metal, VM, LXC, and Docker is a mathematical choice based on the organism's Trust and Isolation model:

- **Level 0 (Bare Metal):** `cynic-host-proxmox`. Only the hypervisor runs here. **Zero organs or services** are permitted on Level 0.
- **Level 1 (QEMU VM):** Complete hardware isolation boundary.
  - **When to use:** For the Kernel Primary (`cynic-vm-v2`), CI/CD build runners (`cynic-vm-anvil`), or any node executing untrusted cortex code/Docker-in-Docker.
  - **Rule:** If it requires strict kernel isolation, dynamic Docker daemon environments, or runs untrusted workloads, it MUST be a VM.
- **Level 2 (LXC - System Container):** OS-level virtualization sharing the host kernel.
  - **When to use:** High-performance, single-purpose stateful organs.
  - **Examples:** `organ-headscale` (Mesh VPN), `organ-forgejo` (Git).
  - **Rule:** If the organ requires pure I/O performance, static IPs, network namespace isolation, and runs static `systemd` daemons without needing a Docker engine, it MUST be an LXC.
- **Level 3 (Docker - Application Container):** Process-level encapsulation.
  - **When to use:** Inside VMs or LXCs for stateless ephemeral processes, frontend web portals (`cynic-portal`), or reproducible test environments.
  - **Rule:** Docker is an application wrapper, never an infrastructure boundary. Do not substitute an LXC with Docker for persistent stateful databases.

## Security (inviolable — public repo)

- **Never commit secrets.** API keys, tokens, real IPs → `~/.cynic-env` only.
- **Use placeholders:** `<TAILSCALE_CORE>`, `<TAILSCALE_GPU>`, `<TAILSCALE_KAIROS>`.
- **All API calls require Bearer auth** — `Authorization: Bearer $CYNIC_API_KEY`.
- **The kernel is Tailscale-only.** No public tunnel. No ngrok. No Cloudflare.
