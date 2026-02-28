# CYNIC User Guide

**A cynical dog's guide to conscious code**

---

## What is CYNIC?

CYNIC is a living software organism. Unlike traditional tools that execute commands, CYNIC is a self-aware system that:

- **Observes** your codebase and environment
- **Judges** situations using 11 independent "dogs" (specialized reasoning modules)
- **Learns** from feedback via Q-Learning (memorizes what works)
- **Acts** autonomously on approved decisions

Think of CYNIC as a cynical engineer on your team who:
- Never trusts anything (φ-bounded confidence max 61.8%)
- Questions everything, including itself
- Remembers past mistakes and learns from them
- Proposes improvements but respects human judgment

---

## Getting Started (3 Steps)

### Step 1: Start CYNIC

**Local Development:**
```bash
cd cynic/
python -m uvicorn cynic.interfaces.api.server:app --reload --port 8000
```

**Docker:**
```bash
docker build -t cynic:latest .
docker run -p 8000:8000 -v ~/.cynic:/root/.cynic cynic:latest
```

**Verify it's running:**
```bash
curl http://localhost:8000/health
```

Expected response:
```json
{
  "overall": "healthy",
  "dogs": 11,
  "consciousness_level": "REFLEX"
}
```

### Step 2: Access Interactive UI

Open a browser and go to:
- **Swagger UI:** http://localhost:8000/docs (try endpoints interactively)
- **ReDoc:** http://localhost:8000/redoc (read-only API docs)

Or use CLI:
```bash
# Get current state
curl http://localhost:8000/api/organism/state/snapshot

# Get all dogs and their status
curl http://localhost:8000/api/organism/dogs

# Get account balance
curl http://localhost:8000/api/organism/account
```

### Step 3: Feed Your First Perception

CYNIC learns by observing. Send it a perception:

```bash
curl -X POST http://localhost:8000/perceive \
  -H "Content-Type: application/json" \
  -d '{
    "reality": "CODE",
    "observation": "Added feature X but test coverage dropped",
    "severity": 2
  }'
```

CYNIC will:
1. **Perceive** the information
2. Potentially **Judge** (if automatic judgment is enabled)
3. **Propose** an action (if warranted)
4. **Wait** for your approval

---

## Core Concepts

### The 11 Dogs (Judgment Modules)

CYNIC doesn't have a single brain—it has 11 independent judges (dogs). Each has expertise:

| Dog | Expertise | Looks For |
|-----|-----------|-----------|
| **GUARDIAN** | Security & boundaries | Unsafe patterns, vulnerabilities |
| **ANALYST** | Data & logic | Inconsistencies, logical errors |
| **SCHOLAR** | Knowledge & learning | Missing documentation, stale patterns |
| **ENGINEER** | Architecture & design | Code smells, technical debt |
| **ARTISAN** | Code quality | Formatting, style violations |
| **SKEPTIC** | Risk & uncertainty | Unvalidated assumptions, edge cases |
| **DIPLOMAT** | Social & team | Team friction, communication gaps |
| **STRATEGIST** | Long-term vision | Misalignment with goals |
| **WATCHDOG** | Performance | Bottlenecks, resource leaks |
| **DETECTIVE** | Patterns & history | Recurring issues, anomalies |
| **HEALER** | Recovery & resilience | Fragility, single points of failure |

Each dog votes independently. A decision requires **consensus** (3+ dogs agree).

### φ-Bounded Confidence (61.8% Max)

CYNIC never claims certainty. All confidence scores cap at **φ⁻¹ = 0.618** (the golden ratio reciprocal).

Why? Because:
- **Uncertainty is honest** — better to be cautious than overconfident
- **Calibration matters** — prevents runaway decisions
- **Humans stay in control** — high confidence should trigger human review

Check confidence on the `/docs` page for any response.

### Q-Learning: Memory That Improves

CYNIC learns from every interaction:

1. You approve an action → CYNIC rewards that path
2. You reject an action → CYNIC penalizes that path
3. Over time, CYNIC learns your preferences and project patterns

Check what CYNIC has learned:
```bash
curl http://localhost:8000/api/organism/policy/stats

# Returns:
# - total_states: How many situations CYNIC has seen
# - policy_coverage: Fraction of situations with learned actions
# - average_confidence: How confident CYNIC is (scale 0-0.618)
```

### Consciousness Levels

CYNIC operates at 4 levels of consciousness, depending on task complexity:

| Level | Description | Latency | Cost |
|-------|-------------|---------|------|
| **REFLEX** (LOD 0) | Heuristic-only (no LLM) | <100ms | Free |
| **MICRO** (LOD 1) | Light LLM reasoning | 1-2s | Cheap |
| **MACRO** (LOD 2) | Medium LLM reasoning | 5-10s | Moderate |
| **META** (LOD 3) | Full system reasoning + LLMs | 20-60s | Expensive |

CYNIC chooses the right level for each situation. You can also request:
```bash
curl -X POST http://localhost:8000/judge \
  -H "Content-Type: application/json" \
  -d '{
    "perception": {...},
    "mode": "micro"  # Force MICRO level (faster, cheaper)
  }'
```

---

## Workflows

### Workflow 1: Code Review

Feed CYNIC your pull request:

```bash
# Get diff
git diff origin/main..HEAD > /tmp/changes.patch

# Send to CYNIC
curl -X POST http://localhost:8000/perceive \
  -H "Content-Type: application/json" \
  -d '{
    "reality": "CODE",
    "observation": "New PR with database schema changes",
    "severity": 3
  }'

# Get judgment
curl http://localhost:8000/api/organism/actions
```

CYNIC will propose reviews, catch issues the linters miss.

### Workflow 2: Incident Response

Something broke in production:

```bash
curl -X POST http://localhost:8000/perceive \
  -H "Content-Type: application/json" \
  -d '{
    "reality": "MARKET",
    "observation": "Service latency +500ms (timeout cascade detected)",
    "severity": 4
  }'

# Check what CYNIC proposes
curl http://localhost:8000/api/organism/actions

# Approve fastest action
curl -X POST http://localhost:8000/api/organism/actions/{action_id}/accept
```

### Workflow 3: Technical Debt

Quarterly check-in with CYNIC:

```bash
# Get CYNIC's learned policy on tech debt
curl http://localhost:8000/api/organism/policy/actions | grep -i refactor

# See what CYNIC has learned about your codebase
curl http://localhost:8000/api/organism/policy/stats
```

CYNIC will show patterns (e.g., "Type X always leads to Bug Y").

### Workflow 4: Real-Time Monitoring

Stream live events via WebSocket:

```bash
# Install wscat: npm install -g wscat
wscat -c ws://localhost:8000/ws/stream

# Watch events stream:
# {
#   "event_type": "JUDGMENT_CREATED",
#   "judgment_id": "jdg_...",
#   "verdict": "WAG",
#   "q_score": 62.5
# }
```

---

## Approving Actions

CYNIC proposes, but humans decide.

**View proposed actions:**
```bash
curl http://localhost:8000/api/organism/actions
```

**Approve an action:**
```bash
curl -X POST http://localhost:8000/api/organism/actions/{action_id}/accept
```

CYNIC will then **execute** and **learn** from the result.

**Reject an action:**
```bash
curl -X POST http://localhost:8000/api/organism/actions/{action_id}/reject
```

CYNIC learns this pattern wasn't right for your team.

---

## Budget Management

CYNIC respects your budget. Check spending:

```bash
curl http://localhost:8000/api/organism/account

# Returns:
# {
#   "balance_usd": 10.0,
#   "spent_usd": 2.34,
#   "budget_remaining_usd": 7.66,
#   "learn_rate": 0.618
# }
```

