# CYNIC — Minimal Agent Protocol

This repo has been reduced to a low-layer Rust runtime baseline.

## Long-Horizon Direction

This section preserves product direction for agents. It is not evidence that a
capability exists and it does not outrank the runtime authorities below.
Detailed ambition status belongs in `FrameworkSpec`; runtime truth still comes
from observation and read-back proof.

```text
the human remains the sovereign authority
Talaria writes and proposes code
CYNIC bounds, connects, executes, observes, and proves
Askesis projects a durable digital companion
CYNIC Inference supplies portable sovereign intelligence
Solana commits and settles explicitly disclosed public matter
Futard.io projects that matter to a futarchic audience
markets estimate consequences; they do not manufacture facts
```

CYNIC aims to be the private sovereign membrane for one-person futarchy. It
connects host, source, build, artifact, agent, inference, fleet, evidence, and
public-settlement planes without collapsing their authorities into one global
controller.

Durable direction:

- Solana is a piece of engineering: study its solutions and patterns for
  anything useful to CYNIC (ledger design, PoH, validator storage, fee and
  stake mechanics — study first, adopt only what survives the design gate);
- coordinate multiple agents without implicit global enforcement;
- reuse Kubernetes-like typed-resource, controller, reconciliation, and
  operator patterns in Rust without cloning Kubernetes as the framework;
- make important intent, delegation, mutation, provenance, outcome, and
  residual unknowns inspectable;
- let non-expert operators use their own data and common ML capabilities
  through simple portable ports;
- keep model, inference engine, accelerator, provider, chain, repository,
  scheduler, and public audience as injected adapters;
- let Talaria propose changes to CYNIC through the same isolated workspace,
  build, conformance, admission, canary, read-back, and rollback boundaries
  imposed on every other code-producing workload;
- disclose user activity only through explicit opt-in publication policy,
  minimization, consent, and evidence-preserving projection.

Guardrails:

- coordination without global enforcement does not remove enforcement at an
  explicit capability or mutation boundary;
- Kubernetes-like does not mean a Kubernetes clone;
- digital companion does not mean an omnipotent agent or opaque memory;
- simple ML does not hide provenance, consent, quality, cost, or uncertainty;
- sovereign inference may be local or remote and must remain adapter-driven;
- Rust-native implementation is justified by ownership, safety, portability,
  or observability, not by a mandate to rewrite every dependency;
- Solana is the strategic public commitment and settlement target, not the
  source of observed host truth;
- Futard.io is an audience projection, not private runtime authority;
- E-Score is a contextual, versioned, contestable interpretation of attributed
  evidence, never a universal measure of human worth;
- no token, market, score, company, agent, or model may silently amplify its own
  authority.

Public narrative projections live in `MANIFESTO.md` and `WHITEPAPER.md`. They
must label claims honestly and remain subordinate to contracts and runtime
proof. Capability status must not be maintained as a second editorial ledger;
project it from `FrameworkSpec` with `cynic dev framework --json` or
`cynic dev framework --markdown`.

## Authority

Use this order when files disagree:

1. observed runtime state
2. `contracts/foundation.json`
3. `infra/runtime-authority.json`
4. `infra/registry.json`
5. `Cargo.toml`
6. canonical Rust crates under `crates/`

Deleted authorities:

- `axioms/`
- `organs/`
- `docs/` as runtime law
- Python lifecycle gates

## Runtime Core

Keep work inside these surfaces unless the user explicitly expands scope:

- `crates/cynic-kernel`
- `crates/cynic-host`
- `crates/cynic-node`
- `crates/cynic-cli`
- `crates/cynic-protocol`
- `crates/cynic-askesis`
- `contracts/foundation.json`
- `infra/`

### Host substrate observation layering (applies to all substrate families)

The host substrate is observed in 4 ascending abstraction layers; every
family (disk, memory, CPU, network) reuses the same layering so a missing
lower layer stays an explicit unknown instead of a fabricated number.

