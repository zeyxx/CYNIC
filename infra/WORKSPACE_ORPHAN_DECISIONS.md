# Workspace Orphan Decision Matrix — per-crate disposition

- **schema**: `cynic.audit.orphan_decisions.v1`
- **status**: `decision_proposal`
- **method**: read-only. No production file mutated. Companion to `infra/CLI_SURFACE_AUDIT.md` §6.2.
- **purpose**: 16 crates carry orphaned source. They are not one situation. This proposes a
  disposition per crate so the owner can sign each one off instead of ruling on a single
  workspace-wide policy.
- **authority_order applied**: observed runtime state > `contracts/foundation.json` > `infra/runtime-authority.json` > `infra/registry.json` > workspace `Cargo.toml` > canonical crates > `infra/agent-exposure.json`

## 1. What is measured

Same validated module-graph resolver as `CLI_SURFACE_AUDIT.md` §1, applied to all 47 workspace
members. `mod tests;` and `mod test;` are excluded by design, so `#[cfg(test)]` modules are not
production orphans.

| | files |
|---|---|
| `.rs` files across `crates/*/src` | 1 973 |
| reachable from their crate root | 1 581 |
| orphaned, total | 392 |
| **orphaned production code** | **390** |
| orphaned `#[cfg(test)]` modules | 33 (of a larger test population) |

The 390 production orphans split by how deep they sit, because depth decides the cost:
a **root-level** orphan is one file whose parent is the crate root, so a single `mod` line
recovers it. A nested orphan usually means the *parent* is orphaned, and one declaration
recovers the whole subtree.

**49 root-level orphans workspace-wide.** That is the true size of the wiring problem: 49
declarations, not 390 files.

## 2. Origin: one commit explains five crates, not sixteen

`1fa6ec3d "fix(workspace): restore clean verified baseline from 24c9769d"` (2026-09-26) is a
189-file commit with 7 532 deletions and 0 `.rs` files removed. It resolved committed merge
conflicts by keeping one side and deleting the other, stripping the markers with the code.

At `1fa6ec3d^`, **11 crate roots carried committed conflict markers**. Of those, 5 lost real
module declarations:

| crate | root | HEAD side | github/main side | present now | discarded side | declarations lost |
|---|---|---|---|---|---|---|
| `cynic-kernel` | `lib.rs` | 0 | 2 | 5 | github/main | `api`, `backends` |
| `cynic-mcp` | `lib.rs` | 1 | 4 | 1 | github/main | `local_tools`, `project_root`, `proxy`, `types` |
| `cynic-mcp` | `main.rs` | 0 | 2 | 0 | github/main | `kernel_discovery`, `project_root` |
| `cynic-askesis` | `lib.rs` | 3 | 10 | 3 | github/main | `agent_loop`, `anchor`, `audit`, `domains`, `error`, `eval`, `experiment`, `memory`, `reflection` |
| `cynic-cli` | `main.rs` | 0 | 45 | 0 | github/main | 45 declarations, 15 of them never in `lib.rs` |
| `cynic-host` | `lib.rs` | 43 | 17 | 118 | HEAD | `sandbox`, `autoresearch`, `bucket_signatures`, `build`, `build_proof_compressor`, `detection_engine` |

`cynic-node` and `cynic-observer` also carried markers but lost nothing — both sides' modules
are present today. The other 9 crates with orphans were not touched by this commit.

**The remaining 11 crates are a different problem** and are not covered by any finding here.

## 3. Per-crate disposition

`LOC` is orphaned production code. `root` is the count of root-level orphans, i.e. the number
of declarations that would recover the crate.

