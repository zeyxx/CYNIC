# CYNIC Data Flow Analysis — End-to-End Trace

**Session**: 2026-02-21  
**Status**: CRITICAL FINDINGS DISCOVERED  
**Confidence**: 48% (φ-bounded — multiple silent failure paths found)

## MISSION STATEMENT

Trace a single request through the organism to answer:
- Is data ACTUALLY persisted or just in memory?
- Does ConsciousState actually get updated?
- Are state mutations visible to the next request?
- Where could data be lost?

## CASE STUDY: POST /judge Request

### FLOW DIAGRAM

1. User calls endpoint
2. FastAPI receives request
3. Data flows through: handler → service → storage → ConsciousState
4. Response returned to user

STEP-BY-STEP:

1. POST /judge arrives at router
   - Parse JudgeRequest
   - Get compressed context
   - Create Cell (in memory)

2. JudgeOrchestrator.run()
   - Select consciousness level
   - Run handler chain
   - Dogs vote
   - Produce Judgment (in memory)

3. Back in router — FOUR OPERATIONS:
   a) state.last_judgment = {state_key, action, judgment_id} [MEMORY]
   b) _write_guidance(cell, judgment) [JSON FILE, SYNC, NO ERROR REPORTING]
   c) _persist_judgment(judgment) [ASYNC TASK, NON-BLOCKING, NO AWAIT]
   d) Emit JUDGMENT_CREATED event [ASYNC EVENT, NON-DETERMINISTIC]

4. Return HTTP 200 to user
   - At this point: guidance.json written
   - At this point: last_judgment in memory
   - At this point: DB save may not have started yet
   - At this point: ConsciousState may not have updated yet

## CRITICAL FINDINGS

### Risk 1: Database Persistence Race Condition

_persist_judgment() creates async task WITHOUT AWAITING:

    async def _do_save():
        try:
            repo = _get_judgment_repo()
            await repo.save(data)  # May never execute!
        except Exception as e:
            logger.debug("Judgment persistence skipped: %s", e)  # Silent!

    asyncio.get_running_loop().create_task(_do_save())  # Fire and forget
    # Returns IMMEDIATELY

IMPACT:
- Request returns before DB insert
- If process crashes → judgment lost from DB
- No user notification of failure
- NO CONFIRMATION that save actually worked

### Risk 2: ConsciousState May Not Sync In Time

Event handler runs async:

    await get_core_bus().emit(Event.typed(
        CoreEvent.JUDGMENT_CREATED,
        JudgmentCreatedPayload(...)
    ))
    # Handler fires in background, timing non-deterministic

Then ConsciousState subscribes:

    async def _on_judgment_created(self, event):
        async with self._state_lock:
            self._recent_judgments.append(snapshot)

IMPACT:
- High-load scenario (10+ RPS): events queue up
- Request 2 arrives before event handler for Request 1 fires
- ConsciousState reader sees stale data
- Next /health call may not include latest judgment

### Risk 3: Silent Failures Everywhere

All persistence operations suppress errors:

    except Exception as e:
        logger.debug("Judgment persistence skipped: %s", e)

If DB down, disk full, or pool exhausted:
- User sees HTTP 200 OK
- Judgment never actually saved
- No error in response
- No retry attempted
- No alert system triggered

### Risk 4: Multiple Writers Same File (Race)

guidance.json written in TWO places:

1. Router: _write_guidance() [sync, immediate]
2. Event handler: _on_judgment_created() [async, delayed]

Both write to:
- ~/.cynic/guidance.json
- ~/.cynic/guidance-{instance_id}.json

Last write wins, but which one fires last?
If async handler slower, stale data persists.

## DATA PERSISTENCE SUMMARY

What is ACTUALLY persisted?

| Component | Memory | Disk | Database | Next Request |
|-----------|--------|------|----------|--------------|
| Judgment | YES | YES (guidance.json) | MAYBE | INCONSISTENT |
| Dog Votes | YES | YES (guidance.json) | MAYBE | INCONSISTENT |
| Q-Table | YES | NO | MAYBE | INCONSISTENT |
| ConsciousState | YES | YES (JSON) | NO | RACE CONDITION |
| last_judgment | YES | NO | NO | YES (same process) |

## NEXT REQUEST VISIBILITY TEST

Request 1: POST /judge
  → Returns HTTP 200
  → guidance.json written (visible)
  → DB save queued (maybe)
  → Event emitted (maybe not fired yet)

Request 2: GET /health (1ms later)
  → Reads state.conscious_state.get_recent_judgments()
  → May or may not include judgment from Request 1
  → Depends on whether event handler fired
  → UNDEFINED BEHAVIOR

## ROOT CAUSE

The organism uses EVENT-DRIVEN state updates, but:
1. Events are emitted synchronously, handled asynchronously
2. HTTP responses returned BEFORE handlers complete
3. No synchronization between request completion and state mutation
4. No confirmation that async tasks actually completed
5. No retry logic for failed persists

This is CORRECT ARCHITECTURE for autonomous operation,
but WRONG ARCHITECTURE for request/response semantics.

## TIER 1 FIXES REQUIRED

1. AWAIT persistence before returning response
   - Don't hide database failures
   - Add persistence_error field to response

2. SYNC ConsciousState before returning response
   - Update singleton synchronously
   - Guarantee next request sees state change

3. QUERY DB to confirm persistence
   - Poll after async save
   - Add db_confirmed field to response

4. ELEVATE ERROR LOGGING
   - Log at WARNING level, not DEBUG
   - Alert on persistence failures
   - Include in health metrics

## CONFIDENCE

Confidence: 48% (φ-bounded, two failure modes at 30% probability each)
- Architecture sound (async, event-driven)
- Implementation unsound (silent failures, race conditions)
- Observation ports out of sync with data source

Territory mapped. Bloodstream analyzed. Critical leaks found.

