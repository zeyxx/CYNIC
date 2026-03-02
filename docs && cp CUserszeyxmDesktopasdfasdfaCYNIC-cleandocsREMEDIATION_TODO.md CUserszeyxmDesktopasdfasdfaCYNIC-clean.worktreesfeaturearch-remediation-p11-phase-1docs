# CYNIC Architectural Rules — Enforcement Guidelines

These rules prevent regressions identified during Phase 1-2B remediation (March 2, 2026).

## Rule 1: No `asyncio.run()` in Async Contexts

**Violation:** Using `asyncio.run()` inside an `async` function or method.

**Why it's bad:**
- RuntimeError at runtime ("This event loop is already running")
- Crashes under real load (10k+ TPS)
- Blocks graceful shutdown

**Correct pattern:**
```python
# WRONG
async def metrics(request):
    data = asyncio.run(collector.fetch())  # RuntimeError!

# RIGHT
async def metrics(request):
    data = await collector.fetch()  # Correct
```

**Scope:** ALL code, including CLI tools and examples
**Enforcement:** Pre-commit hook (grep for pattern)

---

## Rule 2: No Bare `except:` Blocks

**Violation:** Using bare `except:` without specifying exception type.

**Why it's bad:**
- Swallows `asyncio.CancelledError` → breaks graceful shutdown
- Hides bugs (KeyboardInterrupt also caught)
- Silent failures with no observability

**Correct pattern:**
```python
# WRONG
try:
    await something()
except:  # Catches everything including CancelledError
    pass

# RIGHT
try:
    await something()
except asyncio.CancelledError:
    raise  # Let the framework handle shutdown
except (OSError, ValueError) as e:
    logger.error("Specific error: %s", e)
```

**Scope:** All async code, especially handlers
**Enforcement:** Pre-commit hook (grep for `except\s*:`)

---

## Rule 3: No Direct `os.getenv()` Outside Config System

**Violation:** Calling `os.getenv()` in modules other than `cynic/kernel/core/config.py`.

**Why it's bad:**
- Scattered configuration sources
- Hard to audit for security (credentials buried in random files)
- Inconsistent defaults across code
- Difficult to validate credentials centrally

**Correct pattern:**
```python
# WRONG (llm_bridge.py)
import os
api_key = os.getenv("ANTHROPIC_API_KEY")

# RIGHT
from cynic.kernel.core.config import CynicConfig
config = CynicConfig.from_env()
api_key = config.anthropic_api_key
```

**Scope:** All production code except config.py itself
**Enforcement:** Pre-commit hook (grep with exclusion list)

---

## Rule 4: Event Handlers Must Be Unregistered

**Violation:** Registering event listeners without a corresponding `unregister()` or `stop()` method.

**Why it's bad:**
- Memory leaks in long-running processes
- Duplicate handlers on reinitialization
- Accumulating event queue pressure

**Correct pattern:**
```python
# WRONG
class MyListener:
    def __init__(self, bus):
        bus.on("event", self.handle)  # No way to unregister!

# RIGHT
class MyListener:
    def __init__(self, bus):
        self.bus = bus
        self.bus.on("event", self.handle)

    def stop(self):
        self.bus.off("event", self.handle)  # Can unregister
```

**Scope:** All event-based components
**Enforcement:** Code review checklist + test coverage

---

## Rule 5: Global State Must Have Locks

**Violation:** Mutable global variables without synchronization primitives.

**Why it's bad:**
- Race conditions at scale (10k TPS)
- Non-deterministic failures
- Double-initialization of singletons

**Correct pattern:**
```python
# WRONG
_cache = {}
def get_cached(key):
    if key not in _cache:
        _cache[key] = expensive_operation()
    return _cache[key]

# RIGHT
import threading
_cache = {}
_cache_lock = threading.Lock()

def get_cached(key):
    with _cache_lock:
        if key not in _cache:
            _cache[key] = expensive_operation()
        return _cache[key]
```

**Scope:** All module-level and class-level mutable state
**Enforcement:** Code review + architectural audit

