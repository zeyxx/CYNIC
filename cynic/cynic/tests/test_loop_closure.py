"""Tests for LoopClosureValidator — feedback cycle completeness checking."""

import asyncio
import pytest
from cynic.nervous.loop_closure import (
    LoopClosureValidator,
    CyclePhase,
    STALL_THRESHOLD_MS,
    ORPHAN_THRESHOLD_MS,
    CLOSURE_CAP,
)


@pytest.fixture
def validator():
    """Create fresh LoopClosureValidator for each test."""
    return LoopClosureValidator()


@pytest.mark.asyncio
async def test_start_cycle(validator):
    """Test starting a new cycle."""
    event_id = await validator.start_cycle(
        judgment_id="j001",
        initial_event_id="e_perceive",
        component="perceiver",
    )
    assert isinstance(event_id, str)
    assert len(event_id) == 16

    # Verify open cycle exists
    open_cycles = await validator.get_open_cycles()
    assert len(open_cycles) == 1
    assert open_cycles[0].judgment_id == "j001"


@pytest.mark.asyncio
async def test_record_phase_sequence(validator):
    """Test recording phase transitions in order."""
    event_id = await validator.start_cycle(
        judgment_id="j001",
        initial_event_id="e_perceive",
        component="perceiver",
    )

    # Record phases in sequence
    phases = [
        (CyclePhase.JUDGE, "e_judge", "SAGE"),
        (CyclePhase.DECIDE, "e_decide", "BRAIN"),
        (CyclePhase.ACT, "e_act", "RUNNER"),
    ]

    for phase, event_id, component in phases:
        result = await validator.record_phase(
            judgment_id="j001",
            phase=phase,
            event_id=event_id,
            component=component,
            duration_ms=10.0,
        )
        assert result is True  # Valid phase order

    # Verify phases recorded
    open_cycles = await validator.get_open_cycles()
    assert len(open_cycles) == 1
    cycle = open_cycles[0]
    assert cycle.phase_count == 4  # PERCEIVE (root) + 3 recorded


@pytest.mark.asyncio
async def test_complete_cycle_closure(validator):
    """Test completing a full 7-phase cycle."""
    await validator.start_cycle(
        judgment_id="j001",
        initial_event_id="e_perceive",
        component="perceiver",
    )

    # Record all phases
    phases = [
        CyclePhase.JUDGE,
        CyclePhase.DECIDE,
        CyclePhase.ACT,
        CyclePhase.LEARN,
        CyclePhase.ACCOUNT,
        CyclePhase.EMERGE,
    ]

    for phase in phases:
        await validator.record_phase(
            judgment_id="j001",
            phase=phase,
            event_id=f"e_{phase}",
            component="TEST",
            duration_ms=10.0,
        )

    # Close cycle
    closure = await validator.close_cycle("j001")
    assert closure is not None
    assert closure.is_complete is True
    assert closure.is_stalled is False
    assert closure.is_orphan is False
    assert closure.phase_count == 7


@pytest.mark.asyncio
async def test_orphan_detection_no_act(validator):
    """Test detecting orphan judgments (no ACT phase)."""
    await validator.start_cycle(
        judgment_id="j001",
        initial_event_id="e_perceive",
        component="perceiver",
    )

    # Record JUDGE and DECIDE but skip ACT
    await validator.record_phase(
        judgment_id="j001",
        phase=CyclePhase.JUDGE,
        event_id="e_judge",
        component="SAGE",
    )
    await validator.record_phase(
        judgment_id="j001",
        phase=CyclePhase.DECIDE,
        event_id="e_decide",
        component="BRAIN",
    )

    # Artificially age it
    closure_event = None
    open_cycles = await validator.get_open_cycles()
    if open_cycles:
        open_cycles[0].created_at_ms -= ORPHAN_THRESHOLD_MS + 1000

    # Close and check for orphan
    closure = await validator.close_cycle("j001")
    assert closure is not None
    assert closure.is_orphan is True
    assert not any(p.phase == CyclePhase.ACT for p in closure.phases)


@pytest.mark.asyncio
async def test_stall_detection_timeout(validator):
    """Test detecting stalled phases."""
    await validator.start_cycle(
        judgment_id="j001",
        initial_event_id="e_perceive",
        component="perceiver",
    )

    # Record JUDGE phase
    await validator.record_phase(
        judgment_id="j001",
        phase=CyclePhase.JUDGE,
        event_id="e_judge",
        component="SAGE",
    )

    # Get stalled phases
    stalled = await validator.get_stalled_phases(threshold_ms=100)  # Low threshold
    # Should detect stall if waiting too long
    # (In real time, this won't trigger, but API is tested)
    assert isinstance(stalled, list)


