# CYNIC Domain Registry

**Single source of truth for domain definitions, routing, and gaps**

Last updated: 2026-04-30 16:00  
Scope: Authoritative for May 3 launch and beyond

---

## Problem Statement

Domains are scattered across three inconsistent taxonomies:

1. **Python `domains/__init__.py`** (authoritative per code):
   - D1: Solana/Tokens
   - D2: Inference/LLM
   - D3: Sovereignty
   - D4: Security/Scams
   - D5: Macro/Politics
   - D6: Epistemology/Philosophy

2. **Hermes X Organ (RELABELED 2026-04-30)**:
   - D1_curated.jsonl: Token (97.3K) ✓
   - HERMES_TWITTER_curated.jsonl: Twitter/Social (311.6K) (was D2)
   - HERMES_CHESS_curated.jsonl: Chess (7.5K) (was D4)
   - HERMES_WALLET_curated.jsonl: Wallet (56.3K) (was D5)
   - D6_curated.jsonl: Other (4.8K) ✓ (kept — epistemology placeholder)

3. **Kernel routing**:
   - Domain is optional hint, NOT routing dispatcher
   - No explicit "if domain == X use Dog Y" branching
   - Stimulus builders exist for: chess, wallet (explicit)
   - No routing for: token, twitter, security, macro, philosophy

---

## RESOLVED: Canonical Domain Taxonomy (May 3+)

**Adopt Python's taxonomy as authoritative. Hermes X to be relabeled.**

| ID | Name | Status | Kernel Support | Dogs | Python Consumer | Hermes X Data | Gap/Blocker |
|----|------|--------|-----------------|------|-----------------|---------------|-------------|
| **D1** | **Solana/Token** | LIVE | ✓ stimulus | deterministic-dog, qwen-7b, qwen-9b | token_heuristics.py | D1_curated (97K) | None — ready |
| **D2** | **Inference/LLM** | LIVE | ✓ stimulus? | (embedded in kernel) | (NA) | D2_curated (311K) labeled as Twitter | **MISMATCH: Hermes X calls twitter "D2", Python calls it "D2 Inference"** |
| **D3** | **Sovereignty** | **BLOCKED** | ✗ | (none) | (axiom grounding only) | (0 signals) | **CRITICAL GAP: No Dogs, no data, no consumer** |
| **D4** | **Security/Scams** | LIVE | ✓ stimulus | (rug detection in D1?) | (security heuristics?) | D4_curated (7.5K) labeled as Chess | **MISMATCH: Hermes X calls chess "D4"** |
| **D5** | **Macro/Politics** | NOISY | ? | (market context?) | (unclear) | D5_curated (56K) labeled as Wallet | **MISMATCH: Hermes X calls wallet "D5"** |
| **D6** | **Epistemology/Philosophy** | STUB | ✗ | (none) | (calibration only) | D6_curated (4.8K) labeled as Other | **STUB: Labeled but no Dogs** |

---

## Immediate Actions (Before May 3)

### 1. Clarify Hermes X Relabeling
**Question:** Is Hermes X's "D2=Twitter" actually capturing Inference/LLM signals, or is it mislabeled?

**If mislabeled** (most likely):
- Rename: `D2_curated.jsonl` → `HERMES_TWITTER.jsonl` (transient naming)
- Relabel in curation: `"domain": "D2"` → `"domain": "HERMES_TWITTER"` (do not use D2 until clarified)
- Add note: "Hermes X uses different domain taxonomy than kernel. Consolidation pending."

**If actually Inference signals** (unlikely):
- Keep as D2
- Update HERMES_ARCHITECTURE.md to clarify

### 2. Create Routing Dispatcher in Kernel
**New file:** `cynic-kernel/src/domain/dispatch.rs`

```rust
pub fn route_by_domain(domain: &str) -> DogSet {
    match domain {
        "token" | "solana" | "D1" => DogSet::Token,
        "inference" | "llm" | "D2" => DogSet::Inference,
        "security" | "scam" | "rug" | "D4" => DogSet::Security,
        "wallet" | "sybil" | "D5" => DogSet::Wallet,
        "chess" | "game" | "personality" => DogSet::Chess,
        "sovereignty" | "D3" => DogSet::Sovereign,
        "philosophy" | "epistemology" | "D6" => DogSet::Philosophy,
        _ => DogSet::Default,
    }
}
```

**Status:** Unblocks domain-aware routing. Maps kernel domain field to actual Dog selection.

### 3. Define DogSets (Which Dogs per Domain)

