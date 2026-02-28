# CYNIC Module Evaluation — Quick Reference Card

## The 5 Axioms in 1 Sentence Each

| Axiom | Question | Scoring |
|-------|----------|---------|
| **FIDELITY** | Does it keep its promise? (Design intent = reality?) | Does module's actual behavior match what it claims to do? |
| **PHI** | Is it well-proportioned? (Size justified by value?) | Is complexity/LOC ratio φ-aligned (1.6:1)? |
| **VERIFY** | Can we prove it works? (Tests, usage, evidence?) | Is there measurable proof of correctness and active use? |
| **CULTURE** | Does it fit CYNIC? (Philosophy, resonance, purpose?) | Does it align with CYNIC's identity and architecture? |
| **BURN** | Should it exist? (Value exceeds cost?) | Is it worth the maintenance burden? Could it be simpler? |

**Q-Score = Weighted Geometric Mean of 5 axioms**
- 82+ (HOWL) = Exceptional → KEEP
- 62-81 (WAG) = Good → KEEP
- 38-61 (GROWL) = Needs work → EVOLVE
- 0-37 (BARK) = Critical problem → DELETE/ISOLATE

---

## The Verdict Quick Matrix

```
┌─────────────────────────────────────────────────────────────┐
│ Module                          Score    Verdict   Action   │
├─────────────────────────────────────────────────────────────┤
│ cynic/protocol/lnsp             20/100   BARK     DELETE    │
│ cynic/training                  30/100   BARK     DELETE    │
│ cynic/organism                  59/100   GROWL    EVOLVE    │
│ cynic/metabolism                59/100   GROWL    EVOLVE    │
│                                                              │
│ cynic/cognition                 74/100   WAG      KEEP      │
│ cynic/nervous                   77/100   WAG      KEEP      │
│ cynic/senses                    72/100   WAG      KEEP      │
│ cynic/integrations              70/100   WAG      KEEP      │
│ cynic/immune                    70/100   WAG      KEEP      │
│                                                              │
│ cynic/core                      88/100   HOWL     KEEP      │
│ cynic/judges                    90/100   HOWL     KEEP      │
│ cynic/consensus                 90/100   HOWL     KEEP      │
│ cynic/learning                  84/100   WAG      KEEP      │
└─────────────────────────────────────────────────────────────┘
```

---

## Red Flags (When Module Scores Low)

