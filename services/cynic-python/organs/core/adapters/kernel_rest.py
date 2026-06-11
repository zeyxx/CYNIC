# Tier 3
import os
import json
import logging
import requests
from datetime import datetime
from pathlib import Path
from typing import Optional, List, Dict, Tuple
from urllib.parse import quote

from ..ports.perception import KernelPort

logger = logging.getLogger("kernel_rest_adapter")

def _safe_agent_component(value: str, fallback: str) -> str:
    clean = "".join(c if c.isalnum() or c in "-_" else "-" for c in value.strip())
    clean = "-".join(part for part in clean.split("-") if part)
    return clean or fallback

def hermes_coord_agent_id(task: dict) -> str:
    domain = _safe_agent_component(str(task.get("domain", "unknown")), "unknown")
    task_id = _safe_agent_component(str(task.get("id", "task")), "task")
    agent_id = f"hermes-agent-{domain}-{task_id}"
    return agent_id[:64]

def task_targets(task: dict) -> list[str]:
    raw = (
        task.get("targets")
        or task.get("repo_targets")
        or task.get("coord_targets")
        or task.get("files")
        or []
    )
    if isinstance(raw, str):
        raw = [raw]
    if not isinstance(raw, list):
        return []
    targets = []
    seen = set()
    for item in raw:
        target = str(item).strip()
        if not target or target in seen:
            continue
        seen.add(target)
        targets.append(target)
    return targets[:20]

def task_requires_repo_claims(task: dict) -> bool:
    for key in ("requires_repo_claims", "repo_write", "repo_mutation", "edits_repo"):
        if task.get(key) is True:
            return True
    if task_targets(task):
        return True

    actions = task.get("actions", [])
    action_text = " ".join(str(a) for a in actions) if isinstance(actions, list) else str(actions)
    objective = str(task.get("objective", task.get("content", "")))
    haystack = f"{task.get('domain', '')} {objective} {action_text}".lower()
    repo_words = ("repo", "file", "files", "commit", "push", "patch", "edit", "write", "modify", "anvil")
    return any(word in haystack for word in repo_words)