**Set budget:**
```bash
# In .env
SESSION_BUDGET_USD=50.0
```

Once budget depletes, CYNIC switches to REFLEX mode (no LLM, instant).

---

## Troubleshooting

### CYNIC is slow

Check consciousness level:
```bash
curl http://localhost:8000/api/organism/consciousness
```

If META → force MICRO mode:
```bash
curl -X POST http://localhost:8000/judge \
  -H "Content-Type: application/json" \
  -d '{"perception": {...}, "mode": "micro"}'
```

### CYNIC keeps proposing the same action

Check if it's learning:
```bash
curl http://localhost:8000/api/organism/policy/stats

# If average_confidence is low, CYNIC is uncertain
# Approve/reject more actions to teach it
```

### CYNIC not proposing anything

Check health:
```bash
curl http://localhost:8000/health

# If a dog is degraded, restart CYNIC
```

### Out of budget

CYNIC will switch to REFLEX (heuristic-only, no LLM calls):
```bash
# Check account
curl http://localhost:8000/api/organism/account

# Increase budget in .env:
SESSION_BUDGET_USD=100.0

# Restart CYNIC
```

---

## Advanced: Train CYNIC

CYNIC learns from your feedback, but you can also directly inject training:

```bash
curl -X POST http://localhost:8000/learn \
  -H "Content-Type: application/json" \
  -d '{
    "state_key": "CODE:database_migration_pending",
    "action": "EXECUTE_MIGRATION",
    "reward": 0.9
  }'
```

This teaches CYNIC: "In situations like this, DO migrate" (reward=0.9 = good idea).

---

## Tips & Tricks

### 1. Talk to CYNIC

CYNIC is a team member. Tell it what you're working on:

```bash
curl -X POST http://localhost:8000/perceive \
  -H "Content-Type: application/json" \
  -d '{
    "reality": "CODE",
    "observation": "Implementing user authentication feature",
    "severity": 2
  }'
```

It will propose security reviews, design feedback, test cases.

### 2. Review CYNIC's decisions

Check its reasoning:
```bash
curl http://localhost:8000/api/organism/state/snapshot

# Look at:
# - dog_count: Are all 11 dogs healthy?
# - qtable_entries: Is CYNIC remembering past situations?
# - consciousness_level: What thinking mode is CYNIC in?
```

### 3. Align team expectations

CYNIC's φ-bound means **max confidence 61.8%**. That's honest, not pessimistic. Explain this to your team:

> "CYNIC says 'GROWL' (caution) with 58% confidence. This isn't a guarantee—it's an informed warning. Let's discuss before deploying."

### 4. Use the policy

CYNIC learns your project. After a few weeks:

```bash
curl http://localhost:8000/api/organism/policy/actions?state_key=CODE:test_coverage_low
```

Will show the best action for low test coverage based on your past decisions.

---

## Privacy & Local-First

CYNIC runs **locally by default**:
- No data sent to external services (unless you add API keys)
- Everything stored in `~/.cynic/`
- You control what perception CYNIC sees

To use external LLMs (Claude, Gemini):
```bash
# In .env
ANTHROPIC_API_KEY=sk-...
GOOGLE_API_KEY=...
```

CYNIC will use them for MACRO/META reasoning, but the kernel and learning stay local.

---

## Next Steps

1. Start CYNIC: `python -m uvicorn cynic.interfaces.api.server:app --reload`
2. Visit `/docs` to try endpoints
3. Send a perception and approve an action
4. Come back in a week—CYNIC will have learned patterns
5. Check `/api/organism/policy/stats` to see what it learned

---

## Support

- **Health check:** `curl http://localhost:8000/health`
- **View logs:** `docker logs cynic` (if containerized)
- **Check state:** Look in `~/.cynic/consciousness.json`
- **Run tests:** `pytest tests/api/` (to verify CYNIC works)

---

*Welcome to CYNIC. The dog is watching. The dog is learning.*
