# CYNIC Agent-Source Audit — Debt & Asymmetries (machine-readable log)

This file logs concrete debt and asymmetries found while formalising the
agent-source protocol (`infra/agent-source-protocol.json`,
`crates/cynic-protocol/src/source_policy.rs`). It is an audit ledger, NOT
normative prose. Each entry is observed evidence, not a claim of completion.

## Pre-existing compile debt (OUT OF SCOPE for the source-protocol work)

These block `make check` on the whole workspace but are NOT introduced by the
source-protocol change. They must be repaired in their own commit, not bundled.

| id | crate | symptom | evidence | status |
|----|-------|---------|----------|--------|
| DEBT-KERNEL-1 | cynic-kernel | `E0583: file not found for module proof_history` | `crates/cynic-kernel/src/api/rest/mod.rs:30: pub mod proof_history;` — no `proof_history.rs` on disk | open |
| DEBT-KERNEL-2 | cynic-kernel | `E0425: cannot find coord_identify_handler` | `crates/cynic-kernel/src/api/rest/mod.rs:271` routes `/coord/identify` to `coord_identify_handler`, but only `coord_verify_handler` exists in `coord_lease.rs:37` | open |
| DEBT-KERNEL-3 | cynic-kernel | `git status` shows `M crates/cynic-kernel/src/api/rest/mod.rs` already modified before the source-protocol work began | observed at session start (18 modified files) | open |

Note: `cynic-kernel` pins toolchain 1.95 (`rust-toolchain.toml`); the other
core crates pin 1.96. `make check` forces a single root toolchain over the
whole graph and ICEs/errors on kernel handlers.

## Substrate / build asymmetries

| id | observation | evidence | resolution |
|----|-------------|----------|------------|
| SUBSTRAT-1 | cortex rustc SIGSEGV on `syn`/`serde_derive` (proc-macro codegen) — ININCURABLE substrat | `signal: 11, SIGSEGV` on `syn v3.0.3` / `serde_derive`, symptom changes between identical runs | build on builder (10.10.10.119), not cortex |
| SUBSTRAT-2 | builder disk saturated by a full `make check` (no `--lib` scope) | `No space left on device` writing `/var/lib/cynic/cargo-targets/...`; later `ld terminated with signal 7 [Bus error]` from corrupted partial artifacts | clean builder target dir, then scope tests to `-p cynic-protocol` only |
| TOPO-1 | cortex local remote `forgejo` points at the BUILDER (10.10.10.119), not the Forgejo SSOT (10.10.10.101) | `git config remote.forgejo.url` = `ssh://root@10.10.10.119/var/lib/cynic/repos/CYNIC.git`; host key `EoeJ…` ≠ enrolled Forgejo key `OJDM…` | repoint to `ssh://forgejo@forgejo.synapse.cynic:22/jeanterre13/CYNIC.git` (canonical_host) |
| TOPO-2 | `repo-remotes.json` + `cynic-source-topology` skill said `forgejo@forgejo.synapse.cynic:22`, reality was `root@10.10.10.119` | doc drift (3 conflicting identities) | `repo-remotes.json` now carries `canonical_host`/`canonical_url`/`fallback_ip` |

## Doc asymmetries (fixed in this change)

- `protocols/agent-git-protocol.md` was cited by `agent-protocol.md` but did not
  exist. Created; points to `source_policy.rs` + `source_control.rs` as law.
- `agent-protocol.md` Part 13 checklist marked `cynic agent branch/pr` as
  delivered though it does not exist. Corrected to `[ ]` + pointer.
- `.agent-claims.json` schema is `cynic.agent_claims.v2` (real) vs `v1` in
  `agent-protocol.md` Part 4. No code change; documented in
  `infra/agent-source-protocol.json` rules.

## Proof status of the source-protocol change itself

- L1 Rust (`source_policy.rs`, `source_control.rs` extensions) is written and
  committed on branch `agent/hermes/agent-source-protocol`.
- Compile/test proof on `cynic-protocol` alone was NOT yet obtained: cortex ICE
  (SUBSTRAT-1) + builder disk saturation (SUBSTRAT-2) blocked it. Plan: clean
  builder, then `cargo test -p cynic-protocol --lib source_` (scoped, avoids
  DEBT-KERNEL-*).
- Pushed to real SSOT (forgejo.synapse.cynic, 10.10.10.101) — Forgejo offers a
  PR from the branch. Awaiting review/merge.

*This ledger is evidence, not a completion report. Update only with observed
proof, never with claimed success.*
