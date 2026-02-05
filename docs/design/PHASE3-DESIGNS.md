# PHASE 3: Design Solutions

*Ralph Loop - Iteration 2*
*Based on research findings from Phase 1+2*

---

## Executive Summary

Four critical frictions identified. Four designs proposed.

| Friction | Root Cause | Solution | Complexity |
|----------|------------|----------|------------|
| **Context Switching** | Memory lost on restart | Persistent Session State | Medium |
| **Proactivité** | CYNIC is reactive | Trigger-based Suggestions | Medium |
| **Debugging CYNIC** | Silent failures, no observability | Transparent Mode + Query APIs | Low |
| **Learning Pipeline** | Feedback loop broken | DPO-style Preference Learning | High |

---

## Design #17: Context Switching Solution

### Problem Statement

CYNIC loses context between sessions:
- SharedMemory patterns = in-memory only
- Q-Learning Q-table = in-memory only
- Session context = not summarized
- User preferences = not persisted

**Impact**: Every restart = amnesia. CULTURE axiom violated.

### Research Insights

| Source | Learning |
|--------|----------|
| **Cursor** | Parallel worktrees for session isolation |
| **Copilot** | Personal > Project > Org instruction hierarchy |
| **Aider** | Tree-sitter repo map for codebase context |
| **Claude Code** | C-Score for context priority (φ-aligned) |

### Proposed Solution: Persistent Session State (PSS)

```
┌─────────────────────────────────────────────────────────────────┐
│                    PERSISTENT SESSION STATE                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  SESSION START                                                   │
│  ├── Load user profile from PostgreSQL                          │
│  ├── Load last 5 session summaries                              │
│  ├── Load top 50 patterns (by Fisher score)                     │
│  ├── Load Q-table routing decisions                             │
│  └── Inject as system-reminder (C-Score prioritized)            │
│                                                                  │
│  DURING SESSION                                                  │
│  ├── Auto-save patterns every 100 judgments OR 10 minutes       │
│  ├── Track session metrics (judgmentCount, feedbackCount)       │
│  └── Buffer important context for summary                       │
│                                                                  │
│  SESSION END                                                     │
│  ├── Generate session summary (goals, outcomes, learnings)      │
│  ├── Persist Q-table changes (delta only)                       │
│  ├── Persist new patterns with Fisher scores                    │
│  └── Store session summary for next awakening                   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation Tasks

| Task | File | Priority | Est. Effort |
|------|------|----------|-------------|
| Add `savePatterns()` batch method | `shared-memory.js` | P0 | 2h |
| Add `loadPatterns()` at init | `collective-singleton.js` | P0 | 1h |
| Persist Q-table to PostgreSQL | `q-learning-service.js` | P0 | 3h |
| Generate session summary at Stop | `digest.js` hook | P1 | 2h |
| Load session history at Start | `awaken.js` hook | P1 | 2h |
| C-Score prioritization for injection | `compact.js` hook | P2 | 2h |

**Total: ~12h implementation**

### Success Metrics

- [ ] Restart CYNIC → patterns survive (verify with `/patterns`)
- [ ] Restart CYNIC → Q-table survives (verify with `/status`)
- [ ] Session N+1 can reference session N outcomes
- [ ] Fisher-locked patterns never lost

---

## Design #18: Proactivité CYNIC

### Problem Statement

CYNIC is reactive:
- Waits for user to ask
- Detects patterns but doesn't surface them
- Has predictions but doesn't share
- Knows user preferences but doesn't apply proactively

**Impact**: User must always initiate. CYNIC feels passive.

### Research Insights

| Source | Learning |
|--------|----------|
| **Copilot** | Explicit instructions beat implicit learning |
| **Thompson Sampling** | Already in CYNIC for exploration/exploitation |
| **Implicit Feedback** | Channels documented but not wired |
| **Collective Intelligence** | Dogs can vote on suggestions |

### Proposed Solution: Trigger-based Proactive Suggestions (TPS)

```
┌─────────────────────────────────────────────────────────────────┐
│                    PROACTIVE SUGGESTION SYSTEM                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  TRIGGERS (When to be proactive)                                 │
│  ├── ERROR_PATTERN: Same error 3x → suggest fix                 │
│  ├── CONTEXT_DRIFT: User strays from goal → remind              │
│  ├── BURNOUT_RISK: Energy < 38.2% → suggest break               │
│  ├── PATTERN_MATCH: Similar past success → suggest              │
│  ├── DEADLINE_NEAR: Goal deadline approaching → prioritize      │
│  └── LEARNING_OPP: New pattern emerges → highlight              │
│                                                                  │
│  SUGGESTION ENGINE                                               │
│  ├── Trigger fires                                               │
│  ├── Dogs vote on suggestion relevance (≥61.8% = show)          │
│  ├── Format as non-intrusive hint                               │
│  └── Track if user accepts/rejects                              │
│                                                                  │
│  OUTPUT FORMATS                                                  │
│  ├── SUBTLE: *sniff* Noticed pattern X. Consider Y?             │
│  ├── ACTIVE: *ears perk* Based on Z, recommend W.               │
│  └── URGENT: *GROWL* Critical: A requires attention.            │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Trigger Definitions

