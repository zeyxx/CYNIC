"""
CYNIC Ultimate Life Test — Prediction vs Reality.

Proves that CYNIC can:
1. Predict an outcome from code.
2. Execute an action to verify it.
3. Learn from the delta between dream and reality.
"""
import asyncio
import os
import sys
import time
from pathlib import Path

# Add root to path
sys.path.append(os.getcwd())

from cynic.kernel.organism.organism import awaken
from cynic.kernel.core.judgment import Cell
from cynic.kernel.core.consciousness import ConsciousnessLevel

async def ultimate_test():
    print("\n--- 🧬 CYNIC ULTIMATE LIFE TEST ---")
    
    # 1. AWAKEN
    o = await awaken()
    await o.start()
    
    try:
        # 2. CREATE DILEMMA (Instable Code)
        dilemma_path = Path("dilemma.py")
        dilemma_path.write_text("def unstable_fn():\n    # This function is intentionally broken\n    return 1 / 0\n", encoding="utf-8")
        print(f"Step 1: Created physical dilemma at {dilemma_path}")

        # 3. PREDICT (Oracle Dog)
        # Use BARK as the action because it's a valid verdict
        target_action = "BARK" 
        
        print(f"Step 2: Asking ORACLE to predict outcome for action '{target_action}'...")
        prediction_q = o.cognition.qtable.predict_q("dilemma:unstable_fn", target_action)
        print(f"   - Oracle Current Q-Prediction: {prediction_q:.3f} (Before Learning)")

        # 4. EXECUTE (Verify Reality)
        print("Step 3: Verifying Reality via execution...")
        success = False
        error = ""
        try:
            import importlib
            import dilemma
            importlib.reload(dilemma)
            dilemma.unstable_fn()
            success = True
        except Exception as e:
            success = False
            error = str(e)
        
        # Reality Score: 0.0 for failure, 1.0 for success
        reality_reward = 0.0 if not success else 1.0
        print(f"   - Reality Check: SUCCESS={success}, ERROR='{error}'")
        print(f"   - Reality Reward Signal: {reality_reward}")

        # 5. LEARN (The Delta)
        print("Step 4: Feeding the Prediction Error to the Learning Loop...")
        
        from cynic.kernel.core.event_bus import get_core_bus, Event, CoreEvent
        from cynic.kernel.core.events_schema import LearningEventPayload
        
        # We manually emit the learning event to simulate the feedback loop
        await get_core_bus().emit(Event.typed(
            CoreEvent.LEARNING_EVENT,
            LearningEventPayload(
                state_key="dilemma:unstable_fn",
                action=target_action,
                reward=reality_reward,
                q_value_old=prediction_q,
                q_value_new=0.0 # Will be calculated by loop
            ),
            source="ultimate_test"
        ))
        
        # Wait for async learning loop to process
        await asyncio.sleep(0.5)
        
        # Final check: Q-Table update
        new_q = o.cognition.qtable.predict_q("dilemma:unstable_fn", target_action)
        print(f"   - Q-Table Verification: New Q-Value = {new_q:.3f}")
        
        if new_q < prediction_q or (reality_reward == 1.0 and new_q > prediction_q):
            print("\n✅ ULTIMATE TEST PASSED: CYNIC has learned from Reality.")
        else:
            print("\n❌ ULTIMATE TEST FAILED: Q-Value did not evolve as expected.")

    except Exception as e:
        print(f"\n❌ ULTIMATE TEST FAILED: {e}")
        import traceback
        traceback.print_exc()
    finally:
        if dilemma_path.exists(): os.remove(dilemma_path)
        await o.stop()

if __name__ == "__main__":
    asyncio.run(ultimate_test())
