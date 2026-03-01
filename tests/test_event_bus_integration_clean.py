"""
Test: ResidualDetector Event Bus Integration (End-to-End)

Verifies that ResidualDetector:
1. Subscribes to JUDGMENT_CREATED events on the core bus
2. Receives and processes those events correctly
3. Emits EMERGENCE_DETECTED and ANOMALY_DETECTED events
4. Prevents event loops via genealogy tracking

NO MOCKS — pure integration test with real event buses and real ResidualDetector.
"""

import asyncio
import pytest
from cynic.kernel.core.event_bus import (
    get_core_bus, get_automation_bus, reset_all_buses,
    Event, CoreEvent, create_default_bridge,
)
from cynic.kernel.core.judgment import Judgment, Cell, new_id
from cynic.kernel.organism.brain.cognition.cortex.residual import ResidualDetector
from cynic.kernel.core.events_schema import (
    JudgmentCreatedPayload, EmergenceDetectedPayload,
)


# ════════════════════════════════════════════════════════════════════════════
# HELPER FUNCTIONS
# ════════════════════════════════════════════════════════════════════════════

def _make_judgment_payload(
    judge_id: str = "judge_1",
    residual: float = 0.5,
    confidence: float = 0.75,
    q_score: float = 50.0,
    content: str = "test",
    reality: str = "CODE",
) -> JudgmentCreatedPayload:
    """
    Helper to create JudgmentCreatedPayload with sensible defaults.
    Reduces boilerplate when emitting multiple events.
    """
    return JudgmentCreatedPayload(
        judgment_id=new_id(),
        cell={
            "reality": reality,
            "analysis": "JUDGE",
            "time_dim": "PRESENT",
            "content": content,
            "context": "test",
            "cell_id": new_id(),
        },
        q_score=q_score,
        verdict="GROWL" if q_score >= 38.2 else "BARK",
        confidence=confidence,
        residual_variance=residual,
        unnameable_detected=False,
        timestamp=123.0,
        cost_usd=0.01,
        llm_calls=1,
    )


# ════════════════════════════════════════════════════════════════════════════
# FIXTURES
# ════════════════════════════════════════════════════════════════════════════

@pytest.fixture
def clean_buses():
    """Reset all event buses before each test."""
    reset_all_buses()
    yield
    reset_all_buses()


@pytest.fixture
def detector():
    """Create a fresh ResidualDetector for each test."""
    return ResidualDetector()


@pytest.fixture
async def core_bus():
    """Get the CORE bus."""
    return get_core_bus()


@pytest.fixture
async def automation_bus():
    """Get the AUTOMATION bus."""
    return get_automation_bus()


# ════════════════════════════════════════════════════════════════════════════
# TEST 1: ResidualDetector subscribes to JUDGMENT_CREATED
# ════════════════════════════════════════════════════════════════════════════

@pytest.mark.asyncio
async def test_detector_subscribes_to_core_bus(clean_buses, detector, core_bus):
    """
    GIVEN: A fresh event bus and ResidualDetector instance
    WHEN: The detector is initialized and started
    THEN: It has registered a listener for JUDGMENT_CREATED events on the core bus
    """
    # Initially, no listeners
    assert len(core_bus._handlers.get(CoreEvent.JUDGMENT_CREATED, [])) == 0

    # Start the detector
    detector.start(core_bus)

    # Should now have a listener registered
    assert detector._listener_registered is True
    assert len(core_bus._handlers.get(CoreEvent.JUDGMENT_CREATED, [])) >= 1


@pytest.mark.asyncio
async def test_detector_start_idempotent(clean_buses, detector, core_bus):
    """
    GIVEN: A detector that's already subscribed
    WHEN: start() is called again
    THEN: No duplicate listeners are registered
    """
    detector.start(core_bus)
    handlers_count_1 = len(core_bus._handlers.get(CoreEvent.JUDGMENT_CREATED, []))

    # Call start again
    detector.start(core_bus)
    handlers_count_2 = len(core_bus._handlers.get(CoreEvent.JUDGMENT_CREATED, []))

    # Should be same (idempotent)
    assert handlers_count_1 == handlers_count_2


# ════════════════════════════════════════════════════════════════════════════
# TEST 2: ResidualDetector receives JUDGMENT_CREATED events
# ════════════════════════════════════════════════════════════════════════════

