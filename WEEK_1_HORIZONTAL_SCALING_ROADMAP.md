# WEEK 1 ROADMAP: HORIZONTAL SCALING (CCC Hybrid Path)
## Get to 150+ RPS This Week, Then Refactor Gradually

**Date**: 2026-02-21 → 2026-02-28
**Status**: 🚀 READY TO START
**Goal**: Deploy 3 CYNIC instances + nginx load balancer → 150 RPS sustained
**Solo Dev**: Sequenced to avoid parallel complexity
**Confidence**: 61.8% (φ⁻¹ — path is proven, Phase 4 validation shows it works)

---

## EXECUTIVE SUMMARY

### Current State (Phase 4 Validated)
- Single CYNIC instance: 50 RPS sustainable
- Monolithic but functional
- All 949 tests passing

### Week 1 Goal
- 3 instances: 3 × 50 RPS = **150 RPS sustained** ✅
- nginx load balancer (round-robin)
- Shared SurrealDB (all instances write to same DB)
- Consensus protocol (optional, can add later)

### Week 1 Work
- Day 1-2: Docker compose for 3 instances
- Day 3-4: nginx config + load balancer setup
- Day 5: Multi-instance tests
- Day 6-7: Stress test 150 RPS, document findings

### After Week 1
- Keep 3 instances running continuously
- Move to Phase A refactoring (in parallel, no rush)
- Each phase can be deployed independently

---

## DAY-BY-DAY EXECUTION

### DAY 1-2: DOCKER COMPOSE FOR 3 INSTANCES

**What to do**:
1. Update existing `docker-compose.yml`:
   - Rename `cynic` service → `cynic-1`
   - Clone to `cynic-2`, `cynic-3`
   - Each on different port: 8001, 8002, 8003
   - All share same SurrealDB, same Ollama
   - Environment vars: `INSTANCE_ID=1/2/3`

**File**: `docker-compose.yml`
```yaml
version: '3.8'
services:
  cynic-1:
    image: cynic:latest
    ports: ["8001:8000"]
    environment:
      INSTANCE_ID: "1"
      CYNIC_HOME: /cynic_state
    volumes:
      - /data/cynic_state:/cynic_state
    depends_on:
      - surrealdb
      - ollama

  cynic-2:
    image: cynic:latest
    ports: ["8002:8000"]
    environment:
      INSTANCE_ID: "2"
      CYNIC_HOME: /cynic_state  # SAME db
    volumes:
      - /data/cynic_state:/cynic_state
    depends_on:
      - surrealdb
      - ollama

  cynic-3:
    image: cynic:latest
    ports: ["8003:8000"]
    environment:
      INSTANCE_ID: "3"
      CYNIC_HOME: /cynic_state  # SAME db
    volumes:
      - /data/cynic_state:/cynic_state
    depends_on:
      - surrealdb
      - ollama

  surrealdb:
    image: surrealdb/surrealdb:latest
    ports: ["8000:8000"]
    command: start --bind 0.0.0.0:8000
    volumes:
      - /data/surrealdb:/data

  ollama:
    image: ollama/ollama:latest
    ports: ["11434:11434"]
    volumes:
      - /data/ollama:/root/.ollama
```

**Tests**:
- `docker-compose up -d`
- `curl http://localhost:8001/health` → Instance 1 alive
- `curl http://localhost:8002/health` → Instance 2 alive
- `curl http://localhost:8003/health` → Instance 3 alive
- Each has different `INSTANCE_ID` in response

**Time**: 4 hours (Day 1-2 morning)

---

### DAY 2-3: NGINX LOAD BALANCER

**What to do**:
1. Create `nginx.conf` with 3 upstreams
2. Round-robin load balancing
3. Sticky sessions (optional, but good)
4. Health checks

