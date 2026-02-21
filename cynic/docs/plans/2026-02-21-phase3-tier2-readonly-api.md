# Phase 3 Tier 2-3: Read-Only API Endpoints Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Wire OrganismState into read-only HTTP endpoints so CYNIC can observe its own reality and external clients can query the organism's internal state.

**Architecture:**
Create FastAPI endpoints that query the unified OrganismState (installed in Day 2). Each endpoint is read-only, returns immutable snapshots, and demonstrates that state consolidation works in practice. Pattern: `GET /api/organism/{query}` → `organism.state.{method}()` → JSON response.

**Tech Stack:**
- FastAPI (existing HTTP framework)
- Pydantic (response models)
- OrganismState (from Day 2)
- pytest + httpx (API testing)

---

## Summary of Tasks

### Phase 3 Tier 2-3: Read-Only Endpoints
1. Create StateSnapshotResponse model (Pydantic)
2. Create `GET /api/organism/state/snapshot` endpoint
3. Create `GET /api/organism/consciousness` endpoint
4. Create `GET /api/organism/dogs` endpoint
5. Create `GET /api/organism/actions` endpoint
6. Wire endpoints to organism.state in server.py
7. Add API integration tests
8. Verify endpoints return correct reality

---

## Task 1: Create Pydantic Response Models

**Files:**
- Create: `cynic/api/models/organism_state.py` (~150 LOC)
- Modify: `cynic/api/models/__init__.py` (add exports)
- Test: `tests/api/test_organism_models.py` (~80 LOC)

**Step 1: Write failing test**

```python
# tests/api/test_organism_models.py
from cynic.api.models.organism_state import (
    StateSnapshotResponse,
    ConsciousnessResponse,
    DogsResponse,
    ActionsResponse,
)

def test_state_snapshot_response_model():
    """StateSnapshotResponse can be created with required fields."""
    response = StateSnapshotResponse(
        timestamp=1708123456.789,
        consciousness_level="MACRO",
        judgment_count=5,
        dog_count=11,
        qtable_entries=42,
        residuals_count=3,
        pending_actions_count=7,
    )
    assert response.consciousness_level == "MACRO"
    assert response.judgment_count == 5

def test_consciousness_response_model():
    """ConsciousnessResponse model."""
    response = ConsciousnessResponse(level="MICRO")
    assert response.level == "MICRO"

def test_dogs_response_model():
    """DogsResponse model."""
    dogs = {
        "GUARDIAN": {"q_score": 0.75, "verdict": "WAG"},
        "ANALYST": {"q_score": 0.62, "verdict": "GROWL"},
    }
    response = DogsResponse(dogs=dogs, count=2)
    assert response.count == 2
    assert response.dogs["GUARDIAN"]["q_score"] == 0.75

def test_actions_response_model():
    """ActionsResponse model."""
    actions = [
        {"id": "a1", "type": "INVESTIGATE", "priority": 1},
        {"id": "a2", "type": "MONITOR", "priority": 3},
    ]
    response = ActionsResponse(actions=actions, count=2)
    assert response.count == 2
```

**Step 2: Run test to verify it fails**

```bash
cd C:\Users\zeyxm\Desktop\asdfasdfa\CYNIC\cynic
pytest tests/api/test_organism_models.py::test_state_snapshot_response_model -v
```

Expected: `FAILED - ModuleNotFoundError: No module named 'cynic.api.models.organism_state'`

**Step 3: Write response models**