1. Physical  — raw hardware device/counter. e.g. disk `/dev/sda` (lsblk);
   memory DIMM MemTotal (`/proc/meminfo`); CPU model + core count (`/proc/cpuinfo`);
   network NIC link (`ip -j link`).
2. Volume/logical  — configuration bounding addressable substrate.
   e.g. disk filesystems/mounts (`df --output`); memory page size + swappiness +
   hugepages (`/proc/meminfo` Hm/hugepage reservation); CPU governor +
   isolation mask; network route table.
3. Usage  — actual consumption against the configured volume.
   e.g. disk used/avail per mount (`df`); memory used/free/available/swap
   (`/proc/meminfo` + `free`); CPU user/sys/idle jiffies (`/proc/stat`);
   network bytes/Tx queue against the route interface.
4. Pressure/fidelity  — backpressure signalling the volume is saturated.
   e.g. disk (none — best-effort); memory PSI `/proc/pressure/memory` some/full;
   CPU PSI `/proc/pressure/cpu`; network socket backpressure / drops.

Canonical ownership per family:
- types live in `cynic_protocol::<family>` (never in the CLI surface);
- pure parsers (no I/O) live in `cynic_host::domain::<family>::probe`;
- the observe port (`ObservationPort::observe_<family>`) is implemented by
  `Local<Family>Adapter` (reads `/proc`/`/sys`/argv) and
  `Remote<Family>Adapter<R: ArgvRunner>` (SSH, injectable for tests);
- the fleet rollup is `Fleet<Family>Observer<R>` in `cynic_host`, emitting
  `<family>.FleetReport` to `cynic.fleet_<family>.v1`;
- the CLI surface (`RuntimeCommand::<Family>` / `FleetCommand::<Family>`) is a
  thin projection only — it never owns policy.

A report is coherent only when every layer has a concrete owner; an absent
lower layer is reported as a residual unknown, never back-filled by a guess.

### Organic opportunities — `Organic always wins`

An **organic opportunity** is an observed, bounded candidate for operator
attention. It is not a task, a grant, a claim, or an authority to mutate. It
arises when unfinished work, recurring session friction, emerging patterns,
plumbing debt, or cross-surface dependency evidence indicates that a small next
step could unlock disproportionate proof, coverage, or operator capacity.

The operator's priority order must be continuously informed by fresh organic
opportunities rather than a static backlog or the most recently proposed
feature. A proposal carries its observed sources, expected unlock value,
estimated energy, scope, freshness, confidence, and residual unknowns. Its
ranking is contextual and contestable; it is never a universal score, urgency,
or autonomous mandate.

`Organic always wins` therefore means: when a fresh, higher-unlock opportunity
is observed, an agent must surface it before spending a bounded budget on a
lower-unlock planned item, unless the operator explicitly records a different
choice. It never overrides human sovereignty, admission, claims, capability
boundaries, consent, safety gates, or the requirement for independent
read-back proof.

The compass is local and read-only by default. Its minimal coherence loop is:

```text
desired operator capacity → observe sessions + source state → reconcile opportunities
  → propose a smallest next observation/repair → report ranked evidence and unknowns
```

It may run periodically as a bounded local observer, but it must not create
tasks, alter priorities, apply repairs, write skills, publish telemetry, or
promote its own conclusions. Each of those is a separate, human-admitted
action with its own proof obligation.

## CYNIC Coherence Pattern

Use this pattern at every scale. It is the compact form of the runtime model,
not a new authority above the contracts.

```text
desired state → observe → reconcile → bounded action → read-back proof
                         ↘ retain only explicit, machine-readable evidence
```

An implementation is coherent only when each term has a concrete owner:

| Term | Meaning | Canonical form |
|---|---|---|
| desired state | an intent that can be compared without touching the target | registry data, typed request, or plan input |
| observe | a read-only account of the actual substrate | port + adapter, with structured report |
| reconcile | the pure decision from desired and observed state | typed policy or plan |
| bounded action | an explicit, reviewable mutation | argv command, declared privilege/mutation, explicit `--yes` / `--apply` |
| read-back proof | evidence after the action, not a claim of success | versioned JSON report, test, or verification command |

