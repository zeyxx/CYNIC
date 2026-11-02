"""Session-scoped claim heartbeat lifecycle for the Hermes adapter."""

from __future__ import annotations

import hashlib
import logging
import threading
from dataclasses import dataclass
from threading import Event, Thread
from typing import Callable

_LOGGER = logging.getLogger(__name__)
DEFAULT_CLAIM_TTL_SECONDS = 1800
DEFAULT_HEARTBEAT_INTERVAL_SECONDS = 300


def agent_id_for_session(session_id: str) -> str:
    """Return a stable, non-reversible CYNIC identity for one Hermes session."""
    if not isinstance(session_id, str) or not session_id.strip():
        raise ValueError("session_id is required")
    digest = hashlib.sha256(session_id.encode("utf-8")).hexdigest()[:24]
    return f"hermes:session:{digest}"


def heartbeat_loop(
    stop_event: Event,
    heartbeat: Callable[[], None],
    interval_seconds: float,
) -> None:
    """Refresh a live session's claims until session finalization."""
    if interval_seconds <= 0:
        raise ValueError("interval_seconds must be positive")
    while not stop_event.wait(interval_seconds):
        heartbeat()


@dataclass
class _SessionWorker:
    stop_event: Event
    thread: Thread | None = None


class SessionLeaseLifecycle:
    """Run claim heartbeats for the session lifetime and release at finalization."""

    def __init__(
        self,
        heartbeat: Callable[[str], None],
        release: Callable[[str], None],
        interval_seconds: float = DEFAULT_HEARTBEAT_INTERVAL_SECONDS,
        on_error: Callable[[str, str, Exception], None] | None = None,
        join_timeout_seconds: float = 10,
    ) -> None:
        if interval_seconds <= 0 or interval_seconds >= DEFAULT_CLAIM_TTL_SECONDS:
            raise ValueError("interval_seconds must be positive and shorter than the claim TTL")
        self._heartbeat = heartbeat
        self._release = release
        self._interval_seconds = interval_seconds
        self._join_timeout_seconds = join_timeout_seconds
        self._on_error = on_error or self._log_error
        self._lock = threading.Lock()
        self._workers: dict[str, _SessionWorker] = {}
        self._closed_sessions: set[str] = set()

    @staticmethod
    def _log_error(agent_id: str, phase: str, error: Exception) -> None:
        _LOGGER.warning("CYNIC session claim %s failed for %s: %s", phase, agent_id, error)

    def _safe_heartbeat(self, agent_id: str) -> None:
        try:
            self._heartbeat(agent_id)
        except Exception as error:  # A failed renewal must not fake liveness.
            self._on_error(agent_id, "heartbeat", error)

    def _run_session_heartbeats(self, agent_id: str, worker: _SessionWorker) -> None:
        self._safe_heartbeat(agent_id)
        heartbeat_loop(
            worker.stop_event,
            lambda: self._safe_heartbeat(agent_id),
            self._interval_seconds,
        )

    def start_session(self, session_id: str) -> None:
        if not session_id:
            return
        agent_id = agent_id_for_session(session_id)
        with self._lock:
            worker = self._workers.get(session_id)
            if worker is not None and worker.thread is not None and not worker.thread.is_alive():
                self._workers.pop(session_id, None)
                worker = None
            if session_id in self._closed_sessions or worker is not None:
                return
            worker = _SessionWorker(stop_event=Event())
            thread = Thread(
                target=self._run_session_heartbeats,
                args=(agent_id, worker),
                name=f"cynic-claim-heartbeat-{agent_id[-8:]}",
                daemon=True,
            )
            worker.thread = thread
            self._workers[session_id] = worker
            try:
                thread.start()
            except Exception as error:
                self._workers.pop(session_id, None)
                self._on_error(agent_id, "heartbeat_start", error)

    def _stop_session_heartbeat(self, session_id: str) -> bool:
        with self._lock:
            worker = self._workers.get(session_id)
        if worker is None:
            return True
        worker.stop_event.set()
        if worker.thread is not None and worker.thread is not threading.current_thread():
            worker.thread.join(timeout=self._join_timeout_seconds)
        stopped = worker.thread is None or not worker.thread.is_alive()
        if stopped:
            with self._lock:
                if self._workers.get(session_id) is worker:
                    self._workers.pop(session_id, None)
        return stopped

    def finalize_session(self, session_id: str) -> bool:
        if not session_id:
            return False
        if not self._stop_session_heartbeat(session_id):
            return False
        with self._lock:
            if session_id in self._closed_sessions:
                return True
        agent_id = agent_id_for_session(session_id)
        try:
            self._release(agent_id)
        except Exception as error:
            self._on_error(agent_id, "release", error)
            return False
        with self._lock:
            self._closed_sessions.add(session_id)
        return True

    def reset_session(self, old_session_id: str | None) -> bool:
        if not old_session_id:
            return False
        return self.finalize_session(old_session_id)
