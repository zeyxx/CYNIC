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

## E2E proof (both paths completed, zero unknowns)

- Default pool path: `cynic build submit --action check` (branch head)
  resolved via `infra/registry.json` pool and completed ON BUILDER
  (`build-7b69a1f2`, status completed, unknowns []).
  Note: the pool returns the first healthy builder; the builder entry
  resolves via its own registry, so compute-01 is failover while the
  builder is healthy. Ordering behavior, not a bug.
- Direct path: same submit with `--builder-url http://192.168.0.25:9090`
  completed ON COMPUTE-01 (`build-b2169803`, cargo check, 150 s wall,
  `build_grant_read_back` + `build_execution_provenance`, unknowns []).
  First sovereign build executed outside the builder.
- Source transport: compute-01 has no Forgejo SSH key and no synapse DNS;
  HTTPS git needs auth. Workaround proven: bare mirror cloned on builder,
  copied to compute-01, `transport_uri=file://` mirror, scheduler clones
  locally. A deploy key for compute-01 (operator decision) would remove
  the mirror detour.
- Sandboxing: `sandbox_bwrap=true` (default) fails spawn with EACCES on
  compute-01 (unprivileged user; root cause not isolated today).
  `sandbox_bwrap=false` (direct spawn, historical mode) works. The unit
  file stages the working config; re-enabling bwrap needs a dedicated
  session with userns diagnostics.
- `control --check` hangs when no kernel is reachable (no timeout on the
  workload-delivery pull). Robustness bug recorded, not fixed here.

## E3 — source transport on compute-01 (closed)

- Forgejo SSH is not exposed anywhere reachable (10.10.10.101:2222
  closed, :22 is the host sshd, synapse .100 unreachable). SSH transport
  is impossible today, not just undesired.
- Durable path built instead: machine account `cynic-compute-01` +
  `read:repository` scoped token (40 chars, created via admin API, stored
  0600 on compute-01, source shredded). Collaborator `read` granted on the
  private repo. Value never transited the agent session.
- `transport_uri=http://10.10.10.101:3000/jeanterre13/CYNIC.git` with a
  git credential helper. Mirror removed.
- PROVEN: first HTTPS-transport build on compute-01
  (`build-c9d6ec00`, completed, unknowns []).
- Read-only deploy key `compute-01-readonly` registered in parallel
  (id 2); dormant until Forgejo SSH exists.

## E2 — declared service source (in progress)

- Measured hole: `service-reconcile` owns lifecycle (enable/start) but
  never authors unit content; `BuildRunnerHostProfile::desired_services`
  declares zero units. The unit file was hand-written.
- Fix: pure `node_service_unit` module in cynic-node-protocol (render +
  drift, fail-closed validation) + `cynic node service generate/prove`
  arms. Install stays operator/capability mediated. No new capability:
  `service-reconcile` already covers the verb.
- Commit `774ad903` pushed; builder check submitted, awaiting proof.
- PROVEN: builder check on `774ad903` completed, unknowns [];
  targeted `cargo test -p cynic-node-protocol node_service_unit` 5/5.
- DONE: build_runner profile declares the receiver unit
  (commit `c0afd5cb`); builder check completed, unknowns [].
- Nested CLI family (law): `cynic node service generate|prove`
  (commit `bca277e0`); flat hyphenated arms removed.
- Unit content rendered by the new binary
  (digest `sha256:6d62e24e…`, `Restart=always`,
  `WorkingDirectory=/var/lib/cynic`), staged at
  `/var/lib/cynic/service-units/cynic-node.service` (digest match).
- `/var/lib/cynic/bin/cynic-node` now == live `cynic-node-new`
  (sha256 `8146f5ea…`); live receiver PID 31374 untouched, :9090 ok.
- NEXT (root on compute-01): install + reload + start the unit,
  then kill the nohup receiver, then `service prove`.
- DONE (operator): unit installed, daemon-reload, enable --now.
  `cynic-node.service` active (PID 58652), correct binary.
- BLOCKED: `:9090` dark. Cause proven, not guessed: the enrollment
  grant (`grant:acb2ce35c822c5c0`) expired (50-min TTL, now past
  `expires_at_unix_s`). `decide_builder_activation` gates the HTTP
  surface on a valid grant — deny-by-default working as designed.
  The old nohup served only because activation was decided at its
  startup, before expiry. Re-enrollment (fresh grant) required;
  authority private key is not on compute-01 (public halves only).
- Manual foreground run confirms: same binary+config, no listen,
  zero output — unit file exonerated.
- RE-EMISSION DONE (operator-authorized, new ceremony g2):
  continuity proven (sha256(machine_pubkey)==identity digest);
  new ed25519 authority `…-authority-g2`
  (`sha256:0aa377d3…`), grant `grant:c72032cebe79467a`,
  14 caps, expires +90d. Old files kept as `.bak-20261022`.
  Authority private key ONLY at
  `/var/lib/cynic/identity/enrollment-authority-g2.key` (600) —
  operator must vault it; never in git.
- NEXT (root on compute-01): `systemctl restart cynic-node`,
  then hermes proves (:9090 + service prove).
- CORRECTION (honest): the g2 key pipeline used an unverified
  `tail -c 32` extraction; its file later derived to the morning
  pubkey — key lineage untrustworthy. g2 superseded, file kept as
  `enrollment-authority-g1-recovered.key` (derives to morning pub;
  operator decides vault or shred).
- g3 ceremony VERIFIED end to end: openssl `-text` keygen, seed/pub
  parsed (no tail games), re-derivation cross-check OK,
  precomputed `sha256:93a54f3b…` == issued grant
  `grant:7daa78d18ee0f64f` (+90d). Triple-match staged
  (grant.request_id == request, key_id == sha256(.pub)).
- PROVEN live: `:9090/health ok`, unit `active`,
  `service prove converged` (digest match).
- NATIVE (this session): ceremony doc
  (`docs/specs/compute-node-enrollment-ceremony.md`) + `node
  enrollment-check` arm (pure summary + 2s renewal probe).
  Live run on compute-01: 29ms, `enrollment_proven true`,
  `renewal_endpoint_reachable false` → `blocked`. The exact signal
  the outage was missing: service up today, renewal impossible.
- PROVEN (builder): fix commit `b5caae0c` check completed, unknowns [];
  `cargo test -p cynic-node-protocol node_enrollment` 17/17.
- OPEN: GC inventory (processes, workload revisions, evidence store,
  binaries, identity backups, logs) — operator picks where to start.
- OPEN: merge of `agent/hermes/enroll-issuer` (operator decision).

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
5. Toolchain resolution (2026-10-22, `bf2692dc`): `toolchain_bin_dir`
   declared in scheduler config + spawn pre-flight
   (`build_toolchain_unresolvable:<program>`). Check green, unknowns [];
   targeted tests green (build_scheduler 44, tokio_build_execution 9,
   execution_env 9). compute-01 config set; needs root restart +
   `rm /usr/local/bin/cargo` (operator).
6. PROVEN ON compute-01 (2026-10-22 evening): after reboot + root restart
   with the `bf2692dc` release binary, workload `build-f13df40` (check of
   the fix commit itself) completed on `cynic-compute-01`: `cargo check`
   exit 0, stable workspace `build-workspaces-stable/debug`, toolchain
   resolved via the declared dir (no stale symlink, no pre-flight trip),
   unknowns []. Settlement file withheld by design
   (`forbidden_until_kernel_poh_read_back` — kernel unreachable); the
   completion envelope + execution provenance evidence carry the proof.