class KernelRestAdapter(KernelPort):
    def __init__(self, kernel_addr: str, auth_key: str, organ_dir: str, 
                 domain_allowlist: set[str], kind_allowlist: set[str], domain_denylist: set[str]):
        if kernel_addr.startswith("http://") or kernel_addr.startswith("https://"):
            self.kernel_addr = kernel_addr
        else:
            self.kernel_addr = f"http://{kernel_addr}"
        self.auth_key = auth_key
        self.organ_dir = organ_dir
        self.domain_allowlist = domain_allowlist
        self.kind_allowlist = kind_allowlist
        self.domain_denylist = domain_denylist

    def _headers(self):
        return {
            "Authorization": f"Bearer {self.auth_key}",
            "Content-Type": "application/json",
        }
        
    def _task_matches(self, task: dict) -> bool:
        domain = str(task.get("domain", "")).strip()
        kind = str(task.get("kind", "")).strip()
        if domain in self.domain_denylist:
            return False
        if self.domain_allowlist and domain not in self.domain_allowlist:
            return False
        if self.kind_allowlist and kind not in self.kind_allowlist:
            return False
        return True

    def poll_tasks(self, limit: int = 1) -> List[Dict]:
        if not self.kernel_addr or not self.auth_key:
            return []
            
        try:
            query_domains: list[Optional[str]] = sorted(self.domain_allowlist) if self.domain_allowlist else [None]
            tasks = []
            seen_ids = set()

            for domain in query_domains:
                params = {"status": "pending", "limit": str(limit)}
                if domain:
                    params["domain"] = domain
                response = requests.get(f"{self.kernel_addr}/agent-tasks", headers=self._headers(), params=params, timeout=5)
                if response.status_code != 200:
                    continue

                kernel_tasks = response.json()
                if isinstance(kernel_tasks, dict) and "tasks" in kernel_tasks:
                    kernel_tasks = kernel_tasks["tasks"]
                elif isinstance(kernel_tasks, int):
                    kernel_tasks = []

                for t in kernel_tasks:
                    if not isinstance(t, dict):
                        continue
                    
                    # Merge JSON content into task object if it's a JSON string
                    content_val = t.get("content")
                    if isinstance(content_val, str):
                        try:
                            parsed_content = json.loads(content_val)
                            if isinstance(parsed_content, dict):
                                t.update(parsed_content)
                        except json.JSONDecodeError:
                            pass
                    elif isinstance(content_val, dict):
                        t.update(content_val)

                    t["_source"] = "kernel"
                    t["id"] = t.get("id", t.get("agent_id", ""))
                    task_id = str(t.get("id", ""))
                    if task_id in seen_ids or not self._task_matches(t):
                        continue
                    seen_ids.add(task_id)
                    tasks.append(t)

            return tasks[:limit]
        except requests.RequestException as e:
            logger.warning(f"kernel /agent-tasks fetch failed: {e}")
            return []

    def claim_task(self, task: dict) -> bool:
        task_id = task.get("id", "")
        source = task.get("_source", "local")
        
        if source == "kernel":
            lock_file = Path(self.organ_dir) / "agent-tasks" / f".{task_id}.lock"
            try:
                lock_file.parent.mkdir(parents=True, exist_ok=True)
                lock_file.write_text(json.dumps({
                    "pid": os.getpid(),
                    "claimed_at": datetime.now().isoformat() + "Z",
                    "source": "kernel",
                }))
                return True
            except OSError as e:
                logger.warning(f"Failed to create coordination lock for task {task_id}: {e}")
                return True
        return False

    def complete_task(self, task: dict, result: Optional[str], error: Optional[str]) -> bool:
        if task.get("_source") != "kernel":
            return True
            
        task_id = str(task.get("id", ""))
        payload = {}
        if result is not None:
            payload["result"] = result
        if error is not None:
            payload["error"] = error

        try:
            response = requests.post(
                f"{self.kernel_addr}/agent-tasks/{quote(task_id, safe='')}/result",
                headers=self._headers(),
                json=payload,
                timeout=5,
            )
            return response.status_code < 400
        except requests.RequestException as e:
            logger.warning(f"kernel task completion unreachable for {task_id}: {e}")
            return False

    def release_task(self, task: dict, success: bool):
        task_id = task.get("id", "")
        source = task.get("_source", "local")

        if source == "kernel":
            lock_file = Path(self.organ_dir) / "agent-tasks" / f".{task_id}.lock"
            try:
                if lock_file.exists():
                    lock_file.unlink()
            except OSError as e:
                logger.warning(f"Failed to release coordination lock: {e}")

    def check_soma_gate(self, task_id: str) -> dict:
        try:
            soma_req = {
                "task_name": task_id,
                "priority": "hermes",
                "estimated_duration_secs": 300,
                "llama_url": os.environ.get("LLAMA_SERVER_URL", "http://127.0.0.1:8080"),
            }
            url = f"{self.kernel_addr}/soma/request"
            response = requests.post(url, json=soma_req, headers=self._headers(), timeout=2)

            if response.status_code == 200:
                return response.json()
            return {"decision": "allocate", "data": {"slot_id": f"{task_id}-{int(time.time())}"}}
        except requests.RequestException:
            return {"decision": "allocate", "data": {"slot_id": f"{task_id}-fallback"}}

    def coord_claim(self, task: dict) -> Tuple[str, List[str], Optional[str]]:
        agent_id = hermes_coord_agent_id(task)
        targets = task_targets(task)

        if task_requires_repo_claims(task) and not targets:
            return agent_id, targets, "repo-affecting Hermes task must declare targets before execution"

        if targets:
            # Register
            try:
                payload = {
                    "agent_id": agent_id,
                    "agent_type": "hermes-agent",
                    "intent": str(task.get("objective", task.get("content", "hermes agent task")))[:500],
                }
                requests.post(f"{self.kernel_addr}/coord/register", headers=self._headers(), json=payload, timeout=5)
            except requests.RequestException:
                pass
                
            # Claim
            payload = {"agent_id": agent_id, "targets": targets, "claim_type": "file"}
            try:
                response = requests.post(f"{self.kernel_addr}/coord/claim-batch", headers=self._headers(), json=payload, timeout=5)
                if response.status_code >= 400:
                    return agent_id, targets, f"coord claim failed: HTTP {response.status_code}"
                data = response.json()
                conflicts = data.get("conflicts", []) if isinstance(data, dict) else []
                if conflicts:
                    return agent_id, targets, f"coord conflicts: {json.dumps(conflicts)}"
            except requests.RequestException as e:
                return agent_id, targets, f"coord claim unreachable: {e}"

        return agent_id, targets, None

    def coord_release(self, agent_id: str):
        try:
            requests.post(
                f"{self.kernel_addr}/coord/release",
                headers=self._headers(),
                json={"agent_id": agent_id},
                timeout=5,
            )
        except requests.RequestException:
            pass