| # | crate | prod orphans | LOC | root | origin | proposal | cost / risk |
|---|---|---|---|---|---|---|---|
| 1 | `cynic-kernel` | 144 | 32 626 | 1 | `1fa6ec3d`, lost `api` + `backends` | **restore**, but not first | 2 declarations recover ~32 kLOC. `api/mcp/mod.rs:26` declares `mod observe_tools;` for a file that does not exist, so this crate does not currently build its test targets. Highest blast radius, so it needs its own slice and its own proof |
| 2 | `cynic-mcp` | 42 | 12 812 | 8 | `1fa6ec3d`, lost 6 declarations | **restore** | 4-6 declarations recover the entire agent-facing tool surface. `infra/agent-exposure.json` declares 109 tools against 9 servable. Restoring is what makes that number honest, in either direction |
| 3 | `cynic-cli` | 60 | 10 708 | 20 | `1fa6ec3d`, lost 45 in `main.rs` | **restore**, 17 known-good first | 15 declarations recover 5 domains. Attempted: 14 compile errors, all API drift. The list is derived from git, not invented. See §4 |
| 4 | `cynic-host` | 63 | 8 308 | 2 | `1fa6ec3d` lost 8 on the HEAD side | **owner decision** | 2 root-level orphans: `runtime_takeover_activation.rs` (260 LOC, last touched by the `cynic-node-protocol` extraction) and `signed_heartbeat.rs` (59 LOC). `AGENTS.md` puts this crate under a strict no-net-growth ratchet with a clean-room extraction policy, and `9e7ad3e7` deliberately deleted the `update_lifecycle` shim. Restoring here contradicts a recorded direction |
| 5 | `cynic-node` | 34 | 4 263 | 7 | not `1fa6ec3d` | **investigate** | 7 root-level orphans including `lifecycle.rs` and `fleet_placement.rs`. No origin identified. Do not restore blind |
| 6 | `cynic-inference` | 10 | 2 696 | 0 | not `1fa6ec3d` | **investigate** | 0 root-level: parents are orphaned, so one declaration each at the parent. `runtime-authority.json:193-201` already flags this crate `status: "extracting"`, `experimental_workspace_crates` |
| 7 | `cynic-askesis` | 19 | 2 070 | 3 | `1fa6ec3d`, lost 9 | **restore** | 3 root-level (`agent_loop.rs`, `error.rs`, `memory.rs`). Smallest of the five conflict casualties and the lowest-risk first restore |
| 8 | `cynic-protocol` | 11 | 1 481 | 7 | not `1fa6ec3d` | **investigate** | 7 root-level orphans including `build.rs` and `build_proof.rs`. `cynic-protocol` is the DAG root; restoring it changes what every other crate can depend on. Highest authority sensitivity |
| 9 | `cynic-observer` | 2 | 300 | 0 | not `1fa6ec3d` | **delete or wire** | 300 LOC, 2 files, no root-level orphan. Trivial either way |
| 10 | `cynic-zeroclaw-solana` | 1 | 148 | 0 | not `1fa6ec3d` | **delete or wire** | 148 LOC, 1 file |
| 11 | `cynic-vcs` | 1 | 131 | 0 | not `1fa6ec3d` | **delete or wire** | 131 LOC, 1 file |
| 12 | `cynic-lifecycle` | 1 | 118 | 1 | not `1fa6ec3d` | **owner decision** | `repair_loop.rs`. This is the crate `9e7ad3e7` extracted *to*. An orphan here may be a deliberately parked capability rather than an accident |
| 13 | `cynic-build` | 1 | 97 | 0 | not `1fa6ec3d` | **delete or wire** | 97 LOC, 1 file |
| 14 | `cynic-lxc` | 1 | 89 | 0 | not `1fa6ec3d` | **delete or wire** | 89 LOC, 1 file |
| 15 | `cynic-channel` | 0 prod | 55 | 0 | `#[cfg(test)]` only | **none** | 55 LOC is a test module. Nothing to decide |
| 16 | `cynic-intel`, `cynic-workload` | 0 prod | 360, 560 | 0 | `#[cfg(test)]` only | **none** | no production orphans |

### 3.1 Recommended sequence

Derived, not planned: each step's proof is the next step's input.

1. **`cynic-askesis`** first. 3 declarations, 2 070 LOC, lowest blast radius of the five
   conflict casualties, and it exercises the whole restore procedure on something small.
   Proof: the crate's tests compile and run.
