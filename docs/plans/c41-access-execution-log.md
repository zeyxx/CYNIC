# C4-1 / C4-3 / C1-1 execution log — 2026-10-22 (steps 1-4 DONE, step 5 NOT done)

Branch: `agent/hermes/c41-access`. Approvals: operator `R + 2` then `fais le`.
Operator console: agent-held root shell (`sudo -n -s`, session kept open as fallback, disclosed to operator).

## Step 1 — fallback console: DONE (agent-held, see note)

## Step 2 — reads: DONE (recorded in plan §1, commit 2bea598a)

## Step 3 — escalation removal: DONE 2026-10-22 ~10:50 CEST

- Rollback copies: `/root/c41-rollback-20261022/` (6 sudoers files + `id.before` + `stat.before` + `claude-code.new`). Root-only, off-git by design.
- Deleted: `/etc/sudoers.d/temp-claude`, `/etc/sudoers.d/cynic-tools`.
- Rewrote `/etc/sudoers.d/claude-code` (staged to `/tmp`, `visudo -c -f` parsed OK before install, `install -m 0440`): 6 read-only systemctl verbs (`status`, `is-active`, `is-failed`, `show`, `list-units`, `list-timers` with `*` unit). Dropped: `apt-get`, `loginctl`, all restarting verbs.
- Fixed modes to 0440: `claude-code`, `cynic-firewall`.
- `visudo -c` after: all files parsed OK.
- Removed groups (last, via `gpasswd -d`): `user` from `sudo`, `docker`, `cynic`. Kept: `cynic-agent` and the rest.

## Step 4 — ownership: DONE

- `/etc/cynic/registry.json` -> `root:cynic 0640`.
- Fleet private keys (`agent-host-ssh`, `compute-01-ssh`, `headscale-ssh`, `laptop-ssh-authority`) -> `cynic:cynic 600`.

## Verification (fresh `su -s /bin/sh user` sessions, post-change)

- `sudo -n true` as user: FAILS (blocked OK).
- `cat /etc/cynic/secrets/forgejo-token` as user: denied (blocked OK).
- `cat /etc/cynic/registry.json` as user: denied (blocked OK).
- `id user`: no `sudo`, `docker`, `cynic`. Exit criteria 1-4 of the plan HOLD.

## Step 5 — rotation: NOT DONE (explicit handoff)

Old secret values were readable by `user` until ~10:50. Assume exposure, rotate forward (no rollback possible by design):

1. Forgejo token (`forgejo-token`, also at `/etc/cynic/secrets/`): regenerate in Forgejo admin, replace file `root:cynic 640`, restart dependent readers.
2. `headscale-api-key`, `domain.env`, `kernel-runtime.env`: same pattern.
3. Storage password: rotate in SurrealDB, update the runtime secret file, restart kernel; never pass via argv.
4. Fleet SSH keys now `cynic:cynic`: generate fresh pairs, update `authorized_keys` counterparts (pve, CT105, compute-01, laptop), remove old public keys, then destroy old private material.
5. Prove each: old value fails, new value works from the legitimate principal only; receipts (which, when, by whom) stay in the vault, never in git.

## Step 6 — agent path: NOT DONE (breakage disclosed)

- Agents running as `user` (hermes included) can no longer read `/etc/cynic/registry.json` or secrets. The `cynic-agent`-socket-only path does not exist yet.
- Live processes keep their pre-change groups until re-login; all NEW agent sessions run unprivileged. Expect `cynic` CLI reads of the registry/secrets to fail for agents until step 6 lands.
- Next: dedicated chantier for the agent read path (socket/projection), then re-run the dry-runs.

## Known behavior changes for the operator

- `systemctl restart/start/stop`, `apt-get`, `loginctl` via `claude-code` no longer work: use the root console.
- `nft` via `cynic-firewall` unchanged (still `flush ruleset` capable — tracked, not narrowed in this chantier).
