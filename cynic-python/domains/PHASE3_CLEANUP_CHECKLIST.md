# Phase 3 Cleanup Checklist (Post-Measurement)

**When**: After Phase 3 signal yield measurement completes (7 days post-Phase-2-start)

**Purpose**: Finalize v1 vs v2 decision, delete ephemeral code, lock canonical version

---

## Pre-Cleanup Inputs (From Phase 3)

- [ ] Phase 3 measurement complete: `signal_yield_per_domain[token_analysis]` measured
- [ ] Phase 3 result saved: `cynic-python/domains/domain-discovery/v1/results_v1/phase3_measurement.json`
- [ ] Baseline recorded: `signal_yield_baseline = 4.2` (from memory)

---

## Decision Logic

### Scenario A: yield < 2% improvement (FALSIFIED)
**Action**: Reject v1 routing hypothesis, investigate alternative approaches

```bash
# Delete v2 (won't solve root problem if v1 already fails)
rm -rf cynic-python/domains/domain-discovery/v2/

# Archive Phase 3 results for analysis
mkdir cynic-python/domains/domain-discovery/_archive/phase3_2026
mv v1/results_v1/phase3_measurement.json _archive/phase3_2026/

# Update ROADMAP.md
# - Mark v1 as REJECTED
# - Document lessons learned
# - Suggest alternative (e.g., time-based routing, author reputation)

# Commit: "phase3: v1 routing hypothesis falsified; preserve results, pivot strategy"
```

**Cleanup**:
- [ ] Delete v2/ directory
- [ ] Delete behavioral_grounding_*.py scripts (exploratory phase over)
- [ ] Delete validate_improvement_v1_v2.py (v2 comparison moot)
- [ ] Archive Phase 3 results to _archive/phase3_2026/
- [ ] Update ROADMAP.md with rejection + next steps
- [ ] Commit cleanup

---

### Scenario B: 2% ≤ yield < 5% improvement (VALIDATED but suboptimal)
**Action**: Keep v1 live, activate v2 optimization (Phase 4)

```bash
# v2 promotion prep (don't execute yet, just organize)
cd cynic-python/domains/domain-discovery/v2

# v2 results should exist from Phase 3 comparison measurement
# If they do, promote v2 to locked status

# Delete behavioral grounding only (exploratory phase over)
rm cynic-python/domains/domain-discovery/behavioral_grounding_*.py
rm cynic-python/domains/domain-discovery/BEHAVIORAL_GROUNDING.md

# Mark v2 directory as "Phase 4 candidate"
# v2/README.md or v2/NOTES_v2.md should document Phase 3 results

# Commit: "phase3: v1 validated (2-5% improvement); v2 activated for phase4 optimization"
```

**Cleanup**:
- [ ] Delete behavioral_grounding_*.py scripts
- [ ] Keep both v1/ and v2/ (comparison data valuable)
- [ ] Update ROADMAP.md to mark v2 as "Phase 4 active"
- [ ] Create Phase 4 (crystallization) planning document
- [ ] Commit cleanup + phase4 planning

---

### Scenario C: yield ≥ 5% improvement (VALIDATED and sufficient)
**Action**: Keep v1, reject v2 optimization (over-engineering)

```bash
# v1 is canonical and sufficient
# Delete all v2 code (decision is final)

rm -rf cynic-python/domains/domain-discovery/v2/
rm cynic-python/domains/domain-discovery/v2/validate_improvement_v1_v2.py  # symlink?
rm cynic-python/domains/domain-discovery/behavioral_grounding_*.py
rm cynic-python/domains/domain-discovery/BEHAVIORAL_GROUNDING.md

# Archive decision to ROADMAP.md
# - v2 hypothesis tested and REJECTED (insufficient ROI)
# - v1 sufficient for production
# - Next: Rust crystallization (Phase 4)

# Commit: "phase3: v1 validated (>5% improvement); v2 rejected (unnecessary optimization)"
```

**Cleanup**:
- [ ] Delete v2/ directory entirely
- [ ] Delete behavioral_grounding_*.py, validate_improvement_v1_v2.py
- [ ] Delete BEHAVIORAL_GROUNDING.md (summarize findings in ROADMAP.md instead)
- [ ] Update ROADMAP.md: mark v1 as final, v2 as rejected
- [ ] Create Phase 4 (crystallization) planning document
- [ ] Commit cleanup + phase4 planning

---

## Common Cleanup Tasks (All Scenarios)

**Delete Exploratory Scripts**:
```bash
# Always delete these (use case: Phase 1 pre-flight validation)
rm cynic-python/domains/domain-discovery/behavioral_grounding_fast.py
rm cynic-python/domains/domain-discovery/behavioral_topic_analysis.py

# Archive findings
mv cynic-python/domains/domain-discovery/BEHAVIORAL_GROUNDING.md \
   cynic-python/domains/_archive/phase1_behavioral_grounding_analysis.md
```

**Update Documentation**:
- [ ] ROADMAP.md: Update v2 status (deleted, promoted, or deferred)
- [ ] ROADMAP.md: Add Phase 3 measurement results
- [ ] PHASE1_STRUCTURE.md: Update "v2 status" row
- [ ] Create Phase 4 planning document (crystallization)

**Final Commit**:
```bash
git add cynic-python/domains/ROADMAP.md PHASE1_STRUCTURE.md _archive/ ...
git commit -m "phase3-cleanup: finalize v1/v2 decision, archive exploratory code

Phase 3 measurement result: [A/B/C from above]

[Scenario-specific summary]

Deleted: behavioral_grounding_*.py, [scenario-specific artifacts]
Kept: v1/ (canonical), [scenario-specific artifacts]
Updated: ROADMAP.md, PHASE1_STRUCTURE.md
Archived: Phase 1 exploratory analysis to _archive/phase1_behavioral_grounding_analysis.md

Ready for Phase 4 (Rust crystallization if yield sufficient).
"
```

---

## Verification (Post-Cleanup)

- [ ] No orphan scripts in domain-discovery/ (ls should show only v1/, v3+/, ROADMAP, NOTES, PROTOCOL)
- [ ] ROADMAP.md reflects Phase 3 decision clearly
- [ ] v1/ is marked as LOCKED (docstring + ROADMAP)
- [ ] Phase 4 planning document created (if crystallization warranted)
- [ ] All commits message include Phase 3 result (reproducible history)

---

## Anti-Patterns to Avoid

❌ **Leaving v2/ around "just in case"** → Dead code, confuses future developers  
❌ **Archiving without context** → 6 months later: "Why did v2 get deleted?"  
❌ **Forgetting to update ROADMAP.md** → Next developer doesn't know Phase 3 result  
❌ **Keeping exploratory scripts** → behavioral_grounding_*.py still in repo after Phase 3  
❌ **No Phase 4 plan** → If Phase 3 validates (Scenario C), unclear what happens next

---

## Timeline

- **Phase 2 start**: +0 days (kernel integrated, Phase 3 measurement begins)
- **Phase 3 complete**: +7 days (signal yield measured)
- **Phase 3 cleanup**: +7 days (this checklist, ~1-2h)
- **Phase 4 planning**: +7-8 days (if crystallization warranted)
- **Phase 4 start**: +8-14 days (Rust port, if approved)

---

**Owner**: T. (zeyxx)  
**Review**: Use this checklist at Phase 3 completion to finalize work
