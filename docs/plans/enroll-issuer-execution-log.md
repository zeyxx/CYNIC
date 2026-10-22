# enroll-issuer execution log — 2026-10-22

Branch: `agent/hermes/enroll-issuer`. Goal: remove the enrollment issuance
bottleneck (brick 3) in code, then prove compute-01 as build_runner.
Approval: operator R + "tout aujourd'hui"; per-command gate approvals kept.

## The hole (mapped, not guessed)

- `admit_node_enrollment` (pure) had ZERO callers in the tree.
- No `/node-enrollment` server route exists in `cynic-kernel` or anywhere.
- No CLI surface issues grants (`node identity/csr/cert` exist; issue missing).
- The deployed `cynic-node-enroll` binary has no source in the tree
  (its `--help` string is absent from all crates).
- Consequence: every grant in the field was signed out-of-tree. The builder's
  own bundle grant carries `authority_key_id sha256:4c48b1...`, matching
  neither the on-disk authority key nor `/etc/cynic/enrollment-authority.pub`.
  Trust-root provenance is an open question for the operator (recorded, not
  resolved by this branch).

## What this branch adds (3 commits)

1. `cynic-node-protocol::build_node_enrollment_grant` — pure grant
   constructor, deterministic id, evidence_refs populated
   (`enrollment-request:<id>`, `enrollment-authority:<id>`).
   Without evidence_refs the reconcile check
   (`enrollment_grant_evidence_missing`) refutes even valid grants —
   found because the first issued grant failed read-back on compute-01.
   Note: the builder's own production grant has NO evidence_refs and
   would fail the tree's current reconcile check (pre-existing drift).
2. `NodeIdentityWorkflow::issue_signed_enrollment_grant` (cynic-host) —
   admit-first, sign with operator-held authority key (read-only, never
   generated here), structural revalidation before return.
3. `cynic node enrollment-issue --request --authority [--token-file]
   --signing-key --output` — thin surface; `--token-file` absent means
   operator-mediated issuance (`require_token=false`), always reported.
4. `infra/registry.json`: `cynic-compute-01` role `embedding_runner` ->
   `build_runner`, ssh transport corrected to observed reality
   (`cynic@192.168.0.25`, was stale `titouan`/password), `network.service`
   added (`192.168.0.25`, required by `resolve_builder_pool`).

## Proofs (builder, targeted — never workspace-all)

- `cargo test -p cynic-node-protocol node_enrollment`: 12/12.
- `cargo test -p cynic-host node_identity`: 6/6.
- `cargo check -p cynic-cli`: OK (after fixing a missing `Sha256` import).
- E2E `cynic build submit --action check` on the branch head: launched,
  result pending (background).

## Deployed today (compute-01, as cynic)

- `/var/lib/cynic/bin/cynic-node` (release v0.8.0, pre-existing) — kept.
- `/var/lib/cynic/bin/cynic-node-new` (v0.9.0, built from this branch).
- `/var/lib/cynic/bin/cynic` (v0.9.0 CLI, pre-existing builder build).
- `/var/lib/cynic/node-control.toml` (adapted: LAN listen, local roots).
- `/var/lib/cynic/identity/`: pre-existing `node-signing.key` (proven to
  match the Oct CSR pubkey via CLI observe) + issued `enrollment-grant.json`
  + `enrollment-request.json` + fresh authority `enrollment-authority.pub`.
- Receiver live: `http://192.168.0.25:9090/health` = 200. Enrollment
  read-back converged (earlier explicit blockers gone from logs).
- Fresh per-node authority `cynic-compute-01-enrollment-authority`
  (keypair generated on builder by the new CLI; private key never left
  the builder; pub installed on compute-01).
- Stray `/etc/cynic/enrollment-authority.pub` on builder (staged by an
  earlier throwaway enroll run, trust root of a destroyed key) deleted.

## Left for the operator (explicit, no implicit escalation)

1. Builder registry deploy (cynic cannot write root-owned
   `/etc/cynic/registry.json`): staged at
   `/var/lib/cynic/registry.json.new` (backup of current at
   `/root/c03-hypervisor-20261022/registry.json.bak` — pve side, plus
   repo SSOT in this branch). Install: copy staged file over
   `/etc/cynic/registry.json` as root on builder.
2. Receiver unit install on compute-01 (`docs/plans/cynic-node-receiver.service`
   in this branch; binary must first be renamed `cynic-node-new` ->
   `cynic-node`): needs titouan sudo. Receiver currently runs under nohup.
3. Trust-root archaeology: three authority identities observed (on-disk key,
   stale pub file (removed), builder bundle grant signer). Nothing forged;
   the new per-node authority sidesteps the question for compute-01 only.
4. `--check` hangs when the kernel is unreachable (no timeout on the
   workload-delivery pull) — robustness bug recorded, not fixed here.