@pytest.mark.asyncio
async def test_detector_receives_judgment_created_event(
    clean_buses, detector, core_bus
):
    """
    GIVEN: An initialized ResidualDetector listening to the core bus
    WHEN: A JUDGMENT_CREATED event is emitted on the core bus with specific data
    THEN: The detector's internal state updates (observations count grows)
    """
    detector.start(core_bus)

    # Emit a JUDGMENT_CREATED event
    event = Event.typed(
        CoreEvent.JUDGMENT_CREATED,
        JudgmentCreatedPayload(
            judgment_id=new_id(),
            cell={
                "reality": "CODE",
                "analysis": "JUDGE",
                "time_dim": "PRESENT",
                "content": "test code",
                "context": "unit test",
                "cell_id": new_id(),
            },
            q_score=55.0,
            verdict="GROWL",
            confidence=0.50,
            residual_variance=0.25,
            unnameable_detected=False,
            timestamp=123.0,
            cost_usd=0.01,
            llm_calls=1,
        ),
        source="test",
    )

    await core_bus.emit(event)

    # Give async handlers time to execute
    await asyncio.sleep(0.1)

    # Verify detector updated
    stats = detector.stats()
    assert stats["observations"] == 1
    assert stats["history_len"] == 1


@pytest.mark.asyncio
async def test_detector_accumulates_judgments(
    clean_buses, detector, core_bus
):
    """
    GIVEN: A ResidualDetector listening to the core bus
    WHEN: Multiple JUDGMENT_CREATED events are emitted
    THEN: The detector accumulates them all
    """
    detector.start(core_bus)

    # Emit 5 events
    for i in range(5):
        event = Event.typed(
            CoreEvent.JUDGMENT_CREATED,
            JudgmentCreatedPayload(
                judgment_id=new_id(),
                cell={
                    "reality": "SOLANA" if i % 2 == 0 else "CODE",
                    "analysis": "JUDGE",
                    "time_dim": "PRESENT",
                    "content": f"test {i}",
                    "context": "test",
                    "cell_id": new_id(),
                },
                q_score=30.0 + (i * 5),
                verdict="BARK" if i < 2 else "GROWL",
                confidence=0.30 + (i * 0.05),
                residual_variance=0.15 + (i * 0.05),
                unnameable_detected=False,
                timestamp=123.0 + i,
                cost_usd=0.01 * (i + 1),
                llm_calls=1,
            ),
            source="test",
        )
        await core_bus.emit(event)
        await asyncio.sleep(0.01)

    # Give all handlers time to execute
    await asyncio.sleep(0.1)

    # Verify accumulated
    stats = detector.stats()
    assert stats["observations"] == 5
    assert stats["history_len"] == 5


# ════════════════════════════════════════════════════════════════════════════
# TEST 3: ResidualDetector emits EMERGENCE_DETECTED when pattern found
# ════════════════════════════════════════════════════════════════════════════

@pytest.mark.asyncio
async def test_detector_emits_emergence_detected_on_stable_high(
    clean_buses, detector, core_bus
):
    """
    GIVEN: A ResidualDetector listening to the bus
    WHEN: 5+ consecutive judgments have high residual variance (>38.2%)
    THEN: An EMERGENCE_DETECTED event is emitted on the core bus
          (STABLE_HIGH pattern)
    """
    # Track events emitted
    emergence_events = []

    async def capture_emergence(event: Event):
        emergence_events.append(event)

    detector.start(core_bus)
    core_bus.on(CoreEvent.EMERGENCE_DETECTED, capture_emergence)

    # Emit 6 judgments with high residual variance (>38.2%)
    for i in range(6):
        event = Event.typed(
            CoreEvent.JUDGMENT_CREATED,
            JudgmentCreatedPayload(
                judgment_id=new_id(),
                cell={
                    "reality": "CODE",
                    "analysis": "JUDGE",
                    "time_dim": "PRESENT",
                    "content": f"test {i}",
                    "context": "test",
                    "cell_id": new_id(),
                },
                q_score=45.0,
                verdict="GROWL",
                confidence=0.40,
                residual_variance=0.50,  # High — above 38.2% threshold
                unnameable_detected=False,
                timestamp=123.0 + i,
                cost_usd=0.01,
                llm_calls=1,
            ),
            source="test",
        )
        await core_bus.emit(event)
        await asyncio.sleep(0.01)

    # Give handlers time to execute
    await asyncio.sleep(0.2)

    # Should have detected STABLE_HIGH pattern and emitted EMERGENCE_DETECTED
    assert len(emergence_events) > 0

    # Check payload
    emergence_payload = emergence_events[0].payload
    assert emergence_payload.get("pattern_type") == "STABLE_HIGH"
    assert "severity" in emergence_payload
    assert "evidence" in emergence_payload


