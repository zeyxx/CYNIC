"""
CYNIC Load Capacity Test — Stress-testing the event bus for 10k TPS readiness.
"""

import asyncio
import time
import logging
import sys
import os

# Set PYTHONPATH
sys.path.insert(0, os.getcwd())

from cynic.kernel.core.event_bus import get_core_bus, Event, CoreEvent, current_instance_id

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("load_test")

async def mock_handler(event):
    """Simulate a fast cognitive reflex (1ms)."""
    await asyncio.sleep(0.001)

async def run_stress_test(events_count: int = 1000):
    instance_id = "STRESS-TEST-001"
    current_instance_id.set(instance_id)
    bus = get_core_bus(instance_id)
    
    # 1. Setup handlers
    for i in range(10): # 10 dogs
        bus.on(CoreEvent.PERCEPTION_RECEIVED, mock_handler)
    
    logger.info(f"🚀 Starting Stress Test: {events_count} events...")
    
    t0 = time.perf_counter()
    
    # 2. Blast events
    tasks = []
    for i in range(events_count):
        tasks.append(bus.emit(Event.typed(
            CoreEvent.PERCEPTION_RECEIVED, 
            {"tick": i}, 
            source="stress_generator"
        )))
    
    await asyncio.gather(*tasks)
    
    # 3. Wait for handlers to finish (Drain)
    await bus.drain(timeout=10.0)
    
    duration = time.perf_counter() - t0
    stats = bus.stats()
    
    logger.info("📊 STRESS TEST RESULTS:")
    logger.info(f"  Duration: {duration:.2f}s")
    logger.info(f"  Throughput: {events_count / duration:.2f} EPS (Events Per Second)")
    logger.info(f"  Total Handlers Executed: {stats['emitted'] * 10}")
    logger.info(f"  Peak Pending Tasks: {stats['peak_pending']}")
    logger.info(f"  Avg Bus Latency: {stats['avg_latency_ms']:.4f}ms")
    logger.info(f"  Load Factor: {stats['load_factor']:.2%}")

if __name__ == "__main__":
    count = 1000
    if len(sys.argv) > 1:
        count = int(sys.argv[1])
    asyncio.run(run_stress_test(count))
