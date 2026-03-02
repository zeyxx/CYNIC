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

---

## Rule 2: No Bare `except:` Blocks

**Violation:** Using bare `except:` without specifying exception type.

**Why it's bad:**
- Swallows `asyncio.CancelledError` → breaks graceful shutdown
- Hides bugs
- Silent failures

**Correct pattern:**
```python
# WRONG
except:
    pass

# RIGHT
except asyncio.CancelledError:
    raise
except (OSError, ValueError) as e:
    logger.error("Specific error: %s", e)
```

---

## Rule 3: No Direct `os.getenv()` Outside Config System

**Violation:** Calling `os.getenv()` outside `cynic/kernel/core/config.py`.

**Why it's bad:**
- Scattered configuration sources
- Credentials buried in random files
- Inconsistent defaults across code

**Correct pattern:**
```python
# WRONG
api_key = os.getenv("ANTHROPIC_API_KEY")

# RIGHT
from cynic.kernel.core.config import CynicConfig
config = CynicConfig.from_env()
api_key = config.anthropic_api_key
```

---

## Rule 4: Event Handlers Must Be Unregistered

**Violation:** Registering listeners without unregister capability.

**Why it's bad:**
- Memory leaks in long-running processes
- Duplicate handlers on reinitialization

**Correct pattern:**
```python
class MyListener:
    def __init__(self, bus):
        self.bus = bus
        self.bus.on("event", self.handle)

    def stop(self):
        self.bus.off("event", self.handle)
```

---

## Rule 5: Global State Must Have Locks

**Violation:** Mutable global variables without synchronization.

**Why it's bad:**
- Race conditions at scale (10k TPS)
- Non-deterministic failures

**Correct pattern:**
```python
import threading
_cache = {}
_cache_lock = threading.Lock()

def get_cached(key):
    with _cache_lock:
        if key not in _cache:
            _cache[key] = expensive_operation()
        return _cache[key]
```

---

## Rule 6: Singletons Must Be Thread-Safe

**Violation:** Lazy initialization without double-check locking.

**Why it's bad:**
- Two threads can both pass the `None` check
- Creates duplicate instances

**Correct pattern:**
```python
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

---

## Rule 7: No Hardcoded Credential Defaults

**Violation:** Default values for sensitive config (passwords, API keys).

**Why it's bad:**
- Credentials leak in git
- Accidentally deployed to production

**Correct pattern:**
```python
# WRONG
surreal_password: str = "local_dev_only"

# RIGHT
surreal_password: str = ""
# Then validate in production
```

---

## Rule 8: Async Resources Must Snapshot Collections

**Violation:** Iterating over mutable collections in async code.

**Why it's bad:**
- RuntimeError: "Set changed during iteration"
- Non-deterministic failures

**Correct pattern:**
```python
# WRONG
await asyncio.gather(*self._pending_tasks)  # Set might change!

# RIGHT
with self._lock:
    tasks = list(self._pending_tasks)  # Snapshot
await asyncio.gather(*tasks)
```

---

**Enforcement:** Pre-commit hooks + code review checklist
**Last Updated:** 2026-03-02