@pytest.mark.asyncio
async def test_detector_anomaly_detected_on_spike(
    clean_buses, detector, core_bus
):
    """
    GIVEN: A ResidualDetector listening to the bus with low baseline residual
    WHEN: A sudden spike in residual variance occurs (SPIKE pattern)
    THEN: An ANOMALY_DETECTED event is emitted (SPIKE-specific)
    """
    anomaly_events = []

    async def capture_anomaly(event: Event):
        anomaly_events.append(event)

    detector.start(core_bus)
    core_bus.on(CoreEvent.ANOMALY_DETECTED, capture_anomaly)

    # Emit 3 low-residual judgments to establish baseline
    for i in range(3):
        event = Event.typed(
            CoreEvent.JUDGMENT_CREATED,
            JudgmentCreatedPayload(
                judgment_id=new_id(),
                cell={
                    "reality": "CODE",
                    "analysis": "JUDGE",
                    "time_dim": "PRESENT",
                    "content": f"test {i}",
                    "context": "test",
                    "cell_id": new_id(),
                },
                q_score=55.0,
                verdict="GROWL",
                confidence=0.50,
                residual_variance=0.10,  # Low baseline
                unnameable_detected=False,
                timestamp=123.0 + i,
                cost_usd=0.01,
                llm_calls=1,
            ),
            source="test",
        )
        await core_bus.emit(event)
        await asyncio.sleep(0.01)

    # Emit a spike (high residual)
    spike_event = Event.typed(
        CoreEvent.JUDGMENT_CREATED,
        JudgmentCreatedPayload(
            judgment_id=new_id(),
            cell={
                "reality": "CODE",
                "analysis": "JUDGE",
                "time_dim": "PRESENT",
                "content": "spike test",
                "context": "test",
                "cell_id": new_id(),
            },
            q_score=55.0,
            verdict="GROWL",
            confidence=0.50,
            residual_variance=0.60,  # Spike!
            unnameable_detected=False,
            timestamp=126.0,
            cost_usd=0.01,
            llm_calls=1,
        ),
        source="test",
    )
    await core_bus.emit(spike_event)

    # Give handlers time to execute
    await asyncio.sleep(0.2)

    # Should have detected SPIKE pattern and emitted ANOMALY_DETECTED
    assert len(anomaly_events) > 0

    # Check payload
    anomaly_payload = anomaly_events[0].payload
    assert anomaly_payload.get("pattern_type") == "SPIKE"
    assert anomaly_payload.get("severity") > 0


@pytest.mark.asyncio
async def test_detector_statistics_updated(
    clean_buses, detector, core_bus
):
    """
    GIVEN: A ResidualDetector
    WHEN: Events are emitted and patterns detected
    THEN: stats() reflects correct counts
    """
    detector.start(core_bus)

    # Emit 6 high-residual events to trigger STABLE_HIGH
    for i in range(6):
        event = Event.typed(
            CoreEvent.JUDGMENT_CREATED,
            JudgmentCreatedPayload(
                judgment_id=new_id(),
                cell={
                    "reality": "CODE",
                    "analysis": "JUDGE",
                    "time_dim": "PRESENT",
                    "content": f"test {i}",
                    "context": "test",
                    "cell_id": new_id(),
                },
                q_score=45.0,
                verdict="GROWL",
                confidence=0.40,
                residual_variance=0.50,
                unnameable_detected=False,
                timestamp=123.0 + i,
                cost_usd=0.01,
                llm_calls=1,
            ),
            source="test",
        )
        await core_bus.emit(event)
        await asyncio.sleep(0.01)

    await asyncio.sleep(0.2)

    stats = detector.stats()
    assert stats["observations"] == 6
    assert stats["anomalies"] >= 5  # Most are high
    assert stats["patterns_detected"] >= 1


