"""Coordinator — main event loop wiring all mirror modules together.

Tier 2 INFRASTRUCTURE: Mirror organ daemon coordinator.

K15 Consumer: Kernel /observe (askesis) and /agent-tasks (action dispatch)
Systemd: mirror-coordinator.service
Promotion date: 2026-05-18 (initial wiring of full mirror pipeline).
Stability: active (L3 pattern integration 2026-05-20).

Input contract: valid paths for mirror_dir, behavior_log, dataset_path;
               reachable kernel at kernel_addr.
Output guarantee: run_once() returns count of events processed; never raises
                  on REST errors (logs warning, continues).
Failure modes: unreadable source files → source read_from() skips gracefully;
               unreachable kernel → REST calls log warning and continue.
Valid domains: personal behavioral mirror loop.
"""
from __future__ import annotations

import logging
import time
from datetime import datetime, timezone
from pathlib import Path

import requests
from requests.exceptions import RequestException

from organs.mirror.action import ActionDecision, ActionGate
from organs.mirror.askesis import generate_insight
from organs.mirror.checkpoint import ProfileCheckpoint
from organs.mirror.learner import OnlineLearner, ProfileDelta  # noqa: F401 (re-export canonical)
from organs.mirror.patterns import PatternDetector
from organs.mirror.predictions import PredictionStore
from organs.mirror.segmenter import Segmenter
from organs.mirror.sources.behavior import BehaviorSource
from organs.mirror.sources.x_signals import XSignalsSource

__version__ = "0.3.0"

logger = logging.getLogger(__name__)

CHECKPOINT_INTERVAL_EVENTS: int = 1000
CHECKPOINT_INTERVAL_SECONDS: float = 900.0  # 15 minutes
POLL_SLEEP_SECONDS: float = 5.0


