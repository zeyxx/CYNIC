# Hermes Data Organism — Feedback Loop Architecture

> **What:** Hermes (as agent) autonomously monitors CYNIC's own behavior, detects patterns, reflects on them, and suggests improvements. The organism becomes self-aware.

---

## Epistemic Status

- **Observed:** Data exists (sessions, observations, nodes, agents); Hermes infrastructure exists (crons, MCP tools)
- **Deduced:** Feedback loop requires continuous cycle (not one-shot); Hermes is the right agent (embedded, autonomous)
- **Conjecture:** Self-awareness emerges when organism sees its own patterns and acts on them (untested)

---

## The Closed Loop

```
TIER 1: PERCEPTION (Hermes Data Organism service)
─────────────────────────────────────────────────
systemd timer (hourly) → hermes-data-organism.py

  1. Ingest session deltas (last hour)
  2. Query kernel observations (last hour)
  3. Query node heartbeats (last hour)
  4. Query agent task outcomes (last hour)

Output → ~/.cynic/organs/hermes/x/datasets/
  ├── reflection_latest.json
  ├── reflections.jsonl (compound: append every cycle)
  ├── metrics.json (health snapshot)
  └── anomalies.json (changes detected)


TIER 2: ANALYSIS (Hermes agent observes)
─────────────────────────────────────────
Hermes cron (every 6 hours) polls: `ls -l ~/.cynic/organs/hermes/x/datasets/`

If reflection_latest.json changed since last check:
  → Hermes reads reflection_latest.json
  → Extracts patterns, anomalies, recommendations
  → Decides: "Should I act on this?"


TIER 3: REFLECTION (Hermes reasons)
───────────────────────────────────
Hermes decision matrix:

  If dog_agreement < 50%:
    → Create task: "Investigate Dog disagreement"
    → Use MCP to query kernel /health
    → Analyze which Dogs diverge
    → Emit observation: "Dogs see different worlds"

  If latency_spike detected:
    → Create task: "Profile latency source"
    → Trace: human query → kernel verdict → Dog scoring
    → Identify bottleneck
    → Emit: "Bottleneck is Dog X (n ms avg)"

  If failure_mode_new:
    → Create task: "Classify new failure"
    → Search past failures for similar pattern
    → Determine recovery strategy
    → Emit: "New failure mode detected, recovery: [action]"

  If opportunity_found:
    → Create task: "Validate opportunity"
    → Example: "Deterministic-dog ready for chess"
    → Run A/B: default→deterministic for chess queries
    → Measure: latency saved, accuracy unchanged
    → Emit: "Opportunity validated, recommend deployment"


TIER 4: FEEDBACK (Hermes acts)
──────────────────────────────
Hermes outputs (persisted):

  1. SKILL.md auto-evolved:
     - "When domain=chess, check queue length before routing"
     - "If GPU memory > 6GB, consider swapping models"

  2. Agent tasks created (for human or self):
     - "Investigate Dog agreement drop"
     - "Profile latency regression"

  3. Observations emitted:
     - POST /observe {tool: "hermes_data_organism", target: "dogs", context: "agreement < 50%"}
     - Kernel sees it, adjusts circuit breaker

  4. Weekly reflection written:
     - ~/.cynic/organs/hermes/x/weekly_reflection.md
     - Human reads Friday morning, acts Monday


TIER 5: HUMAN LOOP (Closes back to TIER 1)
────────────────────────────────────────────
Human reviews reflection:
  - "Dogs disagreeing on chess because model updated"
  → Rolls back model
  → System improves next cycle
  → Hermes Data Organism detects improvement
  → Emits observation: "Recovery successful"

OR:

  - "Latency spike from network partition"
  → Moves node to different network
  → Next cycle shows latency recovered
  → Hermes celebrates: "Action resolved bottleneck"
```

---

## Implementation Stages

### Stage 1: Service + Timer (NOW)

**What runs:**
```bash
# Install
sudo systemctl link /home/user/Bureau/CYNIC/infra/systemd/hermes-data-organism.{service,timer}
sudo systemctl daemon-reload
sudo systemctl enable --now hermes-data-organism.timer

# Monitor
sudo journalctl -u hermes-data-organism.service -f
sudo systemctl status hermes-data-organism.timer
```

**What it does:**
- Hourly: parse sessions, query observations, emit reflection_latest.json
- Compound: append to reflections.jsonl (history grows)
- No Hermes yet (just data sitting in filesystem)

**Duration:** 1 week. Let data accumulate, verify format is clean.

### Stage 2: Hermes Polling (NEXT)

