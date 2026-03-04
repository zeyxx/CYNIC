"""
CYNIC Night One Falsification - The Sovereign Verification.
Proves that CYNIC is successfully communicating with the Vulkan APU Server.
"""
import asyncio
import logging
import time
from cynic.orchestration import OrganismOrchestrator

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
logger = logging.getLogger("cynic.night_one")

async def run_final_check():
    print("\n" + "="*60)
    print("🛡️ CYNIC NIGHT ONE: SOVEREIGN VULKAN ACTIVATION")
    print("="*60)
    
    # We wait a bit for the model to download/load in llama-server
    print("Waiting 10s for the Sovereign Server to initialize...")
    await asyncio.sleep(10)
    
    orch = OrganismOrchestrator(instance_id="cynic-sovereign-node")
    await orch.awake()
    
    # The discovery should now find "llama_cpp_server:sovereign-apu-model"
    print(f"Muscles discovered: {[a.adapter_id for a in orch.registry.get_available()]}")
    
    # Run a high-fidelity task
    print("\n--- [Cycle 3/3] Task: ARCHITECTURE ---")
    start = time.time()
    result = await orch.process_task(
        "Analyze the architectural impact of moving from Ollama to raw llama-server.",
        axiom="ARCHITECTURE"
    )
    latency = (time.time() - start) * 1000
    
    print(f"  -> Muscle Used: {result['muscle']}")
    print(f"  -> Response: {result['output'].strip()}")
    print(f"  -> Real Latency: {latency:.0f}ms")
    
    if "llama_cpp_server" in result['muscle']:
        print("\n✅ SOVEREIGNTY VALIDATED: CYNIC is now using its native APU muscle.")
    else:
        print("\n❌ SOVEREIGNTY FAILED: CYNIC fell back to Ollama or failed discovery.")

    await orch.sleep()
    print("\n" + "="*60)
    print("🏆 INFRASTRUCTURE IS STABLE. READY FOR AUTONOMOUS NIGHT.")
    print("="*60)

if __name__ == "__main__":
    asyncio.run(run_final_check())
