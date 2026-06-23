import asyncio
import json
import logging
from urllib import request, error
from ..src.adapters.kernel_adapter import KernelRestClient, delegate_intent
from ..src.core.entities import AgentTask

logger = logging.getLogger(__name__)

class LifecycleAgentController:
    def __init__(self, repo_path: str):
        self.repo_path = repo_path
        self.kernel_client = KernelRestClient()
        self.domain = self.kernel_client.lifecycle["task_domain"]
        self.kind = self.kernel_client.lifecycle["task_kind"]

    def _fetch_pending_tasks(self) -> list[AgentTask]:
        url = f"{self.kernel_client.base_url}/agent-tasks?kind={self.kind}&domain={self.domain}&status=pending"
        req = request.Request(url, headers={"Authorization": f"Bearer {self.kernel_client.api_key}"})
        try:
            with request.urlopen(req, timeout=10) as resp:
                raw_tasks = json.loads(resp.read().decode())
                return [AgentTask.from_dict(t) for t in raw_tasks]
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

    async def execute_task(self, task: AgentTask):
        if not task.content.intent:
            self._submit_result(task.id, False, {"error": "Missing intent in task content"})
            return
            
        if task.content.lifecycle != "kernel_release":
            self._submit_result(task.id, False, {"error": f"Unsupported lifecycle: {task.content.lifecycle}"})
            return

        logger.info(f"Executing intent: {task.content.intent} for domain {task.domain}")
        try:
            # Reusing the existing delegate_intent logic
            result = await delegate_intent(self.repo_path, task.content.intent, emit_logs=False)
            self._submit_result(task.id, True, {
                "intent": task.content.intent, 
                "deploy_target": task.content.deploy_target,
                "execution": result
            })
        except Exception as e:
            logger.error(f"Task execution failed: {e}")
            self._submit_result(task.id, False, {"error": str(e), "intent": task.content.intent})

    async def run_loop(self):
        logger.info("Forge Lifecycle Agent starting autonomous mempool consumption...")
        while True:
            try:
                tasks = self._fetch_pending_tasks()
                for task in tasks:
                    if self._claim_task(task.id):
                        logger.info(f"Claimed task {task.id}, processing intent: {task.content.intent}")
                        await self.execute_task(task)
            except Exception as e:
                logger.error(f"Agent loop error: {e}")
            
            # Organic heartbeat interval
            await asyncio.sleep(30)
