# CYNIC Agentic Runtime Roles

Status: normative foundation.

Machine contract: `axioms/agentic-runtime-topology-v1.json`.

This document defines the canonical agentic roles and shared runtimes for
CYNIC. Its purpose is to stop role drift, runtime duplication, and accidental
second anatomies such as domain logic, memory, policy, and execution framework
living in the same path.

## 1. Core Rule

Every persistent or reusable software surface in CYNIC must be classified in
two independent ways:

1. by **agentic role**: what responsibility it performs in the organism;
2. by **runtime class**: what shared execution framework or membrane it uses.

If a component cannot be classified cleanly, it is not mature enough to extend
safely.

## 2. Agentic Roles

### Cortex

- Purpose: episodic reasoning, DNA mutation, operator arbitration.
- May own:
  - design decisions
  - policy exceptions
  - direct repo mutation after coordination claims
- Must not own:
  - durable runtime supervision
  - canonical memory transport
  - autonomous deploy loops

### Judge

- Purpose: evaluate, object, score, or gate.
- May own:
  - policy evaluation
  - topology validation
  - readiness verdicts
  - supply-chain objections
  - proof requirements
- Must not own:
  - direct infra mutation
  - domain execution side effects
  - hidden memory ledgers

### Executor

- Purpose: perform a bounded task.
- May own:
  - task execution
  - subprocess/session handling
  - bounded retries and budgets
  - structured task result publication
- Must not own:
  - policy canon
  - organism memory canon
  - deploy authority by itself

### Supervisor

- Purpose: observe organism drift and dispatch corrective work.
- May own:
  - fragmentation detection
  - stale-runtime detection
  - missing-evidence detection
  - structural task dispatch
- Must not own:
  - secret policy forks
  - unbounded product/domain logic

### Actuator

- Purpose: apply a bounded mutation to runtime or infrastructure.
- May own:
  - deploy/apply/restart actions
  - service and node operations
  - storage/network bounded mutation
- Must not own:
  - governance rules
  - autonomous product intent
  - proof-free success claims

### Archivist / Askesis

- Purpose: append, compress, and replay organism memory and proof lineage.
- May own:
  - structured observation append
  - anti-forgetting memory flows
  - replayable evidence bundles
  - semantic compression
- Must not own:
  - execution of unrelated domain work
  - policy gating by itself
  - direct deployment authority

### Platform

- Purpose: provide the substrate the organism runs on.
- May own:
  - node placement
  - mesh, DNS, IPAM, storage activation
  - build-runner and deploy-target placement
- Must not own:
  - domain semantics
  - agentic policy interpretation beyond declared contracts

### Reliability / SRE

- Purpose: preserve organism homoeostasis under load, drift, or failure.
- May own:
  - health, readiness, liveness policy
  - saturation and backpressure policy
  - rollback and degraded-mode criteria
  - incident evidence and operational budgets
- Must not own:
  - domain product behavior
  - legal/protocol governance canon

### Observability

- Purpose: make organism state, evidence, and degradation legible.
- May own:
  - telemetry envelopes
  - evidence envelopes
  - introspection surfaces
  - trace, event, heartbeat and metric normalization
- Must not own:
  - domain execution itself
  - policy canon
  - deployment placement

## 3. Runtime Classes

Shared runtimes belong under `services/` and must remain framework surfaces,
not second organ trees.

### `execution-runtime`

- Purpose: reusable execution loop and task-running mechanics.
- Owns:
  - scheduling loops
  - subprocess/session control
  - execution budgets
  - retries/backoff
  - runtime heartbeats
- Must not own:
  - domain logic
  - policy canon
  - durable askesis memory

### `judge-runtime`

- Purpose: reusable validation and verdict mechanics.
- Owns:
  - verdict envelopes
  - objections
  - scores
  - gate evaluation
  - proof checks
- Must not own:
  - actuation
  - policy authorship
  - domain work execution

### `askesis-runtime`

- Purpose: reusable structured memory and proof-lineage mechanics.
- Owns:
  - append contracts
  - semantic compression
  - replay metadata
  - anti-forgetting memory hooks
- Must not own:
  - policy
  - unrelated execution logic
  - deployment side effects

### `actuation-runtime`

- Purpose: reusable bounded mutation mechanics.
- Owns:
  - deploy/apply/restart primitives
  - bounded side-effect envelopes
  - rollback hooks
  - mutation proof envelopes
- Must not own:
  - product/domain semantics
  - operator governance

### `observability-runtime`

