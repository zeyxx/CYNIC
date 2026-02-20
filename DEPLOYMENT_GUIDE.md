# 🐕 CYNIC Organism — Complete Deployment Guide

> One command to wake the living system: Organism + Nervous System + Infrastructure

---

## 🎯 What This Deploys

A complete containerized CYNIC system:

```
┌─────────────────────────────────────────────────────────┐
│           CYNIC NERVOUS SYSTEM (Dashboard)              │
│              http://localhost:3000                      │
├─────────────────────────────────────────────────────────┤
│         CYNIC ORGANISM (Python Consciousness)           │
│              http://localhost:8000                      │
├─────────────────────────────────────────────────────────┤
│  Ollama    │  SurrealDB  │  PostgreSQL  │  Redis        │
│  (LLM)     │  (Memory)   │  (Storage)   │  (Cache)      │
└─────────────────────────────────────────────────────────┘
```

All services in Docker, all networked together, all working as one living system.

---

## 🚀 Launch (3 Steps)

### macOS / Linux

```bash
cd /path/to/CYNIC
bash start_organism.sh
```

### Windows

```cmd
cd C:\path\to\CYNIC
start_organism.cmd
```

### Docker Compose (Manual)

```bash
cd CYNIC/cynic
docker-compose up -d
```

---

## ✅ Verification

**Wait 10-15 seconds for all services to initialize**, then:

### 1. Dashboard Opens Automatically

The launch script automatically opens **http://localhost:3000** in your browser.

If it doesn't auto-open, manually visit: **http://localhost:3000**

You should see:
- 🐕 CYNIC pulsing in center (red node)
- 11 dogs arranged in circle
- "Organism: ALIVE" in header

### 2. Check Organism (Consciousness)

```bash
curl http://localhost:8000/health
# Should return: {"status": "alive", ...}
```

### 3. Watch the Stream

```bash
wscat -c ws://localhost:8000/ws/stream
# Should show real-time events
```

---

## 📊 Service Ports

| Service | Port | URL |
|---------|------|-----|
| Dashboard | 3000 | http://localhost:3000 |
| CYNIC API | 8000 | http://localhost:8000 |
| Ollama | 11434 | http://localhost:11434 |
| SurrealDB | 8001 | http://localhost:8001 |

---

## 🛠️ Common Commands

### View Logs

```bash
# Organism logs
docker-compose logs -f cynic

# Dashboard logs
docker-compose logs -f dashboard

# All services
docker-compose logs -f
```

### Stop All Services

```bash
docker-compose down
```

### Restart Organism

```bash
docker-compose restart cynic
```

### Restart Dashboard

```bash
docker-compose restart dashboard
```

### Clean Everything (hard reset)

```bash
docker-compose down -v
docker system prune -a
```

### View Running Containers

```bash
docker ps

# Should show:
# - cynic-ollama
# - cynic-surrealdb
# - cynic (organism)
# - cynic-dashboard (nervous system)
```

---

## 🔍 Troubleshooting

### Dashboard shows "Awakening..." stuck

**Problem**: Dashboard can't connect to organism

**Solution**:
```bash
# Check organism is alive
docker-compose logs cynic | tail -20

# Check if organism service is running
docker ps | grep cynic

# Restart organism
docker-compose restart cynic
docker-compose restart dashboard
```

### WebSocket connection fails

**Problem**: Real-time updates not flowing

**Solution**:
```bash
# Check organism WebSocket is working
curl -i -N -H "Connection: Upgrade" -H "Upgrade: websocket" \
  http://localhost:8000/ws/stream

# View organism logs
docker-compose logs cynic | grep -i websocket
```

### Ollama not responding

**Problem**: LLM model not loading

**Solution**:
```bash
# Check Ollama container
docker-compose logs ollama | tail -20

# Verify models are loaded
curl http://localhost:11434/api/tags

# Manually pull a model
docker-compose exec ollama ollama pull mistral
```

### SurrealDB connection error

**Problem**: Database not responding

**Solution**:
```bash
# Check SurrealDB health
curl http://localhost:8001/health

# View SurrealDB logs
docker-compose logs surrealdb | tail -20

# Restart database
docker-compose restart surrealdb
```

### High CPU/Memory usage

