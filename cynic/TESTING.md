# CYNIC Testing Strategy

## Overview

CYNIC uses a two-tier testing strategy:

1. **Unit Tests** (2265 tests, run in CI)
2. **Integration Tests** (real dependencies, skip in CI, run locally)

## Unit Tests

Fast, isolated, use mocks. Run in CI.

```bash
# Windows canonical command
PYTHONUTF8=1 python3.13 -m pytest tests/

# Or via run_tests.py
python3.13 run_tests.py tests/

# Expected output
# =============== 2265 passed, 1 skipped, 51 deselected in 28.31s ===============
```

## Integration Tests

Real dependencies, skip in CI by default, run explicitly locally.

### Running Integration Tests

```bash
# Run all integration tests
pytest -m integration tests/test_integration/

# Run specific integration test
pytest -m integration tests/test_integration/test_cognition_real.py

# Run with verbose output
pytest -m integration tests/test_integration/ -v
```

### Integration Test Categories

#### 1. Cognition Integration (Real Ollama)
**File**: `tests/test_integration/test_cognition_real.py`

**Requires**: Ollama running at `localhost:11434`

**Setup**:
```bash
# Start Ollama (if not already running)
ollama serve

# In another terminal:
ollama pull gemma2:2b  # or your preferred model
```

**Tests**:
- Real LLM judgment (no mocks)
- Multi-perspective temporal analysis (7 parallel calls)

**Skip condition**: Gracefully skips if Ollama not available

#### 2. Perception Integration (Real File System)
**File**: `tests/test_integration/test_perception_real.py`

**Requires**: None (uses temporary directories and system metrics)

**Tests**:
- Real git operations on test repository
- Real system health metrics (CPU, memory, disk)
- Real file system changes

**Skip condition**: None (always runs)

#### 3. Storage Integration (Real Database)
**File**: `tests/test_integration/test_storage_real.py`

**Requires**: SurrealDB or PostgreSQL configured

**Setup with SurrealDB**:
```bash
# Start SurrealDB
surreal start --bind 0.0.0.0:8000 memory

# Set environment variable
export CYNIC_STORAGE_URL="surreal://root:root@localhost:8000/test/test"
```

**Setup with PostgreSQL**:
```bash
# Using Docker (if available)
docker run --name cynic_py -e POSTGRES_PASSWORD=postgres -p 5432:5432 postgres

# Set environment variable
export CYNIC_DATABASE_URL="postgresql://postgres:postgres@localhost:5432/cynic"
```

**Tests**:
- Real save/load cycle (SurrealDB)
- Real asyncpg connection (PostgreSQL)

**Skip condition**: Gracefully skips if database not configured

#### 4. SDK Bridge Integration (Real Claude API)
**File**: `tests/test_integration/test_sdk_real.py`

**Requires**: `ANTHROPIC_API_KEY` environment variable set

**Setup**:
```bash
# Set API key
export ANTHROPIC_API_KEY="sk-..."

# Optionally set model (defaults to haiku-4-5)
export CLAUDE_MODEL="claude-opus-4-6"
```

**Tests**:
- Real Claude API availability check
- Real completion call to Claude
- Claude Code runner executable verification

**Skip condition**: Gracefully skips if ANTHROPIC_API_KEY not set

### Environment Variables

```bash
# Ollama configuration
OLLAMA_BASE_URL=http://localhost:11434   # Default

# SurrealDB configuration
CYNIC_STORAGE_URL=surrealdb://root:root@localhost:8000/test/test

# PostgreSQL configuration
CYNIC_DATABASE_URL=postgresql://user:password@localhost:5432/cynic

# Claude API configuration
ANTHROPIC_API_KEY=sk-...
CLAUDE_MODEL=claude-haiku-4-5-20251001  # Default
```

## CI/CD Behavior

By default, pytest is configured to **skip integration tests**:

```ini
addopts = "-m 'not integration and not db_integration'"
```

This means:
- CI runs unit tests only: `pytest tests/`
- CI never hits external dependencies (Ollama, database, Claude API)
- CI is fast and reliable

Integration tests must be run explicitly:
```bash
pytest -m integration tests/
```

## Test Philosophy

**Unit Tests** verify:
- Component logic in isolation
- Behavior with mocked dependencies
- Fast feedback loop (28s for 2265 tests)
- 100% deterministic results

**Integration Tests** verify:
- Real end-to-end flows
- Actual third-party services work
- Performance characteristics are acceptable
- Data persistence across restarts

Together, they ensure CYNIC is both **reliable** (unit tests) and **functional** (integration tests).

## Development Workflow

1. **Make code changes**
2. **Run unit tests locally**: `python3.13 run_tests.py tests/`
3. **If working on Ollama/DB features**: Run integration tests
4. **Commit & push** — CI runs unit tests only
5. **Optional**: Request integration test run in CI if needed

## Troubleshooting

### Tests still pass but Ollama seems broken?
```bash
# Verify Ollama is actually running
curl http://localhost:11434/api/tags

# Integration tests will gracefully skip if unavailable
pytest -m integration tests/test_integration/test_cognition_real.py -v
```

### Database tests skip but I set CYNIC_STORAGE_URL?
```bash
# Verify environment variable is set
echo $CYNIC_STORAGE_URL

# Some shells don't export to subprocesses
export CYNIC_STORAGE_URL="..."
pytest -m integration tests/test_integration/test_storage_real.py -v
```

### Claude API tests skip but I have ANTHROPIC_API_KEY?
```bash
# Verify API key is set
echo $ANTHROPIC_API_KEY

# Test API connectivity
curl https://api.anthropic.com/v1/models \
  -H "x-api-key: $ANTHROPIC_API_KEY"
```

## Adding New Integration Tests

1. Create file in `tests/test_integration/test_*.py`
2. Mark test class with `@pytest.mark.integration`
3. Include graceful skip if dependencies unavailable
4. Add documentation about required setup
5. Run with: `pytest -m integration tests/test_integration/test_*.py`

Example:
```python
@pytest.mark.integration
class TestMyFeature:
    @pytest.mark.asyncio
    async def test_something_real(self):
        # Check if dependency available
        if not dep_available():
            pytest.skip("Dependency not available")

        # Test with real dependency
        result = await real_operation()
        assert result is not None
```

---

**Last Updated**: 2026-02-20
**Test Suite Size**: 2265 unit + 4 integration suites
**CI Configuration**: Unit tests only (fast & reliable)
