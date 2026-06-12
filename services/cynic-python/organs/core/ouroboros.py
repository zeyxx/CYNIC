# Tier 3
import logging
import time
import signal
from .ports.perception import KernelPort
from .ports.execution import ExecutorPort

logger = logging.getLogger("ouroboros")

class OuroborosLifecycle:
    def __init__(self, kernel: KernelPort, executor: ExecutorPort, organ_dir: str, interval: float = 30.0):
        self.kernel = kernel
        self.executor = executor
        self.organ_dir = organ_dir
        self.running = False
        self.base_interval = interval
        self.current_interval = interval
        self.max_backoff = 300.0  # 5 minutes deep sleep

    def start(self):
        self.running = True
        
        def handle_signal(sig, frame):
            self.running = False
            logger.info("Ouroboros shutdown signal received")
            
        signal.signal(signal.SIGTERM, handle_signal)
        signal.signal(signal.SIGINT, handle_signal)
        
        logger.info(f"Ouroboros waking up in {self.organ_dir} with interval {self.base_interval}s")
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
                    
                    if not self.kernel.claim_task(task):
                        continue
                        
                    coord_agent_id, repo_targets, coord_error = self.kernel.coord_claim(task)
                    if coord_error:
                        logger.error(f"Task {task_id} blocked by repo coordination: {coord_error}")
                        self.kernel.release_task(task, success=False)
                        self.kernel.coord_release(coord_agent_id)
                        tasks_failed += 1
                        continue
                        
                    try:
                        # Check SOMA gate
                        gate = self.kernel.check_soma_gate(task_id)
                        if gate.get("decision") == "queue":
                            wait_secs = gate.get("data", {}).get("wait_secs", 5)
                            logger.warning(f"GPU saturated, Ouroboros backing off for {wait_secs}s")
                            time.sleep(wait_secs)
                            self.kernel.release_task(task, success=False)
                            # Exponential backoff on GPU saturation
                            self.current_interval = min(self.max_backoff, self.current_interval * 2)
                            continue

                        # Execute
                        result, error = self.executor.execute(task, self.organ_dir)
                        
                        if error:
                            logger.error(f"Task {task_id} failed: {error}")
                            self.kernel.complete_task(task, result=None, error=error)
                            self.kernel.release_task(task, success=False)
                            tasks_failed += 1
                        else:
                            logger.info(f"Task {task_id} completed successfully")
                            self.kernel.complete_task(task, result=result, error=None)
                            self.kernel.release_task(task, success=True)
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
        time.sleep(self.current_interval)
