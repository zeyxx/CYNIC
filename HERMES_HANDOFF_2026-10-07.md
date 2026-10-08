# Coordination handoff — `hermes` session, CYNIC

**Captured:** 2026-10-07T12:36:36+0200
**HEAD at capture:** `daa24f4c`
**Worktree:** `/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC` (shared, multi-agent)
**Branch:** `agent/hermes/scan-exclusions`
**Author:** Hermes Agent — commits authored `hermes` (builder identity) or `jeanterre552`

---

## 1. What is DONE and PROVEN

### 1.1 Scan-exclusion registry (committed, proven on builder)

**Commit `bca12c14`** — present in HEAD. The same work exists on the remote as
`739031e5` + `92bb2a7b`; `bca12c14` is the amended descendant, so **the two remote
SHAs are stale duplicates, not lost work.** Do not cherry-pick them.

| File | Change |
|---|---|
`infra/scan-exclusions.json` | **new** — `cynic.infra.scan_exclusions.v1`, keyed per scanner (`secret_lint`, `disk_scan`, `space_gc`) |
`crates/cynic-release/src/adapters/secret_lint/mod.rs` | reads the registry; compiled-in fallback mirrors the registry `fallback` block |
`Makefile` | 7 inlined `if [ -x ... ] else cargo run` ladders consolidated into one `source_bin` macro; `release.deploy` now also builds `cynic-mcp` and `cynic-kernel` |
`.gitignore` | `.venv_data/` |

**Proof — builder `10.10.10.119`, exit 0:**
- baseline `a578b5b`: `Finished in 46.05s`, `1 passed` (no exclusion test existed)
- `92bb2a7b`: `Finished in 39.37s`, `7 passed; 0 failed`

**Root cause it fixes:** `secret_lint` walked the tree with `WalkDir` and skipped only
`.git, target, .cargo-home, node_modules, vendor`. No Python cache, so a 1.3 GB
`.venv_data/` reached the scanner and failed the release gate on a third-party
`verify_token=` literal. The old filter also compared whole path components
(`Path::starts_with`), so nested `crates/<x>/target/` was scanned in full.

---

## 2. What is WRITTEN but UNCOMMITTED — do not lose this

### 2.1 Argv execution port moved to L0

**Claim held:** `crates/cynic-protocol/src/argv` (lease `claim_hermes_1791368745`).

| File | State |
|---|---|
`crates/cynic-protocol/src/argv.rs` | **new, untracked, 158 lines** |
`crates/cynic-protocol/src/lib.rs` | `+1` (`pub mod argv;`) |
`crates/cynic-lifecycle/src/port.rs` | `11+/38−` — definitions removed, `pub use cynic_protocol::argv::{ArgvOutput, ArgvRunner};` |
`crates/cynic-vcs/src/ports.rs` | `49−` — same, `VcsRunner`/`VcsCommandOutput` deliberately NOT merged |

**Why:** `ArgvRunner`/`ArgvOutput` were declared **twice**, with diverging contracts:
- `cynic-lifecycle` (L1): `Serialize + Deserialize`, `MapArgvRunner` available in production
- `cynic-vcs` (L2): `PartialEq + Eq` only, `MapArgvRunner` behind `#[cfg(test)]`

An L2 crate declaring a type an L1 crate owns is a DAG inversion, forbidden by
`AGENTS.md`. The port now sits in `cynic-protocol` beside `argv_allow`, which already
held the admission policy — a port must not sit above its own policy.

`ArgvOutput` is the **union**: `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`.
Adapters (`LocalArgvRunner`, `MapArgvRunner`) stay in `cynic-lifecycle` because they
depend on `cynic-build`, which an L0 module must not.

**NOT PROVEN — no build was ever run on this.** `argv.rs` has 4 unit tests
(`success()` semantics incl. the failed-spawn trap, `summary()` precedence, JSON
round-trip, `not_attempted()`) but **they have never been compiled.**

**Blast radius if it does not compile:** 18 `impl ArgvRunner for` across 4 crates —
`cynic-host` 12 (8 prod, 10 test), `cynic-lifecycle` 3, `cynic-node` 2, `cynic-vcs` 1.
125 `ArgvOutput {` constructions workspace-wide; 55 `lifecycle::ArgvOutput` imports.
Re-exports mean no call site needs editing, but every crate must still typecheck.

---

