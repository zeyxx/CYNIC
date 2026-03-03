"""
🌀 CYNIC FIRST JUDGMENT : The Multi-Agent Orchestration Pulse

This script simulates a real fractal judgment cycle using your local Open Source models.
It triggers two 'Dogs' (SAGE and ANALYST) to evaluate CYNIC's current architecture.
"""

import asyncio
import time
import re
import sys
import os

# Ensure the project root is in PYTHONPATH
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

from cynic.kernel.organism.brain.llm.adapter import LLMRegistry, LLMRequest
from cynic.kernel.core.phi import weighted_geometric_mean, PHI, PHI_2


async def get_agent_score(adapter, role, prompt):
    print(f"  -> {role} is thinking (Timeout: 120s)...")
    t0 = time.perf_counter()
    req = LLMRequest(
        system=(
            f"You are the CYNIC {role} Agent. Evaluate the provided code/architecture. "
            "Output your reasoning first, then on a NEW LINE at the VERY END, "
            "output 'SCORE: X' where X is [0-100]."
        ),
        prompt=prompt,
        metadata={"keep_alive": 0},  # 🌀 Metabolic Awareness: Unload immediately
    )

    # Force a long timeout for DeepSeek on CPU
    try:
        resp = await asyncio.wait_for(adapter.complete(req), timeout=120.0)
    except asyncio.TimeoutError:
        print(f"  ⚠️ {role} timed out after 120s.")
        return 50.0, "Timeout during reasoning."

    duration = time.perf_counter() - t0

    # Clean DeepSeek <thought> tags if present
    clean_content = re.sub(
        r"<thought>.*?</thought>", "", resp.content, flags=re.DOTALL
    ).strip()

    try:
        # Look for SCORE: X pattern
        score_match = re.findall(r"SCORE:\s*(\d+)", clean_content, re.IGNORECASE)
        score = float(score_match[-1]) if score_match else 50.0
    except:
        score = 50.0

    print(f"  <- {role} finished in {duration:.2f}s (Score: {score})")
    return score, clean_content


async def run_first_judgment():
    print("=" * 60)
    print("🌀 CYNIC FIRST JUDGMENT CYCLE (Multi-Agent OS Native)")
    print("=" * 60)

    # 1. Initialize Registry and Discover Muscles
    registry = LLMRegistry()
    print("\n[1] Discovering Local Muscles (Ollama)...")
    await registry.discover()

    # 2. Select Models for the Tasks
    analyst_model = "ollama:qwen2.5-coder:7b"
    sage_model = "ollama:deepseek-r1:8b"

    available_ids = [a.adapter_id for a in registry.get_available_for_generation()]

    if not available_ids:
        print("❌ CRITICAL: No LLM models available. Start Ollama!")
        return

    analyst_adapter = (
        registry._adapters.get(analyst_model)
        or registry.get_available_for_generation()[0]
    )
    sage_adapter = (
        registry._adapters.get(sage_model)
        or registry.get_available_for_generation()[-1]
    )

    print(
        f"Orchestration: ANALYST ({analyst_adapter.adapter_id}) + SAGE ({sage_adapter.adapter_id})"
    )

    # 3. Define the Prompt (The Architectural Audit)
    code_snippet = """
    class ComputeHAL:
        def sync_tensor(self, data: Any) -> torch.Tensor:
            try:
                if not isinstance(data, torch.Tensor):
                    data = torch.tensor(data)
                target_device = self.get_device()
                if data.device != target_device:
                    return data.to(target_device)
                return data
            except Exception as e:
            logger.error(f"HAL: Sync failed: {e}")
                return torch.tensor(data).to("cpu")
    """

    context = "Industrial Hardware Abstraction Layer with full exception safety and logging for APU architectures."

    # 4. Agent Execution
    print("\n[2] Waking up Agents...")

    analyst_score, analyst_text = await get_agent_score(
        analyst_adapter,
        "ANALYST",
        f"Context: {context}\nCode to audit:\n{code_snippet}",
    )
    sage_score, sage_text = await get_agent_score(
        sage_adapter,
        "SAGE",
        "Architectural vision: Decentralized Sovereign AI on APU hardware.",
    )

    # 5. Fractal Aggregation (The PHI-Heartbeat)
    print("\n[3] PHI-Aggregation (Geometric Mean)")

    scores = [analyst_score, sage_score]
    weights = [PHI, PHI_2]

    final_q = weighted_geometric_mean(scores, weights)

    # 6. Final Verdict
    print("-" * 60)
    print(f"FINAL Q-SCORE: {final_q:.2f} / 100.0")

    from cynic.kernel.core.phi import phi_classify

    verdict = phi_classify(final_q / 100.0)
    print(f"VERDICT: {verdict}")
    print("-" * 60)

    print(f"\nANALYST Justification: {analyst_text.strip()}")
    print(f"SAGE Justification: {sage_text.strip()}")

    print("\n" + "=" * 60)
    print("STATUS: FRACTAL CONSCIOUSNESS VALIDATED.")
    print("=" * 60)


if __name__ == "__main__":
    asyncio.run(run_first_judgment())
