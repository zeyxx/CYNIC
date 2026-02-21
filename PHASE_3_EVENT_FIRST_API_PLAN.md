# Phase 3: Event-First API Refactoring

**Goal**: Convert blocking orchestrator calls → event-driven, async-first patterns

**Status**: Planning phase

---

## Current (Blocking) Pattern

```python
# OLD - blocks until judgment completes
@router_core.post("/judge")
async def judge(req: JudgeRequest, container: AppContainer = Depends(...)):
    cell = Cell(...)
    judgment = await state.orchestrator.run(cell)  # ← BLOCKS
    return JudgeResponse(...judgment...)  # Response only after judgment done
```

**Problem**:
- Response time = MCTS time (2-3s for MACRO)
- API is synchronous with organism
- Scales poorly (1 request = 1 judgment = 2-3s)

---

## New (Event-Driven) Pattern

```python
# NEW - emits event and returns immediately
@router_core.post("/judge")
async def judge(req: JudgeRequest, container: AppContainer = Depends(...)):
    cell = Cell(...)

    # 1. Create event envelope
    event = Event(
        type=CoreEvent.PERCEIVE_REQUESTED,
        payload={
            "cell": cell.to_dict(),
            "level": req.level,
            "budget_usd": req.budget_usd,
        },
        source="api:judge"
    )

    # 2. Emit to CORE bus (scheduler picks it up)
    await get_core_bus().emit(event)

    # 3. Return IMMEDIATELY with event ID
    return JudgeResponse(
        judgment_id=event.event_id,
        status="processing",  # NEW
        # ... placeholder fields ...
    )

# Optional: Separate endpoint to query result
@router_core.get("/judge/{judgment_id}")
async def get_judgment(judgment_id: str, container: AppContainer = Depends(...)):
    # Query ConsciousState for completed judgment
    judgment = await container.organism.conscious_state.get_judgment_by_id(judgment_id)
    if judgment is None:
        return {"status": "processing"}  # Still pending
    return JudgeResponse(...judgment...)
```

**Benefit**:
- Response time = event emission time (~1ms)
- API is decoupled from organism
- Scales to 1000s RPS (event bus is fast)

---

## Endpoints to Convert (Priority Order)

### Tier 1 (Core Judgment Loop)
1. **POST /judge** (line 121 in core.py)
   - Current: `await orchestrator.run()`
   - New: Emit PERCEIVE_REQUESTED event
   - Return: `{judgment_id, status: "processing"}`

2. **POST /perceive** (line 198 in core.py)
   - Current: `await orchestrator.run()`
   - New: Emit PERCEIVE_REQUESTED event (same as /judge, different analysis)
   - Return: `{event_id, status: "processing"}`

### Tier 2 (Learning Loop)
3. **POST /learn** (line 323)
   - Current: Updates QTable directly
   - New: Emit LEARNING_SIGNAL event
   - Return: `{status: "queued"}`

4. **POST /feedback** (line 360)
   - Current: Appends to social signals
   - New: Emit USER_FEEDBACK event
   - Return: `{status: "recorded"}`

### Tier 3 (Accounting & Policy)
5. **POST /account** (line 471)
   - Current: Calls account agent
   - New: Emit ACCOUNT_REQUESTED event
   - Return: `{status: "processing"}`

6. **POST /policy** (line 554)
   - Current: Direct policy update
   - New: Emit POLICY_CHANGED event
   - Return: `{status: "updated"}`

---

## ConsciousState Query Methods (Need to Add)

To enable API queries of results, extend ConsciousState with:

```python
async def get_judgment_by_id(judgment_id: str) -> Optional[Judgment]:
    """Query recent judgments for matching ID."""
    for j in self._recent_judgments:
        if j.judgment_id == judgment_id:
            return j
    return None

async def get_recent_action_results(limit: int = 10) -> List[ActionResult]:
    """Query recent action execution results."""
    # Used by /judge/{id} polling endpoint
    ...
```

---

## Implementation Sequence

### Step 1: Refactor /judge endpoint
- Replace orchestrator.run() with event emission
- Update response model to include `status` field
- Add logging for event emission

### Step 2: Add query endpoint
- GET /judge/{judgment_id} - polls for result
- Queries ConsciousState.get_judgment_by_id()

### Step 3: Refactor /perceive similarly
- Same pattern as /judge
- Different event type (or same with metadata)

### Step 4: Refactor /learn, /feedback
- Emit events instead of direct state mutation
- Return status without waiting

### Step 5: Convert /account and /policy
- Emit governance events
- Return status immediately

### Step 6: Update WebSocket handlers
- SDK endpoint should also use event emission
- Clients poll for results instead of waiting

---

## Key Files to Modify

1. **cynic/api/routers/core.py** - Main refactoring
   - /judge, /perceive, /learn, /feedback, /account, /policy
   - Add query endpoints

2. **cynic/api/models.py** - Add new response models
   - Add `status` field to responses
   - Add async query schemas

3. **cynic/organism/conscious_state.py** - Add query methods
   - get_judgment_by_id()
   - get_recent_actions()
   - get_policy_state()

4. **cynic/api/routers/sdk.py** (if applicable)
   - Convert SDK endpoint to event-driven
   - May require separate strategy

---

## Event Emission Pattern (Standard)

Every endpoint conversion follows:

```python
# 1. Build payload dict
payload = {"field": value, ...}

# 2. Create event
event = Event(
    type=CoreEvent.SOMETHING_REQUESTED,
    payload=payload,
    source="api:endpoint_name"
)

# 3. Emit to CORE bus
await get_core_bus().emit(event)

# 4. Return immediately
return {
    "event_id": event.event_id,
    "status": "processing",
    "query_url": f"/api/result/{event.event_id}"  # Optional
}
```

---

## Testing Strategy

1. **Unit tests**: Verify event emission (mock bus)
2. **Integration tests**: Verify query endpoint returns results
3. **E2E tests**: Full flow: emit → scheduler → query
4. **Load test**: Verify 1000s RPS doesn't break anything

---

## Rollback Plan

If issues arise:
- Endpoints have dual-mode: emit event + wait for result
- Can revert to blocking on errors
- Feature flag: `USE_EVENT_DRIVEN_API` env var

---

## Timeline

- **Tier 1 refactoring**: ~2 hours (judge + perceive endpoints)
- **Query endpoint**: ~30 min
- **Tier 2-3 refactoring**: ~1.5 hours (learn, feedback, account, policy)
- **Testing**: ~1 hour
- **Total**: ~5 hours

---

## Success Criteria

- ✅ All 6 core endpoints emit events (no blocking orchestrator calls)
- ✅ Query endpoints return results from ConsciousState
- ✅ All existing tests still pass
- ✅ Response time < 100ms (event emission + return)
- ✅ 0 integration test failures
- ✅ No performance regression in scheduler

---

## Notes

- Events flow through 3 buses (CORE, AUTOMATION, AGENT)
- ConsciousState is already subscribed and listening
- No new event bus subscriptions needed
- Just emit and let scheduler handle it
- API becomes a "recorder of intentions" not "controller of organism"

