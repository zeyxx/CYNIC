# Crate Fusion Disposition Matrix — per-crate disposition with the global view

- **schema**: `cynic.audit.crate_fusion.v1`
- **status**: `decision_proposal`
- **method**: read-only observation plus one correction slice already landed separately. No crate was
  fused, split, or created in producing this. A single-file claim was taken to write this report.
- **authority_order applied**: observed runtime state > `contracts/foundation.json` >
  `infra/runtime-authority.json` > `infra/directory-tree.json` > `infra/registry.json` > workspace
  `Cargo.toml` > canonical crates > `infra/agent-exposure.json`
- **decision rule**: taken from `contracts/pruning-policy.json`, not invented here.
  `split_criteria.crate_justified` lists five conditions for a crate to earn its existence, and
  `suspect_over_splitting` states the inverse signal: *"A crate is too small, too tightly coupled to
  one consumer, and has no strong reason to live on its own"* → *"Candidate for fusion or return to a
  module, not yet another crate."*
- **companions**: `infra/WORKSPACE_ORPHAN_DECISIONS.md` owns unreachable `.rs` files inside crates.
  `infra/DIRECTORY_STRUCTURE_AUDIT.md` owns the filesystem arborescence. Neither covers whether a
  crate should exist at all.
- **not enforced yet**: the same gap as the LOC ratchet, which compiled but never ran. No test asserts
  a crate's consumer count. See §5.

## 1. The global view, which is the point of this document

The workspace holds **47 crates** for **388 747 LOC of `src`**. That is a mean of ~8 300 LOC per
crate. The distribution is not Gaussian — it is a heavy tail over a small, dense body:

| Band | Crates | Meaning |
|---|---|---|
| **0–7 LOC** | 1 | a crate that contains no code |
| **< 500 LOC** | 7 more | below any compilation, reuse, or cycle reason |
| **500–3 000 LOC** | ~19 | the healthy body, and where most work should live |
| **> 25 000 LOC** | 6 | constitutional violations under `contracts/foundation.json:74` |
| **> 50 000 LOC** | 2 of those 6 | `cynic-host` 89 851, `cynic-node` 58 327 |

Two failures are happening **simultaneously and in opposite directions**. The workspace is being
fragmented at the small end while four god-objects grow at the large end. That is the signature of
extraction performed as a ritual rather than as a consequence: a boundary is cut, a crate is
minted, the god-object does not shrink, and the new crate then needs its own god-object rule in a
few months. The two halves of the problem are the same problem.

The 6 god-objects are not listed here. `infra/CANONICAL_CRATE_MAP.md` v3 owns them, and 13 renames
plus 8 crate creations from its §3 remain unexecuted. What this document adds is the other end, and
the observation that **a fusion programme is the only ratchet that pushes god-objects down**: cutting
`cynic-host` requires moving responsibility out of it, and fusing `cynic-discovery` into
`cynic-discovery-host` is the same motion at 1/400th the cost.

## 2. Measured: direct workspace dependents

`grep -rl "^<crate> = " crates/*/Cargo.toml`, per crate. All ten are library-only — none has a
`main.rs`, so a missing dependent means missing use, not a binary that is itself the product.

| Crate | src LOC | test LOC | Direct dependents | Public items in `lib.rs` |
|---|---|---|---|---|
| `cynic-inference-runtime` | **6** | 0 | **0** | 0 |
| `cynic-domain-askesis` | 67 | 0 | **0** | 3 |
| `cynic-agent-bus-server` | 91 | 0 | 1 (`cynic-agent-bus-coordinator`) | 1 |
| `cynic-identity` | 185 | 53 | **0** | 0 |
| `cynic-intel-futarchy` | 213 | 0 | 1 (`cynic-cli`) | 5 |
| `cynic-discovery-host` | 220 | 0 | 1 (`cynic-cli`) | 3 |
| `cynic-channel` | 283 | 40 | **0** | 0 |
| `cynic-discovery` | 352 | 0 | 2 (`cynic-cli`, `cynic-discovery-host`) | 0 |
| `cynic-pool` | 744 | 46 | **0** | 0 |
| `cynic-host-probes` | 1 129 | 22 | **0** | 0 |

`cynic-askesis` is included as the control: 2 373 LOC, **3** dependents, 56 public items. It passes
condition 3 of `crate_justified` — a public API that is genuinely reused.

**Six of these ten have zero direct dependents.** Against `crate_justified` condition 3, "a stable
public API that is reused, or must be reused", zero reuse means the condition is not met.

## 3. Per-crate disposition

Each row is a proposal for the owner to sign, not a ruling. They are ordered by confidence.

### 3.1 Fuse — high confidence

**`cynic-inference-runtime` → delete, fold into `cynic-inference`.**
6 LOC, 0 public items, 0 dependents. `src/lib.rs` is six `//!` lines. Its `Cargo.toml` declares 14
dependencies (`tokio` full, `reqwest`, `anyhow`, `ulid`, `sha2`, `chrono`, `serde_json`) that are all
unused because the crate exposes nothing: 14 phantom edges in the build graph.
`CANONICAL_CRATE_MAP.md` itself labels it "composition shell (new in v3; stub, 6 LOC)".
Deleting is not a fusion — there is nothing to merge. It satisfies no condition at all.

**`cynic-domain-askesis` → fold into `cynic-askesis`.**
67 LOC, 3 public items, 0 dependents. It is one function, `archive_zone_backup`, which calls
`cynic-askesis::AskesisLog`. The parent is 2 373 LOC with 3 dependents. Zero shared public names.
Name also violates R3 (`-domain` is a banned layer prefix) — and the crate map marks three of the
four `-domain-` crates `constellation ✓` while R3 forbids the prefix, a self-contradiction to settle
at the same time.