- Purpose: reusable telemetry, evidence, and introspection mechanics.
- Owns:
  - event/metric/trace envelopes
  - evidence emission
  - health evaluators
  - heartbeat normalization
  - introspection builders
- Must not own:
  - domain execution logic
  - policy authorship

### `reliability-runtime`

- Purpose: reusable liveness, readiness, and degraded-mode mechanics.
- Owns:
  - service-liveness membranes
  - health/readiness/liveness helpers
  - degraded-mode signals
  - operational incident evidence helpers
- Must not own:
  - domain execution logic
  - policy authorship

### `coordination-runtime`

- Purpose: reusable client-side coordination mechanics around the kernel bus.
- Owns:
  - register/claim/release client flows
  - task state-transition helpers
  - read-back verification helpers
  - coordination proof helpers
- Must not own:
  - kernel authority itself
  - domain product behavior

### `policy-runtime`

- Purpose: reusable machine enforcement of policy and permissions.
- Owns:
  - side-effect permission checks
  - fallback policy checks
  - forbidden-surface checks
  - compliance helpers
- Must not own:
  - policy doctrine
  - hidden deployment logic

### `integration-runtime`

- Purpose: reusable external-system membranes.
- Owns:
  - browser/computer-use bridges
  - external API/session normalization
  - adapter envelopes for Slack/Telegram/X/browser/etc.
- Must not own:
  - domain semantics
  - memory canon
  - policy canon

## 4. Canonical Placement

### Policy and contract layers

- `docs/foundation/`: human-readable doctrine and role boundaries
- `axioms/`: machine-readable runtime/role contracts
- `organs/contracts/`: shared Python ports, adapters and envelopes

### Runtime layers

- `services/<runtime>/`: shared runtime frameworks only
- forbidden shape: `services/.../organs/...`

### Organ layers

- `organs/<role>/`: persistent bounded subsystem with domain ownership
- one organ may consume multiple runtimes, but it may not redefine them

### Workspace layers

- experimental, data-heavy, heuristic or lab surfaces must live in an explicit
  noncanonical workspace layer
- they must not silently become production runtime canon

## 5. Current Mapping Pressure

The current pressure points are:

- `services/coordination-runtime/src/cynic_coord_runtime/`,
  `services/observability-runtime/src/cynic_observe_runtime/`, and
  `services/reliability-runtime/src/cynic_reliability_runtime/` are the current
  first-class runtime roots. New persistent runtimes must extend one of these or
  justify a new declared runtime class.
- `services/coordination-runtime/src/cynic_coord_runtime/` now owns the Kernel
  task/claim/register membrane; Hermes-specific repo-claim policy has been
  moved above it into execution-owned composition.
- `services/observability-runtime/src/cynic_observe_runtime/` now owns Kernel
  observation and Hermes execution-event publication, but broader health and
  introspection mechanics are not yet centralized there.
- `services/reliability-runtime/src/cynic_reliability_runtime/` now owns the
  Kernel service-liveness membrane, but broader readiness, degraded-mode and
  rollback helpers are not yet centralized there.
- Hermes-specific coordination policy must compose generic contracts from
  `organs/contracts/` and the shared coordination runtime instead of creating a
  hidden fourth Python runtime root.
- `organs/contracts/` already owns shared Python contracts, which means shared
  runtime code should not masquerade as a second organ tree.
- `services/cynic-python/` currently mixes runtime framework, datasets,
  heuristics, labs, and agent surfaces, so it is not one coherent ownership
  class.

Therefore:

1. no new canonical behavior should be added under the old
   `services/cynic-python/organs/` shape;
2. shared execution framework code should migrate toward explicit runtime roots
   under `services/`;
3. non-runtime surfaces in `services/cynic-python/` should either move to a
   canonical organ owner or be quarantined as workspace material.

## 6. Minimal Runtime Set

The minimum canonical runtime set for the organism is:

1. `execution-runtime`
2. `judge-runtime`
3. `askesis-runtime`
4. `actuation-runtime`
5. `observability-runtime`
6. `reliability-runtime`
7. `coordination-runtime`

`policy-runtime` and `integration-runtime` are canonical extensions when the
shared enforcement or external-membrane pressure becomes large enough to justify
their own runtime roots.

## 7. Review Checklist

Before adding a new runtime, organ, daemon, judge, executor, or adapter:

1. Which agentic role owns it?
2. Which runtime class owns its shared mechanics?
3. What decisions may it take?
4. What decisions are forbidden?
5. What proof or read-back must it emit?
6. Why is it not already covered by an existing runtime or contract?

If those answers are not explicit, the surface is not ready to become
canonical.