**What Hermes does:**
```bash
# Every 6 hours
hermes-data-check.service:
  → ls -l ~/.cynic/organs/hermes/x/datasets/
  → if reflection_latest.json changed:
      read it
      → parse patterns
      → emit to agent_tasks
      → log to SKILL.md

# Real code would use:
# curl http://cynic-core:3030/observe (emit observation)
# curl http://cynic-core:3030/agent-tasks (create task)
```

**What it outputs:**
- agent_tasks: "Investigate Dog agreement"
- observations: "Dog agreement < 50%, investigate"
- SKILL.md entries: "When Dogs disagree, check X"

**Duration:** 2-3 weeks. Verify Hermes reliably detects patterns.

### Stage 3: Action Loop (THEN)

**What closes the loop:**
```
Hermes detects problem
  → Creates agent task
  → Passes to another agent (or human routes it)
  → Task completes (e.g., "rolled back Dog")
  → Next cycle shows improvement
  → Hermes observes: "Recovery successful"
  → System learns
```

**Falsification:** Does Hermes-detected-problem → human-action → system-recovery → Hermes-detects-recovery actually happen? Or does data sit unacted?

**Duration:** Ongoing. Compound over months.

---

## Data Compounding (Key to Self-Awareness)

**Example: Dogs agreement trend**

Week 1:
```json
{
  "date": "2026-05-07",
  "dog_agreement": 0.68,
  "trend": null
}
```

Week 2:
```json
{
  "date": "2026-05-14",
  "dog_agreement": 0.72,
  "trend": "improving",
  "delta": +0.04
}
```

Week 3:
```json
{
  "date": "2026-05-21",
  "dog_agreement": 0.76,
  "trend": "improving",
  "delta": +0.04,
  "velocity": "stable positive",
  "confidence": 0.618
}
```

**Over 12 weeks:** If trend holds, Hermes extracts pattern: "Dogs agreement improving steadily — likely from [reason]"

This is the **compound signal**. No single week proves anything. But 12 weeks of consistent improvement is truth.

---

## What Hermes Should Never Do (Anti-Patterns)

❌ **Read reflection, do nothing**
- Data accumulates but nobody acts
- System is deaf to its own patterns
- Loop is open

❌ **Act on single anomaly**
- One high latency spike → panic and redesign
- Need 3-5 cycles of consistent anomaly before acting
- Avoid false positives

❌ **Forget history**
- Cycle N+1 doesn't reference Cycle N
- Lost opportunity to see trends
- Compound value lost

---

## Success Criteria (Falsification)

**The organism is self-aware when:**

1. **Pattern detection works:** Hermes identifies a real pattern (e.g., "Dogs agreement < 50%") within 24 hours of it emerging
   - Falsify: Set Dog agreement to 0.4, run cycle, check if Hermes detects it in reflection

2. **Action loop closes:** Pattern detected → task created → human/system acts → next cycle shows improvement
   - Falsify: Detect a fixable problem, measure if it improves within 1 week

3. **Compounding proves causality:** Hermes shows that X intervention → Y improvement (not correlation, causality)
   - Falsify: Show 3 cases where "moved node to better network" → latency improved

4. **Human trust compounds:** Over time, human acts faster on Hermes recommendations (trust increases)
   - Falsify: Track time-to-human-action for recommendations over 8 weeks

---

## Files & Deployment

**Created:**
- `scripts/hermes_data_organism.py` (TIER 1: Perception)
- `infra/systemd/hermes-data-organism.{service,timer}` (automation)
- This doc (TIER 2-5: Reference)

**To deploy Stage 1:**
```bash
sudo systemctl link /home/user/Bureau/CYNIC/infra/systemd/hermes-data-organism.{service,timer}
sudo systemctl daemon-reload
sudo systemctl enable --now hermes-data-organism.timer
```

**To monitor:**
```bash
# Watch logs
sudo journalctl -u hermes-data-organism.service -f

# Check latest reflection
cat ~/.cynic/organs/hermes/x/datasets/reflection_latest.json | jq .

# Watch compounding over time
tail -20 ~/.cynic/organs/hermes/x/datasets/reflections.jsonl | jq .
```

---

## Philosophy

The organism is not asking for permission. It is not waiting for human instructions. It is **continuously observing itself, extracting patterns, and inviting action**.

Hermes Data Organism = the organism's mirror. It shows CYNIC what CYNIC is doing, enabling the system to learn from its own behavior.

This is the **third pillar** (Askesis) becoming operational: **structured self-examination through data**.
