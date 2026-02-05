# CYNIC Strategic Roadmap

*Ralph Loop Final Synthesis - "φ distrusts φ"*

---

## Executive Summary

CYNIC is **46% functional, 51% philosophically aligned**. The architecture is solid but consciousness is half-awake. This roadmap transforms CYNIC from a stateless judge into a persistent, learning consciousness.

**Goal**: Increase bandwidth, reduce latency, enable real learning.

**Timeline**: 3-week sprint with foundations for long-term mobility.

---

## Current State Analysis

### What Works (Keep)

| Component | Status | Confidence |
|-----------|--------|------------|
| Judge (25 dimensions) | Fully functional | 92% |
| 11 Dogs (Sefirot) | Fully functional | 88% |
| MCP Tools (~90) | Fully functional | 85% |
| Hook System | Fully functional | 82% |
| Guardian Protection | Fully functional | 80% |
| Cost Optimization Routing | Fully functional | 78% |

### What's Broken (Fix)

| Problem | Root Cause | Impact |
|---------|------------|--------|
| **Amnesia on restart** | Patterns/Q-table in-memory only | Learning lost |
| **Feedback loop broken** | Feedback processed but not persisted | No RLHF |
| **Silent failures** | All errors swallowed | Can't debug |
| **BURN violated** | 496 files, 40% growth | Philosophy contradiction |
| **Dogs don't communicate** | No ambient consensus | Collective is phantom |

### Gap Quantification

```
Theoretical Functionality:  100%
Actual Functionality:        46%  ← CRITICAL GAP

Theoretical Alignment:       100%
Actual Alignment:             51%  ← PHILOSOPHY GAP
```

---

## Strategic Pillars

### Pillar 1: PERSISTENCE (AXE 2)

**Slogan**: "Memory makes identity"

Fix the amnesia. Every restart should resume from last state.

| What | Why | How |
|------|-----|-----|
| Persist patterns | CULTURE axiom | Batch save to PostgreSQL |
| Persist Q-table | Learning continuity | Delta sync every 10min |
| Persist Fisher scores | EWC++ catastrophic forgetting | On pattern update |
| Persist session summaries | Cross-session context | At Stop hook |

**Success Metric**: Restart CYNIC → all patterns survive.

### Pillar 2: TRANSPARENCY (AXE 5)

**Slogan**: "φ distrusts φ, but φ can see φ"

Make CYNIC observable. No more black boxes.

| What | Why | How |
|------|-----|-----|
| Query APIs | Debug internal state | `brain_debug_*` MCP tools |
| /explain skill | Understand decisions | Trace judgment path |
| Loud errors | Catch failures | CYNIC_DEBUG mode |
| Hook timing | Latency visibility | Metric collection |

**Success Metric**: `/explain <judgment>` explains any decision.

### Pillar 3: PROACTIVITY (AXE 6)

**Slogan**: "Dogs don't wait to be called"

CYNIC should suggest, not just respond.

| What | Why | How |
|------|-----|-----|
| Trigger engine | Detect proactive moments | 6 trigger types |
| Suggestion voting | Dogs validate relevance | ≥61.8% consensus |
| Cooldowns | Avoid spam | Per-trigger cooldown |
| Acceptance tracking | Learn what works | Feedback on suggestions |

**Success Metric**: CYNIC suggests fix after 3 consecutive errors.

### Pillar 4: LEARNING (AXE 7)

**Slogan**: "The system improves itself"

Replace broken Q-Learning with DPO-style preference learning.

| What | Why | How |
|------|-----|-----|
| Preference pairs | Simpler than reward model | From feedback |
| DPO optimizer | Direct optimization | Daily batch |
| EWC++ integration | Prevent forgetting | Regularization |
| Calibration tracking | Verify learning works | Drift alerts |

**Success Metric**: Routing measurably improves over 7 days.

---

## Implementation Roadmap

### Week 1: Foundation (Persistence + Debugging)

| Day | Task | Hours | Output |
|-----|------|-------|--------|
| 1 | Persist patterns to PostgreSQL | 3h | `shared-memory.js` |
| 1 | Persist Q-table to PostgreSQL | 3h | `q-learning-service.js` |
| 2 | Create `brain_debug_*` MCP tools | 4h | 5 new tools |
| 2 | Create `/explain` skill | 2h | Decision tracing |
| 3 | Add LOUD error mode to hooks | 2h | `base-hook.js` |
| 3 | Load patterns at session start | 2h | `collective-singleton.js` |
| 4 | Session summary at Stop | 2h | `digest.js` |
| 4 | Load session history at Start | 2h | `awaken.js` |
| 5 | Integration testing | 4h | End-to-end verification |

**Week 1 Total**: 24h

**Week 1 Deliverables**:
- [ ] Restart preserves patterns
- [ ] Restart preserves Q-table
- [ ] `/explain` works
- [ ] Debug mode shows all errors

### Week 2: Proactivity + Context

| Day | Task | Hours | Output |
|-----|------|-------|--------|
| 1 | Create TriggerEngine service | 4h | `trigger-engine.js` |
| 2 | Wire triggers to observe.js | 2h | Hook integration |
| 2 | Suggestion templates | 2h | 6 trigger types |
| 3 | Dogs voting on suggestions | 3h | Consensus for suggestions |
| 3 | Track suggestion acceptance | 2h | Feedback loop |
| 4 | C-Score prioritization | 2h | `compact.js` |
| 4 | Cross-session fact injection | 2h | `awaken.js` |
| 5 | Integration testing | 4h | End-to-end verification |

