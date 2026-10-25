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

## 5. Decisions

Numbered N1 to N5. The storage taxonomy (`infra/storage-taxonomy.json`) already
owns D1 to D3; these are not those.

**N1 — Where the basket is hosted, given `https_only`.**
Operator direction (2026-10-25): plain `http` must be secured everywhere; the
platform is moving toward production grade and this change must not regress
it. A plain-HTTP transport kind is therefore rejected. Direction: secure the
channel (TLS in front of Forgejo, trusted by the nodes) and keep `https_only`.
Open: whether an internal certificate authority exists and who trusts it.

**N2 — Who signs.**
Operator direction: signing must be granular, and there is currently no
stable operator control plane. So a per-release human signature cannot be the
only path yet. Proposal in section 9.

**N3 — Where the desired state lives.**
Operator direction: study the full current storage model first. Done in
section 8; it changes the answer from "a file in the kernel" to a split by
data kind.

**N4 — Who triggers a release.**
Operator direction: a hybrid or more granular model. Proposal in section 9.

**N5 — Daemon restart after the swap.** Not yet decided.
1. The activation restarts the unit itself: one step, but the node cuts itself
   off mid-activation, so the report must be persisted first.
2. The existing signed `/admin/restart` with the new sha256: it already refuses
   a mismatch; two steps, each with a receipt; needs the admin secret.
3. No restart: the link points at the new binary while the old one runs.
   Inconsistent; not recommended.

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

## 8. Storage model applied to deployment

`infra/storage-taxonomy.json` is `proposal_v1`; its decisions D1 to D3 are
`DECIDED_pending_operator_confirmation`, i.e. not confirmed. It defines eight
data kinds and the rule `one_kind_one_home`. Mapping each datum this feature
touches:

| Datum | Kind | Home | Note |
|---|---|---|---|
| Which release each node should run (desired state) | K6 contracts_registries | git (`infra/registry.json` or a contract) | `contracts_are_git`: never duplicated into a database; a store may cache it |
| Signed control plan issued to a node, and its generation | K1 immutable_evidence | append-only signed journal | consistent with taxonomy D1 (journal first, SurrealDB only as a rebuildable projection) |
| The release basket (binary, digest) | K5 artifacts_blobs | content-addressed store, immutable once placed | |
| Basket signature key, plan signing key, admin secret | K8 secrets_identity | `/etc/cynic`, never git or DB | out of every plane by law |
| Node-side lease and in-flight state | K2 operational_state | ACID, fenced | |
| Deployment receipts, observed generation | K1 | evidence store | |
| `/etc/cynic/registry.json` | K6 materialization | `/etc/cynic`, "materialized from git at deploy time" | see below |

Two consequences:

1. **N3 is not "a file in the kernel".** The desired release is K6 and stays in
   git. The kernel signs a plan *derived* from it and journals the issued plan
   (K1). No new store is needed for the desired state; the open questions are
   the journal location and the generation counter, both K1/K2 concerns the
   taxonomy already frames.
2. **The stale installed registry is the same defect, one layer down.** The
   tree says `/etc/cynic/registry.json` is materialized from git at deploy
   time. On this host it was last written 2026-10-23 and differs from git
   (compute-01 as `titouan@192.168.0.25`, no `build_runner_default`). The
   materialization step is the first piece of the deployment chain and it is
   not running. Fixing deployment without it leaves the runners reading a stale
   registry.

Not verified: how the kernel stores its state today (SurrealDB was unreachable
when taxonomy D1 was written; the service is active now), and whether the
journal described in taxonomy D1 exists on disk.

## 9. Granularity proposal for N2 and N4 (for reaction)

Authority is scoped by blast radius, not by a single key and a single trigger.

| Tier | Scope | Trigger | Signature |
|---|---|---|---|
| T0 | no executable (docs, contracts, registry data) | merge to `main` | none; git is the authority |
| T1 | canary runner (`cynic-compute-01`), no agent build depends on it | automatic after a green build proof on `main` | a canary-only key, valid for canary nodes only, held by the kernel |
| T2 | production runner (`cynic-builder`), then the `cynic` client on operator hosts | operator promotion of a release that already ran on T1 | an operator-held key; no agent can issue it |

Properties: an agent can cause a T1 deploy only through a proven commit and
only on the canary; it can never cause T2. The canary key cannot sign for a T2
node, so compromising it does not reach production. T2 stays human until a
stable operator control plane exists, and then the promotion can be delegated
by policy instead of by hand.

Rollback is per tier through `previous_link`, and T1 is where it must be
exercised before any T2 promotion.