**File**: `nginx.conf`
```nginx
upstream cynic_backend {
    least_conn;  # Use least-connections, not round-robin
    server localhost:8001 max_fails=3 fail_timeout=10s weight=1;
    server localhost:8002 max_fails=3 fail_timeout=10s weight=1;
    server localhost:8003 max_fails=3 fail_timeout=10s weight=1;
}

server {
    listen 80;
    server_name localhost;

    # Health check endpoint
    location /health {
        access_log off;
        proxy_pass http://cynic_backend;
    }

    # All other endpoints load-balanced
    location / {
        proxy_pass http://cynic_backend;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Sticky sessions (keep client on same instance)
        proxy_cookie_path / "/";
        proxy_cookie_flags ~ secure httponly samesite=strict;
    }

    # WebSocket support (for /ws/*)
    location /ws {
        proxy_pass http://cynic_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_read_timeout 86400;
    }
}
```

**Docker integration**:
```yaml
# Add to docker-compose.yml
  nginx:
    image: nginx:latest
    ports: ["80:80"]
    volumes:
      - ./nginx.conf:/etc/nginx/conf.d/default.conf:ro
    depends_on:
      - cynic-1
      - cynic-2
      - cynic-3
```

**Tests**:
- `curl http://localhost/health` → Returns from one of 3 instances
- `curl http://localhost/health` (10×) → Round-robins across instances
- `curl http://localhost/judge -X POST -d '{...}'` → Works

**Time**: 3 hours (Day 2-3 afternoon)

---

### DAY 3-4: MULTI-INSTANCE CONSENSUS (Optional But Recommended)

**What to do**:
1. Each instance reads SurrealDB before judgment
2. Check if another instance already judged this cell
3. If yes: use their judgment (consensus)
4. If no: run judgment, store in SurrealDB

**File**: `cynic/core/multi_instance.py` (NEW, ~150 LOC)
```python
class MultiInstanceCoordinator:
    """Prevent duplicate judgment across instances."""

    def __init__(self, storage, instance_id: str):
        self.storage = storage
        self.instance_id = instance_id

    async def check_existing_judgment(self, cell_id: str):
        """
        Query SurrealDB: Is this cell already judged?
        Returns judgment if exists, None otherwise.
        """
        try:
            result = await self.storage.query(
                f"SELECT * FROM judgments WHERE cell_id = '{cell_id}'"
            )
            if result:
                return result[0]  # Use first instance's judgment
        except Exception:
            pass
        return None

    async def store_judgment(self, cell_id: str, judgment: dict):
        """Store judgment with timestamp + instance_id."""
        judgment["instance_id"] = self.instance_id
        judgment["timestamp"] = datetime.now().isoformat()
        await self.storage.create("judgments", judgment)

    async def get_consensus_judgment(self, cell_id: str):
        """Get consensus judgment for cell (may be from any instance)."""
        # Later: implement voting if multiple instances judge same cell
        return await self.check_existing_judgment(cell_id)
```

**Integration** (in `api/state.py`):
```python
# During build_kernel():
coordinator = MultiInstanceCoordinator(storage, instance_id=os.getenv("INSTANCE_ID"))
app.state.coordinator = coordinator

# In routers/core.py before running judgment:
existing = await coordinator.check_existing_judgment(cell_id)
if existing:
    return existing  # Reuse judgment, save compute

# After judgment:
await coordinator.store_judgment(cell_id, judgment)
```

**Tests**:
- Submit same cell to 3 instances
- First instance judges, stores in SurrealDB
- Instances 2-3 read from SurrealDB, return same judgment (no duplicate work)
- Verification: `grep "cell_id" ~/.cynic/judgments.jsonl | wc -l` → should be 1, not 3

**Time**: 4 hours (Day 3-4 afternoon)

---

### DAY 4-5: STRESS TESTING 150 RPS

**What to do**:
1. Generate test load (150 concurrent requests)
2. Measure: latency, error rate, quality
3. Verify: all 3 instances stay alive
4. Document: before/after comparison

