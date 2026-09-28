# Governance Audit — gates, taxonomy, and the Makefile/CLI inversion

- **schema**: `cynic.audit.governance.v1`
- **status**: `read_only_audit`
- **method**: observation only. One file claim taken to write this report. No Makefile, no contract,
  no production file mutated. The clap surface was reflected from a built binary on the build
  runner, not parsed from source, because source parsing gave two wrong answers.
- **authority_order applied**: observed runtime state > `contracts/foundation.json` > `infra/runtime-authority.json` > `infra/registry.json` > workspace `Cargo.toml` > canonical crates > `infra/agent-exposure.json`

## 0. Correction to earlier work in this series

`CLI_SURFACE_AUDIT.md` and the conversation around it asserted that "the only target running
`cargo test` is `check-layers`". That is correct. A script written to enumerate the Makefile
concluded otherwise, because it treated `.PHONY` lines as target headers and so mis-bounded every
recipe. The script was wrong; the assertion was right. Recorded because the same class of
text-parsing error produced a false 134-orphan figure in the first audit attempt.

A second correction, larger: **the executable governance for orphaned sources already exists and
is better than anything proposed here.** `tests/support/orphan_sources.rs` is shared by 8 crates,
follows `mod`, `#[path]`, `include!`, `[[bin]]`/`[lib]` targets and `src/bin/*.rs` autodiscovery,
fails in both directions, and was validated by probe. "A7: add a build-graph test" was the wrong
task. The test is written. The gate does not run it.

## 1. The gate topology: two gates, one canonical

| target | runs tests | in canonical gate | covers |
|---|---|---|---|
| `check` (Makefile:68) | **no** | **yes** | `cynic audit crate --json` via a prebuilt binary |
| `audit` (Makefile:78) | **no** | **yes** | `cynic audit all --json` via a prebuilt binary |
| `check-layers` (Makefile:106) | **yes**, `cargo check` + `cargo test`, fail-closed | no | 5 crates |
| `check-strict` | `cargo check -D warnings` | no | `CORE_CRATES` |
| `test` | yes, `-p` over `CORE_CRATES` | no | 7 crates |
| `check-installed-runtime` | yes, 1 test | no | portability invariants |

`AGENTS.md` names `make check`, `make audit` and `make release-deploy` as the canonical gate.
**Both canonical targets execute a prebuilt binary and never launch a test.**

The consequence is exact and dated. The quarantine list in
`crates/cynic-cli/tests/orphan_sources_ratchet.rs` was last updated `ae90eb10` on **2026-08-31**.
Commit `1fa6ec3d` on **2026-09-26** made 43 files unreachable without touching the list. Running
the ratchet on the build runner today:

```
cynic-cli: 43 source(s) unreachable from lib.rs/main.rs and not quarantined.
new orphans: [ bootstrap_surface.rs, chat_tui/state.rs, dev_surface/*,
               lifecycle_surface*, surface_audit/*, proxmox/*, ... ]
```

**The instrument was correct, went red on 2026-09-26, and has been red and invisible for one day
across 8 crates.** No rule was absent. The rule was executed by a target that nothing invokes.

### 1.1 `check-layers` has a coverage hole

```
CORE_CRATES  := cynic-protocol cynic-host cynic-node cynic-kernel cynic-cli cynic-inference cynic-mcp
LAYER_CRATES := cynic-protocol cynic-host cynic-node cynic-kernel cynic-cli
```

`check-layers` iterates `LAYER_CRATES`, so the two crates in `CORE_CRATES` but not
`LAYER_CRATES` are never test-run by the layered gate:

| crate | production orphans | orphan LOC |
|---|---|---|
| `cynic-mcp` | 42 | 12 812 |
| `cynic-inference` | 10 | 2 696 |

`cynic-mcp` compiles 1 file of 43. The one target that runs tests skips it. Two lists with
overlapping names, differing by two entries, and no comment saying which is authoritative.

## 2. Taxonomy: the hyphen rule is already met at the top level

Reflected from the built binary, not from source.

| level | count | containing a hyphen |
|---|---|---|
| top-level commands | 26 (+`help`) | **0** |
| reachable subcommands, depth ≤ 2 | 208 | **59** |

So the rule "no `*-*` commands" is **already satisfied at the root**: `commands models doctor
court discovery errors advanced runtime session network node source git judge update status audit
agent chat task federation build mask queue forensics version`. The violation is entirely at
depth 2, and entirely in two domains:

- `runtime` — 44 subcommands, ~31 hyphenated: `inference-status`, `inference-vault`,
  `inference-hf-download`, `inference-models-consolidate`, `storage-capabilities`,
  `network-preflight`, `agent-matrix`, `deps-audit`, `disk-clean`, …
- `network` — `mesh apply-plan`, `mesh target-attestation`, `provider-fallback`,
  `transport security-audit`, `peer submit-tx`.

### 2.1 Stripping hyphens is the wrong fix, and would produce a worse CLI

`inference-status` → `inference status` is correct. `list-proofs` → `list proofs` is not; it
should be `proof list`. `provider-fallback` → `provider fallback` is not; it is an adjective on
a noun and belongs inside a hierarchy, not beside a verb.

The defect is not the hyphen. It is that `runtime` encodes a **flat verb-noun set at one level
where a noun hierarchy is missing**. The repository already contains the correct form —
`cynic models list`, `cynic models start`, `cynic models stop` — and
`INFERENCE_SURFACE_AUDIT.md` §5 records that 4 of the 7 declared inference MCP tools have a
`cli_equivalent` of `cynic models list` or `cynic models start`.

So the taxonomy change is to promote the noun: `cynic runtime inference-status` becomes
`cynic inference status`, and the 15 `inference-*` variants collapse onto the existing
`models` hierarchy where they duplicate it. That is a routing decision per variant, not a
transformation. A mechanical de-hyphenation would raise the count of commands while making them
harder to discover, which is the opposite of the rule's intent.

## 3. The Makefile is more capable than the CLI

31 real targets. **30 have no `cynic` command equivalent.** The single overlap is `audit`.

The inversion is concrete: `make deps` exists and works, while `cynic deps` **does not exist** as
a command. Same for `bootstrap`, `dev`, `lifecycle`, `fleet` — every one of the five manifest
domains that `INFERENCE_SURFACE_AUDIT.md` §4.1 records as advertised-but-unroutable has a working
`make` target standing in for it.

That is why the request to "use the `cynic` CLI commands" cannot be executed as stated: for 30 of
31 targets there is nothing to use. The CLI is not a superset of the Makefile; it is a strict
subset.

| make target | `cynic` equivalent | note |
|---|---|---|
| `check`, `check-strict`, `check-layers`, `test` | none | cargo is the right tool here; not a CLI surface |
| `check-installed-runtime` | none | one integration test |
| `surface-audit`, `audit-surface-matrix`, `audit-runtime-phases` | `cynic audit` | partial overlap, not equivalent |
| `authority-check`, `secret-literal-check` | `cynic audit` | the secret lint is `cynic secret lint` |
| `converge`, `operator-day`, `operator-loop`, `operator-reports` | none | orchestration shell |
| `deps` | **should be** `cynic deps` | `RuntimeCommand::Deps*` exists and is dispatched; only argv is missing |
| `lab-*`, `framework-bootstrap-bundle`, `local-check` | none | backed by `cynic-host::lab`, unreachable from argv |
| `run-kernel`, `run-node`, `install-*`, `systemd-list`, `kani-*`, `release-deploy` | none | host operations |

Six targets are legitimately `cargo` wrappers and should stay. The rest are either CLI surfaces
that were never routed, or orchestration that has no home yet.

## 4. Complexity that is already self-documented as a problem

The 36-line `.ONESHELL` preamble is mostly environment plumbing, and it carries its own
post-mortem in a comment:

> Stack size — the previous hardcode of 8G/16G/32G escalated memory → MORE cold rebuilds → MORE
> transitive dep ICE exposure … 256 MiB avoids the OOM cascade; warm incremental cache avoids dep
> recompile.

And the `test` target warns:

> warning: on 2026-07-16 rustc 1.96.0 can still ICE while compiling workspace tests

That is consistent with what was observed on the operator host during this work: a `rustc
SIGSEGV` that moved from crate to crate across 12 attempts, on the same `1.95.0` that compiles
cleanly on the runner. The ICE is a **known, documented condition of workspace test builds**,
not an unknown. `AGENTS.md` records the pinned ICE paths; the Makefile records the same fact with
a date. Neither is wired to the gate.

The 31 targets with 9 `cargo` invocations and 10 `CYNIC_BIN` fallbacks mean the same logical
operation is expressed in two dialects — `make` and `cynic` — and the operator must know which
target is a binary call and which is a cargo call to predict whether it compiles anything.

## 5. Proposed governance changes

Ordered so that each is independently verifiable and none can make the shared gate red by surprise.

