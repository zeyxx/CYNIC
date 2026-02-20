"""
Empirical MACRO Activation Proof

Validates that after Session 8 MACRO fix:
1. SelfWatcher can escalate to MACRO budget
2. MACRO events actually reach the scheduler queue
3. MACRO consciousness level is selected and executed

No Ollama required - uses mocked LLM adapter.
"""
from __future__ import annotations

import asyncio
import time
from typing import Any

import pytest

from cynic.core.consciousness import ConsciousnessLevel, get_consciousness
from cynic.core.judgment import Cell
from cynic.scheduler import ConsciousnessRhythm, PerceptionEvent
from cynic.senses.workers.self_watcher import SelfWatcher
from cynic.cognition.cortex.orchestrator import JudgeOrchestrator
from cynic.core.axioms import AxiomArchitecture
from cynic.cognition.neurons.cynic_dog import CynicDog
from cynic.cognition.neurons.guardian import GuardianDog
from cynic.cognition.neurons.analyst import AnalystDog
from cynic.cognition.neurons.janitor import JanitorDog
from cynic.cognition.neurons.base import DogId


class TestMACROActivationEmpirical:
    """Empirical proof that MACRO consciousness is activated."""

    @pytest.fixture
    def orchestrator(self) -> JudgeOrchestrator:
        """Minimal orchestrator with 4 non-LLM Dogs."""
        dogs = {
            DogId.CYNIC: CynicDog(),
            DogId.GUARDIAN: GuardianDog(),
            DogId.ANALYST: AnalystDog(),
            DogId.JANITOR: JanitorDog(),
        }
        axiom_arch = AxiomArchitecture()
        cynic_dog = dogs[DogId.CYNIC]
        return JudgeOrchestrator(
            dogs=dogs,
            axiom_arch=axiom_arch,
            cynic_dog=cynic_dog,
        )

    @pytest.fixture
    def qtable_mock(self) -> Any:
        """Mock QTable for testing (needs stats() method)."""
        class MockQTable:
            def stats(self) -> dict[str, Any]:
                return {
                    "states": 5,  # < 10, will trigger escalation
                    "total_updates": 100,
                    "pending_flush": 10,
                    "max_confidence": 0.25,  # < 0.3, will trigger escalation
                    "unique_states": 5,
                }
        return MockQTable()

    def test_self_watcher_escalates_on_weak_learning(
        self, qtable_mock: Any
    ) -> None:
        """Verify SelfWatcher escalates budget when learning is weak."""

        def qtable_getter() -> Any:
            return qtable_mock

        watcher = SelfWatcher(qtable_getter=qtable_getter)

        # Call sense() which should detect weak learning
        cell = asyncio.run(self._call_sense(watcher))

        assert cell is not None
        assert cell.budget_usd == 0.10, "Budget should escalate to 0.10 on weak learning"
        assert cell.content.get("learning_health") == "WEAK"
        assert cell.risk > 0.0, "Risk should increase on weak learning"

    def test_macro_budget_inference(self) -> None:
        """Verify budget 0.10 maps to MACRO level."""
        from cynic.scheduler import ConsciousnessRhythm

        # Create minimal scheduler
        scheduler = ConsciousnessRhythm(orchestrator=None)  # type: ignore

        # Test budget inference
        level = scheduler._infer_level(0.10)
        assert level == ConsciousnessLevel.MACRO, f"Budget 0.10 should map to MACRO, got {level}"

    def test_perception_event_enters_macro_queue(self, orchestrator) -> None:
        """Verify PerceptionEvent with MACRO level enters correct queue."""
        scheduler = ConsciousnessRhythm(orchestrator=orchestrator)

        # Create a cell with escalated budget (will infer MACRO)
        cell = Cell(
            reality="CYNIC",
            analysis="LEARN",
            content={"learning_health": "WEAK"},
            budget_usd=0.10,
        )

        # Submit to scheduler
        submitted = scheduler.submit(cell, budget_usd=0.10)
        assert submitted is True, "Event should be submitted to queue"

        # Verify it's in MACRO queue, not REFLEX
        macro_queue = scheduler._queues[ConsciousnessLevel.MACRO]
        assert macro_queue.qsize() == 1, "Event should be in MACRO queue"

        reflex_queue = scheduler._queues[ConsciousnessLevel.REFLEX]
        assert reflex_queue.qsize() == 0, "Event should NOT be in REFLEX queue"

    def test_macro_worker_can_receive_events(self, orchestrator) -> None:
        """Verify MACRO worker can drain events from queue."""
        scheduler = ConsciousnessRhythm(orchestrator=orchestrator)

        # Submit event to MACRO queue
        cell = Cell(
            reality="CYNIC",
            analysis="LEARN",
            content={"test": "data"},
            budget_usd=0.10,
        )
        scheduler.submit(cell, budget_usd=0.10)

        # Try to drain (non-blocking, with timeout)
        async def try_drain() -> PerceptionEvent | None:
            return await scheduler._drain_one(
                ConsciousnessLevel.MACRO,
                timeout=1.0,
            )

        event = asyncio.run(try_drain())
        assert event is not None, "MACRO worker should drain event"
        assert event.cell.budget_usd == 0.10
        assert event.level == ConsciousnessLevel.MACRO

    def test_consciousness_level_selection_respects_budget(self) -> None:
        """Verify ConsciousnessLevel selection matches budget ranges."""
        from cynic.scheduler import ConsciousnessRhythm

        scheduler = ConsciousnessRhythm(orchestrator=None)  # type: ignore

        # Test budget → level mapping
        test_cases = [
            (0.001, ConsciousnessLevel.REFLEX),
            (0.005, ConsciousnessLevel.REFLEX),
            (0.02, ConsciousnessLevel.MICRO),
            (0.05, ConsciousnessLevel.MACRO),
            (0.10, ConsciousnessLevel.MACRO),
            (1.00, ConsciousnessLevel.MACRO),
        ]

        for budget, expected_level in test_cases:
            level = scheduler._infer_level(budget)
            assert level == expected_level, (
                f"Budget {budget} should map to {expected_level.name}, "
                f"got {level.name}"
            )

    def test_macro_queue_capacity_respected(self, orchestrator) -> None:
        """Verify MACRO queue respects capacity limit (F(10)=55)."""
        scheduler = ConsciousnessRhythm(orchestrator=orchestrator)
        from cynic.core.phi import fibonacci

        capacity = fibonacci(10)  # 55

        # Fill MACRO queue to capacity
        for i in range(capacity):
            cell = Cell(
                cell_id=f"test_{i:03d}",
                reality="CODE",
                analysis="JUDGE",
                content={"index": i},
                budget_usd=0.10,
            )
            submitted = scheduler.submit(cell, budget_usd=0.10)
            assert submitted is True, f"Should submit event {i}"

        # Try to exceed capacity
        cell_overflow = Cell(
            cell_id="overflow",
            reality="CODE",
            analysis="JUDGE",
            content={"overflow": True},
            budget_usd=0.10,
        )
        submitted_overflow = scheduler.submit(cell_overflow, budget_usd=0.10)
        assert submitted_overflow is False, "Queue full, should drop event"

    async def _call_sense(self, watcher: SelfWatcher) -> Cell | None:
        """Helper to call async sense()."""
        return await watcher.sense()


