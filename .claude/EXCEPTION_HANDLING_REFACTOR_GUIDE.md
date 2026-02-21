# Exception Handling Refactoring Guide

**Goal**: Replace 374 bare `except Exception` handlers with specific exception types

**Progress**: 3/374 done (3 bare except: fixed)

---

## THE PATTERNS (How to Fix)

### Pattern 1: Database/Storage Operations

**Context**: asyncpg calls, file operations, persistence

**BEFORE**:
```python
try:
    result = await db.query(sql)
except Exception as e:
    logger.error("DB error: %s", e)
    return None
```

**AFTER**:
```python
try:
    result = await db.query(sql)
except asyncpg.Error as e:
    logger.error("DB error: %s", e)
    raise PersistenceError(f"Database failed: {e}") from e
```

**Apply to files**:
- api/routers/nervous.py (5 handlers)
- core/storage/postgres.py (all handlers)

### Pattern 2: JSON/Configuration Parsing

**Context**: json.loads(), config parsing, YAML parsing

**BEFORE**:
```python
try:
    data = json.loads(response)
except Exception as e:
    logger.error("Parse failed: %s", e)
    return {}
```

**AFTER**:
```python
try:
    data = json.loads(response)
except (json.JSONDecodeError, ValueError) as e:
    logger.error("Parse failed: %s", e)
    raise ValidationError(f"Invalid JSON: {e}") from e
```

**Apply to files**:
- api/routers/ws.py (8 handlers)
- api/handlers/escore.py (9 handlers)

### Pattern 3: LLM/API Calls

**Context**: ollama requests, HTTP calls, model inference

**BEFORE**:
```python
try:
    result = await llm.complete(prompt)
except Exception as e:
    logger.error("LLM error: %s", e)
    return None
```

**AFTER**:
```python
try:
    result = await llm.complete(prompt)
except (httpx.RequestError, TimeoutError) as e:
    logger.error("LLM request failed: %s", e)
    raise TimeoutError(f"LLM call timed out: {e}") from e
except json.JSONDecodeError as e:
    logger.error("LLM response malformed: %s", e)
    raise ValidationError(f"Invalid LLM response: {e}") from e
```

**Apply to files**:
- llm/adapter.py (11 handlers)
- api/server.py (5+ handlers)

### Pattern 4: Event Bus Operations

**Context**: emit(), subscribe(), handler execution

**BEFORE**:
```python
try:
    await bus.emit(event)
except Exception as e:
    logger.error("Event failed: %s", e)
    pass
```

**AFTER**:
```python
try:
    await bus.emit(event)
except asyncio.CancelledError:
    raise  # Don't suppress cancellation
except asyncio.TimeoutError as e:
    logger.error("Event emission timed out: %s", e)
    raise TimeoutError("Bus emit timeout") from e
except Exception as e:
    logger.error("Event emission failed: %s", e)
    raise EventBusError(f"Bus operation failed: {e}") from e
```

**Apply to files**:
- api/routers/nervous.py (10 handlers)
- cognition/cortex/orchestrator.py (8 handlers)

### Pattern 5: Network/Deployment Operations

**Context**: Docker calls, WebSocket, network connections

**BEFORE**:
```python
try:
    await docker.container_run(...)
except Exception as e:
    logger.error("Docker error: %s", e)
    return False
```

**AFTER**:
```python
try:
    await docker.container_run(...)
except docker.errors.APIError as e:
    logger.error("Docker API error: %s", e)
    raise PersistenceError(f"Docker failed: {e}") from e
except asyncio.TimeoutError as e:
    logger.error("Docker operation timed out: %s", e)
    raise TimeoutError(f"Docker timeout: {e}") from e
```

**Apply to files**:
- deployment/docker_manager.py (9 handlers)
- api/routers/ws.py (3 handlers)

---

## TOP 15 FILES TO FIX (Priority Order)

