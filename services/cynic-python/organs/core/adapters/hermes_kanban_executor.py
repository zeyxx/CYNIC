# Tier 3
import subprocess
import logging
import time
import json
import os
from typing import Tuple, Optional

from ..ports.execution import ExecutorPort

logger = logging.getLogger("hermes_kanban_adapter")


class HermesKanbanAdapter(ExecutorPort):
    def execute(self, task: dict, organ_dir: str) -> Tuple[Optional[str], Optional[str]]:
        task_id = task.get("id", "?")
        objective = task.get("objective", task.get("content", ""))
        actions = task.get("actions", [])
        domain = task.get("domain", "unknown")

        logger.info(f"executing task {task_id} via Hermes Kanban: domain={domain}")

        action_str = "\n".join(f"  - {a}" for a in actions)
        body = f"Domain: {domain}\nCYNIC Task ID: {task_id}\n\nActions:\n{action_str}"

        # 1. Create the task in triage
        create_cmd = [
            "hermes", "kanban", "create", objective,
            "--body", body,
            "--triage", "--json"
        ]
        
        try:
            result = subprocess.run(create_cmd, capture_output=True, text=True, check=True)
            k_task = json.loads(result.stdout)
            k_id = k_task.get("id")
            if not k_id:
                return None, "Hermes kanban create failed: no ID returned"
        except subprocess.CalledProcessError as e:
            return None, f"Hermes kanban create failed: {e.stderr}"
        except json.JSONDecodeError:
            return None, "Hermes kanban create returned invalid JSON"

        logger.info(f"Kanban task created: {k_id}")

        # 2. Decompose the task (planning swarm)
        decompose_cmd = ["hermes", "kanban", "decompose", k_id, "--json"]
        try:
            subprocess.run(decompose_cmd, capture_output=True, text=True, check=True)
            logger.info(f"Kanban task {k_id} decomposed successfully.")
        except subprocess.CalledProcessError as e:
            logger.warning(f"Decompose failed, task may remain in triage: {e.stderr}")

        # 3. Watch for completion
        # We poll every 10 seconds. Timeout after 10 minutes (600s).
        start_time = time.time()
        timeout = 600

        while time.time() - start_time < timeout:
            show_cmd = ["hermes", "kanban", "show", k_id, "--json"]
            try:
                res = subprocess.run(show_cmd, capture_output=True, text=True, check=True)
                current = json.loads(res.stdout)
                
                # hermes kanban show returns {"task": {"status": "..."}}
                status = current.get("task", {}).get("status")

                if status == "done":
                    # Extract the full context (including comments, subtasks results) as the final output
                    ctx_res = subprocess.run(
                        ["hermes", "kanban", "context", k_id],
                        capture_output=True, text=True
                    )
                    
                    # Update cost ledger if defined
                    self._update_ledger(task_id, start_time)
                    return ctx_res.stdout[:2000], None
                    
                elif status in ["archived", "blocked"]:
                    return None, f"Kanban task entered failure/blocked status: {status}"

            except subprocess.CalledProcessError as e:
                logger.error(f"Failed to check status of {k_id}: {e.stderr}")
            except json.JSONDecodeError:
                pass

            time.sleep(10)

        return None, f"Kanban task {k_id} timed out after {timeout} seconds"

    def _update_ledger(self, task_id: str, start_time: float):
        latency_ms = int((time.time() - start_time) * 1000)
        ledger = os.environ.get("CYNIC_COST_LEDGER")
        if ledger:
            try:
                with open(ledger, "a") as f:
                    f.write(json.dumps({
                        "feature_id": "hermes_kanban",
                        "compute_class": "tailnet",
                        "provider": "qwen36-27b-gpu",
                        "latency_ms": latency_ms,
                        "trace_id": task_id
                    }) + "\n")
            except Exception as e:
                logger.warning(f"Failed to write cost ledger: {e}")
