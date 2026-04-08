# CYNIC Organism Contract

Purpose: make CYNIC's health and ops truth explicit.

This document answers four questions for each critical component:
- what role it serves
- what class of criticality it has
- who consumes it
- what failure effect it has

## Criticality Classes

| Class | Meaning |
|---|---|
| `startup-critical` | Must be available for a clean boot path |
| `readiness-critical` | Required for CYNIC to honestly claim it can serve requests |
| `liveness-critical` | If absent, the process itself should be considered dead or wedged |
| `diagnostic-only` | Important for visibility, but should not by itself make the service unready |
| `housekeeping` | Needed for cleanup, reconciliation, and accuracy over time, not immediate serving |
| `startup-one-shot` | Boot or repair work that must stay visible but should not age into permanent readiness failure once successfully finished |

## Runtime Components

| Component | Role | Criticality | Consumer | Failure Effect |
|---|---|---|---|---|
| `cynic-kernel` | Kernel API and composition root | startup-critical, readiness-critical, liveness-critical | users, hooks, agents, ops | service unavailable |
| `surrealdb` | persistent storage | readiness-critical | kernel, audits, crystals, coord | health becomes critical, writes cannot be trusted |
| `llama-server` | sovereign inference backend | readiness-critical | judge path, health loop | service degrades, routing confidence drops |
| `embedding-backend` | embedding generation | diagnostic-only | backfill, memory ops | serving can continue, semantic enrichment lags |
| `coordination` | claim and ownership discipline | housekeeping | hooks, multi-agent edits | stale claims or edit discipline drift |
| `runtime-truth` | unified runtime diagnosis | diagnostic-only | operators | drift exists but remains harder to see |

## Background Tasks

| Task | Role | Criticality | Consumer | Failure Effect |
|---|---|---|---|---|
| `health_loop` | updates live Dog reachability and breaker truth | readiness-critical | judge, `/ready`, remediation | dead Dogs can still look routable, readiness lies |
| `probe_scheduler` | refreshes environment and fleet probes | readiness-critical | `/health`, backend quality gates | environment truth drifts from runtime |
| `dog_ttl` | removes expired dynamic Dogs | readiness-critical | roster, judge, `/health` | expired Dogs can remain in active rotation |
| `coord_expiry` | expires stale claims | housekeeping | coord system | stale ownership lingers |
| `usage_flush` | persists usage/cost telemetry | housekeeping | ops, accounting | cost truth lags |
| `rate_eviction` | evicts stale limiter buckets | housekeeping | REST limiter | limiter state drifts, memory grows |
| `remediation` | attempts backend recovery | housekeeping | backend recovery path | recovery becomes slower, not immediately unready |
| `summarizer` | compacts session memory | diagnostic-only | session summaries | memory quality degrades, serving continues |
| `introspection` | clusters anomalies and emits alerts | diagnostic-only | operators | incidents become noisier and less actionable |
| `event_consumer` | proves internal event bus liveness | diagnostic-only | tracing and lag visibility | observability loses a consumer proof, serving continues |
| `backfill` | one-shot crystal embedding repair | startup-one-shot | crystal search quality | old crystals remain incomplete |

## Rules

1. `diagnostic-only` and `housekeeping` tasks must remain visible in `/health`, but they must not fail `/ready` unless they become direct serving dependencies.
2. `readiness-critical` tasks must be explicitly listed in code, never inferred by omission.
3. A unit profile counts only if it is restart-safe under real `systemd --user` restarts.
4. `/ready` should stay cheap and binary; `/health` must explain the cause.
5. Any new loop must declare its class, consumer, and failure effect in code and in this contract.

## Immediate Consequences

- `event_consumer` stays visible but no longer degrades readiness by itself.
- `health_loop`, `probe_scheduler`, and `dog_ttl` remain readiness-critical.
- systemd hardening is advisory until a strict profile is proven restart-safe.
- future refactors must split by these authorities, not by helper count.
