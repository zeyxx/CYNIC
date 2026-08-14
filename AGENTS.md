# CYNIC — Minimal Agent Protocol

This repo has been reduced to a low-layer Rust runtime baseline.

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
- `crates/cynic-mcp`
- `crates/cynic-cli`
- `crates/cynic-protocol`
- `crates/cynic-askesis`
- `contracts/foundation.json`
- `infra/`

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

## Development Method

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

## Validation

Default:

```bash
make check
```

This validates the minimal authority files and runs `cargo check` on the core
runtime crates with deterministic rustc settings.

Cargo policy:

- Prefer `make ...` or `./scripts/cargo ...` over raw `cargo ...`.
- Default repo mode is offline registry use through a repo-local `CARGO_HOME`.
- Use `CARGO_SOURCE_MODE=vendor` once `./scripts/cargo-vendor-bootstrap` has
  populated `vendor/`.
- Use `CARGO_NETWORK_MODE=online` only when the execution environment truly has
  Cargo network access and a dependency refresh is intentional.

## Rules

- Do not recreate contract forests.
- Do not reintroduce Python organs as runtime authority.
- Prefer Rust types, tests, and runtime proof over prose.
- Keep infra minimal and executable.
- Anything outside the runtime authority map must prove current runtime use
  before it is kept.
