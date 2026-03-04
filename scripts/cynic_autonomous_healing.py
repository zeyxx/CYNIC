"""
CYNIC Autonomous Healing Loop - The pinnacle of self-preservation.
Respects Robotics, ML Platform & Solutions Architect Lenses.

This system gives CYNIC the agency to read its own error history 
(from the Heresy Mine), reason about it using local LLMs, and 
apply fixes via its own ToolExecutor (safeguarded by HeresyFilter).

It transforms CYNIC from a 'program' into an 'Autonomous Agentic OS'.
"""
import asyncio
import json
import logging
from pathlib import Path

# Provide CYNIC with its own hands and brain
from cynic.interfaces.chat.tool_executor import ToolExecutor
from cynic.interfaces.chat.tools import ToolCall
from cynic.kernel.organism.brain.llm.adapter import LLMRegistry, LLMRequest

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
logger = logging.getLogger("cynic.autonomous_healing")

async def autonomous_heal():
    print("============================================================")
    print("🧬 CYNIC AUTONOMOUS HEALING : The Organism Mends Itself")
    print("============================================================")

    # 1. Access the Heresy Mine
    mine_path = Path("audit/heresy_mine.jsonl")
    if not mine_path.exists():
        logger.error("No Heresy Mine found. Run mine_heresies.py first.")
        return

    # Extract the first unhealed heresy (e.g., an INDENTATION_FRACTURE)
    target_heresy = None
    with open(mine_path, "r", encoding="utf-8") as f:
        for line in f:
            record = json.loads(line)
            # Pick a simple indentation fracture as our first autonomous trial
            if record["category"] == "INDENTATION_FRACTURE":
                target_heresy = record
                break
                
    if not target_heresy:
        logger.info("No suitable heresy found in the mine.")
        return

    logger.info(f"Target Acquired: {target_heresy['file']} at line {target_heresy['line']}")
    logger.info(f"Anomaly Signature: {target_heresy['error']}")

    # 2. Awaken the Local Brain
    registry = LLMRegistry()
    await registry.discover()
    available = registry.get_available_for_generation()
    if not available:
        logger.error("No local LLM muscles available. Awaken Ollama.")
        return

    # Use the fastest available coder or default
    brain = next((a for a in available if "coder" in a.adapter_id.lower()), available[0])
    logger.info(f"Cognitive Engine Engaged: {brain.adapter_id}")

    # 3. Formulate the Self-Reflection Prompt
    prompt = f"""
You are CYNIC's autonomous healing system.
A SyntaxError (INDENTATION_FRACTURE) was detected in your own codebase.

File: {target_heresy['file']}
Line: {target_heresy['line']}
Error: {target_heresy['error']}

Context Snippet:
```python
{target_heresy['context']}
```

Your objective is to fix this file. You must return EXACTLY ONE tool call using the 'edit' or 'replace' logic to fix the indentation.
The bug is almost certainly a 'logger' call that is missing 4 spaces of indentation after an 'except Exception as e:' block.

Reply ONLY with a JSON object representing the tool call:
{{
  "name": "edit",
  "arguments": {{
    "path": "{target_heresy['file']}",
    "old_string": "<exact 2 lines of broken code>",
    "new_string": "<exact 2 lines of fixed code>"
  }}
}}
"""

    req = LLMRequest(
        system="You are an autonomous self-repair OS. Reply with JSON tool calls only.",
        prompt=prompt,
        metadata={"keep_alive": 0}
    )

    logger.info("Reasoning...")
    response = await brain.complete(req)
    
    # 4. Parse the Brain's Intention
    try:
        # Extremely primitive JSON extraction for the sake of the ordeal
        content = response.content
        if "```json" in content:
            content = content.split("```json")[1].split("```")[0]
        elif "```" in content:
            content = content.split("```")[1].split("```")[0]
            
        intention = json.loads(content.strip())
        logger.info(f"Intention Formulated: {intention['name']} on {intention['arguments'].get('path')}")
        
    except Exception as e:
        logger.error(f"Brain failed to formulate a valid intention: {e}\nRaw Output: {response.content}")
        return

    # 5. Execute via Safegaurded ToolExecutor (HeresyFilter Active)
    logger.info("Executing Somatic Action (Safeguarded by HeresyFilter)...")
    executor = ToolExecutor()
    
    call = ToolCall(
        call_id="auto-heal-1",
        name=intention["name"],
        arguments=intention["arguments"]
    )
    
    result = await executor.execute(call)
    
    if result.error or getattr(result, 'blocked', False):
        logger.error(f"Somatic Action Rejected: {result.error}")
        logger.info("The HeresyFilter successfully protected the organism from a bad mutation.")
    else:
        logger.info(f"Somatic Action Successful: {result.output}")
        logger.info("✅ CYNIC HAS HEALED ITSELF.")

if __name__ == "__main__":
    asyncio.run(autonomous_heal())
