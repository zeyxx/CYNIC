# CYNIC Meta-Layer Contracts

Status: normative foundation.

This document formalizes where CYNIC rules live, how they compose, and which
surface is allowed to decide what. It removes ambiguity between policy,
contract, runtime projection, and execution surface.

Machine contract: `axioms/meta-layer-contract-v1.json`.

## 1. Meta-Layer Rule

Every durable concern must be classified in one of these layers:

| Layer | Purpose | Canonical home | May decide |
|-------|---------|----------------|------------|
| Policy | Cross-cutting governance and boundaries | `docs/foundation/` | what is allowed, owned, or forbidden |
| Semantic contract | Machine-readable meaning | `axioms/` | payload shape, contract vocabulary, invariants |
| Topology contract | Where concerns live | `axioms/repo-topology-v1.json`, related topology axioms | file placement, root classes, organ boundaries |
| Runtime registry | Desired state and placement | `infra/registry.json`, `infra/agent-exposure.json` | declared nodes, services, exposure, lifecycle targets |
| Kernel contract | Coordination, memory, judgment | `crates/cynic-kernel/` and kernel API docs | tasks, claims, observations, health, read-back |
| MCP surface | Portable client-facing tool surface | `crates/cynic-mcp/` | which tools are exposed and how they are validated |
| Organ runtime | Persistent subsystem implementation | `organs/<role>/` | local behavior within the declared contract |
| Node runtime | Per-machine execution agent | `crates/cynic-node/` and node docs | local supervision, heartbeats, command reception |
| Surface wrapper | Operator convenience or compatibility | `Makefile*`, `scripts/`, transitional wrappers | invocation ergonomics only |

Rule: lower layers implement higher layers; they do not redefine them.

Topology is a separate axis from function. CYNIC uses three distinct topology
surfaces:

- repo topology: which directory owns the concern;
- organ layer topology: which primary rank owns an immediate organ;
- axiom topological layers: which graph position a root axiom contract occupies
  in `axioms/axiom-registry-v1.json`.

## 2. Contract Precedence

When two sources seem to disagree, use this order:

1. Source-of-truth hierarchy for external truth claims.
2. Organism policy matrix for cross-cutting rules.
3. Repo topology policy for file placement.
4. Domain-specific axioms for machine contracts.
5. Runtime registry for placement and exposure.
6. Kernel state for observed current truth.
7. Wrappers and docs only as projections.

This means:

- a wrapper can simplify a command, but it cannot invent semantics;
- a runtime can project state, but it cannot become the policy owner;
- a node can execute or supervise locally, but it cannot replace the orchestrator;
- the kernel can judge and coordinate, but it is not the primary source of legal,
  protocol, or fleet truth.

## 3. Where Commands Should Live

The correct home for a command depends on what it decides:

| Command type | Should live in | Example |
|--------------|----------------|---------|
| Governance rule | policy docs | layer boundaries, fallback policy |
| Machine contract | `axioms/` | lifecycle payloads, topology inventories |
| Fleet placement | registry | node/service desired state |
| Portable tool exposure | MCP surface | `cynic_coord_claim`, `network_mesh_status` |
| Local runtime behavior | organ or node runtime | heartbeats, command polling, supervision |
| Human shortcut | wrapper | `cynic bootstrap`, `cynic codex` |

If a command is reusable but still mutates runtime or dispatches work, it belongs
in a contract or runtime projection, not in a prose-only doc.

## 4. Soma and cynic-node

The architecture is asymmetric:

- **Soma** is the central orchestrator.
- **cynic-node** is the per-machine executor and supervisor.

Interpretation:

- Soma decides allocation, orchestration and state convergence.
- cynic-node runs on each machine, receives commands, supervises local
  processes, and reports observations.
- The node is a hand and a sensor. It is not the brain.

This is the same pattern everywhere in the organism:

- policy lives above execution;
- contracts define the meaning;
- registries define placement;
- runtimes project and enforce;
- wrappers expose ergonomics.

## 5. Native Default Surface

When the organism exposes a task as a native surface, that surface should be the
default entrypoint, not an optional mode switch.

Examples:

- `cynic codex` is the native Codex projection.
- `cynic agy` is the native Antigravity projection.
- `cynic bootstrap` is the operator projection for runtime
  principal convergence.

The wrapper may hide boilerplate, but the durable truth must remain in the
contract and runtime layers above.

## 6. Design Pressure

Use this document to decide whether a new file belongs in:

- `docs/foundation/` for policy;
- `axioms/` for machine contracts;
- `infra/registry.json` for desired state;
- `crates/cynic-kernel/` or `crates/cynic-mcp/` for runtime contracts;
- `organs/<role>/` or `crates/cynic-node/` for execution;
- `Makefile*` or `scripts/` only for wrappers.

If a proposed file would mix these responsibilities, split it before adding new
behavior.

For empirical multi-scale projection work, pair this file with
`docs/foundation/CYNIC_MULTI_SCALE_EMPIRICAL_RULE.md` and
`axioms/multiscale-organism-property-map-v1.json`.

## 7. The "Small Chunk" Recursive Principle

*Small chunk of a big thing with massive upside is much better than a big chunk of a small thing that won't go anywhere.*

CYNIC execution is recursive and atomic, not monolithic. A monolithic agent with an infinitely growing context window is a "big chunk of a small thing" that inevitably degrades. The CYNIC organism (the "big thing with massive upside") scales by dispatching "small chunks" (pure, isolated intents) through the Kernel Mempool.

- **Autonomous Organs (`hermes-agent`)**: Must not accumulate global context. They wake up, claim a specific intent from the mempool, execute their highly bounded skill (`supervised_cortex_delegate`), yield the synthesized result, and die. (`mercury-agent` is the Cosmicstack infra scheduler — a distinct project, NOT a CYNIC autonomous organ; do not conflate the two.):
- **CLI Agents (`codex`, `antigravity`)**: Interactive sessions must follow the same rule. Instead of looping endlessly over complex problems in the same context, the interactive cortex should decompose the problem and dispatch sub-tasks to the organism's mempool, acting as an orchestrator (Recursive Reactor) rather than a monolithic doer.
- **Quantum Integrity**: A spawned sub-agent is initialized with a blank slate, equipped only with the exact context (e.g., a specific RAG bundle) and tools needed to resolve its atomic intent.
