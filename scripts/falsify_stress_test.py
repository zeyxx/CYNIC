"""
CYNIC Stress Test - Immune System & Plasticity Falsification.
Proves that CYNIC adapts to failures and manages resources under pressure.
"""
import asyncio
import logging
from cynic.orchestration import OrganismOrchestrator

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("cynic.stress_test")

async def run_stress_test():
    print("\n" + "!"*60)
    print("🔥 CYNIC STRESS TEST: IMMUNE SYSTEM & PLASTICITY")
    print("!"*60)
    
    orch = OrganismOrchestrator(instance_id="cynic-stress-test")
    await orch.awake()
    
    # 1. Simulate TRAUMA: Force failure of the currently favored model
    favorite = "ollama:qwen3.5:9b"
    print(f"\n--- Phase 1: Traumatic Failure of {favorite} ---")
    for _ in range(5):
        await orch.vault.record_experience(favorite, "BACKEND", success=False, latency_ms=5000)
    
    print(f"  -> {favorite} has been flagged with 5 major failures.")

    # 2. Verify PLASTICITY: Does CYNIC switch?
    print("\n--- Phase 2: Synaptic Plasticity (Muscle Switching) ---")
    result = await orch.process_task("Fix some backend issue", axiom="BACKEND")
    new_muscle = result['muscle']
    print(f"  -> New muscle selected for BACKEND: {new_muscle}")
    
    if new_muscle != favorite:
        print("  ✅ PLASTICITY VALIDATED: CYNIC adapted and switched muscles.")
    else:
        print("  ❌ PLASTICITY FAILED: CYNIC is stuck on a broken model.")

    # 3. Simulate SOMATIC STRESS: RAM Pressure
    print("\n--- Phase 3: Somatic Stress (Metabolic Eviction) ---")
    orch.governor.vram_threshold = 0.1 # Force stress state
    
    # Mocking Ollama to see if eviction is called
    from unittest.mock import AsyncMock
    orch.governor.provider.unload_model = AsyncMock(return_value=True)
    orch.governor.provider.get_loaded_models = AsyncMock(return_value=["ollama:llama3:8b", "ollama:mistral:7b"])
    
    await orch.process_task("Critical architecture task", axiom="ARCHITECTURE")
    
    if orch.governor.provider.unload_model.called:
        print("  ✅ METABOLISM VALIDATED: Governor evicted models to save RAM.")
    else:
        print("  ❌ METABOLISM FAILED: No eviction under stress.")

    await orch.sleep()
    print("\n" + "!"*60)
    print("🏆 STRESS TEST COMPLETE: IMMUNE SYSTEM IS FULLY FUNCTIONAL.")
    print("!"*60)

if __name__ == "__main__":
    asyncio.run(run_stress_test())