Apply the same shape recursively; do not substitute a higher layer for proof
from a lower one:

| Scale | Desired state | Observer/action boundary | Proof consumer |
|---|---|---|---|
| Rust primitive | input and invariants | pure function or trait | unit test / caller |
| domain capability | typed request and policy | port, then adapter | host/node use case |
| command surface | CLI/MCP request | thin adapter over a use case | human or agent JSON consumer |
| host or node | registry role and runtime layout | explicit execution plan | runtime proof bundle |
| fleet | placement/topology registry | node-scoped plans and bounded rollout | kernel history, metrics, and operator |

### Design gate

Before adding code, a command, a crate, or a contract, answer these questions
from the bottom up:

1. What is the desired state, and where is its authoritative representation?
2. What can be observed without mutation, through which portable port?
3. What pure rule selects the next safe action from desired and observed state?
4. If mutation is needed, what exact argv/action, privilege boundary, consent,
   rollback intent, and stop condition does it expose?
5. What post-action read-back proves the result, and which lower layer produced
   it?
6. Which existing boundary owns each answer? Extend that boundary unless it
   cannot absorb the responsibility cleanly.

If an answer is unknown, preserve it as an explicit unknown or blocked state.
Do not turn a missing observer, adapter, proof, or coordination store into a
successful-looking report.

### Cross-scale invariants

- **Truth flows upward; intent flows downward.** Registries and requests may
  direct work downward, but liveness and health flow upward only from read-back.
- **Policy is portable; substrate is injected.** Types and plans name roles and
  capabilities; adapters bind systemd, process, VM, LXC, provider, path, or
  transport details.
- **A surface projects; it does not own.** CLI, MCP, reports, and prose expose
  capabilities but do not duplicate domain policy or become runtime authority.
- **Every mutation is a small transaction.** Keep desired state, preflight,
  apply intent, observation, proof, and rollback intent separately inspectable.
- **Scale changes data and adapters, not the core model.** One host is the
  first fleet case; adding nodes must not require a parallel orchestration
  semantics.
- **Unknown is data.** Coverage, capability, and proof freshness may be
  unknown; report residual risk instead of fabricating convergence.
- **Repair flows downward to the violated contract.** A symptom in a report,
  CLI, MCP tool, or agent session is not evidence that the projection owns the
  defect. Trace it through the use case, policy, port, adapter, and substrate;
  correct the lowest layer that can express the missing or violated contract.
  A higher-layer guard is acceptable only as an explicit temporary containment
  with its residual risk and removal condition recorded in machine-readable
  evidence or a test.
- **Every bug is evidence of an unmodelled contract.** Classify the gap as
  desired state, observation, reconciliation policy, execution boundary, or
  proof. The durable repair makes that contract explicit and adds a regression
  proof at the layer that owns it; suppressing the visible symptom alone is not
  a completed repair.

### Agent decision rule

Agents are bounded workloads, not a source of runtime truth. For each task,
they must name the surface they use, retain the evidence they observed, run the
canonical validation gate where applicable, and report what remains unknown.
Chat memory, a plan, or a successful command invocation alone is never proof.

### Fractal agent recursion

Every runtime node is a sovereign recursion root and may host its own tree of
specialized agents. The fleet is therefore a federation of node-local agent
trees, not one global agent with an unbounded context.

- Local child delegation uses `cynic.agent_delegation_envelope.v1`.
- A child remains on its parent's `node_id`, names its parent, and increments
  recursion depth exactly once. Crossing a node boundary is federation, not
  local recursion, and requires an explicit coordination/transport contract.
- Authority and budgets are monotonic: a child may receive a subset of its
  parent's capabilities, mutation authority, context budget, action budget,
  child budget, and wall-time budget; it may never amplify them.
- Context is a disposable working set assembled from content-addressed evidence
  references. Raw chat history, provider session state, and model memory are
  not delegation authority.
- Intent and bounded capability flow downward. Evidence, consumed budget,
  explicit unknowns, and completion status flow upward through
  `cynic.agent_completion_envelope.v1`.
