# CYNIC Testing Strategy

> "Don't trust, verify" — κυνικός

## Test Categories

### 1. Unit Tests (113 tests, 5 seconds)
**Location**: `cynic/cynic/tests/test_*.py` (excluding `test_integration_*`)
**Run**: `py -3.13 cynic/run_tests.py cynic/tests/`
**Purpose**: Fast feedback on individual components
**Dependencies**: None (all mocked)
**CI/CD**: ✅ Always runs

```bash
py -3.13 cynic/run_tests.py cynic/tests/
# Output: 113 passed in 5.69s
```

### 2. Integration Tests (external dependencies)
**Location**: `cynic/cynic/tests/test_integration_*.py`
**Mark**: `@pytest.mark.integration`
**Run locally**: `py -3.13 -m pytest -m integration cynic/cynic/tests/`
**CI/CD**: ⏭️ SKIPPED (no external services in CI)

#### A. Real Ollama Integration
**File**: `cynic/cynic/tests/test_integration_real_ollama.py`
**Requires**: Ollama at `localhost:11434`
**Tests**:
- OllamaAdapter real API calls (not mocked)
- DogCognition judgment with real Ollama scoring
- Temporal MCTS with 7-parallel Ollama calls
- Model discovery
- Performance: single call latency, parallel throughput

**Setup**:
```bash
docker-compose up ollama
ollama pull qwen2.5-coder:7b
```

**Run**:
```bash
py -3.13 -m pytest -m integration cynic/cynic/tests/test_integration_real_ollama.py -v
```

#### B. Real SurrealDB Integration
**File**: `cynic/cynic/tests/test_integration_real_surrealdb.py`
**Requires**: SurrealDB at `ws://localhost:8000`
**Tests**:
- Real connection to SurrealDB
- Create/retrieve/delete cycle (persistence)
- Schema definition and queries
- HNSW vector search setup
- Performance: write/read latency, batch throughput

**Setup**:
```bash
docker-compose up surrealdb
```

**Run**:
```bash
py -3.13 -m pytest -m integration cynic/cynic/tests/test_integration_real_surrealdb.py -v
```

---

## Docker Deployment

### Full Stack (CYNIC + Ollama + SurrealDB)

**Build and start all services**:
```bash
cd cynic
docker-compose up --build
```

This starts:
- **cynic-kernel** (FastAPI on port 8000)
- **ollama** (LLM inference on port 11434)
- **surrealdb** (Storage on port 8000, different container)

**Verify health**:
```bash
curl http://localhost:8000/health
curl http://localhost:11434/api/tags
```

### Run Integration Tests in Docker

After `docker-compose up`, run tests from host (tests connect to container services):

```bash
# Terminal 1: Start services
cd cynic
docker-compose up

# Terminal 2: Run integration tests
cd cynic
py -3.13 -m pytest -m integration cynic/tests/test_integration_real_*.py -v

# Or run both in one command (services start in background)
cd cynic && docker-compose up -d && py -3.13 -m pytest -m integration cynic/tests/ && docker-compose down
```

---

## Test Execution Matrix

| Test Type | Command | Time | CI? | External Deps? |
|-----------|---------|------|-----|---|
| Unit | `py -3.13 run_tests.py cynic/tests/` | 5s | ✅ | None |
| Integration (Ollama) | `py -3.13 -m pytest -m integration test_integration_real_ollama.py` | 30-60s | ⏭️ | Ollama |
| Integration (SurrealDB) | `py -3.13 -m pytest -m integration test_integration_real_surrealdb.py` | 10-20s | ⏭️ | SurrealDB |
| Full Stack | `docker-compose up && pytest -m integration` | 120s | ⏭️ | All |

---

## Paradigm Shift Validation

**Empirical tests proving φ encoding**:
- `test_eleven_dog_consensus.py`: 5 dogs reach consensus via geometric mean (O(1))
- `test_integration_empirical.py`: φ naturally emerges in confidence, cost scaling, variance

**Run empirical validation**:
```bash
py -3.13 -m pytest cynic/tests/test_eleven_dog_consensus.py -v
py -3.13 -m pytest cynic/tests/test_integration_empirical.py -v
```

Expected output:
```
test_five_dog_consensus_geometric_mean PASSED
test_consensus_stability_with_noise PASSED
test_consensus_breaks_on_conflict PASSED
test_consensus_aggregation_is_constant_time PASSED
test_dog_confidence_phi_bounded PASSED
test_five_dogs_all_judge_in_parallel PASSED
test_signal_diversity_drives_entropy PASSED
test_phi_ratio_in_judgment_distribution PASSED

8 passed in X.XXs
```

---

## Key Patterns

### Skipping Gracefully in Integration Tests

```python
@pytest.mark.integration
async def test_something_requiring_ollama():
    if not has_ollama:
        pytest.skip("Ollama not available at localhost:11434")

    # Real test code here
```

**Behavior**:
- If Ollama running: test executes normally
- If Ollama not running: test skips (not fail)
- In CI: all integration tests skipped (marked with `@pytest.mark.integration`)

### Connection String Configuration

Tests use hardcoded localhost addresses:
- Ollama: `http://localhost:11434` (configurable via `CYNIC_OLLAMA_BASE_URL` env var)
- SurrealDB: `ws://localhost:8000` (configurable via `CYNIC_SURREAL_URL` env var)

To point to different service:
```bash
export CYNIC_OLLAMA_BASE_URL=http://remote-ollama:11434
export CYNIC_SURREAL_URL=ws://remote-db:8000
py -3.13 -m pytest -m integration cynic/tests/
```

---

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Unit Tests
on: [push, pull_request]
jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-python@v4
        with:
          python-version: "3.13"
      - run: pip install poetry
      - run: poetry install
      - run: py -3.13 cynic/run_tests.py cynic/tests/
        # Integration tests skipped automatically (no external services)
```

### Local Pre-Commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit
set -e
cd cynic
py -3.13 run_tests.py cynic/tests/
echo "✓ All unit tests passed"
```

---

## Philosophy

```
UNIT TESTS    — Trust the components, verify fast (5s)
INTEGRATION   — Don't trust external services, verify with real deps
EMPIRICAL     — Prove φ encoding emerges naturally from real data

"φ distrusts φ" — even passing tests leave 38.2% doubt. Validate again.
```