## 3. Coordination hazards — the reason this report exists

### 3.1 One agent identity, many processes

| Fact | Value |
|---|---|
Distinct agent identities in the last 40 commits | **1** (`hermes` = 32, `jeanterre552` = 8) |
Active claims in `.agent-claims.json` | **0** (84 claims, all `status: released`, 0 `lease_id`) |
| Uncommitted tracked files | **89** (90 within minutes of capture — the tree is actively moving) |

`AGENTS.md` declares `one_agent_one_worktree` and
`agent_branch_template: "agent/<agent_id>/<scope>"`. **Neither is enforced.** There is
no `core.hooksPath` and no `pre-commit` hook.

The claims store *works* — `cynic agent claim` returns `fencing_token: 267` — but no
claim was ever held on a branch, so nothing was ever blocked. An observation I made
while working: a commit from another session (`docs(secrets)`, `e94a7313`) landed on
my branch, and my own amend changed SHA and author identity (`bca12c14` /
`jeanterre552`). Branch-level admission is absent.

### 3.2 Branch moved under me, three times

While this session ran, HEAD on `agent/hermes/scan-exclusions` moved:
`bca12c14` → `0aaf5ee7` → `890674a8` → `7cab3606` → `fe760cbf` → `daa24f4c`.
The 89 dirty files belong to several concurrent writers, not to one agent.

**Do not `reset --hard`, `stash`, `checkout`, or `add -A` in this worktree.**
`git add -A` is forbidden by `AGENTS.md`; 89 files include other agents' work.

### 3.3 Builder has no workspace — the "build SSOT" is not implemented

`infra/agent-source-protocol.json:12` calls `10.10.10.119` the builder that "only
mirrors the SSOT". **No code feeds it.** Measured: no `Cargo.toml` anywhere on the
builder; neither the `Makefile` nor `cynic-build` performs a remote build. There is no
`rsync`, no clone step.

Working procedure (not enforced by any gate), verified this session:
```
git clone --depth 1 --branch <branch> \
  ssh://forgejo@forgejo.synapse.cynic:22/jeanterre13/CYNIC.git
cd <clone> && RUSTFLAGS="-C debuginfo=0" RUST_MIN_STACK=8589934592 \
  CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_NETWORK_MODE=offline \
  cargo test -p <crate> --lib
```
Builder: 8 GB RAM (~6 available), 48 GB free, cargo 1.95.0. Forgejo is reachable from it.

---

## 4. Measured findings not yet acted on

### 4.1 The 89 uncommitted files are real work, not formatting

| Measure | Value |
|---|---|
Tracked files modified | 74 |
**Pure reformat** (`git diff -w` empty) | **1** |
Carrying a real change | **73** |
Raw delta | +984 / −269 |
Real delta | +975 / −260 |

Concentration: `cynic-host` 45, `cynic-node` 8, `cynic-cli` 6. Largest single file
`crates/cynic-host/src/audit_engine/path_rules.rs` (+542 raw, **+539 real**) — a new
audit rule, plus a glob-recursion fix in `cynic-cli/tests/publication_contract.rs`
(`"".split('/')` yields `[""]`, so the matcher never reached its base case).

**This is unfinished engineering, not noise.** It is unowned — every claim is
`released`.

### 4.2 `cynic host` is 88 071 LOC against a 25 000 ceiling

`AGENTS.md`: god-object ceiling 25 000 LOC, `contracts/foundation.json` hard boundary.
`cynic-host` is **3.5x over**. Distribution: `adapters` 9 823, `domain` 6 290,
`framework_bootstrap` 8 410, `audit_engine` 5 104.

Correction to a plausible-but-wrong reading: `domain/` is **not** the problem.
It is only 6 290 LOC across 11 families, has **0 pure-core IO violations**, and only
**1** provable duplicate (`domain/storage` 511 LOC vs `cynic-storage`).

`framework_bootstrap/` is the real extraction target — 13 modules, 8 410 LOC.
Measured symbol-level edges: **11 of 13 have no internal dependency at all.**
`lifecycle` (997), `build_runner` (662, needs only `crate::clock`), `build_runner_route`
(167), `source_snapshot` (755), `epoch` (172) are each independently extractable.
`cynic-lifecycle` is imported by 4 of them for `ArgvRunner`/`MapArgvRunner`, which is
why §2.1 must land first.

