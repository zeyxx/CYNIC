# /goal — solder the debt, by causal order, against fragmentation

**Status:** SPECIFIED, NOT STARTED. No work authorised by this document alone.
**Schema:** `cynic.goal.debt_settlement.v1`
**Authority:** this file is a *directive*, not a contract. It changes no gate,
no ledger and no threshold. Each tranche it names must still prove itself
through `make check` and `make audit`.

---

## 0. The problem this goal is solving

The workspace does not lack findings. It lacks an **order**. Three symptoms,
all measured on 2026-10-05, none of them hypothetical:

| symptom | measured |
|---|---|
| Findings exist and cannot be ordered | `make audit`: **blocked, 8 critical**, all `LEDGER-001-UNDECLARED-DEBT` |
| The same fact is declared in several places | `infra/rust-toolchain.json` conflicts with `rust-toolchain.toml` on **3 crates**, declares a **deleted crate**, and its `"*"` default is implemented by no build path |
| Consolidating one thing fragments another | 12 binaries for one operator surface; `cynic-audit` and `cynic audit all` called the same function with **different envelopes** |

`d3868a58` is the proof that the order matters. The gate reported `ok` on
**20 critical violations** because it validated a stale install (`9cfa033e`).
The moment the gate became trustworthy, twenty findings appeared that nobody
had to go looking for. A gate that cannot see is not a gate, and no ordering
of debt is possible while it is blind.

---

## 1. Operating principle: settle by causal order, not by count

**Debt that creates cognitive debt or duplicates a truth comes first.** A debt
that is merely large comes after.

Concretely, the ratchet must be ordered by this question: *does fixing this one
unblock or un-deceive something else?* If yes, it precedes. If it only reduces a
number, it follows.

This inverts the usual triage. The 92 356-LOC `cynic-host` is **not** first
because it is large; it is not even second. It is first only because it is the
crate whose modules produce the module inversions, the uncompiled-source count
and the hotspot count — one extraction there moves three measured numbers at
once. After the inversions it creates are paid, it stops being worth touching
until a gate says otherwise.

## 2. Operating principle: a baseline is not a gate

Three sensors in this repository do not decide. They were all found by running
them, not by reading them:

| sensor | assertion | what it actually does |
|---|---|---|
| `the_compiler_has_one_pinned_version_every_consumer_agrees_on` | `assert!(n <= 3)` | validates **known** debt. Its name promises convergence; its body tolerates three divergences, one of which is a floating CI toolchain |
| `no_source_is_sitting_on_disk_uncompiled` | `assert_eq!(402, 557)` | **FAILS because the debt shrank.** 155 LOC were removed by deleting `cynic-observer`; the baseline was never refreshed |
| `toolchain_divergences`'s `read()` | `fs::read_to_string(..).ok()` | a source file that has **disappeared** reads as "no conflict found" |

**Rule: a gate either decides or it does not get called a gate.** Every
ratchet that will be touched in this goal must first answer: what is the
observed value, what is the declared value, and who updates the declared one
when the observed value improves? A baseline that punishes the agent who pays
debt is worse than no baseline, because it stops the debt from being paid.

`no_source_is_sitting_on_disk_uncompiled` is **failing right now** and is the
first thing to fix: the debt went down and the workspace is blocked for it.

## 3. Rule for every tranche in this goal

1. **Name the causal chain before touching anything.** If the reason this slice
   exists is not "X duplicates Y", stop — that is a cleanup, not a settlement.
2. **Measure first.** Every number in the commit message must come from a
   command that ran. A number recalled from an earlier session is a guess.
3. **Never make the gate green by absorbing new debt silently.** Declaring a
   violation in a ledger is legitimate; declaring it in the same commit that
   created it is not.
4. **One isolated commit per slice, explicit paths staged, never `git add -A`.**
5. **Update the ledgers in the same commit that moves the code.** `d3868a58`
   exists because a slice moved files and did not update the ledgers that
   record them; that single omission cost 17 violations.

---

## 4. The settlement order

Each stage states what it is for, what it is measured to be now, and what
"done" means. Stages are ordered causally; do not reorder them for convenience.

### Stage 1 — make the sensors decide (blocks everything)

*Why first:* every later stage is scored against these numbers. One of them is
currently failing, and one of them cannot fail.

- Refresh `BASELINE_UNCOMPILED_LOC` from 557 to the observed 402, **and**
  convert the assertion from `assert_eq!` to a directional one so the next
  reduction is not a regression.