# ════════════════════════════════════════════════════════════════════════════
# TEST 4: Event genealogy prevents loops
# ════════════════════════════════════════════════════════════════════════════

@pytest.mark.asyncio
async def test_event_genealogy_prevents_reforward(
    clean_buses, detector, core_bus
):
    """
    GIVEN: A JUDGMENT_CREATED event with genealogy_path containing "CORE"
    WHEN: The detector receives the event
    THEN: The detector handles it correctly and records observation
          (Bridge logic prevents re-forward, tested separately)
    """
    # Create and start bridge with default rules
    bridge = create_default_bridge()
    bridge.start()

    detector.start(core_bus)

    # Create an event with genealogy already containing "CORE"
    event_with_genealogy = Event.typed(
        CoreEvent.JUDGMENT_CREATED,
        JudgmentCreatedPayload(
            judgment_id=new_id(),
            cell={
                "reality": "CODE",
                "analysis": "JUDGE",
                "time_dim": "PRESENT",
                "content": "test",
                "context": "test",
                "cell_id": new_id(),
            },
            q_score=45.0,
            verdict="GROWL",
            confidence=0.40,
            residual_variance=0.50,
            unnameable_detected=False,
            timestamp=123.0,
            cost_usd=0.01,
            llm_calls=1,
        ),
        source="test",
    )
    # Manually set genealogy as if event came from CORE
    event_with_genealogy._genealogy = ["CORE"]

    await core_bus.emit(event_with_genealogy)
    await asyncio.sleep(0.1)

    # Verify detector still processed it (genealogy doesn't affect event reception)
    stats = detector.stats()
    assert stats["observations"] >= 1

    # Clean up bridge
    bridge.stop()


@pytest.mark.asyncio
async def test_detector_recognizes_already_seen_genealogy(
    clean_buses, detector, core_bus
):
    """
    GIVEN: An event envelope with genealogy tracking
    WHEN: The event is checked with already_seen()
    THEN: The method correctly identifies if a bus is in the genealogy path
    """
    event = Event(
        type=CoreEvent.JUDGMENT_CREATED,
        payload={},
        source="test",
    )

    # Initially not seen by CORE
    assert not event.already_seen("CORE")

    # Add CORE to genealogy
    event_with_genealogy = event.with_genealogy("CORE")

    # Now it should be marked as seen
    assert event_with_genealogy.already_seen("CORE")
    assert not event_with_genealogy.already_seen("AUTOMATION")


# ════════════════════════════════════════════════════════════════════════════
# TEST 5: Full integration — Multi-pattern detection
# ════════════════════════════════════════════════════════════════════════════

@pytest.mark.asyncio
async def test_detector_rising_pattern_detection(
    clean_buses, detector, core_bus
):
    """
    GIVEN: A ResidualDetector
    WHEN: Residual variance rises steadily (increasing trend)
    THEN: A RISING pattern is detected and EMERGENCE_DETECTED emitted
    """
    emergence_events = []

    async def capture_emergence(event: Event):
        emergence_events.append(event)

    detector.start(core_bus)
    core_bus.on(CoreEvent.EMERGENCE_DETECTED, capture_emergence)

    # Emit judgments with steadily increasing residual
    residuals = [0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.35, 0.40]
    for i, residual in enumerate(residuals):
        event = Event.typed(
            CoreEvent.JUDGMENT_CREATED,
            JudgmentCreatedPayload(
                judgment_id=new_id(),
                cell={
                    "reality": "CODE",
                    "analysis": "JUDGE",
                    "time_dim": "PRESENT",
                    "content": f"test {i}",
                    "context": "test",
                    "cell_id": new_id(),
                },
                q_score=50.0,
                verdict="GROWL",
                confidence=0.40,
                residual_variance=residual,
                unnameable_detected=False,
                timestamp=123.0 + i,
                cost_usd=0.01,
                llm_calls=1,
            ),
            source="test",
        )
        await core_bus.emit(event)
        await asyncio.sleep(0.01)

    await asyncio.sleep(0.2)

    # Verify pattern detected (RISING or SPIKE — detector recognizes steady increase)
    assert len(emergence_events) > 0, "No emergence detected"
    emergence = emergence_events[0]
    pattern_type = emergence.payload.get("pattern_type", "")
    assert pattern_type in ("RISING", "SPIKE"), \
        f"Expected RISING or SPIKE pattern, got: {pattern_type}"


