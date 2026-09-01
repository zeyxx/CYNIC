# Agent Session Practice

Agent work must reduce fragmentation.

## CYNIC session handshake

When the CYNIC MCP surface is available, an agent session must:

1. observe `cynic_mcp_surface`;
2. register through `cynic_coord_register`;
3. read back its identity through `cynic_coord_who`;
4. claim the affected source target before mutation;
5. use `cynic_source_control` for admitted commit/push work;
6. use `cynic_validate` to plan validation for `build_runner`.

`cynic_validate` must not dispatch through the legacy `AgentTask` queue. Agent
mutation execution remains inactive until a node-owned scheduler durably
enforces `cynic.agent_workload_envelope.v1`, including policy admission,
monotonic budgets, lease fencing, cancellation, recovery, and completion
evidence. A handshake or queue acknowledgement is not execution authority.

When the CYNIC MCP surface is unavailable, incompatible, or unstable, the
agent must first retain that read-back as an explicit unknown. A human may then
admit a temporary `human_scoped_fallback` only for a bounded local source target
and local source validation. The admission must name an action and wall-time
budget, prove there is no active conflicting baseline claim, and retain Git
read-back before and after the slice. It is `best_effort_unfenced`, never a
lease, and expires as soon as a stable compatible coordination runtime is
observed. It never authorizes runtime, credential, network, fleet, release,
commit, push, or merge mutation. Without that explicit human admission, report
`blocked_disconnected`. The machine-readable fallback contract and tool
ownership remain in `infra/agent-exposure.json`; this section only instructs
agent projection.

MCP is a replaceable projection, never a bootstrap prerequisite or the source
of coordination law. A removed, unavailable, or incompatible MCP artifact does
not block local read-only observation or a human-admitted bounded local source
slice. Rebuild MCP only after the contracts, CLI projection, and independent
local read-back it exposes are stable; the rebuilt surface must project those
owners without recreating policy, scheduler, or handler monoliths.

- Before adding a new crate, command, document, or contract, prove an existing
  boundary cannot absorb the capability.
- Keep CLI entrypoints thin; business logic belongs in crates with clear roles.
- Prefer one canonical command path over aliases unless the alias is a
  compatibility surface.
- Every non-trivial change should move one concrete capability closer to:
  standalone bootstrap, OS portability, agnostic execution, distributed runtime
  formation, or runtime proof.
- If coordination storage is unavailable, say so explicitly and do not pretend
  the organism remembered the session.
- Do not create backlog Markdown as memory; use executable tests, contracts, or
  runtime state instead.
- Treat `docs/` as quarantined historical prose (`docs/README.md`). Never use
  docs, deleted `axioms/`, or deleted `organs/` as runtime law.
- When adding or renaming an MCP tool, update `infra/agent-exposure.json` in the
  same change and keep `cynic dev agent-exposure --json` green.
- Do not commit machine-local absolute paths in agent projections
  (`.codex/config.toml`, `infra/agent-exposure.json`, repo-access profiles).
- Prefer `make check` / MCP `cynic_validate` / `cynic dev check --json` as the
  same validation gate. Do not invent a parallel fmt/build/clippy/test path.
- For kernel extraction debt, consult `cynic dev kernel-audit --json` before
  growing product-domain logic in `cynic-kernel`. Leave `cynic-inference`
  extraction to the owner currently working that surface unless asked otherwise.
- Product domain prompt files under `crates/cynic-kernel/domains/` are residual
  product material (`domains/README.md`), not low-layer runtime law.

## Crate ownership & extraction (systemic, not kernel-only)

Ownership is a property of EVERY crate, not just `cynic-kernel`. The
kernel is the first large case to formalize, but `cynic-host` (84k LOC)
is larger than the kernel (58k) — the defect is systemic.

- Canonical spec: `infra/CRATE_OWNERSHIP.md`. It transposes
  `contracts/foundation.json` §481 `one_boundary_one_reason` and §489
  (non-goal: "centralize all responsibilities in one orchestrator
  crate") into a per-crate `owns / delegates / forbidden` grid and an
  incremental extraction plan. The kernel map there is corroborated by
  the coded `KERNEL_AREAS` table in `crates/cynic-cli/src/kernel_boundary_audit.rs`.
- The three dev-surface audits are the machine-readable read-back that
  enforce this spec; run them before growing any crate:
  - `cynic dev kernel-audit --json` — kernel areas vs target owners
    (schema `cynic.cli.kernel_audit.v1`); reports `blocked` while any
    area is `extract`/`stabilize` or uncovered hotspots remain.
  - `cynic dev ownership-audit --json` — 11 ownership surfaces with
    crate_owner + migration_target (schema `cynic.cli.ownership_audit.v1`).
  - `cynic dev crate-audit --json` — per-crate LOC, hotspots, bounded
    entrypoints (schema `cynic.cli.crate_audit.v1`).
- Rule of thumb (machine-checkable, see `crate-audit`): a crate over
  ~40k LOC, or one that implements ports belonging to another domain,
  violates §481 and must be split. Doubled responsibilities across two
  crates are the same violation.
- Before adding a module/adapter/crate: prove an existing boundary
  cannot absorb the responsibility (§481), then record the decision in
  `infra/CRATE_OWNERSHIP.md` and `infra/surface-registry.json`.
- Extraction precondition (every kernel SoC slice): inventory the
  TARGET crate's existing surfaces (ports, modules, `pub use`) BEFORE
  writing code. Extraction extends/reuses an existing surface; when an
  equivalent exists (e.g. `cynic-inference` `backend_identity` vs
  kernel `BackendHealth`), the existing one becomes canonical — never
  create a parallel duplicate. Record the inventory result in the slice
  plan under `.hermes/plans/` and verify post-extraction by re-grepping
  that the kernel imports no concrete type of the extracted surface.
- Canonical per-crate ownership & SoC model: `infra/CRATE_MODELS.md`.
  It assembles `contracts/framework.json` modularity_rules
  (protocol_is_pure, cli_is_adapter, proof_producers_are_low_layer,
  one_boundary_one_reason) + agnosticity_rules (no_provider_in_domain,
  n_case_by_registry, substrate_behind_ports) + non_goal §489 (no
  centralize) into one PRIMARY / OWNS / DELEGATES / FORBIDDEN grid per
  crate, plus the absorb-vs-new-crate decision form (§2). Use it to
  classify a suspected responsibility before extracting.

## Git isolation in shared worktrees (mandatory)

Multiple agents share one worktree. Every mutation slice ends in an isolated
commit with explicit paths — a PR is even better when Forgejo/GitHub is
reachable. This workflow is not optional.

- Never `git add -A` or `git add .`. Stage only your files, by explicit path,
  then read back `git status --short` before committing: the staged list must
  equal your file list exactly.
- One commit per coherent slice, with design decisions and proof counts in the
  message (test command → N passed; rustfmt --check PASS; git diff --check
  PASS). Read back with `git show --stat HEAD`.
- Other agents' `M`/`D`/`??` entries are their debt. If a dependency crate is
  red from their mid-edit files, do not repair it silently: wait for
  stabilization (files untouched ~15 min) or isolate (scratch validator,
  detached worktree at HEAD plus only your files).
- Untracked files have no git safety net; after a green slice, commit them
  promptly or back them up outside the repo.
- Work on a dedicated branch `agent/<name>/<scope>`; never push directly to
  main. Push the branch and open a PR when the remote is available.
