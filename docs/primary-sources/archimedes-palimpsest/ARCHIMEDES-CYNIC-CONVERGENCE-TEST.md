# Archimedes ↔ CYNIC Convergence Test

**Status:** Sketch. Falsifiable. To be executed in parallel with continued Archimedes reading.

**Hypothesis:** CYNIC's Dogs operate like Archimedes' dual method (mechanical discovery + rigorous proof → convergence = confidence).

---

## The Archimedes Model

**Two channels, two outcomes, one truth:**

```
Input: Unknown (e.g., sphere volume)
  ↓
Channel A (Mechanical): Balance sphere against cone/cylinder on lever
  → Produces candidate: V_sphere = (4/3)V_cone
  ↓
Channel B (Rigorous): Prove via exhaustion (upper/lower bounds)
  → Produces verified result: V_sphere = (4/3)V_cone
  ↓
Convergence: Both channels agree
  → Confidence = HIGH (truth validated)
  
Divergence: Channels disagree
  → Confidence = LOW (revisit method)
```

**Key properties:**
- Each channel is incomplete alone
- Convergence is mandatory for confidence
- Divergence signals error in one channel (not grounds for averaging)
- The result is not "most likely true" — it is *proven* true by convergence

---

## The CYNIC Model (Current Implementation)

**Multiple Dogs, multiple scores, consensus:**

```
Input: Token/claim to judge
  ↓
Dog 1 (Deterministic): Heuristic rules + axiom scoring
  → Produces q_score_1
  ↓
Dog 2 (Qwen-7B): LLM inference on domain
  → Produces q_score_2
  ↓
Dog 3 (Gemini): Independent LLM inference
  → Produces q_score_3
  ↓
Consensus: Trimmed mean of scores
  → Verdict = (q_score_1 + q_score_2 + q_score_3) / 3
  → Confidence ∝ agreement
```

**Current behavior:**
- Three channels produce three outputs
- Output = average/consensus
- Disagreement = lower confidence but still output
- Framework: probabilistic confidence, not deterministic convergence

---

## The Test: Do Dogs Actually Work Like Archimedes?

