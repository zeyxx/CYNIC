# N-1 store migration plan — cortex SurrealKV to CT103 (doc only, no execution)

- Scope: findings N-1, N-2, N-3 (+ N-4 compat as precondition guard)
- Branch: `agent/hermes/n1-store-preservation` from `origin/main` @ `d88f1312`
- Status: PLAN ONLY. No host command in this step mutates anything. Every phase below needs explicit operator approval before execution.

## 1. Current state (observed 2026-10-22, read-only)

- Kernel store in use: SurrealDB 3.0.3 on cortex, `surrealkv:///home/user/.surrealdb/data` (213M), url `http://127.0.0.1:8000`, namespace `cynic` / database `main`.
- Corruption active: kernel 62 checksum failures last hour, 6646 `expire_stale failed` since 2026-10-17; surreal 2880 checksum/corrupt lines last hour, 294515 since 2026-10-17. Kernel introspection reports `0 alerts` (blind).
- Backups of the store in use: none. `~/.surrealdb/backups/` holds only 83-byte files from June. Auditor hot copy from 2026-10-22 09:28 not found on cortex.
- Unsafe runtime: process runs as `user`, data under operator home, full capabilities allowed, credentials passed in argv (exact value deliberately not recorded).
- Registered store host: CT103 (`storage_primary`, volume `storage_primary_artifacts` = `lxc/103:mp0`, per `infra/registry.json`). The kernel does NOT use it.
- CT103 live (read-only via pve, 2026-10-22): `surrealdb` active; `cynic-node` still crash-looping (`activating`, NRestarts=261112, up from 250542 at audit time).
- Repo already defines the target shape: `infra/os/systemd/surrealdb.service` (bind `0.0.0.0:8000`, data `/var/lib/cynic/.surrealdb/data`, `NoNewPrivileges`, `MemoryMax=4G`, no creds in argv) and `surrealdb-backup.{service,timer}` (daily, namespace `cynic` / database `v2`, size-verified, 7/14-day retention). Mismatch to resolve: live cortex uses database `main`, repo units use `v2`.

## 2. Target state

- SurrealDB runs on CT103 as a dedicated principal, data under `/var/lib/cynic`, no credentials in argv (env/file), least-privilege DB user for the kernel.
- Kernel points at CT103 over the internal resolver; cortex SurrealDB retired only after the new store proves readable and backed up.
- Daily logical backups of the store in use, each restore-tested; newest backup < 26h (exit criterion from the audit).
- `kernel.service.cynic` resolves to the real store host; no consumer depends on the phantom 10.10.10.100.

## 3. Phases (each = separate approval, separate proof)

- Phase 0 — Preserve (operator-run, nothing stops the kernel):
  1. Operator runs logical `surreal export` of namespace `cynic` / database `main` from cortex (needs DB credentials; agent was declined).
  2. Operator copies the export plus the hot on-disk copy off-host to `pve:/var/lib/vz/dump/cynic_archives/cortex-surreal/`.
  3. Agent verifies read-only: file sizes > 100 bytes, gzip integrity, sha256 recorded. No stop of `surreal.service` (cold copy deliberately deferred: corrupt store may not restart).
- Phase 1 — Stand up CT103 store:
  1. Deploy repo `surrealdb.service` shape on CT103 through the release path (decide database name `main` vs `v2` first; kernel uses `main`).
  2. Create least-privilege DB user for the kernel; rotate the weak root password; credentials via file/env, never argv.
  3. Prove: health endpoint, kernel can authenticate with the new user, write/read round-trip of one test record then delete it.
- Phase 2 — Migrate readable data:
  1. Export what is still readable from cortex (tolerant export; record skipped ranges).
  2. Import into CT103; row-count comparison per table; diff report committed as evidence.
  3. Explicit unknown: rows under the corrupt SSTable block may be unrecoverable; that set must be enumerated, not hand-waved.
- Phase 3 — Cut over and retire:
  1. Restore `~synapse.cynic` compat zone on cortex first (N-4 guard) so stale binaries keep resolving during the move.
  2. Repoint kernel to CT103 in a planned window; monitor `checksum verification failed` = 0 for 1h and introspection non-zero coverage.
  3. Stop cortex `surreal.service` only after CT103 backup + restore proof exists. Keep the preserved cortex copy until the operator releases it.

## 4. Backup fix (N-2, part of this chantier)

- The repo backup unit targets CT103 layout and database `v2`; it never covered cortex `main`. After Phase 1, install and enable `surrealdb-backup.timer` on CT103, run once, prove with a restore into a scratch DB. Until cutover, the cortex store needs its own daily export (operator-run, credentials-gated).

## 5. Exit criteria (must all hold before closing N-1/N-2/N-3)

- `journalctl -u cynic-kernel --since -1h | grep -c 'checksum verification failed'` = 0.
- Kernel store runs on CT103 as a dedicated principal, no credentials in argv.
- Restore-tested backup of the store in use is < 26h old.
- `cynic-node` on CT103 is out of crash loop (separate chantier C0-11/C2-2, tracked dependency, not silently absorbed here).

## 6. Explicit unknowns

- Which of the three pre-swap SSTables (2026-09-11, 2026-09-23, 2026-10-08) holds the failing block at offset 45189021.
- Whether the 2026-06-06 `data.corrupt.*` event and the current corruption share a cause.
- SMART state of cortex `sda` (not read in this step).
- Namespace/database naming decision (`main` live vs `v2` in repo units).
