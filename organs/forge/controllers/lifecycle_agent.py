import asyncio
import logging
import os
from pathlib import Path
from ..src.adapters.git_adapter import LocalGitPerceiver
from ..src.adapters.fleet_node_exec_adapter import FleetNodeExecCommandTransport
from ..src.adapters.kernel_adapter import KernelRestClient
from ..src.core.entities import AgentTask
from ..src.core.runtime_host_guard import build_runner_host_guard, runtime_host_candidates
from ..src.core.lifecycle_workflow import (
    persist_lifecycle_activation_evidence_patch,
    verify_lifecycle_runtime_paths,
)
from ..src.core.lifecycle_bootstrap_executor import BuildRunnerBootstrapExecutor
from ..src.core.lifecycle_intent_executor import LifecycleIntentExecutor
from ..src.core.organ_memory import OrganMemoryStore
from ..src.core.services import BuildDelegationService

logger = logging.getLogger(__name__)
ACTIVATION_SCHEMA_VERSION = "activation.v1"
DEFAULT_POLL_LIMIT = 5
MAX_POLL_LIMIT = 20
DEFAULT_EXTRA_DOMAINS = ("mcp-runtime-request",)


def _poll_limit_from_env() -> int:
    raw = os.environ.get("HERMES_AGENT_BATCH_LIMIT") or os.environ.get(
        "HERMES_AGENT_POLL_LIMIT", str(DEFAULT_POLL_LIMIT)
    )
    try:
        value = int(raw)
    except ValueError:
        return DEFAULT_POLL_LIMIT
    return max(1, min(value, MAX_POLL_LIMIT))


def _extra_domains_from_env() -> list[str]:
    raw = os.environ.get("HERMES_AGENT_EXTRA_DOMAINS", ",".join(DEFAULT_EXTRA_DOMAINS))
    domains = [item.strip() for item in raw.split(",") if item.strip()]
    return domains


def _runtime_root_from_repo(repo: Path) -> Path:
    marker = Path(".cynic") / "repos" / "CYNIC"
    if repo.match(f"*{marker}"):
        for parent in (repo, *repo.parents):
            if parent.name == "CYNIC" and parent.parent.name == "repos" and parent.parent.parent.name == ".cynic":
                return parent.parent.parent.parent
        fallback = repo
        for _ in range(2):
            fallback = fallback.parent
        return fallback
    raise RuntimeError(
        "organ-forge repo_path must resolve under <home>/.cynic/repos/CYNIC "
        "for runtime path verification"
    )

