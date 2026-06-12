# Burn Audit: cynic-python Graveyard

**Scope**: 132 Python modules, 131 dead, 1 wired

**Goal**: Categorize each file (salvage / delete / archive) + extract lessons for data-architecture design

**Timeline**: 2-3 hours

---

## Audit Methodology

For each directory in cynic-python/, assess:

1. **Is this code running?**
   - Wired to systemd? (check ~/.config/systemd/user/ + infra/systemd/)
   - Called by scripts/hermes-x/? (grep imports)
   - Has K15 consumer? (someone acting on output?)
   - Yes → **TIER 2 INFRASTRUCTURE** (keep, maybe improve)
   - No → Continue to #2

2. **Is this code salvageable?**
   - Last edit date? (>30 days = stale, <7 days = recent = might have context)
   - Does it solve a real problem? (check docstring, code intent)
   - Is error handling clean? (crashes vs graceful)
   - Could it become Tier 2 with 1-2 days work?
   - Yes → **TIER 1 EXPERIMENTAL SALVAGEABLE** (archive with resurrection guide)
   - No → Continue to #3

3. **Should this be deleted?**
   - No consumer identified (ever was there one?)
   - No tests (no way to verify it works)
   - No docstring explaining intent
   - Code is fragile (bad error handling, hardcoded paths, etc.)
   - Yes → **DELETE** (with reason in commit message)

4. **Special case: Does this contain valuable research?**
   - Findings documented? (paper, report, analysis?)
   - Should be archived in docs/research/ instead of deleted?
   - Yes → **ARCHIVE TO DOCS** (move code out, keep findings)

---

## Audit Template (per directory)

```
DIRECTORY: domains/
Files: 23
Wired: 0
Salvageable: ?
Delete: ?
Archive: ?

FILE ASSESSMENTS:
├── domain_discovery_lite_v0.py
│   Last edit: 2026-02-15 (>60 days)
│   Intent: Clustering-based domain discovery
│   Status: No tests, no docstring, abandoned
│   Decision: DELETE (no clear use case)
│
├── domain_lifecycle_predictor_v0.py
│   Last edit: 2026-04-20 (recent!)
│   Intent: Predict which domains user engages with next
│   Status: Has unit tests, docstring clear, 250 lines
│   Consumer: None identified (why was it built?)
│   Decision: SALVAGEABLE (2 days to add systemd consumer + kernel wiring)
│
└── ...

PATTERNS OBSERVED:
- All clustering experiments have no output consumer
- Behavioral analysis module (v1) has type hints (good)
- Archive-folder contains graduated research (keep history)
```

---

## Expected Patterns to Look For

**Pattern 1: "Built but No Consumer"**
- Code is solid (tests, docs, error handling)
- Problem: Never wired to systemd or kernel
- Cause: Researcher forgot to promote to Tier 2
- Example: behavioral/behavior_simulator.py
- Action: SALVAGEABLE or DELETE (decide based on usefulness)

**Pattern 2: "Research Dead End"**
- Code was an experiment (kenosis_mining, domain_discovery v0-v3)
- Findings were negative ("doesn't work") or marginal
- Problem: Never became operational
- Cause: Hypothesis failed, abandoned
- Action: DELETE (with findings archived if any)

**Pattern 3: "Duplicate/Superseded"**
- Same functionality reimplemented (heuristics/twitter_dog.py vs heuristics/twitter_heuristics.py)
- One wired, one abandoned
- Problem: Code rot + confusion
- Action: DELETE the superseded version, document in wired version

**Pattern 4: "Infrastructure Debt"**
- Valid code, but written pre-Tier2/Tier1 protocol
- No K15 consumer documented
- Problem: Unclear why it exists
- Action: Either tag as Tier 2 (add consumer) or delete

**Pattern 5: "Valuable Research Findings"**
- Code itself is dead, but analysis/paper is valuable
- Examples: kenosis_mining (found 1 pattern), phase2_human_filtering (methodological insights)
- Problem: Findings could get lost with code deletion
- Action: ARCHIVE to docs/research/[topic]/ with summary

