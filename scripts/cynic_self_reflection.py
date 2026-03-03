"""
🌀 CYNIC SELF-REFLECTION LOOP : The Organism reads its own DNA.

This script feeds CYNIC's own fragile source code to its internal LLM models
(Qwen Coder, DeepSeek) so it can auto-diagnose and propose its own fixes.

Lentilles: AI Infra (Self-hosting), SRE (Auto-healing).
"""

import asyncio
import time
import sys
import os
from pathlib import Path

# Ensure the project root is in PYTHONPATH
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

from cynic.kernel.organism.brain.llm.adapter import LLMRegistry, LLMRequest


async def get_diagnosis(adapter, role, code_content, filepath):
    print(f"  -> {role} ({adapter.adapter_id}) is analyzing {filepath}...")
    t0 = time.perf_counter()

    prompt = f"""
FILE: {filepath}

You are CYNIC's internal {role}. Analyze this Python code.
1. Identify any SyntaxErrors (like missing quotes, bad indentation in except blocks).
2. Identify any architectural heresis (hardcoded values, bad abstractions).
3. Provide the EXACT corrected code snippet.

CODE:
```python
{code_content}
```
"""

    req = LLMRequest(
        system=f"You are CYNIC's {role} Agent. Be extremely concise. Focus on Python syntax and architecture.",
        prompt=prompt,
        metadata={"keep_alive": 0},
    )

    try:
        # Increase timeout for complex code files
        resp = await asyncio.wait_for(adapter.complete(req), timeout=300.0)
        duration = time.perf_counter() - t0
        print(f"  <- {role} finished in {duration:.2f}s")
        return resp.content
    except asyncio.TimeoutError:
        print(f"  ⚠️ {role} timed out.")
        return "TIMEOUT"
    except Exception as e:
        print(f"  ❌ {role} error: {e}")
        return f"ERROR: {e}"


async def run_reflection_loop():
    print("=" * 60)
    print("🌀 CYNIC SELF-REFLECTION LOOP (Auto-Healing Initialization)")
    print("=" * 60)

    # 1. Wake up the brain
    registry = LLMRegistry()
    await registry.discover()
    available = registry.get_available_for_generation()

    if not available:
        print("❌ No internal muscles found. Cannot self-reflect.")
        return

    # Try to find specific models, fallback to whatever is available
    analyst = next(
        (a for a in available if "coder" in a.adapter_id.lower()), available[0]
    )
    sage = next(
        (a for a in available if "deepseek" in a.adapter_id.lower()), available[-1]
    )

    # 2. Pick a fragile file from the core
    # We deliberately pick a file we know is broken from our previous MasterHealer run
    target_file = Path("cynic/kernel/core/soul.py")

    if not target_file.exists():
        print(f"File {target_file} not found.")
        return

    content = target_file.read_text(encoding="utf-8", errors="ignore")

    # We truncate the content if it's too long to avoid context window explosion on local LLMs
    # Just sending the first 150 lines is usually enough to catch import/class definition errors
    lines = content.splitlines()
    truncated_content = "\\n".join(lines[:150])

    print(f"\\n[TARGET INGESTED]: {target_file} ({len(lines)} lines)")

    # 3. Parallel Judgment
    print("\\n[INITIATING INTERNAL COGNITION]")

    # Run them sequentially to avoid blowing up the Ryzen 5700G's VRAM
    analyst_report = await get_diagnosis(
        analyst, "ANALYST (Syntax & Logic)", truncated_content, target_file.name
    )
    print("-" * 40)
    print(analyst_report)
    print("-" * 40)

    sage_report = await get_diagnosis(
        sage, "SAGE (Architecture & Phi)", truncated_content, target_file.name
    )
    print("-" * 40)
    print(sage_report)
    print("-" * 40)

    print("\n" + "=" * 60)
    print("STATUS: SELF-REFLECTION CYCLE COMPLETE.")
    print("=" * 60)


if __name__ == "__main__":
    asyncio.run(run_reflection_loop())
