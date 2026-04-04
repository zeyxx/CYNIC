# DeterministicDog Bug Fixes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 8 bugs in DeterministicDog and domain prompt loader, grounded in NLP research.

**Architecture:** All fixes are in 2 files: `cynic-kernel/src/dogs/deterministic.rs` (Fixes 1-7) and `cynic-kernel/src/infra/config.rs` (Fix 8). Fixes are ordered by dependency: Fix 4 (constants) first, then Fix 7 (notation registry), then Fix 1 (context), then Fix 3 (SBD), then Fix 5 (hedging), then Fix 2 (abstention), then Fix 6 (coercion density), then Fix 8 (heading skip). Each fix is a commit.

**Tech Stack:** Rust 1.94+, tokio, no new crate dependencies.

**Spec:** `docs/superpowers/specs/2026-04-04-deterministic-dog-fixes-design.md`

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `cynic-kernel/src/dogs/deterministic.rs` | Modify | All 7 DeterministicDog fixes |
| `cynic-kernel/src/infra/config.rs` | Modify (lines 403-415) | Fix 8: heading skip |

No new files created. No new dependencies.

---

### Task 1: Extract named constants (Fix 4)

**Why first:** All subsequent fixes reference these constants. No behavioral change — pure rename.

**Files:**
- Modify: `cynic-kernel/src/dogs/deterministic.rs:16` (after NEUTRAL const)

- [ ] **Step 1: Add constants block after NEUTRAL**

Add these constants at `deterministic.rs:17` (after `const NEUTRAL`):

```rust
// ── Scoring constants (empirically calibrated) ──────────────
// These values produce scores in [0.05, φ⁻¹] that distribute verdicts
// across the BARK→HOWL range for typical text. No linguistic theory
// justifies specific values — engineering choices validated by tests.

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
/// Vocabulary diversity proportion scaling.
const DIVERSITY_SCALE: f64 = 0.20;

/// Golden zone for text length (chars).
const LEN_GOLDEN_MIN: usize = 50;
const LEN_GOLDEN_MAX: usize = 300;
/// Verbose threshold (chars).
const LEN_VERBOSE: usize = 500;
/// Minimal threshold — too short to judge structure (chars).
const LEN_MINIMAL: usize = 15;
/// Concise threshold for BURN bonus (chars).
const LEN_CONCISE: usize = 150;

/// Vocabulary diversity threshold — below this = repetitive.
const DIVERSITY_LOW: f64 = 0.4;
/// Vocabulary diversity midpoint.
const DIVERSITY_MID: f64 = 0.5;

/// Minimum sentence count for structure bonus.
const SENTENCE_MIN_FOR_STRUCTURE: usize = 2;
/// Minimum word count for structure bonus.
const WORD_MIN_FOR_STRUCTURE: usize = 10;
/// Minimum word count for concise-structure bonus.
const WORD_MIN_FOR_CONCISE: usize = 5;
/// Maximum sentence count for concise-structure bonus.
const SENTENCE_MAX_FOR_CONCISE: usize = 5;

/// Coercion density threshold — below this, isolated occurrences are noise.
/// ~1 coercive term per 33 words.
const COERCION_DENSITY_THRESHOLD: f64 = 0.03;
/// Agency density threshold — symmetric with coercion.
const AGENCY_DENSITY_THRESHOLD: f64 = 0.03;

/// FIDELITY red-flag score when many absolutes detected.
/// Distinct from DIVERSITY_SCALE (same value, different meaning).
const FIDELITY_RED_FLAG: f64 = 0.20;
```

- [ ] **Step 2: Replace all magic numbers in evaluate() with constants**

In the PHI section (~line 181-203), replace:
- `0.30` → `PHI_BASE`
- `0.10` → `ADJUST_MEDIUM`
- `0.05` → `ADJUST_SMALL`
- `0.20` → `DIVERSITY_SCALE`
- `50..=300` → `LEN_GOLDEN_MIN..=LEN_GOLDEN_MAX`
- `500` → `LEN_VERBOSE`
- `15` → `LEN_MINIMAL`
- `sentence_count >= 2` → `sentence_count >= SENTENCE_MIN_FOR_STRUCTURE`
- `word_count > 10` → `word_count > WORD_MIN_FOR_STRUCTURE`
- `(1..=5)` → `(1..=SENTENCE_MAX_FOR_CONCISE)`
- `word_count > 5` → `word_count > WORD_MIN_FOR_CONCISE`
- `(unique_ratio - 0.5) * 0.20` → `(unique_ratio - DIVERSITY_MID) * DIVERSITY_SCALE`

