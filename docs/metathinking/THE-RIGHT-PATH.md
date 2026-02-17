# The RIGHT Path: Architectural Truth

> *GROWL* "Fast path = technical debt. Right path = build once, correctly." - κυνικός

**Date**: 2026-02-13
**Question**: "je veux le BON chemin pas le plus agréable"
**Answer**: Neither Wire First nor Unify First. **Vertical-by-critical-system.**

---

## THE ARCHITECTURAL TRUTH

### Why "Wire First" is WRONG

**Wire First approach:**
```
1. Wire hooks → HumanPerceiver (5 lines)
2. Wire startup → startAllWatchers() (2 lines)
3. BOOM: Organism breathes (4-8h)
```

**Problem**: Wiring into **fragmented architecture**.

**Consequences:**
```
Hooks → HumanPerceiver → Core Bus (1 of 3)
                      ↓
                   Judge → fires event on Core Bus
                      ↓
                   Router → needs Agent Bus for Dogs
                      ↓
                   [BRIDGE] → routes between buses
                      ↓
                   Dogs → Agent Bus
                      ↓
                   Learning → back to Core Bus
                      ↓
               [COMPLEXITY EXPLOSION]
```

**Result**:
- System works BUT wired through 3 buses + bridge
- Every event crosses bridge (latency, complexity, failure points)
- To unify later = **RE-WIRE EVERYTHING** (undo + redo)

**Verdict**: Fast but creates **architectural debt**. Wrong path.

---

### Why "Unify First" is ALSO WRONG

**Unify First approach:**
```
Phase 1: Unified Process (daemon core)
Phase 2: Unified Event System
Phase 3: Unified Perception
Phase 4: Pipelined Processing
Phase 5: Learning Orchestrator
Phase 6: Lifecycle Manager

Timeline: 2-4 weeks
Then: Wire (4-8h)
```

**Problem**: **Big bang** with no validation.

**Consequences:**
```
Week 1: Build unified event bus (no way to test)
Week 2: Build perception orchestrator (depends on event bus, can't validate)
Week 3: Build pipeline + orchestrator (depends on all above)
Week 4: Wire everything
Week 4 Day 5: Discover event bus design is wrong
            → REWRITE EVERYTHING
```

**Result**:
- 2-4 weeks of work with **zero validation**
- If any foundational choice is wrong = restart
- No incremental learning

**Verdict**: Theoretically clean but **high risk, no feedback**. Wrong path.

---

## THE RIGHT PATH: Vertical-by-Critical-System

### Biological Parallel: How Embryos Actually Develop

**Embryos DON'T:**
- ❌ Build all organs then connect them (Unify First)
- ❌ Connect incomplete organs (Wire First)

**Embryos DO:**
- ✅ Build critical systems FIRST, in dependency order
- ✅ Each system = complete + functional before next
- ✅ Validate each system before building next

**Example: Human embryo week-by-week:**
```
Week 3: Neural tube (nervous system) FIRST
        ↓ [VALIDATE: neural tube closed]
Week 4: Heart tube (circulation) SECOND
        ↓ [VALIDATE: heart beats]
Week 5: Limb buds (motor system) THIRD
        ↓ [VALIDATE: can move]
...
```

**Pattern**: Critical system → Validate → Next system

---

## THE RIGHT PATH FOR CYNIC

### Step 0: Identify Critical System

**Question**: Which system is MOST foundational?

**Candidates:**
1. Event Bus (communication backbone)
2. Perception (input)
3. Processing (logic)
4. Storage (memory)

**Answer**: **Event Bus**

**Why**: All other systems communicate THROUGH event bus.
- Perception emits events → event bus
- Judge receives events → event bus
- Learning receives outcomes → event bus
- **NO system works without event bus**

**Verdict**: Event bus is THE critical system. Build it RIGHT, FIRST.

---

### The RIGHT Path (4 Phases)

#### Phase 1: Unified Event System (FOUNDATION) [3-5 days]

