# CYNIC — Minimal Agent Protocol

This repo has been reduced to a low-layer Rust runtime baseline.

## Long-Horizon Direction

CYNIC is a private sovereign membrane for one-person futarchy, connecting host,
source, build, artifact, agent, inference, fleet, evidence, and public settlement
planes without collapsing their authorities into one global controller.

Public narrative projections live in `MANIFESTO.md` and `WHITEPAPER.md`. They
must label claims honestly and remain subordinate to contracts and runtime
proof. Capability status projects from `FrameworkSpec` via
`cynic dev framework --json` or `cynic dev framework --markdown`.

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

### Host Substrate Observation Layering

The host substrate is observed in 4 ascending abstraction layers across all
families (disk, memory, CPU, network):

1. **Physical**: raw device/counter (`/proc/meminfo` MemTotal, CPU model, link).
2. **Volume/logical**: configuration bounding addressable substrate (mounts, hugepages, governor, routes).
3. **Usage**: actual consumption against configured volume (`df`, free/avail memory, stat jiffies, interface bytes).
4. **Pressure/fidelity**: backpressure signalling volume saturation (PSI memory/cpu, socket drops).

Canonical ownership per family:
- types live in `cynic_protocol::<family>` (never in CLI);
- pure parsers (no I/O) live in `cynic_host::domain::<family>::probe`;
- observation port (`ObservationPort::observe_<family>`) implemented by `Local<Family>Adapter` and `Remote<Family>Adapter<R: ArgvRunner>`;
- fleet rollup is `Fleet<Family>Observer<R>` in `cynic_host`, emitting `<family>.FleetReport` to `cynic.fleet_<family>.v1`;
- CLI is a thin projection only (`RuntimeCommand::<Family>` / `FleetCommand::<Family>`).

An absent lower layer is reported as an explicit residual unknown, never guessed.

### Organic Opportunities — `Organic always wins`

An **organic opportunity** is an observed, bounded candidate for operator
attention where a small next step unlocks disproportionate proof, coverage, or
capacity.

- The compass is local and read-only by default.
- Minimal loop: `desired operator capacity → observe sessions + source state → reconcile opportunities → propose smallest next observation/repair → report ranked evidence and unknowns`.
- Surfaces fresh, higher-unlock opportunities before lower-unlock planned items.
- Never overrides human sovereignty, claims, admission, or independent read-back proof.

## CYNIC Coherence Pattern

```text
desired state → observe → reconcile → bounded action → read-back proof
                         ↘ retain only explicit, machine-readable evidence
```

|Term|Meaning|Canonical form|
|---|---|---|
|desired state|intent compared without touching target|registry data, typed request, plan input|
|observe|read-only account of actual substrate|port + adapter, with structured report|
|reconcile|pure decision from desired and observed state|typed policy or plan|
|bounded action|explicit, reviewable mutation|argv command, declared privilege, explicit `--apply`|
|read-back proof|evidence after action, not a claim of success|versioned JSON report, test, verification|

### Design Gate

Answer from bottom up before adding code, command, crate, or contract:
1. What is desired state, and where is its authoritative representation?
2. What can be observed without mutation, through which portable port?
3. What pure rule selects next safe action from desired and observed state?
4. If mutation needed, what exact argv, privilege boundary, rollback intent, and stop condition?
5. What post-action read-back proves result, and which lower layer produced it?
6. Which existing boundary owns each answer? Extend existing boundaries first.

### Cross-Scale Invariants

- **Truth flows upward; intent flows downward.** Liveness flows only from read-back.
- **Policy is portable; substrate is injected.** Adapters bind systemd, process, VM, LXC, provider, or transport.
- **A surface projects; it does not own.** CLI, MCP, and reports do not own domain policy.
- **Every mutation is a small transaction.** Keep desired state, preflight, apply, observation, proof, and rollback inspectable.
- **Scale changes data and adapters, not the core model.**
- **Unknown is data.** Report residual risk instead of fabricating convergence.
- **Repair flows downward to the violated contract.** Correct the lowest layer expressing the contract.
- **Every bug is evidence of an unmodelled contract.** Add explicit contract and regression proof.

### Causal Dependency Chain (Inviolable Governance Order)

Every valid computation, mutation, and state transition must strictly follow:

```text
Constituent (HumanAdmin / Sovereign Operator)
  └── Identity (CitizenAccount, NodeAccount, AgentAccount)
       └── Capability & Bounds (OfficeAccount, MandateAccount)
            └── Resource Quota & Metabolism (ResourceAccount, TreasuryAccount)
                 └── Workload Admission (AgentWorkloadEnvelope)
                      └── Bounded Action (ArgvRunner, Sandboxed Execution)
                           └── Read-back Observation (Ports & Probes)
                                └── Outcome & Proof (RuntimeProof, EvidenceJournal)
                                     └── State Transition (Deterministic SMR Engine)
                                          └── Materialized View (Embedded SQLite & REST Ingress)
```

