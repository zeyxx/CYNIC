# CYNIC Intel — Salvage Notes

**Date:** 2026-10-08
**Purpose:** Recover the good ideas from intelligence code that is being purged. The code was
mis-architected on first instance (written Sept 2026, never wired to a consumer, never tested
in real conditions). The ideas below are worth keeping; the code is not.

**Status:** reference only. Not an authority. Nothing here is scheduled for implementation.

---

## Why this purge

Audit of `crates/cynic-intel` (7,647 LOC) + `crates/cynic-intel-escore` (944 LOC) on 2026-10-08:

- ~5,500 LOC have **no consumer** outside the crate.
- Only two modules are really wired: `autoresearch/bucket_signatures` (cynic-host) and
  `adapters/agentic_tool_call_adapter` (cynic-zeroclaw-solana).
- ~2,400 LOC **duplicate what @gcrtrd ships in production** (TrenchScanner rug/holder detection,
  TokenSage semantics).
- The E-Score is **unique but has no data source**: it computes on `EScoreSignal` inputs that
  nothing produces. No data port exists.

Operator statement: *"du code mort à purger qui peut avoir des bonnes idées, mais ça a été mal
architecturé/conceptualisé en première instance."*

---

## Idea 1 — CCM: Cognitive Crystallization Mechanism

**Source (being purged):** `crates/cynic-intel/src/ccm/` (~1,200 LOC: `crystal.rs`, `engine.rs`, `intake/`)

**The idea:** ephemeral verdicts → persistent wisdom. A "crystal" is a domain-agnostic insight that
crystallizes out of many independent observations, with an auditable provenance trail.

**The 4D crystal model:**
| Dimension | How it is computed |
|---|---|
| **Quality** | running mean of normalized Q-Score totals |
| **Certainty** | `concordance × volume`; concordance = `1 / (1 + (stddev / φ⁻³)²)`, volume ramps to 21 obs |
| **Polarity** | dominant verdict kind (howl/wag/growl/bark counts) |
| **Time** | decay relevance |

**The interesting decision — the crystallization gate uses CERTAINTY, not quality.**
`certainty ≥ φ⁻¹` (0.618). This lets *negative truths* (BARK/GROWL) crystallize when the system is
**certain**, not only when the content is "good". A confirmed bad outcome is knowledge too.

**Lifecycle (Fibonacci):** `Forming` → `Crystallized` (≥21 obs, F8) → `Canonical` (≥233 obs, F13)
→ `Decaying` (certainty < φ⁻²) → `Dissolved`.

**Provenance:** `contributing_verdicts: Vec<String>` — you can always ask *"why did this
crystallize?"* and trace back to the verdicts that produced it. Bounded at 500 entries.

**Why it was mis-architected:** it depends on `cynic_protocol::{RawObservation, delimit_crystal_content}`
and a SurrealDB storage path that was never wired. The engine computes certainty in Rust while
"production uses atomic SQL" — two implementations of the same logic, only one tested.

**If revived, it should be:** a pure domain module with a declared data port, fed by a real
observation stream, with a single implementation of the certainty math.

---

## Idea 2 — Epistemic consensus (multi-judge robust scoring)

**Source (being purged):** `crates/cynic-intel/src/epistemic/` (~285 LOC: `consensus.rs`, `types.rs`, `crystal.rs`)

**The idea:** several judges (dogs) score the same stimulus on 6 axioms; the consensus must resist
outliers and single-judge capture.

**The 6 axioms:** `fidelity`, `phi`, `verify`, `culture`, `burn`, `sovereignty`.

**Robust aggregation:**
- **Trimmed mean** in basis points: drops min and max when ≥4 voters, plain mean below.
- **Geometric mean** of the 6 axioms (`(f1·f2·…·f6)^(1/6)` computed in log space to avoid overflow).
  Geometric mean punishes a single zero axiom hard — you cannot buy your way out of a failed axiom.

