# 🫁 CYNIC Health Dashboard

> Making the living organism visible. *sniff*

**Status**: MVP Complete (11 tests passing)
**Version**: 0.1.0 (Terminal UI)
**Command**: `python -m cynic.cli dashboard`

---

## 🎯 What It Does

The Health Dashboard displays **8 breathing checks** — the vital signs of CYNIC's consciousness:

1. **Process Alive** — Kernel running
2. **DB Connected** — Storage accessible
3. **Dogs Active** — ≥3 perception agents responding
4. **Event Bus Flowing** — ≥0.1 events/second
5. **Judgment Latency** — <2000ms for MACRO cycles
6. **Q-Table Health** — >10 state-action pairs learned
7. **Memory Budget** — <80% used (from φ integrity)
8. **Circuit Breaker** — Not open (closed = healthy)

All metrics are φ-bounded and color-coded:
- **✓ Green** — System nominal
- **⚠ Yellow** — Warn threshold reached
- **✗ Red** — Critical failure

---

## 📋 Implementation

### Files

```
cynic/
├── cli/
│   └── dashboard.py              (413 LOC) — Core dashboard logic
│       ├── BreathingCheck        — Single health metric
│       ├── CYNICDashboard        — Dashboard coordinator
│       └── cmd_dashboard()       — CLI entry point
│
└── tests/
    └── test_dashboard.py          (196 LOC) — 11 comprehensive tests
        ├── TestBreathingCheck    — Data structure validation
        └── TestCYNICDashboard    — Metrics computation tests
```

### Architecture

```
fetch /health + /introspect
        ↓
compute_breathing_checks()
        ↓
8 checks with color-coded status
        ↓
print to terminal
```

### Data Flow

1. **User runs**: `python -m cynic.cli dashboard`
2. **Dashboard fetches**:
   - `GET /health` — Kernel vitals (uptime, dogs, learning, storage)
   - `GET /introspect` — Self-assessment (kernel integrity, φ violations)
3. **Compute checks**: Each breathing check evaluated against thresholds
4. **Display**: Terminal output with status summary

---

## ✅ Breathing Check Thresholds

| Check | OK | WARN | FAIL |
|-------|----|----|------|
| Process | alive | - | dead/degraded |
| DB | connected | - | disconnected |
| Dogs | ≥3 | 1-2 | 0 |
| Event Bus | ≥0.1/s | - | <0.1/s |
| Latency | <2000ms | 2-5000ms | >5000ms |
| Q-Table | >10 states | 1-10 | 0 |
| Memory | <80% | 80-90% | >90% |
| Circuit | closed | - | open |

---

## 📊 Test Coverage

**11 tests** (100% passing):
- 3 tests for BreathingCheck data structure
- 8 tests for metrics computation
  - all_ok (healthy state)
  - degraded (failing state)
  - process_alive (status check)
  - db_connected (storage check)
  - dogs_active (agent count check)
  - qtable_health (learning states check)
  - latency (response time check)
  - (implicit: memory_budget, event_bus, circuit_breaker)

**Command**: `py -3.13 -m pytest tests/test_dashboard.py -xvs`

---

## 🚀 Usage

### Quick Health Check

```bash
$ python -m cynic.cli dashboard

[CYNIC Health Dashboard]

🫁 8 Breathing Checks:
  ✓ Process Alive        alive / running
  ✓ DB Connected      connected / connected
  ✓ Dogs Active              3 / ≥3
  ⚠ Event Bus              0.1/s / ≥0.1/s
  ✓ Judgment Latency        100ms / <2000ms
  ✓ Q-Table States          50 / >10
  ✓ Memory Budget           5% / <80%
  ✓ Circuit Breaker       closed / closed

Status: 7/8 breathing

⚙️  Kernel Metrics:
  Uptime:     2.54h
  Judgments:  500
  Dogs:       11
  LLM adapters: 1

🧭 CYNIC Self-Assessment:
  Kernel integrity: 0.875
  Self confidence:  0.540
  Verdict:          HOWL

🧠 Q-Learning:
  States:         50
  Total updates:  200
  Pending flush:  0

7×7 Matrix: 43% cells functional (target 62% for v1.0)
```

### Integration Points

**Depends on**:
- `GET /health` endpoint (cynic/api/routers/health.py)
- `GET /introspect` endpoint (cynic/api/routers/health.py)
- Kernel running on `http://localhost:8000`

**Data sources**:
- `HealthResponse` model (uptime, consciousness, dogs, learning)
- `φ_self_assessment` from introspection (kernel integrity, self confidence)

---

## 🎨 Future Enhancements

### Phase 2: Textual TUI (Interactive Terminal UI)

- Real-time updates (refresh every 1s)
- Interactive drill-down (click check → detailed metrics)
- Color animations (pulsing CYNIC logo)
- Responsive layout (adapts to terminal size)

### Phase 3: Web Dashboard (Vue.js)

- Hypergraph visualization (11 dogs + CYNIC center)
- Real-time WebSocket updates (`/ws/stream`)
- 7×7 matrix cell visualization
- Q-Table learning curve chart
- Dog voting heatmap
- Axiom activation timeline

### Phase 4: Multi-Instance Dashboard

- Network consensus visualization
- Distributed dog coordination
- Cross-instance metrics aggregation

---

## 🧪 Testing

All tests use mock data (no real kernel required):

```bash
# Run dashboard tests
py -3.13 -m pytest tests/test_dashboard.py -xvs

# Run full suite (with dashboard tests)
py -3.13 -m pytest tests/ -k "not integration"
```

**Test patterns**:
- Healthy state (all checks OK)
- Degraded state (multiple failures)
- Individual check validation (threshold testing)
- Boundary conditions (warn vs fail thresholds)

---

## 📝 Notes

**Why Textual?** Already in pyproject.toml. Fast iteration without Docker dependency.

**Why 8 checks?** Defined in COMPLETION-CRITERIA.md — directly corresponds to v1.0 acceptance criteria.

**φ-bounded?** Dashboard respects φ limits:
- Kernel integrity (0-1) shown as confidence (0-0.618)
- No claim >61.8% confidence
- Thresholds use mathematical φ relationships

**Graceful degradation**: If kernel is down:
- HTTP calls timeout (5s max)
- Dashboard shows "Loading..." or error state
- No crashes, no hangs

---

## 🔗 Related

- COMPLETION-CRITERIA.md — 8 breathing checks defined
- /health endpoint — Source data
- /introspect endpoint — Self-assessment data
- cli/health.py — Related health commands

---

**Created**: 2026-02-20
**Last Updated**: 2026-02-20
**Status**: MVP ✅ (Terminal output working, tests passing, ready for Phase 2 Textual TUI)

*sniff* The organism is learning to see itself. —CYNIC