Invariants:
1. **No action without mandate**: Agents cannot act without admitted, time-bounded `MandateAccount`.
2. **No mandate without observed resource**: Governance allocates consumption rights, not physical substrate.
3. **No state transition without causal proof**: Read-back proof required before ledger commit.
4. **View derives from log, never inverse**: Materialized state is disposable cache from hash-chained WAL.
5. **Orthogonality of emergent and legacy layers**: New constitutional primitives live in pure `cynic-protocol::peer` without depending on legacy `cynic-host`/`cynic-kernel`.

### Fractal Agent Recursion

- Every runtime node is a sovereign recursion root hosting its own tree of specialized agents.
- Local child delegation uses `cynic.agent_delegation_envelope.v1`.
- A child remains on its parent `node_id`, names its parent, and increments recursion depth once.
- Authority and budgets are monotonic: children may only receive a subset of capabilities and budgets.
- Intent and bounded capability flow downward; evidence and completion flow upward via `cynic.agent_completion_envelope.v1`.
- Model, provider, scheduler, and transport remain adapters.

## Agent Coordination Claims

Store: `.agent-claims.json` at repo root (schema `cynic.agent_claims.v2`).

- **Modes**: `baseline` (cooperative external agents) vs `cynic` (durably enforced, preempts baseline).
- **Lifecycle**: `claim` (hierarchical: fleet → node → crate → module → file with TTL + action budget) → `heartbeat` (renew TTL) → `complete` (`cynic agent release <id>`, enqueues merge).
- **Conflict**: same scope/mode = first-wins; cynic preempts baseline; batch claims atomic.
- **Hierarchy**: parent budget >= sum of children budgets (monotonic).

## Structural Invariants (Immutable Framework Law)

1. **No monolith surfaces**: Split into `types`, `policy`/`plan`, `probe`/`observe`, `decision`, `plan`, `apply`, `render`, thin CLI glue.
2. **No hardcoding when a dynamic contract exists**: Distro, operator user, paths, and packaging modes must be observed or injected.
3. **Dynamic detection before specialization**: Observe substrate first, then select adapter/recipe. Preserve `unknown`/`blocked` if incomplete.
4. **Primary system components must have explicit install authority**: Track principal owner, channel (`scripted`, `external`, `distro_managed`), and observable provenance. Upgrade/repair derives from observed provenance.
5. **Declarative desired state plus observed proof, not blind IaC**: Desired state or commands without read-back proof are not convergence.
6. **One canonical decision source per concern**: Extract shared logic into canonical decision modules; projections must consume, not rebuild.
7. **Adapters are disposable; ports and proofs are durable**: Tool/provider commands are adapter details. Framework law lives in contracts, ports, decision modules, proof schemas, and validation gates.

## Execution Standard

- Mutable work must pass through explicit execution surfaces with explicit `--apply` / `--yes`.
- No hidden shell execution in apply paths; no Python wrapper as runtime authority.
- Commands must be argv-based, typed, observable, and stop on first failure.
- Plans must separate desired state, preflight, apply, observe, proof, and rollback intent.
- Runtime proof generated as machine-readable JSON, validated by tests or verification commands.

## Runtime Harness

Dimensions: `kind` (lab, conformance, staging, canary, production), `authority`, `isolation_boundary`, `target_scope`, `mutates_host_network`.
- `lab`: closed ephemeral local environment; never canonical authority.
- `conformance`: portable contract validation.
- `staging`: production-like VM/LXC context.
- `canary`: bounded subset of fleet nodes with rollback.
- `production`: declared authority, SLO/uptime tracking.

Surfaces:
```bash
cynic runtime harness --json
cynic runtime conformance --json
cynic runtime metrics --json
```

## Agent Session Practice

### Handshake & Coordination
When CYNIC MCP is available:
1. Observe `cynic_mcp_surface`;
2. Register through `cynic_coord_register`;
3. Read back identity via `cynic_coord_who`;
4. Claim affected source target before mutation;
5. Use `cynic_source_control` for admitted commit/push;
6. Use `cynic_validate` to plan validation for `build_runner`.

When MCP is unavailable/unstable: retain read-back as explicit unknown. A human may admit a temporary `human_scoped_fallback` (bounded action/wall-time budget, verify no conflicting baseline claims, git read-back before/after). It is `best_effort_unfenced`, never a lease, and never authorizes runtime/network/release mutation.

