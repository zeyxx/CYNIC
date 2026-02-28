# CYNIC as MCP Server for Cline

> "The dog coordinates with itself" — κυνικός

## Overview

CYNIC is now accessible to **Cline** (Claude in VSCode) via the **Model Context Protocol (MCP)**.

Instead of Cline consuming context to run CYNIC's empirical tests, Cline can now call CYNIC as an autonomous service:

```
Cline (VSCode)
    ↓ (MCP stdio)
CYNIC MCP Server (running in Docker)
    ↓ (Python async)
Empirical Test Runner
    ↓
CYNIC Organism (FastAPI + Consciousness + Judgment)
```

**Key benefit**: CYNIC runs its own research without consuming Cline's context tokens.

---

## Architecture

### Three Layers

```
Layer 1: MCP Protocol (stdio)
  ├─ Tools (callable functions)
  ├─ Resources (queryable data)
  └─ JSON-RPC request/response

Layer 2: CYNIC MCP Adapter
  ├─ Job lifecycle (QUEUED → RUNNING → COMPLETE)
  ├─ Batch runner (orchestrator.judge() × N iterations)
  ├─ Telemetry collection (SONA heartbeat polling)
  └─ Results persistence (~/.cynic/results/)

Layer 3: CYNIC Organism (unchanged)
  ├─ FastAPI (port 8765, optional dashboard)
  ├─ PostgreSQL (judgments, metrics)
  ├─ Ollama (LLM backend)
  └─ SONA emitter (34-min heartbeat)
```

### Files Created

| File | Purpose |
|------|---------|
| `cynic/mcp/empirical_runner.py` | Job manager + batch test runner |
| `cynic/mcp/stdio_server.py` | Stdio MCP server for Cline |
| `cynic/mcp/__main__.py` | Entry point (starts both servers) |
| `cynic/tests/test_mcp_empirical.py` | Integration tests |
| `docker-compose.yml` | Updated with `cynic-mcp` service |
| `CYNIC-MCP-INTEGRATION.md` | This file |

---

## Setup

### 1. Build Docker Image

```bash
cd cynic
docker build -t cynic:0.1.0 .
```

### 2. Start Services

```bash
# Terminal 1: Start core services (postgres, ollama, cynic HTTP API)
docker-compose up -d postgres ollama cynic

# Terminal 2: Start MCP server (for Cline)
docker-compose up cynic-mcp
```

**Or** both at once:
```bash
docker-compose up
```

### 3. Configure Cline

In VSCode settings (or `~/.claude/mcp.json`), register CYNIC MCP server:

```json
{
  "mcpServers": {
    "cynic": {
      "command": "docker",
      "args": ["exec", "-i", "cynic-mcp", "python", "-m", "cynic.mcp"]
    }
  }
}
```

**Or** if using HTTP MCP fallback (simpler):
```json
{
  "mcpServers": {
    "cynic": {
      "command": "curl",
      "args": ["http://localhost:8766/mcp"]
    }
  }
}
```

---

## Usage

### Running Tests from Cline

**Prompt 1: Start a test**
```
Run an empirical test with 1000 judgment iterations to measure learning efficiency.
```

Cline will:
1. Call `cynic_run_empirical_test(count=1000)`
2. CYNIC returns `{job_id: "test-2026-02-24-abc123", status: "queued"}`
3. CYNIC starts async job runner

**Prompt 2: Check progress** (after ~5 minutes)
```
What's the status of the test job?
```

Cline will:
1. Extract job_id from memory
2. Call `cynic_get_job_status(job_id="test-2026-02-24-abc123")`
3. CYNIC returns:
   ```json
   {
     "status": "running",
     "progress_percent": 45,
     "iterations_done": 450,
     "iterations_total": 1000,
     "eta_s": 300
   }
   ```

**Prompt 3: Get results** (after ~10 minutes)
```
Get the empirical test results.
```