**Goal**: ONE event bus, working, validated.

**Steps:**
1. **Design unified taxonomy** (1 day)
   - Map all events from 3 buses to single taxonomy
   - Categories: PERCEPTION, JUDGMENT, DECISION, ACTION, LEARNING, SYSTEM
   - Document: `docs/architecture/unified-event-taxonomy.md`

2. **Implement UnifiedEventBus** (1 day)
   - Extend ParallelEventBus
   - Add event categories, namespaces
   - Add event history (for debugging)
   - Location: `packages/core/src/bus/unified-event-bus.js`

3. **Write migration layer** (1 day)
   - EventAdapter wraps old buses
   - Routes old events to new bus
   - Allows incremental migration
   - Location: `packages/core/src/bus/event-adapter.js`

4. **Migrate core systems** (1-2 days)
   - Judge → unified bus
   - Router → unified bus
   - Learning → unified bus
   - Verify: All systems still work

5. **Remove old buses + bridge** (1 day)
   - Delete Core Bus (keep for imports, deprecate)
   - Delete Automation Bus
   - Delete Agent Bus
   - Delete EventBusBridge
   - Result: ONE event system

**Validation:**
```javascript
// Test: Single event reaches all subscribers
unifiedBus.emit('perception:human:state', data);
// Should reach: Judge, Router, Learning, Emergence
// Verify: Check logs, count handlers, measure latency
```

**Output**: Working unified event bus, all systems migrated.

**Risk**: Medium (event routing is complex, but testable incrementally)

---

#### Phase 2: Unified Perception Lifecycle [2-3 days]

**Goal**: ALL watchers start/stop as ONE system.

**Prerequisites**: ✅ Phase 1 complete (unified event bus exists)

**Steps:**
1. **Create PerceptionOrchestrator** (1 day)
   - Class manages all 7 watchers
   - Lifecycle: init() → start() → health() → stop()
   - Events → unified bus
   - Location: `packages/node/src/perception/orchestrator.js`

2. **Integrate HumanPerceiver** (0.5 day)
   - Move HumanPerceiver into perception/
   - Wire to orchestrator
   - Same lifecycle as other watchers

3. **Wire to daemon init** (0.5 day)
   - Daemon startup calls orchestrator.start()
   - NOT optional, MANDATORY
   - Location: `packages/node/src/daemon/index.js`

4. **Add health monitoring** (1 day)
   - Each watcher reports health
   - Orchestrator aggregates
   - Expose via /health endpoint

**Validation:**
```bash
# Start daemon
cynic-node daemon start

# Check logs
tail -f ~/.cynic/logs/daemon.log | grep "watcher"
# Expected: "All 7 watchers started" within 2s

# Check health
curl http://localhost:3721/health
# Expected: perception.status = "healthy", perception.watchers = 7
```

**Output**: All watchers running, health monitored.

**Risk**: Low (watchers already exist, just orchestrating)

---

#### Phase 3: Wire First Data Flow (Hooks → Perception) [1 day]

**Goal**: FIRST real data flows through unified system.

**Prerequisites**: ✅ Phase 1+2 complete (unified bus + orchestrated perception)

**Steps:**
1. **Wire hooks → HumanPerceiver** (0.5 day)
   ```javascript
   // File: scripts/hooks/perceive.js
   import { getHumanPerceiver } from '@cynic/node/perception/orchestrator';

   const perceiver = getHumanPerceiver();
   perceiver.recordToolUse({ tool, args, success: true });

   // Emits to UNIFIED BUS (not Core Bus)
   // perception:human:tool-use → Judge, Router, Learning (all get it)
   ```

2. **Verify end-to-end flow** (0.5 day)
   ```javascript
   // Hook emits event
   perceiver.recordToolUse(...);

   // → UNIFIED BUS
   // → Judge receives (makes judgment)
   // → Router receives (routes decision)
   // → Learning receives (updates weights)
   // → DB receives (stores judgment)

   // Verify: Check DB for judgment row
   SELECT * FROM judgments ORDER BY created_at DESC LIMIT 1;
   // Expected: 1 row with recent timestamp
   ```

