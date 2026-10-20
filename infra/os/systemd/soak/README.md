# Soak units — durability bench, not production services

These two units were running on builder-119 with no source in the repository.
They were installed by hand on 2026-10-16 and their unit files were the only
record of what they do. That is a drift condition, not a convention: a service
whose definition exists only on one host cannot be audited, cannot be
reinstalled, and cannot be compared to what the code actually does.

## What they are

- `cynic-supervise-soak-node.service` — the soak parent. It runs the
  `supervise run` verb with a placeholder `node` role and a sacrificial
  `sleep infinity` child. The metric under observation is the parent crash
  rate against the child crash rate over a 30-day durability run.
- `cynic-supervise-watch-soak-node.service` — the consumer. It polls the
  status file every 5 s, appends one JSON line per verdict change plus an
  hourly fresh confirmation, and never restarts the parent. It is a separate
  unit so that a dead parent is still recorded rather than silently absent.

Neither unit owns a production role. The soak has its own state directory
(`/var/lib/cynic/supervise-soak`) and touches no production state.

## Why they are here and not in `installed/`

`infra/os/systemd/installed/` holds the units the bootstrap bundle deploys on
a production node — they are embedded with `include_str!` and installed by
`install.sh`. The soak is not deployed by the bootstrap and must not be
installed on a production node: a 30-day crash-rate bench has no business
running there. Keeping it in a separate directory makes that boundary
explicit instead of implied.

## The ExecStart points into a Cargo cache

Both units execute `/var/lib/cynic/shared-target/debug/cynic`. That path is
inside a Cargo target directory, which is exactly the kind of path the build
cache reclaim policy is allowed to remove.

Two consequences, both now handled in code rather than by convention:

1. The reclaim refuses to remove a path a live unit executes from. The
   decision reads the unit files themselves, so it cannot drift from what the
   units actually run.
2. The evidence distinguishes the two reasons a path is skipped —
   `skipped_absent` (already gone) and `skipped_live_service` (in use by a
   running unit) — so an operator reading a reclaim result knows which
   happened.

The durable fix is to install the soak binary outside the Cargo tree
(`/usr/local/bin/cynic` on builder-119 is 0.8.0 and does not yet know the
`supervise` verb, so it cannot be switched over yet). Until the binary is
rebuilt and reinstalled, the units keep pointing at the cache and the fence
keeps the cache alive.

## The verbs, proven

The `supervise` verb and the three arms the units use were exercised against
the installed release binary, with an isolated state directory, before the
units were pointed at it:

| Command | Result |
|---|---|
| `supervise run --role node --state-dir <d> --status-file <d>/node.status.json -- sleep infinity` | parent published `cynic.supervisor_parent_status.v1`, `state: active`, `restart_count: 0`, `residual_unknowns: []` |
| `supervise observe --role node --status-file <d>/node.status.json` | `cynic.supervisor_observation.v1` with `state: active` |
| `supervise watch --role node --status-file <d>/node.status.json --ledger-file <d>/watch.jsonl` | `cynic.supervisor_watch_ledger.v1` lines: `fresh` / `stale` / `fresh` with `heartbeat_age_ms` and `heartbeat_counter` |

`observe` takes `--status-file` only; `--state-dir` belongs to `run`. The
watcher is a long-running consumer by design, so a timeout on it is the
expected exit, not a failure.

## Install

    install -m 0644 infra/os/systemd/soak/cynic-supervise-soak-node.service /etc/systemd/system/
    install -m 0644 infra/os/systemd/soak/cynic-supervise-watch-soak-node.service /etc/systemd/system/
    systemctl daemon-reload
    systemctl enable --now cynic-supervise-soak-node.service cynic-supervise-watch-soak-node.service