class Coordinator:
    """Wires all mirror modules into a single event-processing daemon.

    Pipeline: L0 events → Segmenter (L1) → PatternDetector (L3) + Learner (L2)
              → askesis → kernel /observe (max 3/day)
    """

    def __init__(
        self,
        mirror_dir: Path,
        behavior_log: Path,
        dataset_path: Path,
        kernel_addr: str,
        api_key: str,
    ) -> None:
        mirror_dir.mkdir(parents=True, exist_ok=True)

        self._checkpoint_path = mirror_dir / "behavioral_profile.json"
        self._checkpoint = ProfileCheckpoint.load(self._checkpoint_path)

        self._segmenter = Segmenter()
        self._pattern_detector = PatternDetector()
        self._learner = OnlineLearner(self._checkpoint.profile)
        self._gate = ActionGate()
        self._predictions = PredictionStore(mirror_dir / "predictions.jsonl")

        self._behavior_source = BehaviorSource(behavior_log)
        self._x_signals_source = XSignalsSource(dataset_path)

        self._kernel_addr = kernel_addr
        self._auth_key = api_key

        self._events_since_checkpoint: int = 0
        self._last_checkpoint_time: float = time.monotonic()

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def run_once(self) -> int:
        """Process all currently available events across both sources."""
        total = 0
        total += self._process_source(self._behavior_source, is_x_signals=False)
        total += self._process_source(self._x_signals_source, is_x_signals=True)

        self._maybe_checkpoint()
        return total

    def run_forever(self) -> None:
        """Main daemon loop: tail, process, sleep when idle, repeat."""
        logger.info("mirror coordinator starting (version=%s)", __version__)
        while True:
            count = self.run_once()
            if count == 0:
                time.sleep(POLL_SLEEP_SECONDS)

    def shutdown(self) -> None:
        """Save final checkpoint before exit."""
        self._segmenter.flush()
        self._checkpoint.save(self._checkpoint_path)
        logger.info("mirror coordinator shutdown — checkpoint saved")

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _process_source(
        self,
        source: BehaviorSource | XSignalsSource,
        *,
        is_x_signals: bool,
    ) -> int:
        """Read and process all new events from one source.

        Behavior events flow: L0 → Segmenter → PatternDetector + Learner → askesis.
        X signals bypass segmentation (already content-level events).
        """
        cursor = self._checkpoint.cursors.get(source.name, 0)
        count = 0

        for event in source.read_from(cursor):
            if is_x_signals:
                delta = self._learner.ingest(event)
                self._predictions.check_outcomes([event])
            else:
                self._segmenter.ingest(event)
                delta = self._learner.ingest(event)

                # Feed focus_change to pattern detector for switch rate tracking
                if event.event_type == "focus_change":
                    self._pattern_detector.ingest_focus_change(event.timestamp)

            # Drain completed L1 segments → L3 patterns + L2 learner
            for segment in self._segmenter.completed_segments:
                self._pattern_detector.ingest_segment(segment)
                seg_delta = self._learner.ingest_segment(segment)
                self._maybe_emit_askesis(seg_delta)

            # Drain completed sessions → L3 patterns
            for session in self._segmenter.completed_sessions:
                self._pattern_detector.ingest_session(session)

            # L3: summarize patterns and update profile
            if not is_x_signals:
                summary = self._pattern_detector.summarize()
                changed = self._pattern_detector.update_profile(
                    self._learner.profile, summary
                )
                if changed:
                    pattern_delta = ProfileDelta(
                        changed_fields=changed,
                        max_confidence_change=0.05,
                    )
                    self._maybe_emit_askesis(pattern_delta, force=True)

            self._maybe_emit_askesis(delta)
            self._maybe_dispatch_action()

            count += 1
            self._events_since_checkpoint += 1

            if self._events_since_checkpoint >= CHECKPOINT_INTERVAL_EVENTS:
                self._save_checkpoint(source)

        self._checkpoint.cursors[source.name] = source.current_offset
        return count

    def _maybe_emit_askesis(
        self, delta: ProfileDelta, force: bool = False
    ) -> None:
        """Emit an askesis insight to the kernel if conditions are met.

        force=True bypasses the learner's confidence gate (for L3 pattern events)
        but still respects the 3/day cap.
        """
        if not force and not self._learner.should_emit_askesis(delta):
            return

        today = datetime.now(timezone.utc).date().isoformat()
        if not self._checkpoint.can_send_askesis(today):
            return

        insight = generate_insight(self._learner.profile, delta, force=force)
        if insight is None:
            return

        try:
            requests.post(
                f"http://{self._kernel_addr}/observe",
                headers={"Authorization": f"Bearer {self._auth_key}"},
                json={
                    "tool": "mirror_askesis",
                    "target": insight.insight_type.value,
                    "domain": "mirror-askesis",
                    "context": insight.message,
                    "agent_id": "mirror-agent",
                    "tags": ["mirror-askesis", insight.insight_type.value],
                },
                timeout=10,
            )
            self._checkpoint.record_askesis_sent(today)
            logger.info(
                "askesis emitted: type=%s confidence=%.3f surprise=%.3f score=%.3f",
                insight.insight_type.value,
                insight.confidence,
                insight.surprise,
                insight.score,
            )
        except RequestException:
            logger.warning("askesis POST failed (kernel unreachable?)", exc_info=True)

    def _maybe_dispatch_action(self) -> None:
        """Dispatch a Hermes agent task if ActionGate decides ACT."""
        decision = self._gate.evaluate(self._learner.profile, "x_curation")
        if decision is not ActionDecision.ACT:
            return

        content = self._gate.build_task_content(self._learner.profile)
        try:
            requests.post(
                f"http://{self._kernel_addr}/agent-tasks",
                headers={"Authorization": f"Bearer {self._auth_key}"},
                json={
                    "kind": "hermes",
                    "domain": "mirror-action",
                    "content": content,
                    "agent_id": "mirror-agent",
                },
                timeout=10,
            )
            logger.info("action dispatched: %s", content[:80])
        except RequestException:
            logger.warning("action POST failed (kernel unreachable?)", exc_info=True)

    def _maybe_checkpoint(self) -> None:
        """Save checkpoint if time-based interval has elapsed."""
        elapsed = time.monotonic() - self._last_checkpoint_time
        if elapsed >= CHECKPOINT_INTERVAL_SECONDS:
            self._save_checkpoint(source=None)

    def _save_checkpoint(self, source: BehaviorSource | XSignalsSource | None) -> None:
        """Persist current checkpoint state."""
        self._checkpoint.save(self._checkpoint_path)
        self._events_since_checkpoint = 0
        self._last_checkpoint_time = time.monotonic()
        logger.debug("checkpoint saved (source=%s)", source.name if source else "timer")