### Crate Ownership & Extraction (Systemic)
- Canonical specs: `infra/CRATE_OWNERSHIP.md` and `infra/CRATE_MODELS.md`.
- Run audits before growing any crate:
  - `cynic dev kernel-audit --json` (reports blocked while areas extract/stabilize).
  - `cynic dev ownership-audit --json` (ownership surfaces and migration targets).
  - `cynic dev crate-audit --json` (LOC, hotspots, entrypoints; flags >40k LOC).
- **Precondition**: inventory target crate existing surfaces before writing code. Extend/reuse existing canonical surfaces; never duplicate.

### Git Isolation in Shared Worktrees (Mandatory)
- Never `git add -A` or `git add .`. Stage only your files by explicit path.
- Read back `git status --short`; staged list must equal file list exactly.
- One commit per coherent slice with proof counts in message. Read back with `git show --stat HEAD`.
- Dedicated branch `agent/<name>/<scope>`; open PR when remote reachable.

## Sovereignty vs Relay

- Runtime authority stays local (`cynic`, `/etc/cynic`, harness proofs).
- Sovereign Git: Forgejo (`infra/repo-remotes.json`, port 3000, `forgejo.synapse.cynic`; token lifecycle via `cynic runtime forgejo-token`).
- Public relay: GitHub `origin` while Forgejo connectivity is established.

## Secrets (Industrial, Frictionless, Agnostic)

|Class|When allowed|Examples|
|---|---|---|
|`HumanAdmin`|bootstrap only (console/once)|password, sudo gesture, Forgejo login|
|`MachineEnrollment`|first join / enroll|install token, one-time join key|
|`RecoveryBreakglass`|incident only|emergency root, recovery codes|
|`RuntimeMachineIdentity`|runtime steady-state|llama API key, machine TLS, helper identity|

Runtime must not depend on `HumanAdmin`. Placement priority:
1. Environment injection (process scope): `CYNIC_BACKEND_<NAME>_API_KEY`, `FORGEJO_TOKEN`.
2. Principal-owned file (`0640`, owner `cynic:cynic`): `$PRINCIPAL_HOME/.config/cynic/<name>` or `/etc/cynic/<name>`.
3. OS secret store adapter (systemd-creds, kernel keyring, age/sops).
4. **Never**: git, `infra/agent-exposure.json`, docs, chat JSONL, proof JSON. Reports show `has_api_key=true`, never the secret.

## Inference Supervisor

- Native engines behind `LocalInferenceRuntimePort`:
  - `CYNIC_INFERENCE_SUPERVISOR=systemd` — `llama-server@` user units.
  - `CYNIC_INFERENCE_SUPERVISOR=process` — pidfile/process spawn (lab, fallback if `XDG_RUNTIME_DIR` missing).

## Agent CLI / Host Surface Patterns (Anti-Monolith)

- Layered modules: `types`, `policy`/`plan`, `port` traits, adapters, thin CLI glue.
- Execution ports: domain logic must not call `std::process::Command::new("cynic")`; inject `ArgvRunner` port.
- CLI thinness: clap parse → inject ports → call crate API → print JSON/human.
- Growth thresholds: soft limit 200-250 lines for facade; hard limit 400 lines for any single surface file (extract port/submodule before exceeding).
- Prohibited: monolith `*_lifecycle.rs`/`*_surface.rs`, Python organs as authority, hardcoded paths.

## Industrial Bar

Framework is industrial-grade when:
1. Fresh Linux node bootstraps from standalone `cynic` binary;
2. Privileged changes are bounded, auditable, and reversible;
3. Distro behavior selected by detected capabilities, not hardcoded assumptions;
4. Node/network actions run in closed environments without risking operator connectivity;
5. Runtime reports prove desired state, applied mutation, observation, and resulting health;
6. Distributed formation works for 1 node first, then N nodes without changing core model.

## Validation & Compiler Alphas

- Canonical spec: `infra/RUST_COMPILER_ALPHAS.md`.
- Targeted checks with stripped debuginfo:
  - `RUSTFLAGS="-C debuginfo=0" cargo check -p <crate> --lib`
  - `ulimit -s unlimited && RUST_MIN_STACK=8589934592 RUSTFLAGS="-C debuginfo=0" cargo test -p <crate> --lib <module>`
  - `cynic-kernel` tests require admission: `CYNIC_BUILD_ADMISSION_VERIFIED=1`.
- **Library-First Invariant**: domain types, ports, use cases belong in `src/lib.rs`. `src/main.rs` is thin facade.
- Never mask unverified code with `#![allow(dead_code)]`: dead code is an epistemic sensor.

## Rules

- Do not recreate contract forests.
- Do not reintroduce Python organs as runtime authority.
- Prefer Rust types, tests, and runtime proof over prose.
- Keep infra minimal and executable.
- Anything outside the runtime authority map must prove current runtime use before it is kept.
