"""
CYNIC Kernel Lifecycle Lock — Atomic Initialization

File-based locking system ensuring only one CYNIC kernel is initialized,
even when multiple MCP bridges (Claude Code instances) start concurrently.

Lock file: ~/.cynic/kernel.lock
Owner info: {pid, hostname, timestamp}
Lock timeout: 60 seconds
Stale detection: Check if PID alive, release if dead
"""

import json
import logging
import os
import platform
import time
from pathlib import Path

logger = logging.getLogger(__name__)

LOCK_DIR = Path.home() / ".cynic"
LOCK_FILE = LOCK_DIR / "kernel.lock"
LOCK_TIMEOUT = 60.0
STALE_THRESHOLD = 300.0  # 5 minutes


class KernelLockError(Exception):
    """Base exception for kernel lock operations."""
    pass


class KernelLockTimeout(KernelLockError):
    """Lock acquisition timed out."""
    pass


class KernelLockConflict(KernelLockError):
    """Lock is held by another process."""
    pass


class KernelLockInvalid(KernelLockError):
    """Lock file is corrupted or invalid."""
    pass


def _is_process_alive(pid: int) -> bool:
    """Check if a process with given PID is still running."""
    if pid <= 0:
        return False

    try:
        if platform.system() == "Windows":
            import psutil
            return psutil.pid_exists(pid)
        else:
            # Unix: signal 0 doesn't kill but checks if process exists
            os.kill(pid, 0)
            return True
    except (ImportError, OSError, ProcessLookupError):
        return False


class KernelLockManager:
    """Atomic kernel initialization lock manager."""

    def __init__(self):
        self.lock_file = LOCK_FILE
        self.timeout = LOCK_TIMEOUT
        self.stale_threshold = STALE_THRESHOLD
        self.owner_pid: int | None = None
        self.owner_hostname: str | None = None
        self.acquired_at: float | None = None

    async def acquire(self, timeout: float = LOCK_TIMEOUT) -> bool:
        """
        Acquire kernel lock (serialize initialization).

        Returns:
            True if lock acquired, False if timeout

        Raises:
            KernelLockError: On invalid/corrupted lock
            KernelLockConflict: If lock held by running process
        """
        start_time = time.time()
        attempt = 0

        while time.time() - start_time < timeout:
            attempt += 1

            # Try to create lock file atomically
            try:
                LOCK_DIR.mkdir(exist_ok=True, parents=True)
            except OSError as e:
                logger.error("Failed to create lock directory: %s", e)
                raise KernelLockError(f"Cannot create lock directory: {e}") from e

            # Check if lock already exists
            if self.lock_file.exists():
                try:
                    lock_data = json.loads(self.lock_file.read_text())
                    holder_pid = lock_data.get("pid")
                    holder_hostname = lock_data.get("hostname")
                    lock_age = time.time() - lock_data.get("timestamp", 0)

                    logger.debug(
                        "Lock exists: PID=%s, hostname=%s, age=%.1fs",
                        holder_pid, holder_hostname, lock_age
                    )

                    # Check if lock holder is alive
                    if holder_pid and _is_process_alive(holder_pid):
                        # Lock held by running process
                        remaining = timeout - (time.time() - start_time)
                        logger.debug(
                            "Lock held by PID %s, waiting (remaining: %.1fs)",
                            holder_pid, remaining
                        )
                        await asyncio.sleep(min(1.0, remaining))
                        continue

                    # Lock is stale, force release
                    if lock_age > self.stale_threshold:
                        logger.warning(
                            "Releasing stale lock: PID=%s age=%.1fs",
                            holder_pid, lock_age
                        )
                        try:
                            self.lock_file.unlink()
                        except OSError:
                            pass

                except (json.JSONDecodeError, KeyError) as e:
                    logger.warning("Lock file corrupted: %s", e)
                    try:
                        self.lock_file.unlink()
                    except OSError:
                        pass
                    raise KernelLockInvalid(f"Invalid lock file: {e}") from e

            # Try to write our lock
            current_pid = os.getpid()
            current_hostname = platform.node()

            lock_data = {
                "pid": current_pid,
                "hostname": current_hostname,
                "timestamp": time.time(),
            }

            try:
                self.lock_file.write_text(json.dumps(lock_data, indent=2))
                self.owner_pid = current_pid
                self.owner_hostname = current_hostname
                self.acquired_at = time.time()

                logger.info(
                    "Kernel lock acquired (PID=%s, attempt=%d)",
                    current_pid, attempt
                )
                return True

            except OSError as e:
                logger.debug("Failed to write lock file: %s", e)
                await asyncio.sleep(0.1)

        elapsed = time.time() - start_time
        logger.error(
            "Failed to acquire kernel lock after %.1fs (timeout=%.1fs)",
            elapsed, timeout
        )
        raise KernelLockTimeout(f"Could not acquire lock within {timeout}s")

    async def release(self) -> bool:
        """
        Release kernel lock.

        Returns:
            True if released, False if not owned
        """
        if not self.lock_file.exists():
            return False

        try:
            lock_data = json.loads(self.lock_file.read_text())
            if lock_data.get("pid") != os.getpid():
                logger.warning("Cannot release lock owned by different process")
                return False

            self.lock_file.unlink()
            logger.info("Kernel lock released")
            return True

        except (json.JSONDecodeError, OSError) as e:
            logger.error("Failed to release lock: %s", e)
            return False

    async def is_held(self) -> bool:
        """Check if lock is currently held."""
        if not self.lock_file.exists():
            return False

        try:
            lock_data = json.loads(self.lock_file.read_text())
            holder_pid = lock_data.get("pid")
            return holder_pid and _is_process_alive(holder_pid)
        except (json.JSONDecodeError, OSError):
            return False

    async def get_holder(self) -> tuple[int, str, float] | None:
        """Get lock holder info: (pid, hostname, timestamp)."""
        if not self.lock_file.exists():
            return None

        try:
            lock_data = json.loads(self.lock_file.read_text())
            return (
                lock_data.get("pid"),
                lock_data.get("hostname"),
                lock_data.get("timestamp"),
            )
        except (json.JSONDecodeError, OSError):
            return None


# Module-level singleton
_lock_manager: KernelLockManager | None = None


def get_lock_manager() -> KernelLockManager:
    """Get or create the kernel lock manager singleton."""
    global _lock_manager
    if _lock_manager is None:
        _lock_manager = KernelLockManager()
    return _lock_manager


# Async support
import asyncio
