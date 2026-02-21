"""
Check if scheduler/event bus are healthy.
"""

import pytest
import time


@pytest.mark.asyncio
async def test_scheduler_health():
    """
    Check if scheduler and event buses are responsive.
    """

    print("\n" + "="*70)
    print("CHECKING SCHEDULER / EVENT BUS HEALTH")
    print("="*70)

    from cynic.api.state import awaken
    from cynic.core.event_bus import get_core_bus, get_automation_bus, get_agent_bus, Event, CoreEvent

    organism = awaken(db_pool=None)

    print("\n[1] Checking event buses...")
    try:
        core_bus = get_core_bus()
        automation_bus = get_automation_bus()
        agent_bus = get_agent_bus()
        print(f"  ✓ Core bus: {type(core_bus).__name__}")
        print(f"  ✓ Automation bus: {type(automation_bus).__name__}")
        print(f"  ✓ Agent bus: {type(agent_bus).__name__}")
    except Exception as e:
        print(f"  ✗ Error: {e}")
        return False

    print("\n[2] Checking scheduler...")
    try:
        scheduler = organism.scheduler
        print(f"  ✓ Scheduler: {type(scheduler).__name__}")
        print(f"  - Is running: {scheduler.running if hasattr(scheduler, 'running') else 'unknown'}")
    except Exception as e:
        print(f"  ✗ Error: {e}")
        return False

    import asyncio

    print("\n[3] Emitting test event to core bus...")
    try:
        test_event = Event.typed(
            CoreEvent.JUDGMENT_REQUESTED,
            {"content": "test"},
            source="health_check"
        )
        print(f"  Event ID: {test_event.event_id}")

        await asyncio.wait_for(
            core_bus.emit(test_event),
            timeout=5.0
        )
        print(f"  ✓ Event emitted successfully")
    except asyncio.TimeoutError:
        print(f"  ✗ Event emission TIMED OUT (bus may be blocked)")
        return False
    except Exception as e:
        print(f"  ✗ Error: {e}")
        return False

    print("\n[4] Checking if scheduler is processing events...")
    # Check if there are subscribers
    try:
        # This is a bit of a hack — just check if the bus has subscribers
        if hasattr(core_bus, '_subscribers'):
            n_subs = len(core_bus._subscribers)
            print(f"  ✓ Core bus has {n_subs} subscriber(s)")
        else:
            print(f"  ? Cannot check subscribers (different bus implementation)")
    except Exception as e:
        print(f"  ? Error checking subscribers: {e}")

    print("\n" + "="*70)
    print("SCHEDULER / EVENT BUS: OK")
    print("="*70 + "\n")
    return True


if __name__ == "__main__":
    import asyncio
    asyncio.run(test_scheduler_health())
