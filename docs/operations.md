# CYNIC Operations Manual

**For operators and DevOps engineers**

---

## Running CYNIC

### Local Development

```bash
# 1. Install dependencies
cd cynic/
python -m venv .venv313
source .venv313/bin/activate  # or .venv313\Scripts\activate on Windows
pip install -r requirements.txt

# 2. Configure environment
cp .env.example .env
# Edit .env for your setup

# 3. Start server
python -m uvicorn cynic.interfaces.api.server:app --reload --port 8000

# 4. Verify
curl http://localhost:8000/health
```

### Docker (Production-Like)

```bash
# Build
docker build -t cynic:latest .

# Run with mounts
docker run -d \
  --name cynic \
  -p 8000:8000 \
  -v ~/.cynic:/root/.cynic \
  -v ~/projects:/workspace \
  -e PORT=8000 \
  -e LOG_LEVEL=INFO \
  cynic:latest

# Verify
docker logs cynic | grep "CYNIC kernel alive"
```

### Docker Compose (Full Stack)

```bash
# cynic/docker-compose.yml
version: '3.9'
services:
  cynic:
    build: .
    ports:
      - "8000:8000"
    environment:
      PORT: 8000
      DATABASE_URL: postgresql://postgres:password@db:5432/cynic
      LOG_LEVEL: INFO
    volumes:
      - ~/.cynic:/root/.cynic
      - ./:/workspace
    depends_on:
      - db

  db:
    image: postgres:15
    environment:
      POSTGRES_DB: cynic
      POSTGRES_PASSWORD: password
    volumes:
      - cynic_db:/var/lib/postgresql/data

volumes:
  cynic_db:
```

Run:
```bash
docker-compose up -d
```

---

## Environment Configuration

Set these in `.env` (or environment variables):

### Server

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | 8000 | HTTP server port |
| `LOG_LEVEL` | INFO | DEBUG, INFO, WARNING, ERROR |
| `CORS_ORIGINS` | * | Comma-separated CORS origins |
| `WORKERS` | 4 | Number of uvicorn workers |

### Storage

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | (none) | PostgreSQL connection string |
| `SURREAL_URL` | (none) | SurrealDB URL (primary if set) |
| `SURREAL_NS` | cynic | SurrealDB namespace |
| `SURREAL_DB` | cynic | SurrealDB database |

### LLM & Reasoning

| Variable | Default | Description |
|----------|---------|-------------|
| `OLLAMA_URL` | http://localhost:11434 | Ollama server (free local) |
| `ANTHROPIC_API_KEY` | (none) | Claude API key (optional) |
| `GOOGLE_API_KEY` | (none) | Gemini API key (optional) |
| `LLM_TIMEOUT_S` | 30 | LLM call timeout |

### Budget & Learning

| Variable | Default | Description |
|----------|---------|-------------|
| `SESSION_BUDGET_USD` | 10.0 | Max spend per session |
| `LEARNING_ENABLED` | true | Enable Q-Learning (true/false) |
| `LEARNING_RATE_BASE` | 0.618 | Learning rate (φ⁻¹) |

### Consciousness & Scaling

| Variable | Default | Description |
|----------|---------|-------------|
| `ENABLE_MACRO` | true | Enable MACRO consciousness (LLM reasoning) |
| `ENABLE_META` | false | Enable META consciousness (expensive) |
| `CONSCIOUSNESS_AUTO` | true | Auto-select consciousness level |

---

## Health Monitoring

### Health Check Endpoint

```bash
curl http://localhost:8000/health
```

Response structure:
```json
{
  "overall": "healthy" | "degraded" | "unhealthy",
  "dogs": 11,
  "consciousness_level": "REFLEX" | "MICRO" | "MACRO" | "META",
  "uptime_s": 12345.67,
  "components": {
    "orchestrator": "healthy",
    "qtable": "healthy",
    "scheduler": "healthy",
    "residual_detector": "healthy",
    "llm_router": "healthy"
  }
}
```