class LifecycleAgentController:
    def __init__(self, repo_path: str):
        self.repo_path = repo_path
        self.kernel_client = KernelRestClient()
        self.domain = self.kernel_client.lifecycle["task_domain"]
        self.kind = self.kernel_client.lifecycle["task_kind"]
        self.poll_limit = _poll_limit_from_env()
        self.poll_domains = _unique_domains([self.domain, *_extra_domains_from_env()])
        self.bootstrap_transport = FleetNodeExecCommandTransport.from_env()
        self.intent_executor = LifecycleIntentExecutor(
            repo_path,
            build_node=getattr(self.kernel_client, "build_node", {}) or {},
        )
        self.memory_store = OrganMemoryStore(Path(repo_path) / "organs" / "forge" / "state")

    async def _delegate_intent(self, intent: str) -> dict:
        perceiver = LocalGitPerceiver(self.repo_path)
        service = BuildDelegationService(perceiver, self.kernel_client)
        return await service.delegate_task(intent, emit_logs=False)

    def _runtime_home(self) -> str:
        repo = Path(self.repo_path).resolve()
        return str(_runtime_root_from_repo(repo))

    def _verify_runtime_paths(self, node_name: str) -> dict:
        return verify_lifecycle_runtime_paths(node_name, home=self._runtime_home())

    def _execute_bootstrap_contract(self, contract: dict) -> dict:
        return BuildRunnerBootstrapExecutor(
            getattr(self, "bootstrap_transport", None)
        ).execute(contract)

    def _persist_activation_evidence_patch(self, patch: dict) -> dict:
        return persist_lifecycle_activation_evidence_patch(
            patch,
            registry_path=Path(self.repo_path) / "infra" / "registry.json",
        )

    def _fetch_pending_tasks(self) -> list[AgentTask]:
        tasks: list[AgentTask] = []
        for domain in getattr(self, "poll_domains", [self.domain]):
            tasks.extend(self._fetch_pending_tasks_for_domain(domain))
        return tasks

    def _fetch_pending_tasks_for_domain(self, domain: str) -> list[AgentTask]:
        try:
            payload = self.kernel_client.kernel_tasks.list_agent_tasks(
                kind=self.kind,
                domain=domain,
                completed=False,
                limit=self.poll_limit,
            )
            raw_tasks = payload.get("tasks", []) if isinstance(payload, dict) else []
            return [AgentTask.from_dict(t) for t in raw_tasks]
        except Exception as e:
            logger.error(f"Failed to fetch pending tasks: {e}")
            return []

    def _claim_task(self, task_id: str) -> bool:
        try:
            return self.kernel_client.kernel_tasks.claim_agent_task(task_id, "organ-forge-controller")
        except Exception as e:
            logger.warning(f"Failed to claim task {task_id}: {e}")
            return False

    def _submit_result(
        self,
        task_id: str,
        success: bool,
        result_data: dict,
        *,
        domain: str | None = None,
    ):
        result_domain = domain or self.domain
        try:
            return self.kernel_client.kernel_tasks.submit_agent_task_result(
                task_id,
                success=success,
                result=result_data,
                kind=self.kind,
                domain=result_domain,
            )
        except Exception as e:
            logger.error(f"Failed to submit result for task {task_id}: {e}")
            self._spool_result_submission_failure(
                task_id,
                success,
                result_data,
                str(e),
                domain=result_domain,
            )
            return False

    def _spool_result_submission_failure(
        self,
        task_id: str,
        success: bool,
        result_data: dict,
        error: str,
        *,
        domain: str,
    ) -> None:
        store = getattr(self, "memory_store", None)
        if store is None:
            return
        try:
            store.enqueue_outbox(
                event_type="agent_task_result_submit_failed",
                destination="kernel:/agent-tasks/result",
                payload={
                    "task_id": task_id,
                    "success": success,
                    "kind": self.kind,
                    "domain": domain,
                    "result": result_data,
                    "error": error,
                },
            )
        except Exception as spool_error:
            logger.error(f"Failed to spool result for task {task_id}: {spool_error}")

    def _runtime_host_candidates(self) -> list[str]:
        return runtime_host_candidates()

    def _build_runner_guard(self, task: AgentTask) -> dict | None:
        kernel_client = getattr(self, "kernel_client", None)
        return build_runner_host_guard(
            task.content.build_target,
            build_node=getattr(kernel_client, "build_node", {}) or {},
            observed_hosts=self._runtime_host_candidates(),
            registry_build_target=getattr(kernel_client, "build_target", ""),
            intent=task.content.intent,
            task_domain=task.domain,
        )

    async def execute_task(self, task: AgentTask):
        if task.content.action:
            await self.execute_activation_task(task)
            return

        if not task.content.intent:
            self._submit_result(task.id, False, {"error": "Missing intent in task content"}, domain=task.domain)
            return
            
        if task.content.lifecycle != "kernel_release":
            self._submit_result(
                task.id,
                False,
                {"error": f"Unsupported lifecycle: {task.content.lifecycle}"},
                domain=task.domain,
            )
            return
        guard = self._build_runner_guard(task)
        if guard is not None:
            logger.warning(
                "Refusing lifecycle task %s on non-build-runner host(s) %s; expected one of %s",
                task.id,
                guard["observed_execution_hosts"],
                guard["allowed_execution_hosts"],
            )
            self._submit_result(task.id, False, guard, domain=task.domain)
            return

        try:
            result = self.intent_executor.execute(task)
        except Exception as exc:
            result = {
                "status": "failed",
                "error_kind": "lifecycle_intent_executor_crashed",
                "error": str(exc),
                "intent": task.content.intent,
                "request_profile": task.content.request_profile,
            }
        self._submit_result(task.id, result.get("status") == "ok", result, domain=task.domain)

    async def execute_activation_task(self, task: AgentTask):
        action_name = task.content.action.get("action")
        result = {
            "error_kind": "activation_action_not_implemented",
            "error": "activation action requires an organ-forge executor handler",
            "activation_schema_version": task.content.activation_schema_version,
            "correlation_id": task.content.correlation_id,
            "action": action_name,
            "targets": task.content.targets,
            "coord_targets": task.content.coord_targets,
        }
        if task.content.lifecycle != "kernel_release":
            result.update(
                {
                    "error_kind": "unsupported_lifecycle",
                    "error": f"Unsupported lifecycle: {task.content.lifecycle}",
                }
            )
            self._submit_result(task.id, False, result)
            return
        if task.content.activation_schema_version != ACTIVATION_SCHEMA_VERSION:
            result.update(
                {
                    "error_kind": "activation_schema_version_mismatch",
                    "error": f"Unsupported activation schema: {task.content.activation_schema_version}",
                }
            )
            self._submit_result(task.id, False, result)
            return
        if not task.content.correlation_id:
            result.update(
                {
                    "error_kind": "missing_correlation_id",
                    "error": "Activation task is missing correlation_id",
                }
            )
            self._submit_result(task.id, False, result)
            return
        if action_name in {"verify_runtime_path_proof", "materialize_runtime_path"}:
            self._execute_runtime_path_activation(task, result)
            return
        logger.info(
            "Activation action %s is valid but has no executor handler yet (correlation_id=%s)",
            action_name,
            task.content.correlation_id,
        )
        self._submit_result(task.id, False, result)

    def _execute_runtime_path_activation(self, task: AgentTask, base_result: dict):
        node_name = task.content.action.get("node")
        if not node_name:
            base_result.update(
                {
                    "error_kind": "missing_activation_node",
                    "error": "Runtime path activation requires action.node",
                }
            )
            self._submit_result(task.id, False, base_result)
            return
        verification = self._verify_runtime_paths(node_name)
        bootstrap_execution = None
        if verification.get("status") != "ok":
            bootstrap_contract = task.content.action.get("bootstrap_contract")
            if bootstrap_contract:
                bootstrap_execution = self._execute_bootstrap_contract(bootstrap_contract)
                if bootstrap_execution.get("status") == "ok":
                    verification = self._verify_runtime_paths(node_name)
                elif bootstrap_execution.get("status") == "requires_transport":
                    base_result.update(
                        {
                            "error_kind": "bootstrap_transport_required",
                            "error": "Bootstrap contract requires canonical fleet transport execution",
                            "verification": verification,
                            "bootstrap_execution": bootstrap_execution,
                        }
                    )
                    self._submit_result(task.id, False, base_result)
                    return
                else:
                    base_result.update(
                        {
                            "error_kind": "bootstrap_execution_failed",
                            "error": "Bootstrap contract execution failed before runtime path verification",
                            "verification": verification,
                            "bootstrap_execution": bootstrap_execution,
                        }
                    )
                    self._submit_result(task.id, False, base_result)
                    return
        if verification.get("status") != "ok":
            base_result.update(
                {
                    "error_kind": "runtime_path_not_verified",
                    "error": "Runtime path proof is not verified",
                    "verification": verification,
                }
            )
            self._submit_result(task.id, False, base_result)
            return
        persistence = self._persist_activation_evidence_patch(
            verification["activation_evidence_patch"]
        )
        if not persistence.get("registry_path"):
            base_result.update(
                {
                    "error_kind": "activation_persistence_missing_registry_path",
                    "error": "Activation evidence persistence did not report registry_path",
                    "verification": verification,
                    "persistence": persistence,
                }
            )
            self._submit_result(task.id, False, base_result)
            return
        self._submit_result(
            task.id,
            True,
            {
                "status": "ok",
                "activation_schema_version": task.content.activation_schema_version,
                "correlation_id": task.content.correlation_id,
                "action": task.content.action.get("action"),
                "node": node_name,
                "bootstrap_execution": bootstrap_execution,
                "verification": verification,
                "persistence": persistence,
            },
        )

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


def _unique_domains(domains: list[str]) -> list[str]:
    seen: set[str] = set()
    result: list[str] = []
    for domain in domains:
        if domain in seen:
            continue
        seen.add(domain)
        result.append(domain)
    return result
