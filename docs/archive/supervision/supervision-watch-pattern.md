# Supervision watch pattern — producer/consumer reference (2026-10-16)

How CYNIC supervises, formally: a producer emits, a consumer decides, an
outer ring restarts, an external eye audits. Four rings, one direction.

## Rings

1. **Producer** (`cynic supervise run`): holds the child, publishes the
   status envelope atomically (`cynic.supervisor_parent_status.v1`) at
   `heartbeat_interval_secs` (clamped 1–5s). Owns no opinion about being
   watched.
2. **Consumer** (`cynic supervise watch`): polls the status file, decides
   per heartbeat age, appends ledger entries
   (`cynic.supervisor_watch_ledger.v1`, JSONL). Observes and records
   only — never restarts, never signals. A consumer that acts is a
   second producer.
3. **Outer ring** (systemd): owns restart for both producer and watcher
   units, independently. `host_supervisor_relationship` already decided:
   systemd stays outer, CYNIC owns CYNIC children and observations.
4. **External audit** (Hermes cron, local-only): daily one-line verdict
   from the ledger. Evidence trail, not detection.

## Consumer decision (pure)

Inputs: parsed envelope (or parse/missing reason), heartbeat age
`now - observed_at`, cadence, terminal states.

- age <= 2 × cadence → `fresh`
- age > 2 × cadence, state `active` → `stale` (producer silent, child
  fate unknown — never infer the child from the parent's silence)
- file missing/unparseable → `unknown` with reason (not `failed`)
- state `stopped` → `terminal_expected`; state `failed` → `terminal_failed`
- A `stopped`/`failed` envelope is a terminal fact, not a missed beat:
  the consumer records it once (idempotent on heartbeat counter) and
  keeps polling (a new parent may republish).

Freshness threshold mirrors `is_role_parented` (2× the 5s default):
one threshold, two readers.

## Ledger entry

`role_id`, `decided_at_unix_ms`, `decision` (fresh/stale/unknown/
terminal_expected/terminal_failed), `heartbeat_age_ms`,
`heartbeat_counter` (dedupes re-reads), `observed_state`,
`restart_count`, `reason`, `residual_unknowns`. One line per decision
change plus one periodic `fresh` confirmation per hour (bounds volume:
a healthy month is ~720 lines, not 500k).

## Placement

- `cynic-protocol::supervision`: `WatchDecision`, `WatchLedgerEntry`,
  `decide_watch` — pure.
- `cynic-supervisor`: `watch.rs` poll loop (file read + append), same
  atomic-write discipline as the producer.
- `cynic-cli::supervise_surface`: `Watch` arm — thin (parse → loop).
- systemd units: `cynic-supervise-<role>.service` (producer, exists) +
  `cynic-supervise-watch-<role>.service` (consumer, new).

## Non-goals

No auto-restart in the consumer, no alerting transport in this slice
(Telegram/`notify-send` delivery belongs to the fleet-watchdog chantier),
no fleet aggregation (one file, one role, one ledger).
