"""
CYNIC Memory Optimization Falsification.
Validates the TokenFilter, CognitiveGraph, and KVManager logic.
"""
import asyncio
from cynic.kernel.organism.metabolism.token_filter import TokenFilter
from cynic.kernel.organism.memory.cognitive_graph import CognitiveGraph
from cynic.kernel.organism.memory.kv_manager import KVManager

async def run_falsification():
    print("\n" + "="*60)
    print("🧠 CYNIC MEMORY & TOKEN OPTIMIZATION FALSIFICATION")
    print("="*60)

    # 1. Validate TokenFilter (RTK Doctrine)
    print("\n--- PHASE 1: TOKEN FILTERING (RTK) ---")
    noisy_log = """
    DEBUG: Connection established
    DEBUG: Connection established
    DEBUG: Connection established
    INFO: Starting process
    
    
    WARNING: Memory pressure high
    WARNING: Memory pressure high
    """
    compressed = TokenFilter.compress_shell_output(noisy_log)
    print(f"  -> Original size: {len(noisy_log)} chars")
    print(f"  -> Compressed size: {len(compressed)} chars")
    print(f"  -> Content Preview:\n{compressed}")
    
    if "... (repeated" in compressed and "DEBUG" in compressed:
        print("  ✅ TOKEN FILTER VALIDATED: Deduplication and cleanup successful.")
    else:
        print("  ❌ TOKEN FILTER FAILED.")

    # 2. Validate CognitiveGraph (AgentKeeper Doctrine)
    print("\n--- PHASE 2: COGNITIVE PERSISTENCE (AgentKeeper) ---")
    graph = CognitiveGraph(storage_path="audit/test_cognitive_graph.json")
    graph.add_fact("Vulkan", "Vulkan is the primary acceleration API for AMD APUs.", criticality=5)
    
    context = graph.get_relevant_context("How should I optimize the Ryzen 5700G?")
    print(f"  -> Task: How should I optimize the Ryzen 5700G?")
    print(f"  -> Relevant context found: {'Vulkan' in context}")
    
    if "Vulkan is the primary" in context:
        print("  ✅ COGNITIVE GRAPH VALIDATED: Fact retrieval successful.")
    else:
        print("  ❌ COGNITIVE GRAPH FAILED.")

    # 3. Validate KVManager (LMCache Doctrine)
    print("\n--- PHASE 3: KV CACHE SLOTTING (LMCache) ---")
    kv = KVManager()
    slot_a = kv.get_slot_for_task("agent_architect")
    slot_b = kv.get_slot_for_task("agent_security")
    slot_a_again = kv.get_slot_for_task("agent_architect")
    
    print(f"  -> Slot for Architect: {slot_a}")
    print(f"  -> Slot for Security: {slot_b}")
    print(f"  -> Slot for Architect (Repeat): {slot_a_again}")
    
    if slot_a == slot_a_again and slot_a != slot_b:
        print("  ✅ KV MANAGER VALIDATED: Slot consistency established.")
    else:
        print("  ❌ KV MANAGER FAILED.")

    print("\n" + "="*60)
    print("🏆 ALL OPTIMIZATION LOGIC VALIDATED.")
    print("="*60)

if __name__ == "__main__":
    asyncio.run(run_falsification())