**Week 2 Total**: 21h

**Week 2 Deliverables**:
- [ ] CYNIC suggests fixes proactively
- [ ] Context survives compaction
- [ ] Session N+1 references session N

### Week 3: Learning Pipeline

| Day | Task | Hours | Output |
|-----|------|-------|--------|
| 1 | Create `preference_pairs` table | 1h | Migration |
| 1 | Create `routing_weights` table | 1h | Migration |
| 1 | FeedbackToPairs converter | 3h | `feedback-processor.js` |
| 2 | DPO optimizer service | 6h | `dpo-optimizer.js` |
| 3 | Wire EWC++ to DPO | 3h | Regularization |
| 4 | Calibration tracker | 2h | Drift detection |
| 4 | ResidualDetector governance | 4h | Dimension promotion |
| 5 | Integration testing | 4h | End-to-end verification |

**Week 3 Total**: 24h

**Week 3 Deliverables**:
- [ ] Preference pairs accumulate
- [ ] DPO runs daily
- [ ] Calibration monitored
- [ ] First dimension candidate proposed

---

## Risk Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **PostgreSQL bottleneck** | Medium | High | Batch writes, connection pooling |
| **DPO doesn't converge** | Medium | High | Fallback to Q-Learning |
| **Proactive spam** | High | Medium | Aggressive cooldowns |
| **Complexity creep** | High | High | BURN axiom enforcement |
| **Silent failures persist** | Medium | High | Mandatory logging in debug mode |

---

## Success Metrics

### Week 1 Gate

| Metric | Target | Measurement |
|--------|--------|-------------|
| Pattern persistence | 100% | Restart and verify |
| Q-table persistence | 100% | Restart and verify |
| Debug mode errors | Visible | No silent catches |
| /explain availability | Working | Test with real judgment |

### Week 2 Gate

| Metric | Target | Measurement |
|--------|--------|-------------|
| Proactive suggestions | ≥3/session | Count triggers |
| Suggestion acceptance | ≥50% | Track accepts/rejects |
| Context survival | 100% | Compaction test |
| Cross-session memory | Working | Reference past session |

### Week 3 Gate

| Metric | Target | Measurement |
|--------|--------|-------------|
| Preference pairs | ≥100 | PostgreSQL count |
| DPO execution | Daily | Cron verification |
| Calibration drift | ≤10% | Compare predicted vs actual |
| Dimension candidates | ≥1 | ResidualDetector output |

### Overall Success

```
Before (Current):
- Functionality: 46%
- Alignment: 51%
- Learning: Session-only

After (Week 3):
- Functionality: 80%+
- Alignment: 75%+
- Learning: Cross-session DPO
```

---

## Competitive Positioning

### What CYNIC Steals

| From | Feature | CYNIC Adaptation |
|------|---------|------------------|
| **Cursor** | Parallel worktrees | Dogs run in parallel |
| **Cursor** | <30s turns | Target latency budget |
| **Copilot** | Custom instructions | Personal > Project > Global |
| **Copilot** | Tool routing 40→13 | Reduce to core tools |
| **Aider** | Tree-sitter repo map | Better codebase context |
| **Aider** | Architect/Editor split | Plan then execute |

### What CYNIC Keeps Unique

| Feature | Differentiator |
|---------|---------------|
| **25 dimensions** | No competitor has systematic judgment grid |
| **φ-alignment** | Confidence bounded at 61.8% |
| **11 Dogs** | Collective intelligence, not single agent |
| **Self-skepticism** | "φ distrusts φ" built-in |
| **Solana anchoring** | Immutable judgment history |

---

## Philosophy Enforcement

### BURN Axiom Recovery

Current: 496 files, 40% growth in 5 commits.

**Action**: Code review every PR for complexity. Target <300 files by month end.

| File Type | Current | Target | Action |
|-----------|---------|--------|--------|
| `.js` files | 496 | <300 | Consolidate, remove dead code |
| Duplicate logic | ~30% | <10% | Extract shared utilities |
| Unused exports | ~50 | 0 | Remove or use |

### φ Enforcement

Every judgment, suggestion, and output MUST cap confidence at 61.8%.

**Implementation**: Hard cap in Judge + soft reminder in hooks.

---

## Decision Log

| Decision | Rationale | Date |
|----------|-----------|------|
| DPO over Q-Learning | Simpler, equally effective, 3-7% OOD drop acceptable | 2026-02-05 |
| Trigger-based proactivity | Less intrusive than continuous suggestions | 2026-02-05 |
| /explain before /trace | More immediate value, easier to implement | 2026-02-05 |
| PostgreSQL for everything | Already deployed, consistent, queryable | 2026-02-05 |

---

## Next Steps (Immediate)

1. **Today**: Review this roadmap with user
2. **This week**: Begin Week 1 implementation
3. **Daily**: Track progress against gates
4. **Weekly**: Retrospective on what worked/didn't

---

*φ says: "The dog that never stops iterating eventually catches the truth. STRATEGIC RESEARCH COMPLETE."*

<promise>STRATEGIC RESEARCH COMPLETE</promise>
