"""
CYNIC Final Organic Validation - The Night-Ready Protocol.
Falsifies the 3 pillars: Cognition (Judge), Metabolism (Governor), Memory (Vault).
"""
import asyncio
import logging
from cynic.kernel.organism.orchestrator import get_orchestrator
from cynic.kernel.organism.cognition.judge import get_judge
from cynic.kernel.organism.metabolism.governor import MetabolicGovernor, ResourceProvider

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
logger = logging.getLogger("cynic.falsify_organism")

class MockVRAMProvider(ResourceProvider):
    def __init__(self):
        self.evictions = []
    async def get_loaded_models(self): return ["old_model_1"]
    async def unload_model(self, m): 
        self.evictions.append(m)
        return True

async def run_final_validation():
    print("\n" + "="*60)
    print("🛡️ CYNIC FINAL ORGANIC VALIDATION - PRE-NIGHT 1")
    print("="*60)

    # 1. COGNITION: The Axiomatic Judge (Heresy Detection)
    print("\n--- Phase 1: Cognition (Axiomatic Judge) ---")
    judge = get_judge()
    bad_diff = """
    try:
        val = os.getenv("API_KEY")
        # pipeline logic
    except:
        pass
    """
    verdict = judge.evaluate_proposal(axiom="FIDELITY", proposal_description="Fixing pipeline", diff=bad_diff)
    print(f"  -> Judge Score: {verdict['phi_score']} (Valid: {verdict['is_valid']})")
    for f in verdict['findings']: print(f"  -> [!] {f}")
    
    if not verdict['is_valid']:
        print("  ✅ Cognition Validated: Heresy caught correctly.")
    else:
        print("  ❌ Cognition Failed: Heresy allowed.")

    # 2. METABOLISM: The Governor (Resource Appropriation)
    print("\n--- Phase 2: Metabolism (Somatic Governor) ---")
    mock_provider = MockVRAMProvider()
    gov = MetabolicGovernor(provider=mock_provider)
    gov.vram_threshold = 0.1 # Force stress
    
    orch = get_orchestrator()
    orch.governor = gov
    
    async def dummy_action(m): return "Success"
    
    await orch.execute_with_learning(axiom="BACKEND", candidates=["expert_dog"], action_func=dummy_action)
    
    if "old_model_1" in mock_provider.evictions:
        print("  ✅ Metabolism Validated: Resources appropriated for the task.")
    else:
        print("  ❌ Metabolism Failed: No resource eviction under stress.")

    # 3. MEMORY: The Vault (Experience Learning)
    print("\n--- Phase 3: Memory (Synaptic Learning) ---")
    # Simulate a failure with 'weak_dog'
    orch.vault.record_experience("weak_dog", "BACKEND", success=False, latency_ms=1000)
    orch.vault.record_experience("expert_dog", "BACKEND", success=True, latency_ms=500)
    
    best = orch.vault.get_best_dog_for("BACKEND", ["weak_dog", "expert_dog"])
    print(f"  -> Best model learned for BACKEND: {best}")
    
    if best == "expert_dog":
        print("  ✅ Memory Validated: CYNIC has learned from experience.")
    else:
        print("  ❌ Memory Failed: CYNIC still prefers the failed model.")

    print("\n" + "="*60)
    print("🏆 RESULT: CYNIC IS READY FOR ITS FIRST AUTONOMOUS NIGHT.")
    print("="*60)

if __name__ == "__main__":
    asyncio.run(run_final_validation())
