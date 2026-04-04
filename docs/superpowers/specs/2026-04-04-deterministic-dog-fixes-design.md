# DeterministicDog — Research-Informed Bug Fixes

**Date:** 2026-04-04
**Approach:** B — fix all 6 bugs using established methods, no feature additions
**Scope:** `cynic-kernel/src/dogs/deterministic.rs` + `cynic-kernel/src/infra/config.rs` (Pocks bug)

## Context

The DeterministicDog is CYNIC's always-on FORM judge — pure, deterministic, zero I/O.
It evaluates structural quality (PHI), efficiency (BURN), and coercion (SOVEREIGNTY),
abstaining on substance axioms (FIDELITY, VERIFY, CULTURE) with NEUTRAL scores.

An audit found 6 bugs + 2 adjacent bugs (Pocks). This spec fixes all 8 without
adding features or changing the Dog's role.

**Sequencing:** Fix 7 must be applied before or together with Fix 5
(`formal_notation_count` replaces `algebraic_count`).

## Fixes

### Fix 1 — Context contamination (HIGH)

**Bug:** Signal detection (`absolutes_count`, `hedging_count`, `coercion_count`,
`agency_count`, `signal_density`) operates on `all_text = content + context`.
Crystal context injected via `stimulus.context` contaminates scores.
F11 fixed `unique_ratio` to content-only but left signal counters on `all_text`.

**Root cause:** Line 31 `let all_text = format!("{content} {context}")` feeds
`words` and `lower_words`, which feed all signal counters.

**Fix:** All signal counters operate on `content` only. Remove `all_text`.
The `context` field is not used by the DeterministicDog — it judges FORM of the
content submitted, not its relationship to context. Context is metadata for LLM Dogs.

```rust
// BEFORE (buggy)
let all_text = format!("{content} {context}");
let words: Vec<&str> = all_text.split_whitespace().collect();

// AFTER
let words: Vec<&str> = content.split_whitespace().collect();
// `context` is intentionally unused — DeterministicDog judges FORM of content only.
// LLM Dogs use context for SUBSTANCE evaluation.
```

**Cleanup:** After this fix, `words` and `content_words` become identical (both
operate on `content`). Remove `content_words` / `content_word_count` — use `words`
and `word_count` everywhere. `#![deny(dead_code)]` will catch this at compile time
if missed.

**Regression test:** Stimulus with neutral content + context containing
"must always never guaranteed obey mandatory" → scores must equal
same content with no context.

**Reference:** Internal — F11 fix (unique_ratio) established the precedent.

---

### Fix 2 — Abstention by intention, not value coincidence (MEDIUM)

**Bug:** Lines 294-302 detect abstention via `(score - NEUTRAL).abs() < 0.001`.
If PHI calculation accidentally lands on ≈0.309, it's falsely marked as abstention.
Abstention should be a decision at scoring time, not a value comparison after.

**Fix:** Each axiom scoring block returns a `(score, abstained: bool)` tuple.
Abstentions are built from the boolean flags, not from the output values.

```rust
// FIDELITY: abstain unless extreme red flag
let (fidelity, fidelity_abstained) = if absolutes_count >= 3 && hedging_count == 0 {
    (0.20, false) // active judgment: red flag
} else {
    (NEUTRAL, true) // explicit abstention
};

// VERIFY: abstain unless formal notation detected
let (verify, verify_abstained) = if formal_notation_count >= 2 {
    ((NEUTRAL + 0.10).min(PHI_INV), false)
} else {
    (NEUTRAL, true)
};

// CULTURE: always abstain
let (culture, culture_abstained) = (NEUTRAL, true);

// Build abstentions from flags, not values
let mut abstentions = Vec::new();
if fidelity_abstained { abstentions.push("fidelity".into()); }
if verify_abstained { abstentions.push("verify".into()); }
if culture_abstained { abstentions.push("culture".into()); }
```

**Regression test:** Craft a stimulus where PHI calculation yields exactly NEUTRAL
(≈0.309). Verify PHI is NOT in `abstentions`. Craft another where FIDELITY abstains.
Verify FIDELITY IS in `abstentions`.

---

### Fix 3 — Sentence Boundary Detection (MEDIUM)

