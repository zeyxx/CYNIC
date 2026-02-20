# 🐳 CYNIC Docker Deployment Guide — Phase 3

> **Status**: Production-Ready (v1.0 validation)
> **Date**: 2026-02-20
> **Confidence**: 58% (φ-bounded)

---

## Overview

This guide activates CYNIC as a containerized multi-service organism:

```
CYNIC Kernel (port 8000)
    ├── Health: ✅ ALIVE
    ├── Ollama LLM (port 11434)
    ├── SurrealDB (port 8001)
    └── Dashboard (port 3000)
```

**Requirements:**
- Docker Engine ≥24.0
- Docker Compose ≥2.0
- 4GB RAM available
- 2GB disk for models (Ollama)

---

## Phase 3A: Build & Validate

### Step 1: Build CYNIC Kernel Image

```bash
cd /path/to/CYNIC
docker build -t cynic-kernel:latest .
```

**What happens:**
- Python 3.13 + Poetry dependencies installed
- Claude CLI installed globally (enables ACTION execution)
- Non-root `cynic` user created (security)
- HEALTHCHECK endpoint configured

**Expected output:**
```
Successfully tagged cynic-kernel:latest
```

### Step 2: Validate Image

```bash
docker images | grep cynic-kernel
```

Should show:
```
cynic-kernel   latest    <hash>    <size>    <date>
```

### Step 3: Test Claude CLI in Container

```bash
docker run --rm cynic-kernel:latest which claude
```

Should output:
```
/home/cynic/.npm/bin/claude
```

---

## Phase 3B: Deploy Full Stack

### Step 1: Start All Services

```bash
cd cynic
docker-compose up -d
```

**What happens:**
1. CYNIC kernel starts (waits for Ollama + SurrealDB)
2. Ollama starts (downloads/loads gemma2:2b model ~5GB, ~30sec)
3. SurrealDB starts (memory database, instant)
4. Dashboard starts (Vue.js frontend, instant)

**Expected timeline:**
```
T+0s:   Services launching
T+15s:  Ollama model loading...
T+30s:  SurrealDB HEALTHY
T+35s:  CYNIC kernel connects to Ollama
T+40s:  Dashboard available
T+50s:  All services HEALTHY ✓
```

### Step 2: Verify Health

```bash
# Check all containers running
docker ps --filter "label=com.docker.compose.project=cynic" --format "table {{.Names}}\t{{.Status}}"

# Expected output:
# cynic               Up X seconds (healthy)
# cynic-ollama        Up X seconds (healthy)
# cynic-surrealdb     Up X seconds (healthy)
# cynic-dashboard     Up X seconds (healthy)
```

### Step 3: Test Endpoints

```bash
# CYNIC Kernel
curl http://localhost:8000/health | python3 -m json.tool | head -20

# Ollama LLM
curl http://localhost:11434/api/tags | python3 -m json.tool | head -20

# SurrealDB (should error — no auth, expected)
curl http://localhost:8001/health 2>/dev/null || echo "SurrealDB listening"

# Dashboard
open http://localhost:3000 # or curl -I http://localhost:3000
```

---

## Phase 3C: Run Empirical Validation

Once all services are healthy:

```bash
python3.13 scripts/empirical_campaign.py --max-judgments 50
```

**Expected output:**
```
[empirical_campaign] INFO: Kernel is alive ✓
[empirical_campaign] INFO: Found 50 Python files
[empirical_campaign] INFO: Starting judgment loop (50 files)...
...
======================================================================
EMPIRICAL CAMPAIGN SUMMARY
======================================================================
Campaign ID: 2026-02-20T...
Judgments: 35+
Mean latency: 50-100ms
Mean Q-Score: 40-60
Q-Table states: 8+
Verdicts: Distributed (BARK/WAG/GROWL)
======================================================================
```

---

## Phase 3D: Monitoring

### Live Logs

```bash
# All containers
docker-compose logs -f

# Specific service
docker-compose logs -f cynic

# Real-time health
watch -n 5 'curl -s http://localhost:8000/health | python3 -m json.tool | head -15'
```

### Access Dashboard

Open browser: **http://localhost:3000**

Real-time visualization of:
- Consciousness state (REFLEX/MICRO/MACRO/META)
- Dogs voting activity
- Q-Table learning
- Judgment stream

### Performance Metrics

```bash
# CPU/Memory usage
docker stats cynic cynic-ollama cynic-surrealdb

# Judgment latencies
curl http://localhost:8000/introspect | python3 -c "
import sys, json
data = json.load(sys.stdin)
handlers = data['handlers']
for h in handlers:
    if 'latency' in h:
        print(f'{h[\"id\"]}: {h[\"latency_ms\"]:.1f}ms')
"
```