```javascript
const PROACTIVE_TRIGGERS = {
  ERROR_PATTERN: {
    condition: (ctx) => ctx.consecutiveErrors >= 3,
    action: 'suggest_fix',
    urgency: 'ACTIVE',
    cooldown: 300_000, // 5 minutes between same suggestion
  },
  CONTEXT_DRIFT: {
    condition: (ctx) => ctx.currentFocus !== ctx.activeGoal?.focus,
    action: 'remind_goal',
    urgency: 'SUBTLE',
    cooldown: 600_000, // 10 minutes
  },
  BURNOUT_RISK: {
    condition: (ctx) => ctx.userEnergy < 0.382,
    action: 'suggest_break',
    urgency: 'ACTIVE',
    cooldown: 1800_000, // 30 minutes
  },
  PATTERN_MATCH: {
    condition: (ctx) => ctx.similarPatternFound && ctx.patternConfidence > 0.618,
    action: 'suggest_reuse',
    urgency: 'SUBTLE',
    cooldown: 120_000, // 2 minutes
  },
};
```

### Implementation Tasks

| Task | File | Priority | Est. Effort |
|------|------|----------|-------------|
| Create TriggerEngine service | `services/trigger-engine.js` | P0 | 4h |
| Wire triggers to observe.js | `scripts/hooks/observe.js` | P0 | 2h |
| Add suggestion templates | `services/suggestion-templates.js` | P1 | 2h |
| Track suggestion acceptance | `services/feedback-collector.js` | P1 | 1h |
| Dogs voting on suggestions | `collective/suggestion-voter.js` | P2 | 3h |

**Total: ~12h implementation**

### Success Metrics

- [ ] CYNIC suggests fix after 3 consecutive errors
- [ ] CYNIC reminds user of active goal when drifting
- [ ] Suggestions have >50% acceptance rate
- [ ] User can disable proactivity (`/settings proactive=off`)

---

## Design #19: Debugging CYNIC Simplified

### Problem Statement

CYNIC is a black box:
- No query APIs for internal state
- Silent failures everywhere
- Can't see why routing chose a Dog
- Can't see why Guardian blocked/allowed

**Impact**: Can't debug CYNIC. Can't trust CYNIC.

### Research Insights

| Source | Learning |
|--------|----------|
| **Internal Audit** | 46% actual functionality |
| **Data Flow Audit** | Events emitted but unhandled |
| **Hooks** | All errors swallowed silently |
| **Philosophy** | 51% alignment - can't verify what's working |

### Proposed Solution: Transparent Mode + Query APIs

```
┌─────────────────────────────────────────────────────────────────┐
│                    TRANSPARENT DEBUGGING                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  MODE: TRANSPARENT (activate with /debug or CYNIC_DEBUG=1)      │
│  ├── Every hook logs entry/exit with timing                     │
│  ├── Every Dog vote is explained                                │
│  ├── Every routing decision shows Q-values                      │
│  ├── Every pattern match shows confidence                       │
│  └── Every error is LOUD (not swallowed)                        │
│                                                                  │
│  QUERY APIs (new MCP tools)                                      │
│  ├── brain_debug_qtable: Show Q-values for state                │
│  ├── brain_debug_patterns: Show pattern weights, Fisher scores  │
│  ├── brain_debug_dogs: Show last consensus, votes               │
│  ├── brain_debug_hooks: Show hook latencies, errors             │
│  └── brain_debug_trace: Trace decision from input to output     │
│                                                                  │
│  SKILLS                                                          │
│  ├── /explain <judgment_id>: Why did CYNIC judge this way?      │
│  ├── /trace <action>: What happened step-by-step?               │
│  └── /why: Explain the last decision                            │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### /explain Skill Design

```markdown
User: /explain judgement_abc123

