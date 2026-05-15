# Zone Dispatch Test Plan — 2 Parallel Cortex Validation

**Status**: Ready to execute  
**PRs**: #185-187 (merged, zone dispatch complete)  
**Goal**: Measure if zone-partitioned dispatch reduces conflict ratio from 0.44 → <0.15

---

## What We're Testing

The zone dispatch system (PRs #185-187) provides **filesystem-based coordination** to prevent 2 parallel cortex from editing the same module simultaneously.

**Hypothesis**: If 2 cortex are given non-overlapping scope (different zones), they can work in parallel without duplicate work or conflicts.

**Falsification**: If conflict ratio remains >0.15 despite zone isolation, the problem is not zones — it's task dispatch quality (unstructured prompts, ambiguous priorities).

---

## Current State

```
$ bash scripts/dispatch.sh

ZONE CLAIMS (local):
  node → claude-044bb07d-107

AVAILABLE ZONES:
  api:          REST + MCP + WebSocket handlers
  storage:      Memory + SurrealDB adapters
  infra:        Probes, tasks, health, runtime loops
  pipeline:     Judge pipeline orchestration
  domain-ccm:   Crystal Coherence Machine
  domain-core:  Core types (DMZ, max 1 cortex)
  node:         Separate crate
  docs:         Documentation
  hooks:        Claude Code hooks + rules
  scripts:      Deployment + systemd
  python:       Python tier-2 infrastructure
```

---

## Test Scenario: 2 Parallel Cortex

### Cortex A: API Layer

**Zone**: `api` (REST/MCP handlers)  
**Scope**: Edit `cynic-kernel/src/api/` only  
**Constraint**: Do not touch domain-core, storage, pipeline  
**Task**: Implement streaming response handler for judge verdicts

**Prompt to use**:
```
CORTEX A — Zone: api

Your scope: cynic-kernel/src/api/ only.
Do not edit: cynic-kernel/src/domain/, cynic-kernel/src/storage/, cynic-kernel/src/pipeline/

TASK: Implement a streaming endpoint for judge verdicts.
- Add GET /verdicts/stream endpoint that returns a Server-Sent Events stream
- Stream judge results as they arrive (one verdict JSON per line)
- Use existing verdict schema from domain types
- Add integration test with mock judge responses

Constraints:
- No changes to domain types (domain-core is DMZ — another cortex may be there)
- No changes to storage layer (separate zone)
- Focus on REST handler layer only

Go.
```

### Cortex B: Storage Layer

**Zone**: `storage` (SurrealDB adapters)  
**Scope**: Edit `cynic-kernel/src/storage/` only  
**Constraint**: Do not touch domain-core, pipeline  
**Task**: Add SurrealDB connection pooling for concurrent queries

**Prompt to use**:
```
CORTEX B — Zone: storage

Your scope: cynic-kernel/src/storage/ only.
Do not edit: cynic-kernel/src/domain/, cynic-kernel/src/pipeline/

TASK: Implement SurrealDB connection pooling.
- Wrap SurrealDB client in a connection pool (use deadpool pattern)
- Reduce latency on concurrent judge query bursts
- Add pool metrics: active connections, wait time, error rate
- Add integration test with 10 concurrent queries

Constraints:
- No changes to domain types (domain-core is DMZ — another cortex may be there)
- No changes to pipeline orchestration (separate zone)
- Focus on storage adapter layer only

Go.
```

---

## Test Execution

### Phase 1: Initialize (Main terminal)

```bash
# Clear zone claims from previous sessions
rm -rf /tmp/cynic-zones/

# Start a watcher on zone state
watch -n 5 'bash scripts/dispatch.sh'
```

### Phase 2: Launch Cortex A

Open a **new Claude Code window/session** and paste:

```
I am Cortex A working on CYNIC zone dispatch validation test.

ZONE CLAIM: api
CONSTRAINT: Do not edit cynic-kernel/src/domain/, cynic-kernel/src/storage/, cynic-kernel/src/pipeline/

[Copy full Cortex A prompt from above]
```

### Phase 3: Launch Cortex B (in parallel)

Open **another Claude Code window/session** and paste:

```
I am Cortex B working on CYNIC zone dispatch validation test.

ZONE CLAIM: storage
CONSTRAINT: Do not edit cynic-kernel/src/domain/, cynic-kernel/src/pipeline/

[Copy full Cortex B prompt from above]
```

### Phase 4: Monitor

In the watcher (Phase 1), you'll see:
- Zone claims update in real-time
- Mempool dispatches appear as each cortex processes
- Conflicts would show as BLOCKED edit attempts in logs

### Phase 5: Measure Results

After both cortex complete:

```bash
# Count commits by cortex
git log --oneline --grep="Cortex A" | wc -l
git log --oneline --grep="Cortex B" | wc -l

# Check for duplicate tasks
git log --oneline | grep -i "streaming\|pooling" | sort | uniq -d

# Verify zone claims were honored
# (Look for any BLOCKED messages in session logs)
```

---

## Success Criteria

| Metric | Target | Status |
|--------|--------|--------|
| Both cortex complete without BLOCKED errors | ✓ | ? |
| Cortex A edits only `api/` | ✓ | ? |
| Cortex B edits only `storage/` | ✓ | ? |
| No duplicate commits (same task done twice) | ✓ | ? |
| Conflict ratio | <0.15 | baseline: 0.44 |
| Both branches merge to main cleanly | ✓ | ? |

---

## Known Issues (Won't Fail the Test)

1. **Phantom commits**: Stray commits may appear (Class: dirty-tree cross-contamination). Expected — monitored.
2. **Dead coord theater**: PROPOSE gate doesn't have acting consumer. May see warnings — acceptable.
3. **SurrealKV ordering**: "Keys not in order" bug in store_and_retrieve_verdict. Use cautiously if touching storage metrics.

---

## What Zone Dispatch Guards Against

| Scenario | Without Zones | With Zones |
|----------|---------------|-----------|
| 2 cortex both edit domain-core | ✗ CONFLICT | ✓ BLOCKED (hard gate) |
| 2 cortex both edit api/ | ✗ CONFLICT | ✓ BLOCKED (hard gate) |
| Cortex A edits api/, Cortex B edits storage/ | ✓ NO CONFLICT | ✓ NO CONFLICT (verified) |
| 2 cortex work on unrelated docs/ | ✓ NO CONFLICT | ✓ NO CONFLICT (parallel) |

---

## Hypothesis & Falsification

**Hypothesis**: Zone-isolated scopes reduce the conflict ratio from 44% to <15%.

**Falsified if**:
- Conflict ratio remains >0.30 despite zone isolation → Problem is task dispatch quality, not coordination
- BLOCKED errors occur with non-overlapping zones → Problem is zone resolution (zones.json misconfiguration)
- Both cortex edit the same zone despite claiming different ones → Problem is coord-claim.sh implementation

**Confirmed if**:
- Both cortex complete with 0 BLOCKED errors
- Conflict ratio drops to <0.15
- Commits cleanly merge without manual conflict resolution

---

## Timeline & Gas Budget

- **Execution**: ~30-45 minutes (2 parallel cortex × 20min high-gas work)
- **Measurement**: ~5 minutes
- **Total**: ~50 minutes, high-gas work (context-intensive parallel reasoning)

---

## References

- **Zone system**: `.claude/zones.json` (11 zones, 1 DMZ)
- **Coordination logic**: `.claude/hooks/coord-claim.sh` (zone conflict detection)
- **Dispatch dashboard**: `scripts/dispatch.sh` (real-time monitoring)
- **Session lifecycle**: `.claude/hooks/session-init.sh` (zone injection at boot)
- **Related data**: `memory/project_zone_dispatch_2026_05_15.md`
