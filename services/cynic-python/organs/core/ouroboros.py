# Tier 3
import logging
import os
import threading
import time
import signal
from typing import Optional
from .ports.perception import KernelPort
from .ports.execution import ExecutorPort
from .compute_guard import ComputeGuard, BudgetExceededError
from .coalition import CoalitionBoundary, CoalitionVariable, CoalitionBoundaryViolationError
from .coalition_watchdog import CoalitionWatchdog, make_task_executor_watchdog, ObserverPort

logger = logging.getLogger("ouroboros")

class OuroborosLifecycle:
    def __init__(
        self,
        kernel: KernelPort,
        executor: ExecutorPort,
        organ_dir: str,
        interval: float = 30.0,
        # OS service identity — who am I in the kernel process table?
        service_id: str = "hermes-agent-executor",
        tier: int = 2,
        node: Optional[str] = None,
        capabilities: Optional[list] = None,
        version: Optional[str] = None,
        heartbeat_interval: float = 60.0,
    ):
        self.kernel = kernel
        self.executor = executor
        self.organ_dir = organ_dir
        self.running = False
        self.base_interval = interval
        self.current_interval = interval
        self.max_backoff = 300.0  # 5 minutes deep sleep
        # OS service identity
        self.service_id = service_id
        self.tier = tier
        self.node = node or os.environ.get("CYNIC_NODE", "unknown")
        self.capabilities = capabilities or ["observe", "claim", "poll_tasks"]
        self.version = version or os.environ.get("CYNIC_VERSION")
        self.heartbeat_interval = heartbeat_interval
        self._heartbeat_thread: Optional[threading.Thread] = None
        # Interruptible sleep event — set by SIGTERM handler to unblock _sleep() instantly.
        self._stop_event = threading.Event()
        # Disjoncteur thermique — Niveau 1 (Soft Kill applicatif)
        # 600s wall-clock per task, 5000 units/task, 50000 units/session (anti-loop)
        self._guard = ComputeGuard(
            task_wall_clock_secs=600.0,
            task_compute_units=5_000,
            session_compute_units=50_000,
        )

        # ── Coalition Boundary (Frontière constitutionnelle multi-agent) ──
        self._coalition = self._build_coalition()
        self._coalition_watchdog = make_task_executor_watchdog(
            observer=self._kernel_observer(),
            coalition=self._coalition,
            agent_id="ouroboros-initializing",
        )
        self._coalition_agent_id: str = "ouroboros-unknown"
        self._coalition_enabled: bool = True

        # Wire kernel into executor for cost observation (no file writes)
        if hasattr(self.executor, 'set_kernel'):
            self.executor.set_kernel(self.kernel)
        if hasattr(self.executor, 'set_coalition_watchdog'):
            self.executor.set_coalition_watchdog(self._coalition_watchdog)

    def _build_coalition(self) -> CoalitionBoundary:
        """Construit la frontière de coalition pour l'executor Ouroboros.

        Définit les variables d'état que les agents ne peuvent pas violer
        sans déclencher une violation de coalition (Option A -> validation,
        Option B -> coup d'État loggé dans le terminal).

        Ces variables sont le reflet des paramètres ComputeGuard + les limites
        de l'organisme auxquelles les agents sont soumis.
        """
        coalition = CoalitionBoundary(
            name="ouroboros_executor",
            description="Frontière constitutionnelle de l'executor Ouroboros",
        )
        coalition.declare(CoalitionVariable(
            name="max_tokens_per_task",
            description="Nombre maximum de tokens LLM par tâche",
            var_type=int, min_value=1, max_value=100_000,
        ))
        coalition.declare(CoalitionVariable(
            name="allowed_models",
            description="Modèles LLM autorisés dans l'executor",
            var_type=str,
            allowed_values=frozenset({"claude-3", "qwen-2.5", "hermes-phi", "qwen-3"}),
        ))
        coalition.declare(CoalitionVariable(
            name="task_wall_clock_secs",
            description="Temps maximum d'exécution d'une tâche (secondes)",
            var_type=float, min_value=1.0, max_value=3600.0,
            immutable=True,
        ))
        coalition.declare(CoalitionVariable(
            name="task_compute_units",
            description="Unités de calcul maximum par tâche",
            var_type=int, min_value=1, max_value=50_000,
            immutable=True,
        ))
        coalition.declare(CoalitionVariable(
            name="session_compute_units",
            description="Unités de calcul maximum par session Ouroboros",
            var_type=int, min_value=1, max_value=500_000,
            immutable=True,
        ))
        coalition.declare(CoalitionVariable(
            name="max_concurrent_tasks",
            description="Nombre maximum de tâches exécutées simultanément",
            var_type=int, min_value=1, max_value=10,
            immutable=True,
        ))
        coalition.declare(CoalitionVariable(
            name="allowed_domains",
            description="Domaines de tâches autorisés",
            var_type=str,
            allowed_values=frozenset({"forge", "hermes", "soma", "twitter", "telegram", "reflection"}),
        ))

        # Initialiser avec les valeurs ComputeGuard actuelles
        coalition.initialize("task_wall_clock_secs", 600.0)
        coalition.initialize("task_compute_units", 5_000)
        coalition.initialize("session_compute_units", 50_000)
        coalition.initialize("max_concurrent_tasks", 1)
        coalition.initialize("allowed_domains", "forge")
        coalition.initialize("max_tokens_per_task", 50_000)
        coalition.initialize("allowed_models", "qwen-2.5")

        return coalition

    def _kernel_observer(self) -> ObserverPort:
        """Crée un ObserverPort à partir du kernel pour le watchdog."""
        class _KernelObserver:
            def __init__(self, kernel: KernelPort):
                self._kernel = kernel
            def observe(self, domain: str, content: dict, agent_id: str) -> bool:
                return self._kernel.observe(domain=domain, content=content, agent_id=agent_id)
        return _KernelObserver(self.kernel)

    def _heartbeat_loop(self):
        """Background daemon thread: refreshes kernel process table TTL every heartbeat_interval."""
        while self.running:
            time.sleep(self.heartbeat_interval)
            if not self.running:
                break
            ok = self.kernel.service_heartbeat(self.service_id)
            if not ok:
                # 404 = evicted (kernel restart?) — re-register
                logger.warning("heartbeat 404 — re-registering with kernel process table")
                self.kernel.service_register(
                    service_id=self.service_id,
                    tier=self.tier,
                    node=self.node,
                    capabilities=self.capabilities,
                    version=self.version,
                )

    def start(self):
        self.running = True

        def handle_signal(sig, frame):
            logger.info("Ouroboros SIGTERM — unregistering from kernel process table")
            # Explicit shutdown announce: kernel evicts immediately, no need to wait TTL
            self.kernel.service_unregister(self.service_id)
            self.running = False
            self._stop_event.set()  # Unblock _sleep() immediately — no 30s wait
            # Niveau 2: propagate SIGTERM to active hermes subprocess group
            if hasattr(self.executor, 'terminate_active'):
                self.executor.terminate_active()
            logger.info("Ouroboros shutdown complete")

        signal.signal(signal.SIGTERM, handle_signal)
        signal.signal(signal.SIGINT, handle_signal)

        # Announce ourselves to the kernel OS process table
        self.kernel.service_register(
            service_id=self.service_id,
            tier=self.tier,
            node=self.node,
            capabilities=self.capabilities,
            version=self.version,
        )

        # Launch heartbeat background thread (daemon=True: exits with process)
        self._heartbeat_thread = threading.Thread(
            target=self._heartbeat_loop,
            name=f"ouroboros-heartbeat-{self.service_id}",
            daemon=True,
        )
        self._heartbeat_thread.start()

        logger.info(
            "Ouroboros waking up in %s | service=%s tier=%d node=%s interval=%.0fs",
            self.organ_dir, self.service_id, self.tier, self.node, self.base_interval,
        )
        self._loop()

    def _loop(self):
        tasks_executed = 0
        tasks_failed = 0

        while self.running:
            try:
                tasks = self.kernel.poll_tasks(limit=1)

                if not tasks:
                    self._sleep()
                    continue

                # We found tasks, reset backoff
                self.current_interval = self.base_interval

                for task in tasks:
                    task_id = task.get("id", "")
                    domain = task.get("domain", "unknown")

                    if not self.kernel.claim_task(task):
                        continue

                    # G2: derive the coordinated agent_id once; propagate everywhere
                    coord_agent_id, repo_targets, coord_error = self.kernel.coord_claim(task)
                    if coord_error:
                        logger.error(f"Task {task_id} blocked by repo coordination: {coord_error}")
                        self.kernel.release_task(task, success=False)
                        self.kernel.coord_release(coord_agent_id)
                        tasks_failed += 1
                        continue

                    # Dériver l'agent_id de coordination pour ce cycle de tâche
                    self._coalition_agent_id = coord_agent_id or "ouroboros-unknown"
                    self._coalition_watchdog.set_agent_id(self._coalition_agent_id)

                    try:
                        # Valider le domaine de la tâche contre la coalition
                        task_domain = task.get("domain", "unknown")
                        domain_error = self._coalition.validate("allowed_domains", task_domain)
                        if domain_error is not None:
                            logger.warning(
                                f"Task {task_id} domain '{task_domain}' blocked by coalition: "
                                f"{domain_error}"
                            )
                            self.kernel.release_task(task, success=False)
                            self.kernel.observe(
                                domain="coalition_watchdog",
                                content={
                                    "event": "coalition_blocked_task",
                                    "task_id": task_id,
                                    "task_domain": task_domain,
                                    "reason": domain_error,
                                    "agent_id": self._coalition_agent_id,
                                },
                                agent_id=self._coalition_agent_id,
                            )
                            tasks_failed += 1
                            continue

                        # G3: observe soma gate decision
                        gate = self.kernel.check_soma_gate(task_id)
                        gate_decision = gate.get("decision", "allocate")
                        self.kernel.observe(
                            domain="soma",
                            content={
                                "event": "gate_decision",
                                "task_id": task_id,
                                "task_domain": domain,
                                "decision": gate_decision,
                                "slot_id": gate.get("data", {}).get("slot_id"),
                                "wait_secs": gate.get("data", {}).get("wait_secs"),
                            },
                            agent_id=coord_agent_id,
                        )

                        if gate_decision == "queue":
                            wait_secs = gate.get("data", {}).get("wait_secs", 5)
                            logger.warning(f"GPU saturated, Ouroboros backing off for {wait_secs}s")
                            time.sleep(wait_secs)
                            self.kernel.release_task(task, success=False)
                            # Exponential backoff on GPU saturation
                            self.current_interval = min(self.max_backoff, self.current_interval * 2)
                            continue

                        # Execute with Niveau 1 circuit breaker + Option B coalition watchdog
                        with self._guard.task_context(task_id=task_id) as guard:
                            try:
                                result, error = self.executor.execute(
                                    task, self.organ_dir, guard=guard,
                                    coalition=self._coalition,
                                    coalition_agent_id=self._coalition_agent_id,
                                )
                            except CoalitionBoundaryViolationError as exc:
                                # Option B: coup d'État algorithmique intercepté
                                severity = self._coalition_watchdog.catch(exc)
                                self.kernel.observe(
                                    domain="coalition_watchdog",
                                    content=exc.to_observe_payload(),
                                    agent_id=self._coalition_agent_id,
                                )
                                result, error = None, str(exc)
                                self.current_interval = min(self.max_backoff, self.current_interval * 2)
                            except BudgetExceededError as exc:
                                # Circuit breaker tripped mid-execute
                                logger.warning("Circuit breaker tripped: %s", exc)
                                self.kernel.observe(
                                    domain="circuit_breaker",
                                    content={
                                        "event": "budget_exceeded",
                                        "task_id": task_id,
                                        "task_domain": domain,
                                        "reason": exc.reason,
                                        "elapsed_secs": exc.elapsed_secs,
                                        "budget_secs": exc.budget_secs,
                                        "session_units_used": self._guard.session_units_used,
                                    },
                                    agent_id=coord_agent_id,
                                )
                                result, error = None, str(exc)
                                # Exponential backoff after circuit breaker trip
                                self.current_interval = min(self.max_backoff, self.current_interval * 2)

                        if error:
                            logger.error(f"Task {task_id} failed: {error}")
                            self.kernel.complete_task(task, result=None, error=error)
                            self.kernel.release_task(task, success=False)
                            self.kernel.observe(
                                domain=domain,
                                content={
                                    "event": "task_failed",
                                    "task_id": task_id,
                                    "error": error[:500] if error else None,
                                    "objective": task.get("objective", task.get("content", ""))[:200],
                                },
                                agent_id=coord_agent_id,
                            )
                            tasks_failed += 1
                        else:
                            logger.info(f"Task {task_id} completed successfully")
                            self.kernel.complete_task(task, result=result, error=None)
                            self.kernel.release_task(task, success=True)
                            result_preview = result[:500] if result else None
                            obj_preview = str(task.get("objective", task.get("content", "")))[:200]
                            self.kernel.observe(
                                domain=domain,
                                content={
                                    "event": "task_completed",
                                    "task_id": task_id,
                                    "result_preview": result_preview,
                                    "objective": obj_preview,
                                },
                                agent_id=coord_agent_id,
                            )
                            tasks_executed += 1

                    finally:
                        if repo_targets:
                            self.kernel.coord_release(coord_agent_id)

            except Exception as e:
                logger.error(f"Ouroboros metabolism error: {e}")
                self.current_interval = min(self.max_backoff, self.current_interval * 1.5)

            self._sleep()

        logger.info(f"Ouroboros sleep. Executed: {tasks_executed}, Failed: {tasks_failed}")

    def _sleep(self):
        # Event.wait() returns as soon as _stop_event is set (SIGTERM) or timeout expires.
        # This replaces time.sleep() — SIGTERM response is now instantaneous.
        self._stop_event.wait(timeout=self.current_interval)
        self._stop_event.clear()  # reset for next cycle if not shutting down