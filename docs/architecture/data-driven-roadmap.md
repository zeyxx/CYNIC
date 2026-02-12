# CYNIC Data-Driven Roadmap
## *"Measure everything, trust nothing, improve constantly"*

**Date:** 2026-02-12
**Status:** DRAFT
**Confidence:** 58% (φ⁻¹ limit)

---

## 🎯 NORTH STAR METRIC

**Primary Goal:** Transform CYNIC from **38% structural, 5% functional** to **80% functional autonomous organism** in 4 weeks.

**Success Definition:**
```
Functional Autonomy = (
  0.25 × Perception_Autonomy +      // Watchers polling continuously
  0.25 × Learning_Velocity +         // Weights improving per session
  0.20 × Routing_Intelligence +      // Decisions getting smarter
  0.15 × Cost_Efficiency +           // $/task decreasing
  0.15 × Multi_Domain_Coverage       // Beyond CODE dimension
)

Target: 80% functional autonomy by 2026-03-12
Current: ~5% (measured 2026-02-12)
```

---

## 📊 CURRENT STATE (Baseline Metrics)

### Measured 2026-02-12

| Metric | Current | Target | Gap | Priority |
|--------|---------|--------|-----|----------|
| **PERCEPTION** |
| Watchers polling | 0/4 active | 3/4 active | -3 | 🔥 CRITICAL |
| Proactive events/hour | 0 | 120+ | -120 | 🔥 CRITICAL |
| MARKET data availability | 0% | 80%+ | -80% | 🔥 CRITICAL |
| **LEARNING** |
| Loops consuming data | 0/11 | 11/11 | -11 | 🔥 CRITICAL |
| Sessions logged | 0 | 100+ | -100 | 🔥 CRITICAL |
| Q-weights updated live | 0% | 100% | -100% | 🔥 CRITICAL |
| Learning velocity | N/A | +5%/100ep | N/A | 🟠 HIGH |
| **ROUTING** |
| KabbalisticRouter used | 0% | 80%+ | -80% | 🔥 CRITICAL |
| LLMRouter used | 0% | 60%+ | -60% | 🔥 CRITICAL |
| Multi-LLM routing | 0% | 40%+ | -40% | 🟠 HIGH |
| DogPipeline usage | 0 calls | 20+ calls | -20 | 🟠 HIGH |
| **COST** |
| $/task tracked | NO | YES | - | 🟠 HIGH |
| Budget warnings | 0 | Auto | - | 🟡 MEDIUM |
| Tier optimization | 0% | 30% tasks to Ollama | -30% | 🟠 HIGH |
| **AUTONOMY** |
| Background loops active | 1/7 | 6/7 | -5 | 🔥 CRITICAL |
| Daemon uptime | 0% | 99%+ | -99% | 🟠 HIGH |
| Self-corrections/week | 0 | 5+ | -5 | 🟡 MEDIUM |
| **MULTI-DOMAIN** |
| Dimensions active | 2/7 | 5/7 | -3 | 🔥 CRITICAL |
| Cross-domain tasks | ~5% | 20%+ | -15% | 🟠 HIGH |
| MARKET perception | 0% | 50%+ | -50% | 🔥 CRITICAL |

---

## 🎯 GOALS (Data-Driven Success Criteria)

### Week 1 Goals (2026-02-19)

**Theme:** Wire the nervous system

| Goal | Metric | Target | Measurement |
|------|--------|--------|-------------|
| G1.1: Watchers polling | `SELECT COUNT(*) FROM watcher_heartbeats WHERE timestamp > NOW() - INTERVAL '1 hour'` | ≥3 watchers | PostgreSQL query |
| G1.2: Learning consuming data | `SELECT COUNT(DISTINCT loop_type) FROM learning_events WHERE date > '2026-02-12'` | ≥5 loop types | PostgreSQL query |
| G1.3: Q-weights updating | `SELECT COUNT(*) FROM qlearning_state WHERE updated_at > NOW() - INTERVAL '1 day'` | ≥10 updates/day | PostgreSQL query |
| G1.4: KabbalisticRouter active | `grep "KabbalisticRouter.route" ~/.cynic/daemon/daemon.log | wc -l` | ≥20 calls | Log analysis |
| G1.5: LLMRouter routing | `SELECT COUNT(*) FROM llm_usage WHERE adapter != 'anthropic' AND date > '2026-02-12'` | ≥10 non-Anthropic | PostgreSQL query |

**Success Criteria:** 4/5 goals met → Week 1 PASS

---

### Week 2 Goals (2026-02-26)

**Theme:** Prove learning works