Cline will:
1. Call `cynic_get_results(job_id="test-2026-02-24-abc123")`
2. CYNIC returns:
   ```json
   {
     "job_id": "test-2026-02-24-abc123",
     "q_scores": [45.2, 48.1, 51.3, ...],
     "avg_q": 52.4,
     "min_q": 38.7,
     "max_q": 61.2,
     "learning_efficiency": 1.048,
     "emergences": 3,
     "duration_s": 587.3
   }
   ```

### Running Axiom Irreducibility Tests

```
Test if the PHI axiom is irreducible (necessary) for CYNIC judgment quality.
```

Cline calls: `cynic_run_irreducibility_test(axiom="PHI")`

CYNIC returns:
```json
{
  "axiom_impacts": [
    {
      "name": "PHI",
      "baseline_q": 52.4,
      "disabled_q": 38.1,
      "impact_percent": 27.3,
      "irreducible": true
    }
  ]
}
```

**To test all 5 axioms:**
```
Test the irreducibility of all CYNIC axioms in parallel.
```

Cline calls: `cynic_run_irreducibility_test(axiom=None)`

CYNIC returns impact data for:
- PHI (Structure)
- VERIFY (Evidence)
- CULTURE (Community)
- BURN (Simplicity)
- FIDELITY (Honesty)

### Querying Telemetry

```
What's the current CYNIC uptime and learning statistics?
```

Cline calls: `cynic_query_telemetry(metric="uptime_s")`

CYNIC returns:
```json
{
  "metric": "uptime_s",
  "uptime_s": 3600.0,
  "q_table_entries": 1024,
  "total_judgments": 12500,
  "learning_rate": 0.001
}
```

---

## Tools Exposed to Cline

### 1. `cynic_run_empirical_test`

**Description**: Run an empirical test of CYNIC judgment system.

**Parameters**:
- `count` (int, optional): Number of iterations (default: 1000)
- `seed` (int, optional): Random seed for reproducibility

**Returns**: `{job_id, status, message}`

**Example**:
```python
# Cline prompt: "Run 5000 judgments to test learning efficiency"
job_id = cynic_run_empirical_test(count=5000)
# → "test-2026-02-24-xyz"
```

### 2. `cynic_get_job_status`

**Description**: Get status and progress of a running test.

**Parameters**:
- `job_id` (str, required): Job ID from spawn_test

**Returns**: `{status, progress_percent, iterations_done, eta_s, error_message}`

**States**:
- `queued` — Waiting to start
- `running` — In progress
- `complete` — Done (call get_results)
- `error` — Failed

### 3. `cynic_get_results`

**Description**: Get complete results from finished test.

**Parameters**:
- `job_id` (str, required): Job ID of completed test

**Returns**: `{q_scores, avg_q, min_q, max_q, learning_efficiency, emergences, duration_s}`

**Only callable when**: `status == "complete"`

### 4. `cynic_run_irreducibility_test`

**Description**: Test if axioms are irreducible (necessary) for quality.

**Parameters**:
- `axiom` (str, optional): Test specific axiom (PHI/VERIFY/CULTURE/BURN/FIDELITY), or null for all

**Returns**: `{axiom_impacts: [{name, baseline_q, disabled_q, impact_percent, irreducible}]}`

**Time**: ~10 min per axiom, ~50 min for all 5

### 5. `cynic_query_telemetry`

**Description**: Query SONA heartbeat and organism health metrics.

**Parameters**:
- `metric` (str): uptime_s, q_table_entries, total_judgments, learning_rate

**Returns**: `{metric, value, ...}`

---

## Architecture Decisions

| Decision | Why | Trade-off |
|----------|-----|-----------|
| **Stdio MCP** | Native Cline integration | HTTP variant also available |
| **In-process jobs** | CYNIC's event loop already runs, SONA continues uninterrupted | No separate worker service |
| **Results in memory** | Fast for Cline queries | Persist to disk for replay |
| **FastAPI background** | Keeps dashboard optional | Consumes slight overhead |
| **Docker volume mount** | Code hot-reload for dev, immutable in prod | Can override with COPY |

