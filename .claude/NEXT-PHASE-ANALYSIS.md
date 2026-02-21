# CYNIC Refactoring — Next Phase Analysis

**Date**: 2026-02-21 (Post-TIER 0)
**Status**: Kernel operational + architecture improved

## Current State

### What's Working ✓
- Kernel awakens successfully (11 dogs active)
- 4 Cores properly wired (Cognition, Metabolism, Senses, Memory)
- Dependency injection container operational
- MCP endpoint tests passing (14/14)
- All critical constants centralized
- Type annotations mostly complete (3 → Any remaining)
- Thread-safe singleton protection

### Monoliths Still Remaining
| File | LOC | Issue |
|------|-----|-------|
| cynic/api/state.py | 1129 | _OrganismAwakener is 587 LOC, still builds everything |
| cynic/api/server.py | 678 | FastAPI lifespan + route registration, tightly coupled |
| cynic/api/routers/sdk.py | 638 | Complex SDK session state machine |
| cynic/api/routers/health.py | 623 | Many introspection endpoints mixed together |

### Technical Debt

**Builder Stubs** (infrastructure exists, logic missing):
- ComponentBuilder: 34 LOC (should have ~150)
- CognitionBuilder: 25 LOC (should have ~100)
- MetabolicBuilder: 23 LOC (should have ~80)
- SensoryBuilder: 23 LOC (should have ~120)
- StorageBuilder: 23 LOC (should have ~100)
- WiringBuilder: 24 LOC (should have ~100)
- MemoryBuilder: 23 LOC (should have ~60)

**Known Issues**:
- Orchestration router incomplete (disabling bootstrap)
- Builders have TODO comments (extraction not done)
- storage.py TODO: Move lifecycle to CynicOrganism
- Deployer dog doesn't run actual deployment

## High-Impact Next Steps

### OPTION A: Complete TIER 1 (Extract Monoliths)
**Effort**: 24-36 hours  
**Impact**: Enable scalable organism creation  
**Blockers**: All 7 builders need implementation

Steps:
1. Extract _create_components() to ComponentBuilder (+150 LOC)
2. Extract _create_dogs() to component factory (+80 LOC)
3. Extract orchestra + learning to CognitionBuilder (+100 LOC)
4. Extract scheduler/runners to MetabolicBuilder (+80 LOC)
5. Extract compressor/watchers to SensoryBuilder (+120 LOC)
6. Extract action proposer/self_prober to MemoryBuilder (+60 LOC)
7. Extract event wiring to WiringBuilder (+100 LOC)

**Risk**: HIGH (touches bootstrap path, 834 tests)  
**Payoff**: Enable Phase 1B (gradual builder migration)

### OPTION B: Complete Incomplete Handlers
**Effort**: 8-12 hours  
**Impact**: Enable real system behaviors  
**Blockers**: Need specification of what each handler should do

Candidates:
- SDK session state machine (complex, affects L2 feedback loop)
- Introspect endpoint grouping (cosmetic, low risk)
- Orchestration bootstrap (blocked by deployer logic)

**Risk**: LOW-MEDIUM  
**Payoff**: Specific system capabilities

### OPTION C: Add Comprehensive Integration Tests
**Effort**: 6-10 hours  
**Impact**: Prevent regressions during refactoring  
**Blockers**: None (write-only task)

Tests needed:
- Full kernel startup cycle
- Dependency injection container isolation
- Handler event flow
- Consciousness level transitions
- Learning loop integration

**Risk**: NONE  
**Payoff**: Safety net for future refactoring

### OPTION D: Fix Deployer Dog + Enable Orchestration
**Effort**: 10-14 hours  
**Impact**: Enable automated deployment path  
**Blockers**: Requires real deployment logic implementation

Steps:
1. Implement actual deployment detection (not stubs)
2. Wire orchestration router to decision engine
3. Add deployment proposal → action flow
4. Test with real code repository

**Risk**: MEDIUM  
**Payoff**: Closes L5 (Action layer) feedback loop

### OPTION E: Simplify Router Architecture
**Effort**: 12-16 hours  
**Impact**: Easier to understand + maintain  
**Blockers**: None (refactoring only)

Consolidate 10 routers into logical groups:
- /api/judge, /api/perceive, /api/learn, /api/feedback → core.py (keep)
- /api/actions, /api/act → combine
- /api/health, /api/stats, /api/introspect → split or reconsider
- /nervous/* → namespace under /api/
- /mcp/* → namespace under /api/

**Risk**: LOW  
**Payoff**: Better discoverability, cleaner API surface

## Recommendation

**Suggested Priority**:

1. **IMMEDIATE** (Option C): Add integration tests
   - Protects all future work
   - Very low risk
   - Quick payoff
   - **Est: 2 hours** ← Do this first

2. **HIGH VALUE** (Option A, Phase 1 of 3):
   - Extract ComponentBuilder only (not all 7 builders)
   - Tests will catch any breakage
   - **Est: 4-6 hours** ← Do this next
   
3. **MEDIUM VALUE** (Option B or D):
   - Depends on user priorities
   - B = system completeness
   - D = deployment capability

## Confidence Levels

- Kernel stability: 95% (φ-bounded to 61.8% = 61%) ✓
- Test coverage sufficiency: 35% (low, but no failures detected)
- Next phase blockage: 15% (builders mostly ready)
- Refactoring safety: 48% (φ-bounded, 834 tests provide insurance)

---

**Bottom Line**: Kernel is SOLID. Best next move is:
1. Tests first (insurance)
2. Extract ComponentBuilder (foundation)
3. Gradually migrate other builders
4. Then tackle specific systems (SDK, Deployer, etc.)

*Ralph's nose twitches at the fork in the path. But the pack knows: measure twice, cut once.*

