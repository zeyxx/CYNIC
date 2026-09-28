# Next Session Brief — coordination re-ownership, items 2 and 3

- **schema**: `cynic.handoff.brief.v1`
- **status**: `read_only_handoff` — this document is a handoff, not authority. It carries no decision.
- **written**: 2026-09-28, at the end of the session that produced it.
- **PROTOCOL EXCEPTION, recorded deliberately**: this file was written **without a claim**.
  `cynic agent claim` and `cynic agent status` both return `Missing Bearer token or Signature
  headers` — the coordination channel is down, which is the very defect items 2 and 3 exist to
  fix. Writing outside a claimed scope is a protocol violation, so the operator authorised this
  one instance explicitly. It was repeated here and in the commit message because an exception
  that is not written down is indistinguishable from a mistake. The file is new, so no other
  agent's work could conflict with it, and no compilation depends on it.

## 1. Sequence position

Decided and done:

- `contracts/pruning-policy.json` and `infra/crate-topology.json` ratified `normative`.
- Orphan ratchet extended to all 47 crates. 5 crates had no gate at all; `cynic-kernel` had one
  listing `&[]` next to 72 real orphans. 277 orphans now measured and visible.
- `cynic audit paths` added (`cynic-host::audit_engine`, rules PATH-001/002/003), then its
  false positives cut 40 -> 21 before any finding was actioned.
- Renames derived from behaviour, not from the old name: `pattern_gc` -> `rate_classify`,
  `MetabolicState` (host load) -> `LoadTier`, the unrelated L3 `struct MetabolicState` ->
  `ObservationCounters`. `homeostasis` / `GarbageCollector` / `Waste` deleted as unreachable and
  built on a deleted authority.
- Git object store repaired: 16 zero-byte objects. Root cause was `write_loose_object` calling
  `freshen_packed_object`, which short-circuits on file existence and never verifies size.
- `port: 3030` restored to the `cynic-kernel` service in `infra/registry.json`, and the test that
  had been inventing that field in its own fixture was replaced with one that reads the real file.

Decided and **not** done: `cynic-api` (item 2), the claim store (item 3), `A1` (MCP 9 vs 114),
the crate fusion matrix.

## 2. Item 2 — `cynic-api`

**Why a crate and not a module.** `crates/cynic-kernel/tests/architecture_boundary.rs:88-99`
forbids `pub mod api;` in the kernel, with the comment *"asserted their PRESENCE — aspirational
fiction. Lock absence."* And `crates/cynic-kernel/src/bootstrap/runtime_orchestrator.rs:9` already
names it: `// TODO: Wire storage (cynic-storage), API (cynic-api), agents (cynic-agent-bus)`. A
crate the repository names and does not have.

**Specification.**
- L3, its own `[[bin]]`, `depends_on: ["cynic-protocol"]` and nothing else in the first commit.
- Routes `/coord/{identify,register,claim,release,who}` — the five the CLI already calls
  (`crates/cynic-cli/src/agent_surface/coordination.rs:125,178,224,305,672`). Nothing more.
- Port **read** from the registry, never written.
- `infra/crate-topology.json` amended **in the same change**, or `ARCH-004-UNDECLARED-EDGE` goes
  red. That is the intended behaviour, not an obstacle.
- The smallest verifiable step is a listener that answers, not a handler.

`pruning-policy.crate_justified` justification: condition 2 (distinct service requirement) and
condition 5 (a boundary the DAG imposes — the kernel is not permitted to have this surface).

**Why it can land before item 3.** `cynic-api` needs no store to start. It can bind and serve
against the L0 schema alone, which is what makes item 2 independently verifiable.

## 3. Port constitution — four clauses

1. **A port is declared, never written.** It lives in the service entry of `infra/registry.json`.
   Every port literal in `crates/**/*.rs` must match a declared `port`. The check is **derived**,
   not a list of forbidden values, so a newly declared port passes and a newly hidden one fails.
2. **A service does not choose its port.** It reads the port declared for its role and fails
   loudly when the registry declares none. Today's behaviour is the inverse:
   `resolve_kernel_base` returns `None` silently and the agent degrades to file mode — which is
   how a missing `port` key went unnoticed.
3. **Contention is a named conflict, not a fallback.** On a taken port the service fails and names
   the tenant. The vocabulary already exists at L0 (`CoordinationCode::ScopeConflict`). A silent
   fallback is the same class of defect as a silent `None`.
4. **Disagreement is a violation.** Registry says 3030, service uses 9090, that is `ARCH-004` —
   same both-ways equality the crate topology now enforces.

## 4. Established facts — do not re-derive

