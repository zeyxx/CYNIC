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
from cynic.kernel.organism.metabolism.git_cortex import GitCortex

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
logger = logging.getLogger("cynic.autonomous_healing")


async def autonomous_heal():
    print("============================================================")
    print("🧬 CYNIC AUTONOMOUS HEALING : The Straight Corridor (GitOps)")
    print("============================================================")

    # 1. Access the Heresy Mine
    mine_path = Path("audit/heresy_mine.jsonl")
    if not mine_path.exists():
        logger.error("No Heresy Mine found. Run mine_heresies.py first.")
        return

    # Extract the first unhealed heresy
    target_heresy = None
    with open(mine_path, "r", encoding="utf-8") as f:
        for line in f:
            record = json.loads(line)
            if record["category"] == "INDENTATION_FRACTURE":
                target_heresy = record
                break

    if not target_heresy:
        logger.info("No suitable heresy found in the mine.")
        return

    logger.info(
        f"Target Acquired: {target_heresy['file']} at line {target_heresy['line']}"
    )

    # --- THE GITOPS CORRIDOR ---
    # 2. Establish Isolated Sandbox
    git_cortex = GitCortex()
    branch_name = f"auto-heal/E999-{target_heresy['file'].split('/')[-1].replace('.py', '')}-{target_heresy['line']}"

    sandbox_dir = await git_cortex.create_sandbox_branch(branch_name)
    if not sandbox_dir:
        logger.error("Failed to establish isolated Sandbox. Aborting healing.")
        return

    try:
        # 3. Awaken the Local Brain
        registry = LLMRegistry()
        await registry.discover()
        available = registry.get_available_for_generation()
        if not available:
            logger.error("No local LLM muscles available.")
            return

        brain = next(
            (a for a in available if "coder" in a.adapter_id.lower()), available[0]
        )
        logger.info(f"Cognitive Engine Engaged: {brain.adapter_id}")

        # 4. Formulate the Self-Reflection Prompt
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

Your objective is to fix this file. You must return EXACTLY ONE tool call using the 'edit' logic.
The bug is almost certainly a 'logger' call missing 4 spaces of indentation after an 'except' block.

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
            metadata={"keep_alive": 0},
        )

        logger.info("Reasoning inside Sandbox...")
        response = await brain.complete(req)

        # 5. Parse the Brain's Intention
        try:
            content = response.content
            if "```json" in content:
                content = content.split("```json")[1].split("```")[0]
            elif "```" in content:
                content = content.split("```")[1].split("```")[0]

            intention = json.loads(content.strip())
        except Exception as e:
            logger.error(f"Brain failed to formulate a valid intention: {e}")
            return

        # 6. Execute via Safeguarded ToolExecutor inside the Sandbox
        logger.info("Executing Somatic Action in isolated Sandbox...")
        # Point the executor to the Sandbox root instead of the live repo
        executor = ToolExecutor(cwd=str(sandbox_dir))

        call = ToolCall(
            call_id="auto-heal-1",
            name=intention["name"],
            arguments=intention["arguments"],
        )

        result = await executor.execute(call)

        if result.error or getattr(result, "blocked", False):
            logger.error(f"Somatic Action Rejected: {result.error}")
            return

        # 7. Local Falsification (Validation)
        import subprocess

        logger.info("Falsifying mutation with HeresyGuard...")
        verify_proc = subprocess.run(
            ["python", "scripts/verify_surgery.py", intention["arguments"]["path"]],
            cwd=str(sandbox_dir),
            capture_output=True,
            text=True,
        )

        if verify_proc.returncode != 0:
            logger.error(
                f"Falsification Failed! Mutation is structurally invalid. {verify_proc.stderr}"
            )
            return

        logger.info("✅ Falsification Passed. Mutation is pure.")

        # 8. Commit and Pull Request (The Submission)
        commit_msg = f"fix(core): autonomous healing of INDENTATION_FRACTURE in {target_heresy['file']}"
        pr_body = f"## CYNIC Autonomous Healing\n\n- **Target**: `{target_heresy['file']}`\n- **Anomaly**: `{target_heresy['error']}`\n- **Cognitive Engine**: `{brain.adapter_id}`\n\n*This PR was generated automatically from an isolated OpenSandbox.*"

        logger.info("Committing changes and opening Pull Request...")
        pushed = await git_cortex.commit_and_push(sandbox_dir, commit_msg)
        if pushed:
            await git_cortex.create_pull_request(
                sandbox_dir, title=commit_msg, body=pr_body
            )

    finally:
        # 9. Destroy Sandbox (UPLINK Rigor)
        await git_cortex.destroy_sandbox(sandbox_dir)


if __name__ == "__main__":
    asyncio.run(autonomous_heal())