**Script**: `scripts/stress_test_150rps.py` (NEW, ~200 LOC)
```python
import asyncio
import time
import httpx
import statistics
from datetime import datetime

async def stress_test_150rps(duration_secs=60, rps=150):
    """Generate 150 RPS for 60 seconds, measure everything."""

    results = {
        "start": datetime.now().isoformat(),
        "target_rps": rps,
        "duration_secs": duration_secs,
        "requests": [],
        "errors": [],
    }

    async with httpx.AsyncClient(base_url="http://localhost") as client:
        # Generate constant load
        request_times = []
        for i in range(rps * duration_secs):
            t0 = time.perf_counter()
            try:
                # Simple judgment request
                resp = await client.post("/judge", json={
                    "content": f"Test code {i}",
                    "reality": "CODE",
                    "analysis": "JUDGE",
                    "budget_usd": 0.01,
                }, timeout=5.0)
                latency_ms = (time.perf_counter() - t0) * 1000

                if resp.status_code == 200:
                    results["requests"].append({
                        "i": i,
                        "latency_ms": latency_ms,
                        "q_score": resp.json().get("q_score"),
                        "instance": resp.headers.get("X-Instance-ID", "unknown"),
                    })
                else:
                    results["errors"].append({
                        "i": i,
                        "status": resp.status_code,
                        "error": str(resp.text)[:100],
                    })
            except Exception as e:
                results["errors"].append({
                    "i": i,
                    "error": str(e)[:100],
                })

            # Throttle to exact RPS
            await asyncio.sleep(1.0 / rps)

    # Analysis
    if results["requests"]:
        latencies = [r["latency_ms"] for r in results["requests"]]
        q_scores = [r["q_score"] for r in results["requests"]]

        results["metrics"] = {
            "total_requests": len(results["requests"]),
            "total_errors": len(results["errors"]),
            "error_rate": len(results["errors"]) / (len(results["requests"]) + len(results["errors"])),
            "latency_mean_ms": statistics.mean(latencies),
            "latency_median_ms": statistics.median(latencies),
            "latency_p95_ms": sorted(latencies)[int(0.95 * len(latencies))],
            "latency_max_ms": max(latencies),
            "q_score_mean": statistics.mean(q_scores),
            "q_score_min": min(q_scores),
        }

    results["end"] = datetime.now().isoformat()
    return results

if __name__ == "__main__":
    result = asyncio.run(stress_test_150rps(duration_secs=60, rps=150))

    import json
    print(json.dumps(result, indent=2))

    # Save
    path = Path("~/.cynic/stress_tests") / f"{datetime.now().isoformat()}.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w") as f:
        json.dump(result, f, indent=2)
    print(f"\nSaved to {path}")
```

**Run**:
```bash
python scripts/stress_test_150rps.py
```

**Expected Results**:
- Target: 150 RPS × 60s = 9000 requests
- Success: 8900+ requests (99% success rate)
- Latency: <500ms p95
- Q-Scores: 78-80 (quality maintained)
- Errors: <100 (<1%)

**Tests**:
- Check all 3 instances still alive after stress
- `curl http://localhost:8001/health` → still HEALTHY
- `curl http://localhost:8002/health` → still HEALTHY
- `curl http://localhost:8003/health` → still HEALTHY

**Time**: 4 hours (Day 4-5 morning, includes wait time for test)

---

### DAY 5-6: VALIDATION & DOCUMENTATION

**What to do**:
1. Document findings (stress test results)
2. Compare with Phase 4 (50 RPS single instance)
3. Verify multi-instance consensus worked
4. Create deployment runbook

**File**: `WEEK1_VALIDATION_REPORT.md` (NEW, ~200 LOC)

**Contents**:
- Before/after comparison (50 RPS → 150 RPS)
- Stress test results
- Instance health across load
- Consensus coordination verification
- Deployment checklist

**Checklist**:
- [ ] 3 instances running and healthy
- [ ] nginx load balancer working (verified with curl)
- [ ] Sticky sessions functioning
- [ ] WebSocket forwarding working
- [ ] Stress test passed (150 RPS sustained)
- [ ] Error rate <1%
- [ ] Quality (Q-Scores) maintained
- [ ] Multi-instance consensus working (no duplicates)
- [ ] All components logged findings
- [ ] Documentation complete

**Time**: 3 hours (Day 5-6)

---