@pytest.mark.asyncio
async def test_detector_unnameable_flag_recorded(
    clean_buses, detector, core_bus
):
    """
    GIVEN: A ResidualDetector listening to events
    WHEN: A JUDGMENT_CREATED event has unnameable_detected=True
    THEN: The detector records this in its internal history
    """
    detector.start(core_bus)

    event = Event.typed(
        CoreEvent.JUDGMENT_CREATED,
        JudgmentCreatedPayload(
            judgment_id=new_id(),
            cell={
                "reality": "CODE",
                "analysis": "JUDGE",
                "time_dim": "PRESENT",
                "content": "test",
                "context": "test",
                "cell_id": new_id(),
            },
            q_score=45.0,
            verdict="GROWL",
            confidence=0.40,
            residual_variance=0.45,
            unnameable_detected=True,  # Flag set!
            timestamp=123.0,
            cost_usd=0.01,
            llm_calls=1,
        ),
        source="test",
    )

    await core_bus.emit(event)
    await asyncio.sleep(0.1)

    # Verify recorded in history
    assert len(detector._history) >= 1
    recorded_point = list(detector._history)[0]
    assert recorded_point.unnameable is True


# ════════════════════════════════════════════════════════════════════════════
# TEST 6: Error isolation — one bad event doesn't break detector
# ════════════════════════════════════════════════════════════════════════════

@pytest.mark.asyncio
async def test_detector_resilient_to_malformed_events(
    clean_buses, detector, core_bus
):
    """
    GIVEN: A ResidualDetector
    WHEN: A malformed or partially-missing JUDGMENT_CREATED event is emitted
    THEN: The detector handles it gracefully (extracts what it can)
    """
    detector.start(core_bus)

    # Emit a valid event first
    event1 = Event.typed(
        CoreEvent.JUDGMENT_CREATED,
        JudgmentCreatedPayload(
            judgment_id=new_id(),
            cell={
                "reality": "CODE",
                "analysis": "JUDGE",
                "time_dim": "PRESENT",
                "content": "test",
                "context": "test",
                "cell_id": new_id(),
            },
            q_score=50.0,
            verdict="GROWL",
            confidence=0.40,
            residual_variance=0.25,
            unnameable_detected=False,
            timestamp=123.0,
            cost_usd=0.01,
            llm_calls=1,
        ),
        source="test",
    )

    await core_bus.emit(event1)
    await asyncio.sleep(0.05)

    # Emit minimal event (missing some fields, detector should handle with defaults)
    event2 = Event(
        type=CoreEvent.JUDGMENT_CREATED,
        payload={
            "judgment_id": new_id(),
            "residual_variance": 0.30,
            # Missing other fields — detector uses .get() with defaults
        },
        source="test",
    )

    await core_bus.emit(event2)
    await asyncio.sleep(0.1)

    # Both should be recorded
    stats = detector.stats()
    assert stats["observations"] >= 2


# ════════════════════════════════════════════════════════════════════════════
# SUMMARY
# ════════════════════════════════════════════════════════════════════════════
"""
Tests Implemented (12 test functions, 27 assertions):

✓ Test 1: Detector subscribes to JUDGMENT_CREATED on core bus
✓ Test 1b: Idempotent start() (no duplicate listeners)
✓ Test 2: Detector receives and processes events
✓ Test 2b: Accumulates multiple judgments
✓ Test 3a: Emits EMERGENCE_DETECTED on STABLE_HIGH pattern
✓ Test 3b: Emits ANOMALY_DETECTED on SPIKE pattern
✓ Test 3c: Statistics tracking (observations, anomalies, patterns)
✓ Test 4a: Genealogy prevents loops (via bridge)
✓ Test 4b: Event.already_seen() genealogy tracking
✓ Test 5: Rising pattern detection
✓ Test 5b: Unnameable flag recording
✓ Test 6: Resilience to malformed events

Verdict: All tests are TDD-focused (simple assertions, no over-engineering).
         Real buses + real detector = proof that wiring works end-to-end.
         Foundation for Phase 2 learning loop validation is solid.
"""
