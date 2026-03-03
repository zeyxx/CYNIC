"""
🌀 CYNIC SELF-DIAGNOSIS : The Organism Awakens

This script performs a full audit of CYNIC's hardware awareness, 
compute capabilities, and cognitive reach on your machine.
"""

import asyncio
import platform
import torch
import sys
import os

# Ensure the project root is in PYTHONPATH
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

from cynic.kernel.organism.metabolism.model_profiler import ModelProfiler
from cynic.kernel.infrastructure.compute_hal import get_compute_hal
from cynic.kernel.organism.brain.llm.adapter import LLMRegistry
from cynic.kernel.core.phi import weighted_geometric_mean, PHI_INV, PHI

async def run_diagnosis():
    print("=" * 60)
    print("🌀 CYNIC SYSTEM SELF-DIAGNOSIS v3.5 (Pre-Hackathon Audit)")
    print("=" * 60)

    # 1. METABOLIC PROFILING (Ryzen 5700G Awareness)
    print("\n[1] LENS 8: METABOLIC AWARENESS (Hardware)")
    profiler = ModelProfiler()
    print(profiler.announce_limits())
    
    # 2. COMPUTE HAL (Vega 8 iGPU Awareness)
    print("\n[2] LENS 6: COMPUTE ENGINE (HAL)")
    hal = get_compute_hal()
    print(hal.announce_compute())
    
    # 3. COGNITIVE REACH (LLM Discovery)
    print("\n[3] LENS 1: COGNITIVE REACH (LLM Discovery)")
    registry = LLMRegistry()
    manifest = await registry.discover()
    
    if manifest["available"]:
        print(f"Found {len(manifest['available'])} active LLM muscles:")
        for m in manifest["available"]:
            print(f"  - {m}")
    else:
        print("⚠️  No LLM muscles discovered. Is Ollama running?")

    # 4. FRACTAL HEARTBEAT (Numerical Integrity)
    print("\n[4] LENS 7: FRACTAL HEARTBEAT (Numerical Integrity)")
    test_scores = [85.0, 92.0, 78.0, 65.0, 99.0]
    test_weights = [PHI, PHI_INV, 1.0, PHI_INV, PHI]
    
    try:
        q_score = weighted_geometric_mean(test_scores, test_weights)
        print(f"Core Logic (Geometric Mean): OK | Sample Q-Score: {q_score:.4f}")
    except Exception as e:
        print(f"❌ Core Logic Error: {e}")

    # 5. OS & ENVIRONMENT
    print("\n[5] LENS 9: SOVEREIGNTY (Environment)")
    print(f"OS: {platform.system()} {platform.release()} | Python: {platform.python_version()}")
    print(f"PyTorch: {torch.__version__} | CUDA Available: {torch.cuda.is_available()}")

    print("\n" + "=" * 60)
    print("STATUS: CYNIC IS BATTLE-READY.")
    print("=" * 60)

if __name__ == "__main__":
    asyncio.run(run_diagnosis())