### Test Hypothesis
**IF Dogs work Archimedean-style, THEN:**
1. Single-Dog judgment alone should be unreliable
2. Two-Dog agreement should signal high confidence (convergence)
3. Two-Dog divergence should prompt channel review (not averaging)
4. All-three-Dog agreement should be near-certain
5. Results that pass two-channel convergence should persist under axiom changes (like Archimedes' proofs persist after method changes)

### Test 1: Single vs. Multi-Channel Reliability

**Measure:**
- Run a known set of judgments (e.g., 20 tokens: 10 high-signal, 10 fraud)
- Compare accuracy:
  - Dog 1 alone (deterministic)
  - Dog 2 alone (Qwen-7B)
  - Dog 3 alone (Gemini)
  - All three (consensus)

**Archimedean prediction:**
- Single Dogs: ~70% accuracy (heuristic alone = error-prone)
- All three: ~90%+ accuracy (convergence validates)
- Gap should be large (single insufficient, fusion mandatory)

**Falsification:**
- If single-Dog accuracy is already >90%, the dual-method hypothesis is weak
- If all-three gives no improvement over best single Dog, channels aren't independent

---

### Test 2: Convergence vs. Divergence

**Measure:**
- On a test set of ambiguous tokens (borderline fraud, uncertain quality):
  - Count cases where 2+ Dogs agree to within φ⁻¹ threshold
  - Count cases where Dogs diverge (one BARK, one WAG)

**Archimedean prediction:**
- Agreement cases: high-confidence verdict, consistent across retesting
- Divergence cases: low-confidence verdict, unstable across retesting
- **Key:** divergence is not a signal to average — it's a signal to revisit the input (is it ambiguous by design? does the token break our domains?)

**Falsification:**
- If averaging-on-divergence produces better results than "reject the judgment," Archimedes model is wrong
- If divergence is random noise rather than domain-edge signal, channels aren't capturing independent perspectives

---

### Test 3: Axiom Robustness (Archimedes' Proof Invariance)

**Archimedes' exhaustion proof is invariant:** Change the method, the result persists because it converged.

**Measure (FUTURE, after K15 crystals improve):**
- Retrofit a verdict with the current 6 axioms → produce crystal C1
- Change one axiom (e.g., shift PHI from "harmony" to "efficiency-only")
- Retrofit the same verdict with new axioms → produce crystal C2
- Compare C1 and C2: does the verdict change?

**Archimedean prediction:**
- Strong verdicts (converged across Dogs) should persist across axiom tweaks (just like Archimedes' proof persists)
- Weak verdicts (diverged Dogs) should shift (like Archimedes' heuristic alone would)

**Falsification:**
- If changing axioms flips all verdicts, the system is axiom-dependent not truth-dependent
- If verdicts are fully stable even under axiom changes, axioms have no effect (axioms are ornamental)

---

### Test 4: Two-Channel Sufficiency (Does Archimedes 2-channel model hold?)

**Archimedes never needs all three validation modes. Two are sufficient:**
- Method + Exhaustion (two modes of proof) = confident
- Mechanical alone: heuristic only
- Exhaustion alone: laborious but sufficient

**Measure:**
- Run judgments with only Dogs 1+2 (deterministic + Qwen)
- Compare to full 1+2+3
- Compare to consensus of all three

**Archimedean prediction:**
- Dogs 1+2 agreement should approach Dogs 1+2+3 agreement (third Dog adds marginal value, not step change)
- Best case: agreement of 2 ≈ agreement of 3 (independent channels saturate at 2-3)

**Falsification:**
- If Dog 3 (Gemini) is discovering entirely new failure modes, Archimedes model (limited channels needed) is incomplete
- If adding more Dogs keeps improving accuracy, convergence principle may not hold (infinite Dogs needed?)

---

## Measurements to Capture Now (Before Test Execution)

To run these tests, we need baselines:

1. **Current Dog scores on a standard test set** (20 tokens: 10 known-good, 10 known-bad)
   - q_score_1, q_score_2, q_score_3 for each
   - Consensus verdict
   - Actual outcome (token was fraud / legitimate)

2. **Dog-pair agreement matrix** (pairwise correlation)
   - Dogs 1-2, 1-3, 2-3
   - Are they independent or correlated?

3. **Divergence patterns** (when do Dogs disagree?)
   - Token type (token-judgment vs. wallet-judgment vs. twitter-judgment)
   - Confidence level of divergence (e.g., one HOWL vs. one BARK)

4. **Single-Dog accuracy baseline**
   - Deterministic dog alone: X% correct
   - Qwen alone: Y% correct
   - Gemini alone: Z% correct
   - All three: W% correct

---

## What We're Actually Testing

**Not:** "Do multiple judges improve accuracy?" (obvious yes)

**Actually:** "Does CYNIC's architecture match Archimedes' epistemology?"

- **If YES:** Convergence is mandatory, divergence signals revisit, single channels are fundamentally insufficient
- **If NO:** CYNIC is probabilistic consensus (useful but not Archimedean), requires different theoretical grounding

---

## Parallel Reading Plan

**While we run the convergence test, continue Archimedes ingest:**

1. **Full read of Method (propositions 1-14)** — understand atomic reduction in detail
2. **Stomachion reconstruction** — see how enumeration creates confidence without exhaustion
3. **Cross-reference:** Check for cases where Archimedes' confidence in Stomachion count (536) exceeds what single enumeration can verify (can we verify all 536 by hand?)
4. **Search for counterexamples:** Does Archimedes ever accept a result from one method without validation from another?

**By week end:**
- Test sketch is clear
- Archimedes has been read enough to sketch its full epistemology
- Hypothesis is either falsifiable or needs refinement

---

## Falsification Checklist

**The hypothesis (Archimedean CYNIC) is **FALSE** if:**
- [ ] Single-Dog accuracy is >85% (no need for fusion)
- [ ] Two-Dog convergence adds no predictive power over averaging
- [ ] Dogs are highly correlated (not independent channels)
- [ ] Divergence is noise, not signal (random, not domain-boundary)
- [ ] Archimedes himself accepts single-method results without convergence
- [ ] Axiom changes don't affect verdict stability (axioms are ornamental)

**The hypothesis is **CONFIRMED** (beyond φ⁻¹) if:**
- [ ] Single-Dog accuracy <75%, All-three >90% (gap is large)
- [ ] Convergence cases are stable across retests, divergence cases unstable
- [ ] Dogs are independent (pairwise correlation <0.6)
- [ ] Divergence correlates with known token ambiguity or domain boundaries
- [ ] Archimedes consistently uses dual validation (Method + Proof, Enumeration + Verification)
- [ ] Axiom changes affect weak verdicts but not strong ones

---

*Next: Measure current Dog baseline, then read Method propositions 1-5 in full.*