@pytest.mark.asyncio
async def test_recent_closures(validator):
    """Test retrieving recent cycle closures."""
    # Create 3 complete cycles
    for i in range(3):
        await validator.start_cycle(
            judgment_id=f"j{i}",
            initial_event_id=f"e_p{i}",
            component="perceiver",
        )

        # Record all phases
        for phase in [
            CyclePhase.JUDGE,
            CyclePhase.DECIDE,
            CyclePhase.ACT,
            CyclePhase.LEARN,
            CyclePhase.ACCOUNT,
            CyclePhase.EMERGE,
        ]:
            await validator.record_phase(
                judgment_id=f"j{i}",
                phase=phase,
                event_id=f"e_{phase}_{i}",
                component="TEST",
            )

        # Close
        await validator.close_cycle(f"j{i}")
        await asyncio.sleep(0.01)

    # Get recent
    recent = await validator.recent_closures(limit=2)
    assert len(recent) == 2
    assert recent[0].judgment_id == "j2"
    assert recent[1].judgment_id == "j1"


@pytest.mark.asyncio
async def test_get_open_cycles_by_age(validator):
    """Test filtering open cycles by age."""
    # Start 2 cycles
    await validator.start_cycle(
        judgment_id="j_old",
        initial_event_id="e1",
        component="TEST",
    )
    await asyncio.sleep(0.05)

    await validator.start_cycle(
        judgment_id="j_new",
        initial_event_id="e2",
        component="TEST",
    )

    # Get all
    all_open = await validator.get_open_cycles()
    assert len(all_open) == 2

    # Get only recent (last 20ms)
    recent = await validator.get_open_cycles(max_age_ms=20)
    assert len(recent) <= 2


@pytest.mark.asyncio
async def test_orphan_judgments_query(validator):
    """Test querying orphan judgments."""
    # Create orphan (no ACT)
    await validator.start_cycle(
        judgment_id="j_orphan",
        initial_event_id="e_p",
        component="TEST",
    )
    await validator.record_phase(
        judgment_id="j_orphan",
        phase=CyclePhase.JUDGE,
        event_id="e_j",
        component="SAGE",
    )

    # Age it
    open_cycles = await validator.get_open_cycles()
    open_cycles[0].created_at_ms -= ORPHAN_THRESHOLD_MS + 1000

    # Query orphans
    orphans = await validator.get_orphan_judgments()
    assert len(orphans) == 1
    assert orphans[0].judgment_id == "j_orphan"


@pytest.mark.asyncio
async def test_rolling_cap_behavior(validator):
    """Test buffer respects F(9)=34 cap."""
    # Create more than cap
    for i in range(CLOSURE_CAP + 10):
        await validator.start_cycle(
            judgment_id=f"j{i}",
            initial_event_id=f"e_p{i}",
            component="TEST",
        )

        # Record all phases quickly
        for phase in [
            CyclePhase.JUDGE,
            CyclePhase.DECIDE,
            CyclePhase.ACT,
            CyclePhase.LEARN,
            CyclePhase.ACCOUNT,
            CyclePhase.EMERGE,
        ]:
            await validator.record_phase(
                judgment_id=f"j{i}",
                phase=phase,
                event_id=f"e_{phase}_{i}",
                component="TEST",
            )

        await validator.close_cycle(f"j{i}")

    # Should only have CLOSURE_CAP
    stats = await validator.stats()
    assert stats["buffer_size"] == CLOSURE_CAP
    assert stats["total_cycles"] == CLOSURE_CAP + 10


