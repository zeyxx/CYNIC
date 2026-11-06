# CYNIC — Minimal Agent Protocol

CYNIC is a low-layer sovereign Rust runtime for multi-agent systems. No mutation without read-back proof.

## Gates & Build Policy

- **Builder SSOT**: Dynamic builder pool via `infra/registry.json` (`cynic-builder` 10.10.10.119, `cynic-compute-01` 192.168.0.25). Selected dynamically at runtime by `/capacity` pressure (lowest load / in_flight). NEVER run cargo/rustc locally on cortex (faulty RAM).
- **Canonical gates**: `cynic audit crate` (`make audit-crate`; `make check` is a deprecated alias), `cynic audit all` (`make audit`), `make release.deploy`.
- **Fast checks**: `RUSTFLAGS="-C debuginfo=0" cargo check -p <crate> --lib`, `cynic audit all --json`.
- Note: `make build` and `make build-release` do not exist.

## Authority SSOT

1. Observed runtime state
2. `contracts/foundation.json`
3. `infra/runtime-authority.json`
4. `infra/registry.json` (hosts, IPs, paths — never hardcode)
5. `Cargo.toml` workspace & crates under `crates/` (census in `infra/crate-topology.json`)

## Invariants

- **Canonical Vocabulary & Glossary**: Every concept, domain enum, and lifecycle stage must adhere strictly to `contracts/glossary.json`. Any observed jargon drift, homonym confusion, or misnamed domain concept must be seized as an opportunity to enrich `contracts/glossary.json` and canonicalize the code. Never invent unnormed terminology.

- **Bottom-Up over Top-Down**: Always observe, measure, and validate physical substrate reality (hardware, raw bytes, exit codes, machine receipts, AST) before introducing speculative, probabilistic, or higher-layer abstractions across all domains. No model or policy without an empirically verified baseline.
- **Strict DAG downward**: Higher layers depend on lower layers, never the reverse.
- **FCIS / Zero-I/O Core**: Pure logic in domain modules; side effects behind ports in `adapters/`.
- **Boundaries**: Dynamic registries over hardcoded values; typed argv execution; explicit `--yes`/`--apply` for mutation.
- **SSH Transport SSOT**: All SSH transport, command construction, batch options, and target validations MUST live exclusively in `crates/cynic-ssh`. Reimplementing SSH argv, bypasses, or raw `Command::new("ssh")` anywhere outside `cynic-ssh` is an architectural defect; an observation issue must be opened immediately upon detection.

## Git & Worktree Isolation

- **Forbidden**: `git add -A` and `git add .` are strictly prohibited. Stage explicit paths only.
- **Commit identity**: `-c user.name=<agent-id> -c user.email=<agent>@agent.local`.
- **Branches**: Work on `agent/<name>/<scope>`; never push to `main` directly.
- **One worktree per agent**: allocate it with `cynic source worktree --allocate agent/<name>/<scope>` and work only inside it. Never edit, switch branch, reset or cherry-pick in the primary checkout; other agents share it.
- **Ownership**: a worktree and its branch belong to the agent that allocated them. No other agent removes, prunes, resets or deletes them. Stale GC (`cynic source worktree --apply`) acts only on what the observation classifies as stale.
- **Cleanup at passage**: when the work is merged or abandoned, the agent that allocated the worktree removes it and deletes its branch (local and remote) in the same session. Leave nothing behind.
- **Main only through a PR**: never cherry-pick or merge old branches onto `main` outside a PR. Three commits that reached `main` without one (`9d3bf7d6`, `ad5ce4eb`, `6f371183`) broke the build for every agent.
- **Sovereign merges**: Forgejo is the sovereign store, GitHub the public projection. Merge only via `cynic source pr merge` on the sovereign side; never `gh pr merge` (it writes to the projection and splits the line — 2026-10-23). The hourly drift watch pages on divergence; converge before any further merge.
- **Never push to GitHub directly**: Direct `git push github ...` is strictly forbidden. The automated mirror handles replication from sovereign Forgejo to GitHub. Agents interact exclusively with Forgejo (`origin`).
- **Claims**: Scope claim before mutation; release with proof reference.
- **Upstream freshness (external projects)**: Before reading or modifying any external project (TrenchScanner, TokenSage, GASdf, XSCanner, etc.), always run `git fetch origin && git pull origin main` first. Local clones can be hundreds of commits behind. Stale code = false conclusions. Verify `git log --oneline -1` matches `origin/main` before any analysis.
- **No duplicate upstream logic**: Before implementing a feature, verify the upstream project doesn't already do it. Reimplementing on-chain discovery, rug screening, or scoring in a downstream project is a defect.
- **Trace data flows before coding**: Before writing code for a multi-project integration, map who sends what to whom (prefetch vs webhook, sync vs async). No code without a documented data flow.
- **Honest progress**: A component is "done" only when tested AND connected. Written but untested/unconnected code is "draft", not "done".

## Convergence Procedure (Git & Worktree Hygiene)

- **Methodology (project creation)**: Before writing code for a new project or feature, follow the sequence: Research → Whiteboard (architecture diagram + data flow doc) → Grill (stress-test the plan with adversarial questions) → PRD (formalize scope, IS/ISN'T, success metrics) → Build+QA (milestone by milestone, tested + connected) → Ship in public. Do not skip Whiteboard or Grill. No code without a documented data flow.

1. **Observe**: `cynic source git branch status --json` + `cynic source worktree --json`.
2. **Triate**: `cynic source git reconcile --json` → classify branches (ReadyForPr/NeedsRebase/Archive/GcWorktree).
3. **Act**:
   - Merge ready branches via `cynic source pr merge` (Forgejo sovereign).
   - Rebase stale branches onto main; resolve conflicts or abandon.
   - Archive dead branches (0 commits ahead): `cynic source git reconcile --apply`.
   - GC stale worktrees: `cynic source worktree --apply`.
4. **Verify**: `cynic audit all` must show:
   - `store_convergence.converged = true` (main == origin/main == github/main via automated mirror).
   - `branch_drift.orphan_count <= 50`.
   - `host_observed = true`.
5. **Never push to main or GitHub directly** — always via PR merge on sovereign side (Forgejo). Automated mirror replicates to GitHub.

### Gates
- `cynic audit crate` (`make audit-crate`) — crate-level audit.
- `cynic audit all` (`make audit`) — full audit including store convergence + drift watch.
- `make release.deploy` — deployment gate.

### Drift Watch
- Hourly drift watch pages on divergence between Forgejo main and GitHub main.
- Converge before any further merge.
- If `branch_drift.orphan_count > 50`, triage branches before new work.

## Language

- **Technical substrate**: 100% English (code, tests, docstrings, commits, schemas).
- **Operator dialogue**: 100% French.

## Operator Readability

- **Replies stay airy**: short sentences, one idea per line, lists over dense blocks, plain words (operator reads with compensated dyslexia).
- **Shape follows task**: execution → changed / verified / left; explanation → one idea per line, then examples; choice → numbered options with (R); error → likely cause first, then numbered steps; plan → short steps, one question at a time.

## Session Compass (advisory — not law, not a gate)

Orientation par défaut pour chaque session. Overridable par le jugement. Pas d'enforcement.
Voir `docs/plans/session-compass.md` pour le détail.

**Avant d'agir:** `cynic know search <pattern>` — vérifier si le pattern existe déjà.
**Avant de créer:** `cynic know search <concept>` — vérifier si le concept existe déjà.
**Quand tu hésites:** observe d'abord, mesure, puis agis.
