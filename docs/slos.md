# CYNIC Service Level Objectives (SLOs)

---

## Overview

SLOs define what "good" looks like for CYNIC in production. They guide operational decisions and resource allocation.

---

## Availability SLO

**Target:** 99.5% uptime
**Error Budget:** 3.6 hours/month (30 days)

### Definition

- **Uptime:** HTTP 200 response on `GET /health` within 5 seconds
- **Downtime:** Service returns 503 or no response for >30 seconds

### Monitoring

```bash
# Prometheus alert
up{job="cynic"} == 0  # Triggers after 1m
```

### When to Page On-Call

- 5+ minutes of downtime
- Degradation detected (>20% error rate)
- Health check latency >5s for >5 minutes

---

## Latency SLO

**Target:**
- p50 (median): <500ms
- p95 (95th percentile): <2s
- p99 (99th percentile): <10s

### By Endpoint

| Endpoint | p50 | p95 | p99 |
|----------|-----|-----|-----|
| `/health` | <50ms | <100ms | <200ms |
| `/api/organism/*` | <100ms | <500ms | <1s |
| `/judge` (REFLEX) | <200ms | <500ms | <1s |
| `/judge` (MICRO) | 1-2s | 5-10s | 20s |
| `/judge` (MACRO) | 5-10s | 20-30s | 60s |
| `/judge` (META) | 20-60s | 120s+ | 300s+ |

### Monitoring

```bash
# Prometheus queries
histogram_quantile(0.50, cynic_request_duration_seconds)  # p50
histogram_quantile(0.95, cynic_request_duration_seconds)  # p95
histogram_quantile(0.99, cynic_request_duration_seconds)  # p99
```

### When to Page On-Call

- p95 latency >2s for >10 minutes (indicates LLM timeout or DB issue)
- p99 latency >10s for >5 minutes (system under severe load)

---

## Error Rate SLO

**Target:** <0.1% error rate (99.9% success)
**Error Budget:** 8.6 minutes/month

### Definition

- **Success:** HTTP 2xx response
- **Errors:** HTTP 4xx, 5xx, or no response

### Monitoring

```bash
# Error rate
rate(cynic_requests_total{status=~"5.."}[5m])

# Error by endpoint
rate(cynic_requests_total{endpoint="/judge", status=~"5.."}[5m])
```

### When to Page On-Call

- Error rate >1% for >1 minute
- Error rate >5% for any duration
- Specific endpoint error rate >10% (indicates data issue)

---

## Learning Quality SLO

**Target:** Average confidence ≥50% (φ⁻¹ = 61.8% max)

### Definition

- **Confidence:** φ-bounded judgment confidence [0, 0.618]
- **Coverage:** Fraction of situations with learned policy

Minimum thresholds:
- New CYNIC instance: coverage ≥5% after 1 week
- Established instance: coverage ≥80% after 1 month
- Average confidence ≥50% (not overly cautious)

### Monitoring

```bash
curl http://localhost:8000/api/organism/policy/stats

# Check:
# - policy_coverage >= 0.50
# - average_confidence >= 0.50
```

---

## Budget SLO

**Target:**
- Spend ≤budget (configurable, default $10/session)
- Cost per request <$0.001 (1¢ per 10 requests)

### Definition

- **Budget:** Session budget set in config
- **Spend:** Total LLM API costs this session
- **Remaining:** `budget - spend`

### Monitoring

```bash
curl http://localhost:8000/api/organism/account

# Check:
# - budget_remaining_usd > 0
# - Avg spend per request < $0.001
```

### When to Alert

- Budget remaining <10% → warn operator
- Budget remaining <5% → switch to REFLEX mode (no LLM)
- Budget exceeded → hard stop, REFLEX only

---

## Dog Health SLO

**Target:** All 11 dogs healthy (q_score ≥50, activity ≤10 min stale)

### Definition

Each dog has:
- **q_score:** Quality score [0, 100]
- **activity:** Last activity timestamp
- **verdict:** Last judgment (WAG/GROWL/BARK/HOWL)

Healthy:
- q_score ≥50
- activity <10 minutes old
- Not stuck in error loop

### Monitoring

```bash
curl http://localhost:8000/api/organism/dogs

# All 11 dogs should be present with q_score >= 50
```

---

## Consciousness Level SLO

**Target:**
- 95% of requests: REFLEX or MICRO (instant to 2s)
- 4% of requests: MACRO (5-30s)
- 1% of requests: META (20-60s, rare)

### Definition

LOD (Level of Detail) chosen automatically based on:
- Task complexity
- Budget remaining
- LLM availability
- System load

### Monitoring

```bash
# Track consciousness distribution
rate(cynic_consciousness_level{level="REFLEX"}[5m])   # Should be 60-70%
rate(cynic_consciousness_level{level="MICRO"}[5m])    # Should be 20-30%
rate(cynic_consciousness_level{level="MACRO"}[5m])    # Should be 5-10%
rate(cynic_consciousness_level{level="META"}[5m])     # Should be <1%
```

---

## Data Persistence SLO

