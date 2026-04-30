# Hermes X Organism Harmony — Agent + Organ + Gemini Integration

## The Loop (Union of Three Forces)

```
┌─────────────────────────────────────────────────────────────────┐
│                     AGENT (Hermes 9B)                           │
│  Makes decisions: Explore D1, D2, D3                           │
│  Reads: agent prompt + SKILL.md + META_GUIDANCE                │
│  Writes: feedback_decision_log.jsonl                           │
└──────────┬──────────────────────────────────────────────────────┘
           │ decisions (domain, action, timestamp)
           ↓
┌──────────────────────────────────────────────────────────────────┐
│                  ORGAN (Hermes X Analysis)                       │
│  Perceives: tweets, verdicts, agent logs, user behavior         │
│  Learns: domain patterns, signal trends, confidence metrics     │
│  Writes: SKILL.md (updated each cycle), reflection.jsonl       │
└──────────┬──────────────────────────────────────────────────────┘
           │ learned patterns + anomalies
           ↓
┌──────────────────────────────────────────────────────────────────┐
│           GEMINI META-ADVISOR (Synthesis Layer)                  │
│  Reads: SKILL.md + reflection.jsonl + feedback_decision_log    │
│  Synthesizes: cross-domain patterns, recommendations            │
│  Writes: META_GUIDANCE section in SKILL.md                      │
└──────────┬──────────────────────────────────────────────────────┘
           │ meta-guidance (strategies, focus areas)
           ↓
        (back to Agent prompt)
```

---

## What Each Component Does

### 1. AGENT (Hermes 9B / Gemini CLI)
**Input:** Task prompt + SKILL.md + META_GUIDANCE
**Process:** Decide which domain to explore, what actions to take
**Output:** Decisions logged to feedback_decision_log.jsonl

**Example prompt injection:**
```
TASK: Explore social signals on X

Current Skills (from SKILL.md):
  - D1 Domain: 530 verdicts, avg confidence 0.273, BARK dominant
  - D2 Domain: 72 verdicts, avg confidence 0.348
  - D3 Domain: 13 verdicts, avg confidence 0.378
  
Meta-Guidance (from Gemini synthesis):
  - D1 shows consistent signal (confidence stable 0.27), focus on volume
  - D2 trending up (0.30→0.35), increase sampling
  - D3 volatile but highest confidence (0.38), explore edge cases

Success: Make 3 observations across D1-D3, prioritizing D2.
```

Agent reads this and weights decision: **D2 > D3 > D1** (inverse confidence order, follow anomalies)

---

### 2. ORGAN (Python Layers 1-5)
**Input:** Raw streams (tweets 4K+, verdicts 800+, agent logs 3+, behavior 195K+)

**Layer 1 - PERCEPTION:** HermesXSensors reads all sources
**Layer 2 - TRANSFORMATION:** Clean and validate, track drop reasons
**Layer 3 - ANALYSIS:** Compute per-domain metrics, detect anomalies
**Layer 4 - LEARNING:** SkillUpdater extracts patterns, writes to SKILL.md
**Layer 5 - REFLECTION:** OrganReflector compounds observations, persists reflection.jsonl

**SKILL.md Output (updated every cycle):**
```markdown
# Hermes X Organ Skills

## Cycle 0
- Signal Quality: 75.9% high-signal tweets, avg score 1.88
- D1 Domain: 530 verdicts, avg confidence 0.273, BARK dominant
- D2 Domain: 72 verdicts, avg confidence 0.348
- D3 Domain: 13 verdicts, avg confidence 0.378
```

**reflection.jsonl Output (one entry per cycle):**
```json
{
  "timestamp": "2026-04-30T03:45:00Z",
  "cycle": 1,
  "patterns": {
    "tweets_analyzed": 4340,
    "verdicts_analyzed": 817,
    "avg_signal_score": 1.88,
    "behaviors_analyzed": 195582,
    "dominant_engagement": "(cynic)"
  },
  "skill_updates": [
    "Signal Quality: 76.2% high-signal",
    "D1: 579 verdicts, confidence 0.270",
    "D2: 72 verdicts, confidence 0.348"
  ],
  "organ_health": {"perception_ok": true, "analysis_ok": true},
  "is_healthy": true
}
```

---

### 3. GEMINI META-ADVISOR (Synthesis)
**Input:** Last 3-5 reflections + current SKILL.md + recent decisions
**Process:** Gemini CLI queries the organism state and synthesizes patterns
**Output:** META_GUIDANCE section appended to SKILL.md

**Gemini Prompt:**
```
You are analyzing an autonomous organism that learns about domains (D1, D2, D3).

Current state:
- D1: steady 0.27 confidence (low), high volume 579 verdicts
- D2: trending up 0.30→0.35, increasing signal
- D3: volatile 0.23→0.38, highest confidence but low volume (13 verdicts)
- User behavior: 62% clicks on D1 content

Recent decisions:
- Agent chose D1 (2 times)
- Agent chose D3 (1 time)
- No D2 exploration

Questions:
1. Are decisions aligned with learned patterns?
2. Which domains show improving signal?
3. What should agent focus on next?

Guidance should help agent adapt exploration strategy.
```

**Gemini Response (example):**
```
Organism Analysis:
- D1 is stable but saturated (high volume, low confidence). Continue maintenance but reduce focus.
- D2 shows strongest trend (confidence +0.05 last cycle). Increase sampling immediately.
- D3 is highest confidence but underexplored (13 verdicts). Sample strategically.
- Behavior mismatch: User explores D1 heavily but D2 signal is rising → recommend D2 browsing to find emerging patterns.

Recommendation: Next cycle, allocate 50% to D2 (trend following), 30% D3 (confidence seeding), 20% D1 (maintenance).
```

