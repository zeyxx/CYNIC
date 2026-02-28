# CYNIC Codebase Evaluation Framework — Getting Started

**Created:** 2026-02-27 by Claude Code
**Purpose:** Distinguish good ideas from bad ones across the CYNIC codebase
**Scope:** 108,306 lines of Python; 32 major modules

---

## 🎯 What This Framework Does

Answers the question: **"LNSP was interesting, but how do we separate truth from false across the codebase? What stays, what goes, what evolves?"**

Using **CYNIC's 5 Core Axioms** (FIDELITY, PHI, VERIFY, CULTURE, BURN), we evaluate every module and provide:
- Clear scores (0-100 Q-Score)
- Specific verdicts (KEEP / EVOLVE / DELETE / ISOLATE)
- Actionable recommendations
- Implementation timeline

---

## 📚 Three Documents (Read in This Order)

### 1. **CODEBASE_EVALUATION_FRAMEWORK.md** (1,062 lines)
**What:** Comprehensive analysis of all questionable modules
**Who:** Read this if you want deep understanding
**Time:** 30-45 minutes

**Contains:**
- Full axiom definitions (FIDELITY, PHI, VERIFY, CULTURE, BURN)
- Detailed evaluation of 9 questionable modules:
  - BARK (delete): lnsp, training
  - GROWL (evolve): organism, metabolism
  - WAG (keep): cognition, nervous, senses, integrations, immune
- Scoring methodology
- Summary evaluation table
- Critical actions section

**Start here if:** You want to understand the reasoning behind each verdict

---

### 2. **EVALUATION_QUICK_REFERENCE.md** (308 lines)
**What:** Quick lookup card for the framework
**Who:** Read this for fast decision-making
**Time:** 10 minutes

**Contains:**
- 5 Axioms in 1 sentence each
- Quick verdict matrix (all 14 modules)
- Red flags that indicate low scores
- Decision trees ("Should we keep this?")
- Axiom scorecard template
- The real test: 5 questions to ask any module

**Start here if:** You want the summary without deep analysis

---

### 3. **ACTION_PLAN.md** (446 lines)
**What:** Step-by-step implementation guide
**Who:** Read this when you're ready to execute
**Time:** 15 minutes (planning); several days (execution)

**Contains:**
- Immediate actions (delete LNSP & training)
- High-priority actions (evolve organism & metabolism)
- Medium-priority actions (expand test coverage)
- Verification checklist
- Expected outcomes
- Timeline (4-week plan)

**Start here if:** You're ready to clean up the codebase

---

## 🚀 Quick Start (5 Minutes)

1. **Read EVALUATION_QUICK_REFERENCE.md** (5 min)
   - Understand the 5 axioms
   - See the verdict matrix
   - Learn the red flags

2. **Find your module** in the matrix
   - Score 80+? → KEEP (no action needed)
   - Score 60-79? → KEEP (monitor for improvements)
   - Score 40-59? → EVOLVE (needs work)
   - Score <40? → DELETE (remove or archive)

3. **If you own a low-scoring module:**
   - Read detailed evaluation in CODEBASE_EVALUATION_FRAMEWORK.md
   - Find your axiom breakdown
   - Understand which axiom(s) failed
   - Check ACTION_PLAN.md for remediation steps

---

## 🎓 Understanding the 5 Axioms

| Axiom | Meaning | Example |
|-------|---------|---------|
| **FIDELITY** | Does it keep its promise? | Design says distributed nervous system; implementation is monolithic → FAIL |
| **PHI** | Is it well-proportioned? | 3,275 LOC for unused spec → BLOATED → FAIL |
| **VERIFY** | Can we prove it works? | Zero imports, no tests, never deployed → UNVERIFIED → FAIL |
| **CULTURE** | Does it fit CYNIC's philosophy? | Spec contradicts organism architecture → MISFIT → FAIL |
| **BURN** | Should it exist? | Dead weight draining attention → NOT WORTH IT → FAIL |

**Scoring:** 0-100 scale per axiom
- 82+ = HOWL (exceptional) ✅ KEEP
- 62-81 = WAG (good) ✅ KEEP
- 38-61 = GROWL (needs work) ⚠️ EVOLVE
- 0-37 = BARK (critical) ❌ DELETE

**Q-Score = Weighted geometric mean of 5 axioms**

---

## 📊 Module Verdict Summary

### The Verdict Matrix

```
KEEP (No action)              EVOLVE (Plan work)       DELETE (Cleanup)
─────────────────────         ──────────────────       ─────────────────
core        (88/100) ✅       organism   (59/100) ⚠️   lnsp    (20/100) ❌
judges      (90/100) ✅       metabolism (59/100) ⚠️   training(30/100) ❌
consensus   (90/100) ✅
learning    (84/100) ✅
cognition   (74/100) ✅
nervous     (77/100) ✅
senses      (72/100) ✅
integrations(70/100) ✅
immune      (70/100) ✅
api         (72/100) ✅
cli         (68/100) ✅
dialogue    (72/100) ✅
observability(75/100)✅
llm         (72/100) ✅
```

### Impact of Actions

**Delete LNSP & training:**
- Remove 5,525 LOC of dead code
- Eliminate cognitive overhead
- Keep nervous system single (not dual)

