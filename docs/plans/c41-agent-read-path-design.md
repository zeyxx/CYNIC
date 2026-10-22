# C4-1 Agent Read Path — Design

Status: design only. No host mutation, no privileged command.

## Problem

After the 2026-10-22 hardening, `user` is no longer in `sudo`, `docker`, or `cynic`.
Files under `/etc/cynic/` are mode `0640 root:cynic` (registry) and `0640` (secrets).
Agents that run as `user` (e.g. Hermes) can therefore no longer read the runtime
registry (`/etc/cynic/registry.json`) nor the secret store, even though they only
need read access. Re-adding `user` to `cynic` is rejected: it would also grant
write/execute surfaces that violate deny-by-default.

## Goal

Give `user`-running agents **read-only** access to:

1. the runtime registry (`/etc/cynic/registry.json`)
2. the secret store (`/etc/cynic/secrets/*`)

without rejoining the `cynic` group, without plaintext secrets in logs, with a
fail-closed posture, and with re-auditability.

## Readers to serve (exact)

Runtime readers of `/etc/cynic/registry.json` (via `CYNIC_REGISTRY` /
`CYNIC_REGISTRY_PATH`):

- `cynic-node` (systemd unit `cynic-node.service`) — fleet mesh bootstrap
- `cynic-kernel` (systemd unit `cynic-kernel.service`) — storage topology
- `cynic-forgejo-webhook` (systemd unit `cynic-forgejo-webhook.service`)
- `cynic-host` crate — `runtime_registry.rs` (bootstrap / framework scripts)
- `cynic-build` crate — release/bootstrap admission
- `cynic-cli` — discovery/network/node surfaces (`runtime_paths.rs`, `discovery_surface.rs`)
- Hermes agent running as `user` — **the unserved reader** (this design)

The first six run under their own service accounts / root and already have access.
Only Hermes-as-`user` is unserved.

## Option A — Unix socket `cynic-agent` with a small projection daemon

A privileged daemon (running as `cynic`, socket owned `root:cynic`, mode `0660`
with an ACL granting `user`) exposes a minimal read-only RPC:

- `ReadRegistry()` -> JSON bytes (full registry, read-only)
- `ReadSecret(name)` -> bytes for one named secret (allow-listed names only)

Properties:

- Fail-closed: daemon refuses if caller not in allow-list; default deny on any
  parse/IO error; no fallback to filesystem.
- No plaintext in logs: daemon logs only request id, caller uid, name, byte count,
  and a SHA-256 of returned bytes. Secret values never logged.
- Re-audit: every read is appended to an append-only audit log with
  `(timestamp, uid, op, name, sha256, byte_count)`; a re-audit recomputes the
  hash of the on-disk source and compares.
- Rotation: daemon re-reads source on each request (no cache), so rotated
  secrets are visible immediately.

Cost: new daemon binary, socket lifecycle, protocol, ACL management.

## Option B — Projected mirror file with hash

A privileged writer (cron or systemd timer, running as `cynic`) regenerates a
mirror at a `user`-readable path, e.g. `/var/lib/cynic/agent-read/registry.json`
and `/var/lib/cynic/agent-read/secrets/<name>`, mode `0640 root:user` (or `0644`
for registry only), plus a sidecar `*.sha256`. Agents read the mirror directly.

Properties:

- Fail-closed: writer is the only producer; if generation fails, stale mirror is
  removed (or marked invalid) rather than served; agent checks hash sidecar and
  refuses on mismatch.
- No plaintext in logs: writer logs only path, byte count, hash; secret values
  never logged.
- Re-audit: recompute SHA-256 of mirror and compare to sidecar and to source;
  any drift fails the audit.
- Latency: agent reads are local file reads (fast); but freshness lags the timer
  interval.

Cost: timer/cron unit, atomic write discipline (tmp + rename), hash sidecar
management, cleanup of stale mirrors.

## Comparison

| Criterion | A (socket daemon) | B (projected mirror) |
|---|---|---|
| Fail-closed | Strong (runtime gate) | Strong (hash gate) |
| Plaintext in logs | None (hash only) | None (hash only) |
| Re-audit | Live hash compare | Static hash compare |
| Freshness | Immediate | Timer-lagged |
| Complexity | Higher (daemon+protocol) | Lower (writer+timer) |
| Attack surface | Socket + protocol | Filesystem permissions |

## Recommendation (R)

**Option B — projected mirror with hash.**

Rationale: it reuses existing privileged-writer patterns already present in the
repo (systemd units, atomic file writes), keeps the agent read path a plain
filesystem read (no new protocol, no daemon to supervise), and satisfies
fail-closed + no-plaintext-logs + re-audit via the hash sidecar. The freshness
lag is acceptable for registry/secrets consumption by an agent. Option A remains
the upgrade path if sub-second freshness or per-request authorization becomes
required.

## Open questions

1. Timer cadence and staleness threshold acceptable to Hermes consumers.
2. Exact allow-list of secret names projected (must be enumerated, not globbed).
3. Whether registry mirror should be `0644` (non-secret) or `0640 root:user`.
4. Audit log location and retention for the hash trail.
5. Whether to also project into XDG user config (`~/.config/cynic`) as a
   fallback for dev builds.