| Goal | Metric | Target | Measurement |
|------|--------|--------|-------------|
| G2.1: Learning velocity | `SELECT AVG(accuracy) FROM routing_accuracy WHERE episode_batch = N+1` - `batch N` | +3% improvement | PostgreSQL query |
| G2.2: Sessions logged | `SELECT COUNT(*) FROM judgment_sessions WHERE created_at > '2026-02-19'` | ≥50 sessions | PostgreSQL query |
| G2.3: Consciousness depth | `SELECT COUNT(*) FROM consciousness_snapshots WHERE state = 'AWAKE'` / total | ≥30% AWAKE | PostgreSQL query |
| G2.4: DogPipeline used | `SELECT COUNT(*) FROM orchestration_decisions WHERE method = 'pipeline'` | ≥5 calls | PostgreSQL query |
| G2.5: Cost tracking | `SELECT COUNT(DISTINCT judgment_id) FROM cost_ledger WHERE task_id IS NOT NULL` | ≥30 tasks | PostgreSQL query |

**Success Criteria:** 4/5 goals met → Week 2 PASS

---

### Week 3 Goals (2026-03-05)

**Theme:** Multi-domain + autonomy

| Goal | Metric | Target | Measurement |
|------|--------|--------|-------------|
| G3.1: MARKET data flowing | `SELECT COUNT(*) FROM market_events WHERE date > '2026-02-26'` | ≥500 events | PostgreSQL query |
| G3.2: Multi-domain tasks | `SELECT COUNT(*) FROM judgments WHERE dimension_count > 1` / total | ≥15% | PostgreSQL query |
| G3.3: Daemon uptime | `ps aux | grep 'daemon/index.js' | wc -l` | 1 (running) | Process check |
| G3.4: Self-corrections | `SELECT COUNT(*) FROM judgments WHERE item_type = 'cynic_code' AND verdict != 'BARK'` | ≥3 good self-mods | PostgreSQL query |
| G3.5: Background loops | `SELECT COUNT(DISTINCT loop_name) FROM background_tasks WHERE active = true` | ≥5 loops | PostgreSQL query |

**Success Criteria:** 4/5 goals met → Week 3 PASS

---

### Week 4 Goals (2026-03-12)

**Theme:** Benchmark + optimize

| Goal | Metric | Target | Measurement |
|------|--------|--------|-------------|
| G4.1: Learning maturity | Average of all 11 loop maturity scores | ≥61.8% | Maturity formula |
| G4.2: Cost efficiency | `SELECT AVG(cost_usd) FROM tasks WHERE date > '2026-03-05'` vs Week 1 | -20% cost/task | PostgreSQL query |
| G4.3: Routing accuracy | `SELECT AVG(first_try_success) FROM routing_decisions WHERE date > '2026-03-05'` | ≥75% | PostgreSQL query |
| G4.4: Functional autonomy | Calculate composite metric | ≥80% | Formula above |
| G4.5: HumanEval benchmark | Run benchmark suite | ≥70% pass | External benchmark |

**Success Criteria:** 4/5 goals met → v1.0 ACHIEVED

---

## 🔄 COORDINATION PLAN (Parallel Work)

### Work Streams (4 parallel tracks)

```
┌─────────────────────────────────────────────────────────┐
│ STREAM 1: PERCEPTION (Week 1-2)                         │
│ Owner: Perception Team                                  │
│ Dependencies: None                                      │
├─────────────────────────────────────────────────────────┤
│ • GAP-2: Start watchers (SolanaWatcher, FileWatcher)   │
│ • GAP-4: Implement MARKET perception (MarketWatcher)   │
│ • Metrics: G1.1 (watchers polling)                     │
│ • Data flow: Watchers → globalEventBus → Orchestrator │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ STREAM 2: LEARNING (Week 1-3)                          │
│ Owner: Learning Team                                    │
│ Dependencies: GAP-1 (orchestrator wired)               │
├─────────────────────────────────────────────────────────┤
│ • GAP-3: Wire learning feedback loop                   │
│ • R3: Close consciousness read-back                    │
│ • Wiring Gap 3: Q-Learning weights applied live       │
│ • Metrics: G1.2, G1.3, G2.1 (learning velocity)       │
│ • Data flow: Feedback → LearningService → Weights     │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ STREAM 3: ROUTING (Week 1-2)                           │
│ Owner: Orchestration Team                              │
│ Dependencies: None                                      │
├─────────────────────────────────────────────────────────┤
│ • GAP-1: Wire UnifiedOrchestrator to daemon           │
│ • Wiring Gap 1: KabbalisticRouter integration         │
│ • Wiring Gap 2: LLMRouter activation                  │
│ • R1: Wire DogPipeline                                 │
│ • R2: Automatic retry                                  │
│ • Metrics: G1.4, G1.5, G2.4 (router usage)            │
│ • Data flow: Events → Router → Dogs → Consensus       │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ STREAM 4: COST & MEMORY (Week 2-3)                     │
│ Owner: Infrastructure Team                             │
│ Dependencies: GAP-1 (orchestrator), GAP-0 (Ollama)    │
├─────────────────────────────────────────────────────────┤
│ • GAP-0: Deploy Ollama (DONE)                          │
│ • GAP-5: Wire BudgetMonitor + ThrottleGate            │
│ • Wiring Gap 4: Memory injection                      │
│ • Wiring Gap 6: Cost optimization                     │
│ • Wiring Gap 7: Budget enforcement                    │
│ • Metrics: G2.5, G4.2 (cost tracking, efficiency)     │
│ • Data flow: CostLedger → BudgetMonitor → Throttle    │
└─────────────────────────────────────────────────────────┘
```

