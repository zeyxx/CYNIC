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

## One path, one authority

These units are versioned HERE and nowhere else. An earlier state of the
repository also carried copies at `infra/os/systemd/`, and the copy at the
repository root was the one installed on builder-119 — so the corrected
units looked right in the repository while the service kept failing at
runtime. The installed file was byte-identical to the stale copy
(sha256 `cf8bdb0e…`), which is the only thing that settles which one the
host used. See killchain 13 in
`docs/design/OPERATIONAL_DRIFT_KILLCHAINS.md`.

A file versioned twice is a service with two definitions, and which one
wins is decided by whoever copied a path, not by which one is correct.

## Install

    install -m 0644 infra/os/systemd/soak/cynic-supervise-soak-node.service /etc/systemd/system/
    install -m 0644 infra/os/systemd/soak/cynic-supervise-watch-soak-node.service /etc/systemd/system/
    systemctl daemon-reload
    systemctl enable --now cynic-supervise-soak-node.service cynic-supervise-watch-soak-node.service

## Installation scope, and the two lines that must be absent

These units are installed in the **systemd user scope** — this account's own
manager, kept alive by `Linger`. The agent's service account cannot write
`/etc/systemd/system` (root-owned) and polkit refuses a system-scope
restart, so the user manager is the scope that actually runs here.

Two directives follow from that scope, and both were measured rather than
assumed:

- **no `User=`/`Group=` line.** The user manager already runs as this
  account. With the line present the service exits `216/GROUP` in about
  3 ms, before the binary is ever exec'd. Without it: `0/active`.
- **`WantedBy=default.target`, not `multi-user.target`.** The user manager
  has no `multi-user.target`, so the activation symlink would point at
  nothing.

The scope also decides which binary runs. The installed unit
`/etc/systemd/system/cynic-supervise-soak-node.service` predates the
convergence and still points at `/var/lib/cynic/shared-target/debug/cynic` —
a build that predates the `supervise` verb and fails with
`error: unrecognized subcommand 'supervise'`. That is what the four
restarts in the old `NRestarts=4` were. The user-scope units point at
`/var/lib/cynic/bin/cynic`, and the running process resolves to
`/var/lib/cynic/releases/v0.9.0/cynic` — outside the Cargo tree, so a build
can no longer replace the file a live unit executes.

### Read-back of the running pair

| Observation | Value |
|---|---|
| parent `ActiveState` | `active`, `NRestarts=0` |
| parent `/proc/<pid>/exe` | `/var/lib/cynic/releases/v0.9.0/cynic` |
| `node.status.json` | `cynic.supervisor_parent_status.v1`, `state: active`, `restart_count: 0`, `residual_unknowns: []` |
| `watch.jsonl` | `cynic.supervisor_watch_ledger.v1`, `fresh` / `heartbeat_within_bound` |
| `supervise-node.lock` | held by the live parent pid |

One parent may hold the lock for a given `--state-dir`: a second parent on
the same state directory exits `1` rather than evicting the first. That is
the intended single-writer contract, and it is why a failed start looks
like a plain exit `1`.

## The retention timer — the missing caller

The retention policy exists in code (`cynic_protocol::retention`, the
`RetentionObserver` → `RetentionPlanner` → `RetentionExecutor` trio) and
`cynic runtime build artefact gc` is its native surface. It was never called
by anything: the checkout clones were only ever removed by hand.

Measured on builder-119, read-only (`--json`, no `--apply`):

| Observation | Value |
|---|---|
| schema | `cynic.retention_observation.v1` |
| items | 16 |
| reclaimable | 7.27 GiB |
| orphans | 12 |
| live-service paths | 0 |

`cynic-build-artefact-gc.timer` is the caller. It runs
`cynic runtime build artefact gc --apply` daily, with a randomized delay so
several builders do not reclaim in lockstep, and `Persistent=true` so a
missed window is caught up once after a reboot.

Two properties come from the command rather than from the unit: the GC
observes and plans before it applies, and it refuses any path a live service
executes (the live-service fence). The unit therefore needs no path list of
its own — inventing one there would be a second, weaker authority for the
same decision.

## Retiring the stale system-scope pair

The two units at `/etc/systemd/system/` were installed from the duplicate
repository path (`infra/os/systemd/`, byte-identical to the file on the
host, sha256 `cf8bdb0e…`). They execute a Cargo-cache binary that does
not know the `supervise` verb, so the parent fails at every boot, and
they are `enabled` — they come back on every reboot.

Retiring them is a **native, bounded** operation, not a hand-run sudo:

```
cynic host service reconcile --apply
```

The envelope is exactly `--apply`; the target and the policy are
registry-derived. The units to retire are declared in the registry, per
node:

```json
"retired_units": [
  "cynic-supervise-soak-node.service",
  "cynic-supervise-watch-soak-node.service"
]
```

so the registry stays the single authority for what a specific node
runs. The operation then:

1. `disable` — drops the boot activation symlink. A unit that is stopped
   but still enabled comes back at the next reboot, which is exactly the
   failure this ends.
2. `stop` — ends the running instance.
3. removes the unit file and any `<target>.wants/` link.

The proof is the **absence** of the unit: the run reports success only
when the unit is neither active nor enabled. A unit that is still running
leaves the proof gate unsatisfied and the run is reported as a failure,
not a success. A unit that was never installed is already retired —
retiring is idempotent.

The unit identifier is guarded: plain `*.service` only, no path, no
leading `-` (which the service manager would read as an option), no
shell metacharacter.

The host registry (`/etc/cynic/registry.json`) is root-owned and is the
operator's to install; the declaration above is the repository SSOT.