**φ-calibrated thresholds (bps):**
| Threshold | Value | Meaning |
|---|---|---|
| φ⁻¹ | 6180 | Howl floor / crystallization |
| φ⁻² + φ⁻⁴ | 5279 | Howl (exceptional) |
| φ⁻² | 3820 | Wag (good) |
| φ⁻³ | 2361 | Growl (doubtful) / below = Bark (rejected) |

**Verdict vocabulary:** Howl / Wag / Growl / Bark.

**Why it was mis-architected:** the thresholds and the verdict type live in `epistemic/types.rs`
while `cynic_protocol` also carries φ constants — duplicated authority. The consensus function has
no caller: the "dogs" it aggregates were never wired to it.

**If revived, it should be:** one φ-constant authority (cynic-protocol), one consensus function,
called by a real multi-judge dispatch.

---

## Idea 3 — E-Score: 7-dimension reputation

**Source (being frozen, not purged):** `crates/cynic-intel-escore/` (944 LOC) + `crates/cynic-intel/src/escore/`

**The idea:** a wallet's reputation is not one number but 7 Greek virtues, each with its own decay
rate, weighted into a composite.

**The 7 dimensions (with default weights, bps summing to 10,000):**
| Virtue | Meaning | Weight | Daily retention |
|---|---|---|---|
| **Karteria** | endurance, long-term consistency | 25% | 98%/day (most stable) |
| **Parrhesia** | honesty: no wash-trading, no rug | 20% | 97%/day |
| **Anaideia** | humility: not a dominant whale | 15% | 96%/day |
| **Philanthropia** | generosity: LP, grants | 15% | 95%/day |
| **Autarkeia** | independence from leverage/risky protocols | 10% | 96%/day |
| **Poiesis** | value creation: public contracts, tools, bots | 5% | 94%/day |
| **Koinonia** | community life: governance, voting | 10% | 92%/day (most volatile) |

**Two design decisions worth keeping:**
1. **Integer arithmetic only.** `ScoreUnit = u64`, 1 point = 1,000,000 units. Two nodes computing
   the same sequence get *exactly* the same result — no IEEE 754 non-associativity. Determinism is
   a requirement, not a nicety.
2. **Evidence tiers.** Each signal carries a trust tier: `T1OnChain` (100%), `T2Attestation` (75%),
   `T3Oracle` (50%), `T4Solo` (25%). The further from the SVM, the less it counts.

**Why it is frozen, not purged:** it is the only *unique* asset (no @gcrtrd equivalent), but it
**has no fuel**. `EScoreSignal` inputs are never produced; no data port exists; the Helius adapter
that could feed it is not wired. It needs a large, long-horizon on-chain data source to mean
anything. Building that is a separate project.

**If revived, it needs:** a declared data port, a real on-chain history source, and enough
observation time that decay and endurance become meaningful.

---

## Other purged ideas (smaller)

- **`rug_prefilter` / `kscore` / `rug_risk` / `token_postprocessor` / `wallet_judgment` /
  `wallet_enrichment` / `stimulus`**: deterministic Solana token-hygiene heuristics. **No salvage —
  @gcrtrd already ships these in production** (TrenchScanner). If CYNIC needs them, consume his API.
- **`solana_safety_dog` + `sop_sanitizer`**: KEPT (wired to cynic-zeroclaw-solana).
- **`futarchy/`**: KEPT (wired to cynic-intel-futarchy + cynic-cli).
- **`autoresearch/bucket_signatures`**: KEPT (wired to cynic-host).
- **`compliance`**, **`domain_enrichment`**, **`helius_credit`**, **`adapters/helius/`**,
  **`ports/market_data_port`**, **`ports/solana_ports` (RPC part)**: no consumer, no unique idea
  worth recording. Purged.

---

## Rule extracted from this audit

**Write intelligence only when a consumer exists or a data source is committed.** Code with no
consumer and no fuel is not an asset; it is debt that looks like progress. The `cynic-intel` crate
grew to 7,647 LOC where ~450 LOC were alive — the gap was never visible because nothing forced it
to be. A crate's real size is its *consumed* surface, not its line count.