2. **`cynic-mcp`**. 4-6 declarations, 12 812 LOC, and it is the only crate where a declared
   authority (`agent-exposure.json`, 109 tools) already disagrees with the compiled reality.
   Proof: `cynic_mcp_surface` reports a served count that matches the declaration, or the
   declaration is corrected to match.
3. **`cynic-cli`**, the 17 known-good declarations. The list is derived from `1fa6ec3d^`, and
   the 14 compile errors are enumerated in `CLI_SURFACE_AUDIT.md` §6.2.2.
4. **`cynic-kernel`** last of the restores. Largest, and the crate does not currently build
   its test targets because `api/mcp/mod.rs:26` declares a module whose file is absent — a
   second, independent defect that must be fixed in the same slice.
5. **The 6 single-file crates** in one cleanup slice, decision `delete` or `wire` recorded per
   file. 1 000 LOC total. Cheap, and it removes the long tail that makes the number look scary.
6. **`cynic-host` and `cynic-protocol` only after an explicit owner ruling.** Both are
   authority-bearing: one under a no-net-growth ratchet with a recorded extraction direction,
   the other the DAG root.

### 3.2 What no proposal here can decide

Restoring an orphan is only correct if the capability it implements is still wanted. The
repository does not record that for any of the 390. `1fa6ec3d` deleted 7 532 lines in the name
of a "clean verified baseline" without recording which deletions were intentional, so for
every orphan the honest status is `unknown` until an owner rules. This matrix proposes
**where to start**, not **that restoration is right**.

## 4. The `cynic-cli` restore, measured

Recorded here so the owner can judge the cost without re-running it.

Adding the 15 `main.rs`-only declarations to `lib.rs` plus `mod build;` and `mod epoch;` in
`runtime_surface.rs` moves `cynic-cli` from **132 to 163** compiled files and from **62 to 31**
orphans, then produces **14 errors**, all API drift in code that predates the current surfaces:

| error | sites | nature |
|---|---|---|
| `cynic_host::update_lifecycle` unresolved | 6 | shim deleted on purpose by `9e7ad3e7` during the `cynic-lifecycle` clean-room extraction |
| `cynic_protocol::admit_build_input` unresolved | 1 | exists at `cynic-protocol/src/build.rs`, not re-exported at the expected path |
| `framework_bootstrap::{FrameworkRole, build_framework_enrollment_request}` unresolved | 2 | both exist at `cynic-host/src/framework_bootstrap/enrollment.rs` |
| `BuildRequest` missing `install_path`, `runner_admission`, `runner_envelope` | 1 | type gained fields |
| arity mismatch against `cynic-build/src/runner.rs:44` | 1 | function gained parameters |
| type mismatches in `surface_audit/checks.rs` | 2 | — |
| `SurfaceMutationPolicy` needs `Display` | 1 | — |

10 of the 14 are import-path repairs against APIs that **exist**. The edit was reverted and the
shared worktree verified back to `25 passed, 9 failed`; nothing from this attempt is committed.

## 5. Explicit unknowns

- The 11 crates whose orphans are **not** explained by `1fa6ec3d` have no identified origin.
  `cynic-node` at 34 orphans and `cynic-protocol` at 11 are the two largest. `unknown`.
- Whether each of the 390 orphans is a wanted capability or deliberate deletion is not recorded
  anywhere in the repository. `unknown`, and not derivable from git.
- `cynic-kernel`'s `api/mcp/mod.rs:26` declares `mod observe_tools;` for an absent file. This is
  a *dangling* declaration, the opposite of an orphan, and it is not caused by `1fa6ec3d`.
  Not investigated beyond locating it.
- The resolver is text-based. A `#[cfg]`-gated `mod`, or a module reachable only through a
  macro, would be misclassified. None was found; absence was not proven.
- Whether the 32 626 orphaned LOC in `cynic-kernel` even compile against the current
  `cynic-kernel` dependencies is untested. Assuming recoverability is an assumption, not a
  finding — this caveat applies to every restore in §3.