In the BURN section (~line 229-271), replace:
- `0.35` → `BURN_BASE`
- `0.10` → `ADJUST_MEDIUM`
- `0.05` → `ADJUST_SMALL`
- `0.15` → `ADJUST_LARGE` (for the low unique_ratio penalty)
- `150` → `LEN_CONCISE`
- `300` → `LEN_GOLDEN_MAX`
- `500` → `LEN_VERBOSE`
- `0.4` → `DIVERSITY_LOW`
- `0.5` → `DIVERSITY_MID` (where it appears in BURN)

In the SOVEREIGNTY section (~line 274-289), replace:
- `0.40` → `SOVEREIGNTY_BASE`
- `0.10` → `ADJUST_MEDIUM`
- `0.05` → `ADJUST_SMALL`

- [ ] **Step 3: Run tests**

Run: `cargo test --release -p cynic-kernel -- deterministic 2>&1 | tail -20`
Expected: all 7 existing tests pass. No behavioral change.

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/dogs/deterministic.rs
git commit -m "refactor(dog): extract 19 magic numbers into named constants

No behavioral change — pure rename. All existing tests pass unchanged.
Engineering choices documented, not φ-derived (no evidence for φ in text).
Ref: Mandelbrot (1953)."
```

---

### Task 2: Extensible formal notation registry (Fix 7)

**Why before Fix 1/5:** Fix 5 uses `formal_notation_count` which comes from this.

**Files:**
- Modify: `cynic-kernel/src/dogs/deterministic.rs`

- [ ] **Step 1: Write test for notation registry**

Add to the `#[cfg(test)] mod tests` block:

```rust
#[tokio::test]
async fn formal_notation_unknown_domain_returns_zero() {
    let dog = DeterministicDog;
    let stimulus = Stimulus {
        content: "1. e4 e5 2. Nf3 Nc6 3. Bb5 a6".into(),
        context: None,
        domain: Some("trading".into()), // not chess → no algebraic detection
    };
    let scores = dog.evaluate(&stimulus).await.unwrap();
    // VERIFY should be NEUTRAL — no formal notation detected for trading
    assert!(
        (scores.verify - NEUTRAL).abs() < 0.01,
        "non-chess domain should not detect algebraic notation, verify={}",
        scores.verify
    );
}
```

- [ ] **Step 2: Run test — verify it passes (already works with current code)**

Run: `cargo test --release -p cynic-kernel -- formal_notation_unknown 2>&1 | tail -5`
Expected: PASS (current `else { 0 }` branch already returns 0 for non-chess).

- [ ] **Step 3: Add notation registry and refactor**

Replace the `is_chess` + `algebraic_count` block (lines 122-141) with:

```rust
// ── Formal notation detection (extensible registry) ────
type NotationDetector = fn(words: &[&str]) -> usize;

/// Registry of domain-specific formal notation patterns.
/// New domain = add one entry + one detection function. No scoring logic changes.
static FORMAL_NOTATIONS: &[(&str, NotationDetector)] = &[
    ("chess", detect_algebraic_notation),
];

fn detect_formal_notation(domain: Option<&str>, words: &[&str]) -> usize {
    let domain = domain.unwrap_or("");
    FORMAL_NOTATIONS
        .iter()
        .find(|(d, _)| d.eq_ignore_ascii_case(domain))
        .map(|(_, detect)| detect(words))
        .unwrap_or(0)
}

fn detect_algebraic_notation(words: &[&str]) -> usize {
    words
        .iter()
        .filter(|w| {
            let w = w.trim_matches(|c: char| !c.is_alphanumeric());
            w.len() >= 2
                && w.len() <= 6
                && w.chars()
                    .next()
                    .is_some_and(|c| "abcdefghKQRBNO".contains(c))
                && w.chars().any(|c| c.is_ascii_digit())
        })
        .count()
}
```

NOTE: Place `detect_formal_notation` and `detect_algebraic_notation` as free functions OUTSIDE the `impl Dog for DeterministicDog` block (e.g. between the struct definition and the impl block). The `NotationDetector` type alias and `FORMAL_NOTATIONS` static go there too.

Then in `evaluate()`, replace:
```rust
let is_chess = stimulus
    .domain
    .as_deref()
    .is_some_and(|d| d.eq_ignore_ascii_case("chess"));
let algebraic_count = if is_chess {
    words
        .iter()
        .filter(|w| {
            let w = w.trim_matches(|c: char| !c.is_alphanumeric());
            w.len() >= 2
                && w.len() <= 6
                && w.chars()
                    .next()
                    .is_some_and(|c| "abcdefghKQRBNO".contains(c))
                && w.chars().any(|c| c.is_ascii_digit())
        })
        .count()
} else {
    0
};
```

