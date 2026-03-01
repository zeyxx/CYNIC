# Priority 4: Unified State Mutability — Analysis & Fix

**Date:** 2026-03-01
**Goal:** Enforce immutable-by-default across all state models
**Status:** In Progress

## Problem: Inconsistent Mutation Strategies

Current codebase has 5 different mutation patterns:

### Pattern 1: Frozen Dataclasses ✅ GOOD
```python
@dataclass(frozen=True)
class UnifiedJudgment:
    # Cannot be mutated after creation
    verdict: str
    q_score: float
```
**Examples:** UnifiedJudgment, UnifiedLearningOutcome
**Score:** 10/10 (perfect immutability)
**Count:** 2 classes

### Pattern 2: Mutable Pydantic Models ❌ BAD
```python
class ValueCreation(UnifiedModel):
    # Default: frozen=False
    model_config = ConfigDict(extra="allow", frozen=False)
    creation_id: str
    creator_id: str
```
**Examples:** ValueCreation, ImpactMeasurement, GovernanceCommunity, etc.
**Score:** 3/10 (easily mutated)
**Count:** 8+ classes
**Risk:** Silent data corruption if mutated during async operations

### Pattern 3: Mutable Methods on BaseModel ❌ BAD
```python
class JudgmentBuffer(BaseModel):
    buffer: deque = Field(...)

    def add(self, item):  # Mutates buffer in-place
        self.buffer.append(item)
```
**Examples:** JudgmentBuffer, OutcomeBuffer, CommunityBuffer, etc.
**Score:** 2/10 (encourages mutation)
**Count:** 6 classes
**Risk:** Buffers can be modified from anywhere

### Pattern 4: Mutable Collections ❌ BAD
```python
class UnifiedConsciousState(BaseModel):
    dog_agreement_scores: dict[int, float] = Field(default_factory=dict)  # Mutable!
    active_axioms: list[str] = Field(default_factory=list)  # Mutable!
```
**Examples:** Dicts and lists in state models
**Score:** 3/10 (easily mutated)
**Risk:** Concurrent mutations during async operations

### Pattern 5: No Mutation Model ❌ BAD
```python
class GovernanceProposal(UnifiedModel):
    status: str = "PENDING"  # Can be mutated directly
    yes_votes: float = 0.0   # Can be mutated directly
```
**Examples:** Most governance classes
**Score:** 2/10 (anything can be changed)
**Risk:** No auditability of state changes

## The Fix: Immutable-by-Default Architecture

### Step 1: Convert Mutable Pydantic Models → Frozen Dataclasses

**Before:**
```python
class ValueCreation(UnifiedModel):
    model_config = ConfigDict(extra="allow", frozen=False)
    creation_id: str
```

**After:**
```python
@dataclass(frozen=True)
class ValueCreation:
    creation_id: str
    creator_id: str
    creation_type: str
    description: str
    # ... other fields

    # For extra flexibility, add evolve() method
    def evolve(self, **kwargs) -> ValueCreation:
        """Create new instance with updated fields."""
        return dataclasses.replace(self, **kwargs)
```

### Step 2: Immutable Collections

**Before:**
```python
class UnifiedConsciousState(BaseModel):
    dog_agreement_scores: dict[int, float] = Field(default_factory=dict)  # Mutable
```

**After:**
```python
@dataclass(frozen=True)
class UnifiedConsciousState:
    dog_agreement_scores: MappingProxyType = Field(default_factory=lambda: MappingProxyType({}))
    active_axioms: tuple[str, ...] = Field(default_factory=tuple)  # tuple instead of list
```

### Step 3: Immutable Buffers

**Before:**
```python
class JudgmentBuffer(BaseModel):
    buffer: deque = Field(...)

    def add(self, item):
        self.buffer.append(item)  # Mutates!
```

**After:**
```python
@dataclass(frozen=True)
class JudgmentBuffer:
    buffer: tuple[UnifiedJudgment, ...] = Field(default_factory=tuple)

    def add(self, item: UnifiedJudgment) -> "JudgmentBuffer":
        """Return NEW buffer with item added (immutable operation)."""
        new_buffer = self.buffer + (item,)
        # Trim to maxlen (fibonacci(11) = 89)
        if len(new_buffer) > fibonacci(11):
            new_buffer = new_buffer[-fibonacci(11):]
        return JudgmentBuffer(buffer=new_buffer)
```

## Implementation Plan

### Phase 1: Core State Models (Week 1)
- [ ] UnifiedJudgment → verify frozen ✅
- [ ] UnifiedLearningOutcome → verify frozen ✅
- [ ] ValueCreation → convert to frozen + evolve()
- [ ] ImpactMeasurement → convert to frozen + evolve()
- [ ] GovernanceCommunity → convert to frozen + evolve()
- [ ] GovernanceProposal → convert to frozen + evolve()
- [ ] GovernanceVote → convert to frozen + evolve()

### Phase 2: Buffers (Week 2)
- [ ] JudgmentBuffer → convert to tuple-based, immutable add()
- [ ] OutcomeBuffer → convert to tuple-based, immutable add()
- [ ] ValueBuffer → convert to tuple-based, immutable add()
- [ ] ImpactBuffer → convert to tuple-based, immutable add()
- [ ] CommunityBuffer → convert to dict-based frozen, immutable operations
- [ ] ProposalBuffer → convert to tuple-based, immutable add()

### Phase 3: Collections (Week 3)
- [ ] UnifiedConsciousState → use MappingProxyType + tuples
- [ ] Audit all dict/list fields → convert to MappingProxyType/tuple
- [ ] Add validators to reject mutation attempts

### Phase 4: Tests (Week 4)
- [ ] Frozen enforcement tests
- [ ] Evolve/add return new instances
- [ ] Immutable collections (MappingProxyType/tuple)
- [ ] No silent mutations possible

## Success Criteria

| Metric | Target | Status |
|--------|--------|--------|
| All state models frozen=True | 100% | 0% (in progress) |
| No mutable buffers | 0 | 6 (need to convert) |
| No mutable collections | 0 | 8+ (need to convert) |
| evolve() methods where needed | All mutable states | 0 (need to add) |
| Test coverage | 100% | TBD |

## Benefits After Fix

1. **Thread Safety**: No race conditions from concurrent mutations
2. **Auditability**: All changes go through evolve() (traceable)
3. **Testability**: Can snapshot state without worrying about mutations
4. **Performance**: Immutable objects are cacheable and can be reused
5. **Debuggability**: State always consistent, no surprise mutations
6. **Fractal Harmony**: All state follows same pattern (8.0+ consistency)

## Risk Mitigation

**Risk:** Buffers using tuples might be slow for large buffers
**Mitigation:** Buffer sizes are bounded (fibonacci-based maxlen), operations stay O(n) where n < 200

**Risk:** evolve() might be tedious for frequent updates
**Mitigation:** Only use evolve() for strategic state; batch updates together

**Risk:** Breaking change to existing code
**Mitigation:** evolve() provides drop-in replacement for mutations

---

**Next Steps:**
1. Start with ValueCreation, ImpactMeasurement (most used)
2. Add evolve() methods
3. Update tests
4. Gradually migrate buffers
5. Verify no regressions
