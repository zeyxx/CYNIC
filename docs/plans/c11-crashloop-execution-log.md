# C0-11 execution log — 2026-10-22 (crash loop stopped)

Branch: `agent/hermes/c11-crashloop`. Approval: operator blanket go + audit prescription ("Stop the unit now").

## Stopped: DONE 2026-10-22 ~11:20 CEST (via pve root, `pct exec 103`)

- `systemctl stop cynic-node` -> `ActiveState=inactive`. Counter froze at NRestarts=262322 (was 250542 at audit, 261112 one hour ago — ~12k restarts/hour of log churn, now zero).
- `systemctl disable cynic-node` -> `disabled` (survives reboot; symlink removed).
- `systemctl mask` FAILED: "File /etc/systemd/system/cynic-node.service already exists". Stop+disable holds; mask deferred, not needed while disabled.
- Rollback: `systemctl enable --now cynic-node` (one command, recorded here).

## NOT done (separate work)

- C2-2 glibc target declaration (contract change, needs branch + review).
- Rebuild for glibc <= 2.36 + redeploy through release path (needs builder proof).
- Root cause of the 250k+ restarts beyond the glibc mismatch: unexamined; the journal on CT103 holds the evidence.

## Exit criteria status

- [x] `systemctl show cynic-node -p NRestarts` stable over 1h (frozen at 262322, unit inactive).
- [ ] Rebuilt + redeployed binary running (blocked on C2-2 + builder).