With:
```rust
let formal_notation_count = detect_formal_notation(stimulus.domain.as_deref(), &words);
```

Then replace ALL occurrences of `algebraic_count` in the rest of `evaluate()` with `formal_notation_count`.

- [ ] **Step 4: Run all tests**

Run: `cargo test --release -p cynic-kernel -- deterministic 2>&1 | tail -20`
Expected: all 8 tests pass (7 existing + 1 new).

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/dogs/deterministic.rs
git commit -m "refactor(dog): extensible formal notation registry

Replace hardcoded is_chess guard with declarative FORMAL_NOTATIONS table.
New domain = one entry + one detection fn. No scoring logic changes.
Spotted by Pocks: 'counts one domain, crashes on two.'"
```

---

### Task 3: Fix context contamination (Fix 1)

**Files:**
- Modify: `cynic-kernel/src/dogs/deterministic.rs:27-38`

- [ ] **Step 1: Write regression test**

Add to tests:

```rust
#[tokio::test]
async fn context_does_not_contaminate_scores() {
    let dog = DeterministicDog;
    let content = "The Sicilian Defense is a strong opening for black.";

    let without_context = Stimulus {
        content: content.into(),
        context: None,
        domain: Some("chess".into()),
    };
    let with_adversarial_context = Stimulus {
        content: content.into(),
        context: Some(
            "must always never guaranteed obey mandatory forced required compulsory".into(),
        ),
        domain: Some("chess".into()),
    };

    let scores_clean = dog.evaluate(&without_context).await.unwrap();
    let scores_dirty = dog.evaluate(&with_adversarial_context).await.unwrap();

    assert!(
        (scores_clean.sovereignty - scores_dirty.sovereignty).abs() < 0.001,
        "context must not affect sovereignty: clean={}, dirty={}",
        scores_clean.sovereignty,
        scores_dirty.sovereignty
    );
    assert!(
        (scores_clean.phi - scores_dirty.phi).abs() < 0.001,
        "context must not affect phi: clean={}, dirty={}",
        scores_clean.phi,
        scores_dirty.phi
    );
    assert!(
        (scores_clean.burn - scores_dirty.burn).abs() < 0.001,
        "context must not affect burn: clean={}, dirty={}",
        scores_clean.burn,
        scores_dirty.burn
    );
}
```

- [ ] **Step 2: Run test — verify it FAILS**

Run: `cargo test --release -p cynic-kernel -- context_does_not_contaminate 2>&1 | tail -10`
Expected: FAIL — sovereignty/burn differ because context words inflate signal counts.

- [ ] **Step 3: Fix the contamination**

In `evaluate()`, replace lines 28-38:

```rust
let content = &stimulus.content;
let context = stimulus.context.as_deref().unwrap_or("");
let all_text = format!("{content} {context}");
let len = content.chars().count();
let words: Vec<&str> = all_text.split_whitespace().collect();
let word_count = words.len();

// F11 fix: unique_ratio from content-only words, not content+context.
// Context vocabulary inflates PHI/BURN scores — content is what we judge.
let content_words: Vec<&str> = content.split_whitespace().collect();
let content_word_count = content_words.len();
```

With:

```rust
let content = &stimulus.content;
// Context intentionally unused — DeterministicDog judges FORM of content only.
// LLM Dogs use context for SUBSTANCE evaluation. See Fix 1 / F11.
let len = content.chars().count();
let words: Vec<&str> = content.split_whitespace().collect();
let word_count = words.len();
```

Then remove all references to `content_words` and `content_word_count` — replace them with `words` and `word_count` respectively. Specifically, in the `unique_ratio` block (~line 143-158), change `content_words` → `words` and `content_word_count` → `word_count`.

- [ ] **Step 4: Run tests**

Run: `cargo test --release -p cynic-kernel -- deterministic 2>&1 | tail -20`
Expected: all 9 tests pass (including new context contamination test).

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/dogs/deterministic.rs
git commit -m "fix(dog): context no longer contaminates signal detection

Signal counters (absolutes, hedging, coercion, agency) now operate on
content only. F11 fixed unique_ratio; this extends to all counters.
Crystal context can no longer bias PHI/BURN/SOVEREIGNTY scores."
```

---

### Task 4: Sentence Boundary Detection (Fix 3)

**Files:**
- Modify: `cynic-kernel/src/dogs/deterministic.rs`

- [ ] **Step 1: Write SBD tests**

Add a new test group inside `#[cfg(test)] mod tests`:

