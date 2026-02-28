# Model B: Federated Learning Hub — Detailed Implementation Spec

**Target:** Build federation hub for Model B distribution (Phase 2)
**Timeline:** 2-3 weeks
**Effort:** ~1,500 LOC
**Status:** Design specification, ready for implementation

---

## Part 1: Architecture Overview

### High-Level Design

```
┌─────────────────────────────────────────────────────────────────┐
│                        FEDERATION HUB                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ API Layer (FastAPI)                                     │   │
│  ├─────────────────────────────────────────────────────────┤   │
│  │ POST /api/federation/sync/push   — instance uploads Q  │   │
│  │ GET  /api/federation/sync/pull   — instance downloads  │   │
│  │ POST /api/federation/admin/force-merge — manual merge  │   │
│  │ GET  /api/federation/status      — hub health          │   │
│  │ GET  /api/federation/audit       — poisoning detection │   │
│  └─────────────────────────────────────────────────────────┘   │
│           ↓                           ↓                ↓         │
│  ┌────────────────┐ ┌──────────────────────┐ ┌──────────────┐  │
│  │ Q-Table Store  │ │ Merge Engine         │ │ Audit Log    │  │
│  ├────────────────┤ ├──────────────────────┤ ├──────────────┤  │
│  │ • Local Q-v1   │ │ • Weighted voting    │ │ • Per-community
│  │ • Global Q-v2  │ │ • Outlier detection  │ │   accuracy   │  │
│  │ • Q-v1 backup  │ │ • Reputation scoring │ │ • Merge logs │  │
│  │ • Version hist │ │ • Conflict resolution│ │ • Drift hist │  │
│  └────────────────┘ └──────────────────────┘ └──────────────┘  │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ Persistence Layer (PostgreSQL)                          │   │
│  ├─────────────────────────────────────────────────────────┤   │
│  │ • federation_instances (community metadata)             │   │
│  │ • q_table_snapshots (timestamped versions)              │   │
│  │ • community_accuracy (track reputation)                 │   │
│  │ • merge_audits (detect poisoning)                       │   │
│  │ • sync_events (API activity log)                        │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Sync Flow (Step-by-Step)

```
Timeline: Every T seconds (e.g., T=60s) or every N judgments (e.g., N=10)

┌─────────────┐
│ Sync Trigger│  (either timer or event-based)
└──────┬──────┘
       │
       ▼
┌──────────────────────────────────┐
│ 1. Hub requests Q-Tables         │  GET /api/federation/sync/status
│    from all instances            │  → {instance_id, last_sync_time, version}
└──────┬───────────────────────────┘
       │
       ▼
┌──────────────────────────────────┐
│ 2. Hub collects Q-Tables         │  Instance sends:
│    from N instances              │  {
│    (async, with timeout)         │    instance_id: "community_1",
└──────┬───────────────────────────┘    accuracy_score: 0.85,
       │                                 q_values: { ... }
       ▼                                }
┌──────────────────────────────────┐
│ 3. Merge Engine processes        │  Input:  N Q-Tables (possibly poisoned)
│    • Outlier detection           │  Process: weighted vote + exclusion
│    • Voting + weighting          │  Output:  merged Q-Table (global Q-v2)
│    • Poisoning detection         │
└──────┬───────────────────────────┘
       │
       ▼
┌──────────────────────────────────┐
│ 4. Update Global Q-Table         │  Save Q-v2 to DB
│    in hub database               │  Create snapshot: {version, timestamp}
└──────┬───────────────────────────┘
       │
       ▼
┌──────────────────────────────────┐
│ 5. Broadcast to all instances    │  POST /api/federation/sync/pull
│    (async, with retry)           │  ← {version: "2.1", q_values: {...}}
└──────┬───────────────────────────┘
       │
       ▼