```python
# cynic/api/models/organism_state.py
"""
Pydantic response models for OrganismState queries.

Used by read-only endpoints to return CYNIC's observable state.
All models are immutable (frozen=True) to prevent external mutation.
"""

from pydantic import BaseModel, Field
from typing import Dict, List, Any, Optional
from datetime import datetime


class StateSnapshotResponse(BaseModel):
    """Immutable snapshot of OrganismState at a moment in time."""

    timestamp: float = Field(..., description="Unix timestamp of snapshot")
    consciousness_level: str = Field(..., description="Current consciousness level")
    judgment_count: int = Field(0, description="Total recent judgments")
    dog_count: int = Field(0, description="Total dogs in registry")
    qtable_entries: int = Field(0, description="Total Q-table state entries")
    residuals_count: int = Field(0, description="Active residuals being tracked")
    pending_actions_count: int = Field(0, description="Pending actions in queue")

    class Config:
        frozen = True


class ConsciousnessResponse(BaseModel):
    """Current consciousness level."""

    level: str = Field(..., description="REFLEX|MICRO|MACRO|META")

    class Config:
        frozen = True


class DogStatus(BaseModel):
    """Individual dog status."""
    q_score: float = Field(..., description="Last Q-score [0, 1]")
    verdict: str = Field(..., description="Last verdict (BARK/GROWL/WAG/HOWL)")
    confidence: Optional[float] = Field(None, description="Confidence level")
    activity: Optional[str] = Field(None, description="idle|judging|learning")


class DogsResponse(BaseModel):
    """Registry of all dogs."""

    dogs: Dict[str, DogStatus] = Field(default_factory=dict)
    count: int = Field(0, description="Total dog count")

    class Config:
        frozen = True


class ProposedAction(BaseModel):
    """Proposed action in queue."""
    action_id: str = Field(..., description="Unique action ID")
    action_type: str = Field(..., description="INVESTIGATE|REFACTOR|ALERT|MONITOR")
    priority: int = Field(..., description="1=critical, 4=FYI")
    description: Optional[str] = Field(None, description="Human-readable summary")


class ActionsResponse(BaseModel):
    """Pending actions queue."""

    actions: List[ProposedAction] = Field(default_factory=list)
    count: int = Field(0, description="Total pending action count")

    class Config:
        frozen = True
```

**Step 4: Run test to verify it passes**

```bash
cd C:\Users\zeyxm\Desktop\asdfasdfa\CYNIC\cynic
pytest tests/api/test_organism_models.py -v
```

Expected: `PASSED (4 tests)`

**Step 5: Commit**

```bash
cd C:\Users\zeyxm\Desktop\asdfasdfa\CYNIC\cynic
git add cynic/api/models/organism_state.py tests/api/test_organism_models.py
git commit -m "feat(api): Task 1 - Create Pydantic response models for organism state"
```

---

## Task 2: Create GET /api/organism/state/snapshot Endpoint

**Files:**
- Create: `cynic/api/routers/organism.py` (~100 LOC)
- Modify: `cynic/api/server.py` (add router)
- Test: `tests/api/test_organism_endpoints.py` (~60 LOC)

**Step 1: Write failing test**

```python
# tests/api/test_organism_endpoints.py
import pytest
from fastapi.testclient import TestClient
from cynic.api.server import app

@pytest.fixture
def client():
    """FastAPI test client."""
    return TestClient(app)

def test_get_organism_state_snapshot(client):
    """GET /api/organism/state/snapshot returns StateSnapshotResponse."""
    response = client.get("/api/organism/state/snapshot")
    assert response.status_code == 200

    data = response.json()
    assert "consciousness_level" in data
    assert "judgment_count" in data
    assert "timestamp" in data
    assert data["consciousness_level"] in ["REFLEX", "MICRO", "MACRO", "META"]
```

**Step 2: Run test to verify it fails**

```bash
cd C:\Users\zeyxm\Desktop\asdfasdfa\CYNIC\cynic
pytest tests/api/test_organism_endpoints.py::test_get_organism_state_snapshot -v
```

Expected: `FAILED - 404 Not Found`

**Step 3: Create endpoint**

```python
# cynic/api/routers/organism.py
"""
Read-only endpoints for OrganismState queries.

These endpoints provide observability into CYNIC's internal state.
All responses are immutable (Pydantic frozen models).

Pattern:
  GET /api/organism/{resource} → organism.state.{method}() → JSON response
"""

from fastapi import APIRouter, Depends, HTTPException
from cynic.api.state import get_app_container, AppContainer
from cynic.api.models.organism_state import (
    StateSnapshotResponse,
    ConsciousnessResponse,
    DogsResponse,
    ActionsResponse,
)
import logging

logger = logging.getLogger("cynic.api.routers.organism")

router = APIRouter(prefix="/api/organism", tags=["organism"])


@router.get("/state/snapshot", response_model=StateSnapshotResponse)
async def get_organism_state_snapshot(
    container: AppContainer = Depends(get_app_container),
) -> StateSnapshotResponse:
    """
    Get immutable snapshot of organism's current state.

    Returns all observable state at this moment in time:
    - Consciousness level
    - Judgment count
    - Dog registry size
    - Q-table entries
    - Residuals being tracked
    - Pending actions queue

    Response is frozen (immutable) to prevent external mutations.
    """
    try:
        organism = container.organism
        snapshot = organism.state.snapshot()

        return StateSnapshotResponse(
            timestamp=snapshot.timestamp,
            consciousness_level=snapshot.consciousness_level,
            judgment_count=snapshot.judgment_count,
            dog_count=snapshot.dog_count,
            qtable_entries=snapshot.qtable_entries,
            residuals_count=snapshot.residuals_count,
            pending_actions_count=snapshot.pending_actions_count,
        )
    except Exception as e:
        logger.error(f"Failed to get organism state snapshot: {e}")
        raise HTTPException(status_code=500, detail="Internal state error")
```

