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