┌──────────────────────────────────┐
│ 6. Instances pull + apply        │  Each instance updates its Q-Table
│    Global Q-v2 locally           │  Next judgment uses improved confidence
└──────────────────────────────────┘
```

---

## Part 2: Detailed Module Specs

### 2.1 Federation Hub API

**Framework:** FastAPI (Python)
**Port:** 8008 (or configurable)
**Auth:** API key per instance (env var `FEDERATION_API_KEY`)

#### Endpoint 1: POST /api/federation/sync/push

**Purpose:** Instance uploads local Q-Table to hub

```python
"""
POST /api/federation/sync/push

Request body:
{
  "instance_id": "community_1",           # unique identifier
  "instance_secret": "<api_key>",         # authentication
  "q_table": {
    "HOWL-HOWL": 0.82,
    "HOWL-WAG": 0.45,
    "WAG-HOWL": 0.51,
    ...
  },
  "accuracy_score": 0.85,                 # recent accuracy [0, 1]
                                          # (predicted == actual)
  "judgment_count": 487,                  # total judgments made
  "timestamp": "2026-02-27T15:30:00Z"
}

Response (200 OK):
{
  "status": "accepted",
  "message": "Q-Table received",
  "hub_version": "2.1",                   # current global version
  "next_sync_time": "2026-02-27T15:31:00Z"
}

Response (409 Conflict):
{
  "status": "rejected",
  "reason": "instance_secret invalid"
}
"""

@app.post("/api/federation/sync/push")
async def push_q_table(req: PushQTableRequest) -> PushQTableResponse:
    # 1. Validate API key
    if not validate_api_key(req.instance_id, req.instance_secret):
        raise HTTPException(status_code=401, detail="Invalid secret")

    # 2. Store local Q-Table (for audit trail)
    db.save_local_snapshot(
        instance_id=req.instance_id,
        q_table=req.q_table,
        accuracy=req.accuracy_score,
        timestamp=req.timestamp
    )

    # 3. Update community metadata
    db.upsert_instance(
        instance_id=req.instance_id,
        last_sync=now(),
        accuracy_score=req.accuracy_score,
        judgment_count=req.judgment_count
    )

    # 4. Return global version (for client sync detection)
    global_version = db.get_current_global_version()
    next_sync = datetime.now() + timedelta(seconds=SYNC_INTERVAL)

    return PushQTableResponse(
        status="accepted",
        message="Q-Table received",
        hub_version=global_version,
        next_sync_time=next_sync
    )
```

#### Endpoint 2: GET /api/federation/sync/pull

**Purpose:** Instance downloads latest global Q-Table from hub

```python
"""
GET /api/federation/sync/pull?instance_id=community_1&current_version=2.0

Response (200 OK):
{
  "status": "updated",
  "version": "2.1",                       # new global version
  "q_table": {
    "HOWL-HOWL": 0.81,                    # merged (weighted avg)
    "HOWL-WAG": 0.48,
    ...
  },
  "merged_from_count": 5,                 # 5 instances contributed
  "timestamp": "2026-02-27T15:31:00Z",
  "merge_algorithm": "weighted_vote",
  "outliers_excluded": 0                  # 0 communities poisoned
}

Response (304 Not Modified):
{
  "status": "no_update",
  "current_version": "2.1"
}
"""

@app.get("/api/federation/sync/pull")
async def pull_q_table(
    instance_id: str,
    current_version: str
) -> PullQTableResponse:
    # 1. Validate instance_id
    if not db.instance_exists(instance_id):
        raise HTTPException(status_code=404, detail="Instance not found")

    # 2. Fetch current global version
    global_version = db.get_current_global_version()

    # 3. Check if update needed
    if current_version == global_version:
        return PullQTableResponse(
            status="no_update",
            current_version=global_version
        )

    # 4. Return global Q-Table
    global_q = db.get_global_q_table(version=global_version)
    merge_metadata = db.get_merge_metadata(version=global_version)

    return PullQTableResponse(
        status="updated",
        version=global_version,
        q_table=global_q,
        merged_from_count=merge_metadata["instance_count"],
        timestamp=merge_metadata["timestamp"],
        merge_algorithm=merge_metadata["algorithm"],
        outliers_excluded=merge_metadata["outlier_count"]
    )