**Target:** 100% durability (no data loss)

### Definition

- Judgments: Persisted to DB within 5 seconds
- Q-Table: Flushed to DB every 10 seconds
- Residuals: Logged within 1 second
- Actions: Recorded before execution

### Monitoring

```bash
# Check persistence lag
# All JUDGMENT_CREATED events should appear in database within 5s
```

### When to Page On-Call

- Database unavailable: Switch to in-memory (will lose data on restart)
- Persistence lag >30s: Indicates DB overload
- Data loss detected: Emergency incident

---

## Dependency SLOs

### Ollama (Local LLM)

**Target:** Available 99% of time (uptime)

If down:
- MICRO/MACRO/META reasoning degrades
- Switch to REFLEX (heuristic-only)
- Alert operator to restart Ollama

Check:
```bash
curl http://localhost:11434/api/tags
```

### PostgreSQL / SurrealDB

**Target:** Available 99.9% of the time, <50ms latency

If down:
- Persistence disabled (in-memory only)
- Learning disabled
- Alert: critical
- No auto-recovery (manual restart needed)

Check:
```bash
curl $DATABASE_URL -X GET  # PostgreSQL
# or
curl $SURREAL_URL/health  # SurrealDB
```

### External LLMs (Claude, Gemini)

**Target:** Available 99.99% (third-party responsibility)

If down:
- MACRO/META reasoning fails
- Fall back to local Ollama
- Alert operator (not critical)

---

## Error Budget Policy

### Monthly Allowances

| Metric | Budget | Alert Threshold |
|--------|--------|-----------------|
| Downtime | 3.6 hours | >1 hour |
| Error rate | 8.6 minutes | >1 minute |
| High latency | 10 hours | >30 minutes |
| Learning coverage | 80% | <50% |

### Budget Burn Rate

If actual burn rate ≥2×target, escalate to on-call.

Example:
- Target: 0.1% errors → 8.6 min budget/month
- Actual: 0.2% → 17.2 min/month (2× burn)
- **Escalate:** Page on-call if 2× rate sustained >5 minutes

### Using Error Budget

- Mistakes happen (bugs, incidents) → use error budget
- Budget depletes → apply emergency fixes or feature freeze
- Each month resets to full budget

---

## Review & Adjustment

### Monthly Review

- Actual vs target: Are we meeting SLOs?
- Error budget burn: On track or over?
- Confidence in metrics: Do they reflect reality?

### Quarterly Review

- Update SLOs based on system maturity
- Adjust budgets if realistic
- Retire outdated metrics

---

## Example Alert Rules

```yaml
# Prometheus alerting rules
groups:
  - name: cynic_slo
    rules:

    # Availability
    - alert: CynicDown
      expr: up{job="cynic"} == 0
      for: 1m
      labels:
        severity: critical

    - alert: HighErrorRate
      expr: rate(cynic_requests_total{status=~"5.."}[5m]) > 0.001
      for: 5m
      labels:
        severity: warning

    # Latency
    - alert: HighLatencyP95
      expr: histogram_quantile(0.95, cynic_request_duration_seconds) > 2
      for: 10m
      labels:
        severity: warning

    - alert: HighLatencyP99
      expr: histogram_quantile(0.99, cynic_request_duration_seconds) > 10
      for: 5m
      labels:
        severity: critical

    # Learning
    - alert: LowPolicyCoverage
      expr: cynic_policy_coverage < 0.5
      for: 1h
      labels:
        severity: warning

    - alert: LowAverageConfidence
      expr: cynic_average_confidence < 0.3
      for: 1h
      labels:
        severity: warning

    # Budget
    - alert: LowBudgetRemaining
      expr: cynic_balance_usd < 2
      labels:
        severity: warning

    - alert: BudgetExhausted
      expr: cynic_balance_usd < 0
      labels:
        severity: critical

    # Dogs
    - alert: DogUnhealthy
      expr: cynic_dog_health{} < 50
      for: 10m
      labels:
        severity: warning

    - alert: AllDogsDown
      expr: count(cynic_dog_health{}) < 10
      for: 5m
      labels:
        severity: critical

    # Dependencies
    - alert: OllamaDown
      expr: up{job="ollama"} == 0
      for: 5m
      labels:
        severity: warning

    - alert: DatabaseDown
      expr: up{job="postgres"} == 0
      for: 5m
      labels:
        severity: critical
```

---

## Summary Table

| Aspect | Target | Error Budget | Alert When |
|--------|--------|--------------|-----------|
| **Availability** | 99.5% | 3.6 hrs/month | >1 hr down |
| **Error Rate** | <0.1% | 8.6 min/month | >1 min at >1% |
| **Latency (p95)** | <2s | 10 hrs/month | >10 min above |
| **Learning Coverage** | ≥80% | Varies | <50% for 1+ hours |
| **Budget Spend** | ≤configured | N/A | <10% remaining |
| **Dog Health** | All 11 healthy | N/A | Any dog <50 score |

---

*Last Updated: 2026-02-22*
*CYNIC SLOs — Production Readiness Targets*