**Step 4: Register router in server.py**

```python
# Modify cynic/api/server.py, add to lifespan startup:
from cynic.api.routers.organism import router as organism_router

# In the app creation section:
app.include_router(organism_router)
```

**Step 5: Run test to verify it passes**

```bash
cd C:\Users\zeyxm\Desktop\asdfasdfa\CYNIC\cynic
pytest tests/api/test_organism_endpoints.py::test_get_organism_state_snapshot -v
```

Expected: `PASSED`

**Step 6: Commit**

```bash
cd C:\Users\zeyxm\Desktop\asdfasdfa\CYNIC\cynic
git add cynic/api/routers/organism.py tests/api/test_organism_endpoints.py cynic/api/server.py
git commit -m "feat(api): Task 2 - Create GET /api/organism/state/snapshot endpoint"
```

---

## Tasks 3-5: Additional Endpoints (Consciousness, Dogs, Actions)

Follow same TDD pattern as Task 2. Each endpoint:

1. Add response model (if needed)
2. Write failing test
3. Create endpoint
4. Register in server.py
5. Test passes
6. Commit

**Task 3: GET /api/organism/consciousness**
- Endpoint: `router.get("/consciousness")`
- Returns: `ConsciousnessResponse`
- Implementation: `organism.state.get_consciousness_level()`

**Task 4: GET /api/organism/dogs**
- Endpoint: `router.get("/dogs")`
- Returns: `DogsResponse` with all dogs
- Implementation: `organism.state.get_dogs()`

**Task 5: GET /api/organism/actions**
- Endpoint: `router.get("/actions")`
- Returns: `ActionsResponse` with pending actions queue
- Implementation: `organism.state.get_pending_actions()`

---

## Task 6: Integration Tests (API Reality Check)

**Files:**
- Modify: `tests/api/test_organism_endpoints.py` (add integration tests)

**Write tests that verify endpoints return CYNIC's actual state:**

```python
@pytest.mark.asyncio
async def test_consciousness_endpoint_returns_valid_level(client):
    """Consciousness endpoint returns valid level."""
    response = client.get("/api/organism/consciousness")
    assert response.status_code == 200
    assert response.json()["level"] in ["REFLEX", "MICRO", "MACRO", "META"]

@pytest.mark.asyncio
async def test_dogs_endpoint_returns_all_dogs(client):
    """Dogs endpoint returns complete registry."""
    response = client.get("/api/organism/dogs")
    assert response.status_code == 200
    data = response.json()
    assert "dogs" in data
    assert "count" in data
    assert data["count"] >= 0

@pytest.mark.asyncio
async def test_actions_endpoint_returns_queue(client):
    """Actions endpoint returns pending queue."""
    response = client.get("/api/organism/actions")
    assert response.status_code == 200
    data = response.json()
    assert "actions" in data
    assert "count" in data
    assert isinstance(data["actions"], list)
```

---

## Success Criteria

✅ **All 5 endpoints created and working**
✅ **All endpoints return immutable Pydantic models (frozen=True)**
✅ **All tests passing (20+ API tests)**
✅ **Endpoints query organism.state (not direct subsystem access)**
✅ **API returns CYNIC's actual reality, not stale/mocked data**

---

## Expected Output

After all tasks:
```bash
$ curl http://localhost:8000/api/organism/state/snapshot
{
  "timestamp": 1708123456.789,
  "consciousness_level": "MACRO",
  "judgment_count": 5,
  "dog_count": 11,
  "qtable_entries": 42,
  "residuals_count": 3,
  "pending_actions_count": 2
}

$ curl http://localhost:8000/api/organism/consciousness
{
  "level": "MACRO"
}

$ curl http://localhost:8000/api/organism/actions
{
  "actions": [
    {"action_id": "a1", "action_type": "INVESTIGATE", "priority": 1},
    {"action_id": "a2", "action_type": "MONITOR", "priority": 3}
  ],
  "count": 2
}
```

---

**Plan saved to:** `docs/plans/2026-02-21-phase3-tier2-readonly-api.md`

**Execution options:**

**1. Subagent-Driven** (this session) — Fresh subagent per task, fast iteration
**2. Parallel Session** — Autonomous batch execution with checkpoints

**Which approach?**