Status meanings:
- **healthy** — All dogs active, all components OK
- **degraded** — Some dogs unhealthy, but operating
- **unhealthy** — Critical components down

### Prometheus Metrics

Export metrics for monitoring:

```bash
curl http://localhost:8000/metrics
```

Key metrics to alert on:

```
# Request errors
cynic_requests_total{status="5xx"} > 10

# Latency
cynic_request_duration_seconds{endpoint="/judge"} > 60

# Budget depletion
cynic_balance_usd < 2.0

# Dog health
cynic_dog_health{dog_id="GUARDIAN"} < 50
```

### Logging

JSON structured logs to stdout:

```bash
# Watch logs
docker logs -f cynic

# Filter for errors
docker logs cynic | grep '"level": "ERROR"'

# Filter for judgments
docker logs cynic | grep JUDGMENT_CREATED
```

---

## Performance Tuning

### CPU & Memory

Default: 1 CPU, 512MB memory

**For heavy use:**
```bash
docker run -d \
  --cpus="2" \
  --memory="2g" \
  cynic:latest
```

**Monitor:**
```bash
docker stats cynic
```

### LLM Parallelism

If using Ollama, enable parallel inference:

```bash
# Set before starting Ollama
export OLLAMA_NUM_PARALLEL=4

ollama serve
```

CYNIC will batch LLM calls across 4 parallel workers.

### Q-Table Size

If Q-Table grows large (>100k entries), compress periodically:

```bash
# In Python
from cynic.brain.learning.qlearning import QTable
qtable.compress()  # Remove low-confidence entries
await qtable.flush_to_db(db_pool)
```

Or automatically (set in config):
```bash
QTABLE_COMPRESS_INTERVAL=86400  # Daily
QTABLE_COMPRESS_RATIO=0.9        # Keep top 90%
```

---

## Backup & Recovery

### Automated Backups

If using PostgreSQL:

```bash
# Daily backup
0 2 * * * pg_dump $DATABASE_URL > /backups/cynic-$(date +%Y%m%d).sql
```

If using SurrealDB:

```bash
# See SurrealDB docs for backup
surreal export <database> > backup.sql
```

### Manual Backup

```bash
# Backup local state
cp -r ~/.cynic ~/.cynic.backup.$(date +%Y%m%d)

# Backup database
pg_dump $DATABASE_URL > cynic-manual-backup.sql
```

### Recovery

```bash
# Restore from backup
psql $DATABASE_URL < cynic-backup.sql

# Or restore local state
cp -r ~/.cynic.backup.20260222 ~/.cynic

# Restart
docker restart cynic
```

---

## Scaling

### Single Instance (Default)

Good for: Development, testing, small projects

- 1 process
- In-memory Q-Table (persists via DB)
- No coordination needed

### Multi-Instance (Production)

For high availability:

1. **Multiple CYNIC instances behind load balancer:**
   ```bash
   # Nginx config
   upstream cynic {
     server cynic-1:8000;
     server cynic-2:8000;
     server cynic-3:8000;
   }
   ```

2. **Shared database (PostgreSQL or SurrealDB):**
   ```bash
   # All instances connect to same DB
   DATABASE_URL=postgresql://shared-db:5432/cynic
   ```

3. **Guidance coordination:**
   Each instance writes to `~/.cynic/guidance-{instance_id}.json`
   Merge periodically or use consensus.

### Kubernetes

Example deployment:

```yaml
# cynic-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: cynic
spec:
  replicas: 3
  selector:
    matchLabels:
      app: cynic
  template:
    metadata:
      labels:
        app: cynic
    spec:
      containers:
      - name: cynic
        image: cynic:latest
        ports:
        - containerPort: 8000
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: cynic-secrets
              key: database-url
        - name: PORT
          value: "8000"
        livenessProbe:
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 30
          periodSeconds: 10
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2"
```