| # | change | proof | risk |
|---|---|---|---|
| **G1** | Resolve `LAYER_CRATES` vs `CORE_CRATES`: one list, and include `cynic-mcp` and `cynic-inference` | `check-layers` reports its own crate list, and the two lists are textually identical | low; makes `check-layers` slower and redder |
| **G2** | Make the `check-layers` crate list self-reporting: echo each crate before testing | log names every crate covered | none |
| **G3** | Add `make check-tests` = `check-layers`, documented as **not** the canonical gate, and record in `AGENTS.md` why `check` runs a binary | `make check-tests` fails today, naming the 43 orphans | none; does not touch the canonical gate |
| **G4** | Branch the orphan ratchets onto the canonical gate **only after** the 43 are resolved | `make check` stays green throughout | deferred by owner decision |
| **G5** | Taxonomy: promote the noun for the 31 hyphenated `runtime` subcommands, reusing the existing `models` hierarchy | reflected clap tree has 0 hyphens at depth ≤ 2 | medium; 31 routing decisions |
| **G6** | Route `cynic deps`, `dev`, `bootstrap`, `lifecycle`, `fleet` from argv, then delete the corresponding `make` targets | reflected tree contains all five; `make deps` no longer needed | medium; depends on A0 in `WORKSPACE_ORPHAN_DECISIONS.md` |
| **G7** | Record the rustc ICE as a dated, owned condition in `AGENTS.md` with the runner as the known-good path | an operator reading `AGENTS.md` knows not to trust a local build | none |

G1-G3 and G7 are governance-only and touch no production code. G5 and G6 depend on the orphan
disposition and are not in this slice.

## 6. Explicit unknowns

- Whether `check` running a prebuilt binary is deliberate. It may be intended: the gate is a
  fast operator loop, and `check-layers` is the deep one. If so, the governance debt is not the
  gate but the **absence of any statement of which gate is authoritative for what**. Not
  determined. `unknown`.
- Whether the 26 top-level commands are the intended taxonomy or a stage of one. Nothing in
  `contracts/foundation.json` or `infra/runtime-authority.json` enumerates the CLI surface, so
  there is no authority to compare the tree against. `unknown`.
- Whether the 5 unroutable manifest domains are intentionally make-only. `make deps` works and
  is undocumented in `AGENTS.md`, which suggests accidental rather than deliberate, but that is
  inference. `unknown`.
- `cynic-mcp` is in `CORE_CRATES` but not `LAYER_CRATES`. Whether the omission is deliberate —
  the crate may not have an owned test target — is not recorded. `unknown`.
- The hyphen rule has no authority file. It is an owner instruction in this session only. Before
  G5 is implemented it should be written into `contracts/foundation.json`, because a taxonomy rule
  that lives only in chat cannot be enforced by a test. `unknown` whether the owner wants it
  constitutional.

## 7. Coordination liveness — the active-set lie (2026-09-28, observed + fixed)

- **Method**: store-level census of `.agent-claims.json` (240 entries: 143 `active`, 92
  `released`, 5 `expired`), TTL evaluation per entry, process table, fs mtimes,
  kernel reachability (`:8080` → 000, unreachable).
- **Finding 1**: 69 of 143 `active` claims were past TTL. Status display filtered
  on the status *string* only, so ghosts listed with `remaining 0` and inflated
  totals. Conflict path already filtered expiry (`local_claim_active`) — reads
  lied while writes told the truth.
- **Finding 2**: kernel-first/local-fallback dual-write (300 ms timeout) with no
  sync (`merge_queue` wired to nothing). A `released` ack from one side left the
  other stale. 13 own releases acked-but-unflipped this way.
- **Finding 3**: local claims had no heartbeat (no CLI subcommand, no TTL refresh
  on re-claim — re-claim returned the aging entry). Every TTL window orphaned
  entries; the active set could only grow (revision 367 and counting, no
  compaction, no reaping).
- **Fix applied** (`crates/cynic-cli/src/agent_surface/coordination.rs`): status
  display (both kernel and file paths) evaluates liveness; same-holder re-claim
  refreshes `started_at` (heartbeat-by-reclaim); claim success path compacts
  silently-expired entries (explicit `released` kept as audit trail); unit tests
  pin the liveness boundary (`local_claim_active`).
- **Residual**: kernel-path display trusts kernel expiry fields; `merge_queue`
  still unwired (no drain protocol); `released` entries compact never (audit
  trail by default — needs an explicit retention rule); claims on deleted paths
  accepted (explicit unknown, not refused).
- **Live-holder nuance**: long-TTL actives are not all stale — e.g. gov holds
  with heartbeated windows. Staleness is decided per entry by (status, TTL,
  holder process, fs recency), never by age alone.
