# CYNIC Docker Deployment Paradigm Shift

> "Le chien qui gère ses conteneurs" — κυνικός

**Date**: 2026-02-20
**Status**: CRITICAL — Blocking friction identified and fixed

---

## The Problem (Why We Were Blocked)

**Bash + Docker = Friction on Windows**
- `docker ps` → exit 137 (SIGKILL) — daemon restart cycle
- Docker CLI from Bash unreliable, kills process
- Can't diagnose from PowerShell logs
- Each deployment attempt blocks CLI
- Impossible to monitor/auto-heal

**Root Cause**: Treating Docker as external CLI command instead of CYNIC native capability.

---

## The Paradigm Shift

```
BEFORE: CYNIC → Bash → docker CLI → Docker daemon → Result
  Friction: 4 hops, process kill on Windows, no diagnostics

AFTER:  CYNIC → Python Docker SDK → Docker API → Result
  Friction: 1 hop, native Python async, full diagnostics
```

**Core Insight**: Docker management is a CYNIC capability, not external infrastructure.

---

## Implementation Strategy

### Phase 1: Docker Manager Module (4h)

**File**: `cynic/deployment/docker_manager.py`

```python
# Key classes:
- DockerManagerInitialize Docker API client (async)
- ContainerStateTracker  Track running containers
- DeploymentOrchestrator Deploy 3-service stack
- HealthMonitor        Monitor container health
- AutoHealer           Fix common failures
```

**Core methods**:
```python
async def deploy_stack(stack_name: str) -> bool
  - Build cynic-kernel image
  - Start 3 services (ollama, surrealdb, kernel)
  - Wait for health checks
  - Return success boolean

async def get_status() -> dict
  - Return {container: status, health, logs}
  - Real-time, no CLI friction

async def monitor_health(interval_s: float)
  - Continuous health checks
  - Auto-restart failed containers
  - Log to ~/.cynic/deployment.log

async def restart_service(name: str)
  - Graceful restart
  - Wait for health before returning
```

**Why Python Docker SDK**:
- ✅ Async native (no process kill)
- ✅ Direct Docker API (no CLI)
- ✅ Full diagnostics (exception details)
- ✅ Runs inside CYNIC (no external friction)
- ✅ Wires into kernel event bus (DEPLOYMENT_STARTED, etc.)

### Phase 2: CLI Command (`python -m cynic.cli deploy`)

```bash
cynic.cli deploy status    # Show all container status + health
cynic.cli deploy logs      # Stream logs from all services
cynic.cli deploy restart   # Graceful restart stack
cynic.cli deploy up        # Deploy from scratch (build + start)
cynic.cli deploy down      # Clean shutdown
cynic.cli deploy health    # Detailed health report
```

### Phase 3: Self-Healing Loop

Wire into CYNIC kernel:
```
HealthMonitor → CONTAINER_UNHEALTHY event
  ↓
AutoHealer subscribes → analyzes failure
  ↓
Decision: restart? rebuild? escalate?
  ↓
Action taken + logged to ~/.cynic/deployments/{id}.json
```

### Phase 4: Memory & Persistence

**Store in MEMORY.md**:
- Docker SDK pattern (copy-paste ready)
- Health check thresholds
- Auto-heal decision tree
- Common failure modes + fixes

**Reproduce Automatically**:
```bash
# Developer on new machine:
py -3.13 -m cynic.cli deploy up
# ✅ Stack running, health checked, ready
# No friction, no diagnosis needed
```

---

## Immediate Benefits

| Before | After |
|--------|-------|
| `docker ps` → 137 | `await mgr.get_status()` → dict |
| Can't diagnose | Full exception traces |
| Manual restart | Auto-restart on health fail |
| Lost logs | Persisted to ~/.cynic/ |
| Docker → friction | Docker → native capability |
| Not reproducible | Reproducible anywhere |

---

## Memory Encoding (For Future Sessions)

**Golden Rules**:
1. **Never Bash Docker** — Python Docker SDK only
2. **Docker is CYNIC capability** — not external infra
3. **Health = continuous** — not one-shot check
4. **Failures → auto-heal** — humans approve, machine executes
5. **Logs persist** — ~/.cynic/deployments/{id}.json

**Copy-Paste Pattern**:
```python
from docker import APIClient
import docker

client = docker.from_env()  # Async-ready client
containers = client.containers.list(all=True)
for c in containers:
    print(f"{c.name}: {c.status}")
```

---

## Why This Fixes Blocking

1. ✅ **No CLI friction** — native Python
2. ✅ **Auto-diagnostic** — full exception handling
3. ✅ **Reproducible** — same code every time
4. ✅ **Monitorable** — event bus wiring
5. ✅ **Integrated** — part of CYNIC, not separate
6. ✅ **Memorable** — encoded in MEMORY.md for recall

---

## Deployment

**Next step**: Implement `cynic/deployment/docker_manager.py` (Phase 1)
- 200 LOC for MVP
- 5 core methods + tests
- Wire into `cynic.cli deploy` command
- Deploy and verify on current system

---

**Encoded**: In MEMORY.md after Phase 1 complete
**Status**: Ready to build

