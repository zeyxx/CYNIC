# Native node deployment — the last mile

- **Status**: Draft, for decision. Nothing here is implemented by this document.
- **Author**: claude (agent), 2026-10-25
- **Scope**: how a new `cynic` / `cynic-node` binary reaches a runner (`cynic-builder`, `cynic-compute-01`) without SSH or an unauthenticated shell.

## 1. Problem

A runner's binary can only be replaced today by running `make release.deploy`
on the machine, through SSH (`infra/deploy-locations.json`, per machine). The
node's own restart signal assumes the binary is already in place:
`builder_http.rs`, `/admin/restart` — "install first, then signal — never the
reverse".

The only remote executor on the node is `POST /exec`, an arbitrary `sh -c`
gated by the webhook HMAC secret. It must not become the deploy path.

Measured consequence (2026-10-25): the deployed `cynic` on cortex is 213
commits behind `main`, the builder's is 246 behind, and neither carries
fixes that are already merged (disk probe, runner resolution). A fix on `main`
does not reach a runner by itself.

## 2. What already exists (proved by reading the code)

| Piece | Where | State |
|---|---|---|
| Desired state per node, with a release inside | `cynic-node-protocol/src/node_control.rs` `NodeDesiredState`, `SignedNodeControlPlan` | type + validation |
| A release request: signed basket, digest, detached signature, signer identity, install root, staged slot, `current_link`, `previous_link`, `postflight_argv` | `cynic-protocol/src/runtime_seed.rs` `RuntimeSeedRequest` (24 fields) | type |
| Declaration of a release from a proven build | `cynic-build/src/runtime_release.rs` `RuntimeReleaseDeclaration` | library only |
| Node pulls its plan | `cynic-node/src/adapters/http_node_control.rs` `pull_desired_state` → `GET /node-control/{node_id}/desired-state` | client |
| Stage the basket, digest-checked, inside an inbox | `cynic-node/src/adapters/http_runtime_artifact_transport.rs` (`https_only`, `allowed_root`, `max_bytes`) | adapter |
| Activate locally with swap and rollback links | `cynic-node/src/adapters/local_runtime_seed_actuation.rs`, `runtime_artifact_deployment_uc.rs` | adapter |
| Verify a signed plan | `cynic-host/src/plan_verifier.rs` | library |
| Whole chain, with fake adapters | `cynic-host/tests/autonomous_pipeline_e2e.rs` | test only |

## 3. What is missing (proved by absence)

1. **No server for the plan.** Nothing in `cynic-kernel` serves
   `/node-control/{node_id}/desired-state`. A search of `crates/cynic-kernel/src`
   finds no route and no producer of `SignedNodeControlPlan`.
2. **No operator surface.** No CLI creates a `RuntimeReleaseDeclaration`
   (`cynic release` does not exist; `cynic-cli` never references the type).
3. **No producer of a signed basket.** `cynic build artifact release` attaches
   an artifact and a manifest to a Forgejo release.
   `cynic-build/src/release_publication.rs` only validates the read-back of a
   published basket (digests, sizes, sources); a search for `allowed_signers`,
   `ssh-keygen` and `detached_signature` outside tests finds nothing that
   creates the detached signature.
4. **Transport mismatch.** The node transport is `https_only`; Forgejo is
   served over `http://10.10.10.101:3000`. A basket hosted there is refused.
   `RuntimeArtifactTransportKind` has only `LocalFile` and `Https`.
5. **Unverified runtime facts** (not read, not assumed): whether the kernel on
   cortex runs in node-control mode, and whether `cynic-builder` and
   `cynic-compute-01` are enrolled (`NodeControlConfig.enrollment_*`).

## 4. Proposed flow

```
proven commit ─► release build (builder) ─► basket + digest
      ─► operator signs basket (detached, allowed_signers)
      ─► publish basket + signature
      ─► kernel: RuntimeReleaseDeclaration ─► SignedNodeControlPlan (per node, generation++)
      ─► node: GET /node-control/{id}/desired-state, verify plan signature
      ─► stage into inbox, verify digest, verify detached signature
      ─► install into staged slot, swap current_link (previous_link kept)
      ─► postflight_argv; failure ⇒ swap back
      ─► node reports observed generation; operator reads it back
```

Every arrow has a receipt. Acceptance is observed state, not exit codes: after
the swap, `GET /capacity` on the node returns `state_root`, `free_ram_bytes`,
`total_ram_bytes` and `disk_avail_bytes`, and `cynic version --json` on the
runner reports the declared `source_commit`.

## 5. Decisions required

**D1 — Where the basket is hosted, given `https_only`.**
1. Put TLS in front of Forgejo (or host baskets elsewhere over HTTPS).
2. Add a plain-HTTP transport kind, limited to the service plane. The basket is
   already authenticated by digest and a detached signature, so confidentiality
   is not the property at stake — but it relaxes a stated boundary.
3. Push the basket into the node inbox first and use `LocalFile`.

**D2 — Who signs.** Operator-held key for the basket (`allowed_signers`), kernel
authority key for the control plan. The agent never holds either.

**D3 — Kernel route.** Implement `GET /node-control/{node_id}/desired-state`
returning a `SignedNodeControlPlan`; storage of the desired state and the
generation counter must be decided with it.

**D4 — Declaration surface.** A `cynic` subcommand that turns a commit with a
passing build proof into a `RuntimeReleaseDeclaration`, refusing a commit
without proof.

**D5 — Daemon restart.** After the swap, the running `cynic-node` still holds the
old image. Either the activation restarts the unit, or the existing signed
`/admin/restart` is sent with the new sha256 (it already refuses a mismatch).

## 6. Rollout order

1. `cynic-compute-01` first: it takes no builds from the agent, so a failure
   blocks nothing. Acceptance: the four new `/capacity` fields, then `fleet
   observe --no-ssh` shows it usable.
2. `cynic-builder` when `in_flight` is 0 (drain first).
3. The `cynic` client on cortex last.

## 7. Non-goals

- Replacing `make release.deploy` on a single-host install.
- Using `POST /exec` for anything that installs code.
- Handling the signing keys in an agent session.

## 8. Risks

- Activation is a swap of an executable on a production host. The previous
  link and the postflight check are the only rollback; both must be exercised
  on `cynic-compute-01` before the builder.
- The E2E test uses fakes. Real enrollment, real signing and a real swap have
  never run together in this repository's history that I can see.
- Items in section 3.5 can change this plan; read them first.