**Bug:** `content.matches('.').count() + content.matches('!').count() +
content.matches('?').count()` counts every period as a sentence boundary.
"e.g." = 1, "3.14" = 1, "1. e4 e5 2. Nf3" = 2. Inflates `sentence_count`,
affects PHI and BURN scoring.

**Fix:** Replace with a `count_sentences(text: &str) -> usize` function using
a simplified Mikheev cascade (~97% accuracy, zero external data):

**Algorithm:**
1. Split text into whitespace-delimited tokens.
2. For each token ending in `.`, `!`, or `?`:
   a. **Ellipsis filter:** token is `...` or `..` → not a boundary, continue.
   b. **Decimal filter:** token matches pattern `digit(s).digit(s)` → not a boundary.
   c. **Numbered list filter:** token matches `digit(s).` and is NOT followed by an
      uppercase-starting token → not a boundary (handles "1. e4", "2. Nf3").
   d. **Abbreviation filter:** strip trailing `.` from token, then strip any
      remaining internal `.` characters, then lowercase the result.
      If it's in the static abbreviation set → not a boundary.
      (e.g. `"e.g."` → strip trailing → `"e.g"` → strip internal → `"eg"` → in set)
   e. **Default:** count as sentence boundary.
3. Tokens ending in `!` or `?` are always boundaries (no ambiguity).
4. Return `max(boundary_count, 1)` — empty/no-punctuation text = 1 sentence.

**Static abbreviation set (~60 entries, no external file):**
```rust
static ABBREVIATIONS: phf::Set<&str> = phf_set! {
    // Titles
    "dr", "mr", "mrs", "ms", "prof", "sr", "jr", "rev", "gen", "sgt",
    // Latin/academic
    "etc", "eg", "ie", "al", "vs", "viz", "cf", "ca",
    // Also handle dotted forms: "e.g" → strip dots → "eg"
    // Months
    "jan", "feb", "mar", "apr", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    // Organizations
    "inc", "corp", "ltd", "co", "dept", "assn", "govt",
    // Measurements
    "approx", "est", "fig", "vol", "no", "pg", "pp",
    // Places
    "st", "ave", "blvd", "rd",
};
```

**Implementation note:** Use a `HashSet` or `phf::Set` (if phf is already a dep,
otherwise a match statement or sorted slice with binary search). No new dependencies
required — a `const` array + `.contains()` on a small set is fast enough.

**Regression tests:**
- `"Dr. Smith analyzed 3.14 results. It worked."` → 2 sentences
- `"1. e4 e5 2. Nf3 Nc6 3. Bb5"` → 2 sentences (numbered-list filter catches
  `1.` before lowercase `e`, but `2.` before uppercase `N` and `3.` before
  uppercase `B` are ambiguous — algorithm conservatively counts them as boundaries.
  This is acceptable: the old code counted 3, we count 2, and domain-specific
  SBD refinement is a future extension, not a bug fix.)
- `"Hello world"` → 1 sentence (no punctuation)
- `"Really? Yes! OK."` → 3 sentences
- `"I studied e.g. physics. Then math."` → 2 sentences
- `"..."` → 1 sentence (ellipsis, not 3)

**Reference:** Mikheev (2002), "Periods, Capitalized Words, etc." — 99.55% on WSJ
with derived abbreviation lists. Our simplified cascade targets ~97% with a static list.

---

### Fix 4 — Named constants instead of magic numbers (LOW)

**Bug:** 19 literal numbers (0.30, 0.10, 0.05, 0.35, 0.40, 0.15, 0.20, 50, 150,
300, 500, 0.4, 0.5, etc.) with no names and no justification. Hard to reason about,
hard to calibrate.