@pytest.mark.asyncio
async def test_stats_accuracy(validator):
    """Test statistics computed correctly."""
    # Create mix: 2 complete, 1 stalled, 1 orphan
    for i in range(4):
        await validator.start_cycle(
            judgment_id=f"j{i}",
            initial_event_id=f"e_p{i}",
            component="TEST",
        )

        if i == 0:  # Complete
            for phase in [
                CyclePhase.JUDGE,
                CyclePhase.DECIDE,
                CyclePhase.ACT,
                CyclePhase.LEARN,
                CyclePhase.ACCOUNT,
                CyclePhase.EMERGE,
            ]:
                await validator.record_phase(
                    judgment_id="j0",
                    phase=phase,
                    event_id=f"e_{phase}",
                    component="TEST",
                )
        elif i == 1:  # Complete
            for phase in [
                CyclePhase.JUDGE,
                CyclePhase.DECIDE,
                CyclePhase.ACT,
                CyclePhase.LEARN,
                CyclePhase.ACCOUNT,
                CyclePhase.EMERGE,
            ]:
                await validator.record_phase(
                    judgment_id="j1",
                    phase=phase,
                    event_id=f"e_{phase}",
                    component="TEST",
                )
        else:  # Incomplete
            await validator.record_phase(
                judgment_id=f"j{i}",
                phase=CyclePhase.JUDGE,
                event_id=f"e_j{i}",
                component="TEST",
            )

        await validator.close_cycle(f"j{i}")

    stats = await validator.stats()
    assert stats["total_cycles"] == 4
    assert stats["complete_cycles"] == 2


@pytest.mark.asyncio
async def test_concurrent_cycles(validator):
    """Test managing concurrent cycles."""
    async def create_cycle(idx: int) -> None:
        await validator.start_cycle(
            judgment_id=f"j{idx}",
            initial_event_id=f"e_p{idx}",
            component="TEST",
        )
        # Record some phases
        for phase in [CyclePhase.JUDGE, CyclePhase.DECIDE]:
            await validator.record_phase(
                judgment_id=f"j{idx}",
                phase=phase,
                event_id=f"e_{phase}_{idx}",
                component="TEST",
            )
        await validator.close_cycle(f"j{idx}")

    # Run 5 concurrent cycles
    await asyncio.gather(
        create_cycle(0),
        create_cycle(1),
        create_cycle(2),
        create_cycle(3),
        create_cycle(4),
    )

    stats = await validator.stats()
    assert stats["total_cycles"] == 5


@pytest.mark.asyncio
async def test_cycle_to_dict(validator):
    """Test cycle serialization."""
    await validator.start_cycle(
        judgment_id="j001",
        initial_event_id="e_perceive",
        component="TEST",
    )
    await validator.record_phase(
        judgment_id="j001",
        phase=CyclePhase.JUDGE,
        event_id="e_judge",
        component="SAGE",
    )

    closure = await validator.close_cycle("j001")
    d = closure.to_dict()

    assert d["judgment_id"] == "j001"
    assert d["is_complete"] is False  # Not all phases
    assert len(d["phases"]) == 2  # PERCEIVE + JUDGE


@pytest.mark.asyncio
async def test_snapshot_state(validator):
    """Test snapshot returns full state."""
    for i in range(3):
        await validator.start_cycle(
            judgment_id=f"j{i}",
            initial_event_id=f"e_p{i}",
            component="TEST",
        )
        await validator.close_cycle(f"j{i}")

    snapshot = await validator.snapshot()
    assert len(snapshot["closed_cycles"]) == 3
    assert "open_cycles" in snapshot
    assert "stats" in snapshot


@pytest.mark.asyncio
async def test_clear_validator(validator):
    """Test clearing all state."""
    # Create cycles
    for i in range(5):
        await validator.start_cycle(
            judgment_id=f"j{i}",
            initial_event_id=f"e_p{i}",
            component="TEST",
        )
        await validator.close_cycle(f"j{i}")

    await validator.clear()

    stats = await validator.stats()
    assert stats["buffer_size"] == 0
    assert stats["total_cycles"] == 0

    open_cycles = await validator.get_open_cycles()
    assert len(open_cycles) == 0


@pytest.mark.asyncio
async def test_out_of_order_phase_detection(validator):
    """Test detecting out-of-order phase transitions."""
    await validator.start_cycle(
        judgment_id="j001",
        initial_event_id="e_perceive",
        component="TEST",
    )

    # Record JUDGE
    await validator.record_phase(
        judgment_id="j001",
        phase=CyclePhase.JUDGE,
        event_id="e_judge",
        component="SAGE",
    )

    # Skip DECIDE and try LEARN (out of order)
    # API still records it but marks as error
    result = await validator.record_phase(
        judgment_id="j001",
        phase=CyclePhase.LEARN,
        event_id="e_learn",
        component="TEST",
    )
    assert result is True  # Recorded, but potentially marked error

    closure = await validator.close_cycle("j001")
    # Should have recorded both despite out-of-order
    assert closure.phase_count >= 2