- Model, inference provider, scheduler, and transport remain adapters. The same
  delegation and completion contracts apply to local models, remote models,
  humans, one node, or recursively composed supervisors.
- Recursive execution remains inactive until a node-owned scheduler can prove
  aggregate sibling budgets, cancellation, lease fencing, durable evidence,
  and recovery after parent or node failure.

## Agent Coordination Claims

When multiple agents share one worktree (baseline mode), use claims to avoid
conflicts. The canonical store is `.agent-claims.json` at the repo root.

### Two modes

| Mode | Who | Enforcement |
|---|---|---|
| `baseline` | External agents (Hermes, Codex, CLI) | Best-effort, cooperative. Claims are advisory. |
| `cynic` | Native scheduler workloads | Durably enforced. Cynic claims may preempt baseline. |

### Claim lifecycle

1. **Claim** — `cynic agent claim <scope>` before mutation.
   - Scope is hierarchical: `fleet` → `node` → `crate` → `module` → `file`.
   - Each claim carries a budget (`actions`, `wall_seconds`) and TTL.
   - Two claims with overlapping scopes conflict. First-wins for same mode;
     cynic preempts baseline.

2. **Heartbeat** — renew TTL while working.
   - Stale heartbeat (no renewal within TTL) → claim expires → others may
     preempt.

3. **Complete** — `cynic agent release <claim_id>`.
   - Sets status `done`, enqueues merge.
   - Merge bot resolves conflicts when multiple claims targeted the same scope.

### Conflict resolution

| Situation | Resolution |
|---|---|
| Same scope, same mode | First claim wins. Second waits or picks another scope. |
| Cynic vs baseline | Cynic preempts. Baseline gets grace period to release cleanly. |
| Batch claim partial failure | Atomic: either all claimed or none. |
| Merge conflict | Rebase against current main; re-claim if scope still needed. |

### Scaling via hierarchy

A `Fleet` claim decomposes into `Node` claims. A `Node` claim decomposes
into `Crate` claims. Parent budget must be >= sum of children budgets
(monotonic). Each child is an independent claim — different agents can work
on different crates simultaneously under the same parent.

### Data model

```jsonc
{
  "schema": "cynic.agent_claims.v2",
  "revision": 42,
  "claims": [
    {
      "claim_id": "claim_001",
      "agent_id": "hermes:main",
      "mode": "baseline",
      "scope": { "kind": "file", "path": "crates/cynic-kernel/src/lib.rs" },
      "budget": { "actions": 50, "wall_seconds": 3600 },
      "consumed": { "actions": 0, "wall_seconds": 0 },
      "temporal": { "started_at_unix_s": 1724000000, "ttl_seconds": 1800, ... },
      "priority": 10,
      "parent_claim_id": null,
      "status": "active"
    }
  ],
  "merge_queue": []
}
```

Pure predicates (`cynic_protocol::coordination_lease::predicates`) handle
scope overlap, expiration, heartbeat staleness, and preemption rules.

Work bottom-up from executable substrate to higher orchestration.

- Prefer a small runtime primitive with tests over a broad policy document.
- Stabilize one layer before using it as a dependency for the next layer.
- Treat Rust types, command surfaces, tests, and runtime reports as stronger
  authority than narrative prose.
- Keep domain rules separate from adapters: provider, distro, path, and command
  details belong behind runtime layout or execution adapters.
- Do not hardcode for the current machine when a role, capability, path, or
  provider can be injected.
- Delete or quarantine stale surfaces that cannot prove current runtime use.

## Structural Invariants (immutable framework law)

These rules are durable architecture law. They apply to new surfaces, refactors,
and repairs.

### 1. No monolith surfaces

- A surface must not own, in one file, its types, policy, probe, planning,
  apply, and rendering concerns together.
- Split surfaces at least into bounded modules such as:
  - `types`
  - `specs` or `policy`
  - `probe` or `observe`
  - `decision`
  - `plan`
  - `apply`
  - `render`
  - thin CLI glue