---

## Rule 6: Singletons Must Be Thread-Safe

**Violation:** Lazy initialization pattern without double-check locking.

**Why it's bad:**
- Two threads can both pass the `None` check
- Creates duplicate instances
- State inconsistency

**Correct pattern:**
```python
# WRONG (race condition possible)
_engine = None
def get_engine():
    global _engine
    if _engine is None:
        _engine = Engine()
    return _engine

# RIGHT
import threading
_engine = None
_engine_lock = threading.Lock()

def get_engine():
    global _engine
    if _engine is None:
        with _engine_lock:
            if _engine is None:
                _engine = Engine()
    return _engine
```

**Scope:** All singleton factories
**Enforcement:** Code review + concurrency testing

---

## Rule 7: Production Credentials Cannot Be Defaults

**Violation:** Hardcoded default values for sensitive config (passwords, API keys).

**Why it's bad:**
- Credentials leak in version control
- Accidentally deployed to production
- No way to distinguish development from production

**Correct pattern:**
```python
# WRONG
surreal_password: str = "local_dev_only"  # Will reach production!

# RIGHT
surreal_password: str = ""  # Empty default
# Then validate:
if not config.surreal_password and is_production:
    raise RuntimeError("SURREAL_PASS required in production")
```

**Scope:** All config classes
**Enforcement:** Pre-commit hook (grep for `= "root"` or `= "local"`)

---

## Rule 8: Async Resources Must Snapshot Collections

**Violation:** Iterating over a mutable collection while other code mutates it.

**Why it's bad:**
- RuntimeError: "Set changed during iteration"
- Non-deterministic failures
- Hard to debug race conditions

**Correct pattern:**
```python
# WRONG
async def drain(self, timeout):
    await asyncio.gather(*self._pending_tasks)  # Set might change!

# RIGHT
async def drain(self, timeout):
    with self._lock:
        tasks = list(self._pending_tasks)  # Snapshot under lock
    await asyncio.gather(*tasks)  # Iterate over snapshot
```

**Scope:** All async methods touching shared collections
**Enforcement:** Code review + integration testing

---

## Enforcement Checklist for Code Review

Before merging any PR:

- [ ] No `asyncio.run()` in async code
- [ ] No bare `except:` blocks
- [ ] No `os.getenv()` outside config.py
- [ ] All event listeners have `stop()` methods
- [ ] Global mutable state has locks
- [ ] Singletons use double-check locking
- [ ] No hardcoded credentials in defaults
- [ ] Collections snapshotted before async iteration
- [ ] All changes tested at 10k TPS if applicable
- [ ] Commit message includes Tech Debt impact

---

## Pre-Commit Hook Commands

Add to `.git/hooks/pre-commit`:

```bash
# Rule 1: asyncio.run in async
grep -r "asyncio.run(" cynic/ | grep -v "test" | grep -v ".pyc" && \
  echo "ERROR: asyncio.run() found in production code" && exit 1

# Rule 2: bare except
grep -rE "except\s*:" cynic/ | grep -v "except.*Error" | grep -v "test" && \
  echo "ERROR: bare except: found" && exit 1

# Rule 3: os.getenv outside config
grep -r "os.getenv" cynic/ | grep -v "config.py" | grep -v "test" && \
  echo "ERROR: os.getenv() outside config.py" && exit 1

# Rule 7: hardcoded credentials
grep -r "password.*=" cynic/ | grep -E '"(root|local_dev|cynic)"' && \
  echo "ERROR: Hardcoded credential found" && exit 1
```

---

## Summary

These 8 rules + 4 enforcement mechanisms prevent:
- ✅ Runtime crashes (asyncio.run in async)
- ✅ Graceful shutdown breakage (bare except)
- ✅ Security leaks (scattered credentials)
- ✅ Memory leaks (unregistered listeners)
- ✅ Race conditions (unprotected globals, no snapshots)
- ✅ Configuration confusion (scattered os.getenv)

**Last Updated:** 2026-03-02 (Phase 1-2B Remediation)