---

## Expected Outcomes by Directory

### agents/ (2 files)
- telegram_organ_bot.py: Tier 1 or delete? (telegram not core)
- Prediction: 0/2 salvageable

### behavioral/ (2 files)
- behavior_ml_train.py: Tier 1 salvageable (research, but clean code)
- behavior_simulator.py: Tier 3 candidate? (used by data-architecture?)
- Prediction: 1-2/2 salvageable

### benchmarks/ (18 files)
- Mix of local measurement scripts (convergence, judge quality, hardware profile)
- None wired (not production)
- Prediction: 0/18 active, 3-5/18 salvageable (keep hardware_profiler), 13-15/18 delete

### consumers/ (6 files)
- k15_observation_consumer.py: **WIRED** ✓ (Tier 2)
- Others (k15_emitter, k15_token_analysis, k15_infrastructure): Tier 1 experimental?
- Prediction: 1/6 active, 2-3/6 salvageable, 2-3/6 delete

### domains/ (23 files)
- All clustering/discovery experiments
- None wired
- Quality varies (some have tests, some don't)
- Prediction: 0/23 active, 2-4/23 salvageable, 19-21/23 delete

### heuristics/ (21 files)
- R21 VIOLATION: validation/heuristics exist but no consumer
- twitter_dog.py, twitter_heuristics.py: Core logic?
- Others: Measurement/tuning scripts
- Prediction: 2-3/21 active (maybe), 3-5/21 salvageable, 13-16/21 delete

### inference_organ/ (23 files)
- Full dead code layer (old architecture)
- Some might have been replaced by kernel code
- Prediction: 0/23 active, 1-2/23 salvageable (if architecture docs valuable), 21-22/23 delete

### lab/ (2 files)
- lab.py: Local measurement tool
- measure_domain_quality.py: Research script
- Prediction: 0/2 active, 0-1/2 salvageable, 1-2/2 delete

### organs/ (18 files)
- extract_organ_x_tweets.py: Used by hermes-x? (check)
- Others: Old organ experiments?
- Prediction: 0-1/18 active, 2-3/18 salvageable, 15-16/18 delete

### soma/ (1 file)
- l2_compute_gate.py: **WIRED** ✓ (systemd soma-l2.service)
- Prediction: 1/1 active, 0/1 delete

### validation/ (12 files)
- Research/corpus collection scripts
- Some might have valuable datasets
- Prediction: 0/12 active, 3-5/12 salvageable (if datasets), 7-9/12 delete

### Overall Prediction
- Active (Tier 2): 2 files (k15_consumer, l2_compute_gate)
- Salvageable (Tier 1): 10-15 files
- Delete: 105-120 files
- Archive to docs: 5-10 findings/papers

---

## Execution (Quick Audit)

For each directory:

```bash
# Step 1: List files with last edit date
find cynic-python/DIR/ -name "*.py" -type f | while read f; do
  echo "$(stat -c %y $f | cut -d' ' -f1) $(basename $f)"
done | sort

# Step 2: Check for systemd wiring
grep -r "DIR/" ~/.config/systemd/user/ infra/systemd/

# Step 3: Check for K15 consumer (in memory or docs)
grep -r "DIR/" .cortex/memory/ docs/

# Step 4: Read docstring + assess code quality
head -20 cynic-python/DIR/file.py
```

---

## Salvage Priority (If Audit Succeeds)

If we find 10-15 salvageable files, which to promote to Tier 2?

**Tier 2 Candidates** (in order):
1. **behavioral/behavior_simulator.py** — Might be useful for organic agent training
2. **heuristics/twitter_dog.py** — Core scoring logic (if it's not duplicated in kernel)
3. **domains/[any with tests]** — If domain routing needs research foundation
4. **validation/[datasets]** — If they're valuable for testing

**Don't promote unless**: Clear consumer identified + 4 hours to wire + metrics defined

---

## Burn Cleanup Plan (If Audit Finds Junk)

If we identify 105+ files to delete:

**Phase 1**: Tag all with `# DEAD_CODE: reason` (document why, not silent deletion)

**Phase 2**: Move dead files to `cynic-python/.archive/` (git mv, not delete)

**Phase 3**: Commit with message:
```
fix(burn): archive 105 dead cynic-python modules

Categorized via burn audit (2026-05-05):
- 1 active (k15_consumer)
- 1 active (l2_compute_gate)
- 10 salvageable (in .archive/, resurrection guide in docs/RESURRECT.md)
- 105+ dead (no consumer, no use case)

Affected directories:
- domains/: 21 deleted (all clustering experiments, no consumer)
- heuristics/: 15 deleted (measurement scripts, no use case)
- benchmarks/: 15 deleted (local tools, not production)
- inference_organ/: 22 deleted (old architecture, replaced by kernel)
- validation/: 9 deleted (corpus builders, research-only)
- others/: 23 deleted (mixed experiments, no uptake)

Archived (if findings valuable):
- domains/domain_discovery_lite_v0/ → docs/research/domain-discovery/
- behavioral/behavior_simulator.py → docs/research/behavioral-simulation/
- etc.

This unblocks cynic-python for proper Tier 2 infrastructure (data-architecture module).
Cost: ~2h audit + 1h cleanup. Saved: 105+ modules no longer confusing future work.
```

**Phase 4**: Create docs/RESURRECT.md (guide for resurrecting salvageable code):
```
# How to Resurrect Salvageable Code

If you want to bring back code from .archive/:

1. Identify the module: `git log --oneline -- cynic-python/.archive/` 
2. Restore: `git show <commit>:cynic-python/.archive/file.py > cynic-python/file.py`
3. Tag as Tier 1 EXPERIMENTAL (add docstring with death date)
4. OR tag as Tier 2 INFRASTRUCTURE (add consumer + systemd service)
5. Re-add to version control and commit

Example: Resurrecting behavior_simulator.py
- git show <audit-commit>:cynic-python/.archive/behavioral/behavior_simulator.py > cynic-python/behavioral/behavior_simulator.py
- Update docstring: Tier 2, consumer=organic_agent_training
- Add systemd service
- Commit: "feat(behavioral): resurrect behavior_simulator for organic agent training"
```

---

## Success Criteria for Burn Audit

✓ All 132 files categorized (Tier 2 / Tier 1 salvageable / Delete / Archive)
✓ Reason documented for each file
✓ Salvageable files tagged with resurrection guide
✓ Dead files moved to .archive/ or deleted
✓ cynic-python now has ≤15 modules: 2 active + 10-15 Tier 1 + 1-2 Tier 2 infrastructure
✓ Lesson extracted: Why did experiments die? What pattern?

---

## Lesson Extraction (for data-architecture design)

After audit, answer:

1. **Why did most experiments die?**
   - No consumer (most likely) = K15 violation from day 1
   - No systemd wiring (can't run consistently)
   - Latency too high (couldn't integrate into feedback loop)
   - Research question answered negatively (hypotheses failed)

2. **What made 2 files succeed (k15_consumer, l2_compute_gate)?**
   - Clear consumer identified (kernel routing, compute scheduling)
   - Wired to systemd from day 1
   - Metrics in /health (observable)
   - Error handling for production

3. **What should data-architecture do differently?**
   - Start with Phase 0 audit (don't assume correlations work)
   - Identify Phase 3 consumer FIRST (agent learning loop)
   - Wire Phase 1 to systemd early (not retrofit later)
   - Include metrics + monitoring from Phase 1 (not Phase 4)

---

## Timeline

- Audit: 2-3 hours (categorize + reason)
- Cleanup: 1 hour (move + archive + tag)
- Documentation: 30 min (RESURRECT guide + lessons)
- Total: ~4 hours

**Then**: Start data-architecture Phase 0 (audit + baseline) with lessons learned