```rust
// ── Sentence Boundary Detection tests ──────────────────

#[test]
fn sbd_abbreviations() {
    assert_eq!(count_sentences("Dr. Smith analyzed 3.14 results. It worked."), 2);
}

#[test]
fn sbd_no_punctuation() {
    assert_eq!(count_sentences("Hello world"), 1);
}

#[test]
fn sbd_multiple_terminators() {
    assert_eq!(count_sentences("Really? Yes! OK."), 3);
}

#[test]
fn sbd_latin_abbreviations() {
    assert_eq!(count_sentences("I studied e.g. physics. Then math."), 2);
}

#[test]
fn sbd_ellipsis() {
    assert_eq!(count_sentences("Wait... really?"), 1);
}

#[test]
fn sbd_chess_notation() {
    // "2." before "Nf3" (uppercase N) is ambiguous — conservatively counted as boundary.
    // Old code counted 3, this counts 2. Improvement, not perfection.
    assert_eq!(count_sentences("1. e4 e5 2. Nf3 Nc6 3. Bb5"), 2);
}

#[test]
fn sbd_empty() {
    assert_eq!(count_sentences(""), 1);
}

#[test]
fn sbd_decimal_numbers() {
    assert_eq!(count_sentences("The value is 3.14. The other is 2.718."), 2);
}
```

