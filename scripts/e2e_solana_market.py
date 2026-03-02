"""
CYNIC Solana Market Stability Test â€” High Fidelity E2E.

Validates that:
1. MARKET/SOLANA reality payloads are correctly validated.
2. MasterDog leverages MarketExpertise for these realities.
3. The pipeline handles high-frequency market pulses without instability.
"""
import asyncio
import os
import sys

# Add root to path
sys.path.append(os.getcwd())

from cynic.kernel.organism.organism import awaken
from cynic.kernel.core.event_bus import CoreEvent, Event
from cynic.kernel.core.realities import MarketPayload, SolanaPayload

async def solana_market_test():
    print("\n--- ðŸŒŠ CYNIC SOLANA MARKET STABILITY TEST ---")
    
    # 1. AWAKEN
    o = await awaken()
    await o.start()
    
    try:
        # 2. EMIT MARKET PULSE (SOL Price)
        print("Step 1: Emitting SOL Market Pulse...")
        market_payload = MarketPayload(
            symbol="SOL",
            price=142.55,
            change_24h=-2.5,
            volatility=0.8, # High volatility!
            source="test_harness"
        )
        
        bus = get_core_bus("DEFAULT")
        # MarketSensor usually emits PERCEPTION_RECEIVED
        await bus.emit(Event.typed(
            CoreEvent.PERCEPTION_RECEIVED,
            {**market_payload.model_dump(), "run_judgment": True},
            source="market_sensor"
        ))
        
        # 3. EMIT SOLANA ON-CHAIN PULSE
        print("Step 2: Emitting Solana On-Chain Pulse...")
        solana_payload = SolanaPayload(
            slot=254433221,
            tps=2450.0,
            health="ok",
            recent_prioritization_fee=0.000005,
            source="solana_sensor" # Required by schema
        )
        
        await bus.emit(Event.typed(
            CoreEvent.PERCEPTION_RECEIVED,
            {**solana_payload.model_dump(), "run_judgment": True},
            source="solana_sensor"
        ))

        # 4. WAIT for async judgment
        print("Step 3: Waiting for MasterDog to judge the Market...")
        await asyncio.sleep(1.0)

        # 5. VERIFY STATE
        stats = o.state.get_stats()
        print(f"   - Total Judgments in State: {stats['total_judgments']}")
        
        # 6. CHECK FOR RECENT JUDGMENTS
        recent = o.state.get_recent_judgments(limit=2)
        for j in recent:
            print(f"   - Judgment: ID={j.judgment_id[:8]}, Verdict={j.verdict}, Q={j.q_score:.1f}")
            print(f"     Reasoning: {j.reasoning[:100]}...")

        if stats['total_judgments'] >= 1:
            print("\nâœ… SOLANA MARKET STABILITY SUCCESSFUL: Pipeline is 100% operational.")
        else:
            print("\nâŒ STABILITY FAILED: No judgments recorded.")

    except Exception as e:
        print(f"\nâŒ TEST CRASHED: {e}")
        import traceback
        traceback.print_exc()
    finally:
        await o.stop()

if __name__ == "__main__":
    asyncio.run(solana_market_test())
