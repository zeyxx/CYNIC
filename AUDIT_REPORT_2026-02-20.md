# PRODUCTION AUDIT REPORT — Phase 1 Modules
**Date**: 2026-02-20
**Auditor**: CYNIC Senior Dev Analysis
**Status**: 🔴 BLOCKING — 10 Critical Issues Found

---

## ISSUE TRACKER

### CRITICAL ISSUES (Must Fix)

| ID | Severity | File | Line | Issue | Status |
|----|----------|------|------|-------|--------|
| C1 | 🔴 CRITICAL | environment.py | 238 | Blocking call: psutil.cpu_percent(interval=0.5) in async | PENDING |
| C2 | 🔴 CRITICAL | environment.py | 190-197 | Hardcoded DB credentials (cynic/cynic_dev) | PENDING |
| C3 | 🔴 CRITICAL | config_adapter.py | 63 | Blocking input() in async function | PENDING |
| H1 | ⚠️ HIGH | environment.py | 119-120 | Silent exception on disk_usage | PENDING |
| H2 | ⚠️ HIGH | environment.py | 224 | Blocking urllib.request in async | PENDING |
| H3 | ⚠️ HIGH | config_adapter.py | 115 | JSON corruption on concurrent writes | PENDING |
| H4 | ⚠️ HIGH | environment.py | 126-128 | rglob could hang on large directories | PENDING |
| M1 | 🟡 MEDIUM | config_adapter.py | 74 | Unicode emoji crash on Windows cp1252 | PENDING |
| M2 | 🟡 MEDIUM | environment.py | 135 | Division by zero risk | PENDING |
| M3 | 🟡 MEDIUM | environment.py | 245-249 | Permission denied on CYNIC_HOME silent failure | PENDING |

---

## FIXES TO IMPLEMENT

### C1: Blocking psutil.cpu_percent() in Async

**File**: `perceive/environment.py:238`

**Current**:
```python
cpu_percent=psutil.cpu_percent(interval=0.5),
```

**Problem**: Blocks entire async event loop for 0.5 seconds

**Fix**:
```python
# Use instant CPU reading (no interval = samples once)
cpu_percent=psutil.cpu_percent(interval=None),
```

---

### C2: Hardcoded Database Credentials

**File**: `perceive/environment.py:190-197`

**Current**:
```python
conn = await asyncpg.connect(
    user="cynic",
    password="cynic_dev",
    database="cynic_py",
    host="localhost",
    port=5433,
)
```

**Problem**: Credentials in source code, visible in logs/stack traces

**Fix**:
```python
user = os.getenv("POSTGRES_USER", "cynic")
password = os.getenv("POSTGRES_PASSWORD", None)
database = os.getenv("POSTGRES_DB", "cynic_py")
host = os.getenv("POSTGRES_HOST", "localhost")
port = int(os.getenv("POSTGRES_PORT", "5433"))

if not password:
    postgres_error = "POSTGRES_PASSWORD env var not set (required for security)"
    return StorageStatus(
        postgres_available=False,
        postgres_error=postgres_error,
        surrealdb_available=False,
        surrealdb_error=None,
        state_files_count=0,
    )

try:
    conn = await asyncpg.connect(
        user=user,
        password=password,
        database=database,
        host=host,
        port=port,
        timeout=2,
    )
    await conn.close()
    postgres_available = True
except Exception as e:
    postgres_error = str(e)
```

---

### C3: Blocking input() in Async

**File**: `config_adapter.py:63`

**Current**:
```python
try:
    user_input = input("> ").strip()
except (EOFError, KeyboardInterrupt):
    user_input = ""
```

**Problem**: Blocks async event loop while waiting for user input

**Fix**:
```python
import asyncio

try:
    # Run blocking I/O in executor (thread pool)
    loop = asyncio.get_event_loop()
    user_input = await loop.run_in_executor(
        None,
        input,  # function
        "> "    # argument
    )
    user_input = user_input.strip()
except (EOFError, KeyboardInterrupt):
    user_input = ""
```

---

### H1: Silent Exception on disk_usage

**File**: `perceive/environment.py:119-120`

**Current**:
```python
try:
    c_stat = shutil.disk_usage("C:/")
    d_stat = shutil.disk_usage("D:/")
except Exception:
    c_stat = d_stat = None  # Silently fails
```

**Problem**: Returns 0 for everything, hides actual error

**Fix**:
```python
import logging

logger = logging.getLogger(__name__)

c_stat = None
d_stat = None

try:
    c_stat = shutil.disk_usage("C:/")
except (OSError, ValueError) as e:
    logger.warning(f"Cannot read C: drive: {e}")

try:
    d_stat = shutil.disk_usage("D:/")
except (OSError, ValueError) as e:
    logger.warning(f"Cannot read D: drive: {e}")
```

---

### H2: Blocking urllib.request in Async

**File**: `perceive/environment.py:224`

**Current**:
```python
try:
    import urllib.request
    urllib.request.urlopen(ollama_url, timeout=2)
    ollama_available = True
except Exception as e:
    ollama_error = str(e)
```

**Problem**: Blocks event loop during HTTP request

**Fix**:
```python
import aiohttp

try:
    async with aiohttp.ClientSession() as session:
        async with session.get(
            ollama_url,
            timeout=aiohttp.ClientTimeout(total=2)
        ) as resp:
            if resp.status == 200:
                ollama_available = True
            else:
                ollama_error = f"HTTP {resp.status}"
except asyncio.TimeoutError:
    ollama_error = "Timeout (2s)"
except Exception as e:
    ollama_error = str(e)
```