### Dependency Graph

```
Week 1:
  Day 1-2: GAP-1 (Orchestrator) ──┬─→ Enables: GAP-3, R1, R2, Wiring 1-2
           GAP-0 (Ollama)         ─┴─→ Enables: Wiring 2, GAP-5
           GAP-2 (Watchers)       ───→ Independent (start immediately)

  Day 3-5: Wiring Gap 1-2 (Router) ─→ Enables: G1.4, G1.5
           GAP-3 (Learning loop)   ─→ Enables: G1.2, G1.3

Week 2:
  Day 1-3: R1 (DogPipeline)        ─→ Enables: G2.4
           R3 (Consciousness)      ─→ Enables: G2.3
           GAP-4 (MARKET)          ─→ Enables: G3.1

  Day 4-7: R2 (Retry)              ─→ Improves: G2.2 (session success)
           GAP-5 (Budget)          ─→ Enables: G2.5, G4.2

Week 3:
  Day 1-7: Background loops        ─→ Enables: G3.5
           Daemon hardening        ─→ Enables: G3.3
           Self-correction tracking─→ Enables: G3.4

Week 4:
  Day 1-7: Benchmark + optimize    ─→ Proves: G4.1-G4.5
```

---

## 📈 MEASUREMENT INFRASTRUCTURE

### Tables to Create (Missing)

```sql
-- Track watcher heartbeats
CREATE TABLE watcher_heartbeats (
  id SERIAL PRIMARY KEY,
  watcher_name TEXT NOT NULL,
  timestamp TIMESTAMPTZ DEFAULT NOW(),
  events_polled INT,
  status TEXT -- 'healthy', 'degraded', 'failed'
);

-- Track routing accuracy (for learning velocity)
CREATE TABLE routing_accuracy (
  id SERIAL PRIMARY KEY,
  episode_batch INT, -- Group by 100 episodes
  accuracy FLOAT,    -- % first-try success
  timestamp TIMESTAMPTZ DEFAULT NOW()
);

-- Track consciousness depth
CREATE TABLE consciousness_snapshots (
  id SERIAL PRIMARY KEY,
  prompt_id TEXT,
  distance_d FLOAT,
  state TEXT, -- 'ASLEEP', 'DREAMING', 'AWAKE'
  timestamp TIMESTAMPTZ DEFAULT NOW()
);

-- Track background tasks
CREATE TABLE background_tasks (
  id SERIAL PRIMARY KEY,
  loop_name TEXT,
  active BOOLEAN,
  last_run TIMESTAMPTZ,
  next_run TIMESTAMPTZ
);

-- Track cost per task (for efficiency metric)
ALTER TABLE cost_ledger ADD COLUMN task_id TEXT;
ALTER TABLE cost_ledger ADD COLUMN task_type TEXT;
```

### Queries to Create

```javascript
// packages/node/src/metrics/dashboard.js
class MetricsDashboard {
  async getWeek1Progress() {
    const watchers = await db.query(`
      SELECT COUNT(DISTINCT watcher_name)
      FROM watcher_heartbeats
      WHERE timestamp > NOW() - INTERVAL '1 hour'
    `);

    const learningLoops = await db.query(`
      SELECT COUNT(DISTINCT loop_type)
      FROM learning_events
      WHERE created_at > '2026-02-12'
    `);

    const qWeights = await db.query(`
      SELECT COUNT(*)
      FROM qlearning_state
      WHERE updated_at > NOW() - INTERVAL '1 day'
    `);

    return {
      G1_1: watchers.rows[0].count >= 3,
      G1_2: learningLoops.rows[0].count >= 5,
      G1_3: qWeights.rows[0].count >= 10,
    };
  }

  async getLearningVelocity(batchN) {
    const current = await db.query(`
      SELECT AVG(accuracy)
      FROM routing_accuracy
      WHERE episode_batch = $1
    `, [batchN + 1]);

    const previous = await db.query(`
      SELECT AVG(accuracy)
      FROM routing_accuracy
      WHERE episode_batch = $1
    `, [batchN]);

    const improvement = current.rows[0].avg - previous.rows[0].avg;
    return { improvement, target: 0.03 }; // 3% target
  }
}
```