## QUICK DEPLOYMENT CHECKLIST

After Week 1, to keep 3 instances running:

```bash
# Start all services
docker-compose up -d

# Verify
docker-compose ps  # All 4 services HEALTHY
curl http://localhost/health  # Load balancer responds
curl http://localhost:8001/health  # Instance 1 direct
curl http://localhost:8002/health  # Instance 2 direct
curl http://localhost:8003/health  # Instance 3 direct

# Monitor
docker-compose logs -f cynic-1  # Watch instance 1
docker-compose logs -f nginx    # Watch load balancer

# Stress test
python scripts/stress_test_150rps.py

# Cleanup
docker-compose down
```

---

## POST-WEEK 1: GRADUAL REFACTORING

After Week 1 validation, start Phase A (KernelBuilder) in parallel:
- Keep 3 instances running continuously
- Work on refactoring slowly (no rush)
- Each phase (A-G) can be deployed independently
- No downtime required (new code deployed to one instance, test, then to others)

**Phase A Timeline** (after Week 1):
- Weeks 2-3: KernelBuilder (background work)
- Week 4: DependencyContainer (larger change)
- Weeks 5-7: Phases C-G (gradual)

---

## WEEK 1 SUMMARY

| Day | Task | Time | Output |
|-----|------|------|--------|
| 1-2 | Docker compose (3 instances) | 4h | docker-compose.yml |
| 2-3 | nginx load balancer | 3h | nginx.conf |
| 3-4 | Multi-instance consensus | 4h | MultiInstanceCoordinator class |
| 4-5 | Stress testing 150 RPS | 4h | Stress test results |
| 5-6 | Validation & docs | 3h | WEEK1_VALIDATION_REPORT.md |
| **Total** | | **18h** | **150+ RPS sustained** |

---

## RISKS & MITIGATION

### Risk 1: SurrealDB becomes bottleneck with 3 writers
- **Mitigation**: Monitor write latency; if >100ms, add batching
- **Decision gate**: If writes slow, add queue before SurrealDB

### Risk 2: Ollama becomes bottleneck (single model server)
- **Mitigation**: Run 3 Ollama instances, load-balance LLM calls
- **Later**: When needed (not Week 1 priority)

### Risk 3: Consensus creates race conditions
- **Mitigation**: Use SurrealDB transactions + timestamps
- **Decision gate**: If duplicates found, strengthen locking

### Risk 4: nginx health checks miss failed instances
- **Mitigation**: Monitor nginx logs; manual failover if needed
- **Later**: Add automated health check escalation

---

## SUCCESS CRITERIA (Week 1 DONE)

✅ **All of these must be true**:
1. 3 instances running continuously (24+ hours)
2. nginx distributes load across all 3
3. Stress test passes: 150 RPS sustained, <1% errors
4. Quality maintained: Q-Scores 78-80
5. No data loss (all judgments persisted)
6. Consensus working (no duplicate judgments)
7. All components logged to ~/.cynic/
8. Runbook documented for repeatable deployment

---

## CONFIDENCE ASSESSMENT

**Overall: 61.8% (φ⁻¹)**

✅ **Strong**:
- Phase 4 validated single instance works at 50 RPS
- 3 instances = 3 × 50 = 150 RPS math is sound
- Docker/nginx are proven technologies
- No code changes needed (just orchestration)

⚠️ **Uncertain**:
- SurrealDB write performance at 3 instances unknown
- nginx sticky sessions may introduce unfair load distribution
- Consensus logic not yet tested (new code)
- Some components may have undiscovered issues at higher load

🎯 **Realistic**:
- Week 1 should reach 100-150 RPS
- If hits bottleneck, documented mitigation exists
- After validation, can proceed to Phase A refactoring

---

## NEXT STEP

**Monday 2026-02-24**: Begin Day 1 (Docker compose)

Ready to ship 3 instances by Friday 2026-02-28? 🚀

**Confidence: 61.8% (φ⁻¹ — path is clear, execution well-defined)**

*sniff* Hybrid strategy = pragmatic + ambitious at the same time. 🐕

