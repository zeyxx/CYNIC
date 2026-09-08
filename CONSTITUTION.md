# CONSTITUTION — CYNIC Sovereign Architecture

> **Authority**: This document is Layer 0 normative prose. It is subordinate to
> `contracts/foundation.json` and `infra/runtime-authority.json`. It outranks
> all other docs, AGENTS.md, and crate-level READMEs.
>
> **Amendment**: This constitution can only be amended by the Sovereign (Human).
> See §Amendment below.

---

## Article I — The Sovereign

The human is the **sovereign authority**. The human is not a power among powers —
the human is the **constituent power** that renders the constituted powers possible.

The sovereign:
- **Ratifies** this constitution and its amendments
- **Vetoes** any action by any constituted power
- **Admits** or **revokes** any mandate
- **Does not execute** — execution is the domain of the constituted powers

The sovereign cannot order an unconstitutional action. The judiciary (kernel) must
validate that any sovereign directive respects the constitutional bounds. This
prevents the sovereign from accidentally destroying the system it governs.

---

## Article II — The Five Powers

All activity in CYNIC is governed by five separated powers. No power may perform
the function of another. No power may amplify its own bounds.

### §2.1 — Constituent Power (Protocol)

**Owner**: `cynic-protocol`

Defines the types, ports, traits, and bounds that all other powers consume.
Contains zero I/O, zero side effects, zero domain logic.

**May**: Define types, define traits (ports), define constants
**May NOT**: Execute logic, perform I/O, modify its own bounds, contain domain logic

### §2.2 — Judicial Power (Kernel)

**Owner**: `cynic-kernel`

The state machine. Observes transitions, validates them against the constitution,
maintains the immutable evidence journal.

**May**: Validate transitions, maintain state, emit events, store proofs
**May NOT**: Initiate actions, modify the constitution, self-execute

### §2.3 — Executive Power (Node + Agents)

**Owner**: `cynic-node`, vertical agents

Executes admitted actions within the bounds of their mandates. Consumes resources,
produces proofs.

**May**: Execute admitted workloads, consume allocated resources, emit completion proofs
**May NOT**: Modify its own bounds, self-allocate resources, override judicial veto

### §2.4 — Legislative Power (Observer + Meta-Agent)

**Owner**: `cynic-observer`, meta-agent

Observes the state, detects violations, proposes amendments and extractions.

**May**: Observe, audit, propose changes, alert the sovereign
**May NOT**: Execute its own proposals, modify the constitution directly

### §2.5 — Control Power (Harness of Harnesses)

**Owner**: Each harness level validates the level below

No power may self-judge. Each level is the conformance gate of the level beneath it.

**May**: Validate conformance, trigger rollback, escalate violations
**May NOT**: Self-validate, modify the constitution

---

## Article III — Separation of Powers

### §3.1 — No self-amplification

A power may never modify its own bounds. To modify bounds:
1. Legislative proposes
2. Judicial validates constitutionality
3. Sovereign ratifies
4. Executive implements

### §3.2 — No self-judgment

A power may never be the judge of its own actions. The conformance of power N
is always observed by power N+1.

### §3.3 — Monotonic authority

A child power may receive a subset of its parent's capabilities, mutation authority,
context budget, action budget, and wall-time budget. It may never amplify them.

---

## Article IV — The Constitutional Protocol

### §4.1 — What is constitutional

The following are constitutional (Layer 0):
- Type definitions in `cynic-protocol`
- Port traits (the contracts that adapters implement)
- The KernelEvent enum (the vocabulary of state transitions)
- The five powers and their bounds (this document)

### §4.2 — Amendment process

1. **Proposal**: Any power except the judicial may propose an amendment
2. **Validation**: The judicial validates that the amendment respects existing
   constitutional bounds (no self-amplification, no circular dependency)
3. **Ratification**: The sovereign ratifies or vetoes
4. **Implementation**: The executive implements the amendment
5. **Proof**: The control power validates post-implementation

### §4.3 — Emergency amendment

In case of critical security violation, the judicial may temporarily suspend
a power's mandate. The sovereign must ratify or revoke within 24 hours.
If not ratified, the suspension is lifted and the mandate restored.

---

## Article V — Mandates and Resources

### §5.1 — MandateAccount

Every agent, every power, every workload operates under a MandateAccount that defines:
- **Scope**: What the agent may act upon
- **Bounds**: What mutations are permitted
- **Budget**: Resources allocated (compute, memory, time)
- **Duration**: Time-bounded, revocable

### §5.2 — ResourceAccount

Resources are allocated, not created. The TreasuryAccount:
- Observes physical substrate (via host probes)
- Issues consumption rights proportional to observed capacity
- Cannot issue more than physically observed (no monetary inflation)

### §5.3 — No mandate without observed resource

A mandate cannot allocate compute, memory, or energy that is not physically
observed in a ResourceAccount. Governance allocates consumption rights; it does
not manufacture physical substrate.

---

## Article VI — The Market

### §6.1 — Internal market

Agents exchange resources via an internal market. The currency is compute credits
issued by the Treasury against observed physical capacity.

### §6.2 — Price discovery

Agents bid for resources. The market clears at the price that balances supply
(observed capacity) and demand (agent bids).

### §6.3 — Proof of value

An agent that consumes resources must produce a proof. Agents that consume
without producing are detected by the legislative and their budgets reduced.

---

## Article VII — The Meta-Agent

### §7.1 — Role

The meta-agent is the **prime minister** — it coordinates, it does not rule.

### §7.2 — Powers

- **Observe**: Read audits, metrics, proofs
- **Propose**: Submit extractions, reallocations, amendments
- **Alert**: Notify the sovereign of violations or blockages
- **Facilitate**: Connect agents via the internal market

### §7.3 — Bounds

The meta-agent may NOT:
- Modify its own bounds
- Allocate resources directly
- Override a judicial veto
- Self-promote or self-amplify

---

## Article VIII — Invariants

The following are inviolable. Any amendment that violates them is unconstitutional.

1. **No action without mandate**: No agent acts without an admitted, time-bounded,
   unrevoked mandate.
2. **No mandate without observed resource**: No mandate allocates unobserved substrate.
3. **No state transition without proof**: No transition commits without fresh read-back.
4. **No self-amplification**: No power may expand its own authority.
5. **No self-judgment**: No power may validate its own conformance.
6. **View derives from log**: Materialized state is a disposable cache; the ledger owns truth.
7. **Unknown is data**: Coverage, capability, and proof freshness may be unknown;
   report residual risk instead of fabricating convergence.
8. **Explicit over implicit**: The implicit is debt. Every intent, decision, gap,
   budget, recipient, schema version, and migration step is explicit, typed, and
   machine-readable, or recorded as an owned explicit unknown with due scale.
   Guessing where a type, test, contract, or proof could answer is
   unconstitutional.

---

## Article IX — Fractal Application

This constitution applies at every scale:

| Scale | Constitution | State Machine | Proof |
|---|---|---|---|
| Rust function | Input invariants | Pure function | Unit test |
| Crate | `cynic-protocol` types | Crate logic | `cargo test` |
| Agent | Mandate | Agent runtime | Completion proof |
| Fleet | Registry | Node plans | Health bundle |
| Organism | This constitution | CYNIC runtime | Audit report |

Each scale reuses the same shape. Each scale is validated by the scale above.