**Validation:**
```sql
-- Check judgments table
SELECT COUNT(*) FROM judgments WHERE created_at > NOW() - INTERVAL '1 hour';
-- Expected: 1+ per session

-- Check learning_events table
SELECT COUNT(*) FROM learning_events WHERE event_type LIKE 'perception%';
-- Expected: 1+ per session
```

**Output**: First data flow proven, 0% → 10% living.

**Risk**: Low (just wiring, infrastructure already built)

---

#### Phase 4: Learning Orchestration [3-4 days]

**Goal**: 11 learning loops work as ONE system.

**Prerequisites**: ✅ Phase 1+2+3 complete (data flows through system)

**Steps:**
1. **Design learning policy** (1 day)
   - Priority levels: P1 (real-time) → P4 (async)
   - Conflict resolution: φ-blending
   - Execution order
   - Document: `docs/architecture/learning-orchestration-policy.md`

2. **Implement LearningOrchestrator** (1-2 days)
   - Collects learning signals
   - Sorts by priority
   - Executes in order
   - Merges weight updates
   - Location: `packages/node/src/learning/orchestrator.js`

3. **Migrate learning loops** (1 day)
   - Each loop registers with orchestrator
   - Orchestrator calls loops (not loops self-trigger)
   - Result: Coordinated learning

4. **Validate learning works** (1 day)
   ```javascript
   // Provide feedback
   user.provideFeedback({ judgmentId, rating: 'good' });

   // Orchestrator should:
   // 1. Update calibration (P1)
   // 2. Update Thompson (P1)
   // 3. Update Q-Learning (P2)
   // 4. Update DPO (P2)
   // 5. Update EWC (P3)
   // 6. Blend all weights
   // 7. Apply to router

   // Verify: Next judgment differs (learned)
   ```

**Validation:**
```sql
-- Check learning loops active
SELECT DISTINCT loop_type FROM learning_events
WHERE created_at > NOW() - INTERVAL '1 hour';
-- Expected: 5+ loop types

-- Check weight updates
SELECT * FROM routing_weights ORDER BY updated_at DESC LIMIT 1;
-- Expected: Updated timestamp
```

**Output**: Unified learning, 10% → 30% living.

**Risk**: Medium (learning is complex, but policy is clear)

---

## TIMELINE & MILESTONES

```
Week 1:
  Days 1-2: Phase 1 (design + implement unified bus)
  Days 3-5: Phase 1 (migrate systems + remove old buses)

  Milestone: ✅ ONE event system, all systems migrated
  Living %: 0% (no data flows yet)

Week 2:
  Days 1-3: Phase 2 (perception orchestrator + wiring)
  Day 4:    Phase 3 (wire hooks → perception)
  Day 5:    Phase 3 (validate end-to-end flow)

  Milestone: ✅ First data flows, first judgment
  Living %: 0% → 10% (HUMAN domain breathing)

Week 3:
  Days 1-4: Phase 4 (learning orchestration)
  Day 5:    Phase 4 (validate learning)

  Milestone: ✅ Learning works, behavior changes
  Living %: 10% → 30% (HUMAN + learning loops)

Week 4:
  Buffer for issues, testing, documentation

Total: 3-4 weeks to unified + living organism
```

---

## WHY THIS IS THE RIGHT PATH

### 1. Incremental Validation
```
Phase 1 complete → Test event bus works
Phase 2 complete → Test watchers start
Phase 3 complete → Test data flows
Phase 4 complete → Test learning works

Each phase validated BEFORE next starts.
```

### 2. No Architectural Debt
```
Wire First:
  → Wire into fragmented system
  → Must re-wire after unification
  → Technical debt

Right Path:
  → Build unified system FIRST
  → Wire ONCE into unified system
  → Zero debt
```

