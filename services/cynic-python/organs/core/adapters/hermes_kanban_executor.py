# Tier 3
"""
HermesKanbanAdapter — Executor Port implementation for Hermes Kanban tasks.

Défense en profondeur (deux niveaux de protection) :
  Niveau 1 (Soft Kill) : ComputeGuard.tick() dans la boucle de polling.
                         Lève BudgetExceededError si wall-clock ou compute dépassés.
  Niveau 2 (Hard Kill) : resource.setrlimit() sur le subprocess fils.
                         Le kernel OS tue le processus si CPU time ou mémoire virtuelle dépassés.
                         os.killpg() propage le SIGTERM au groupe de processus entier
                         (évite les zombies hermes enfants).

Coût : observé via kernel.observe(domain="cost") — pas de fichier local (0 scripts en prod).
"""
import os
import signal
import subprocess
import logging
import time
import json
import resource
import threading
from typing import Tuple, Optional, TYPE_CHECKING

from ..ports.execution import ExecutorPort

if TYPE_CHECKING:
    from ..compute_guard import ComputeGuard

logger = logging.getLogger("hermes_kanban_adapter")

# ── Niveau 2 constants (Hard Kill / OS layer) ─────────────────
# CPU time limit per hermes subprocess (seconds of CPU time, not wall-clock).
# Wall-clock is handled by ComputeGuard (Niveau 1).
# A legitimate hermes task uses CPU in bursts — 120s CPU time is generous.
_SUBPROCESS_CPU_LIMIT_SECS = 120

# Virtual memory cap per subprocess (bytes). 1 GB — hermes doesn't need more.
# OOM killer will hit this before swapping and taking down the host.
_SUBPROCESS_MEM_LIMIT_BYTES = 1 * 1024 * 1024 * 1024  # 1 GB


def _set_child_limits():
    """Called in the subprocess *after* fork — sets OS-level resource limits.

    This runs in the child process context (os.fork() has happened).
    resource.setrlimit is only meaningful here, not in the parent.
    Errors are non-fatal — we log and continue rather than refuse to execute.
    """
    try:
        # RLIMIT_CPU: CPU time in seconds. Raises SIGXCPU when soft limit hit,
        # SIGKILL when hard limit hit. We set both to the same value for immediate kill.
        resource.setrlimit(
            resource.RLIMIT_CPU,
            (_SUBPROCESS_CPU_LIMIT_SECS, _SUBPROCESS_CPU_LIMIT_SECS),
        )
    except (ValueError, resource.error) as e:
        # Non-fatal — wall-clock guard (Niveau 1) still protects us.
        logger.debug("Could not set RLIMIT_CPU on subprocess: %s", e)

    try:
        # RLIMIT_AS: virtual address space. Python processes can allocate a lot
        # of virtual memory even without touching it physically — this caps that.
        resource.setrlimit(
            resource.RLIMIT_AS,
            (_SUBPROCESS_MEM_LIMIT_BYTES, _SUBPROCESS_MEM_LIMIT_BYTES),
        )
    except (ValueError, resource.error) as e:
        logger.debug("Could not set RLIMIT_AS on subprocess: %s", e)


