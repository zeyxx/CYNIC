# Zone Dispatch Test Plan — Parallel Cortex Validation

**Hypothesis**: Zone-level isolation (PRs #185-187) reduces multi-cortex conflict ratio from 0.44 → <0.15.

**Baseline data** (2026-05-14, unpartitioned dispatch):
- Sequential cortex: +537 net lines/PR
- Parallel cortex (5 sessions): +177 net lines/PR (3x degradation)
- Conflict ratio: 0.44 (duplicate work, blocking, zone confusion)

**Test goal**: Run 2 cortex in parallel with zone-partitioned dispatch. Measure conflict ratio and branch/commit overlap.

---

## Test Setup

### Prerequisites
- Zone system live (PRs #185-187 merged to main)
- Current branch: `main` (pull latest)
- Both cortex can read/write to `/tmp/cynic-zones/` (filesystem locks)
- Access to `~/.cynic-env` (CYNIC_REST_ADDR, CYNIC_API_KEY)

### Zones
Zone map is in `.cortex/zones.json`. Key zones for this test:
- **api** (API layer): REST handlers, http routing, request/response contracts
- **storage** (Storage layer): SurrealDB adapters, storage ports, migrations
- **domain** (Domain core): Types, traits, pure logic — max_cortex=1 (no parallelism allowed)

Cortex A and B will claim **api** and **storage** zones respectively. Both will avoid **domain**.

---

## Execution Plan

### Phase 1: Initialize (Human)

1. **Pull main and check zone system**:
   ```bash
   git fetch origin && git checkout main && git pull origin main
   cat .cortex/zones.json | jq '.zones | map(.name)' | head -15
   ```

2. **Verify zone locks are clean**:
   ```bash
   ls -la /tmp/cynic-zones/ 2>/dev/null || echo "No stale locks"
   ```

3. **Note baseline state**:
   - Current commit hash: `git rev-parse HEAD`
   - Open PRs: `gh pr list --state open | wc -l`
   - Branch count: `git branch -r | wc -l`

### Phase 2: Launch Cortex A (Human opens 1st Claude Code session)

**Prompt for Cortex A** (copy verbatim):

```
You are Cortex A. Zone: api. Do not edit domain/* or storage/* files.

Goal: Implement zone-aware request validation middleware for the kernel API.

Context:
- Zone-dispatch system merged (PRs #185-187)
- Need to validate incoming requests carry required X-Zone-Identity header
- Middleware should parse header, log zone origin, pass to context
- Files to create/edit: cynic-kernel/src/api/middleware.rs (new), cynic-kernel/src/api/mod.rs (wire)

Task (30 min):
1. Create api/middleware.rs with ZoneValidationMiddleware
2. Wire into API layer routes (POST /agent-dispatch, GET /health)
3. Add test: verify 401 if X-Zone-Identity missing
4. Commit with message: "feat(api): Add zone-aware request validation middleware"
5. Create PR, title same as commit message
6. Post completion note to #cynic: "Cortex A: api middleware complete"

Constraint: Do NOT touch domain/* or storage/* files. If you need types, import from existing domain crates.
```

### Phase 3: Launch Cortex B (Human opens 2nd Claude Code session, parallel with A)

**Prompt for Cortex B** (copy verbatim):

```
You are Cortex B. Zone: storage. Do not edit domain/* or api/* files.

Goal: Add zone audit logging to storage layer for K15 compliance.

Context:
- Zone-dispatch system merged (PRs #185-187)
- Need to log all storage operations with zone origin (from request context)
- Audit trail: (timestamp, operation, zone_claimed_by, success/failure)
- Files to create/edit: cynic-kernel/src/storage/audit.rs (new), cynic-kernel/src/storage/mod.rs (wire)

Task (30 min):
1. Create storage/audit.rs with AuditLog struct and write_audit() function
2. Add LoggingStorageAdapter wrapper that audits all operations
3. Wire into storage::surreal::SurrealHttpStorage via decorator pattern
4. Add test: verify audit log captures zone origin on store_dispatch
5. Commit with message: "feat(storage): Add zone audit logging for K15 compliance"
6. Create PR, title same as commit message
7. Post completion note to #cynic: "Cortex B: storage audit complete"

Constraint: Do NOT touch domain/* or api/* files. If you need types, import from existing storage crates.
```

### Phase 4: Monitor Execution (Human)

While both cortex run in parallel (in separate sessions):

1. **Watch for zone conflicts**:
   ```bash
   watch -n 2 'curl -s http://${CYNIC_REST_ADDR}/agent-dispatch | jq ".[] | select(.status==\"BLOCKED\")" 2>/dev/null || echo "No blocks"'
   ```

2. **Check branch creation**:
   ```bash
   git fetch origin && git branch -a | grep -E "feat/(api|audit)" | wc -l
   ```

3. **Measure elapsed time**:
   - Note wall-clock time from Phase 2 start to Phase 3 completion
   - Expected: <60 min total for both cortex

### Phase 5: Measure Results (Human after both complete)

1. **Conflict ratio**:
   ```bash
   # Branch overlap: do both touch same files?
   git diff <cortex-a-branch> <cortex-b-branch> --name-only | wc -l
   # Target: 0 (no overlapping files)
   ```

2. **Commit messages**:
   ```bash
   git log <cortex-a-branch> --oneline | head -5
   git log <cortex-b-branch> --oneline | head -5
   # Target: distinct commits, no merges needed
   ```

3. **PR merge friction**:
   ```bash
   gh pr view <cortex-a-pr> --json statusCheckRollup
   gh pr view <cortex-b-pr> --json statusCheckRollup
   # Target: both PRs pass CI without conflicts
   ```

4. **Zone claim audit**:
   ```bash
   ls -la /tmp/cynic-zones/
   # Expected: 2 lock files (api.lock, storage.lock) — no contention
   ```

---

## Success Criteria

| Metric | Target | Baseline | Falsification |
|--------|--------|----------|--------------|
| **Conflict ratio** (file overlap) | 0 | 0.44 | >0.15 → zones not isolated |
| **Zone blocks (401s)** | 0 | N/A | >0 → zone gate broken |
| **CI passes** | Both PRs merge | N/A | Either fails → regression |
| **Elapsed time** | <60 min | N/A | >90 min → zone overhead too high |
| **Lock cleanup** | 0 stale locks after test | N/A | >0 → zone cleanup broken |

---

## Hypothesis Validation

**If all criteria pass**:
- ✅ Zone isolation **works** — partition multiple cortex without blocking
- ✅ **Parallel efficiency recovered** — from 0.44 conflict ratio to near-zero
- ✅ Ready for production multi-cortex deployment

**If any metric fails**:
- 🔄 Investigate: which zone boundary broke? (grep `coord-claim.sh` output)
- 🔄 Fallback: sequential cortex until root cause identified
- 🔄 Post analysis to #cynic with evidence

---

## Falsification Conditions

State what would prove the hypothesis **wrong**:

1. **Conflict ratio remains >0.15 after running test** → zones not providing isolation
2. **Zone blocks (exit 2 from coord-claim.sh) occur during parallel execution** → zone gate is too strict or incorrectly configured
3. **Both cortex touch the same file (e.g., domain/mod.rs)** → zone boundaries not enforced
4. **Merged PRs require manual conflict resolution** → zone model incomplete
5. **Test takes >90 minutes for both cortex** → zone overhead unacceptable

If ANY of these occur, zone-dispatch is not production-ready. Report findings to T. with git log/diffs.

---

## Reference

- Zone system: `.cortex/zones.json`
- Implementation: `cynic-kernel/src/api/middleware.rs` (to create)
- Audit: `cynic-kernel/src/storage/audit.rs` (to create)
- Coordination: `~/.cortex/mcp/coord-claim.sh` (enforcement)
- Baseline data: `memory/project_zone_dispatch_2026_05_15.md`