### Score < 30 (BARK - Delete immediately)
- [ ] **Zero active imports** (no one uses it)
- [ ] **Never deployed to production** (purely speculative)
- [ ] **Duplicates existing functionality** (redundant)
- [ ] **No tests** (can't verify it works)
- [ ] **Unowned** (no team member advocates for it)

**Example:** `cynic/protocol/lnsp` (20/100)
- 0 imports ❌
- Never deployed ❌
- Duplicates nervous.EventJournal ❌
- 1 test file, isolated ❌
- Decision: DELETE immediately

### Score 30-40 (BARK - Mark for deletion)
- [ ] **Low usage** (5> imports is concerning)
- [ ] **Experimental phase over** (Phase 1B artifacts)
- [ ] **Strategic pivot** (approach changed)
- [ ] **Zero tests** (high risk)
- [ ] **No clear owner** (who maintains this?)

**Example:** `cynic/training` (30/100)
- 1 self-reference ❌
- Phase 1B relic ❌
- Claude API now used instead ❌
- 0 tests ❌
- Decision: DELETE to archive branch

### Score 40-60 (GROWL - Needs work)
- [ ] **Unverified value** (does it actually help?)
- [ ] **Over-complex for function** (bloated?)
- [ ] **Minimal testing** (<5% test coverage)
- [ ] **Unclear purpose** (would new developer understand?)
- [ ] **Low usage** (10-30 imports)

**Example:** `cynic/organism` (59/100)
- 10 layers; 3 essential? ❌
- 29 imports (used but not core) ⚠️
- 0 tests ❌
- Metaphor confuses (wrapper overhead) ⚠️
- Decision: EVOLVE — flatten to 3-4 layers

### Score 60+ (WAG - Healthy)
- [ ] **Active usage** (20+ imports)
- [ ] **Clear purpose** (documented)
- [ ] **Reasonable test coverage** (5%+ of LOC)
- [ ] **Fits architecture** (aligns with philosophy)
- [ ] **Recent commits** (actively maintained)

---

## The Real Test: Ask This to Any Module

1. **FIDELITY:** "If I read your design docs, does the code match? Or did you drift?"
   - Module: ❌ Drifted → Low FIDELITY score

2. **PHI:** "Would I build you this way today? Or is there bloat?"
   - Module: ❌ Too much code for what it does → Low PHI score

3. **VERIFY:** "Can I prove you work with actual tests? Or is it on faith?"
   - Module: ❌ No tests, unclear correctness → Low VERIFY score

4. **CULTURE:** "Do you feel like CYNIC? Or like you're from a different system?"
   - Module: ❌ Feels foreign, doesn't fit philosophy → Low CULTURE score

5. **BURN:** "Would I delete you tomorrow if times got tight? Or am I genuinely committed?"
   - Module: ❌ First thing to cut if budget tight → Low BURN score

**If more than 2 axioms score <50, the module is failing.**

---

## Why These Scores Matter

### Good Code vs. Bad Code (Axiom Perspective)

| Aspect | Good (WAG+) | Bad (BARK) |
|--------|-----------|-----------|
| **FIDELITY** | Design matches reality | Promise ≠ reality |
| **PHI** | Elegant, minimal bloat | Over-engineered, messy |
| **VERIFY** | Tested, can prove it works | Untested, faith-based |
| **CULTURE** | Feels like CYNIC | Feels like foreign import |
| **BURN** | Worth maintaining | Dead weight |
| **User Experience** | Clear purpose, easy to use | Confusing, hard to justify |
| **Maintenance Cost** | Reasonable investment | Dragging down velocity |

---

## Decision Trees

### "Should we keep this module?"

```
                    START
                      |
          Does it have tests? (>5% coverage)
           /                            \
         YES                            NO
          |                              |
          |                    "Why isn't this tested?"
          |                      /           |
          |              Know why?     Too new/experimental
          |              /        \        |
          |        Good reason  Oversight  |
          |        /             |        |
          |      OK            Add tests  Consider deleting
          |       |             /         (hard to maintain)
          |       |            /
          |       |      Tests pass?
          |       |      /        \
          |      YES             NO
          |       |              |
    Is it used? (10+ imports)    FIX or DELETE
     /        \
   YES        NO
    |          |
   KEEP    Dead code?
    |      /        \
    |    YES        Maybe
    |     |          |
    |   DELETE   Will use
    |            later?
    |            /        \
    |          YES        NO
    |           |          |
    |        ARCHIVE    DELETE
    |
    v
   KEEP (with reviews)
```

### "Is this module scoring too low?"

```
Q-Score < 50?
   |
   +---> FIDELITY < 40? → Design drifted; fix or rewrite
   |
   +---> PHI < 40? → Bloated; refactor
   |
   +---> VERIFY < 40? → Untested; add tests or delete
   |
   +---> CULTURE < 40? → Doesn't fit; isolate or rewrite
   |
   +---> BURN < 40? → Dead weight; delete
   |
   +---> Multiple < 50? → CRITICAL: Mark for remediation
```

---

## The Covenant: How We Judge Modules Going Forward

**When we add a new module, it must:**

1. **Have a clear owner** (named person responsible)
2. **Document its purpose** in `/cynic/MODULE/__init__.py`
3. **Have >5% test coverage** (tests for key logic)
4. **Score ≥ WAG (62)** on all 5 axioms before merge
5. **Be used by ≥2 other modules** or have clear use case

**If a module drops below 40 on any single axiom:**
- Mark for quarterly review
- Owner must document remediation plan
- If still <40 next quarter, delete or isolate

**If a module has 0 imports after 3 months:**
- Owner must justify continued existence
- Archive or delete

---

## Reading This Document Correctly

### If you're a module owner
→ Find your module in evaluation
→ Read the detailed breakdown
→ Understand why you scored what you scored
→ Fix the failing axioms (or accept deletion)

### If you're evaluating a NEW module
→ Create a scorecard (template below)
→ Score each axiom (0-100)
→ Compute Q-Score (weighted geometric mean)
→ If < 62, don't merge without remediation

### If you're deleting a module
→ Document the reason (which axiom failed?)
→ Move code to `archive/` branch
→ Add a comment in codebase: "Module X was deleted because of Y"
→ Thank the module for its service

### If you're reviving old code
→ Score it on 5 axioms first
→ If < 40, rebuild from scratch
→ If 40-60, modernize and add tests
→ If 60+, bring it back carefully

---

## Axiom Scorecard Template

```markdown
# Module Evaluation: [MODULE_NAME]

**LOC:** [count]
**Tests:** [count] lines
**Usage:** [count] imports
**Owner:** [name]

## Scores

| Axiom | Score | Notes |
|-------|-------|-------|
| FIDELITY | __/100 | Design intent vs. reality? |
| PHI | __/100 | Proportion & elegance? |
| VERIFY | __/100 | Tests & evidence? |
| CULTURE | __/100 | Fits CYNIC philosophy? |
| BURN | __/100 | Worth maintaining? |

**Q-Score:** __/100

**Verdict:** KEEP / EVOLVE / ISOLATE / DELETE

**Reasoning:** [1 paragraph explaining decision]

**Action Items:**
- [ ] Item 1
- [ ] Item 2
- [ ] Item 3
```

---

## The Bottom Line

**CYNIC's codebase is 108,306 lines.**

**After evaluation:**
- 68% is excellent (HOWL/WAG) ✅
- 20% needs work (GROWL) ⚠️
- 12% should be deleted (BARK) ❌

**Actions taken:**
- Delete LNSP (3,275 LOC) ❌
- Delete training (2,250 LOC) ❌
- Evolve organism (3,950 LOC) ⚠️
- Validate metabolism (1,435 LOC) ⚠️

**Net result:**
- Codebase improves from 108,306 → 98,396 LOC (9% reduction)
- Cognitive overhead decreases significantly
- Each module has clear, understood purpose
- Tests improve from 265 → 400+ over time

---

**Use this framework for every decision.**
**When in doubt, ask the 5 axioms.**
**They will tell you the truth.**