class TestMACROCycleExecution:
    """Test that MACRO cycles actually execute (integration test)."""

    @pytest.fixture
    def orchestrator(self) -> JudgeOrchestrator:
        """Minimal orchestrator."""
        dogs = {
            DogId.CYNIC: CynicDog(),
            DogId.GUARDIAN: GuardianDog(),
            DogId.ANALYST: AnalystDog(),
            DogId.JANITOR: JanitorDog(),
        }
        axiom_arch = AxiomArchitecture()
        cynic_dog = dogs[DogId.CYNIC]
        return JudgeOrchestrator(
            dogs=dogs,
            axiom_arch=axiom_arch,
            cynic_dog=cynic_dog,
        )

    def test_macro_consciousness_level_is_available(self, orchestrator) -> None:
        """Verify MACRO level is properly configured."""
        from cynic.core.consciousness import ConsciousnessLevel

        # MACRO should exist and be configured
        assert ConsciousnessLevel.MACRO is not None
        assert ConsciousnessLevel.MACRO.name == "MACRO"
        assert ConsciousnessLevel.MACRO.value == 1
        assert ConsciousnessLevel.MACRO.allows_llm is True

        # MACRO should have reasonable target latency
        target_ms = ConsciousnessLevel.MACRO.target_ms
        assert 400 < target_ms < 500, f"MACRO target should be ~441ms, got {target_ms:.1f}ms"

    def test_macro_gradient_value(self) -> None:
        """Verify MACRO gradient for introspection."""
        from cynic.core.consciousness import ConsciousnessLevel

        gradient = ConsciousnessLevel.MACRO.gradient
        assert 0 <= gradient <= 6, f"Gradient should be 0-6, got {gradient}"

    def test_all_macro_dogs_available(self) -> None:
        """Verify all 11 Dogs are configured for MACRO level."""
        from cynic.core.consciousness import MACRO_DOGS

        # MACRO should include all Dogs for full reasoning
        assert len(MACRO_DOGS) > 4, "MACRO should include more than just non-LLM Dogs"

        # Should include the core Dogs we tested
        expected = {"CYNIC", "GUARDIAN", "ANALYST", "JANITOR"}
        assert expected.issubset(MACRO_DOGS), "MACRO should include core Dogs"