**Evolve organism & metabolism:**
- Flatten organism from 10 layers → 3-4
- Validate metabolism routing with A/B test
- Add comprehensive test coverage

**Expected result:**
- 108,306 LOC → 98,396 LOC (9% smaller)
- Dead code: 5,525 → 0
- Test coverage: 265 → 400+ files
- Team clarity: high

---

## 🛠️ How to Use This Framework

### For Module Owners
1. Find your module in QUICK_REFERENCE or FRAMEWORK document
2. Read the full axiom breakdown
3. Understand which axiom(s) are failing
4. Follow remediation steps in ACTION_PLAN

**Example:** "I own metabolism (59/100)"
→ Read FRAMEWORK.md METABOLISM section
→ See that VERIFY scores low (untested routing)
→ Follow ACTION_PLAN.md step for A/B testing
→ Prove value or remove routing

### For Architecture Decisions
1. Ask: "Should we add/keep this module?"
2. Score it on 5 axioms (use QUICK_REFERENCE.md template)
3. Compute Q-Score
4. If < 62: Don't merge without remediation
5. If < 40: Delete or archive

### For Code Review
1. New module PR arrives
2. Check: Does it score 62+ on all axioms?
3. If VERIFY is low: Require tests
4. If CULTURE is low: Require documentation
5. If BURN is low: Require use case

### For Quarterly Reviews
1. Check each module's Q-Score
2. If dropped below 60: Assign owner to remediate
3. If still below 40 next quarter: Delete/archive
4. If zero imports: Mark for removal

---

## ✅ What You'll Learn

By reading these 3 documents, you'll understand:

- ✅ Why LNSP was never deployed and should be deleted
- ✅ Why training is a Phase 1B relic and should be archived
- ✅ Why organism is philosophically excellent but over-complex
- ✅ Why metabolism routing needs validation (A/B test)
- ✅ Why nervous, senses, and immune are healthy core modules
- ✅ How to evaluate ANY module using the 5 axioms
- ✅ How to make architecture decisions with confidence
- ✅ What clear code looks like (HOWL level: 82-100)
- ✅ What broken code looks like (BARK level: 0-37)
- ✅ How to build a codebase that stays clean

---

## 🎯 Next Steps

### If You Just Want to Know the Verdicts
→ Read **QUICK_REFERENCE.md** (10 min)

### If You Want to Understand the Reasoning
→ Read **FRAMEWORK.md** (45 min)

### If You Want to Execute the Cleanup
→ Read **ACTION_PLAN.md** (15 min) + follow checklist

### If You Own a Low-Scoring Module
→ Find your module in FRAMEWORK.md
→ Read evaluation section
→ Check ACTION_PLAN.md for remediation
→ Execute fixes
→ Re-score in 2 weeks

---

## 📞 Questions?

**Question:** Why does my module score low?
**Answer:** Check FRAMEWORK.md detailed breakdown for your module

**Question:** What does FIDELITY really mean?
**Answer:** See QUICK_REFERENCE.md "5 Axioms in 1 Sentence"

**Question:** How do I improve my score?
**Answer:** Check which axiom(s) score lowest; read ACTION_PLAN.md remediation steps

**Question:** Can we use this framework for new modules?
**Answer:** Yes! Template in QUICK_REFERENCE.md — score before merge

**Question:** What if I disagree with a verdict?
**Answer:** Re-read the axiom breakdown; discuss with team; may reveal new insight

---

## 📈 Success Metrics

This framework is working well when:

- [ ] Team understands why each module exists
- [ ] New modules are scored before merge
- [ ] Low-scoring modules get remediation plans
- [ ] Dead code is removed promptly
- [ ] Test coverage improves over time
- [ ] Architecture decisions are justified
- [ ] "Should we keep this?" questions answered in minutes

---

## 🏛️ The Covenant

**From now on:**

1. **Every new module must:**
   - Score ≥ WAG (62) on all 5 axioms before merge
   - Have >5% test coverage
   - Have a clear owner
   - Document its purpose

2. **Every quarter:**
   - Review all modules < WAG (62)
   - If still low next quarter: Delete/archive
   - If zero imports: Mark for removal

3. **When in doubt:**
   - Ask the 5 axioms
   - Score the module
   - Follow the verdict
   - Document the decision

---

## 📎 File Locations

All documents in root directory:
- `/CODEBASE_EVALUATION_FRAMEWORK.md` (1,062 lines)
- `/EVALUATION_QUICK_REFERENCE.md` (308 lines)
- `/ACTION_PLAN.md` (446 lines)
- `/EVALUATION_FRAMEWORK_README.md` (this file)

---

## 🙏 Thank You

**This framework required:**
- Auditing 108,306 LOC of code
- Analyzing 32 major modules
- Scoring 40+ modules/features
- Creating 1,816 lines of documentation
- Building decision frameworks

**Result:** You now have a tool to distinguish truth from falsehood across your codebase.

**Use it well.**

---

*Framework created by Claude Code*
*Based on CYNIC's 5 Core Axioms (FIDELITY, PHI, VERIFY, CULTURE, BURN)*
*Philosophy: Good code can be justified; bad code hides*