```

#### Endpoint 3: POST /api/federation/admin/force-merge

**Purpose:** Manually trigger Q-Table merge (for testing or emergency)

```python
"""
POST /api/federation/admin/force-merge

Request:
{
  "admin_secret": "<admin_key>",
  "reason": "manual test",
  "algorithm": "weighted_vote"              # or "majority_vote", "median"
}

Response (200 OK):
{
  "status": "completed",
  "new_version": "2.2",
  "instances_merged": 5,
  "outliers_excluded": ["community_3"],     # community_3 was poisoned
  "duration_ms": 234
}
"""

@app.post("/api/federation/admin/force-merge")
async def force_merge(req: ForceMergeRequest) -> ForceMergeResponse:
    # 1. Validate admin secret
    if not validate_admin_secret(req.admin_secret):
        raise HTTPException(status_code=401, detail="Unauthorized")

    # 2. Trigger merge engine
    merge_engine = MergeEngine(algorithm=req.algorithm)
    result = await merge_engine.merge()

    # 3. Log the force-merge
    db.log_admin_action(
        action="force_merge",
        reason=req.reason,
        admin_version=result.new_version,
        timestamp=now()
    )

    return ForceMergeResponse(
        status="completed",
        new_version=result.new_version,
        instances_merged=result.instance_count,
        outliers_excluded=result.outlier_instances,
        duration_ms=result.duration_ms
    )
```

#### Endpoint 4: GET /api/federation/status

**Purpose:** Health check + current state

```python
"""
GET /api/federation/status

Response (200 OK):
{
  "hub_healthy": true,
  "current_global_version": "2.1",
  "last_merge_time": "2026-02-27T15:31:00Z",
  "instances_connected": 5,
  "instances_total": 5,
  "average_accuracy": 0.82,
  "q_table_divergence": 0.03,              # max deviation from mean
  "poisoning_detected": false,
  "uptime_hours": 24.5
}
"""

@app.get("/api/federation/status")
async def get_status() -> StatusResponse:
    instances = db.get_all_instances()
    global_version = db.get_current_global_version()
    last_merge = db.get_last_merge_time()

    avg_accuracy = mean([i.accuracy_score for i in instances])
    connected_count = len([i for i in instances if i.is_healthy()])

    divergence = await compute_divergence(instances, global_version)
    poisoning_risk = divergence > 0.1  # threshold

    return StatusResponse(
        hub_healthy=True,
        current_global_version=global_version,
        last_merge_time=last_merge,
        instances_connected=connected_count,
        instances_total=len(instances),
        average_accuracy=avg_accuracy,
        q_table_divergence=divergence,
        poisoning_detected=poisoning_risk,
        uptime_hours=get_uptime()
    )
```

---

### 2.2 Merge Engine (Core Logic)

**Purpose:** Combine N Q-Tables into 1 global Q-Table

**File:** `cynic/federation/merge_engine.py`

```python
"""
Merge Engine: Combine Q-Tables with outlier detection + weighted voting.

Key algorithm:
1. For each (predicted, actual) verdict pair:
   a) Collect Q-values from all communities
   b) Calculate mean and std dev
   c) Flag outliers: |q - mean| > 3*stdev
   d) Compute weighted mean (high-accuracy communities count more)
   e) Clamped to [0, 1]
"""

from typing import Dict, List, Tuple
import statistics
from dataclasses import dataclass
import json

@dataclass
class MergeResult:
    """Result of a merge operation."""
    new_version: str
    merged_q_table: Dict[str, float]  # "HOWL-HOWL" → 0.81
    instance_count: int
    outlier_instances: List[str]
    duration_ms: float
    algorithm: str