- Make `read()` in `divergence_audit.rs` report a missing source as a finding.
- Give `the_compiler_has_one_pinned_version_every_consumer_agrees_on` a target
  of zero and enumerate the three accepted divergences in the ledger rather
  than in a bare integer.
- Add the missing comparison: each per-crate pin against the registry. The
  `cynic-inference` / `cynic-node` / `cynic-protocol` conflicts are invisible
  to the current test because it compares the *set* of declared values to the
  root pin, not each crate to its own declaration.

**Done when:** the suite runs green on a tree with *more* debt than it has now,
and fails when a source file is deleted.

### Stage 2 — one toolchain answer

*Why now:* Stage 1 instruments it, so the fix becomes visible.

Measured: rustup resolves from the **current directory**, not from `-p <crate>`.
The `Makefile` always builds from the workspace root, therefore **the six
per-crate `rust-toolchain.toml` files apply to no build in this repository**.
42 of 48 crates inherit the root pin. The registry asserts
`cynic-host` needs `1.96.0` because `1.95.0` SIGSEGVs on proc-macro2 — if that
were true the `Makefile` would crash on every build. It does not. So either the
reason is stale or it was never observed on this path.

**Done when:** one pin is authoritative, the registry is either derived from it
or deleted, the `"*"` default describes a rule some path actually executes, and
the dead crate entry is gone. Registers are shrink-only: deleting is the only
legal direction.

### Stage 3 — extract `audit_engine` (the one extract that pays three ways)

*Why now:* it is autonomous, and it is the reason Stage 4 can measure anything.

Measured: 3 964 LOC, **1** external consumer (`cynic-cli`, 2 call sites),
**0** external dependencies beyond `cynic-protocol`. `cynic-host` declares 20
`cynic-*` dependencies of which `audit_engine` uses **1**. `make audit` links
92 356 LOC of host substrate to produce a report that uses 3 964.

Known consequences, measured, not assumed:
- **7 integration tests** in `cynic-host/tests/` import `cynic_host::audit_engine`.
  They must move or take a dev-dependency. A dev-dependency from the substrate
  onto its own auditor is an inversion and should be refused.
- `infra/runtime-basket.json` declares `"crate": "cynic-host"` for
  `cynic-audit` and `cynic-topology`. No rule reads that field, but it becomes
  false and must be updated in the same commit.
- `cynic-cortex` added `prose_citation_rule.rs` **inside this directory** in
  `196bc4ac`. The extraction relocates that agent's file. That is a
  cross-agent consequence and it is announced, not done quietly.

**Done when:** the crate exists, `cynic-cli` depends on it instead of on
`cynic-host` for audit, all 7 tests compile against it, the basket is truthful,
and `cynic-topology --write` has been run (the crate count changes).

### Stage 4 — settle the registry and directory fragmentation

*Why now:* the SSOT work in Stage 3 must not land on a structure that is already
duplicated.

`docs/design/directory-fragmentation-diagnosis.md` already measured this:
- **19** `infra/verticals/*/intent.json` with **0** Rust readers
- **24** top-level directories, **6** with zero code readers
- no declared precedence in the config layer

Separately, and worse: the runtime plane vocabulary is declared **twice and
disagreeing**. `registry.planes` is `{service, admin, mesh}` while
`registry.nodes[].network` is `{synapse, management, local}` — **empty
intersection**. 38 `.or_else()` sites guess which one they are reading; 171
literals hardcode the node vocabulary; 203 private IPs are hardcoded in
production.

`cynic-protocol/src/plane.rs` already defines `Plane::{Service, Admin, Mesh}`
with **zero consumers** and is not exported from `exports.rs`. It is the obvious
SSOT for this conflict and it exists.

**Done when:** one vocabulary is authoritative and typed, the dead duplicates are
deleted rather than reconciled, and the IP count is a measured ratchet that
falls.

### Stage 5 — settle the hotspot, by cascade

*Why last:* it is the only debt here that is *created by our own correct work*,
and its fix depends on the network session that owns it.

`crates/cynic-cli/src/node_surface/network_surfaces_uc.rs` is **1 417 lines**
against a 400-line hard threshold (`rules.rs:163`). It grew by 462 when the
`NetworkCommands` enum moved into it. It is **not** in the ledger, deliberately:
declaring it would turn the gate green by absorbing new debt silently.