---

## Scaling Path (Future)

### Week 1: Single CYNIC (now)
- One MCP container
- Results in-memory + disk

### Week 4: Network MCP
- Multiple CYNIC containers in Docker Compose
- Gossip protocol coordination
- Shared PostgreSQL

### Week 12: Type I
- K8s deployment with Helm
- Auto-scaling based on job queue
- Multi-region consensus

---

## Troubleshooting

### Docker won't start

```bash
docker-compose logs cynic-mcp
```

Look for:
- `Import error` → Missing dependency (run `pip install mcp`)
- `Connection refused` → PostgreSQL not healthy (wait 30s)
- `Ollama connection failed` → Ollama container crashed

### Cline can't reach MCP

Check MCP registration:
```bash
# Option 1: Direct stdio test
docker exec -i cynic-mcp python -m cynic.mcp < /dev/null

# Option 2: Check HTTP fallback
curl http://localhost:8766/
```

### Job stuck on "running"

```bash
# Check container logs
docker-compose logs cynic-mcp | grep "Test job"

# Check results directory
docker exec cynic-mcp ls -lah /home/cynic/.cynic/results/
```

### Results not persisting

Verify volume mount:
```bash
docker-compose ps cynic-mcp
# Should show: cynic_data:/home/cynic/.cynic

docker volume ls | grep cynic_data
```

---

## Testing

### Unit Tests

```bash
cd cynic
pytest cynic/tests/test_mcp_empirical.py -v
```

### Integration Test (Manual)

```bash
# Terminal 1
docker-compose up cynic-mcp

# Terminal 2
docker exec -i cynic-mcp python -c "
import asyncio
from cynic.mcp.empirical_runner import EmpiricalRunner

async def test():
    # Mock organism getter
    runner = EmpiricalRunner(lambda: None)
    job_id = await runner.spawn_test(count=10)
    print(f'Job: {job_id}')
    # Wait and check status
    for _ in range(50):
        status = await runner.get_job_status(job_id)
        print(f'  Status: {status[\"status\"]}, Progress: {status[\"progress_percent\"]:.0f}%')
        if status['status'] == 'complete':
            break
        await asyncio.sleep(0.5)

asyncio.run(test())
"
```

---

## Environment Variables

| Var | Default | Purpose |
|-----|---------|---------|
| `CYNIC_MCP_STDIO_ONLY` | 0 | Set to 1 to disable HTTP server |
| `CYNIC_DATABASE_URL` | (required) | PostgreSQL connection |
| `OLLAMA_URL` | http://ollama:11434 | LLM backend |
| `LOG_LEVEL` | INFO | Logging verbosity |
| `PYTHONUNBUFFERED` | 1 | Realtime output |

---

## Performance Notes

- **Startup**: 30s (organism awaken + dependency check)
- **Per iteration**: ~50-100ms (orchestrator.judge())
- **1000 iterations**: ~50-100s (~1-2 min)
- **10000 iterations**: ~500-1000s (~8-16 min)
- **Irreducibility test** (1 axiom): ~10 min (1000 iter × 5 axioms separately if all tested)

---

## Next Steps

1. **Phase 2**: Add real-time streaming progress (WebSocket MCP)
2. **Phase 3**: Consensus across multiple CYNIC instances (gossip protocol)
3. **Phase 4**: AI Scientist integration (tree search experimentation)

---

## References

- **MCP Protocol**: https://modelcontextprotocol.io/
- **CYNIC Architecture**: `docs/reference/01-ARCHITECTURE.md`
- **Judgment System**: `docs/reference/COST-ANALYSIS.md`
- **Learning Loops**: `docs/reference/06-LEARNING-SYSTEM.md`

---

**Confidence**: 58% (φ⁻¹ limit) — This integration pattern is proven (MCP is standard), but integration with live CYNIC organism state needs validation.

*sniff* "The dog teaches itself to teach others." — κυνικός