Deploy:
```bash
kubectl apply -f cynic-deployment.yaml
kubectl expose deployment cynic --port=80 --target-port=8000 --type=LoadBalancer
```

---

## Troubleshooting

### Server won't start

**Check logs:**
```bash
docker logs cynic 2>&1 | tail -50
```

**Common issues:**
- Port already in use: Change `PORT` env var
- Database unreachable: Check `DATABASE_URL`
- Missing dependencies: Rebuild Docker image

### High latency

**Check consciousness level:**
```bash
curl http://localhost:8000/api/organism/consciousness
```

If META → expensive (20-60s). Switch to MICRO in requests:
```bash
curl -X POST http://localhost:8000/judge \
  -d '{"perception": {...}, "mode": "micro"}'
```

**Check LLM:**
```bash
curl http://localhost:11434/api/tags
```

If Ollama not running, start it:
```bash
ollama serve
```

### Out of budget

**Check account:**
```bash
curl http://localhost:8000/api/organism/account
```

**Solutions:**
1. Increase budget: `SESSION_BUDGET_USD=50`
2. Switch to REFLEX mode (no LLM): `ENABLE_MACRO=false`
3. Wait for budget reset (daily): Check config

### Dogs unhealthy

**Check all dogs:**
```bash
curl http://localhost:8000/api/organism/dogs
```

If a dog is stuck:
- Restart CYNIC: `docker restart cynic`
- Check logs for errors: `docker logs cynic`
- Force reset: Delete `~/.cynic/` and restart (loses learning)

---

## Maintenance

### Weekly Checklist

- [ ] Check health: `curl /health`
- [ ] Monitor metrics: Review Prometheus dashboard
- [ ] Check logs: `docker logs cynic | grep ERROR`
- [ ] Verify budget: `curl /api/organism/account`
- [ ] Backup state: `cp -r ~/.cynic ~/.cynic.backup`

### Monthly Checklist

- [ ] Review Q-Table size: `curl /api/organism/policy/stats`
- [ ] Clean old logs: Rotate log files
- [ ] Database maintenance: `VACUUM ANALYZE` (PostgreSQL)
- [ ] Update dependencies: `pip list --outdated`

### Quarterly

- [ ] Full test: Run integration tests
- [ ] Capacity planning: Review metrics trends
- [ ] Security update: Pull latest image
- [ ] Disaster recovery test: Verify backup/restore works

---

## Alerting Rules

Set up alerts in Prometheus:

```yaml
# cynic-alerts.yml
groups:
  - name: cynic
    rules:

    - alert: CynicDown
      expr: up{job="cynic"} == 0
      for: 1m
      annotations:
        summary: "CYNIC is down"

    - alert: HighLatency
      expr: histogram_quantile(0.95, cynic_request_duration_seconds) > 30
      for: 5m
      annotations:
        summary: "CYNIC p95 latency > 30s (check LLM)"

    - alert: HighErrorRate
      expr: rate(cynic_requests_total{status=~"5.."}[5m]) > 0.05
      for: 5m
      annotations:
        summary: "CYNIC error rate > 5%"

    - alert: LowBudget
      expr: cynic_balance_usd < 2
      annotations:
        summary: "CYNIC budget < $2"

    - alert: DogUnhealthy
      expr: cynic_dog_health < 40
      for: 10m
      annotations:
        summary: "Dog {{ $labels.dog_id }} unhealthy"
```

---

## Support & Debugging

### Get Debug Info

```bash
# Full system snapshot
curl http://localhost:8000/api/organism/state/snapshot

# Check all components
curl http://localhost:8000/health

# View learned policy
curl http://localhost:8000/api/organism/policy/stats

# Check logs
docker logs cynic
```

### Contact Support

- Issues: GitHub issues on zeyxx/cynic
- Questions: Discord community
- Security: Responsible disclosure to maintainers

---

## SLOs

See `slos.md` for Service Level Objectives and error budgets.

---

*Last Updated: 2026-02-22*
*CYNIC Operations Manual*