- CLI surfaces are orchestration/projection only. They must not become the
  long-term owner of domain policy tables or runtime decision trees.

### 2. No hardcoding when a dynamic contract exists

- Do not hardcode the current machine, operator username, distro, path, Python
  install mode, provider, or service manager when the fact can be observed or
  injected.
- Stable declarative registries are allowed; ad-hoc runtime branches in CLI or
  agent projection code are not.
- If a value changes across hosts, principals, packaging modes, providers, or
  harness kinds, prefer a typed registry entry or runtime observation over
  literal branching.

### 3. Dynamic detection before specialization

- Observe the substrate first, then choose the adapter or recipe.
- Detection must precede specialization for:
  - packaging/install provenance
  - principal ownership
  - writable vs blocked authority
  - provider/runtime supervisor selection
  - service manager or process boundary
  - connection/adoption state
- If detection is incomplete, preserve `unknown` or `blocked`; do not silently
  guess and present the result as converged.

### 4. Primary system components must have explicit install authority

- CYNIC must know, for every primary runtime or operator component it manages:
  - which principal owns it
  - which channel owns it (`scripted`, `external`, `distro_managed`, or a more
    specific typed provenance under that channel)
  - which observable install provenance produced it
  - whether that provenance is canonical or noncanonical for the declared role
- Examples of install provenance that must be modeled explicitly where relevant:
  - distro-managed package
  - scripted local install
  - `uv tool`
  - `uv pip --system` as mutator authority for Python user-site payloads when
    that is the actual install/update path
  - `uv venv`
  - generic virtualenv
  - `pipx`
  - Python user-site (`pip --user`)
  - system Python package
- Upgrade and repair policy must derive from that observed provenance instead
  of assuming a universal self-update command.
- For Python CLI components, keep **payload layout** and **mutator authority**
  separate when the substrate exposes them differently. Example:
  `uv pip --system` commonly mutates a user-site payload under
  `~/.local/bin` / `~/.local/lib/python*`; in that case observe the payload as
  `pip_user_site`, but keep the canonical repair/update command under the `uv`
  execution surface instead of pretending the payload itself proves `uv tool`
  or `uv venv`.

### 5. Declarative desired state plus observed proof, not blind IaC

- CYNIC should converge toward an IaC-like model, but desired state alone is
  never authority.
- The durable shape is:
  - declarative desired state
  - read-only observation
  - pure reconciliation decision
  - bounded explicit apply
  - read-back proof
- A declarative file without observation and proof is not convergence.
- A successful command without read-back proof is not convergence.

### 6. One canonical decision source per concern

- A decision must not be recalculated independently in multiple CLI, MCP, or
  apply surfaces once a canonical decision layer exists.
- If the same policy appears in three places, extract it into a shared
  primitive or decision module and make the consumers project from it.
- Plans, connect/apply surfaces, and reports must consume canonical decisions,
  not rebuild them locally.

### 7. Adapters are disposable; ports and proofs are durable

- Tool-specific or provider-specific commands are adapter details.
- Durable framework law lives in:
  - typed contracts
  - ports
  - decision modules
  - proof schemas
  - validation gates
- Replacing `uv`, `pip`, `gh`, `hf`, `systemd`, or another adapter must not
  require inventing a second orchestration model.

## Execution Standard

Mutable work must pass through explicit execution surfaces.

- No hidden shell execution in apply paths.
- No Python wrapper may become durable runtime authority.
- Commands should be argv-based, typed, observable, and stop on first failure.
- Plans must separate desired state, preflight, apply, observe, proof, and
  rollback intent.
- A command that changes the host must expose whether it mutates, whether
  privilege is required, and which proof it produces.
- Runtime proof should be generated as machine-readable JSON and validated by
  tests or a verification command.

## Runtime Harness

A runtime harness is a controlled execution context used to turn a framework
capability into proof.

It is not the capability itself, and it is not automatically runtime authority.
It defines where and under which safety boundary a capability may be planned,
applied, observed, and judged.

Required harness dimensions:

- `kind`: the class of execution context, such as closed local lab,
  conformance, staging, canary, or production.