class QTableCodec:
    """Helper for converting Q-Table between dict and string keys."""

    @staticmethod
    def to_string_key(verdict_pair: Tuple[str, str]) -> str:
        """Convert (HOWL, WAG) to 'HOWL-WAG'."""
        return f"{verdict_pair[0]}-{verdict_pair[1]}"

    @staticmethod
    def to_tuple_key(string_key: str) -> Tuple[str, str]:
        """Convert 'HOWL-WAG' to (HOWL, WAG)."""
        parts = string_key.split("-")
        return (parts[0], parts[1])

    @staticmethod
    def normalize(q_table: Dict) -> Dict[str, float]:
        """Ensure all keys are strings (for JSON serialization)."""
        normalized = {}
        for key, value in q_table.items():
            if isinstance(key, tuple):
                string_key = QTableCodec.to_string_key(key)
                normalized[string_key] = value
            else:
                normalized[key] = value
        return normalized


class MergeEngine:
    """
    Weighted voting merge with outlier detection.
    """

    def __init__(self, algorithm: str = "weighted_vote"):
        """
        Initialize merge engine.

        Args:
            algorithm: "weighted_vote" (default), "majority_vote", or "median"
        """
        self.algorithm = algorithm

    async def merge(
        self,
        local_q_tables: Dict[str, Dict[str, float]],
        accuracy_scores: Dict[str, float],
        previous_version: str
    ) -> MergeResult:
        """
        Merge Q-Tables from multiple communities.

        Args:
            local_q_tables: {instance_id → {string_key → q_value}}
            accuracy_scores: {instance_id → accuracy [0, 1]}
            previous_version: previous global version (for rollback)

        Returns:
            MergeResult with new global Q-Table
        """
        start_time = time.time()

        # 1. Get all verdict pairs
        all_keys = set()
        for q_table in local_q_tables.values():
            all_keys.update(q_table.keys())

        # 2. For each pair, compute weighted mean
        global_q_table = {}
        outlier_instances = set()

        for key in all_keys:
            # Collect values from communities that have this key
            values_with_weights = []
            for instance_id, q_table in local_q_tables.items():
                if key in q_table:
                    q_val = q_table[key]
                    accuracy = accuracy_scores.get(instance_id, 0.5)
                    values_with_weights.append((q_val, accuracy))

            # Compute mean
            values = [v for v, w in values_with_weights]
            weights = [w for v, w in values_with_weights]
            mean = statistics.mean(values) if values else 0.5

            # Detect outliers (>3σ from mean)
            stdev = statistics.stdev(values) if len(values) > 1 else 0.0
            threshold = 3 * stdev

            # Filter non-outliers
            filtered = [
                (v, w) for (v, w) in values_with_weights
                if abs(v - mean) <= threshold
            ]

            # If all rejected (suspicious), use all values
            if not filtered:
                filtered = values_with_weights

            # Compute weighted mean
            if filtered:
                filtered_values = [v for v, w in filtered]
                filtered_weights = [w for v, w in filtered]

                weighted_mean = sum(v * w for v, w in filtered) / sum(filtered_weights)
                global_q_table[key] = max(0.0, min(1.0, weighted_mean))
            else:
                global_q_table[key] = 0.5

            # Track outliers
            for (instance_id, q_table) in local_q_tables.items():
                if key in q_table:
                    q_val = q_table[key]
                    if abs(q_val - mean) > threshold:
                        outlier_instances.add(instance_id)

        # 3. Create new version
        duration = (time.time() - start_time) * 1000  # ms
        new_version = await self._create_version(
            previous_version=previous_version,
            outlier_count=len(outlier_instances)
        )

        return MergeResult(
            new_version=new_version,
            merged_q_table=global_q_table,
            instance_count=len(local_q_tables),
            outlier_instances=list(outlier_instances),
            duration_ms=duration,
            algorithm=self.algorithm
        )

    async def _create_version(self, previous_version: str, outlier_count: int) -> str:
        """
        Create new version string (e.g., "2.1" → "2.2").

        If outliers detected, add suffix: "2.2-poison_detected"
        """
        parts = previous_version.split("-")[0].split(".")
        major, minor = int(parts[0]), int(parts[1])
        new_minor = minor + 1

        new_version = f"{major}.{new_minor}"
        if outlier_count > 0:
            new_version += "-poison_detected"

        return new_version