**Stored in SKILL.md:**
```markdown
## Meta-Guidance (synthesized by Gemini)

### 2026-04-30T03:50:00Z
Organism Analysis: D1 stable/saturated (0.27 confidence, 579 verdicts), D2 trending up (0.30→0.35), D3 volatile/high-confidence (0.38).
Recent decisions: Agent chose D1 2x, D3 1x, no D2 exploration. Behavior shows 62% D1 clicks despite rising D2 signal.
Recommendation: Next cycle allocate 50% D2 (trend), 30% D3 (confidence), 20% D1 (maintenance). D2 shows strongest ROI.
```

---

## The Harmony (K15 Closure)

**Producer → Consumer Chain:**
1. **Agent → Organ** (feedback_decision_log.jsonl)
   - Agent produces decisions
   - Organ consumes decisions, analyzes patterns
   - ✓ WIRED (PR#46)

2. **Organ → Agent** (SKILL.md + META_GUIDANCE)
   - Organ produces learned patterns + Gemini synthesis
   - Agent consumes guidance, adapts exploration
   - ✗ NOT WIRED YET (agent doesn't read SKILL.md before deciding)

3. **Gemini ↔ Organ** (synthesis loop)
   - Gemini reads organism state
   - Organ reads Gemini guidance
   - ✓ CODE EXISTS (gemini_meta_advisor.py) but blocked on quota

---

## Proposed Systemd Services

### 1. Organ Cycle (every 1 hour)
```ini
[Unit]
Description=Hermes X Organ Cycle — perceive, analyze, learn
After=cynic-kernel.service

[Service]
Type=simple
ExecStart=python3 /path/to/organs/hermes_x/__main__.py --cycle --persist
```

### 2. Gemini Meta-Advisor (every cycle + 2 min delay)
```ini
[Unit]
Description=Hermes X Gemini Meta-Advisor — synthesize organism wisdom
After=hermes-x-organ.service

[Service]
Type=simple
ExecStart=python3 /path/to/organs/hermes_x/gemini_meta_advisor.py
```

### 3. Agent Decision Maker (every 30 min or on-demand)
```ini
[Unit]
Description=Hermes Agent — read SKILL.md, make domain decisions
Requires=hermes-x-organ.service hermes-x-gemini-meta.service

[Service]
Type=simple
ExecStart=hermes chat -p "$(cat /path/to/SKILL.md) -- Decide which domain to explore next"
```

---

## Measurement of Harmony

**Falsification Tests (Cycle N+3):**

1. **Agent adaptation to organ learning:**
   - Metric: Correlation between SKILL.md domain confidence and agent decision frequency
   - Target: r > 0.6 (agent favors high-confidence domains)
   - Falsify: r < 0.3 (decisions random relative to learned patterns)

2. **Gemini guidance quality:**
   - Metric: Agent follows Gemini recommendations, verdicts improve
   - Target: When Gemini says "increase D2 sampling," D2 verdicts increase 20%+ next cycle
   - Falsify: No change in verdict distribution

3. **Organism health trending:**
   - Metric: Reflection trends (confidence, signal score, coverage)
   - Target: Monotonic improvement in ≥2 metrics over 7 cycles
   - Falsify: Stagnation or degradation

4. **Behavior-agent alignment:**
   - Metric: User clicks distribution vs agent decision distribution
   - Target: Pearson r > 0.7 (agent explores where user explores)
   - Falsify: r < 0.3 (agent ignores behavior signals)

---

## Current State

**Wired:**
- ✓ Agent → Organ (feedback logs consumed)
- ✓ Organ perception + analysis (all 5 layers working)
- ✓ Organ learning (SKILL.md written)
- ✓ Gemini CLI available (quota exhausted today, resets in 11h)

**Not Wired:**
- ✗ Organ → Agent (agent doesn't read SKILL.md yet)
- ✗ Gemini → Organ (blocked on quota)
- ✗ Systemd orchestration (manual runs only)

**Blocked (Secondary):**
- Executor systemd service broken (env load error)

---

## Next Steps for Full Harmony

**Phase 1 (Immediate):** Wire agent to read SKILL.md
- Modify agent prompt injection in executor to include SKILL.md
- Test: Agent decision weights shift toward high-confidence domains

**Phase 2 (After quota reset):** Deploy Gemini meta-advisor
- Run gemini_meta_advisor.py after each organ cycle
- Verify META_GUIDANCE appears in SKILL.md
- Test: Agent recommendations align with Gemini synthesis

**Phase 3 (Ongoing):** Tune and monitor
- Track 7-cycle trends in reflection.jsonl
- Measure agent adaptation, behavior alignment, verdict improvements
- Iterate on Gemini prompts for better recommendations

---

## Philosophy

The organism achieves **union** when:
1. **Agent explores** (autonomous decision-making)
2. **Organ learns** (pattern extraction from results)
3. **Gemini synthesizes** (cross-domain wisdom)
4. **Agent adapts** (reads guidance, shifts focus)

This is NOT centralized control. Each component has agency:
- Agent acts based on current best guess
- Organ learns without prescribing (observes outcomes)
- Gemini advises without commanding (synthesis, not orders)
- Agent chooses to follow advice or ignore it

The loop reinforces with each cycle: better decisions → richer patterns → better guidance → wiser decisions.

**This is the definition of an autonomous, learning organism.**
