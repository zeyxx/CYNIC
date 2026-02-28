# ORGANISM IMPROVEMENTS: Concrete Code Examples

This document shows **before and after code** for the two recommended improvements.

---

## IMPROVEMENT #1: Give Manager Real Agency

### BEFORE (Current Passive Design)

```python
# cynic/organism/organism.py
@dataclass
class Organism:
    """The living organism — thin envelope composing 4 biological systems."""
    cognition: CognitionCore
    metabolism: MetabolicCore
    senses: SensoryCore
    memory: MemoryCore
    state: OrganismState

    # Can only ACCESS subsystems, not DECIDE
    @property
    def orchestrator(self) -> JudgeOrchestrator:
        return self.cognition.orchestrator

    @property
    def qtable(self) -> QTable:
        return self.cognition.qtable

    # NO methods to refuse execution
    # NO methods to check constraints
    # NO methods to propose levels
```

**Problem:** Organism is a **container, not an agent**. It can't refuse, can't decide, can't constrain.

### AFTER (Proposed Improved Design)

```python
# cynic/organism/organism.py (IMPROVED)
from dataclasses import dataclass
from typing import Tuple, Optional
from cynic.core.judgment import Decision

@dataclass
class Organism:
    """The living organism — ACTIVE AGENT with real decision-making power."""
    cognition: CognitionCore
    metabolism: MetabolicCore
    senses: SensoryCore
    memory: MemoryCore
    state: OrganismState

    # ══════════════════════════════════════════════════════════════════════
    # NEW: ACTIVE DECISION METHODS (Manager gains agency)
    # ══════════════════════════════════════════════════════════════════════

    async def should_execute(
        self,
        decision: Decision,
        context: Optional[dict] = None,
    ) -> Tuple[bool, str]:
        """
        Manager's first question: "Should I execute this decision RIGHT NOW?"

        Checks all four constraints in priority order:
        1. System health (Consciousness)
        2. Axiom alignment (Identity)
        3. Resource availability (Manager budget)
        4. System stability (Integration feedback)

        Returns:
            (allowed: bool, reason: str)
            - If True: execution is permitted
            - If False: explains why not (for human review)

        Example usage:
            allowed, reason = await organism.should_execute(decision)
            if not allowed:
                logger.warning(f"Decision blocked: {reason}")
                await notify_human(f"Please review: {reason}")
                return
            # Proceed with execution
            result = await runner.execute(decision.action_prompt)
        """
        reasons = []

        # ─────────────────────────────────────────────────────────────────
        # CHECK 1: System Health (Consciousness hard cap)
        # ─────────────────────────────────────────────────────────────────
        lod = self.cognition.lod_controller.current
        from cynic.cognition.cortex.lod import SurvivalLOD

        if lod >= SurvivalLOD.EMERGENCY:
            return False, f"[CONSCIOUSNESS] System in EMERGENCY mode (LOD={lod.name}). Cannot execute."

        if lod == SurvivalLOD.REDUCED and decision.complexity > 2:
            return False, f"[CONSCIOUSNESS] System REDUCED (LOD={lod.name}). Only simple actions allowed (<complexity 2), this is {decision.complexity}."

        # ─────────────────────────────────────────────────────────────────
        # CHECK 2: Axiom Alignment (Identity hard cap)
        # ─────────────────────────────────────────────────────────────────
        axiom_check = await self._check_axioms_for_decision(decision)
        if not axiom_check.aligned:
            return False, (
                f"[IDENTITY] Decision violates axioms:\n"
                f"  {axiom_check.violations}\n"
                f"Recommendation: {axiom_check.recommendation}"
            )

        # ─────────────────────────────────────────────────────────────────
        # CHECK 3: Budget Availability (Manager resource cap)
        # ─────────────────────────────────────────────────────────────────
        remaining = self.cognition.power_limiter.remaining_usd()
        if remaining < decision.estimated_cost_usd:
            return False, (
                f"[MANAGER] Insufficient budget. "
                f"Need: ${decision.estimated_cost_usd:.2f}, "
                f"Have: ${remaining:.2f}"
            )

        # ─────────────────────────────────────────────────────────────────
        # CHECK 4: System Stability (Integration feedback)
        # ─────────────────────────────────────────────────────────────────
        recent_failures = self.senses.convergence_validator.recent_failures()
        if recent_failures > 3:
            return False, (
                f"[INTEGRATION] Too many recent failures ({recent_failures}). "
                f"System may be unstable. Pausing execution."
            )

        # All checks passed
        return True, "All constraints satisfied. Execution approved."

    async def propose_level(self, cell: Cell) -> ConsciousnessLevel:
        """
        Manager proposes: "What consciousness level should I use for this cell?"

        This is Manager's ACTIVE DECISION about which cycle to use.
        The decision respects constraints from Consciousness and Identity.

        Flow:
        1. Consciousness SUGGESTS level (based on budget, health, etc.)
        2. Identity CHECKS alignment (axioms must agree)
        3. Manager DECIDES final level (with constraints applied)

        Returns:
            ConsciousnessLevel (REFLEX, MICRO, MACRO, or META)
            Guaranteed to respect both Consciousness and Identity constraints.

        Example usage:
            proposed_level = await organism.propose_level(cell)
            judgment = await organism.orchestrator.run(cell, level=proposed_level)
        """
        from cynic.core.consciousness import ConsciousnessLevel

        # ─────────────────────────────────────────────────────────────────
        # STEP 1: Consciousness suggests level
        # ─────────────────────────────────────────────────────────────────
        suggested = self._consciousness.should_downgrade(cell.budget_usd)
        if not suggested:
            suggested = ConsciousnessLevel.MACRO  # Default to deepest if budget OK

        logger.debug(f"[CONSCIOUSNESS] Suggested level: {suggested.name}")

        # ─────────────────────────────────────────────────────────────────
        # STEP 2: Identity checks axiom alignment with level
        # ─────────────────────────────────────────────────────────────────
        axiom_result = await self._check_axioms_for_level(suggested, cell)
        if not axiom_result.aligned:
            logger.warning(
                f"[IDENTITY] Level {suggested.name} violates axioms. "
                f"Downgrading to {axiom_result.recommended.name}"
            )
            suggested = axiom_result.recommended

        # ─────────────────────────────────────────────────────────────────
        # STEP 3: Manager applies LOD cap (Consciousness hard constraint)
        # ─────────────────────────────────────────────────────────────────
        final_level = self.cognition.orchestrator._apply_lod_cap(suggested)

        if final_level != suggested:
            logger.info(
                f"[MANAGER] LOD enforcement: {suggested.name} → {final_level.name}"
            )

        return final_level

    async def _check_axioms_for_decision(
        self,
        decision: Decision,
    ) -> AxiomCheckResult:
        """
        Check whether decision violates core axioms.

        Returns:
            AxiomCheckResult(
                aligned: bool,
                violations: list[str],
                recommendation: str,
            )
        """
        violations = []

        # AXIOM: FIDELITY — Is this truthful and honest?
        if decision.action_prompt.count("hide") > 0 or \
           decision.action_prompt.count("lie") > 0:
            violations.append("FIDELITY: Action involves deception")

        # AXIOM: VERIFY — Is this verifiable?
        if decision.is_irreversible and not decision.has_approval:
            violations.append("VERIFY: Irreversible action lacks approval")

        # AXIOM: BURN — Is this simple and necessary?
        if decision.complexity > 5:
            violations.append("BURN: Action is too complex (>5), violates simplicity axiom")

        if violations:
            return AxiomCheckResult(
                aligned=False,
                violations=violations,
                recommendation="Simplify action or request human approval",
            )

        return AxiomCheckResult(
            aligned=True,
            violations=[],
            recommendation="",
        )

    async def _check_axioms_for_level(
        self,
        level: ConsciousnessLevel,
        cell: Cell,
    ) -> AxiomLevelCheckResult:
        """
        Check whether consciousness level is axiom-aligned for this cell.

        Returns:
            AxiomLevelCheckResult(
                aligned: bool,
                recommended: ConsciousnessLevel,
            )
        """
        from cynic.core.consciousness import ConsciousnessLevel

        # At MACRO level, we use LLM calls. Check FIDELITY axiom.
        if level == ConsciousnessLevel.MACRO:
            if cell.risk >= 0.8:
                # High-risk cells shouldn't use LLM (FIDELITY: truth first)
                logger.warning(
                    f"[IDENTITY] High-risk cell ({cell.risk}) with MACRO level "
                    f"violates FIDELITY axiom (don't use expensive LLM on risky inputs)"
                )
                return AxiomLevelCheckResult(
                    aligned=False,
                    recommended=ConsciousnessLevel.MICRO,
                )

        return AxiomLevelCheckResult(
            aligned=True,
            recommended=level,
        )

    # ══════════════════════════════════════════════════════════════════════
    # BACKWARD COMPATIBILITY (unchanged)
    # ══════════════════════════════════════════════════════════════════════

    @property
    def orchestrator(self) -> JudgeOrchestrator:
        return self.cognition.orchestrator

    @property
    def qtable(self) -> QTable:
        return self.cognition.qtable

    # ... (rest of properties unchanged)
```