---

### H3: JSON Corruption on Concurrent Writes

**File**: `config_adapter.py:115` and `perceive/environment.py:249`

**Current**:
```python
self.discoveries_log.write_text(json.dumps(log, indent=2))  # Not atomic!
```

**Problem**: Multiple async tasks can corrupt file (read-modify-write race)

**Fix**: Use JSONL for environment (already atomic append) and add locking for preferences:

```python
import asyncio
from pathlib import Path

async def _save_preference_atomic(self, key: str, value: Any) -> None:
    """Atomically save preference (safe for concurrent access)."""

    # Atomic write via temp file + rename
    self.preferences_log.parent.mkdir(parents=True, exist_ok=True)

    # Read current
    prefs = {}
    if self.preferences_log.exists():
        try:
            prefs = json.loads(self.preferences_log.read_text())
        except (json.JSONDecodeError, OSError):
            pass

    # Modify
    prefs[key] = value

    # Write to temp
    temp_file = self.preferences_log.parent / f".{self.preferences_log.name}.tmp"
    temp_file.write_text(json.dumps(prefs, indent=2))

    # Atomic rename (atomic on all platforms with pathlib)
    temp_file.replace(self.preferences_log)
```

---

### H4: rglob Could Hang on Large Directories

**File**: `perceive/environment.py:126-128`

**Current**:
```python
cynic_size_mb = sum(
    f.stat().st_size for f in cynic_home.rglob("*") if f.is_file()
) / 1024 / 1024
```

**Problem**: 7,224 files × stat() call = slow, could timeout

**Fix**:
```python
def _calculate_cynic_home_size(self, cynic_home: Path) -> float:
    """Calculate directory size with safety limits."""
    total_size = 0
    file_count = 0
    MAX_FILES = 100000  # Safety limit

    try:
        for dirpath, dirnames, filenames in os.walk(cynic_home):
            if file_count >= MAX_FILES:
                break
            for filename in filenames:
                filepath = os.path.join(dirpath, filename)
                try:
                    total_size += os.path.getsize(filepath)
                    file_count += 1
                except (OSError, FileNotFoundError):
                    pass  # Skip inaccessible files

        return total_size / 1024 / 1024
    except Exception as e:
        logger.warning(f"Error calculating CYNIC_HOME size: {e}")
        return 0.0
```

---

### M1: Unicode Emoji Crash on Windows

**File**: `config_adapter.py:74`

**Current**:
```python
print(f"✓ Adapted: {key} = {final_value}")
```

**Problem**: Windows cp1252 can't render checkmark → UnicodeEncodeError

**Fix**:
```python
import sys
import platform

def _get_ok_symbol() -> str:
    """Platform-aware OK symbol."""
    if platform.system() == "Windows":
        return "[OK]"
    return "✓"

# In code:
symbol = self._get_ok_symbol()
print(f"{symbol} Adapted: {key} = {final_value}")
```

---

### M2: Division by Zero Risk

**File**: `perceive/environment.py:135`

**Current**:
```python
c_drive_used_percent=100 - (c_stat.free / c_stat.total * 100) if c_stat else 0,
```

**Problem**: If c_stat.total == 0 → ZeroDivisionError

**Fix**:
```python
def _safe_percent(used: float, total: float) -> float:
    """Safely calculate percentage."""
    return (100 - (used / total * 100)) if total > 0 else 0.0

# In code:
c_drive_used_percent = self._safe_percent(c_stat.free, c_stat.total) if c_stat else 0.0,
```

---

### M3: Permission Denied on CYNIC_HOME

**File**: `perceive/environment.py:245-249`

**Current**:
```python
self.environment_log.parent.mkdir(parents=True, exist_ok=True)
with open(self.environment_log, "a") as f:
    f.write(json.dumps(asdict(snapshot)) + "\n")
```

**Problem**: If ~/.cynic not writable, fails silently

**Fix**:
```python
async def log_environment(self, snapshot: EnvironmentSnapshot) -> None:
    """Persist environment snapshot with validation."""
    try:
        self.environment_log.parent.mkdir(parents=True, exist_ok=True)

        # Verify write access
        self.environment_log.parent.touch(exist_ok=True)

        # Write JSONL (append-only, atomic)
        with open(self.environment_log, "a") as f:
            f.write(json.dumps(asdict(snapshot)) + "\n")
    except (OSError, PermissionError) as e:
        logger.error(f"Cannot log environment to {self.environment_log}: {e}")
        # Fail gracefully - don't crash CYNIC
```

---

## TESTING PLAN

After each fix:
1. Run syntax check: `python -m py_compile`
2. Run imports: `from cynic.perceive.environment import EnvironmentIntrospector`
3. Run 938 existing tests: `pytest tests/ -q`
4. Create specific unit test for fix

---

## SUCCESS CRITERIA

✅ All 10 issues fixed
✅ No syntax errors
✅ All 938 tests passing
✅ New unit tests for fixes (min 1 per issue)
✅ Code review: no blocking calls in async
✅ Code review: no hardcoded credentials
✅ Code review: no silent failures
✅ Windows compatibility verified

---

**Status**: 🔴 BLOCKING — Fix all before Phase 2

**Estimated Time**: 3-4 hours (fixes + tests)

**Next Step**: Execute fixes in order (C1 → C3, then H1 → M3)