class HermesKanbanAdapter(ExecutorPort):
    def __init__(
        self,
        task_timeout_secs: int = 600,
        poll_interval_secs: float = 10.0,
        kernel=None,  # Optional[KernelPort] — for cost observe
    ):
        """
        Args:
            task_timeout_secs: Total wall-clock timeout for a task (Niveau 1 via ComputeGuard).
            poll_interval_secs: How often to poll hermes kanban show.
            kernel: Optional KernelPort for cost/event observation. If None, cost is not observed.
        """
        self.task_timeout_secs = task_timeout_secs
        self.poll_interval_secs = poll_interval_secs
        self.kernel = kernel
        # Track active subprocess for SIGTERM propagation
        self._active_proc: Optional[subprocess.Popen] = None
        self._proc_lock = threading.Lock()

    def set_kernel(self, kernel) -> None:
        """Wire the kernel after construction (avoids circular init)."""
        self.kernel = kernel

    def terminate_active(self) -> None:
        """Send SIGTERM to the active hermes subprocess group.

        Called from OuroborosLifecycle SIGTERM handler — ensures hermes chat
        doesn't become a zombie when the daemon exits.
        """
        with self._proc_lock:
            proc = self._active_proc
        if proc is None:
            return
        try:
            # os.killpg sends signal to the entire process group (hermes + its children).
            os.killpg(os.getpgid(proc.pid), signal.SIGTERM)
            logger.info("SIGTERM sent to hermes subprocess group (pid=%d)", proc.pid)
        except (ProcessLookupError, PermissionError) as e:
            logger.debug("Could not send SIGTERM to hermes subprocess: %s", e)

    def execute(
        self,
        task: dict,
        organ_dir: str,
        guard: Optional["ComputeGuard"] = None,
    ) -> Tuple[Optional[str], Optional[str]]:
        """Execute a task via hermes kanban with full circuit-breaker protection.

        Args:
            task:      Kernel task dict (id, objective, domain, actions).
            organ_dir: Organ working directory.
            guard:     Optional ComputeGuard for Niveau 1 budget enforcement.
                       If None, falls back to self.task_timeout_secs only.

        Returns:
            (result, error) — either result or error is non-None.
        """
        task_id = task.get("id", "?")
        objective = task.get("objective", task.get("content", ""))
        actions = task.get("actions", [])
        domain = task.get("domain", "unknown")
        start_time = time.time()

        logger.info("executing task %s via Hermes Kanban: domain=%s", task_id, domain)

        action_str = "\n".join(f"  - {a}" for a in actions)
        body = f"Domain: {domain}\nCYNIC Task ID: {task_id}\n\nActions:\n{action_str}"

        # ── 1. Create the kanban task ────────────────────────────────
        k_id, error = self._create_kanban_task(objective, body)
        if error:
            return None, error

        # ── 2. Decompose (planning swarm, best-effort) ───────────────
        self._decompose_kanban_task(k_id)

        # ── 3. Poll for completion with guard checks ─────────────────
        result, error = self._poll_until_done(k_id, task_id, domain, guard)

        # ── 4. Observe cost (no file — kernel.observe or silent) ─────
        elapsed_ms = int((time.time() - start_time) * 1000)
        self._observe_cost(task_id, domain, elapsed_ms, result is not None)

        return result, error

    # ── Private helpers ──────────────────────────────────────────────

    def _run_subprocess(self, cmd: list, timeout: Optional[float] = None) -> subprocess.CompletedProcess:
        """Run a subprocess with OS resource limits (Niveau 2 Hard Kill).

        Uses os.setsid() + start_new_session to create a new process group,
        enabling os.killpg() for clean group termination.
        """
        proc = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            # New session = new process group → os.killpg works
            start_new_session=True,
            # preexec_fn runs in child after fork, before exec
            # Sets RLIMIT_CPU + RLIMIT_AS on the hermes process
            preexec_fn=_set_child_limits,
        )
        with self._proc_lock:
            self._active_proc = proc
        try:
            stdout, stderr = proc.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            # Hard kill the entire process group
            try:
                os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
            except (ProcessLookupError, PermissionError):
                pass
            proc.wait()
            raise
        finally:
            with self._proc_lock:
                if self._active_proc is proc:
                    self._active_proc = None

        return subprocess.CompletedProcess(cmd, proc.returncode, stdout, stderr)

    def _create_kanban_task(self, objective: str, body: str) -> Tuple[Optional[str], Optional[str]]:
        """Create task in hermes kanban. Returns (k_id, error)."""
        create_cmd = [
            "hermes", "kanban", "create", objective,
            "--body", body,
            "--triage", "--json"
        ]
        try:
            result = self._run_subprocess(create_cmd, timeout=30)
            if result.returncode != 0:
                return None, f"Hermes kanban create failed (rc={result.returncode}): {result.stderr[:300]}"
            k_task = json.loads(result.stdout)
            k_id = k_task.get("id")
            if not k_id:
                return None, "Hermes kanban create failed: no ID returned"
            logger.info("Kanban task created: %s", k_id)
            return k_id, None
        except subprocess.TimeoutExpired:
            return None, "Hermes kanban create timed out (30s) — kernel unreachable?"
        except json.JSONDecodeError:
            return None, "Hermes kanban create returned invalid JSON"

    def _decompose_kanban_task(self, k_id: str) -> None:
        """Decompose task via planning swarm (best-effort, non-fatal)."""
        try:
            self._run_subprocess(
                ["hermes", "kanban", "decompose", k_id, "--json"],
                timeout=60,
            )
            logger.info("Kanban task %s decomposed", k_id)
        except (subprocess.TimeoutExpired, subprocess.CalledProcessError) as e:
            logger.warning("Decompose failed (non-fatal), task may remain in triage: %s", e)

    def _poll_until_done(
        self,
        k_id: str,
        task_id: str,
        domain: str,
        guard: Optional["ComputeGuard"],
    ) -> Tuple[Optional[str], Optional[str]]:
        """Poll hermes kanban show until done, archived/blocked, or budget exceeded."""
        from .compute_guard import BudgetExceededError

        deadline = time.time() + self.task_timeout_secs

        while time.time() < deadline:
            # ── Niveau 1: check guard budget ────────────────────────
            if guard is not None:
                try:
                    guard.tick(units=1)  # 1 unit per poll cycle
                except BudgetExceededError as exc:
                    logger.warning(
                        "Circuit breaker tripped for task %s: %s", task_id, exc
                    )
                    return None, str(exc)

            show_cmd = ["hermes", "kanban", "show", k_id, "--json"]
            try:
                res = self._run_subprocess(show_cmd, timeout=15)
                if res.returncode != 0:
                    logger.warning("hermes kanban show failed (rc=%d)", res.returncode)
                    self._sleep_poll()
                    continue

                current = json.loads(res.stdout)
                status = current.get("task", {}).get("status")

                if status == "done":
                    ctx_res = self._run_subprocess(
                        ["hermes", "kanban", "context", k_id],
                        timeout=15,
                    )
                    return ctx_res.stdout[:4000], None

                elif status in ("archived", "blocked"):
                    return None, f"Kanban task entered terminal status: {status}"

            except subprocess.TimeoutExpired:
                logger.warning("hermes kanban show timed out — skipping this poll")
            except json.JSONDecodeError:
                logger.warning("hermes kanban show returned invalid JSON — skipping")

            self._sleep_poll()

        # Wall-clock deadline reached (fallback if ComputeGuard not provided)
        return None, f"Kanban task {k_id} timed out after {self.task_timeout_secs}s wall-clock"

    def _sleep_poll(self) -> None:
        time.sleep(self.poll_interval_secs)

    def _observe_cost(self, task_id: str, domain: str, elapsed_ms: int, success: bool) -> None:
        """Observe compute cost via kernel (no file writes — 0 scripts en prod).

        Falls back silently if kernel is not wired (fire-and-forget).
        """
        if self.kernel is None:
            return
        try:
            self.kernel.observe(
                domain="cost",
                content={
                    "event": "task_cost",
                    "task_id": task_id,
                    "task_domain": domain,
                    "compute_class": "tailnet",
                    "provider": "hermes-kanban",
                    "latency_ms": elapsed_ms,
                    "success": success,
                },
                agent_id=f"hermes-kanban-{task_id}",
            )
        except Exception as e:
            logger.debug("Could not observe cost (fire-and-forget): %s", e)