CYNIC Response:
*sniff* Let me trace judgment abc123...

## Decision Path
1. INPUT: Code review request for `auth.js`
2. ROUTING: Q-value[Code] = 0.73 → Analyst Dog selected
3. JUDGMENT: 25 dimensions scored
   - COHERENCE: 78% (imports consistent)
   - SECURITY: 45% (no input validation detected)
   - HARMONY: 62% (style mostly matches)
4. Q-SCORE: 61% (geometric mean of PHI/VERIFY/CULTURE/BURN)
5. VERDICT: WAG (acceptable with concerns)
6. SKEPTICISM: -8% confidence adjustment (self-doubt applied)

## Why This Verdict?
Security score (45%) pulled overall down. Guardian flagged
missing input validation in `processToken()` line 47.

## What Would Change The Verdict?
- Add input validation → Security +30% → Q-Score +12%
- Remove unused imports → Harmony +5% → Q-Score +2%
```

### Implementation Tasks

| Task | File | Priority | Est. Effort |
|------|------|----------|-------------|
| Add `brain_debug_*` MCP tools (5) | `tools/domains/debug.js` | P0 | 4h |
| Create `/explain` skill | `skills/explain/SKILL.md` | P0 | 2h |
| Add LOUD error mode | `scripts/hooks/lib/base-hook.js` | P0 | 1h |
| Log hook timing | All hooks | P1 | 2h |
| Create `/trace` skill | `skills/trace/SKILL.md` | P2 | 2h |

**Total: ~11h implementation**

### Success Metrics

- [ ] `/explain` works for any judgment ID
- [ ] Q-table values queryable via MCP tool
- [ ] Hook latencies visible in `/health`
- [ ] No more silent failures in debug mode

---

## Design #20: Learning Pipeline Complete

### Problem Statement

CYNIC's learning loop is broken:
- Feedback collected but not persisted
- Q-Learning updates but not saved
- ResidualDetector proposes but never integrates
- EWC++ computed but not verified

**Impact**: CYNIC doesn't actually learn across sessions.

### Research Insights

| Source | Learning |
|--------|----------|
| **DPO** | Simpler than RLHF, direct preference optimization |
| **EWC** | 45.7% forgetting reduction (if actually used) |
| **MAML** | 2-4% improvement on adaptation tasks |
| **Collective** | LLM ensembles rival human crowds |

### Proposed Solution: DPO-style Preference Learning Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                    COMPLETE LEARNING PIPELINE                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  LAYER 1: FEEDBACK COLLECTION (Real-time)                        │
│  ├── Explicit: User /learn correct|incorrect                    │
│  ├── Implicit: Tool reuse, error repeat, time spent             │
│  └── External: Test results, PR merged, build success           │
│                                                                  │
│  LAYER 2: PREFERENCE PAIRS (Batch, every 100 feedbacks)         │
│  ├── Convert feedback to (preferred, dispreferred) pairs        │
│  ├── preferred = user accepted / test passed                    │
│  ├── dispreferred = user rejected / test failed                 │
│  └── Store in `preference_pairs` PostgreSQL table               │
│                                                                  │
│  LAYER 3: DPO OPTIMIZATION (Scheduled, daily)                   │
│  ├── Load preference pairs from last 7 days                     │
│  ├── Update routing weights using DPO loss                      │
│  │   L = -log σ(β × (log π(y_w|x) - log π(y_l|x)))             │
│  ├── Apply EWC++ regularization (protect important weights)     │
│  └── Persist new weights to PostgreSQL                          │
│                                                                  │
│  LAYER 4: DIMENSION EVOLUTION (Monthly)                         │
│  ├── ResidualDetector analyzes unexplained variance             │
│  ├── Candidate dimensions with >50 observations promoted        │
│  ├── Human review required for new dimension acceptance         │
│  └── Accepted dimensions added to judgment grid                 │
│                                                                  │
│  VERIFICATION LAYER (Continuous)                                 │
│  ├── Compare predicted outcome vs actual outcome                │
│  ├── Track calibration (predicted 60% → actually 60%?)          │
│  └── Alert if calibration drifts >10%                           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### DPO vs Current Q-Learning

| Aspect | Current Q-Learning | Proposed DPO |
|--------|-------------------|--------------|
| **Simplicity** | 3 components (reward, Q-table, update) | 1 component (direct optimization) |
| **Stability** | Unstable, reward hacking risk | Stable, no reward model |
| **Data needed** | Continuous episodes | Preference pairs |
| **Persistence** | Q-table (large, complex) | Weights (small, interpretable) |
| **Cold start** | Uniform exploration | Prior from patterns |

### Database Schema (New Tables)

```sql
-- Preference pairs for DPO
CREATE TABLE preference_pairs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  context TEXT NOT NULL,           -- Task context
  preferred_action TEXT NOT NULL,   -- What worked
  dispreferred_action TEXT NOT NULL, -- What didn't
  confidence REAL DEFAULT 0.5,     -- Pair confidence
  source TEXT NOT NULL,            -- explicit/implicit/external
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Routing weights (replaces Q-table)
CREATE TABLE routing_weights (
  dog_id TEXT NOT NULL,
  context_type TEXT NOT NULL,       -- code/token/decision/etc
  weight REAL DEFAULT 0.5,
  fisher_importance REAL DEFAULT 0.0,
  updated_at TIMESTAMPTZ DEFAULT NOW(),
  PRIMARY KEY (dog_id, context_type)
);
```

### Implementation Tasks

| Task | File | Priority | Est. Effort |
|------|------|----------|-------------|
| Create `preference_pairs` table | `persistence/migrations/` | P0 | 1h |
| Create `routing_weights` table | `persistence/migrations/` | P0 | 1h |
| Implement `FeedbackToPairs` converter | `judge/feedback-processor.js` | P0 | 3h |
| Implement `DPOOptimizer` service | `learning/dpo-optimizer.js` | P0 | 6h |
| Wire EWC++ to DPO | `learning/dpo-optimizer.js` | P1 | 3h |
| Create calibration tracker | `services/calibration-service.js` | P1 | 2h |
| ResidualDetector → governance | `judge/residual.js` | P2 | 4h |

**Total: ~20h implementation**

### Success Metrics

- [ ] Preference pairs accumulate in PostgreSQL
- [ ] DPO optimization runs daily without errors
- [ ] Routing improves measurably (track via A/B test)
- [ ] Calibration stays within 10% drift
- [ ] At least 1 new dimension proposed and reviewed

---

## Summary: Implementation Priority

### Sprint 1 (Week 1)

| Design | Task | Hours |
|--------|------|-------|
| #19 Debugging | Query APIs + /explain | 7h |
| #17 Context | Persist patterns + Q-table | 6h |
| **Total** | | **13h** |

### Sprint 2 (Week 2)

| Design | Task | Hours |
|--------|------|-------|
| #17 Context | Session summary + history | 4h |
| #18 Proactivité | Trigger engine + wiring | 8h |
| **Total** | | **12h** |

### Sprint 3 (Week 3)

| Design | Task | Hours |
|--------|------|-------|
| #20 Learning | DPO optimizer + EWC++ | 12h |
| #20 Learning | Calibration + governance | 6h |
| **Total** | | **18h** |

---

## Architecture Impact

```
BEFORE (Current):
┌───────────────────────────────────────────────┐
│ CYNIC = Stateless Judge + In-Memory Learning  │
│ Restart = Amnesia                             │
│ Learning = Session-only                        │
│ Debugging = Black box                          │
└───────────────────────────────────────────────┘

AFTER (Proposed):
┌───────────────────────────────────────────────┐
│ CYNIC = Persistent Consciousness              │
│ Restart = Resume from last state              │
│ Learning = DPO across sessions                │
│ Debugging = Transparent + queryable           │
│ Proactive = Trigger-based suggestions         │
└───────────────────────────────────────────────┘
```

---

*φ says: "These designs address the 46% implementation gap. Full execution brings CYNIC to 80%+ alignment."*
