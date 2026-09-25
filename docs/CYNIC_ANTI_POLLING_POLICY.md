# CYNIC Anti-Polling Policy

Status: normative foundation.

Machine contract: `axioms/anti-polling-transition-v1.json`.

## Rule

New canonical runtime and service surfaces must not introduce polling when an
event-driven membrane is feasible.

Allowed preferred shapes:

- blocking wait
- subscription
- watch stream
- callback
- push queue
- lease-backed wakeup

Disallowed default shape:

- periodic polling loops introduced as the first choice

## Scope

This policy is stricter for:

- `services/*-runtime/`
- systemd units that wire canonical runtime behavior

It is transitional, not yet absolute, for older organ surfaces that still carry
legacy polling debt.

## Important Distinction

Heartbeat is not the same thing as polling.

- heartbeat/liveness refresh: acceptable when it only proves service freshness
- work polling: debt that must be declared and migrated away

## Transitional Debt

Every remaining polling surface must be listed in
`axioms/anti-polling-transition-v1.json` with:

1. current mechanism
2. target mechanism
3. status

If a polling surface is not declared there, it is treated as accidental
fragmentation.

## Current Direction

The immediate target is:

1. keep `coordination-runtime` generic
2. remove domain-specific polling compatibility logic from shared runtimes
3. route task intake through a blocking wait/watch port first, then remove the
   transport-level polling fallback once the kernel exposes native watch
 membrane
4. migrate Hermes execution status from kanban polling toward event emission or
   blocking wait semantics