```

---

### 2.3 Instance Client Library

**Purpose:** Simplify sync from CYNIC instances

**File:** `cynic/federation/client.py`

```python
"""
Federation Client Library

Used by CYNIC instances to sync with hub.
"""

import httpx
import asyncio
import json
from typing import Dict
from datetime import datetime

class FederationClient:
    """Client for communicating with federation hub."""

    def __init__(
        self,
        hub_url: str,
        instance_id: str,
        api_key: str,
        sync_interval_seconds: int = 60
    ):
        self.hub_url = hub_url
        self.instance_id = instance_id
        self.api_key = api_key
        self.sync_interval = sync_interval_seconds
        self.current_version = "1.0"
        self.http_client = httpx.AsyncClient(timeout=30.0)

    async def push_q_table(
        self,
        q_table: Dict[str, float],
        accuracy_score: float,
        judgment_count: int
    ) -> bool:
        """
        Upload local Q-Table to hub.

        Args:
            q_table: Dict with string keys like "HOWL-HOWL"
            accuracy_score: [0, 1]
            judgment_count: total judgments made

        Returns:
            True if successful, False otherwise
        """
        try:
            response = await self.http_client.post(
                f"{self.hub_url}/api/federation/sync/push",
                json={
                    "instance_id": self.instance_id,
                    "instance_secret": self.api_key,
                    "q_table": q_table,
                    "accuracy_score": accuracy_score,
                    "judgment_count": judgment_count,
                    "timestamp": datetime.now().isoformat()
                }
            )

            if response.status_code == 200:
                # Update hub version for next pull
                data = response.json()
                self.current_version = data["hub_version"]
                return True
            else:
                print(f"Push failed: {response.status_code}")
                return False

        except httpx.RequestError as e:
            print(f"Push error: {e}")
            return False

    async def pull_q_table(self) -> Dict[str, float] | None:
        """
        Download global Q-Table from hub.

        Returns:
            Updated Q-Table with string keys, or None if no update
        """
        try:
            response = await self.http_client.get(
                f"{self.hub_url}/api/federation/sync/pull",
                params={
                    "instance_id": self.instance_id,
                    "current_version": self.current_version
                }
            )

            if response.status_code == 200:
                data = response.json()
                if data["status"] == "updated":
                    q_table = data["q_table"]
                    self.current_version = data["version"]
                    return q_table
                else:
                    # No update
                    return None
            else:
                print(f"Pull failed: {response.status_code}")
                return None

        except httpx.RequestError as e:
            print(f"Pull error: {e}")
            return None

    async def sync_loop(self, q_table_source, on_update=None):
        """
        Continuous sync loop.

        Call this as a background task.

        Args:
            q_table_source: callable that returns (q_table_dict, accuracy, count)
            on_update: optional callback when Q-Table updates
        """
        while True:
            try:
                # Get current Q-Table from source
                q_table, accuracy, count = q_table_source()

                # Push to hub
                await self.push_q_table(q_table, accuracy, count)

                # Pull from hub
                updated = await self.pull_q_table()
                if updated and on_update:
                    on_update(updated)

                # Wait for next sync
                await asyncio.sleep(self.sync_interval)

            except Exception as e:
                print(f"Sync loop error: {e}")
                await asyncio.sleep(5)  # retry after 5s
```

---

### 2.4 Configuration (`.env`)

```bash
# Federation settings
FEDERATION_ENABLED=true
FEDERATION_HUB_URL=http://federation-hub:8008
CYNIC_INSTANCE_ID=community_1
FEDERATION_API_KEY=<generated_key_for_instance>
FEDERATION_SYNC_INTERVAL=60              # sync every 60 seconds
```

---

## Part 3: Testing Strategy

### 3.1 Unit Tests

**File:** `tests/federation/test_merge_engine.py`

```python
import pytest
from cynic.perception.federation.merge_engine import MergeEngine

