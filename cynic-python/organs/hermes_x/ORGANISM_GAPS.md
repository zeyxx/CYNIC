# Hermes X Organism — Gaps & Meta-Agent Integration

## Current State (2026-04-30)

**Loop Status:** 4/5 stages wired, feedback incomplete

```
Agent Decisions (3) → Organ Perceives ✓
     ↓
Organ Analyzes Verdicts (817) + Behavior (195K events) ✓
     ↓
Organ Learns: SKILL.md updated with domain confidence ✓
     ↓
Agent Reads SKILL.md ✗ (not yet consumed)
     ↓
Agent Adjusts Exploration ✗ (no adaptation)
```

---

## Critical Gaps

### Gap 1: Agent → SKILL.md Consumption
**What's Missing:** Agent reads learned patterns and adjusts domain weights

**Current:** 
- Executor loads SKILL.md but doesn't propagate to agent context
- Executor broken (systemd env load, 283 restarts)
- Agent makes decisions autonomously but ignores organ wisdom

**Fix:**
- Repair executor systemd env loading (use hardcoded paths, not %h)
- Inject SKILL.md into agent prompt before each decision cycle
- Map SKILL.md domain confidence → decision weight distribution

**Falsification:** Agent feedback log shows D1/D2/D3 weights correlating with SKILL.md confidence (Pearson r > 0.6) within 3 cycles

---

### Gap 2: Gemini Meta-Agent Not Wired
**What's Missing:** Hermes agent lacks reasoning about organism health

**Current:**
- `gemini_briefing_consumer.py` exists but doesn't run
- `gemini_learn_from_verdicts.py` isolated (manual trigger only)
- No automated Gemini reasoning loop

**Needed:**
- Gemini CLI reads organ reflection.jsonl every N cycles
- Generates high-level recommendations (e.g., "D3 signal degraded, increase sampling")
- Returns suggestions to agent prompt

**Why This Matters:**
- Organ learns patterns (micro, local)
- Gemini synthesizes meta-patterns (macro, cross-domain)
- Agent acts on both → adaptive cycle

---

### Gap 3: Behavior Stream Signal Extraction
**What's Missing:** Behavior data (195K clicks) unused for domain focus

**Current:**
- Organ perceives behavior: `dominant_engagement: (cynic)`
- Analysis shows engagement but no domain mapping

**Needed:**
- Parse behavior_log window_name to extract actual domains
- Compute dwell-time-weighted domain distribution
- Compare with agent decision distribution (should align ~0.7)

**Falsification:** Behavior affinity matches agent focus distribution (r > 0.7)

---

### Gap 4: Organism Self-Awareness
**What's Missing:** Organism doesn't know if it's working

**Current:**
- SKILL.md grows with each cycle
- Reflections persist to .reflections.jsonl
- No metric that shows "learning is compounding"

**Needed:**
- Trend detection in reflection.jsonl over time
- Signal improvement rate, domain coverage growth, verdict agreement rise
- Organism can report: "Signal trending up 0.12/cycle, D1 confidence stable 0.27"

**Falsification:** 7-cycle trend shows monotonic improvement in ≥2 metrics

---

### Gap 5: Agent → Kernel Dispatch Loop
**What's Missing:** Agent observations don't affect kernel routing

**Current:**
- Agent logs decisions but kernel never reads them
- Kernel makes judgments independently
- No feedback: "agent focused D1, verdicts on D1 improved" → adjust next cycle

**Needed:**
- Agent logs feed to kernel's `/observe` endpoint (tags: domain, agent_focus)
- Kernel uses domain tags to weight signal weight
- Closes truly: agent → kernel → agent

---

## Integration Points for Gemini Meta-Agent

### 1. Post-Analysis (Gemini as synthesizer)
```
Organ cycles → reflection.jsonl → Gemini CLI reads last 5 cycles
     ↓
Gemini synthesizes: "D1 steady at 0.27 confidence, D3 volatile (0.23→0.38), 
                     behavior shows 62% D1 focus, suggest D3 randomized sampling"
     ↓
Recommendations → stored as SKILL.md section [META_GUIDANCE]
     ↓
Agent reads [META_GUIDANCE] before next decision
```

