# Build & gate economics — what a proof costs, and what it should cost

**Status:** measured on builder-119 (8 GiB RAM, 4 cores, 177 GB disk)
on 2026-10-21, while landing PR #226, #227 and #229.

This document answers one question: **what work is a merge actually
waiting for, and how much of it is necessary?** The gate exists to
prove a claim about a commit. Everything in the gate that does not
contribute to that proof is latency paid for certainty that was
already established.

---

## 1. The measured cost

| Surface | Measured | Notes |
|---|---|---|
| `cargo check --workspace` (cold target) | 2m40s | the full dependency graph |
| `cargo build --release -p cynic-cli` | 5m12s – 8m49s | the release profile of the CLI |
| `cargo test -p cynic-host --lib -- admission` | 194s | one test binary, 15 tests |
| `cargo test -p cynic-host --lib` (whole crate) | OOM / SIGKILL | 800+ tests, one process |
| A full F-gate run | 8 jobs, one at a time | the runner executes jobs SEQUENTIALLY |

**The runner executes one job at a time.** The gate is *sharded into
eight jobs* and the runner runs them one after the other. The
sharding bought nothing on this runner: it bought eight separate
checkout-and-compile phases, each paying its own cold-start, against
a machine that cannot run them in parallel anyway (8 GiB, 4 cores).

## 2. The three costs that are NOT the proof

### 2.1 The gate does not use the compilation cache

`RUSTC_WRAPPER` is **not set anywhere in the gate workflow**. sccache
is wired into the CYNIC build scheduler
(`crates/cynic-node/src/ports/build_runner_adapter.rs`) and into the
builder's login profile — but the Forgejo gate does not go through
that scheduler. Measured sccache counters on the builder:

```
Compile requests           3556
Cache hits (Rust)           129      ← 4% of Rust compiles
Cache misses (Rust)        1521
```

**97% of Rust compilations miss.** The cache exists, is bounded, and
is not consulted by the surface that compiles most.

### 2.2 Four target directories, each its own fingerprint space

```
6.5G  /var/lib/cynic/build-targets/fgate     ← the gate
2.7G  /var/lib/cynic/build-targets/debug
2.5G  /var/lib/cynic/build-targets/default
1.3G  /var/lib/cynic/build-targets/aegis-consortium
311M  /var/lib/cynic/build-targets/p4
```

A crate compiled in one of these is NOT reused in another. The gate
shares `fgate` between its eight jobs, which is right, but the
`debug` / `default` / `p4` directories are separate universes, and
`aegis-consortium` is a **branch that was never merged** still holding
1.3 GB of build output.

### 2.3 Three checkouts, and one of them is dirty

```
/home/user/CYNIC                                  9d2f3cf0   ← the builder's own checkout
/tmp/retire-clone                                 f60817bd   ← my working clone
/var/lib/cynic/source/repositories/CYNIC          701ae9cd   ← the node's source store
```

The node's source store is **dirty** (4 modified files, 4 untracked
directories) and stale. That is why `cynic build submit` fails with
`source_sync_checkout_failed`: `git checkout --detach <rev>` refuses
to move a dirty worktree. **The native build surface is blocked by a
dirty clone on the builder** — which is also why every proof in this
period had to be driven by hand over SSH.

## 3. What a proof actually requires

A merge needs to establish, for the **exact head**:

1. the workspace compiles;
2. the crates that the change touches have their tests passing;
3. the contracts and schemas the change touches are coherent.

It does **not** need to re-run the test suites of crates the change
does not touch, and it does not need eight jobs.

### 3.1 The change-graph rule (not yet formalised — this is the proposal)

**The set of crates to test is derived from the diff, not fixed.**

```
changed crates  =  crates whose sources differ between base and head
                  ∪  crates that depend on a changed crate
                  ∪  crates the changed crates declare as contract-bearing
```