---

## Phase 3E: Production Deployment

### Option 1: Local Docker (Development)

Already done — services available at `localhost:8000`

### Option 2: Docker Swarm (Multi-Node)

```bash
# Initialize swarm
docker swarm init

# Deploy stack
docker stack deploy -c docker-compose.yml cynic

# Monitor
docker service ls
docker service logs cynic_cynic
```

### Option 3: Kubernetes (Cloud)

See `KUBERNETES_DEPLOYMENT.md` (separate guide)

### Option 4: Render/Cloud Services

See `RENDER_DEPLOYMENT.md` (separate guide)

---

## Troubleshooting

### Issue: "Ollama connection refused"

```bash
# Check Ollama is responding
curl http://localhost:11434/api/tags

# If failing, restart Ollama
docker-compose restart ollama

# Wait 30s for model to load
sleep 30

# Verify kernel can reach it (inside container)
docker exec cynic curl http://ollama:11434/api/tags
```

### Issue: "Claude CLI not found in action execution"

```bash
# Verify installation
docker exec cynic which claude

# If missing, rebuild image
docker build --no-cache -t cynic-kernel:latest .

# Restart
docker-compose restart cynic
```

### Issue: "SurrealDB connection failed"

```bash
# Check SurrealDB is running
docker ps | grep surrealdb

# Verify connectivity
docker exec cynic curl -s ws://surrealdb:8000/health

# If failing, check logs
docker logs cynic-surrealdb
```

### Issue: "MACRO consciousness cycles=0"

**Expected behavior**: MACRO cycles only trigger for complex judgments.
- REFLEX/MICRO cycles are more common (~90%)
- MACRO cycles are expensive, triggered only when needed
- Cycles counter in `/health` endpoint shows all levels

Check that system is actually running judgments:
```bash
curl http://localhost:8000/health | grep "cycles"
```

Should show increasing numbers over time.

---

## Useful Commands

```bash
# Full stack restart
docker-compose restart

# Scale handlers (future)
docker-compose up -d --scale cynic=3

# Logs with timestamps
docker-compose logs --timestamps -f

# Shell access
docker exec -it cynic bash

# Resource limits (docker-compose v3.8+)
# Edit docker-compose.yml → add 'resources:' section per service

# Prune old containers
docker system prune -f

# Export metrics
docker stats --no-stream --format json > metrics.json
```

---

## Success Criteria

✅ All services healthy:
```bash
docker-compose ps  # All "Up" with "(healthy)"
```

✅ Kernel responding:
```bash
curl -s http://localhost:8000/health | grep "alive"
```

✅ Judgments running:
```bash
curl http://localhost:8000/health | grep "total"  # Should increment
```

✅ Learning active:
```bash
curl http://localhost:8000/health | grep "updates"  # Should increment
```

✅ Dashboard visible:
```
http://localhost:3000  # Shows live data stream
```

---

## Next Steps (v1.1+)

- [ ] Multi-instance CYNIC (3 dogs, parallel judgment)
- [ ] Distributed consensus (gossip protocol, network resilience)
- [ ] Persistent storage (PostgreSQL, long-term memory)
- [ ] GPU acceleration (Ollama with CUDA/Vulkan)
- [ ] Integration tests (end-to-end validation)

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    CYNIC Organism v1.0                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐     ┌──────────────┐  ┌─────────────┐    │
│  │ CYNIC Kernel │◄───►│ Ollama LLM   │  │ SurrealDB   │    │
│  │ (FastAPI)    │     │ (gemma2:2b)  │  │ (Memory)    │    │
│  │ 8000         │     │ 11434        │  │ 8001        │    │
│  └──────┬───────┘     └──────────────┘  └─────────────┘    │
│         │                                                    │
│         └────────────────────┬─────────────────────────────┤
│                              │                              │
│                         ┌────▼────┐                        │
│                         │Dashboard │                        │
│                         │(Vue.js)  │                        │
│                         │3000      │                        │
│                         └──────────┘                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘

 Judgment Flow:
   Client → POST /judge → CYNIC Kernel
            → Ollama (temporal scoring)
            → Dogs (consensus voting)
            → SurrealDB (learning storage)
            → Response with Q-Score + Verdict

 Consciousness Cycle: PERCEIVE → JUDGE → DECIDE → ACT → LEARN
```

---

**Last Updated**: 2026-02-20
**Version**: 1.0
**Author**: CYNIC Bootstrap Team
**License**: φ (Open-source, credit required)
