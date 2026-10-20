# CYNIC Organic Layer Pattern

**Status:** Active pattern (registry entry `organic-layer-pattern` in `infra/pattern-registry.json`)
**Schema:** `contracts/pattern-registry.v1.json` (`cynic.pattern_registry.v1`)
**Audience:** contributors and agents implementing or extending CYNIC crates, surfaces, and registries.

## Purpose

The organic layer pattern is the bottom-up delivery discipline CYNIC uses to grow
capabilities without speculation. Every change traverses five ordered layers —
**observe → formalize → implement → verify → deliver** — and each layer is gated by
an empirically verified baseline from the layer below. No higher layer may be
introduced on top of an unmeasured substrate.

This document is the human-readable companion to the machine-readable registry
`infra/pattern-registry.json`, which records the pattern's layer progression,
invariants, observed occurrences, and lifecycle status. The registry is the
authority for *what* patterns exist and *where* they occur; this document explains
*how* to apply them.

## The Five Layers

| # | Layer | Question answered | Evidence required |
|---|-------|-------------------|-------------------|
| 1 | **observe** | What is physically true right now? | Raw bytes, exit codes, machine receipts, AST, hardware probes |
| 2 | **formalize** | What contract captures that truth? | Schema, trait, registry entry, typed shape |
| 3 | **implement** | What code honours the contract? | Pure domain logic + adapters behind ports |
| 4 | **verify** | Does it hold under test and audit? | Unit/integration tests, `cynic audit crate`, read-back proof |
| 5 | **deliver** | Is it published and reproducible? | Commit, push, deploy receipt, registry update |

### 1. Observe

Observe the physical substrate before any abstraction. For CYNIC this means reading
actual files, process output, exit codes, storage bytes, and host probes — never
assuming a value, path, or capability exists. The builder SSOT
(`10.10.10.119`) is the only place release builds and runtime truth are produced;
the cortex host is never trusted for build truth because of its faulty RAM.

Representative observation surfaces:

- `crates/cynic-host/src/adapters/` — host probes (`local_hardware_baseline`,
  `local_toolchain_probe`, `meminfo_reader`, `local_cpu`, `local_memory`).
- `crates/cynic-host/src/organic_opportunity/` — observers that report
  unavailable sources distinctly from empty evidence
  (`SourceObservationStatus::Unavailable` vs. `Empty`).

### 2. Formalize

Capture observed truth as a machine-readable contract or a pure Rust type. This is
where port traits, JSON registries, and schemas are authored. Formalization must
not invent values; it only names what observation already established.

Representative formalization surfaces:

- Port traits: `crates/cynic-host/src/ports/`, `crates/cynic-node/src/ports/`,
  `crates/cynic-lifecycle/src/ports/`, `crates/cynic-storage/src/ports/`,
  `crates/cynic-protocol/src/provider/ports.rs`.
- Registries: `infra/registry.json`, `infra/runtime-authority.json`,
  `infra/crate-topology.json`, `infra/pattern-registry.json`.
- Contracts: `contracts/foundation.json`, `contracts/pattern-registry.v1.json`,
  `contracts/crate-fusion-dispositions.json`.

### 3. Implement

Implement behaviour behind the formalized contracts. Domain modules stay pure
(FCIS); side effects live in adapters that implement port traits. Higher layers
depend on lower layers only (strict DAG downward).

Representative implementation surfaces:

- Adapters: `crates/cynic-host/src/adapters/` (local/remote variants for disk,
  CPU, memory, queue, build artefacts, evidence store).
- Fused crates: `crates/cynic-discovery`, `crates/cynic-domain-ops`,
  `crates/cynic-eval` (see `contracts/crate-fusion-dispositions.json`).

### 4. Verify

Prove the implementation against the contract. Verification is mandatory before
delivery and includes unit/integration tests, path-reference audit, and read-back
proof for any mutation.

Commands:

- `make audit-crate` (`cynic audit crate --json`) — native crate audit.
- `make audit` (`cynic audit all`) — full audit.
- `make release.deploy` — release deploy gate.

### 5. Deliver

Publish the verified change: explicit staging of named paths, commit with agent
identity, push to the feature branch, and update the relevant registry/status.
Mutation always requires explicit `--yes`/`--apply` and a read-back proof.

## Invariants

The pattern is bound by the CYNIC invariants (see `AGENTS.md` and
`contracts/foundation.json`):

1. **Bottom-up over top-down.** Observe and measure physical substrate reality
   before introducing any abstraction, model, or policy.
2. **Strict DAG downward.** Higher layers depend on lower layers; never the
   reverse.
3. **FCIS / Zero-I/O core.** Pure logic lives in domain modules; all side
   effects sit behind ports in `adapters/`.
4. **No hardcoded values.** Hosts, IPs, paths, and tool lists come from dynamic
   registries (`infra/registry.json`, `infra/runtime-authority.json`,
   `infra/crate-topology.json`), never literals.
5. **No mutation without read-back proof.** Every write is followed by a
   verified read-back before the change is considered done.
6. **Typed argv execution.** Sibling CLI surfaces are invoked through an injected
   argv runner port, never hard-wired `Command::new`.
