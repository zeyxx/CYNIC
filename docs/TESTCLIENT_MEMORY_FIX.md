# TestClient Memory Leak Fix Guide

**Issue:** FastAPI TestClient creates expensive organism awakenings (~1.5GB per instance).
**Root Cause:** Test methods creating `TestClient(app)` in each method instead of sharing fixtures.
**Impact:** 68GB+ potential RAM when running all tests.

---

## Status

| File | Tests | Pattern | Status | Est. RAM Save |
|------|-------|---------|--------|--------------|
| `test_health_enhanced.py` | 11 | ✅ FIXED | 8/8 passing | ~12GB → ~1.5GB |
| `test_governance.py` | 20 | 🔴 NEEDS FIX | N/A | ~30GB → ~1.5GB |
| `test_consciousness_ecosystem.py` | 7 | 🔴 NEEDS FIX | N/A | ~10GB → ~1.5GB |
| `test_phase3_event_first_api.py` | 17 | 🔴 NEEDS FIX | N/A | ~25GB → ~1.5GB |
| `test_ws_ecosystem.py` | 2 | 🔴 NEEDS FIX | N/A | ~3GB → ~1.5GB |

---

## The Problem Pattern

```python
# ❌ WRONG — creates N organisms (one per test)
class TestSomething:
    def test_a(self):
        with TestClient(app) as client:      # Organism 1 awakens
            response = client.get("/endpoint")
            assert ...

    def test_b(self):
        with TestClient(app) as client:      # Organism 2 awakens
            response = client.get("/other")
            assert ...

    # 17 tests = 17 organisms = 25GB RAM
```

---

## The Solution Pattern

```python
# ✅ RIGHT — creates 1 organism per class
@pytest.fixture(scope="class")
def client():
    """Class-scoped fixture — reuses single organism."""
    with TestClient(app) as c:
        yield c

class TestSomething:
    def test_a(self, client):              # Reuses organism
        response = client.get("/endpoint")
        assert ...

    def test_b(self, client):              # Reuses organism
        response = client.get("/other")
        assert ...

    # 17 tests = 1 organism = 1.5GB RAM
```

---

## How to Fix Each File

### Step 1: Add Fixture Before Class

Add this before the first test class in the file:

```python
@pytest.fixture(scope="class")
def client():
    """Class-scoped HTTP client.

    WARNING: Do not create additional TestClient(app) instances in test methods.
    Each triggers expensive organism awakening (~1.5GB).
    """
    with TestClient(app) as c:
        yield c
```

### Step 2: Update Test Method Signatures

Change:
```python
def test_something(self):
```

To:
```python
def test_something(self, client):
```

### Step 3: Remove `with TestClient` Block

Change:
```python
def test_something(self, client):
    """docstring"""
    with TestClient(app) as client:
        response = client.get(...)
        assert ...
```

To:
```python
def test_something(self, client):
    """docstring"""
    response = client.get(...)
    assert ...
```

**Important:** Remove the `with` block AND un-indent all code inside it by 4 spaces.

### Step 4: If Multiple Classes Exist

Create separate fixtures for each class:

```python
@pytest.fixture(scope="class")
def governance_client():
    with TestClient(app) as c:
        yield c

class TestGovernanceEndpoints:
    def test_x(self, governance_client):
        client = governance_client
        ...

@pytest.fixture(scope="class")
def models_client():
    with TestClient(app) as c:
        yield c

class TestGovernanceModels:
    def test_y(self, models_client):
        client = models_client
        ...
```

---

## Files Needing Fix

### CRITICAL — Fix Now (57GB+ potential RAM impact)

1. **`cynic/tests/api/routers/test_governance.py`**
   - Tests: 20
   - Classes: 2 (TestGovernanceEndpoints, TestGovernanceModels)
   - RAM Impact: ~30GB → ~1.5GB

2. **`cynic/tests/test_phase3_event_first_api.py`**
   - Tests: 17
   - Classes: 1
   - RAM Impact: ~25GB → ~1.5GB

3. **`cynic/tests/api/routers/test_consciousness_ecosystem.py`**
   - Tests: 7
   - Classes: 1
   - RAM Impact: ~10GB → ~1.5GB

### HIGH — Fix Soon

4. **`cynic/tests/api/routers/test_ws_ecosystem.py`**
   - Tests: 2
   - Classes: 1
   - RAM Impact: ~3GB → ~1.5GB

### LOW — Already Fixed ✅

5. **`cynic/tests/api/routers/test_health_enhanced.py`**
   - Tests: 11
   - Classes: 2
   - Status: ✅ FIXED (commit d147597)

---

## Verification

After fixing a file:

```bash
# Run just that test file
pytest cynic/tests/api/routers/test_governance.py -v

# Monitor RAM usage in Task Manager
# Should see ~1.5GB organism awakening, not 20+GB
```

---

## Root Cause Analysis

### Why CYNIC Creates So Much RAM

```python
awaken()
├─ AxiomArchitecture (9 axioms)
├─ 11 Dogs (neural networks)
├─ Learning loops (SONA)
│  ├─ Q-Learning (6 states × N entries)
│  ├─ Thompson (arms tracking)
│  ├─ EWC Fisher consolidation
│  └─ 8 others...
├─ Orchestrators & coordinators
├─ Schedulers & event buses
├─ Storage connections (SurrealDB/PostgreSQL)
├─ MCP bridge
├─ LLM Registry
├─ Telemetry & monitoring
└─ ...~50 total components

Total: ~1.5GB per organism instance
```

### Why Tests Create So Many

```
Each TestClient(app) context:
  1. Enters lifespan context
  2. Calls awaken() synchronously
  3. Allocates all 50+ components
  4. Waits for LLM discovery (blocks on network)
  5. Exits, but memory remains until GC
  6. Repeat for each test method

Result: Sequential tests → Sequential organisms → Linear RAM growth
```

### The Fix

```
Using class-scoped fixtures:
  1. One organism per test class (not per method)
  2. Reused across all methods in class
  3. Destroyed only at class end
  4. ~99% reduction in organism count
```

---

## Prevention

Add this comment to new test files using TestClient:

```python
"""
MEMORY WARNING: If you create TestClient(app) in test methods, each one
will awaken a full CYNIC organism (~1.5GB). Use pytest class-scoped fixtures
instead. See docs/TESTCLIENT_MEMORY_FIX.md for the pattern.
"""
```

---

## Automated Fix Tool (Optional)

To bulk-fix all remaining files, we can create a script. Contact your team lead if you need this.

**Estimated time to fix remaining 4 files manually:** 30-45 minutes
**Estimated time with automated script:** 5 minutes
