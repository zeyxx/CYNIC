# Node Enrollment V2 — zero-touch target model

Schema: `cynic.node_enrollment.v2` (target). Status: `draft`.

## Goal

Make node enrollment a single human approval away from fully automatic:
a node presents a single-use join token, an operator approves once, and the
system provisions peer/config/registry/probes without further human action.
Revocation is one command.

## Non-goals

- No new top-level CLI family. All verbs nest under `cynic federation`.
- No mutation of the host outside the existing runner-token + registry flow.
- No fabrication of observed state; probes remain the authority.

## Actors

- **Joining node** (`cynic node enroll` client, future): generates a keypair,
  requests a join token, waits for approval.
- **Approver** (human): single approval gate.
- **Kernel/registry**: issues single-use tokens, records enrollment, drives probes.

## Flow

1. Node generates an enrollment keypair and calls `cynic federation enroll-request`
   (future) with its public key + requested planes.
2. Kernel issues a **single-use join token** with a TTL (court-controlled, default
   15 min) and records a `stub` enrollment record in `infra/registry.json`
   (reusing the `node_enrollment` + runner-token pattern).
3. Operator performs **one human approval** (`cynic federation enroll-approve <token>`).
4. On approval the system automatically:
   - adds the WireGuard peer entry (mesh plane),
   - writes node config (planes, transports, services),
   - updates `infra/registry.json` to `candidate`,
   - runs the per-plane probes (NETWORK_ENROLLMENT.md §3),
   - promotes to `candidate` only when all probes green; otherwise stays `stub`
     with the missing path named.
5. A background plane-reachability watchdog re-probes and demotes on failure
   (regression: `candidate`/`observed` -> `candidate`/`stub`).
6. Promotion to `observed` happens after 24 h of green probes (runner polls).
7. Revocation: `cynic federation enroll-revoke <node>` removes peer + config +
   registry record in one command.

## Verbs (all under `cynic federation`)

- `enroll status [--json]` (this tranche): read-only per-plane probe report.
- `enroll-request` (future): node-side join request.
- `enroll-approve <token>` (future): human approval gate.
- `enroll-revoke <node>` (future): one-command revocation.

## Reuse

- `infra/registry.json` `node_enrollment` + runner-token pattern.
- NETWORK_ENROLLMENT.md status machine (stub/candidate/observed).
- Per-plane probes §3 (admin/service/mesh/egress).

## Debt register

- D1: `enroll-request`/`approve`/`revoke` not yet implemented (this tranche ships status only).
- D2: join-token TTL court policy lives in `node_enrollment.authority_policy.token_ttl_s`;
  V2 must consume it, not redefine.
- D3: 24 h observed promotion is not automated yet; manual.
- D4: revocation does not yet exist; manual registry edit required.

## Acceptance (this tranche)

- `cynic federation enroll status [--json]` runs §3 probes from the local machine.
- Per-plane green/red + qualified status (stub/candidate/observed) in output.
- Unit tests on `classify_qualified_status`.
- `cargo check -p cynic-cli` clean.
- Runtime proof on at least one host (+ fixture).