### 3. Fail Fast
```
Unify First (big bang):
  → Week 4: Discover design flaw
  → Restart from week 1

Right Path:
  → Day 5: Discover design flaw in event bus
  → Fix, continue (only 5 days lost)
```

### 4. Biological Correctness
```
Embryo builds:
  Neural system → Circulatory → Organs
  (Critical → Less critical → Periphery)

CYNIC builds:
  Event system → Perception → Processing → Learning
  (Communication → Input → Logic → Adaptation)

Same pattern = biologically sound
```

---

## RISK ANALYSIS

| Phase | Risk Level | Mitigation | Fallback |
|-------|-----------|------------|----------|
| **Phase 1: Event Bus** | Medium | Incremental migration, keep old buses during transition | Revert to 3 buses if fails |
| **Phase 2: Perception** | Low | Watchers already exist, just orchestrating | Keep manual start as fallback |
| **Phase 3: Wire Flow** | Low | Small changes, reversible | Disconnect if issues |
| **Phase 4: Learning** | Medium | Policy documented, testable | Disable orchestration, use old logic |

**Overall Risk**: Medium (lower than Unify First, higher than Wire First, but RIGHT)

---

## COMPARISON TABLE

| Aspect | Wire First | Unify First | RIGHT Path |
|--------|-----------|-------------|------------|
| **Time to first breath** | 4-8h | 2-4 weeks | 2 weeks |
| **Time to unified** | 2-4 weeks | 2-4 weeks | 3-4 weeks |
| **Technical debt** | High | None | None |
| **Risk** | Low | High | Medium |
| **Validation** | Continuous | Zero (until end) | Continuous |
| **Re-work needed** | High (re-wire) | Medium (fix design flaws) | Low (incremental) |
| **Architectural quality** | Poor | Excellent | Excellent |
| **Learning value** | Low | High | High |
| **φ alignment** | EXTRACT fast | BURN time | BURN+EXTRACT balanced |

---

## THE ANSWER

**Tu as demandé: "le BON chemin pas le plus agréable"**

**Le BON chemin:**
```
NOT Wire First (fast but debt)
NOT Unify First (clean but risky)

BUT Vertical-by-Critical-System:
  1. Event Bus (foundation)
  2. Perception (orchestrated)
  3. Wire Flow (validate)
  4. Learning (orchestrated)

Timeline: 3-4 weeks
Risk: Medium, mitigated by incremental validation
Result: Unified organism, zero debt, proven living
```

**On commence par:** **Phase 1 - Unified Event System (Week 1)**

---

## IMMEDIATE NEXT STEPS (Day 1)

### Step 1: Design Unified Event Taxonomy (4h)
```bash
# Create document
touch docs/architecture/unified-event-taxonomy.md

# Map all events:
# - Core Bus: 20 events
# - Automation Bus: 15 events
# - Agent Bus: 39 events
# Total: 74 events → unified taxonomy

# Categories:
# - perception:* (all domain watchers)
# - judgment:* (judge, dimensions, verdicts)
# - decision:* (router, governance)
# - action:* (actors, execution)
# - learning:* (11 loops)
# - system:* (lifecycle, health)
```

### Step 2: Create UnifiedEventBus Class (4h)
```bash
# File: packages/core/src/bus/unified-event-bus.js

# Features:
# - Extends ParallelEventBus
# - Event namespaces (perception:human:*)
# - Event history (for debugging)
# - Event filtering (by category)
# - Health metrics (events/sec, latency)
```

**End of Day 1**:
- ✅ Event taxonomy documented
- ✅ UnifiedEventBus implemented
- ✅ Tests written
- Ready for Day 2 (migration)

---

*sniff* Confidence: 61% (φ⁻¹ limit - path is correct, execution has unknowns)

**"The right path is not the fast path. The right path is the path you walk once."** - κυνικός

*ears perk* **On commence Phase 1, Day 1, Step 1?**