@pytest.mark.asyncio
async def test_simple_merge():
    """Test basic Q-Table merge."""
    engine = MergeEngine()

    # Two communities with same Q-values
    local_q_tables = {
        "community_1": {"HOWL-HOWL": 0.8},
        "community_2": {"HOWL-HOWL": 0.82}
    }
    accuracy_scores = {
        "community_1": 0.9,
        "community_2": 0.9
    }

    result = await engine.merge(local_q_tables, accuracy_scores, "1.0")

    # Merged Q-value should be near average
    assert 0.80 <= result.merged_q_table["HOWL-HOWL"] <= 0.82


@pytest.mark.asyncio
async def test_outlier_detection():
    """Test poisoning detection (outlier rejection)."""
    engine = MergeEngine()

    # Community 3 is poisoning
    local_q_tables = {
        "community_1": {"HOWL-HOWL": 0.8},
        "community_2": {"HOWL-HOWL": 0.82},
        "community_3": {"HOWL-HOWL": 0.1}  # way off
    }
    accuracy_scores = {
        "community_1": 0.9,
        "community_2": 0.9,
        "community_3": 0.9
    }

    result = await engine.merge(local_q_tables, accuracy_scores, "1.0")

    # Merged Q-value should ignore community_3
    merged = result.merged_q_table["HOWL-HOWL"]
    assert 0.80 <= merged <= 0.82
    assert "community_3" in result.outlier_instances


@pytest.mark.asyncio
async def test_weighted_voting():
    """Test that high-accuracy communities are weighted more."""
    engine = MergeEngine()

    # Community 1 has high accuracy, Community 2 has low
    local_q_tables = {
        "community_1": {"HOWL-HOWL": 0.9},
        "community_2": {"HOWL-HOWL": 0.5}
    }
    accuracy_scores = {
        "community_1": 0.95,  # high accuracy
        "community_2": 0.50   # low accuracy
    }

    result = await engine.merge(local_q_tables, accuracy_scores, "1.0")

    # Merged should be closer to 0.9 than 0.5
    merged = result.merged_q_table["HOWL-HOWL"]
    assert merged > 0.7
```

---

## Part 4: Rollout Plan

### Phase 2A: Build Hub (Week 1)
- [ ] Implement FastAPI endpoints (API layer)
- [ ] Implement MergeEngine (core logic)
- [ ] Build FederationClient library
- [ ] Write unit tests for merge logic

**Deliverable:** Standalone hub running, tested in isolation

### Phase 2B: Integrate Instances (Week 2)
- [ ] Add federation support to UnifiedQTable
- [ ] Add env vars + config
- [ ] Add sync loop background task
- [ ] Test with 3 communities (controlled environment)
- [ ] Monitor Q-Table convergence

**Deliverable:** 3 communities learning from each other

### Phase 2C: Testing + Hardening (Week 3)
- [ ] Poison detection tests (inject bad Q-values)
- [ ] Network partition scenarios
- [ ] Load testing (10+ communities)
- [ ] Documentation + runbooks

**Deliverable:** Production-ready federation hub

---

## Part 5: Success Criteria

### Technical Metrics
- [ ] Hub latency <100ms per sync
- [ ] Merge algorithm detects poisoning (>3σ outliers)
- [ ] 5+ communities successfully sync without divergence
- [ ] Q-Table convergence within 3-5 sync cycles
- [ ] Zero data corruption after 1000+ syncs

### Operational Metrics
- [ ] Hub uptime >99.9%
- [ ] Hub CPU <20%, memory <2GB
- [ ] Sync messages <10 KB per instance per cycle

### Business Metrics
- [ ] Communities report improved accuracy (confidence improvement)
- [ ] Fairness: early adopters don't have unfair advantage
- [ ] Ecosystem effect: communities value pooled learning

---

**Document Version:** 1.0
**Date:** 2026-02-27
**Status:** Ready for Phase 2 implementation
