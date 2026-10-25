# CYNIC — Minimal Agent Protocol

CYNIC is a low-layer sovereign Rust runtime for multi-agent systems. No mutation without read-back proof.

## Gates & Build Policy

- **Builder SSOT**: `10.10.10.119`. NEVER run cargo/rustc locally on cortex (faulty RAM).
- **Canonical gates**: `cynic audit crate` (`make audit-crate`; `make check` is a deprecated alias), `cynic audit all` (`make audit`), `make release.deploy`.
- **Fast checks**: `RUSTFLAGS="-C debuginfo=0" cargo check -p <crate> --lib`, `cynic audit all --json`.
- Note: `make build` and `make build-release` do not exist.

## Authority SSOT

1. Observed runtime state
2. `contracts/foundation.json`
3. `infra/runtime-authority.json`
4. `infra/registry.json` (hosts, IPs, paths — never hardcode)
5. `Cargo.toml` workspace & crates under `crates/` (census in `infra/crate-topology.json`)

## Policy Chain

- Applied entry: this file. Every agent reads it.
- Citizen law: `contracts/agent-policy.json` (`cynic.agent_policy.v1`, versioned).
  Prose lead: `docs/specs/agent-constitution.md` §12.
- Role capabilities: `infra/agent-roles.json` (machine-read; enforced by Rust where loaded).
- One statement, one home. Pointers, never copies.

- **Facts dictate**: never ask the operator what data already decides; act, report the fact-to-action chain. Canonical: `contracts/agent-policy.json`. Full routing: constitution §12.

## Invariants

- **Bottom-Up over Top-Down**: Always observe, measure, and validate physical substrate reality (hardware, raw bytes, exit codes, machine receipts, AST) before introducing speculative, probabilistic, or higher-layer abstractions across all domains. No model or policy without an empirically verified baseline.
- **Strict DAG downward**: Higher layers depend on lower layers, never the reverse.
- **FCIS / Zero-I/O Core**: Pure logic in domain modules; side effects behind ports in `adapters/`.
- **Boundaries**: Dynamic registries over hardcoded values; typed argv execution; explicit `--yes`/`--apply` for mutation.

## Git & Worktree Isolation

- **Forbidden**: `git add -A` and `git add .` are strictly prohibited. Stage explicit paths only.
- **Commit identity**: `-c user.name=<agent-id> -c user.email=<agent>@agent.local`.
- **Branches**: Work on `agent/<name>/<scope>`; never push to `main` directly.
- **One worktree per agent**: allocate it with `cynic source worktree --allocate agent/<name>/<scope>` and work only inside it. Never edit, switch branch, reset or cherry-pick in the primary checkout; other agents share it.
- **Ownership**: a worktree and its branch belong to the agent that allocated them. No other agent removes, prunes, resets or deletes them. Stale GC (`cynic source worktree --apply`) acts only on what the observation classifies as stale.
- **Cleanup at passage**: when the work is merged or abandoned, the agent that allocated the worktree removes it and deletes its branch (local and remote) in the same session. Leave nothing behind.
- **Main only through a PR**: never cherry-pick or merge old branches onto `main` outside a PR. Three commits that reached `main` without one (`9d3bf7d6`, `ad5ce4eb`, `6f371183`) broke the build for every agent.
- **Sovereign merges**: Forgejo is the sovereign store, GitHub the public projection. Merge only via `cynic source pr merge` on the sovereign side; never `gh pr merge` (it writes to the projection and splits the line — 2026-10-23). The hourly drift watch pages on divergence; converge before any further merge.
- **Claims**: Scope claim before mutation; release with proof reference.

## Convergence Procedure (Git & Worktree Hygiene)

1. **Observe**: `cynic git branch status --json` + `cynic source worktree --json`.
2. **Triate**: `cynic git reconcile --json` → classify branches (ReadyForPr/NeedsRebase/Archive/GcWorktree).
3. **Act**:
   - Merge ready branches via `cynic source pr merge` (Forgejo sovereign).
   - Rebase stale branches onto main; resolve conflicts or abandon.
   - Archive dead branches (0 commits ahead): `cynic git reconcile --apply`.
   - GC stale worktrees: `cynic source worktree --apply`.
4. **Verify**: `cynic audit all` must show:
   - `store_convergence.converged = true` (main == origin/main == github/main).
   - `branch_drift.orphan_count <= 50`.
   - `host_observed = true`.
5. **Converge**: push main to both origin and github; verify SHAs match.
6. **Never push to main directly** — always via PR merge on sovereign side.

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
- **Shape follows task**: execution → changed / verified / left; explanation → one idea per line, then examples; sovereign choice → numbered options with (R); error → likely cause first, then numbered steps; plan → short steps, one question at a time.