| File | Handlers | Pattern | Effort |
|------|----------|---------|--------|
| 1. nervous.py | 25 | Mixed (DB, events, network) | 4 hours |
| 2. server.py | 12 | LLM, events, startup | 3 hours |
| 3. core.py | 12 | DB, persistence, validation | 3 hours |
| 4. escore.py | 11 | JSON parsing, config | 2.5 hours |
| 5. ws.py | 11 | JSON, network, WebSocket | 2.5 hours |
| 6. adapter.py | 11 | LLM, HTTP, timeouts | 3 hours |
| 7. tui_dashboard.py | 10 | UI rendering, events | 2 hours |
| 8. tui/app.py | 10 | UI, rendering, handlers | 2 hours |
| 9. axiom.py | 9 | Event handlers, validation | 2 hours |
| 10. orchestrator.py | 9 | Event, consensus, cycles | 2.5 hours |
| 11. docker_manager.py | 9 | Deployment, network, API | 2.5 hours |
| 12. mcp.py | 8 | API calls, JSON, validation | 2 hours |
| 13. mirror.py | 8 | State, consensus, events | 2 hours |
| 14. intelligence.py | 7 | LLM, inference, parsing | 2 hours |
| 15. sdk.py | 7 | Persistence, sessions, API | 2 hours |
| | **155** | | **37 hours** |

Remaining 115 files: ~30-35 handlers each × systematic pattern application = 20 hours

**Total effort**: ~60 hours to fix all 374 handlers

---

## Automated Fixer (Optional Tool)

Create `scripts/fix_exceptions.py`:

```python
#!/usr/bin/env python3
"""
Semi-automated exception handler fixer.
Scans files, suggests fixes, applies patterns.
"""

import re
import sys
from pathlib import Path

PATTERNS = {
    "database": {
        "regex": r"await db\.|await.*query|asyncpg\.",
        "except": "asyncpg.Error",
        "raise": "PersistenceError",
    },
    "json": {
        "regex": r"json\.loads|json\.dumps|parse",
        "except": "(json.JSONDecodeError, ValueError)",
        "raise": "ValidationError",
    },
    "llm": {
        "regex": r"await llm|await.*complete|ollama",
        "except": "(httpx.RequestError, TimeoutError)",
        "raise": "TimeoutError",
    },
    # ... more patterns
}

def analyze_file(path):
    with open(path, 'r') as f:
        content = f.read()

    # Find all except Exception clauses
    matches = re.finditer(r'except Exception[^:]*:', content)

    for match in matches:
        # Look back to find the try block
        start = max(0, match.start() - 500)
        context = content[start:match.end() + 100]

        # Identify pattern
        pattern = identify_pattern(context)

        print(f"File: {path}")
        print(f"Pattern: {pattern}")
        print(f"Context: {context[:100]}...")
        print()

def identify_pattern(context):
    for name, info in PATTERNS.items():
        if re.search(info["regex"], context):
            return name
    return "unknown"

if __name__ == "__main__":
    for py_file in Path("cynic/cynic").rglob("*.py"):
        analyze_file(py_file)
```

---

## Execution Checklist

### Phase 1: Fix Top 5 Files (TODAY)
- [ ] nervous.py — 25 handlers (4 hours)
- [ ] server.py — 12 handlers (3 hours)
- [ ] core.py — 12 handlers (3 hours)
- [ ] escore.py — 11 handlers (2.5 hours)
- [ ] ws.py — 11 handlers (2.5 hours)
- **Subtotal: 70 handlers (15 hours)**

### Phase 2: Fix Next 10 Files (FOLLOWING DAYS)
- [ ] adapter.py — 11 handlers
- [ ] tui_dashboard.py — 10 handlers
- [ ] tui/app.py — 10 handlers
- [ ] axiom.py — 9 handlers
- [ ] orchestrator.py — 9 handlers
- [ ] docker_manager.py — 9 handlers
- [ ] mcp.py — 8 handlers
- [ ] mirror.py — 8 handlers
- [ ] intelligence.py — 7 handlers
- [ ] sdk.py — 7 handlers
- **Subtotal: 88 handlers (20 hours)**

### Phase 3: Bulk Fix Remaining 115 Files
- [ ] Apply pattern-based fixes
- [ ] Run linter to verify
- [ ] Test coverage maintained
- **Subtotal: 216 handlers (20 hours)**

### Phase 4: Validation
- [ ] All 374 handlers fixed
- [ ] No bare `except Exception` or `except:` in codebase
- [ ] All tests pass
- [ ] Deploy to production

---

## Success Criteria

✅ **COMPLETE** when:
- `grep -r "except Exception" cynic/cynic --include="*.py"` returns 0 matches
- `grep -r "except:" cynic/cynic --include="*.py" | grep -v "asyncio.CancelledError"` returns 0 matches
- All 374 handlers replaced with specific exception types
- Production tests pass

---

**Ralph status**: Blocker 1 strategy documented. Ready for execution.

