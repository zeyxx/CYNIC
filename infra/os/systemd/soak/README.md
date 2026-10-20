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

## The ExecStart points at the installed binary

Both units execute `/usr/local/bin/cynic`. That path is outside the Cargo
tree, and that is deliberate: a build in the shared target directory replaces
the file a running unit executes, which is a service-under-a-reclaimable-path
failure. The installed binary is versioned; the cache is not.

The intermediate state was recorded here and is kept for the audit trail: the
units used to execute `/var/lib/cynic/shared-target/debug/cynic`, and the
reclaim then had to be fenced against live service paths to keep that cache
alive. The fence is still in place and still correct — a path a running unit
executes from is never reclaimed — but the soak no longer depends on it.
The evidence distinguishes the two reasons a path is skipped
(`skipped_absent` and `skipped_live_service`), so an operator reading a
reclaim result knows which happened.

## Install

    install -m 0644 infra/os/systemd/soak/cynic-supervise-soak-node.service /etc/systemd/system/
    install -m 0644 infra/os/systemd/soak/cynic-supervise-watch-soak-node.service /etc/systemd/system/
    systemctl daemon-reload
    systemctl enable --now cynic-supervise-soak-node.service cynic-supervise-watch-soak-node.service