- **Four hardcoded 3030s:** `crates/cynic-host/src/build_evidence.rs:196`,
  `crates/cynic-host/src/federation_runtime.rs:370`,
  `crates/cynic-host/src/registry_divergence_probe.rs:385`,
  `crates/cynic-kernel/MANIFEST.yaml:57`.
- **The CLI never reached the route.** `resolve_kernel_base` required `port`; the registry omitted
  it; every `cynic agent *` took the local file branch without attempting HTTP. Fixed in
  `acfb544e`, but there is no listener behind it yet.
- **`cynic-storage` already has a durable L0-typed claim ledger:** `impl BookEntry for
  cynic_protocol::agent_coordination::Claim` plus `TypedBook<Claim, W>` over WAL + SQLite, with a
  live test and **zero writers** (`crates/cynic-storage/src/book/mod.rs:264,308`). This is the
  asset item 3 builds on.
- **Item 3 template:** `crates/cynic-kernel/src/infra/agent_session_sqlite/`. State persisted to
  terminal, generation bump as fencing, evidence appended, CAS on every write, index
  `(state, expires_at_unix_ms)`, domain validation before and after SQL. **Its own `expire` has
  no caller outside tests** — so item 3 must deliver the trigger, not only the sweep, and
  `crates/cynic-kernel/src/infra/tasks/` is itself an orphan to hang it on.
- **Four parallel claim domains:** L0 `agent_coordination` (types, no store) · `cynic-agent-bus`
  (`Arc<Mutex<HashMap>>`, `expire_claims` implemented and never called, types contradicting L0 on
  identity, budget width, consumed shape, and missing `fencing_token`/`lease_id`) · workspace
  claims in `cynic-node` (five dangling kernel routes) · the CLI file. The kernel's
  `storage/schema_manifest.json` already declares `agent_workspace_claim` and
  `agent_workspace_claim_lock` with `fencing_token`, `expires_at_ms`, `hard_expires_at_ms` DDL, and
  no Rust code touches those tables.
- **Model to copy:** `cynic-node` is the only axum server in the workspace — 4 routes on 9090,
  `crates/cynic-node/src/usecases/builder_http.rs:96-102`.

## 5. Build and test state

- `cargo check -p cynic-cli --lib` clean.
- **The `cynic` binary does not link on this host** — rust-lld `sh_offset` SIGSEGV, the pinned ICE
  path documented in `Cargo.toml`. No fresh `cynic audit` run is possible here.
- `cargo test -p cynic-host --lib` **does not compile**: `cynic-protocol/src/convergence.rs` is
  committed but not declared in `lib.rs`, so `cynic_protocol::CausalEpochKey` does not exist while
  host test code imports it. Use **integration** tests, which link the lib rather than its
  `cfg(test)` code.
- `cargo test -p cynic-cli --lib` compiles; **3 tests red**, all pre-existing:
  `agent_surface::coordination::tests::builder_endpoint_resolves_from_registry`,
  `command_manifest::tests::manifest_domains_match_clap_surface`,
  `mcp_call_proof_index::tests::proof_index_tracks_rollout_gradient`.
- `crates/cynic-agent-bus/tests/claim_expiry_debt.rs` is **red**: the expired-active claim debt
  passed its baseline of 165. Do not raise the baseline without the mechanism.

## 6. Protocol — the two failures of the session that produced this

1. **A failed claim must abort the script.** A `scope_conflict` on
   `crates/cynic-host/src/lib.rs` did not stop a python script, which wrote the file anyway.
   Reverted immediately, no harm, but the lesson stands: a guard that does not block is not a guard.
2. **The coordination channel is down.** `claim` and `status` both fail on
   `Missing Bearer token or Signature headers`. The protocol that says "mutate only inside a
   claimed scope" is therefore currently unexecutable — which is the concrete operational form of
   everything the vapourware inventory measured.

Also standing: 150 active claims are past their own `temporal.ttl_seconds`, two of them holding
`crates/cynic-kernel/src`. Measured by `crates/cynic-agent-bus/tests/claim_expiry_debt.rs`,
shrink-only. And `consumed` is uniformly zero across all 288 active claims because nothing writes
it — a field that is always one value is indistinguishable from a field that does not exist, so it
must never be used as evidence of liveness.

## 7. What not to re-open

`pruning-policy.json` and `crate-topology.json` are ratified; the fusion dispositions and the
topology gate derive from them and are not opinions. The `MetabolicState` rename is done and its
wire names are frozen deliberately — `#[serde(rename)]` keeps `survival_cpu_pct` on disk because
`CYNIC_LOAD_POLICY` points at a JSON file on every node, while the Rust identifier is the honest
one. The contested-claim procedure is settled: verify the evidence (file clean, work committed,
claim counter uniform), change only the lines needed, and record the contest in the commit.