7. **Explicit mutation gates.** Destructive or state-changing operations require
   `--yes`/`--apply`; no silent auto-repair.

## Pattern Registry

`infra/pattern-registry.json` is the machine-readable SSOT for recurring CYNIC
implementation patterns. It conforms to `contracts/pattern-registry.v1.json`
(schema `cynic.pattern_registry.v1`) and records, per pattern: `pattern_id`,
`name`, the ordered `layers` array, `invariants`, `occurrences`, `status`,
`created_at`, `updated_at`.

Patterns registered there:

| pattern_id | name | key invariants (abridged) |
|------------|------|---------------------------|
| `organic-layer-pattern` | Organic Layer Pattern | bottom-up; no mutation without read-back proof; gated layers; strict DAG |
| `port-trait` | Port Trait (FCIS) | pure trait in domain module; side effects behind adapters; admission gate; proof builder |
| `adapter-degrossing` | Adapter Degrossing | adapters move to dedicated crates; domain crates contain no I/O; downward deps only |
| `registry-first` | Registry First | dynamic registries over hardcoded values; registry is authority SSOT; `--yes`/`--apply` |
| `fusion` | Crate Fusion | merge zero-test/no-dependents crates; reduce fragmentation; regenerate `Cargo.lock` |

When adding or updating a pattern, update `infra/pattern-registry.json` first, then
reference it here. Never let prose and registry drift.

## Concrete Examples from This Session

### Port traits

- `crates/cynic-host/src/ports/argv_execution.rs` defines `ArgvExecutionPort`
  with `LocalArgvExecutionAdapter` — typed argv execution with redacted stdin in
  `Debug`, positive-timeout enforcement, and a `success()` predicate.
- `crates/cynic-host/src/organic_opportunity/port.rs` defines
  `OrganicOpportunityObserver` with `SourceObservationStatus<T>`, which keeps
  unavailable sources distinct from genuinely empty evidence.
- Additional port families: `crates/cynic-node/src/ports/` (build runner,
  storage lifecycle, enrollment store, credential broker, mesh, …),
  `crates/cynic-lifecycle/src/ports/`, `crates/cynic-storage/src/ports/`.

### Adapters

- `crates/cynic-host/src/adapters/` holds the side-effecting implementations:
  `local_disk`, `remote_disk`, `local_cpu`, `remote_cpu`, `local_memory`,
  `remote_memory`, `local_queue`, `remote_queue`, `local_evidence_store`,
  `artifact_publisher_ssh/http`, `local_build_artefact`, `remote_build_artefact`,
  `source_admission_adapter`, `local_task_admission`, `cgroup_isolation`, …
- Each adapter implements a port trait from `crates/cynic-host/src/ports/`; no
  adapter leaks into the domain layer.

### Registries

- `infra/registry.json` — hosts, IPs, paths, fleet desired state (authority rank 4).
- `infra/runtime-authority.json` — runtime authority ordering.
- `infra/crate-topology.json` — crate census.
- `infra/pattern-registry.json` — this pattern and its siblings.

### Fusions

- `contracts/crate-fusion-dispositions.json` records the fusion decisions (e.g.
  `cynic-inference-runtime` delete, `cynic-domain-askesis` fuse into
  `cynic-askesis`, `cynic-channel` fuse), measured dependents, and the 5-condition
  justification rule from `contracts/pruning-policy.json`.
- Fused crates observed: `crates/cynic-discovery`, `crates/cynic-domain-ops`,
  `crates/cynic-eval`; `Cargo.lock` regenerated for the fused workspace.

## Layer Walkthrough: adding a port trait

1. **Observe.** Measure the need: e.g. sibling CLI surfaces currently shell out
   via `Command::new("cynic")`; host probes return concrete bytes/exit codes.
2. **Formalize.** Author the port trait in `crates/<crate>/src/ports/` with
   request/response shapes and a `Send + Sync` bound; add registry entry if a new
   pattern emerges.
3. **Implement.** Add a `Local…Adapter` (and remote variant if needed) in
   `adapters/`; keep the domain crate free of I/O.
4. **Verify.** Unit-test with a fake adapter returning fixed JSON; run
   `make audit-crate`; confirm path references resolve.
5. **Deliver.** Stage the named paths, commit as
   `hermes@agent.local`, push to `agent/hermes/aegis-consortium`, update
   `infra/pattern-registry.json` if occurrences changed.

## Build & Verification Policy

- **Builder SSOT:** `10.10.10.119`. Never run `cargo`/`rustc` on the cortex host.
- Fast check: `RUSTFLAGS="-C debuginfo=0" cargo check -p <crate> --lib`.
- Canonical gates: `make audit-crate`, `make audit`, `make release.deploy`.
- `make build` / `make build-release` do not exist.

## Related Documents

- `AGENTS.md` — minimal agent protocol and gates.
- `docs/design/agent-cli-patterns.md` — CLI/host surface patterns.
- `docs/design/CANONICAL_CRATE_MAP.md` — crate census and fusion context.
- `contracts/foundation.json` — constitutional authority.
- `infra/pattern-registry.json` — machine-readable pattern SSOT.