For the changes in this period the derived set was **1 to 3 crates**,
against a fixed set of **5 test jobs** and a full `cargo check
--workspace` on top. The four shards that did not touch the changed
crate were re-running their tests to produce a green checkmark about
code that did not change.

**The invariant:** a shard exists because a changed crate needs a
proof, not because the workflow once had that shard. A shard whose
diff is empty against the base must not run — it is a copy of the
previous run wearing a new head.

### 3.2 The base-reuse rule

The gate already has a base run to compare against. A test that passed
on the base and whose subject did not change **does not need to pass
again** — it is already proven, by the base run, for the same code.
The gate re-runs it anyway, because the shard list is static.

## 4. The concepts that were not formalised

These are the gaps this period exposed. Each is a *concept* that
exists implicitly in the code and in practice, and is not written
down anywhere.

### 4.1 Proof scope — "what changed" versus "what is re-proven"

A gate proves **a proposition about a commit**. Today the workflow
re-proves a proposition about **the whole workspace**. The scope of
the proof is implicit in a YAML file, and the YAML file was written
once and never revised against the shape of the actual changes.

**Rule to formalise:** the proof scope is a function of the diff, and
the diff is the input, not a constant list.

### 4.2 Evidence freshness — "does this proof speak about THIS head?"

This period produced a concrete failure of exactly this. The gate
published a compilation parallelism computed by a binary that predated
the source that the decision was about. The mechanism worked; it
answered the wrong question. The fix was to bind the decision to the
digest of the source it was taken for (`source_digest`), and the
consumer now refuses a decision published for another commit.

**Rule to formalise:** every artifact that another artifact depends on
carries the identity of what it was derived from, and the consumer
refuses a mismatch. This is the same rule as the retention fence, the
capability digest admission, and the release signature — it has never
been stated once for the whole system.

### 4.3 Cache validity — "may this build artifact be reused?"

Four target directories and a dirty source store mean the answer
differs per artifact. Today it is answered implicitly by mtimes, and
mtime-based invalidation has already produced one wrong answer
(measured: a shared target directory served a stale binary).

**Rule to formalise:** a build cache is valid for a source tree whose
identity it records. A cache that cannot name its source is not
reusable, and the reuse is a correctness bug, not a performance one.

### 4.4 Capacity as a function of the machine — "how much work can this host hold?"

Formalised this period as `cynic build pool memory --json`: the
parallelism is the smaller of the memory bound and the core count,
published as a decision, consumed by the shards, and refused when it
does not match the head. The concept is now in the code.

**Rule to formalise:** a concurrency decision is a published artifact
with a producer, a consumer, and an identity — never a constant in a
workflow file.

### 4.5 The cost of a proof — "is this run worth its wall time?"

A gate that takes 40 minutes and 8 jobs to prove that one crate still
compiles is over-proving. The *value* of a proof is not uniform: a
contract change deserves a workspace-wide check, a one-line doc change
deserves almost nothing. The workflow has no notion of this.

**Rule to formalise:** the proof scope, the shard count and the
parallelism are derived from the change AND from the cost budget of
the host. Not constants.

## 5. The proposal, in order

1. **Bind `RUSTC_WRAPPER=sccache` in the gate.** One line, and it
   converts 97% cache misses into hits. Measurable immediately in
   `sccache -s`.
2. **Derive the test shards from the diff.** A shard that has no
   changed subject does not run. This is §3.1.
3. **Reuse the base run for unchanged subjects.** A test that passed
   on the base for identical code is already proven.
4. **Make the source store clean and the checkout canonical.** One
   checkout, one source of truth, so `cynic build submit` works and
   the SSH workaround is not needed.
5. **Retire the dead target directories** (`aegis-consortium` for an
   unmerged branch) through the existing native retention command.
6. **Write §4 as normative rules** so the concepts stop being
   implicit in YAML and start being checked.

Items 1 to 3 are the ones that change the wall time. Items 4 to 6
remove the workarounds and make the concepts enforceable.