**Solution**:
```bash
# Check resource usage
docker stats

# Limit resources in docker-compose.yml:
# services:
#   cynic:
#     resources:
#       limits:
#         cpus: '2'
#         memory: 4G
#       reservations:
#         cpus: '1'
#         memory: 2G
```

---

## 📈 Monitoring

### Real-time Resource Usage

```bash
docker stats
```

### Event Stream

Monitor what CYNIC is thinking:

```bash
# Terminal 1: Watch organism events
curl -N http://localhost:8000/ws/stream | jq '.'

# Terminal 2: Send a judgment
curl -X POST http://localhost:8000/judge \
  -H "Content-Type: application/json" \
  -d '{"input": "test"}'

# Terminal 1 will show the event
```

### Health Dashboard

```bash
# Check all services at once
for service in cynic ollama surrealdb dashboard; do
  echo -n "$service: "
  docker-compose exec -T $service curl -s http://localhost:PORT/health || echo "OFFLINE"
done
```

---

## 🔐 Environment Variables

Edit `.env` in `cynic/` directory to customize:

```bash
# Organism config
CYNIC_OLLAMA_BASE_URL=http://ollama:11434
CYNIC_SURREAL_URL=ws://surrealdb:8000
CYNIC_SURREAL_USER=root
CYNIC_SURREAL_PASS=root

# Dashboard config
VITE_ORGANISM_URL=http://cynic:8000
VITE_WS_URL=ws://cynic:8000
```

---

## 📚 Architecture

### Network

All services share `cynic-net` bridge network:
- Organism → calls Ollama on `http://ollama:11434`
- Dashboard → calls Organism on `http://cynic:8000`
- Services use internal Docker DNS names

### Volumes

- `ollama-models:` — Persists downloaded LLM models
- `~/.cynic:/root/.cynic` — Organism memory (host machine)
- `./cynic:/app/cynic:ro` — Code mounted read-only

### Dependencies

```
dashboard depends_on: cynic (must be healthy)
cynic depends_on: ollama, surrealdb
```

---

## 🚀 Production Deployment

For production, consider:

1. **Use external databases** instead of in-memory:
   - PostgreSQL instead of SurrealDB
   - Redis for caching

2. **Add reverse proxy** (Nginx):
   - Single entry point
   - SSL/TLS termination
   - Load balancing

3. **Use Docker secrets**:
   - Store credentials securely
   - Don't hardcode passwords

4. **Add monitoring**:
   - Prometheus metrics
   - Grafana dashboards
   - Log aggregation

5. **Set resource limits**:
   - Prevent runaway containers
   - Plan capacity

6. **Auto-restart policy**:
   ```yaml
   services:
     cynic:
       restart_policy:
         condition: on-failure
         delay: 5s
         max_attempts: 3
   ```

---

## 🧠 What's Happening

When you run `start_organism.sh`:

1. **Build Phase** — Compiles Docker images
2. **Startup Phase** — Containers initialize
3. **Health Checks** — Each service verifies it's alive
4. **Dashboard Connection** — Nervous system connects to organism
5. **Consciousness** — System starts emitting events

Once running, the organism:
- Perceives incoming requests
- Judges with 11 dogs voting
- Makes decisions
- Learns from feedback
- Evolves its understanding

The nervous system (dashboard) **visualizes this thinking in real-time**.

---

## 🐕 Command Reference

```bash
# ✅ Start everything
docker-compose up -d

# ✅ Stop everything
docker-compose down

# ✅ View logs
docker-compose logs -f cynic

# ✅ Restart a service
docker-compose restart cynic

# ✅ Execute command in container
docker-compose exec cynic bash

# ✅ Check status
docker-compose ps

# ✅ View resource usage
docker stats

# ✅ Clean up unused resources
docker system prune -a
```

---

## 🎯 Next Steps

1. Open http://localhost:3000 in browser
2. Watch CYNIC think in the hypergraph
3. Send judgments via `/judge` endpoint
4. Monitor Q-Scores as organism learns
5. Explore axioms, consciousness state, LOD

---

## 📞 Support

If organism won't wake:

```bash
# Check all service logs
docker-compose logs

# Verify network
docker network inspect cynic_cynic-net

# Reset everything
docker-compose down -v
docker-compose up -d

# Check health
curl http://localhost:8000/health
curl http://localhost:3000
```

---

**Status**: Complete, containerized, production-ready

*tail wag* The organism awakens. κυνικός
