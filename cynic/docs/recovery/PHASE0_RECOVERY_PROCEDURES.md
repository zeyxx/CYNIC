# Phase 0: Data Recovery Procedures

## Overview

CYNIC persists data across three layers. On process crash or restart, data is recovered from the most recent persisted state.

---

## Recovery Layers (Priority Order)

### Layer 1: PostgreSQL (Judgments)

**Data**: Judgment records, Q-Table entries, Learning history
**Persistence**: `_persist_judgment_async()` in orchestrator (immediate on creation)
**Recovery Path**: Automatic on app restart

```python
# During startup: CynicOrganism.restore_state()
# 1. Connect to PostgreSQL
# 2. Query recent judgments (last 1000)
# 3. Reconstruct Q-Table from judgment history
# 4. ConsciousState loads from disk checkpoint (see Layer 3)
```

**Recovery Time**: <1s (depends on DB latency)
**Data Loss Window**: 0 (judgment persisted immediately after creation)

---

### Layer 2: Guidance.json (Current State Guidance)

**Data**: Latest guidance (verdict, q-score, dog votes, timestamp)
**Persistence**: `_write_guidance_async()` called by GuidanceWriterHandler (on JUDGMENT_CREATED event)
**Recovery Path**: Manual or CLI recovery

**File Location**: `~/.cynic/guidance.json`

**Recovery Steps**:
```bash
# 1. Check if file exists after crash
ls -la ~/.cynic/guidance.json

# 2. Verify JSON is valid (not corrupted)
cat ~/.cynic/guidance.json | python -m json.tool > /dev/null

# 3. If valid, guidance state is preserved
# If corrupted, use Layer 3 or Layer 1 for reconstruction
```

**Corruption Detection**:
- JSON parsing fails → File was partially written during crash
- Timestamp is very old (>1 hour) → Stale guidance

**Recovery from Corruption**:
```bash
# Delete corrupted file; next judgment will recreate it
rm ~/.cynic/guidance.json

# Or reconstruct from Layer 1:
# Run: cynic recover --from-judgment-db guidance
```

---

### Layer 3: ConsciousState Checkpoint

**Data**: ConsciousState snapshot (consciousness level, dogs, axioms, judgment count, error count)
**Persistence**: `sync_checkpoint()` called by endpoints BEFORE returning HTTP 200
**Recovery Path**: Automatic on app restart

**File Location**: `~/.cynic/conscious_state.json`

**Recovery Steps** (Automatic):
```python
# During startup: ConsciousState.load_from_disk()
# 1. Check if ~/.cynic/conscious_state.json exists
# 2. If yes, parse JSON and restore state
# 3. Consciousness level set to last-known state
# 4. Dog activity counts restored
# 5. Axiom maturity restored
```

**Recovery Time**: <100ms (local disk read)
**Data Loss Window**: ≤5s (max time between checkpoint calls)

---

## Failure Scenarios & Recovery

### Scenario 1: Process Crash During Judgment Creation

**Timeline**:
```
T0: POST /judge received
T1: Cell built
T2: Event emitted → JUDGMENT_REQUESTED
T3: Orchestrator processes event
T4: _persist_judgment_async() called
T5: PostgreSQL save completes ← CRASH HAPPENS HERE
T6: JUDGMENT_CREATED event emitted
T7: ConsciousState.sync_checkpoint() called in endpoint
T8: HTTP 200 returned
```

**Crashed at T5.5** (between DB save and event emit):

- ✅ PostgreSQL has judgment
- ❌ Guidance.json doesn't exist (event not emitted yet)
- ❌ ConsciousState not synced

**Recovery**:
1. On restart, PostgreSQL recovers the judgment
2. Next query for judgment returns result (with verdict="incomplete")
3. Next judgment triggers GuidanceWriterHandler → writes guidance.json
4. ConsciousState syncs on next endpoint call
5. **Result**: User gets result, may see "incomplete" status briefly

---

### Scenario 2: Process Crash During ConsciousState Sync

**Timeline**:
```
T0: POST /judge received
T1-T6: As above (judgment created, persisted, event processed)
T7: ConsciousState.sync_checkpoint() called
T7.5: Temp file created and written
T8: CRASH during os.replace() atomic rename
    ← CRASH HAPPENS HERE
T9: HTTP 200 returned
```

