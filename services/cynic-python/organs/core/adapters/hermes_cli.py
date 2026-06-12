# Tier 3
import subprocess
import logging
import time
import json
import os
from pathlib import Path
from typing import Tuple, Optional

from ..ports.execution import ExecutorPort

logger = logging.getLogger("hermes_cli_adapter")

def extract_domain_guidance(skill_content: str) -> str:
    if not skill_content:
        return ""

    domain_weights = []
    for line in skill_content.split('\n'):
        if "avg confidence" in line and "Domain:" in line:
            try:
                parts = line.split("Domain:")
                if len(parts) > 1:
                    domain_part = parts[0].split("**")[-1].strip()
                    confidence_parts = parts[1].split("avg confidence ")
                    if len(confidence_parts) > 1:
                        conf_str = confidence_parts[1].split(',')[0].strip()
                        domain_weights.append((domain_part, float(conf_str)))
            except (ValueError, IndexError):
                pass

    if not domain_weights:
        return ""

    domain_weights.sort(key=lambda x: x[1], reverse=True)
    guidance = "\nDOMAIN EXPLORATION WEIGHTS (based on learned confidence):\n"
    for domain, conf in domain_weights:
        weight = conf / max(w[1] for w in domain_weights) if domain_weights else 0
        guidance += f"  {domain}: confidence={conf:.3f}, weight={weight:.2%}\n"

    guidance += "\nPrioritize domains with higher confidence.\n"
    return guidance

def load_skill(organ_dir: str) -> str:
    skill_path = Path(organ_dir) / "SKILL.md"
    if not skill_path.exists():
        skill_path = Path(organ_dir) / "agent" / "SKILL.md"
    if not skill_path.exists():
        return ""
    try:
        return skill_path.read_text()
    except IOError:
        return ""


class HermesCliAdapter(ExecutorPort):
    def execute(self, task: dict, organ_dir: str) -> Tuple[Optional[str], Optional[str]]:
        task_id = task.get("id", "?")
        objective = task.get("objective", task.get("content", ""))
        actions = task.get("actions", [])
        domain = task.get("domain", "unknown")

        logger.info(f"executing task {task_id}: domain={domain}")

        action_str = "\n".join(f"  - {a}" for a in actions)
        skill_context = load_skill(organ_dir)

        prompt = f"""
TASK: {objective}

Domain focus: {domain}
Task ID: {task_id}

Actions to execute:
{action_str}

Success criteria:
- Complete the actions
- Post observations to /observe endpoint
- Include narratives and signal_score in observations
- If editing this repo, only touch the repo targets already claimed
"""

        max_skill_chars = 2000
        if skill_context:
            domain_guidance = extract_domain_guidance(skill_context)
            prompt += domain_guidance
            truncated = skill_context[:max_skill_chars]
            if len(skill_context) > max_skill_chars:
                truncated += f"\n\n(... truncated)\n"
            prompt += f"\nCURRENT ORGANISM KNOWLEDGE:\n\n{truncated}\n"
        else:
            prompt += "\n(No learned patterns yet — establish baseline observations)\n"

        prompt += f"\nYour domain ({domain}) and the patterns above should guide your exploration."

        start_time = time.time()
        try:
            result = subprocess.run(
                ["hermes", "chat", "-q", prompt, "--quiet"],
                capture_output=True,
                text=True,
                timeout=600,
            )
            
            latency_ms = int((time.time() - start_time) * 1000)
            ledger = os.environ.get("CYNIC_COST_LEDGER")
            if ledger:
                try:
                    with open(ledger, "a") as f:
                        f.write(json.dumps({
                            "feature_id": "hermes_agent",
                            "compute_class": "tailnet",
                            "provider": "qwen36-27b-gpu",
                            "latency_ms": latency_ms,
                            "trace_id": task_id
                        }) + "\n")
                except Exception as e:
                    logger.warning(f"Failed to write cost ledger: {e}")

            if result.returncode == 0:
                output = result.stdout[:500] if result.stdout else "(no output)"
                return output, None
            else:
                error_msg = result.stderr[:500] if result.stderr else "(no error message)"
                return None, error_msg

        except FileNotFoundError:
            return None, "hermes CLI not found."
        except subprocess.TimeoutExpired:
            return None, "task execution timed out (10 min)"
        except Exception as e:
            return None, f"task execution failed: {e}"