- `authority`: whether the context is a test projection, conformance gate,
  release candidate, fleet canary, or runtime authority.
- `isolation_boundary`: the membrane that prevents unintended blast radius,
  such as rootless sandbox, isolated VM network, staging VM, canary node, or
  production fleet boundary.
- `target_scope`: the scale being proven, from one local host to N distributed
  nodes.
- `mutates_host_network`: whether the harness may alter operator connectivity
  or host routing.

Harness progression:

- `lab`: closed local runtime lab. It is an ephemeral proof environment for
  bootstrap, network, and runtime evidence. It must not become canonical runtime
  authority.
- `conformance`: portable suite proving a node satisfies CYNIC execution,
  security, bootstrap, and observation contracts.
- `staging`: production-like VM/LXC context for release-candidate validation.
- `canary`: bounded subset of real fleet nodes with rollback and observation
  requirements.
- `production`: declared runtime authority, SLO/uptime tracking, persistent
  evidence, and controlled activation.

Rule: do not grow `lab` into the framework. Extract reusable primitives into
runtime harness, execution, topology, and conformance interfaces. `lab` is only
one harness implementation.

Executable surface:

```bash
cynic runtime harness --json
cynic runtime harness --json --kind conformance
cynic runtime conformance --json
cynic runtime metrics --json
cynic runtime metrics --json --proof <runtime-proof-bundle.json>
```

Agents must consult this surface before inventing a new execution context.

Metric rule: `100%` is not the global success target. Each metric carries its
own `target_percent`; unknown coverage remains `null` and contributes to
residual risk. Industrial-grade means reaching the target thresholds with an
explicit residual unknown budget, not pretending every OS, provider, and future
case is covered. Metrics may consume external runtime proof bundles for surfaces
the current host cannot observe directly; they must not fabricate those proofs.

## Agent Session Practice

Agent work must reduce fragmentation.

### CYNIC session handshake

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

### Crate ownership & extraction (systemic, not kernel-only)

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

### Git isolation in shared worktrees (mandatory)

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

## Sovereignty vs relay (chicken and egg)

- **Runtime authority** stays local: principal `cynic`, `/etc/cynic`, harness proofs.
- **Git**: Forgejo (`infra/repo-remotes.json`) is the sovereign *candidate* SSOT.
  GitHub `origin` is an optional **public relay** while Forgejo DNS/auth/repo is
  repaired — not a reason to reintroduce SaaS CI as law.
- Observed Forgejo HTTP port is **3000** (not 18300). Host aliases live under
  synapse / `/etc/hosts` (`forgejo.synapse.cynic`, `10.10.10.101`).
- Prefer fixing reachability with cynic network/bootstrap surfaces over hardcoding
  operator machine names in domain code.
- Bootstrap Forgejo push (create repo + push) once you have a token:
  `FORGEJO_TOKEN=… ./scripts/forgejo-bootstrap`

## Secrets (industrial, frictionless, agnostic)

Secrets are **not** git content and **not** kernel SSOT. They are injected
material classified by *when* they may exist.

### Secret classes (host bootstrap law)

| Class | When allowed | Examples |
|-------|----------------|----------|
| `HumanAdmin` | bootstrap only (console/once) | human password, sudo gesture, Forgejo login |
| `MachineEnrollment` | first join / enroll | install token, one-time join key |
| `RecoveryBreakglass` | incident only | emergency root, recovery codes |
| `RuntimeMachineIdentity` | **runtime steady-state** | llama API key file, machine TLS, helper identity |

Rule already in host bootstrap: **runtime must not depend on HumanAdmin**
(`runtime_human_secret_dependency` stays blocked until bridge/helper replaces
human sudo for steady work).

### Placement (agnostic — no vendor lock)

Prefer this order of authority for *runtime* secrets:

1. **Env inject** (process scope): `CYNIC_BACKEND_<NAME>_API_KEY`,
   `CYNIC_CHAT_API_KEY`, `FORGEJO_TOKEN` for one-shot tools.
