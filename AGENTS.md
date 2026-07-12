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
  compatibility projection.
- Every non-trivial change should move one concrete capability closer to:
  standalone bootstrap, OS portability, agnostic execution, distributed runtime
  formation, or runtime proof.
- If coordination storage is unavailable, say so explicitly and do not pretend
  the organism remembered the session.
- Do not create backlog Markdown as memory; use executable tests, contracts, or
  runtime state instead.

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

## Rules

- Do not recreate contract forests.
- Do not reintroduce Python organs as runtime authority.
- Prefer Rust types, tests, and runtime proof over prose.
- Keep infra minimal and executable.
- Anything outside the runtime authority map must prove current runtime use
  before it is kept.
