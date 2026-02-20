# CYNIC Docker Deployment

> "Deploy the organism" — κυνικός
> φ = 1.618 ≈ golden ratio ≈ optimal system design

## Quick Start

### 1. Start the full stack (CYNIC + Ollama + SurrealDB)

```bash
cd cynic
docker-compose up --build
```

**Wait for health checks** (watch container logs):
```
cynic-kernel     | INFO:     Uvicorn running on http://0.0.0.0:8000
ollama           | Ready
surrealdb        | Ready
```

### 2. Verify all services running

```bash
# Terminal 2: Check health endpoints
curl http://localhost:8000/health
curl http://localhost:11434/api/tags
```

### 3. Run integration tests

```bash
# Terminal 3: From CYNIC project root (not cynic/)
cd ..
py -3.13 -m pytest -m integration CYNIC/cynic/cynic/tests/test_integration_real_*.py -v
```

---

## Service Details

### cynic-kernel (FastAPI)
- **Port**: 8000
- **Health check**: `curl http://localhost:8000/health`
- **API docs**: `http://localhost:8000/docs`
- **Depends on**: ollama, surrealdb

### ollama (Local LLM)
- **Port**: 11434
- **Health check**: `curl http://localhost:11434/api/tags`
- **Default model**: (pulled on demand)
- **First run**: May pull model (100MB+) and take 1-2 minutes

### surrealdb (Document database)
- **Port**: 8000 (WebSocket)
- **Health check**: `curl http://localhost:8000/health`
- **Credentials**: user=root, pass=root
- **Mode**: In-memory (data lost on restart)

---

## Integration Test Flow

```
┌─────────────────┐
│  Docker Compose │
│  (3 services)   │
└────────┬────────┘
         │
         ├─ ollama:11434 (Model inference)
         ├─ surrealdb:8000 (Persistence)
         └─ cynic-kernel:8000 (Judgment engine)

         ↓ Tests connect to these services

         ├─ test_integration_real_ollama.py
         │  └─ Tests: real Ollama calls, MCTS, model discovery
         │
         └─ test_integration_real_surrealdb.py
            └─ Tests: persistence, schema, vector search

         Result: Empirical validation that φ encoding works
                 with real external dependencies
```

---

## Common Commands

### Start services in background
```bash
cd cynic
docker-compose up -d
```

### Stop services
```bash
docker-compose down
```

### View logs
```bash
docker-compose logs -f cynic-kernel  # Follow CYNIC logs
docker-compose logs -f ollama         # Follow Ollama logs
docker-compose logs -f surrealdb      # Follow SurrealDB logs
```

### Restart a single service
```bash
docker-compose restart cynic-kernel
```

### Rebuild images
```bash
docker-compose up --build
```

### Clean up everything (including volumes)
```bash
docker-compose down -v
# Removes all containers, networks, and volumes
```

---

## Environment Variables

Override defaults via `.env` file:

```bash
# .env (in cynic/ directory)
CYNIC_OLLAMA_BASE_URL=http://ollama:11434
CYNIC_SURREAL_URL=ws://surrealdb:8000
CYNIC_SURREAL_USER=root
CYNIC_SURREAL_PASS=root
```

---

## Performance Notes

### First Run
- **Ollama**: First request may be slow (model warm-up, 5-30s)
- **SurrealDB**: Should start immediately
- **CYNIC**: Starts when both dependencies healthy

### CPU-Only
- **Ollama inference**: 1-5 seconds per call (qwen2.5-coder:7b on CPU)
- **Parallel calls** (SAGE MCTS): 7 calls ≈ 8-10s total (parallel ≠ sequential)
- **Expected latency**: Integration tests take 30-60s (Ollama bottleneck)

### GPU (if available)
- Uncomment GPU lines in `docker-compose.yml`
- **Ollama inference**: 100-500ms per call (GPU-accelerated)
- **Integration tests**: Complete in <10s

---

## Troubleshooting

### "Connection refused: localhost:11434"
Ollama not ready. Wait for logs to show "Ready" or rebuild:
```bash
docker-compose restart ollama
```

### "surrealdb module not installed"
SurrealDB Python client missing. Install in host environment:
```bash
pip install surrealdb
```

### "Tests hang or timeout"
- **Ollama slow**: Normal on CPU. Wait 5-30s per call. Consider GPU.
- **Connection timeout**: Service not responding. Check `docker-compose logs`.
- **Memory**: Ollama + SurrealDB + CYNIC ≈ 2-3GB RAM minimum.

### Port already in use
If port 8000 or 11434 already occupied:
```bash
# Edit docker-compose.yml
# Change ports: "8001:8000" (host:container)
```

---

## Deployment Checklist

- [ ] `docker-compose.yml` created
- [ ] `Dockerfile` updated with CYNIC source
- [ ] Integration tests pass locally
- [ ] Health endpoints verified (all 3 services)
- [ ] Ollama model pulled (`ollama pull qwen2.5-coder:7b`)
- [ ] SurrealDB schema initialized
- [ ] Judgment persistence cycle validated
- [ ] φ-encoding empirically proven (consensus tests pass)

---

## Next Steps

After Docker deployment validates successfully:

1. **Production deployment** (Kubernetes, Render, etc.)
2. **Multi-instance consensus** (L5: network of CYNICs)
3. **Continuous falsification** (live battle cycles with real users)
4. **φ-optimization** (tune axiom weights, LOD thresholds based on empirical data)

---

## Philosophy

```
Docker = Reproducible organism deployment
Compose = Organism + dependencies = integrated system
Integration tests = Verify organism works with real external cells

φ distrusts φ — even Docker deployment leaves 38.2% doubt.
Run the tests. Validate empirically.
```

---

**Created**: 2026-02-20
**CYNIC Version**: 0.2 (Python Kernel, Empirical Phase)
**Status**: 🟢 Ready for Docker deployment + integration validation