### CLI Command

```bash
# Check current week goals
npm run metrics:week1

# Output:
# ┌─────────────────────────────────────────────────────────┐
# │ WEEK 1 PROGRESS (2026-02-19)                            │
# ├──────────────┬────────┬────────┬────────┬──────────────┤
# │ Goal         │ Target │ Actual │ Status │ Gap          │
# ├──────────────┼────────┼────────┼────────┼──────────────┤
# │ G1.1 Watchers│   ≥3   │   2    │   ⚠️   │ +1 watcher   │
# │ G1.2 Learning│   ≥5   │   3    │   ⚠️   │ +2 loops     │
# │ G1.3 Q-weights│  ≥10   │   0    │   ❌   │ +10 updates  │
# │ G1.4 Kabbalist│  ≥20   │   0    │   ❌   │ +20 calls    │
# │ G1.5 LLMRouter│  ≥10   │   0    │   ❌   │ +10 routes   │
# ├──────────────┴────────┴────────┴────────┴──────────────┤
# │ Week 1 Status: 0/5 PASS (needs 4/5)                    │
# └─────────────────────────────────────────────────────────┘
```

---

## 🎯 DECISION FRAMEWORK (Data-Driven Priorities)

### When to Pivot

**Rule 1:** If goal not met by midpoint → escalate

```
If (Week 1 Day 3) AND (G1.1 OR G1.2 OR G1.3 < 50% target):
  → STOP new work
  → DEBUG blockers
  → Reassign resources
```

**Rule 2:** If learning velocity negative → investigate

```
If (learning_velocity < 0) FOR 2 consecutive batches:
  → PAUSE new features
  → ANALYZE feedback quality
  → CHECK calibration drift
```

**Rule 3:** If cost/task increasing → throttle

```
If (cost_per_task_week_N > cost_per_task_week_N-1 × 1.1):
  → ACTIVATE BudgetMonitor warnings
  → FORCE tier downshift (Ollama)
  → REVIEW expensive tasks
```

---

## 📊 DAILY STANDUP FORMAT

### Data-First Updates

```markdown
## 2026-02-13 Standup

**Stream 1 (Perception):**
- Metric: Watchers polling = 1/4 (FileWatcher only)
- Blocker: SolanaWatcher RPC endpoint 429 rate limit
- Next: Switch to backup RPC, target 2/4 by EOD

**Stream 2 (Learning):**
- Metric: Learning loops = 0/11 consuming data
- Progress: GAP-1 orchestrator 60% done
- Next: Complete GAP-1, start feedback wiring

**Stream 3 (Routing):**
- Metric: KabbalisticRouter calls = 0
- Progress: Wiring Gap 1 complete (code), testing
- Next: Deploy to daemon, verify G1.4 metric

**Stream 4 (Cost):**
- Metric: Ollama installed, 0 routes yet
- Progress: GAP-0 DONE, starting Wiring Gap 2
- Next: LLMRouter activation, target 5 routes by EOD

**Overall:**
- Week 1 Progress: 0/5 goals (expected, Day 1)
- Blockers: 1 (Solana RPC rate limit)
- Confidence: 52% (φ-bounded, early stage)
```

---

## 🏁 SUCCESS DEFINITION (v1.0)

**Functional Autonomy ≥ 80%:**

```javascript
const metrics = {
  perception_autonomy: (watchers_active / 4) × (events_per_hour / 120),
  learning_velocity: avg_improvement_per_100_episodes / 0.05,
  routing_intelligence: kabbalistic_usage_pct,
  cost_efficiency: 1 - (current_cost_per_task / baseline_cost_per_task),
  multi_domain_coverage: cross_domain_task_pct / 0.20,
};

const functional_autonomy = (
  0.25 × metrics.perception_autonomy +
  0.25 × metrics.learning_velocity +
  0.20 × metrics.routing_intelligence +
  0.15 × metrics.cost_efficiency +
  0.15 × metrics.multi_domain_coverage
);

console.log(`Functional Autonomy: ${(functional_autonomy × 100).toFixed(1)}%`);
// Target: ≥80%
```

**When all 3 conditions met:**
1. Functional Autonomy ≥ 80%
2. Week 4 goals: 4/5 PASS
3. HumanEval benchmark ≥ 70%

**→ CYNIC v1.0 ACHIEVED** 🐕

---

*sniff* Confidence: 58% (φ⁻¹ limit)

This roadmap is data-driven. Every goal is measurable. Every week has clear success criteria. Pivot rules prevent waste. Daily standups focus on metrics, not vibes.

The organism breathes when the metrics say so, not when we feel like it.