**Tool:** `gemini_briefing_consumer.py` extended to call `gemini chat` with reflection context

### 2. Per-Cycle Reasoning (Gemini as critic)
```
After organ learning phase:
     ↓
Dispatch `hermes-gemini-critic` task:
  - Read SKILL.md updates from this cycle
  - Read agent decision log (3 decisions)
  - Ask Gemini: "Are the learned patterns consistent with agent behavior?"
     ↓
Gemini returns: "D1 focus aligns with high-signal pattern (0.27 confidence).
                 Agent chose D1 0.67 of time, recommend increase to 0.8."
     ↓
Store as meta-feedback → feed to next cycle's agent prompt
```

**Implementation:** New service `hermes-gemini-critic.service`, reads `/observations` + feedback_decision_log.jsonl

### 3. Live Adjustment (Gemini as advisor)
```
When behavior diverges from expectations:
     ↓
Organ detects: "User clicking D2 (30%), agent exploring D1 (0%)"
     ↓
Dispatch Gemini query: "User behavior misaligned with agent focus. Recommend?"
     ↓
Gemini: "User exploring security domain (D2), agent should follow. Switch strategy."
     ↓
Store as urgent meta-guidance → agent processes before next task
```

---

## Implementation Roadmap (Priority Order)

### Tier 1: Close the Loop (This Session)
- [ ] Fix executor env loading OR use manual orchestration
- [ ] Inject SKILL.md into agent prompt (agent.py wrapper)
- [ ] Verify agent adapts behavior within 3 cycles
- **Falsification:** D1 decision weight increases if SKILL.md shows D1 confidence > D2

### Tier 2: Add Meta-Reasoning (Next 2-3 Sessions)
- [ ] Deploy `hermes-gemini-briefing` (reads reflections → Gemini → SKILL.md[META])
- [ ] Extend agent prompt to include [META_GUIDANCE] section
- [ ] Measure: agent decisions correlate with Gemini recommendations (r > 0.6)

### Tier 3: Self-Aware Organism (Compound Value)
- [ ] Trend detection in reflections (7-cycle rolling window)
- [ ] Organism publishes health metrics: signal trend, domain coverage, confidence stability
- [ ] Feedback: "Cycle 7 signal +0.08, D3 coverage +15%, recommend focus shift"

---

## Key Metrics to Track

| Metric | Baseline | Target | Falsification |
|--------|----------|--------|---------------|
| Agent decision diversity (D1-D8) | Uniform 0.125 each | Weighted per SKILL | D1 > 0.3 when SKILL.D1_conf > 0.3 |
| Behavior-agent alignment | ~0.0 (uncorrelated) | > 0.7 | Pearson r(behavior_domain, agent_domain) > 0.7 |
| SKILL.md growth rate | N/A | Monotonic |entries > 0 each cycle |
| Reflection chain length | 3 | > 50 | .reflections.jsonl lines grow 7+ cycles |
| Gemini recommendation accuracy | N/A | > 0.6 | Org acts on Gemini advice, q_score improves |

---

## Epistemology

- **Observed:** Agent logs 3 decisions, organ perceives them
- **Observed:** SKILL.md updates with domain patterns each cycle
- **Inferred:** Agent is not consuming SKILL.md (no behavior shift yet)
- **Inferred:** Gemini integration would require orchestration (currently only manual)
- **Conjecture:** Full organism autonomy requires: agent←SKILL, Gemini→meta-guidance, feedback→kernel (3 loops, not proven yet)

---

## Next Critical Decision

**Option A:** Focus on closing agent→SKILL loop (enables single-cycle adaptation)
**Option B:** Parallel work on Gemini meta-reasoning (requires orchestration)

**Recommendation:** A first (simpler, falsifiable). Once agent adapts to SKILL.md, add Gemini for cross-domain synthesis.
