# Brick 2 (receiver) / brick 3 (enrollment) — 2026-10-22: STAGED, BLOCKED

Branch: `agent/hermes/brick2-receiver`. Approval: operator R + "tout aujourd'hui".

## Brick 2 done (binary + config staged, no unit yet)

- Live invocation observed on builder (read-only `/proc`): `/var/lib/cynic/bin/cynic-node control --config /etc/cynic/node-control.toml`, cwd `/var/lib/cynic`, exe = release `releases/v0.8.0/cynic-node`. :9090 listener confirmed via `ss`.
- Binary copied builder -> compute-01 `/var/lib/cynic/bin/cynic-node` (24 MB, direct builder ssh, host key accepted). Smoke test: binary RUNS on Debian 13 (glibc 2.41 >= 2.39 — forward compatible). It fails only on missing default config, as expected.
- Adapted config written to compute-01 `/var/lib/cynic/node-control.toml` (node_id, LAN listen `192.168.0.25:9090`, local state roots, `build_jobs=4`, `max_rss_cap_mb=8192`, NO builder identity copied). Dropped builder-only fields (capability lists, registry digest) — binary reported the first missing field honestly (`signing_key_path`), fixed by reading the full builder config.
- State dirs created on compute-01. Full builder config read (secret values excluded).
- Unit file NOT installed: compute-01 `cynic` has no sudo; install needs titouan/operator. No unit invented blindly — file staged in next step once enrollment unblocks (ExecStart known).

## Brick 3 BLOCKED — three emergences (the holes the operator asked to see)

E1. **No route compute-01 -> kernel.** TCP probes from compute-01: cortex `192.168.100.2:3030` CLOSED, `:22` CLOSED; builder `:9090` OPEN. The doc's brick-3 handshake (PreAuthKey over kernel WS) cannot run from compute-01. Direction cortex -> compute-01 works; the reverse does not.
E2. **Stale CSR, wrong role.** compute-01 already holds `/var/lib/cynic/identity/node-signing.key` (Oct 16, 600) and `csr-cynic-compute-01.json` (Oct 17, `requested_role: agent_host`, self-signed, valid structure). It was never submitted. Pool filter needs `build_runner`, not `agent_host` — even a signed grant on this CSR would not route builds.
E3. **Grants appear machine-bound.** Builder-issued test grant (`cynic-node-enroll` into a throwaway dir) carries `machine_identity_digest` + a FRESH authority key (mismatch vs builder `enrollment-authority.key` proven by sha256). It is untrusted by everyone and was DELETED (`/tmp/enroll-compute01` removed to avoid confusion with real identity). Consequence: enrollment must happen where the authority lives (builder, kernel-mediated) or via a CSR-signing surface that does not exist in the tree (no `--csr` flag, no `cynic enroll status` verb — both already tracked as known gaps).

## Cleanups done

- Throwaway enrollment dir on builder deleted (fresh authority key destroyed before it could confuse anyone).
- compute-01 test receiver exited on its own (config parse error, expected); nothing running there.

## What unblocks brick 3 (operator decision)

- (a) Open compute-01 -> cortex:3030 route (or a builder-side socat mirror of the kernel-proxy pattern), then run the kernel handshake from compute-01; or
- (b) Operator-mediated join: sign compute-01's CSR with the real authority on the builder (needs the CSR-signing flow, which is code that does not exist yet); or
- (c) Accept agent_host role and use compute-01 for embedding only (abandon builds there).

Unit install + :9090 probe + registry deposit + pool confirmation all wait on this decision.
