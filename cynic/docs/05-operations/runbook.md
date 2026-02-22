# CYNIC Operations Runbook

> **Operational procedures for CYNIC daemon and infrastructure**
>
> *🐕 κυνικός | "The dog that watches the dog"*

---

## Table of Contents

1. [Health Checks](#health-checks)
2. [Startup/Shutdown](#startupshutdown)
3. [Monitoring](#monitoring)
4. [Incident Response](#incident-response)
5. [Troubleshooting](#troubleshooting)

---

## Health Checks

### Quick Check (10s)

```bash
npm run alive
```

**Expected Output:**

```
✅ Database: connected
✅ Migrations: 47 applied
✅ Daemon: PID 46420
✅ Learning: 30 events/24h
✅ Q-Episodes: 2,820/day
✅ Budget: abundant
⚠️ Watchers: 2/3 (MarketWatcher TODO)
⚠️ Learning Loops: 1/11 (fragmentation)

BREATHING: 6/8 checks (75%) - ABOVE φ⁻¹ ✓
```

### Full Check (60s)

```bash
node scripts/ralph-comprehensive-test.js
```

### HTTP Endpoints

```bash
# Health check
curl http://localhost:6180/health

# Prometheus metrics
curl http://localhost:6180/metrics
```

---

## Startup/Shutdown

### Normal Startup

```bash
# Start daemon
npm run daemon:start

# Verify status
npm run alive
```

### Debug Startup

```bash
CYNIC_DEBUG=true npm run daemon:start
```

### Graceful Shutdown

```bash
# Send SIGTERM
kill -TERM $(cat /tmp/cynic-daemon.pid)

# Wait 30s max for flush
```

### Forced Shutdown

```bash
kill -9 $(cat /tmp/cynic-daemon.pid)
```

---

## Monitoring

### Metrics Available

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `cynic_uptime_seconds` | Daemon uptime | < 60s = restart |
| `cynic_judgments_total` | Total judgments | N/A |
| `cynic_q_score_avg` | Average Q-Score | < 50 = warning |
| `cynic_learning_events` | Learning events/24h | < 10 = stalled |
| `cynic_budget_remaining` | Budget left | < $3.82 = critical |
| `cynic_event_loop_lag_ms` | Event loop lag | > 500ms = degraded |

### Grafana Dashboard

Key panels to monitor:

1. **Organism Health** - Breathing checks (6/8 minimum)
2. **Learning Velocity** - Q-updates per day
3. **LLM Routing** - Distribution across models
4. **Budget** - Daily spend vs limit

---

## Incident Response

### Incident Severity Levels

| Level | Name | Criteria | Response Time |
|-------|------|----------|---------------|
| P0 | Critical | Daemon down, data loss | 15 min |
| P1 | High | Learning stalled, budget exhausted | 1 hour |
| P2 | Medium | Degraded performance | 4 hours |
| P3 | Low | Minor issues | 24 hours |

### P0: Daemon Crash

```bash
# 1. Check if daemon is running
ps aux | grep cynic-daemon

# 2. If not, check last logs
tail -100 /var/log/cynic/daemon.log

# 3. Restart daemon
npm run daemon:start

# 4. Verify recovery
npm run alive
```

### P1: Learning Stalled

```bash
# 1. Check Q-updates
psql -c "SELECT COUNT(*) FROM learning_events WHERE created_at > NOW() - INTERVAL '24 hours'"

# 2. If < 10, check why
# 3. Verify Judge is working
curl http://localhost:6180/health | jq '.learning'

# 4. Restart learning pipeline if needed
curl -X POST http://localhost:6180/admin/restart-learning
```

### P1: Budget Exhausted

```bash
# 1. Check budget state
curl http://localhost:6180/budget

# 2. Force local-only mode
curl -X POST http://localhost:6180/admin/budget-mode -d '{"mode":"local_only"}'

# 3. Notify team
```

---

## Troubleshooting

### PostgreSQL Connection Issues

```bash
# Check PostgreSQL status
docker ps | grep postgres

# Test connection
psql postgresql://cynic:password@localhost:5432/cynic -c "SELECT 1"

# Restart if needed
docker compose restart postgres
```

### Event Loop Lag

```bash
# Check current lag
curl http://localhost:6180/health | jq '.event_loop_lag_ms'

# If > 500ms, reduce watcher scope
# Edit config: watchers.scope = ['packages/', 'scripts/']
# Restart daemon
```

### Memory Leak

```bash
# Check memory usage
curl http://localhost:6180/health | jq '.memory'

# If > 80%, force GC
curl -X POST http://localhost:6180/admin/gc

# If persists, restart daemon
npm run daemon:restart
```

### LLM API Failures

```bash
# Check LLM health
curl http://localhost:6180/llm/status

# Test each adapter
curl http://localhost:6180/llm/test/ollama
curl http://localhost:6180/llm/test/claude

# Force fallback to local
curl -X POST http://localhost:6180/admin/llm-mode -d '{"mode":"local_only"}'
```

---

## Maintenance Windows

### Daily (Automated)

- [ ] Health check cron (every 5 min)
- [ ] Learning pipeline cycle (every 60s)
- [ ] Budget reset at midnight UTC

### Weekly (Manual)

- [ ] Review Q-Learning convergence
- [ ] Check calibration (ECE)
- [ ] Clear old learning events (>30 days)

### Monthly (Manual)

- [ ] Database vacuum analyze
- [ ] Rotate logs
- [ ] Update dependencies (patch versions only)

---

## Contact & Escalation

| Issue | First Contact | Escalation |
|-------|---------------|------------|
| Infrastructure | DevOps team | Maintainer |
| LLM/API | Backend team | Maintainer |
| Data/DB | DBA team | Maintainer |
| Security | Security team | Immediate escalation |

---

*Runbook version: 1.0*
*Last updated: 2026-02-22*
*🐕 κυνικός | "The dog that watches the dog"*