2. **Principal-owned file** (mode `0640`, owner `cynic`, group `cynic`):
   - `$PRINCIPAL_HOME/.config/cynic/<name>` (e.g. `llama-api-key`)
   - optional mirror `/etc/cynic/<name>` root:cynic for installed SSOT
3. **OS secret store adapter** (optional later): systemd-creds, kernel keyring,
   age/sops, external vault — always behind a port, never hardcoded provider.
4. **Never**: git, `infra/agent-exposure.json`, docs, chat JSONL, proof JSON.

Config that is *not* secret (URLs, model names, pins) stays in
`inference.toml` / pins / registry. TOML may have a stub `api_key` only for
tests; env/file wins in loaders.

### Frictionless resolution (already partially implemented)

- Inference backends: `CYNIC_BACKEND_<NAME>_API_KEY` → loader inject.
- Chat: `CYNIC_CHAT_API_KEY` → `CYNIC_CHAT_API_KEY_FILE` → backend env →
  principal/`/etc/cynic` `llama-api-key`.
- Git Forgejo: **session token in env** for `scripts/forgejo-bootstrap`, not
  embedded permanently in `git remote` URL. Prefer `git credential` /
  `pass` / OS keyring for recurring push; token rotation is human admin.

### Operator practice

```bash
# Runtime machine identity (llama local)
install -m 640 -o cynic -g cynic /path/to/key /var/lib/cynic/.config/cynic/llama-api-key
# optional installed mirror
sudo install -m 640 -o root -g cynic /path/to/key /etc/cynic/llama-api-key

# One-shot sovereign git (HumanAdmin → temporary)
FORGEJO_TOKEN=… ./scripts/forgejo-bootstrap
# do not commit the token; revoke/rotate when done
```

Reports and MCP must show **presence** (`has_api_key=true`) never the secret.

## Inference supervisor (systemd is optional)

- Native local engines are behind `LocalInferenceRuntimePort` — **not** hardwired
  to systemd forever.
- Default adapters:
  - `CYNIC_INFERENCE_SUPERVISOR=systemd` — `llama-server@` user units
  - `CYNIC_INFERENCE_SUPERVISOR=process` — pidfile/process spawn (lab, no user bus)
- Auto: if `XDG_RUNTIME_DIR` bus missing → prefer `process`.
- Principal linger (`loginctl enable-linger cynic`) only needed for the systemd
  path under user `cynic`.

## Agent CLI / Host Surface Patterns (anti-monolith)

These rules are durable agent law for both Grok and Codex sessions. They exist
because large single-file "surfaces" reintroduce the patterns we are deleting.

### Structure

- Prefer layered modules over one file that owns types + policy + I/O + spawn +
  reports. Reference layouts:
  - `crates/cynic-host/src/domain/ecosystem_package/`
  - `crates/cynic-host/src/update_lifecycle/`
- Split at least: `types` (pure shapes), `policy`/`plan` (pure tables/logic),
  `port` (traits for execution/observation), adapters, and thin CLI glue.
- Do not grow a domain file past ~400 lines without extracting a port or a pure
  submodule. If a change would force that, split first.

### Execution ports

- Domain/host logic must **not** hard-wire `std::process::Command::new("cynic")`
  (or shell composition) for sibling CLI surfaces. Inject an argv runner port
  (`ArgvRunner` / local adapter / map stub for tests).
- CLI crates inject the production runner; unit tests inject fakes that return
  fixed JSON. Gate/repair/apply paths must be unit-testable without a real
  `cynic` binary on PATH.
- Mutable work still requires explicit `--yes` / `--apply`. No silent auto-repair,
  no fail-closed mock that pretends success when an endpoint is missing.

### CLI thinness

- `cynic-cli` surfaces: clap parse → inject ports → call host/crate API → print
  human or JSON. No phase policy tables, no finding classifiers, no state schema
  ownership in the CLI crate.
- One canonical command path; register it in `surface_registry` +
  `infra/agent-exposure.json` when the surface is agent-facing.
- Machine-readable reports are the proof surface; human text is a projection.

