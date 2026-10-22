# C0-1 / C5-2 execution log — 2026-10-22 (data safety)

Branch: `agent/hermes/c01-data-safety`. Approval: operator blanket go for the day.

## C0-1 vzdump jobs: DONE (partial, recorded limits)

- Created job `cynic-nightly` via `pvesh create /cluster/backup`: schedule `02:00`, storage `local`, mode `snapshot`, compress `zstd`, vmid `100,101,102,103,104,106,107`, enabled, notify on failure.
- Verified in `/etc/pve/jobs.cfg` (read-back, content matches).
- Excluded: CT105 (126G used, pve `local` has 72G free — dump would fail; needs off-sda target, still missing) and CT113 (stopped, pending register-or-destroy decision C0-5).
- Proof run: `vzdump 107` snapshot now, 1.4GiB -> 364MB archive in 2m48s, snapshot cleaned, `vzdump-lxc-107-2026_10_22-11_05_35.tar.zst` on disk.
- Remaining for full closure: off-sda storage for CT105; scratch-CT restore test.

## C5-2 SurrealDB backup timer: BLOCKED (repo unit does not fit CT103)

Measured on CT103 (read-only):

- Live surreal runs as `--user root --pass cynic --bind 0.0.0.0:8000 rocksdb://var/lib/surrealdb/cynic.db` — rocksdb engine, custom data path, weak password in argv (N-3 class, second host).
- Repo `surrealdb-backup.service` assumes `surrealkv:///var/lib/cynic/.surrealdb/data`, database `v2`, password from env. On CT103: wrong engine path, unknown namespace/database names, `/etc/cynic/env` holds no `SURREALDB_*` vars — the unit would fail as-is.
- Do NOT install the repo unit verbatim on CT103. Next: probe actual namespace/database names on CT103 with a read-only export attempt, then adapt the unit (or fix CT103 to the repo layout) in a reviewed follow-up.

## Exit criteria status

- [x] `/etc/pve/jobs.cfg` contains a vzdump job (7 of 9 CTs; 105/113 excluded with reason).
- [ ] Newest dump < 26h for every covered CT (first scheduled run tonight 02:00).
- [ ] CT103 `surrealdb-backup.timer` active (blocked above).