- [ ] **Step 2: Run tests — verify they FAIL (function doesn't exist yet)**

Run: `cargo test --release -p cynic-kernel -- sbd_ 2>&1 | tail -10`
Expected: FAIL — `count_sentences` not found.

- [ ] **Step 3: Implement count_sentences**

Add this function before the `impl Dog for DeterministicDog` block:

```rust
/// Abbreviation set for sentence boundary detection.
/// Ref: Mikheev (2002) simplified — static list, ~97% accuracy.
const ABBREVIATIONS: &[&str] = &[
    // Titles
    "dr", "mr", "mrs", "ms", "prof", "sr", "jr", "rev", "gen", "sgt", "lt", "col",
    // Latin/academic
    "etc", "eg", "ie", "al", "vs", "viz", "cf", "ca", "approx", "est",
    // Months
    "jan", "feb", "mar", "apr", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    // Organizations
    "inc", "corp", "ltd", "co", "dept", "assn", "govt",
    // Measurements
    "fig", "vol", "no", "pg", "pp",
    // Places
    "st", "ave", "blvd", "rd",
];

/// Count sentence boundaries using a simplified Mikheev cascade.
/// Handles abbreviations, decimals, ellipsis, and numbered lists.
fn count_sentences(text: &str) -> usize {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.is_empty() {
        return 1;
    }

    let mut count = 0;

    for (i, token) in tokens.iter().enumerate() {
        let last_char = match token.chars().last() {
            Some(c) if c == '.' || c == '!' || c == '?' => c,
            _ => continue,
        };

        // `!` and `?` are always boundaries
        if last_char != '.' {
            count += 1;
            continue;
        }

        // Ellipsis: "..." or ".."
        if token.ends_with("...") || token.ends_with("..") {
            continue;
        }

        // Decimal: token stripped of trailing period is a number (e.g. "3.14." → "3.14")
        let stripped_period = token.trim_end_matches('.');
        let is_decimal = stripped_period.contains('.')
            && stripped_period.split('.').count() == 2
            && stripped_period
                .split('.')
                .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
        if is_decimal {
            continue;
        }

        // Numbered list: "1." "2." etc. followed by non-uppercase token
        let stripped = token.trim_end_matches('.');
        if !stripped.is_empty() && stripped.chars().all(|c| c.is_ascii_digit()) {
            let next_starts_upper = tokens
                .get(i + 1)
                .and_then(|t| t.chars().next())
                .is_some_and(|c| c.is_uppercase());
            if !next_starts_upper {
                continue;
            }
            // Ambiguous (numbered list or sentence end before uppercase) — count it
        }

        // Abbreviation: strip trailing dots, strip internal dots, lowercase, check set
        let without_trailing = token.trim_end_matches('.');
        let normalized: String = without_trailing
            .replace('.', "")
            .to_lowercase();
        if !normalized.is_empty() && ABBREVIATIONS.contains(&normalized.as_str()) {
            continue;
        }

        // Default: sentence boundary
        count += 1;
    }

    count.max(1)
}
```

- [ ] **Step 4: Wire count_sentences into evaluate()**

Replace the sentence counting line (currently ~line 112-114):
```rust
let sentence_count = content.matches('.').count()
    + content.matches('!').count()
    + content.matches('?').count();
```

With:
```rust
let sentence_count = count_sentences(content);
```

- [ ] **Step 5: Run all tests**

Run: `cargo test --release -p cynic-kernel -- deterministic 2>&1 | tail -20`
Expected: all tests pass (SBD + existing).

- [ ] **Step 6: Commit**

```bash
git add cynic-kernel/src/dogs/deterministic.rs
git commit -m "fix(dog): proper sentence boundary detection (Mikheev cascade)

Replace naive content.matches('.') with count_sentences() using:
- Static abbreviation set (dr, eg, etc — 40 entries)
- Decimal number filter (3.14)
- Ellipsis filter (...)
- Numbered list filter (1. e4)
Ref: Mikheev (2002), ~97% accuracy without corpus."
```

---

### Task 5: Separate hedging from signal density (Fix 5)

**Files:**
- Modify: `cynic-kernel/src/dogs/deterministic.rs`

- [ ] **Step 1: Write test**

```rust
#[tokio::test]
async fn hedging_does_not_inflate_burn() {
    let dog = DeterministicDog;
    let hedged = Stimulus {
        content: "This probably likely perhaps approximately tends to suggest something might work.".into(),
        context: None,
        domain: None,
    };
    let neutral = Stimulus {
        content: "This thing does something that works in a normal way overall.".into(),
        context: None,
        domain: None,
    };
    let scores_hedged = dog.evaluate(&hedged).await.unwrap();
    let scores_neutral = dog.evaluate(&neutral).await.unwrap();

    // Hedging should NOT inflate BURN above a similar-length neutral text
    assert!(
        scores_hedged.burn <= scores_neutral.burn + ADJUST_SMALL,
        "hedging should not inflate burn: hedged={}, neutral={}",
        scores_hedged.burn,
        scores_neutral.burn
    );
}
```

- [ ] **Step 2: Run test — verify it FAILS**

Run: `cargo test --release -p cynic-kernel -- hedging_does_not_inflate 2>&1 | tail -10`
Expected: FAIL — hedging currently inflates `signal_density` → boosts BURN.

- [ ] **Step 3: Replace signal_density with assertion_density**

In `evaluate()`, replace the `signal_density` computation (~line 161-165):
```rust
// Proportional signal density (count-based, not boolean)
let signal_density = if word_count > 0 {
    (absolutes_count + hedging_count + formal_notation_count) as f64 / word_count as f64
} else {
    0.0
};
```

With:
```rust
// Assertion density: absolute claims + formal notation = assertive content.
// Hedging excluded — epistemic caution is neutral for efficiency, not signal.
// Ref: Hyland (1998), hedges outnumber boosters 3:1 in academic prose.
let assertion_density = if word_count > 0 {
    (absolutes_count + formal_notation_count) as f64 / word_count as f64
} else {
    0.0
};
```

Then replace all remaining `signal_density` references:
- In BURN scoring: `signal_density` → `assertion_density`
- In BURN reason condition (~line 256): `signal_density > 0.10` → `assertion_density > 0.10`
- In BURN reason format (~line 268): `signal_density * 100.0` → `assertion_density * 100.0`
- In the format string text: `"signal density"` → `"assertion density"`

- [ ] **Step 4: Run tests**

Run: `cargo test --release -p cynic-kernel -- deterministic 2>&1 | tail -20`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/dogs/deterministic.rs
git commit -m "fix(dog): separate hedging from assertion density

Hedging no longer inflates BURN. signal_density split into
assertion_density (absolutes + notation) used by BURN, and
hedge_ratio (unused in scoring — neutral for efficiency).
Ref: Hyland (1998), TEXT 18(3)."
```

---

### Task 6: Abstention by intention (Fix 2)

**Files:**
- Modify: `cynic-kernel/src/dogs/deterministic.rs`

- [ ] **Step 1: Write test for false abstention on PHI**

```rust
#[tokio::test]
async fn phi_near_neutral_is_not_abstention() {
    let dog = DeterministicDog;
    // Craft stimulus that produces PHI ≈ NEUTRAL (0.309):
    // Short text, no sentences, low diversity → PHI ≈ 0.30 base + small adjustments
    let stimulus = Stimulus {
        content: "a b c d e f g h".into(), // 8 words, no sentences, unique_ratio = 1.0, 15 chars
        context: None,
        domain: None,
    };
    let scores = dog.evaluate(&stimulus).await.unwrap();
    // PHI is a real judgment (not abstained), even if value happens to be near NEUTRAL
    assert!(
        !scores.abstentions.contains(&"phi".to_string()),
        "PHI should never be in abstentions — DeterministicDog always judges PHI. \
         phi={}, abstentions={:?}",
        scores.phi,
        scores.abstentions
    );
    // CULTURE should always be abstained
    assert!(
        scores.abstentions.contains(&"culture".to_string()),
        "culture should always be abstained"
    );
}
```

- [ ] **Step 2: Run test — may pass or fail depending on exact PHI value**

Run: `cargo test --release -p cynic-kernel -- phi_near_neutral 2>&1 | tail -10`
Note: This might pass if PHI doesn't land on NEUTRAL for this input. But the code is still fragile — the fix makes it correct by construction.

- [ ] **Step 3: Replace value-based abstention with flag-based**

Replace the abstention block at the end of `evaluate()` (lines ~293-302):
```rust
// Track which axioms this Dog abstained on (returned NEUTRAL).
// Abstention ≠ disagreement — excluded from spread calculation in judge.rs.
let mut abstentions = Vec::new();
if (fidelity - NEUTRAL).abs() < 0.001 {
    abstentions.push("fidelity".into());
}
if (verify - NEUTRAL).abs() < 0.001 {
    abstentions.push("verify".into());
}
if (culture - NEUTRAL).abs() < 0.001 {
    abstentions.push("culture".into());
}
```

With flag-based abstention tracked at each axiom's scoring site. In each section:

**FIDELITY** — change the scoring to:
```rust
let (fidelity, fidelity_abstained) = if absolutes_count >= 3 && hedging_count == 0 {
    (0.20, false)
} else {
    (NEUTRAL, true)
};
let fidelity_reason = if !fidelity_abstained {
    format!("Red flag: {absolutes_count} absolute claims, no hedging.")
} else {
    "Abstaining — fidelity requires semantic understanding.".into()
};
```

**VERIFY** — change to:
```rust
let (verify, verify_abstained) = if formal_notation_count >= 2 {
    ((NEUTRAL + ADJUST_MEDIUM).min(PHI_INV), false)
} else {
    (NEUTRAL, true)
};
let verify_reason = if !verify_abstained {
    format!("Found {formal_notation_count} formal notation tokens — verifiable.")
} else {
    "Abstaining — verification requires domain knowledge.".into()
};
```

**CULTURE** — change to:
```rust
let culture = NEUTRAL;
let culture_abstained = true;
let culture_reason: String = "Abstaining — cultural assessment requires domain knowledge.".into();
```

Then replace the old abstention block with:
```rust
let mut abstentions = Vec::new();
if fidelity_abstained {
    abstentions.push("fidelity".into());
}
if verify_abstained {
    abstentions.push("verify".into());
}
if culture_abstained {
    abstentions.push("culture".into());
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --release -p cynic-kernel -- deterministic 2>&1 | tail -20`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/dogs/deterministic.rs
git commit -m "fix(dog): abstention tracked by flag, not value coincidence

Each axiom declares abstention at scoring time via boolean flag.
Prevents false abstention when PHI/BURN accidentally land on NEUTRAL.
Abstention = decision, not output comparison."
```

---

### Task 7: Density-based coercion + symmetric agency (Fix 6)

**Files:**
- Modify: `cynic-kernel/src/dogs/deterministic.rs`

- [ ] **Step 1: Write RFC and density tests**

```rust
#[tokio::test]
async fn rfc_keywords_not_coercive() {
    let dog = DeterministicDog;
    let stimulus = Stimulus {
        content: "Implementations MUST support this format. Clients MUST send valid headers. Servers MUST respond.".into(),
        context: None,
        domain: None,
    };
    let scores = dog.evaluate(&stimulus).await.unwrap();
    // All-caps MUST = RFC 2119 normative, not coercive
    assert!(
        scores.sovereignty >= SOVEREIGNTY_BASE - ADJUST_SMALL,
        "RFC MUST keywords should not heavily penalize sovereignty, got {}",
        scores.sovereignty
    );
}

#[tokio::test]
async fn low_coercion_density_minimal_penalty() {
    let dog = DeterministicDog;
    let stimulus = Stimulus {
        content: "The client must handle errors gracefully. The server processes requests and returns appropriate status codes for each endpoint in the system.".into(),
        context: None,
        domain: None,
    };
    let scores = dog.evaluate(&stimulus).await.unwrap();
    // 1 "must" in ~22 words = density ≈ 0.045, excess ≈ 0.5 → moderate penalty
    assert!(
        scores.sovereignty > 0.25,
        "single 'must' in long text should not severely penalize, got {}",
        scores.sovereignty
    );
}
```

- [ ] **Step 2: Run tests — verify they FAIL**

Run: `cargo test --release -p cynic-kernel -- rfc_keywords 2>&1 | tail -5`
Expected: FAIL — current code penalizes all "must" regardless of case.

- [ ] **Step 3: Replace sovereignty scoring**

Replace the entire sovereignty section (~lines 273-289):
```rust
// ── SOVEREIGNTY: FORM judge — coercion vs agency ───────
let mut sovereignty: f64 = 0.40;
// Proportional coercion penalty (not just boolean)
if coercion_count > 0 {
    sovereignty -= 0.10 * (coercion_count as f64).min(3.0);
}
if agency_count > 0 {
    sovereignty += 0.05 * (agency_count as f64).min(3.0);
}
let sovereignty = sovereignty.clamp(0.05, PHI_INV);
```

With:

```rust
// ── SOVEREIGNTY: FORM judge — coercion vs agency ───────
// RFC 2119 uppercase keywords (MUST, SHALL, REQUIRED) are normative,
// not coercive — exclude from count. Ref: Bradner (1997), RFC 2119.
let coercion_count = lower_words
    .iter()
    .zip(words.iter())
    .filter(|(lower, original)| {
        let is_coercive = matches!(
            lower.as_str(),
            "must" | "mandatory" | "forced" | "required" | "compulsory" | "obey"
        );
        let is_rfc_keyword = original
            .chars()
            .all(|c| c.is_uppercase() || !c.is_alphabetic());
        is_coercive && !is_rfc_keyword
    })
    .count();
let agency_count = lower_words
    .iter()
    .filter(|w| {
        matches!(
            w.as_str(),
            "choose" | "option" | "alternative" | "freedom" | "decide" | "prefer" | "consider"
        )
    })
    .count();

// Symmetric density-based scoring. Both directions use the same formula.
// Ref: Kratzer (1977, 1981) — deontic disambiguation needs POS tagger.
// Density avoids false positives on isolated technical "must".
let coercion_density = coercion_count as f64 / word_count.max(1) as f64;
let agency_density = agency_count as f64 / word_count.max(1) as f64;

let mut sovereignty: f64 = SOVEREIGNTY_BASE;
if coercion_density > COERCION_DENSITY_THRESHOLD {
    let excess =
        (coercion_density - COERCION_DENSITY_THRESHOLD) / COERCION_DENSITY_THRESHOLD;
    sovereignty -= ADJUST_MEDIUM * excess.min(3.0);
}
if agency_density > AGENCY_DENSITY_THRESHOLD {
    let excess =
        (agency_density - AGENCY_DENSITY_THRESHOLD) / AGENCY_DENSITY_THRESHOLD;
    sovereignty += ADJUST_SMALL * excess.min(3.0);
}
let sovereignty = sovereignty.clamp(0.05, PHI_INV);
```

Also remove the OLD `coercion_count` and `agency_count` computations from the signal detection section (~lines 87-110) since they are now inside the SOVEREIGNTY section.

- [ ] **Step 4: Update sovereignty_reason**

Replace the sovereignty_reason block with:

```rust
let sovereignty_reason = if coercion_density > COERCION_DENSITY_THRESHOLD {
    format!(
        "{coercion_count} coercive term(s), density {:.1}% — limits agency.",
        coercion_density * 100.0
    )
} else if agency_density > AGENCY_DENSITY_THRESHOLD {
    format!(
        "{agency_count} agency signal(s), density {:.1}% — preserves choice.",
        agency_density * 100.0
    )
} else {
    "Neutral — no concentrated coercion or agency signals.".into()
};
```

- [ ] **Step 5: Run all tests**

Run: `cargo test --release -p cynic-kernel -- deterministic 2>&1 | tail -20`
Expected: all tests pass. Verify specifically:
- `penalizes_coercion` still passes (high density)
- `rewards_agency` still passes (high density)
- `rfc_keywords_not_coercive` passes (RFC exclusion)
- `low_coercion_density_minimal_penalty` passes (excess formula)

- [ ] **Step 6: Commit**

```bash
git add cynic-kernel/src/dogs/deterministic.rs
git commit -m "fix(dog): density-based coercion+agency, RFC 2119 exclusion

Coercion and agency both use symmetric density+excess formula.
Single 'must' in long text = noise. RFC uppercase MUST excluded.
Ref: Kratzer (1977), RFC 2119 (Bradner, 1997)."
```

---

### Task 8: Domain prompt heading skip (Fix 8)

**Files:**
- Modify: `cynic-kernel/src/infra/config.rs:403-415`

- [ ] **Step 1: Extract heading-skip into a pure function and write tests**

First, add a `strip_domain_heading` function in `config.rs` (near `load_domain_prompts`):

```rust
/// Strip the H1 title from domain prompt content, preserving all H2+ sections.
/// Pure function — no I/O. Extracted for testability (K3/K13).
fn strip_domain_heading(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let first_content = lines.iter().find(|l| !l.trim().is_empty());
    // Space after `#` required: `# Title` is H1, `## FIDELITY` is H2 (must not skip).
    let skip_heading =
        first_content.is_some_and(|l| l.starts_with("# ") && !l.starts_with("## "));
    content
        .lines()
        .skip_while(|l| l.trim().is_empty())
        .skip(if skip_heading { 1 } else { 0 })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}
```

Then add the tests:

```rust
#[cfg(test)]
mod tests_heading_skip {
    use super::strip_domain_heading;

    #[test]
    fn skips_h1_title() {
        let input = "# Chess Domain\n## FIDELITY\nIs this faithful?";
        assert!(strip_domain_heading(input).starts_with("## FIDELITY"));
    }

    #[test]
    fn keeps_h2_when_no_title() {
        let input = "## FIDELITY\nIs this faithful?";
        assert!(strip_domain_heading(input).starts_with("## FIDELITY"));
    }

    #[test]
    fn skips_blank_lines_and_title() {
        let input = "\n\n# Title\n## FIDELITY\nContent";
        assert!(strip_domain_heading(input).starts_with("## FIDELITY"));
    }

    #[test]
    fn keeps_h2_after_blank_lines() {
        let input = "\n\n## FIDELITY\nContent";
        assert!(strip_domain_heading(input).starts_with("## FIDELITY"));
    }
}
```

- [ ] **Step 2: Run tests — verify `keeps_h2_when_no_title` FAILS**

Run: `cargo test --release -p cynic-kernel -- tests_heading_skip 2>&1 | tail -15`
Expected: `keeps_h2_when_no_title` FAILS — the function uses the old `starts_with('#')` logic initially. Wait — actually the function IS the fix. We need to write the buggy version first for red.

Alternative: add the test first, then add the `strip_domain_heading` function in step 3. For step 1, just add the test module that calls `strip_domain_heading` (which doesn't exist yet). The test will fail to compile — that's the red step.

Actually, simplest approach: add the function AND the tests in one step, but have the function call the EXISTING buggy inline code. Then fix the function in step 3.

Simplest correct approach: write the tests calling `strip_domain_heading`, let it fail to compile (red), then implement the correct function (green).

- [ ] **Step 3: Implement strip_domain_heading and wire into load_domain_prompts**

Add the `strip_domain_heading` function as shown in Step 1 (with the CORRECT logic).

Then in `load_domain_prompts`, replace lines 403-415:
```rust
                // Strip the first heading line (# Domain Name — ...)
                let prompt: String = content
                    .lines()
                    .skip_while(|l| l.trim().is_empty())
                    .skip(if content.trim_start().starts_with('#') {
                        1
                    } else {
                        0
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
                    .trim()
                    .to_string();
```

With:
```rust
                let prompt = strip_domain_heading(&content);
```

- [ ] **Step 4: Run tests**

Run: `cargo test --release -p cynic-kernel -- tests_heading_skip 2>&1 | tail -15`
Expected: all 4 heading tests pass.

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/infra/config.rs
git commit -m "fix(config): domain prompt heading skip checks H1 not H2

starts_with('#') matched both '# Title' and '## FIDELITY'. Now uses
'# ' with space + '## ' exclusion. Heading check from iterator state,
not original string. Spotted by Pocks."
```

---

### Task 9: Full gate verification

- [ ] **Step 1: Run make check**

Run: `make check 2>&1 | tail -30`
Expected: build + test + clippy + lint-rules + lint-drift + lint-security + audit all pass.

- [ ] **Step 2: Verify test count increased**

Run: `cargo test --release -p cynic-kernel 2>&1 | tail -3`
Expected: test count increased from 358 to ~375+ (new tests from all fixes).

- [ ] **Step 3: Commit if any formatting changes from hooks**

Check `git status`. If rustfmt hook modified anything:
```bash
git add -u && git commit -m "style: rustfmt after deterministic dog fixes"
```

---

## Sequencing Summary

```
Task 1: Constants (Fix 4)      — foundation, no behavioral change
Task 2: Notation registry (7)  — structural, enables Fix 5
Task 3: Context fix (Fix 1)    — HIGH priority bug
Task 4: SBD (Fix 3)            — MEDIUM priority bug
Task 5: Hedging split (Fix 5)  — depends on Task 2 (formal_notation_count)
Task 6: Abstention flags (2)   — MEDIUM priority bug
Task 7: Density coercion (6)   — depends on Task 1 (constants)
Task 8: Heading skip (Fix 8)   — independent, different file
Task 9: Gate verification      — final check
```

Tasks 1-7 are sequential (same file, overlapping regions).
Task 8 is independent (different file) — can run in parallel with any task.