### Surface grammar

- Default grammar for non-trivial CLI/runtime surfaces:
  - facade/orchestrator
  - `types`
  - decision/plan/catalog or other pure tables
  - scan/observe/path discovery helpers
  - checks/predicates/classifiers
  - persistence/output helpers when the surface writes artifacts
- Do not collapse these roles back into one file once they have been split.
- If a surface needs more than one of:
  - state schema ownership
  - filesystem/report persistence
  - source/path discovery
  - rule evaluation
  - command dispatch families
  then it should already be a module tree, not a single `*.rs` file.
- Preferred thin-facade pattern in `cynic-cli`:
  - parse command
  - resolve injected context
  - delegate to family module
  - emit JSON/human projection
  - return exit code

### Audit grammar

- Default grammar for audit surfaces:
  - facade/assembly
  - `types`
  - `scan` or source/JSON discovery
  - `checks` / predicates / classifiers
  - shared path helpers where the audit walks source trees
- Audit surfaces must not each reinvent:
  - workspace root discovery
  - Rust file traversal
  - pretty JSON file writing
  - machine-local path detection
  when a shared helper already exists.
- When two audits need the same primitive, extract it on the second occurrence.

### Shared helpers

- Shared helper modules are preferred over copy-pasted local utilities once a
  pattern repeats across surfaces. Current canonical examples:
  - `cli_json` for stdout JSON projection
  - `fs_json` for pretty JSON file persistence
  - `rust_source` for workspace-relative Rust source discovery
- A shared helper must stay primitive. It should not quietly absorb domain
  policy.

### Growth thresholds

- Soft threshold: split before a CLI/audit facade grows past ~200-250 lines.
- Hard threshold: any single surface file above ~400 lines must be treated as
  extraction debt unless it is already a stable pure data table or test-heavy
  fixture file.
- If a file stays large on purpose, the reason should be obvious from its role
  (for example: pure registry table, generated schema projection, or dense test
  fixture data), not because unrelated responsibilities accumulated there.

### What not to reintroduce

- Monolith `*_lifecycle.rs` / `*_surface.rs` that re-implements half the runtime.
- Python organs or docs as runtime authority.
- Hardcoded operator machine paths, distro names, or principal homes in domain
  code when a layout/capability can be injected.
- Parallel "agent helper" shells that bypass argv-typed execution.

## Industrial Bar

The framework is not industrial-grade until these properties are demonstrable:

- a fresh Linux node can bootstrap from a standalone `cynic` artifact
- privileged host changes are bounded, auditable, and reversible
- distro-specific behavior is selected by detected capabilities, not hardcoded
  assumptions
- node/network actions can run in a closed environment without risking operator
  connectivity
- runtime reports prove desired state, applied mutation, observation, and
  resulting health
- distributed formation works for one node first, then N nodes without changing
  the core model

## Validation & Compiler Alphas

- Canonical spec for Rust compiler properties, memory limits, and visibility rules:
  `infra/RUST_COMPILER_ALPHAS.md`.
- Use **targeted** crate checks/tests with stripped debuginfo for fast, deterministic builds:
  - `RUSTFLAGS="-C debuginfo=0" cargo check -p <crate> --lib`
  - `ulimit -s unlimited && RUST_MIN_STACK=8589934592 RUSTFLAGS="-C debuginfo=0" cargo test -p <crate> --lib <module>`
  - `cynic-kernel` tests require admission: `CYNIC_BUILD_ADMISSION_VERIFIED=1`.
- **Library-First Invariant**: All domain types, ports, usecases, and run functions belong in `src/lib.rs`. `src/main.rs` is a thin facade.
- Never mask unverified code with `#![allow(dead_code)]`: dead code is an epistemic sensor indicating orphaned capabilities, missing wiring, or ghost legacy.
## Rules

- Do not recreate contract forests.
- Do not reintroduce Python organs as runtime authority.
- Prefer Rust types, tests, and runtime proof over prose.
- Keep infra minimal and executable.
- Anything outside the runtime authority map must prove current runtime use
  before it is kept.