It is two objects welded together: a 461-line enum of 29 variants (median 16
lines each, mostly doc-comments and `#[arg]`) and an 809-line dispatch whose
median arm is **5 lines**, where only **2** arms exceed 40.

The full spec, including the three things not to do and the measurement that
forbids each, is `docs/design/network-surface-extraction-handoff.md`. That file
is owned by the network session. **This goal does not implement it.**

---

## 5. Explicitly out of scope

- **`cynic-topology` stays a separate binary.** It writes
  `infra/crate-topology.json`; `cynic audit` must never write. A reader that can
  also write cannot be trusted to only read. This is a constraint, not
  fragmentation. Its docstring justifies itself against the audit surface now
  that `cynic-audit` is gone.
- **`cynic-node` daemon surfaces.** Owned by another session.
- **`cynic-inference` (25 971 LOC) and `cynic-kernel` (54 774 LOC).** Large, but
  they create no measured inversion and no duplicate truth, so by §1 they are
  not next.
- **`the_compiler_has_one_pinned_version` reaching zero.** Only after Stage 2
  makes convergence possible. Recording it now would be a false claim.

## 6. The known fail-open, recorded not fixed

`rust_source::workspace_root()` returns the sentinel
`".cynic-source-root-unobserved"` when it cannot find a root, and
`scan_workspace` treats an unreadable directory as empty (`scanner.rs:82`,
`let Ok(entries) = fs::read_dir(&current) else { continue }`). **A missing root
therefore produces an empty report, which reads as clean.** This is a real
fail-open on the canonical gate.

It was deliberately not folded into `02a4895c` (the `cynic-audit` fusion):
mixing a behavioural fix into a deletion makes the diff unreviewable. It needs
its own slice, with a regression test proving that a missing root **fails**.

Related: two environment variables name the same thing. `CYNIC_SOURCE_ROOT` is
read by 1 site (all in `cynic-cli`); `CYNIC_WORKSPACE_ROOT` is read by 7 (all in
`cynic-host`, **0** on the canonical path).

## 7. Baseline — measured 2026-10-05, on `agent/hermes/knowledge-canon`

This is what "no progress" looks like, so that progress is measurable.

```
crates                48
rust LOC           335 889
binaries             11   (the [[bin]] census ART-001 actually reads)
  delivered            5   cynic, cynic-node, cynic-kernel, cynic-mcp,
                          cynic-lxc-exec-bridge
  tooling              5   cynic-node-enroll, cynic-network-actuator,
                          cynic-topology, cynic-pool, zeroclaw-solana-demo
  runtime_seed         1   cynic-host-exec
mixed lib+bin-root    7   cynic-cli, cynic-host, cynic-kernel, cynic-lxc,
                          cynic-mcp, cynic-node, cynic-zeroclaw-solana

make check          ok
make audit          blocked, 8 critical, 8x LEDGER-001-UNDECLARED-DEBT

audit-debt-ledger    218 entries (ARCH-001 193, ARCH-004 15, PATH-002 3, RUN-001 5, SEC-001 2)
module-inversion      42 identities, 117 edges, 8 crates
topology (generator)  up_to_date, differences 0

largest crates     92 356  cynic-host        <- god-object ceiling is 25k (foundation.json:78)
                   54 774  cynic-node                   (3.7x / 2.2x / 1.4x / 1.3x / 1.0x)
                   34 401  cynic-protocol
                   33 007  cynic-cli
                   25 971  cynic-inference

hardcoded in production   203 private IPs
                         171 node-vocabulary literals
                          38 .or_else() vocabulary fallbacks
dead registries            19 vertical intent.json (0 readers)
                           24 top-level dirs, 6 with 0 readers
toolchain conflicts         3 crates disagree, 1 deleted crate still declared
gates that do not decide   3
```

**The god-object ceiling is 25 000 LOC** (`contracts/foundation.json:78`).
`cynic-host` is at **92 356**, which is 3.7x the constitutional limit. It is not
Stage 1 because it is the biggest — it is Stage 3 because it is the only
extraction that moves an inversion count, an uncompiled-source count and a
hotspot count at the same time.

`cynic-audit` was the twelfth binary and was removed in `02a4895c`. The eleven
above are the measured census, and ART-001 accounts for every one of them —
`cynic-host-exec` through `runtime_seed.binary`, which is why it is a contract
entry and not an undeclared artefact. Do not "fix" that one.