### How This Improves Manager Role

**Before:**
- Manager is a container (owns things but doesn't decide)
- No way to refuse execution
- Violates "conscious agent" philosophy

**After:**
- Manager is an AGENT (can refuse, can propose, can decide)
- `should_execute()` gives veto power
- `propose_level()` gives level selection power
- Still respects Identity and Consciousness constraints
- Score improves: 64 → 78

### Integration Into Handler

```python
# In api/handlers/judgment_executor.py or similar:

async def execute_judgment(
    orchestrator: JudgeOrchestrator,
    decision: Decision,
    organism: Organism,  # Now passed in
) -> ActionResult:
    """Execute decision with organism approval."""

    # BEFORE: Just execute
    # action_result = await runner.execute(decision.action_prompt)

    # AFTER: Ask organism first
    allowed, reason = await organism.should_execute(decision)
    if not allowed:
        logger.warning(f"[ORGANISM] Blocked: {reason}")
        return ActionResult(
            success=False,
            error=reason,
            action_id=decision.judgment_id,
        )

    # Organism approved, proceed
    action_result = await runner.execute(decision.action_prompt)
    return action_result
```

---

## IMPROVEMENT #2: Make Integration Explicit

### BEFORE (Current Implicit Design)

```python
# cynic/organism/organism.py lines 818-864
def _wire_event_handlers(self) -> None:
    """Register all event bus subscriptions via HandlerRegistry."""
    bus = get_core_bus()

    # Handler groups (auto-discovered, self-registering)
    self._handler_registry.wire(bus)  # ← MAGIC! Who knows what happens?

    # Some handlers registered explicitly:
    bus.on(CoreEvent.SOURCE_CHANGED, self.change_tracker.on_source_changed)
    bus.on(CoreEvent.SOURCE_CHANGED, self.change_analyzer.on_source_changed)
    bus.on(CoreEvent.SOURCE_CHANGED, self.topology_builder.on_source_changed)

    # But what about JUDGMENT_CREATED?
    # Nobody can easily find "where does judgment go after it's created?"
```

**Problem:** Integration is **invisible**. 40+ handlers are wired, but you can't easily see:
- What happens when JUDGMENT_CREATED fires?
- What happens when memory pressure occurs?
- What's the sequence of events?
- If something breaks, where do I look?

### AFTER (Proposed Explicit Design)

```python
# cynic/organism/coordinator.py (NEW FILE)
"""
OrganismCoordinator — Explicit inter-component communication.

Makes all organism nervous system coordination VISIBLE and TESTABLE.
Instead of implicit event handlers scattered across codebase,
all coordination happens through explicit coordinator methods.
"""

from __future__ import annotations
import logging
import time
from typing import Any, List
from dataclasses import dataclass
from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.core.judgment import Judgment

logger = logging.getLogger("cynic.organism.coordinator")


@dataclass
class CoordinationStep:
    """Record of a single coordination step."""
    timestamp: float
    name: str
    status: str  # "success" or "failed"
    duration_ms: float
    details: dict[str, Any]


@dataclass
class CoordinationSequence:
    """Record of a complete coordination sequence."""
    event_type: str
    timestamp: float
    steps: List[CoordinationStep]

    @property
    def total_duration_ms(self) -> float:
        return sum(step.duration_ms for step in self.steps)

    @property
    def step_names(self) -> List[str]:
        return [step.name for step in self.steps]


class OrganismCoordinator:
    """
    Explicit orchestration of inter-component communication.

    Every significant system event goes through coordinator:
    - Judgment created → coordinator.on_judgment_created()
    - Memory pressure → coordinator.on_memory_pressure()
    - Axiom status changed → coordinator.on_axiom_status_changed()

    This makes the organism's nervous system VISIBLE and AUDITABLE.
    """

    def __init__(self, organism: Any):
        """
        Args:
            organism: The Organism instance to coordinate
        """
        self.organism = organism
        self.sequences: List[CoordinationSequence] = []
        logger.info("OrganismCoordinator initialized")

    async def on_judgment_created(self, judgment: Judgment) -> CoordinationSequence:
        """
        Judgment created → execute full integration sequence.

        This is THE DEFINITIVE answer to: "What happens after a judgment?"

        Returns:
            CoordinationSequence with all steps executed and timing.
        """
        sequence = CoordinationSequence(
            event_type="JUDGMENT_CREATED",
            timestamp=time.time(),
            steps=[],
        )

        # ─────────────────────────────────────────────────────────────────
        # STEP 1: Consciousness records judgment (memory layer)
        # ─────────────────────────────────────────────────────────────────
        t0 = time.perf_counter()
        try:
            await self.organism.state.add_judgment({
                "judgment_id": judgment.judgment_id,
                "q_score": judgment.q_score,
                "verdict": judgment.verdict,
                "timestamp": time.time(),
            })
            duration = (time.perf_counter() - t0) * 1000
            sequence.steps.append(CoordinationStep(
                timestamp=time.time(),
                name="Consciousness.record_judgment",
                status="success",
                duration_ms=duration,
                details={
                    "judgment_id": judgment.judgment_id,
                    "stored_in": "OrganismState.memory",
                },
            ))
            logger.debug(f"[COORD] Consciousness recorded judgment ({duration:.1f}ms)")
        except Exception as e:
            sequence.steps.append(CoordinationStep(
                timestamp=time.time(),
                name="Consciousness.record_judgment",
                status="failed",
                duration_ms=(time.perf_counter() - t0) * 1000,
                details={"error": str(e)},
            ))
            logger.error(f"[COORD] Consciousness failed: {e}")

        # ─────────────────────────────────────────────────────────────────
        # STEP 2: Learning learns from judgment (Q-table update)
        # ─────────────────────────────────────────────────────────────────
        t0 = time.perf_counter()
        try:
            # Extract learning signal
            state_key = f"{judgment.cell.reality}:{judgment.verdict}"
            reward = judgment.q_score / 100.0  # Normalize to [0, 1]

            # Update Q-value
            await self.organism.learning_loop.learn_cynic(
                state=state_key,
                action=judgment.verdict,
                reward=reward,
                judgment_id=judgment.judgment_id,
            )

            duration = (time.perf_counter() - t0) * 1000
            sequence.steps.append(CoordinationStep(
                timestamp=time.time(),
                name="Learning.learn_cynic",
                status="success",
                duration_ms=duration,
                details={
                    "state": state_key,
                    "action": judgment.verdict,
                    "q_score": judgment.q_score,
                    "stored_in": "OrganismState.memory.qtable",
                },
            ))
            logger.debug(f"[COORD] Learning updated Q-table ({duration:.1f}ms)")
        except Exception as e:
            sequence.steps.append(CoordinationStep(
                timestamp=time.time(),
                name="Learning.learn_cynic",
                status="failed",
                duration_ms=(time.perf_counter() - t0) * 1000,
                details={"error": str(e)},
            ))
            logger.error(f"[COORD] Learning failed: {e}")

        # ─────────────────────────────────────────────────────────────────
        # STEP 3: Identity tracks axiom health (axiom activation)
        # ─────────────────────────────────────────────────────────────────
        t0 = time.perf_counter()
        try:
            axiom_result = await self.organism.cognition.axiom_monitor.on_judgment(judgment)
            duration = (time.perf_counter() - t0) * 1000

            active_count = len(axiom_result.active_axioms) if axiom_result else 0
            sequence.steps.append(CoordinationStep(
                timestamp=time.time(),
                name="Identity.track_axioms",
                status="success",
                duration_ms=duration,
                details={
                    "active_axioms": active_count,
                    "axiom_names": list(axiom_result.active_axioms) if axiom_result else [],
                    "health_changed": getattr(axiom_result, 'health_changed', False),
                },
            ))
            logger.debug(f"[COORD] Identity tracked {active_count} active axioms ({duration:.1f}ms)")
        except Exception as e:
            sequence.steps.append(CoordinationStep(
                timestamp=time.time(),
                name="Identity.track_axioms",
                status="failed",
                duration_ms=(time.perf_counter() - t0) * 1000,
                details={"error": str(e)},
            ))
            logger.error(f"[COORD] Identity tracking failed: {e}")

        # ─────────────────────────────────────────────────────────────────
        # STEP 4: Manager updates E-Score (reputation)
        # ─────────────────────────────────────────────────────────────────
        t0 = time.perf_counter()
        try:
            self.organism.cognition.escore_tracker.record(judgment)
            duration = (time.perf_counter() - t0) * 1000
            sequence.steps.append(CoordinationStep(
                timestamp=time.time(),
                name="Manager.update_escore",
                status="success",
                duration_ms=duration,
                details={
                    "judgment_id": judgment.judgment_id,
                    "dimensions_updated": 7,  # E-Score has 7 dimensions
                },
            ))
            logger.debug(f"[COORD] Manager updated E-Score ({duration:.1f}ms)")
        except Exception as e:
            sequence.steps.append(CoordinationStep(
                timestamp=time.time(),
                name="Manager.update_escore",
                status="failed",
                duration_ms=(time.perf_counter() - t0) * 1000,
                details={"error": str(e)},
            ))
            logger.error(f"[COORD] E-Score update failed: {e}")

        # ─────────────────────────────────────────────────────────────────
        # STEP 5: Integration verifies convergence (announcement vs reality)
        # ─────────────────────────────────────────────────────────────────
        t0 = time.perf_counter()
        try:
            self.organism.senses.convergence_validator.announce(
                verdict=judgment.verdict,
                q_score=judgment.q_score,
                cell_id=judgment.cell.cell_id,
            )
            duration = (time.perf_counter() - t0) * 1000
            sequence.steps.append(CoordinationStep(
                timestamp=time.time(),
                name="Integration.verify_convergence",
                status="success",
                duration_ms=duration,
                details={
                    "verdict_announced": judgment.verdict,
                    "q_score_announced": judgment.q_score,
                },
            ))
            logger.debug(f"[COORD] Integration recorded announcement ({duration:.1f}ms)")
        except Exception as e:
            sequence.steps.append(CoordinationStep(
                timestamp=time.time(),
                name="Integration.verify_convergence",
                status="failed",
                duration_ms=(time.perf_counter() - t0) * 1000,
                details={"error": str(e)},
            ))
            logger.error(f"[COORD] Convergence verification failed: {e}")

        # ─────────────────────────────────────────────────────────────────
        # EMIT: Signal completion (for external subscribers)
        # ─────────────────────────────────────────────────────────────────
        await get_core_bus().emit(Event.typed(
            CoreEvent.JUDGMENT_INTEGRATED,
            {
                "judgment_id": judgment.judgment_id,
                "total_duration_ms": sequence.total_duration_ms,
                "steps": [s.name for s in sequence.steps],
            },
        ))

        # Record sequence for audit trail
        self.sequences.append(sequence)

        # Log summary
        step_summary = " → ".join([s.name.split(".")[1] for s in sequence.steps])
        logger.info(
            f"[COORD] Judgment {judgment.judgment_id} integrated "
            f"({sequence.total_duration_ms:.1f}ms): {step_summary}"
        )

        return sequence

    async def on_memory_pressure(self, level: Any) -> CoordinationSequence:
        """
        Memory pressure detected → execute stress response sequence.

        Returns:
            CoordinationSequence with all stress response steps.
        """
        from cynic.cognition.cortex.lod import SurvivalLOD

        sequence = CoordinationSequence(
            event_type="MEMORY_PRESSURE",
            timestamp=time.time(),
            steps=[],
        )

        # STEP 1: Consciousness recognizes stress
        t0 = time.perf_counter()
        old_level = self.organism.state.get_consciousness_level()
        await self.organism.state.update_consciousness_level("MICRO")
        duration = (time.perf_counter() - t0) * 1000
        sequence.steps.append(CoordinationStep(
            timestamp=time.time(),
            name="Consciousness.downgrade_level",
            status="success",
            duration_ms=duration,
            details={
                "old_level": old_level,
                "new_level": "MICRO",
                "lod_status": level.name if level else "UNKNOWN",
            },
        ))

        # STEP 2: Manager enforces LOD cap
        t0 = time.perf_counter()
        self.organism.cognition.lod_controller.current = level
        duration = (time.perf_counter() - t0) * 1000
        sequence.steps.append(CoordinationStep(
            timestamp=time.time(),
            name="Manager.enforce_lod_cap",
            status="success",
            duration_ms=duration,
            details={"lod_enforced": level.name if level else "UNKNOWN"},
        ))

        # STEP 3: If EMERGENCY, pause learning
        if level and level >= SurvivalLOD.EMERGENCY:
            t0 = time.perf_counter()
            self.organism.cognition.learning_loop.pause()
            duration = (time.perf_counter() - t0) * 1000
            sequence.steps.append(CoordinationStep(
                timestamp=time.time(),
                name="Manager.pause_learning",
                status="success",
                duration_ms=duration,
                details={"reason": "EMERGENCY mode"},
            ))
            logger.critical(
                f"[COORD] EMERGENCY: Learning paused due to {level.name}"
            )

        self.sequences.append(sequence)
        return sequence

    def get_audit_trail(self, limit: int = 50) -> List[CoordinationSequence]:
        """
        Return audit trail of recent coordination sequences.

        This is THE AUDIT TRAIL for organism behavior:
        "Here's exactly what happened when X occurred."

        Args:
            limit: Maximum number of sequences to return (most recent first)

        Returns:
            List of CoordinationSequence objects (newest first)
        """
        return list(reversed(self.sequences[-limit:]))

    def get_step_summary(self) -> dict[str, Any]:
        """Return summary statistics about coordination."""
        if not self.sequences:
            return {
                "total_sequences": 0,
                "total_duration_ms": 0.0,
                "avg_duration_ms": 0.0,
            }

        total_duration = sum(seq.total_duration_ms for seq in self.sequences)
        return {
            "total_sequences": len(self.sequences),
            "total_duration_ms": total_duration,
            "avg_duration_ms": total_duration / len(self.sequences),
            "recent_5": [seq.event_type for seq in self.sequences[-5:]],
        }
```

### Integration Into Awaken

```python
# cynic/organism/organism.py
def awaken(db_pool=None, registry=None) -> Organism:
    """Awaken the CYNIC organism with explicit coordinator."""
    awakener = _OrganismAwakener(db_pool, registry)

    # Phase 1-3: Create components (unchanged)
    awakener._create_components()
    svc = awakener._create_services()
    awakener._create_handler_registry(svc)
    awakener._build_container()
    awakener._wire_event_handlers()
    awakener._wire_perceive_workers()

    # Phase 4: CREATE COORDINATOR (NEW)
    organism = awakener._make_app_state()

    # Wire coordinator to handle major events
    from cynic.organism.coordinator import OrganismCoordinator

    coordinator = OrganismCoordinator(organism)
    organism._coordinator = coordinator  # Store reference

    # Subscribe coordinator to major events
    bus = get_core_bus()
    bus.on(CoreEvent.JUDGMENT_CREATED, coordinator.on_judgment_created)
    bus.on(CoreEvent.MEMORY_PRESSURE, coordinator.on_memory_pressure)
    # ... (add more coordination events)

    logger.info("Organism awakened with explicit coordinator")
    return organism
```

### How This Improves Integration Role

**Before:**
- 40+ handlers auto-registered (magic)
- Hard to trace "what happens after judgment?"
- Race conditions possible (async without ordering)
- Debugging requires grep + mental model

**After:**
- Coordinator makes all sequences explicit
- Single source of truth: `coordinator.on_judgment_created()`
- Ordered, testable, auditable
- Audit trail: `coordinator.get_audit_trail()`
- Score improves: 69 → 81

### Test Example

```python
# tests/test_coordinator.py
import pytest
from cynic.organism.organism import awaken
from cynic.organism.coordinator import OrganismCoordinator
from cynic.core.judgment import Judgment, Cell

@pytest.mark.asyncio
async def test_coordinator_judgment_sequence():
    """Verify coordinator executes all integration steps."""
    organism = awaken()
    coordinator = organism._coordinator

    # Create test judgment
    cell = Cell(
        cell_id="test_123",
        content="harmless code",
        reality="CODE",
        risk=0.1,
    )
    judgment = Judgment(
        cell=cell,
        q_score=75.0,
        verdict="WAG",
        confidence=0.618,
    )

    # Execute coordination
    sequence = await coordinator.on_judgment_created(judgment)

    # Verify all steps completed
    assert len(sequence.steps) == 5
    assert sequence.steps[0].name == "Consciousness.record_judgment"
    assert sequence.steps[0].status == "success"
    assert sequence.steps[1].name == "Learning.learn_cynic"
    assert sequence.steps[1].status == "success"
    assert sequence.steps[2].name == "Identity.track_axioms"
    assert sequence.steps[2].status == "success"
    assert sequence.steps[3].name == "Manager.update_escore"
    assert sequence.steps[3].status == "success"
    assert sequence.steps[4].name == "Integration.verify_convergence"
    assert sequence.steps[4].status == "success"

    # Verify state was updated
    assert organism.state.get_recent_judgments(1)[0]["judgment_id"] == "test_123"

    # Verify Q-table was updated
    assert organism.qtable.get("CODE:WAG", 0.0) > 0.0

    # Verify audit trail exists
    trail = coordinator.get_audit_trail()
    assert len(trail) == 1
    assert trail[0].event_type == "JUDGMENT_CREATED"
```

---

## SUMMARY OF IMPROVEMENTS

| Aspect | Before | After | Effort | Benefit |
|--------|--------|-------|--------|---------|
| **Manager Agency** | No methods to decide | `should_execute()`, `propose_level()` | 100 LOC | High (veto power) |
| **Integration Visibility** | Implicit handlers | Explicit `OrganismCoordinator` | 200 LOC | High (audit trail) |
| **Testing** | Hard to isolate | Can test coordinator sequences | — | High (better tests) |
| **Debugging** | Grep scattered handlers | Follow coordinator flow | — | High (clear path) |
| **Performance** | Same | Same (coordinator is thin wrapper) | — | Neutral |

**Total code additions: ~300 LOC**
**Effort: 4-6 hours**
**Score improvement: 7.5 → 8.3 (10% better)**

---

## FILES TO MODIFY

1. **cynic/organism/organism.py**
   - Add `should_execute()` method
   - Add `propose_level()` method
   - Add `_check_axioms_for_decision()` helper
   - Add `_check_axioms_for_level()` helper

2. **cynic/organism/coordinator.py** (NEW)
   - Create `OrganismCoordinator` class
   - Implement `on_judgment_created()`
   - Implement `on_memory_pressure()`
   - Add audit trail methods

3. **cynic/organism/__init__.py**
   - Export `OrganismCoordinator`

4. **tests/test_organism.py** (existing)
   - Add tests for `should_execute()`
   - Add tests for `propose_level()`

5. **tests/test_coordinator.py** (NEW)
   - Test coordinator sequences
   - Test audit trail
   - Test error handling

---

## DEPLOYMENT STRATEGY

**Phase 1 (Week 1): Manager Agency**
- Add `should_execute()` and `propose_level()` methods
- Wire into ACT phase and level selection
- Test with unit tests
- **No breaking changes** (backward compatible)

**Phase 2 (Week 2): Coordinator**
- Create `OrganismCoordinator`
- Wire into event handlers
- Collect audit trail
- **No breaking changes** (parallel to existing handlers)

**Phase 3 (Week 3): Migration**
- Gradually migrate handlers to coordinator
- Remove old handler subscriptions
- Full audit trail enabled
- **Clean up technical debt**

---

**End of Examples**
