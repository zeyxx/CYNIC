# C4-1 / C4-3 / C1-1 access remediation plan — cortex (doc only, no execution)

- Scope: C4-1 (desktop user is root), C4-3 (secrets readable/owned by user), C1-1 (registry writable by user)
- Branch: `agent/hermes/c41-access` from `origin/main` @ `d88f1312`
- Status: PLAN ONLY. Every step below is a privileged mutation and needs explicit operator approval with a root console open before execution.

## 1. Current state (observed 2026-10-22, read-only)

- `sudo -n true` as `user` -> EXIT 0.
- `/etc/sudoers.d/temp-claude` present (29 bytes, 2026-04-23). Content unreadable without privilege escalation; size/date match the audit's NOPASSWD-ALL grant.
- `/etc/sudoers.d/cynic-tools` grants NOPASSWD `/usr/bin/find`, `/usr/bin/cat` (plus rustdesk, apt) — each a root escalation alone.
- `id user` groups: `sudo`, `docker`, `cynic`, `cynic-agent` (plus adm, kvm, ollama, libvirt).
- `/etc/cynic/registry.json`: `0664 user:user` (C1-1 holds).
- `/etc/cynic/secrets/` (`2770 root:cynic`): fleet SSH **private** keys `agent-host-ssh`, `compute-01-ssh`, `headscale-ssh`, `laptop-ssh-authority` are owned `user:cynic 600` — the desktop user owns the fleet keys (C4-3 holds, worst case confirmed). Token files (`forgejo-token`, `domain.env`, `headscale-api-key`, `kernel-runtime.env`) are `root:cynic 640`, readable via the `cynic` group.
- `claude-code` sudoers (129 bytes, 2026-03-13) not yet read; `cynic-firewall`, `cynic-host-exec`, `cynic-network-actuator` scope unconfirmed — read them during execution before narrowing.

## 2. Target state

- `sudo -n true` as `user` fails.
- `id user` shows neither `cynic`, `docker`, nor `sudo`.
- `/etc/cynic/registry.json` is `root:cynic 0640`; only the release/projection path writes it.
- Fleet private keys are `cynic:cynic 600` (or a dedicated actuator identity), never owned by `user`.
- Every secret `user` could read is rotated after the ownership fix.
- `cynic` sudoers entries are subcommand-scoped with exact argv (C4-8 pinned lists included where touched).

## 3. Steps (strict order, one approval per step, root console open throughout)

- Step 1 — Prepare fallback: operator opens a root console (pve or local) that does NOT depend on `user` sudo. Prove: `whoami` = root on that console. Rollback: nothing changed yet; abort freely.
- Step 2 — Read the grants: dump `temp-claude`, `claude-code`, `cynic-tools`, `cynic-firewall`, `cynic-host-exec`, `cynic-network-actuator` from the root console and record exact argv. No change. Rollback: n/a (reads).
- Step 3 — Remove escalation path: delete `temp-claude` and `cynic-tools`; narrow `claude-code` to named units; remove `user` from `docker` and `sudo`. Prove: `sudo -n true` fails, `id user` clean of those groups. Rollback: recreate the removed files from the Step-2 record (kept in the execution log, not in git).
- Step 4 — Fix ownership: `chown root:cynic 0640 /etc/cynic/registry.json`; `chown cynic:cynic` the fleet private keys; remove `user` from `cynic`. Prove: `stat` outputs, `sudo -u user cat /etc/cynic/secrets/forgejo-token` fails. Rollback: reverse chown/chgrp from the recorded before-state.
- Step 5 — Rotate: every secret `user` could read (forgejo-token, headscale-api-key, domain.env, kernel-runtime.env, storage password, fleet keys or their authorized_keys counterparts). Prove: old values fail, new values work from the legitimate principal only. Rollback: n/a — rotation is forward-only; keep new values in the vault, never in git.
- Step 6 — Agent path: give agents a `cynic-agent`-socket-only path for the operations they legitimately need (registry projection reads, store access), so removing `user` from `cynic` breaks nothing. Prove: one dry-run of each agent read path as the agent identity.

## 4. Exit criteria

- `sudo -n true` as `user` fails.
- `id user` contains neither `cynic` nor `docker` (nor `sudo`).
- `stat /etc/cynic/registry.json` = `0640 root:cynic`.
- No file under `/etc/cynic/secrets/` is owned by `user`.
- Rotation receipts recorded (which secret, when, verified by whom — values stay in the vault).

## 5. Explicit unknowns

- Content of `temp-claude`, `claude-code`, `cynic-firewall`, `cynic-network-actuator` (read in Step 2).
- Which agent flows break when `user` leaves group `cynic` (covered by Step 6 dry-runs).
- Whether `hermes :8644` or `llama-server :8081` bind paths depend on the current groups (C0-9 neighbours, not silently absorbed here).