| Domain | Dogs | Rationale | Status |
|--------|------|-----------|--------|
| **Token (D1)** | deterministic-dog, qwen-7b, qwen-9b | Rug detection, launch patterns, token auth | ✓ LIVE |
| **Inference (D2)** | (kernel-only, no LLM Dogs) | Performance, VRAM math, model selection | ✓ embedded |
| **Security (D4)** | qwen-7b, qwen-9b | Exploit patterns, social engineering, scam taxonomy | ⏳ READY (needs testing) |
| **Wallet (D5)** | deterministic-dog | Anti-Sybil, game authenticity, archetype consistency | ✓ LIVE (B&C integration) |
| **Chess** | deterministic-dog | Personality card validation, move pattern analysis | ✓ LIVE (B&C integration) |
| **Sovereignty (D3)** | (TBD — need new Dog) | Infrastructure independence, epistemic authority | ✗ BLOCKED |
| **Philosophy (D6)** | (TBD — axiom calibration) | Truth definition, confidence bounds, falsification | ✗ STUB |

---

## K15 Wiring (Producer → Consumer)

**Before May 3, verify each producer has an acting consumer:**

| Producer | Consumer | Status | Blocker |
|----------|----------|--------|---------|
| `/judge` (token) | Dogs → Verdict → Storage → CCM | ✓ LIVE | None |
| `/judge` (wallet) | Dogs → Verdict → B&C integration | ✓ READY | Awaiting S. decision (May 1) |
| `/judge` (chess) | Dogs → Verdict → B&C integration | ✓ READY | Awaiting S. decision (May 1) |
| Hermes X Twitter data | Curation → Lab briefing → Agent tasks | ⏳ PARTIAL | Briefing is 1 day old (lab.py needs cron 4h→2h) |
| Domain curation signals | Kernel wisdom/enrichment | ✓ WIRED | None — loading from jsonl |
| D3 (Sovereignty) signals | ??? | ✗ NO CONSUMER | **No axiom source, no Dogs, no routing** |
| D6 (Philosophy) signals | ??? | ✗ PARTIAL | Embedded in prompt engineering, not testable |

---

## Gaps (Do Not Launch With These)

1. **D2/D4/D5 mislabeling:** Hermes X uses D2/D4/D5 for Twitter/Chess/Wallet. Kernel expects D1-D6 for Solana/Inference/Security/Macro/Epistemology. Chaos on first `domain=D2` query.

2. **D3 (Sovereignty) missing:** 0 data, 0 Dogs, 0 consumer. The axiom that most matters to CYNIC's identity has no way to be grounded.

3. **D6 (Philosophy) stub:** Only in prompt engineering. Not measurable, not falsifiable, not learnable via crystallization.

4. **No domain routing:** Kernel treats domain as hint, not dispatcher. If client sends `domain=token`, which Dogs actually run? Undefined.

5. **Hermes X Twitter quality:** D2_curated (311K) — is this Inference/LLM signals or Twitter social signals? Unknown.

---

## Remediation (Scope for Separate Session)

- [ ] Clarify Hermes X domain semantics
- [ ] Implement kernel dispatch router
- [ ] Define DogSets per domain
- [ ] Establish K15 consumers for D3, D6
- [ ] Test end-to-end: `POST /judge {"domain": "security", "content": "..."}` routes to security Dogs
- [ ] Measurement: Before/after routing, do Dogs discriminate better?

---

## For May 3 Launch (Unblock By)

**Must decide before May 1:**

1. **Keep Hermes X labels as-is?** (Twitter as D2, Chess as D4, Wallet as D5 — breaks Python taxonomy)
   - **Pro:** Doesn't break existing code
   - **Con:** Two incompatible taxonomies forever

2. **Relabel Hermes X?** (Twitter → TRANSIENT_HERMES_D2, etc.)
   - **Pro:** Clarifies the mismatch, buys time for consolidation
   - **Con:** Requires curation script change

3. **Launch with domain optional** (don't require domain in May 3 experiments)?
   - **Pro:** Avoids routing ambiguity
   - **Con:** No domain discovery in experiment data

---

## Resources

- Python canonical taxonomy: `cynic-python/domains/__init__.py`
- Hermes X data: `~/.cynic/organs/hermes/x/curated/*.jsonl`
- Kernel judge: `cynic-kernel/src/api/rest/judge.rs`
- Stimulus builders: `cynic-kernel/src/domain/stimulus.rs`
- Wisdom enrichment: `cynic-kernel/src/domain/wisdom/mod.rs`

---

**Owner:** T. (decision on taxonomy consolidation)  
**Blocker for May 3 launch:** Hermes X relabeling decision + kernel dispatcher implementation
