# Tier 3
import os
import json
import logging
import time
import requests
from datetime import datetime
from pathlib import Path
from typing import Optional, List, Dict
from urllib.parse import quote

from cynic_observe_runtime.adapters.kernel_observe import KernelObserveTransportMixin
from cynic_reliability_runtime.adapters.kernel_service_liveness import KernelServiceLivenessMixin
from cynic_coord_runtime.ports.perception import KernelPort

logger = logging.getLogger("kernel_rest_adapter")

class KernelRestAdapter(KernelObserveTransportMixin, KernelServiceLivenessMixin, KernelPort):
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

    def _collect_pending_tasks(self, limit: int = 1) -> List[Dict]:
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

    def wait_for_tasks(self, timeout_secs: float, limit: int = 1) -> List[Dict]:
        timeout_secs = max(0.0, float(timeout_secs))
        if not self.kernel_addr or not self.auth_key:
            return []

        watch_timeout = max(1.0, timeout_secs)
        try:
            response = requests.get(
                f"{self.kernel_addr}/agent-tasks/watch",
                headers=self._headers(),
                params={"limit": str(limit), "timeout_secs": str(int(watch_timeout))},
                timeout=watch_timeout + 2.0,
            )
            if response.status_code == 200:
                payload = response.json()
                tasks = payload.get("tasks", payload) if isinstance(payload, dict) else payload
                if isinstance(tasks, list):
                    normalized: list[Dict] = []
                    for task in tasks:
                        if not isinstance(task, dict):
                            continue
                        content_val = task.get("content")
                        if isinstance(content_val, str):
                            try:
                                parsed_content = json.loads(content_val)
                                if isinstance(parsed_content, dict):
                                    task.update(parsed_content)
                            except json.JSONDecodeError:
                                pass
                        elif isinstance(content_val, dict):
                            task.update(content_val)
                        task["_source"] = "kernel"
                        task["id"] = task.get("id", task.get("agent_id", ""))
                        if self._task_matches(task):
                            normalized.append(task)
                    if normalized:
                        return normalized[:limit]
                    return []
            if response.status_code not in {404, 405, 501}:
                logger.warning("kernel /agent-tasks/watch failed: HTTP %s", response.status_code)
        except requests.RequestException as e:
            logger.warning("kernel /agent-tasks/watch unavailable: %s", e)

        return []

    def poll_tasks(self, limit: int = 1) -> List[Dict]:
        return self._collect_pending_tasks(limit=limit)

    def claim_task(self, task: dict, agent_id: str) -> bool:
        task_id = task.get("id", "")
        source = task.get("_source", "local")
        
        if source == "kernel":
            try:
                response = requests.post(
                    f"{self.kernel_addr}/agent-tasks/{quote(str(task_id), safe='')}/claim",
                    headers=self._headers(),
                    json={"agent_id": agent_id},
                    timeout=5,
                )
                if response.status_code >= 400:
                    logger.warning(
                        "kernel task claim failed for %s: HTTP %s",
                        task_id,
                        response.status_code,
                    )
                    return False
                data = response.json()
                if not isinstance(data, dict) or data.get("claimed") is not True:
                    return False
            except (requests.RequestException, ValueError) as e:
                logger.warning(f"kernel task claim unreachable for {task_id}: {e}")
                return False

            lock_file = Path(self.organ_dir) / "agent-tasks" / f".{task_id}.lock"
            try:
                lock_file.parent.mkdir(parents=True, exist_ok=True)
                lock_file.write_text(json.dumps({
                    "pid": os.getpid(),
                    "claimed_at": datetime.now().isoformat() + "Z",
                    "source": "kernel",
                    "agent_id": agent_id,
                }))
                return True
            except OSError as e:
                logger.warning(f"Failed to create coordination lock for task {task_id}: {e}")
                return False
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

    def coord_register(self, agent_id: str, agent_type: str, intent: str) -> Optional[str]:
        try:
            payload = {
                "agent_id": agent_id,
                "agent_type": agent_type,
                "intent": intent,
            }
            response = requests.post(
                f"{self.kernel_addr}/coord/register",
                headers=self._headers(),
                json=payload,
                timeout=5,
            )
            if response.status_code >= 400:
                return f"coord register failed: HTTP {response.status_code}"
        except requests.RequestException as e:
            return f"coord register unreachable: {e}"
        return None

    def coord_claim_targets(self, agent_id: str, targets: List[str]) -> Optional[str]:
        payload = {"agent_id": agent_id, "targets": targets, "claim_type": "file"}
        try:
            response = requests.post(
                f"{self.kernel_addr}/coord/claim-batch",
                headers=self._headers(),
                json=payload,
                timeout=5,
            )
            if response.status_code >= 400:
                return f"coord claim failed: HTTP {response.status_code}"
            data = response.json()
            conflicts = data.get("conflicts", []) if isinstance(data, dict) else []
            if conflicts:
                return f"coord conflicts: {json.dumps(conflicts)}"
        except (requests.RequestException, ValueError) as e:
            return f"coord claim unreachable: {e}"
        return None

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
