# Deploy surface — measured reality (2026-10-19)

Every number below was OBSERVED on the live machines, not read from a
registry. The registry was wrong; this file is the correction.

## The registry lied

`infra/binary-locations.json` claimed one path, one sha, one status for
all three machines. Measured:

| machine | path | sha256 (16) | baked sha | profile | owner |
|---|---|---|---|---|---|
| builder | `/usr/local/bin/cynic` | `cd62eaed44641343` | `9d2f3cf0` | debug | root |
| builder | `/var/lib/cynic/bin/cynic` | `998ddf12236bb715` | `42e56f5f` | release | cynic |
| cortex | `/usr/local/bin/cynic` | `6da2745fee7f4c27` | `0bbbccb7` | debug | root |
| cortex | `/home/user/.local/bin/cynic` | `998ddf12236bb715` | `42e56f5f` | release | user |
| agent-host | `/usr/local/bin/cynic` | `b8e3e6b6124659f3` | `ce0d6a2a` | release | root |
| agent-host | `/var/lib/cynic/bin/cynic` | `15987545151d9550` | `56317c91` | debug | cynic |

The declared `sha256` (`944753f2…`) and `sha_baked` (`8602936d`) match
NONE of the six live binaries. The declared `status: current` was true for
none of them in the sense that matters (the registry's "current" sha is
nowhere on disk).

TWO binaries per machine, different profiles, different SHAs. The registry
modeled one binary per machine. That is the whole defect.

## The declared canonical path is not deployable by the principal

`binary-locations.json` says `canonical_path: /usr/local/bin/cynic`.
Measured write access as the `cynic` principal:

- builder: `/usr/local/bin` → **Permission denied**
- agent-host: `/usr/local/bin` → **Permission denied**
- cortex: `/usr/local/bin` → root-owned; the operator has `sudo NOPASSWD:
  ALL`, so the operator can, but no machine principal can.

The path that IS deployable by the machine principal on every machine:

- builder: `/var/lib/cynic/bin/` (cynic-owned, symlink `cynic ->
  /var/lib/cynic/current/cynic`, `current -> /var/lib/cynic/releases/v0.8.0`)
- agent-host: `/var/lib/cynic/bin/` (cynic-owned, plain file, NO releases/
  current/ symlink structure)
- cortex: `/home/user/.local/bin/` (user-owned)

## Live transport reality (registry vs reality)

| machine | registry `transports.ssh.target` | reality |
|---|---|---|
| builder | `cynic-builder-ssh` | works (ssh config alias) |
| agent-host | `cynic-agent-host.synapse.cynic` | DNS fails from cortex AND from builder; the IP `10.10.10.102` works from cortex with the build authority key |
| cortex | `cynic-host-cortex` (no transports) | local machine (`ubuntu`, 192.168.100.2) |

The agent-host alias resolves only inside the mesh DNS. From the operator
host the reachable form is `cynic@10.10.10.102` with
`~/.ssh/cynic-build-authority`. Proof: `ssh cynic@10.10.10.102` returned
`uid=1000(cynic)` and hashed the binary; `ssh cynic-agent-host.synapse.cynic`
returned `Could not resolve hostname`.

## So the deploy target is NOT `/usr/local/bin/cynic`

Three facts force a different shape:

1. `/usr/local/bin/cynic` is root-owned on all three machines and is NOT
   the binary the system actually uses. The builder's own `cynic-node`
   service runs `/var/lib/cynic/bin/cynic-node`; the `cynic` CLI on the
   builder is a stale root-owned copy (`9d2f3cf0`, debug) that predates
   the release at `/var/lib/cynic/bin/cynic` (`42e56f5f`, release).
2. The deployable, principal-owned path is `<state-root>/bin/cynic` on
   builder and agent-host, `~/.local/bin/cynic` on cortex.
3. `cynic-pkg`'s blue-green release layout (`releases/<tag>/` + `current`
   symlink) EXISTS on the builder only. agent-host has a flat `bin/`.

Therefore the deploy policy must declare, per machine: the deployable
path, the release layout (symlink vs flat), and the read-back command —
not a single global `canonical_path`.

## Layer B design consequence

The smoke gate must probe the DEPLOYED binary at its deployed path and
assert the baked SHA equals the deployed commit. Measured smoke command:
`cynic source status --json` exits 0 on all three machines.

The executor must:
- stage the artifact to a temp path on the target (scp, principal-owned);
- back up the live binary as `binary.bak-<sha>`;
- swap atomically (symlink swap where the layout has one, plain rename
  where it does not);
- read back with `sha256sum` AND `cynic version --json` (both, because
  sha256 alone does not prove which commit was baked);
- run the smoke command and assert exit 0.

Both are required: a binary with the right bytes but the wrong baked SHA
is a different proof, and `cynic version --json` is the surface that
carries the commit.

## Authority constraint (measured)

`sudo -n -l` as `cynic` on the builder grants exactly: restart +
is-active of `cynic-node.service`, and a set of `cynic-host-exec` verbs.
The deploy of the `cynic` CLI itself needs NO privilege on builder /
agent-host / cortex — the principal owns the target path. `/usr/local/bin`
would need a new HumanAdmin grant; the deploy does not need it because the
principal-owned path is the one in use.

agent-host has NO sudo grant at all (password required) — so any layer
that needs privilege there is blocked. The principal-owned path is also
the only path available there.

## Reversal of the earlier claim

The deploy-policy.json written for layer A declares
`canary_order: [builder, cortex, cynic-agent-host]` and a single
`canonical_path`. The order stands. The path model does not: it must
become per-machine declared data, and `binary-locations.json` must be
corrected to the measured truth (two paths per machine, no false sha).