### 4.3 `QueuePolicy` is a phantom authority

`cynic-protocol::QueuePolicy` (L0, `queue.rs`, 91 lines) has **0 implementations**.
`cynic queue` works end-to-end — `root_dispatch.rs:100` → `queue_surface.rs:17` →
`LocalQueueAdapter.observe()` → `QueuePlan`, with `--apply` executing — but the policy
escaped into a host adapter (`cynic-host/src/adapters/local_queue.rs`) as
`unwrap_or_else` defaults. **An observation failure yields an empty graph and SUCCESS**,
which is a fail-open, and it is why the system is invisible: the queue has four homes
(`cynic-protocol/queue.rs`, `cynic-host/local_queue.rs`, `cynic-cli/queue_surface.rs`,
`cynic-build/build_queue.rs`) and `build_queue` is FIFO by timestamp, not data-centric.

`QueueItem` is also hard-bound to one domain: `branch: String`, `recipe_id: String`, and
a closed `QueueReason` enum whose 5 variants (`OrphanReclaimable`, `AgedReclaimable`,
`BranchWithoutUpstream`, `StaleBranch`, `UpstreamDivergent`) are meaningless outside
branch GC. Extension to other domains requires opening that enum and the item type.

### 4.4 The argv "central execution point" is 32% bypassed

450 `ArgvRunner` references across 8 crates, but **40 direct `Command::new(` calls in
19 files of `cynic-host`** bypass it. The centralisation is paid for but not achieved.

### 4.5 Build-gate path is off-SSOT

`Makefile:9` `CYNIC_CARGO_TARGET_ROOT ?= /tmp/cynic-cargo-targets` — a hardcoded `/tmp`
path. `infra/directory-tree.json` places `build-targets/` under the user share layer
and `cache` at `/var/cache/cynic`; `/tmp` is in no layer and does not survive reboot.
Also `CYNIC_SOURCE_BIN ?= $(CARGO_TARGET_DIR)/debug/cynic` hardcodes `debug` while
`RELEASE_ENV` builds with `-C debuginfo=0`.

*(Correction: I initially reported this as a fail-open onto the installed binary. That
was wrong — the ladder falls back to `cargo run`, not to `/usr/local/bin/cynic`. The real
defect was 7× duplication, now consolidated.)*

### 4.6 `cynic source release` fails on 3 gates

- `git_clean` — 89 dirty files (§4.1)
- `secret_lint` — the `.venv_data` false positive (§1.1 fixes it; **the fix is
  committed but the installed binary at `/usr/local/bin/cynic` dates from 2026-09-28**,
  so running that binary measures stale code, not this tree)
- `cargo_check` — not detailed by the gate

---

## 5. Recommended order for the converging agent

1. **Do not lose §2.1.** Commit those 4 paths explicitly, or they are one `reset --hard`
   from gone:
   `crates/cynic-protocol/src/argv.rs`, `crates/cynic-protocol/src/lib.rs`,
   `crates/cynic-lifecycle/src/port.rs`, `crates/cynic-vcs/src/ports.rs`.
2. **Prove §2.1 on the builder** (clone → `cargo test -p cynic-protocol --lib argv`),
   then `cargo check -p cynic-host --lib` to cover the 12 impls.
3. **Do not cherry-pick `739031e5`/`92bb2a7b`** — superseded by `bca12c14`.
4. **Triage the 89 files by tranche**, not as a block. `cynic-host` 45 files is its own
   tranche; start with the two standalone fixes (the glob recursion, `path_rules.rs`).
5. **Then** enforce branch admission + `core.hooksPath` — §3.1 is why three of my four
   measurements had to be redone from scratch.
6. `state.db` remains deferred by operator decision. Not touched.

---

## 6. Honesty notes

- Three of my own intermediate conclusions were wrong and corrected in-session:
  the fail-open attribution (§4.5), "most of the 89 files are reformatting" (§4.1),
  and "my proven commits were lost" (they are in HEAD as `bca12c14`).
- My first two CLI-surface censuses produced `real=2` then `real=10` from regex
  heuristics. Both were artefacts; **no CLI-surface health number is reported here**,
  because I have no sound method yet. `Update` is the only top-level command with no
  executor file (27/28 have one, all 28 route).
- One recursive-scan probe hit the tool timeout and was blocked awaiting consent. It was
  not retried and no result from it is claimed.