**Recovery**:
1. Temp file left behind at `~/.cynic/.conscious_state_tmp_*`
2. On restart, app ignores temp files
3. Old ConsciousState.json is still valid
4. **Result**: Loses <5ms of state; minimal impact

---

### Scenario 3: Guidance.json Corruption

**Timeline**:
```
T0: _write_guidance_async() starts
T1: Temp file created
T2: JSON written to temp
T3: CRASH during os.replace()
T4: guidance.json exists but may be partial/corrupted
```

**Detection**:
```python
# On app startup, GuidanceWriter health check:
try:
    with open("~/.cynic/guidance.json") as f:
        json.load(f)
except json.JSONDecodeError:
    logger.error("Corrupted guidance.json detected")
    # Proceed without it; will be recreated on next judgment
```

**Recovery**:
```bash
# Option A: Delete corrupted file (auto-recreate)
rm ~/.cynic/guidance.json

# Option B: Restore from backup (if exists)
cp ~/.cynic/guidance.json.backup ~/.cynic/guidance.json

# Option C: Reconstruct from PostgreSQL
cynic recover --from-judgment-db guidance
```

---

## Recovery Commands (CLI)

### Check Recovery Status

```bash
# Check all three layers
cynic status --recovery

# Output:
# PostgreSQL:  ✓ Connected, 1247 judgments
# Guidance:    ✓ Valid JSON, timestamp=2026-02-21T14:15:33Z
# State:       ✓ Valid checkpoint, consciousness_level=MACRO, dogs=11
```

### Force Recovery From PostgreSQL

```bash
# Rebuild ConsciousState from judgment history
cynic recover --from-judgment-db all

# Rebuild only guidance.json
cynic recover --from-judgment-db guidance

# Rebuild only Q-Table
cynic recover --from-judgment-db qtable
```

### Manual Backup/Restore

```bash
# Backup all three layers
cynic backup --output ~/backups/cynic-backup-$(date +%s).tar.gz

# Restore from backup
cynic restore ~/backups/cynic-backup-1708516533.tar.gz
```

---

## Monitoring for Recovery Issues

### Metrics to Watch

**From `PersistenceMetricsCollector`**:
```python
metrics = get_persistence_metrics()

# Alert if:
metrics["persist_judgment"]["error_rate"] > 0.05  # >5% failures
metrics["write_guidance"]["error_rate"] > 0.05
metrics["sync_checkpoint"]["error_rate"] > 0.05

# Alert if:
metrics["persist_judgment"]["mean_duration_ms"] > 500
metrics["write_guidance"]["mean_duration_ms"] > 100
metrics["sync_checkpoint"]["mean_duration_ms"] > 100
```

### Health Check Endpoint

```bash
# Check persistence layer health
curl http://localhost:8000/health/persistence

# Response:
{
  "status": "healthy",  # or "degraded", "warning"
  "error_rate_last_100": 0.02,
  "statistics": {
    "persist_judgment": {
      "mean_duration_ms": 45.3,
      "success_rate": 0.98,
      "failed_calls": 2
    },
    ...
  }
}
```

---

## Prevention Checklist

- ✅ All persistence functions are async and awaitable
- ✅ All endpoints call `sync_checkpoint()` before return
- ✅ Orchestrator calls `_persist_judgment_async()` immediately after creation
- ✅ GuidanceWriterHandler subscribes to JUDGMENT_CREATED events
- ✅ All writes use atomic operations (temp → rename)
- ✅ Error handling: failures raise (don't silently catch)
- ✅ Metrics tracking: all operations timed and tracked
- ✅ No fire-and-forget patterns for critical data

---

## Learning for Future

**If you add new persistence**, ensure:
1. Three-layer recovery possible
2. Metrics tracking added
3. Health check monitors layer
4. Recovery CLI command implemented
5. Documentation updated

**Pattern**:
```python
# Layer 1: Kernel persistence (immediate)
# Layer 2: API persistence (event-driven)
# Layer 3: State durability (endpoint checkpoint)
```

**Never skip**. This pattern prevents data loss and supports crash recovery.

---

**Last Updated**: 2026-02-21
**Confidence**: 61.8% (φ⁻¹)