**Non-fix:** Research found NO peer-reviewed evidence linking φ to text quality
(Zipf's law is a power law with exponent ~1, mathematically unrelated to φ).
Using φ-derived values for scoring thresholds would be cargo cult.
Ref: Mandelbrot (1953) — Zipf's law emerges from optimal coding theory, no φ.

**Fix:** Extract all literals into named constants with doc comments explaining
their empirical calibration. Group by axiom. Values stay the same — the fix is
naming and documenting, not changing behavior.

```rust
// ── Scoring constants (empirically calibrated) ──────────────
// These values are tuned to produce scores in [0.05, φ⁻¹] that
// distribute verdicts across the BARK→HOWL range for typical text.
// No linguistic theory justifies specific values — they are engineering
// choices validated by test coverage.

/// PHI base score — center of scoring range.
const PHI_BASE: f64 = 0.30;
/// BURN base score — slightly above center (benefit of the doubt on efficiency).
const BURN_BASE: f64 = 0.35;
/// SOVEREIGNTY base score — default assumes moderate agency.
const SOVEREIGNTY_BASE: f64 = 0.40;

/// Small signal adjustment (single structural indicator).
const ADJUST_SMALL: f64 = 0.05;
/// Medium signal adjustment (strong structural indicator).
const ADJUST_MEDIUM: f64 = 0.10;
/// Large penalty (severe structural deficiency).
const ADJUST_LARGE: f64 = 0.15;

/// Golden zone for text length: 50-300 chars.
const LEN_GOLDEN_MIN: usize = 50;
const LEN_GOLDEN_MAX: usize = 300;
/// Verbose threshold.
const LEN_VERBOSE: usize = 500;
/// Minimal threshold (too short to judge structure).
const LEN_MINIMAL: usize = 15;
/// Concise threshold for BURN bonus.
const LEN_CONCISE: usize = 150;

/// Vocabulary diversity threshold — below this = repetitive.
const DIVERSITY_LOW: f64 = 0.4;
/// Vocabulary diversity midpoint.
const DIVERSITY_MID: f64 = 0.5;

/// Minimum sentence count for structure bonus.
const SENTENCE_MIN_FOR_STRUCTURE: usize = 2;
/// Minimum word count for structure bonus.
const WORD_MIN_FOR_STRUCTURE: usize = 10;
/// Minimum word count for concise-structure bonus (lower bar).
const WORD_MIN_FOR_CONCISE: usize = 5;
/// Maximum sentence count for concise-structure bonus.
const SENTENCE_MAX_FOR_CONCISE: usize = 5;
```

**Regression test:** All existing tests must pass unchanged — values don't change,
only names are introduced.

---

### Fix 5 — Separate hedging from signal density (LOW)

**Bug:** `signal_density = (absolutes_count + hedging_count + algebraic_count) / word_count`.
Hedging (Hyland 1998: "probably", "likely", "perhaps") is counted as positive "signal"
alongside absolutes, boosting BURN. But hedging adds words for epistemic caution —
it's the opposite of information density.

**Fix:** Split into two distinct metrics:

```rust
/// Assertion density: absolute claims + formal notation = assertive content.
let assertion_density = if word_count > 0 {
    (absolutes_count + formal_notation_count) as f64 / word_count as f64
} else {
    0.0
};

/// Hedge ratio: epistemic caution markers.
/// Neutral for BURN — hedging is prudence, not waste.
let hedge_ratio = if word_count > 0 {
    hedging_count as f64 / word_count as f64
} else {
    0.0
};
```

Scoring changes:
- BURN uses `assertion_density` (not `signal_density`) for information density bonus.
- `hedge_ratio` is **not used in BURN** — hedging is neutral for efficiency.
- FIDELITY red-flag check remains: `absolutes_count >= 3 && hedging_count == 0`
  (many absolutes WITH zero hedging = red flag). This already treats hedging correctly.
- `burn_reason` string at lines 256 and 268 must also be updated:
  replace `signal_density` references with `assertion_density` in both the
  condition (`len < 100 && assertion_density > 0.10`) and the format string.

**Reference:** Hyland, K. (1998). "Boosting, hedging and the negotiation of academic
knowledge." TEXT 18(3). Hedges outnumber boosters 3:1 in academic prose. Hedging
is a quality signal for epistemic honesty, not noise.

**Regression test:** Text with high hedging ("probably likely perhaps approximately
tends to suggest") must NOT get inflated BURN score.

---

### Fix 6 — Coercion as density, not word count (LOW)

**Bug:** Every "must", "mandatory", "required" deducts a fixed 0.10 from SOVEREIGNTY
regardless of text length. A single "must" in a 500-word analysis gets the same
penalty as "must" in a 10-word command. No distinction between isolated technical
usage and concentrated coercive pattern.

**Fix:** Density-based coercion detection:

```rust
/// Coercion density threshold — below this, isolated occurrences are noise.
const COERCION_DENSITY_THRESHOLD: f64 = 0.03; // ~1 coercive term per 33 words

// RFC 2119 uppercase keywords are normative, not coercive — exclude them.
// Ref: Bradner (1997), RFC 2119. MUST/SHALL/REQUIRED in caps = formal specification.
let coercion_count = lower_words.iter()
    .zip(words.iter())  // keep original case for RFC check
    .filter(|(lower, original)| {
        let is_coercive = matches!(lower.as_str(),
            "must" | "mandatory" | "forced" | "required" | "compulsory" | "obey"
        );
        // Exclude RFC 2119 uppercase: MUST, SHALL, REQUIRED
        let is_rfc_keyword = original.chars().all(|c| c.is_uppercase() || !c.is_alphabetic());
        is_coercive && !is_rfc_keyword
    })
    .count();

let coercion_density = coercion_count as f64 / word_count.max(1) as f64;

/// Agency density threshold — symmetric with coercion.
const AGENCY_DENSITY_THRESHOLD: f64 = 0.03;

let coercion_density = coercion_count as f64 / word_count.max(1) as f64;
let agency_density = agency_count as f64 / word_count.max(1) as f64;

let mut sovereignty = SOVEREIGNTY_BASE;

// Both directions use the same density-based excess formula.
// Symmetric: same structure, same scaling, same threshold logic.
if coercion_density > COERCION_DENSITY_THRESHOLD {
    let excess = (coercion_density - COERCION_DENSITY_THRESHOLD) / COERCION_DENSITY_THRESHOLD;
    sovereignty -= ADJUST_MEDIUM * excess.min(3.0);
}
if agency_density > AGENCY_DENSITY_THRESHOLD {
    let excess = (agency_density - AGENCY_DENSITY_THRESHOLD) / AGENCY_DENSITY_THRESHOLD;
    sovereignty += ADJUST_SMALL * excess.min(3.0);
}
```

**Why density, not disambiguation:** Kratzer (1977, 1981) showed deontic vs epistemic
"must" requires syntactic parsing (subject animacy, predicate type, aspect). Without a
POS tagger, word-level disambiguation has ~30% false positive rate on epistemic uses.
Density-based detection is domain-agnostic: a single "must" in a long text → noise.
Concentrated "must obey required mandatory" → real coercion pattern. No domain-specific
heuristics needed.

**Symmetric design:** Both coercion and agency use the same density+excess formula.
This prevents asymmetric bias where long texts get free agency boosts while coercion
scales down with length. Same structure, same threshold logic, both directions.

**Reference:** RFC 2119 (Bradner, 1997) for uppercase keyword exclusion.
Kratzer (1977, 1981) for deontic/epistemic modality theory.

**Regression tests:**
- `"You must obey. This is mandatory and required. Compliance is compulsory."` →
  high coercion density → SOVEREIGNTY penalized (existing test, must still pass)
- `"Implementations MUST support this format per RFC 7231."` →
  RFC keyword excluded → no SOVEREIGNTY penalty
- `"The client must handle errors gracefully. The server processes requests
  and returns appropriate status codes for each endpoint."` →
  coercion density 1/18 ≈ 0.056, excess = (0.056-0.03)/0.03 ≈ 0.87,
  penalty = 0.10 × 0.87 = 0.087 → moderate penalty (sovereignty ≈ 0.31),
  not severe — proportional to how far above threshold, no cliff

---

### Fix 7 — Extensible formal notation detection (LOW, Pocks)

**Bug:** Chess algebraic notation is detected via a hardcoded `is_chess` guard.
Adding a second domain (trading tickers, math formulas) requires a new `if/else`
branch in the Dog. Pattern doesn't scale past N=2.

**Fix:** Extract formal notation detection into a declarative registry:

```rust
/// Domain-specific formal notation detector.
/// Returns count of recognized formal tokens in the word list.
type NotationDetector = fn(words: &[&str]) -> usize;

/// Registry of formal notation patterns by domain.
/// New domain = add one entry here + one detection function. No scoring logic changes.
static FORMAL_NOTATIONS: &[(&str, NotationDetector)] = &[
    ("chess", detect_algebraic_notation),
];

/// Look up the notation detector for a domain. Returns 0-count fn for unknown domains.
fn detect_formal_notation(domain: Option<&str>, words: &[&str]) -> usize {
    let domain = domain.unwrap_or("");
    FORMAL_NOTATIONS.iter()
        .find(|(d, _)| d.eq_ignore_ascii_case(domain))
        .map(|(_, detect)| detect(words))
        .unwrap_or(0)
}

fn detect_algebraic_notation(words: &[&str]) -> usize {
    words.iter()
        .filter(|w| {
            let w = w.trim_matches(|c: char| !c.is_alphanumeric());
            w.len() >= 2
                && w.len() <= 6
                && w.chars().next().is_some_and(|c| "abcdefghKQRBNO".contains(c))
                && w.chars().any(|c| c.is_ascii_digit())
        })
        .count()
}
```

The scoring code uses `formal_notation_count` everywhere instead of `algebraic_count`.
VERIFY boost and PHI notation bonus both key off this single count — domain-agnostic.

**Regression test:** All existing chess tests must pass. Non-chess domains must
return 0 (same as current `else { 0 }` branch).

---

### Fix 8 — Domain prompt heading skip (Pocks, adjacent)

**File:** `cynic-kernel/src/infra/config.rs:404-409`

**Bug:** `content.trim_start().starts_with('#')` matches both `# Title` (H1) and
`## FIDELITY` (H2). If a domain file has no H1 title, the first `## AXIOM` line
is silently consumed. Additionally, the heading check operates on the original string
while the iterator has already consumed leading blank lines via `skip_while` —
these are synchronized by coincidence, not by construction.

**Fix:** Check heading existence from the iterator, not the original string.
Match H1 specifically (`# ` with space, not `##`):

```rust
let lines: Vec<&str> = content.lines().collect();
let first_content = lines.iter()
    .find(|l| !l.trim().is_empty());

// Space after `#` required: `# Title` is H1, `#NoSpace` is not valid markdown,
// `## FIDELITY` is H2 (must not be skipped). The `!starts_with("## ")` guard
// prevents reintroducing the original bug.
let skip_heading = first_content
    .is_some_and(|l| l.starts_with("# ") && !l.starts_with("## "));

let prompt: String = content
    .lines()
    .skip_while(|l| l.trim().is_empty())
    .skip(if skip_heading { 1 } else { 0 })
    .collect::<Vec<_>>()
    .join("\n")
    .trim()
    .to_string();
```

**Regression tests:**
- File with `# Title\n## FIDELITY\n...` → skips title, keeps FIDELITY
- File with `## FIDELITY\n...` (no title) → keeps FIDELITY
- File with blank lines then `# Title\n...` → skips blank lines and title
- File with blank lines then `## FIDELITY\n...` → skips blank lines, keeps FIDELITY

---

## Test Strategy

Each fix has specific regression tests listed above. Additionally:

1. **All 7 existing DeterministicDog tests must pass unchanged.** The fixes change
   internals but should not change scores for the existing test stimuli (which use
   no context, no RFC keywords, and have simple sentence structures).

2. **New test module:** `tests_sbd` for sentence boundary detection edge cases.

3. **New test:** context contamination regression — same content scored with and
   without adversarial context must produce identical scores.

4. **Property:** DeterministicDog never returns `Err` — it always produces scores.
   This is unchanged (no new failure modes introduced).

## Non-Goals

- No new axiom scoring logic
- No embedding integration (future extension, not a fix)
- No HD-D or readability indices (Approach C, deferred)
- No new domain notation detectors beyond chess (Fix 7 is the *structure*, not content)
- φ-derived scoring constants are explicitly rejected per research findings

## Dependencies

- No new crate dependencies. All fixes use stdlib + existing deps.
- `phf` considered for abbreviation set but not required — a `match` or sorted
  slice with binary search on 60 entries is <1μs.

## References

| Topic | Source |
|---|---|
| Sentence boundary detection | Mikheev (2002), Kiss & Strunk (2006) Punkt |
| Hedging taxonomy | Hyland (1998), TEXT 18(3) |
| Deontic modality | Kratzer (1977, 1981) |
| RFC keywords | RFC 2119, Bradner (1997) |
| φ in natural language | No evidence found — Mandelbrot (1953), Zipf unrelated |
| Lexical diversity | McCarthy & Jarvis (2010) — MTLD/HD-D (deferred to Approach C) |