**`cynic-channel` → fuse into its natural parent.** 283 LOC + 40 test LOC, 0 dependents, 0 public
items in `lib.rs`. `AGENTS.md` names it L2 and the map lists a rename to `cynic-agent-channel`; a
rename preserves a boundary that nothing crosses, which is the wrong move. Fusing removes the
question.

**`cynic-pool` → fuse.** 744 LOC + 46 test, 0 dependents, 0 public items. Also declares a `cli`
feature that is never conditioned anywhere in the workspace — a knob wired to nothing. The map
already votes `ORPHAN (rename: cynic-agent-pool)`; fusion supersedes the rename.

### 3.2 Fuse — needs one fact confirmed first

**`cynic-host-probes` → decide by publication intent.** 1 129 LOC, 0 dependents, 0 public items in
`lib.rs`, but a real 22-line test file and its own `version = "0.1.0"` separate from the workspace
`0.8.0` with a `description` field — the shape of a crate intended for external consumption. Commit
`2e6f7210` exists solely to make its probe functions `pub` "crate is a reusable probe library". So
it is either (a) a library awaiting a consumer, or (b) a workspace-internal module that was
promoted one layer too far. **Condition: is it published to a registry, and is there a consumer
outside this workspace?** If neither, fuse into `cynic-host`. If it is genuinely external, it is
legitimate and should be excluded from this matrix — but then the external consumer is the missing
evidence, and the matrix should say so.

**`cynic-identity` → fuse, once its API surface is read.** 185 LOC + 53 test, 0 dependents, 0 public
items at the `lib.rs` level. AGENTS.md classes it L0 pure, next to `cynic-protocol` and `cynic-types`.
A pure leaf with no consumer is a module. Note the asymmetry: `cynic-types` has 4 dependents and
earns its place; `cynic-identity` has 0 and does not. Same layer, opposite verdicts, decided by
consumption — which is exactly what the criterion says to measure.

### 3.3 Correct, do not fuse

**`cynic-intel-futarchy` → delete, not fuse. It is a clone.**
213 LOC, 1 dependent, 5 public names — **all 5 shared with `cynic-intel`**
(`LiquidityVault`, `create_market`, `execute_bet`, `resolve_market`, `calculate_prices_bps`).
`cpmm.rs` (138/138 LOC) and `vault.rs` (69/69) differ from their `cynic-intel/src/futarchy/`
counterparts by a single `use` line. The "extracted" crate also declares
`cynic-intel = { path = "../cynic-intel" }` — the extraction removed nothing. R7 forbids
micro-crate cascades. Deleting it and routing `cynic-cli` to `cynic-intel::futarchy` is the fix;
a fusion would merge a crate with itself.

**`cynic-agent-bus-server` → keep, rename in place.** 91 LOC, 1 public item, 1 dependent
(`cynic-agent-bus-coordinator`). Below the map's 5–15k band, so it looks like over-splitting, but it
is a thin axum `Router` projection — a real deployment shape, and `pruning-policy` excludes modules
whose reason is a distinct service requirement. A 3-way split
(`cynic-agent-bus` 908 / `-server` 113 / `-coordinator` 479) with **zero** shared public names is the
well-formed constellation in this workspace, not a defect. Lowest-priority row in this matrix.

**`cynic-discovery` + `cynic-discovery-host` → fuse into one, or keep both deliberately.**
352 + 220 LOC, 2 and 1 dependents, and `cynic-discovery-host` already depends on `cynic-discovery`.
The two are named by the map for renames to `cynic-node-discovery` and `cynic-host-discovery`,
i.e. the map treats them as belonging to *different* layers. That is the one place in this matrix
where the evidence is genuinely ambiguous rather than merely missing, and it is an owner's call:
either the layer distinction is real and both stay, or it is an artefact of an old split and they
merge. Do not rename first and decide later — the rename is what forecloses the question.

## 4. What this is not

- Not a ruling. Every row is a proposal with a stated condition. `WORKSPACE_ORPHAN_DECISIONS.md`
  exists because "16 crates carry orphaned source" is not one situation, and it is not one situation
  here either.
- Not the god-object programme. The 6 crates over the 25k ceiling are `CANONICAL_CRATE_MAP.md`'s.
  This document proposes the cheap half of the same motion.
- Not a licence to add crates. Every row here removes one. If a new crate appears while this matrix
  is open, the new one needs a `crate_justified` row before it lands.

## 5. The missing gate, stated plainly

**Nothing enforces any of this.** The LOC ratchet existed and never executed, because
`crate_loc_ratchet.rs` included the shared detector and used half of it, and `dead_code = "deny"`
failed the build silently. That was found and fixed in `6b0a5f0a`-era work; the lesson generalises.

There is no test that asserts a crate has a consumer, and no rule that a new crate must justify
itself. A fivable check is cheap: for each workspace member, assert it is either a binary, or has at
least one dependent, or appears on an explicit allow-list with a written reason — the same shape as
`GOD_OBJECTS` in `crate_loc_ratchet.rs`, with the same discipline that the list may only shrink.
Until that exists, this matrix is a document, and the fragmenting pressure has no counterweight.

## 6. Evidence

Observations in this document are reproducible:

```
grep -rl "^<crate> = " crates/*/Cargo.toml            # direct dependents
find crates/<crate>/src -name '*.rs' | xargs cat | wc -l
[ -f crates/<crate>/src/main.rs ]                      # library or binary
```

The six `/etc/cynic/registry.json` literals in `cynic-build` that an earlier pass flagged as
production hardcoding were re-checked and are all inside `#[cfg(test)]` modules — fixtures. Recorded
because it corrects a finding rather than adding one.
