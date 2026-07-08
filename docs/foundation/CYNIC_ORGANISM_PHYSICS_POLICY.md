CYNIC Organism Physics Policy

Machine contract: `axioms/organism-physics-v1.json`.

Purpose: describe the physical invariants that distinguish a local code mutation
from a real organism mutation. This policy is not metaphor-first. It is an
engineering lens for propagation, coupling, dissipation, inertia, and closed
control loops.

## Why this exists

The organism already has lifecycle, topology, anti-polling, and runtime role
contracts. Those contracts explain ownership and allowed placement. They do not
yet state a compact rule for when a change is physically real across the whole
organism.

This policy fills that gap.

## The physical invariants

1. Causal closure

A mutation is not real when it only exists in the cortex checkout. It becomes
real only when the signal propagates across the declared lifecycle chain and is
proven by applied and verified read-back.

2. Energy discipline

Polling, duplicated compatibility logic, and competing capability vocabularies
are energy leaks. Transitional leaks may exist, but they must be declared as
debt and converge toward watch, push, lease, callback, or event membranes.

3. Coupling coherence

Interfaces between kernel, MCP, runtime frameworks, and organs must converge on
the same semantic primitive. If one layer is `watch` and the next layer still
exposes only `list`, the signal is only partially propagated.

4. Inertia visibility

Legacy env vars, systemd settings, or fallback code are allowed only when the
organism explicitly marks them as transitional inertia. Hidden inertia is drift.

5. Closed-loop control

Every canonical mutation path should converge toward:

`observe -> decide -> dispatch -> execute -> read back -> correct`

If a loop stops at local validation, manual inference, or unverified success,
the loop is open and the organism remains physically incoherent.

## Required consequences

- A canonical kernel watch surface must eventually propagate to cortex-facing
  MCP or another canonical consumer surface.
- A runtime that moved to `wait/watch` must stop advertising `poll_tasks` as
  its default capability vocabulary.
- Hermes execution status should converge from `kanban show` polling to a
  blocking or streamed status membrane.
- Lifecycle claims about freshness or deployment are incomplete while the
  freshness controller and lifecycle proof chain remain `planned` or lack
  verified read-back.

## Audit posture

The supervisor may emit warnings for declared transitional debt. Those warnings
are not false positives. They are the machine-readable expression of physical
misalignment that still exists intentionally.
