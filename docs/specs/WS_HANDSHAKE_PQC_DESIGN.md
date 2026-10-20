# WS Handshake PQC — Frozen Scope Design

Status: design frozen; the CLIENT half is implemented and proven on the
builder. The SERVER half is not: the current tree has no WebSocket
endpoint, so the interop matrix below remains the acceptance gate for
the whole feature.

## 0. Implementation state (2026-10-19)

Client side, proven on the builder at `e265e751`:

- `cynic-protocol/src/workload_signature.rs` — the pure wire contract:
  algorithm enum, header names, payload bytes, replay-window predicate,
  verification decision.
- `cynic-node/src/adapters/pqc_workload_signer.rs` — ML-DSA-44 signer,
  detached verifier, keypair generation. Key file holds the raw
  serialized key (2560 bytes); the public key is hex on the wire.
- `cynic-node/src/adapters/workload_request_signer.rs` — the signer
  carries an optional post-quantum half and signs the SAME payload bytes
  with both algorithms.
- `apply_signature_headers()` — one application point for the classical
  and post-quantum headers, shared by the HTTP node-control adapter and
  the WebSocket handshake.
- Config: `federation.pqc_signing_key_path` or
  `CYNIC_NODE_PQC_SIGNING_KEY_FILE`. The post-quantum half is opt-in:
  without a key the handshake stays classical and a non-migrated kernel
  keeps working.
- Library: `fips204` 0.4.6, `ml-dsa-44` parameter set only.

Not yet done: the kernel-side verifier and the live interop matrix.

## 1. Observed state (facts)

- The deployed kernel (build 26.08.11, sha c26e4d73) serves `GET
  /events/ws` and requires workload-signature headers. Unsigned
  handshakes get 401 (`Missing workload signature timestamp`).
- Server contract (source at c26e4d73,
  `crates/cynic-kernel/src/api/websocket/mod.rs:270-340`):
  headers `X-Node-Public-Key`, `X-Node-Identity`, `X-Cynic-Timestamp`,
  `X-Cynic-Signature`; payload `timestamp + "GET" + "|" + "/events/ws"
  + "|" + body(empty)`; `infra::crypto::verify_signature` (Ed25519,
  hex, 300 s replay window, 30 s future drift); then
  `role_keys.resolve_signer` binds the key to a workload principal and
  requires organ or internal role.
- The current tree has NO server: `cynic-kernel` is a minimal
  orchestrator stub. The node client now signs Ed25519 (shared
  `workload_request_signer`, algorithm field present, Ed25519-only).
- Scheme precedent: `HttpNodeControlAdapter` uses the identical wire
  format for REST. One scheme, two adapters.

## 2. Target

Hybrid dual-sign handshake. The client sends BOTH signatures over the
SAME payload bytes:

- `X-Cynic-Public-Key` / `X-Cynic-Signature` / `X-Cynic-Timestamp` —
  Ed25519, byte-identical to today. Old servers keep working.
- `X-Cynic-Public-Key-Pqc` / `X-Cynic-Signature-Pqc` /
  `X-Cynic-Algorithm-Pqc: ml-dsa-44` — ML-DSA-44 over the same payload.

No negotiation round trip. No pure-PQC cutover (would brick old
servers). No stateful schemes (WOTS+/XMSS/LMS: a reconnecting node must
never reuse one-time state). No Falcon-512 first (float FFT signing is
fragile to implement in constant time; revisit after FN-DSA
implementations mature).

## 3. Verify policy (server)

| Client sends | Old server | New server |
|---|---|---|
| unsigned | 401 (today) | 401 |
| Ed25519 only | register | register |
| hybrid | register (ignores PQC headers) | register iff BOTH verify |

Policy flag for strict mode (require PQC once the fleet is enrolled):
explicit config, never silent default.

## 4. Canonical homes

- `cynic-protocol` (pure): algorithm enum, header names, payload
  constructor, negotiation/verify predicates. Converges the current
  duplication (kernel `infra::crypto` at c26e4d73 vs node
  `workload_request_signer`).
- `cynic-kernel`: reimplement `api/websocket` verification from
  c26e4d73, extended with PQC verify + policy flag.
- `cynic-node`: dual-sign behind the existing algorithm field.
- Keys: PQC public keys enroll through the same principal binding
  (`resolve_signer`); rotation reuses enrollment renewal.

## 5. Constraints

- ML-DSA-44 signature is ~2420 bytes in a header: verify axum header
  limits at implementation time.
- Replay window (300 s) and drift (30 s) unchanged.
- Library choice (libcrux vs pqcrypto) decided at implementation with
  audit evidence, not here.

## 6. Acceptance: interop matrix

Live kernel + node, all cells exercised: old/new server x
unsigned/ed25519/hybrid client, expected outcomes per table in §3,
plus expired-timestamp and wrong-key rejections. Green = matrix pass
on the builder, not unit tests alone.

## 7. Non-goals

Pure-PQC cutover, stateful hash-based signatures, Falcon-512 as first
algorithm, touching the Ed25519 payload bytes.
