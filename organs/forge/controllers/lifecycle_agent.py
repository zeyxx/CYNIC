import asyncio
import json
import logging
from urllib import request, error
from ..src.adapters.kernel_adapter import KernelRestClient, delegate_intent

logger = logging.getLogger(__name__)

class LifecycleAgentController:
    def __init__(self, repo_path: str):
        self.repo_path = repo_path
        self.kernel_client = KernelRestClient()
        self.domain = self.kernel_client.lifecycle["task_domain"]
        self.kind = self.kernel_client.lifecycle["task_kind"]

    def _fetch_pending_tasks(self) -> list[dict]:
        url = f"{self.kernel_client.base_url}/agent-tasks?kind={self.kind}&domain={self.domain}&status=pending"
        req = request.Request(url, headers={"Authorization": f"Bearer {self.kernel_client.api_key}"})
        try:
            with request.urlopen(req, timeout=10) as resp:
                return json.loads(resp.read().decode())
        except Exception as e:
            logger.error(f"Failed to fetch pending tasks: {e}")
            return []

    def _claim_task(self, task_id: str) -> bool:
        url = f"{self.kernel_client.base_url}/agent-tasks/{task_id}/claim"
        req = request.Request(url, method="POST", headers={"Authorization": f"Bearer {self.kernel_client.api_key}"})
        try:
            with request.urlopen(req, timeout=10) as resp:
                return resp.status in (200, 201)
        except error.HTTPError as e:
            logger.warning(f"Failed to claim task {task_id}: HTTP {e.code}")
            return False
        except Exception as e:
            logger.error(f"Error claiming task {task_id}: {e}")
            return False

    def _submit_result(self, task_id: str, success: bool, result_data: dict):
        url = f"{self.kernel_client.base_url}/agent-tasks/{task_id}/result"
        payload = json.dumps({
            "status": "completed" if success else "failed",
            "result": json.dumps(result_data)
        }).encode()
        
        req = request.Request(
            url, 
            data=payload, 
            method="POST", 
            headers={
                "Authorization": f"Bearer {self.kernel_client.api_key}",
                "Content-Type": "application/json"
            }
        )
        try:
            with request.urlopen(req, timeout=10) as resp:
                return resp.status in (200, 201)
        except Exception as e:
            logger.error(f"Failed to submit result for task {task_id}: {e}")
            return False

    async def execute_task(self, task: dict):
        task_id = task.get("id")
        content_raw = task.get("content", "{}")
        
        try:
            content = json.loads(content_raw)
        except json.JSONDecodeError:
            self._submit_result(task_id, False, {"error": "Invalid JSON content"})
            return

        intent = content.get("intent")
        if not intent:
            self._submit_result(task_id, False, {"error": "Missing intent in task content"})
            return

        logger.info(f"Executing intent: {intent}")
        try:
            # Reusing the existing delegate_intent logic from entrypoints/cli.py
            result = await delegate_intent(self.repo_path, intent, emit_logs=False)
            self._submit_result(task_id, True, {"intent": intent, "execution": result})
        except Exception as e:
            logger.error(f"Task execution failed: {e}")
            self._submit_result(task_id, False, {"error": str(e), "intent": intent})

    async def run_loop(self):
        logger.info("Forge Lifecycle Agent starting autonomous mempool consumption...")
        while True:
            try:
                tasks = self._fetch_pending_tasks()
                for task in tasks:
                    task_id = task.get("id")
                    if self._claim_task(task_id):
                        logger.info(f"Claimed task {task_id}, processing...")
                        await self.execute_task(task)
            except Exception as e:
                logger.error(f"Agent loop error: {e}")
            
            # Organic heartbeat interval
            await asyncio.sleep(30)
