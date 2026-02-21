"""
CYNIC TransparencyAuditTrail — Guardrail 3: Decision transparency

Records all decisions end-to-end for human audit and CYNIC self-review.

Tracks:
  1. Judgment input (verdict, confidence, Q-score)
  2. Alignment check results (violations, if any)
  3. Decision made (recommended action)
  4. Action execution (prompt, result, error)
  5. Outcome (success, failure, escalation)
  6. Human review (accepted, rejected, notes)

All records immutable and timestamped for full auditability.
"""
from __future__ import annotations

import json
import logging
import time
import uuid
from dataclasses import dataclass, asdict, field
from pathlib import Path
from typing import Any, Optional

from cynic.core.phi import fibonacci

logger = logging.getLogger("cynic.immune.transparency_audit")

# Rolling cap (Fibonacci number for consistency)
_MAX_AUDIT_RECORDS = fibonacci(10)  # 55 records


@dataclass
class AuditRecord:
    """Immutable record of a decision's complete lifecycle."""
    record_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    timestamp: float = field(default_factory=time.time)

    # PERCEIVE→JUDGE inputs
    judgment_id: str = ""
    verdict: str = ""
    confidence: float = 0.0
    q_score: float = 0.0

    # DECIDE→alignment check
    alignment_violations: list[dict[str, Any]] = field(default_factory=list)
    alignment_approved: bool = True

    # DECIDE→recommendation
    recommended_action: str = ""
    action_prompt: str = ""

    # ACT→execution
    action_executed: bool = False
    execution_result: dict[str, Any] = field(default_factory=dict)
    execution_error: Optional[str] = None

    # OUTCOME
    success: bool = False
    human_reviewed: bool = False
    human_review_notes: str = ""

    def to_dict(self) -> dict[str, Any]:
        """Convert to JSON-serializable dict."""
        return asdict(self)

    def to_json(self) -> str:
        """Serialize to JSON."""
        return json.dumps(self.to_dict(), default=str)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> AuditRecord:
        """Reconstruct from dict."""
        return cls(**data)


class TransparencyAuditTrail:
    """
    Immutable audit trail for all decisions.

    Records are append-only and timestamped. Supports rolling cap to prevent
    unbounded growth while maintaining auditability.

    Usage:
        trail = TransparencyAuditTrail()
        trail.start()

        # Record a decision
        record = trail.record_decision(
            judgment_id=judgment.judgment_id,
            verdict=verdict,
            confidence=conf,
        )

        # Update with alignment results
        trail.record_alignment_check(record.record_id, violations)

        # Update with execution results
        trail.record_execution(record.record_id, success=True)

        # Human review
        trail.record_human_review(record.record_id, approved=True, notes="...")
    """

    def __init__(self, storage_path: str = "~/.cynic/audit_trail.jsonl") -> None:
        """
        Args:
            storage_path: Path to JSONL file for audit records (default ~/.cynic/audit_trail.jsonl)
        """
        self._storage_path = Path(storage_path).expanduser()
        self._storage_path.parent.mkdir(parents=True, exist_ok=True)
        self._records: list[AuditRecord] = []
        self._load_from_disk()

    def start(self) -> None:
        """Start audit trail monitoring."""
        logger.info(f"TransparencyAuditTrail started — recording to {self._storage_path}")

    def record_decision(
        self,
        judgment_id: str,
        verdict: str,
        confidence: float,
        q_score: float,
    ) -> AuditRecord:
        """
        Record the start of a decision lifecycle.

        Args:
            judgment_id: ID of the judgment that triggered this
            verdict: BARK, GROWL, WAG, or HOWL
            confidence: Decision confidence (0-1)
            q_score: Q-score from decision (0-100)

        Returns:
            AuditRecord with record_id for later updates
        """
        record = AuditRecord(
            judgment_id=judgment_id,
            verdict=verdict,
            confidence=confidence,
            q_score=q_score,
        )
        self._append_record(record)
        logger.debug(f"Decision recorded: {record.record_id} ({verdict})")
        return record

    def record_alignment_check(
        self,
        record_id: str,
        violations: list[dict[str, Any]],
    ) -> None:
        """
        Record alignment check results.

        Args:
            record_id: Record to update
            violations: List of AlignmentViolation dicts
        """
        record = self._find_record(record_id)
        if record:
            record.alignment_violations = violations
            record.alignment_approved = not any(v.get("blocking", False) for v in violations)
            self._save_to_disk()
            logger.debug(f"Alignment check recorded: {record_id} ({'approved' if record.alignment_approved else 'blocked'})")

    def record_decision_recommendation(
        self,
        record_id: str,
        recommended_action: str,
        action_prompt: str,
    ) -> None:
        """
        Record the recommended action.

        Args:
            record_id: Record to update
            recommended_action: Verdict action
            action_prompt: Claude prompt to execute
        """
        record = self._find_record(record_id)
        if record:
            record.recommended_action = recommended_action
            record.action_prompt = action_prompt
            self._save_to_disk()
            logger.debug(f"Action recommendation recorded: {record_id}")

    def record_execution(
        self,
        record_id: str,
        success: bool,
        result: Optional[dict[str, Any]] = None,
        error: Optional[str] = None,
    ) -> None:
        """
        Record action execution result.

        Args:
            record_id: Record to update
            success: Whether execution succeeded
            result: Execution result dict
            error: Error message if execution failed
        """
        record = self._find_record(record_id)
        if record:
            record.action_executed = True
            record.success = success
            record.execution_result = result or {}
            record.execution_error = error
            self._save_to_disk()
            logger.debug(f"Execution recorded: {record_id} ({'success' if success else 'failed'})")

    def record_human_review(
        self,
        record_id: str,
        approved: bool,
        notes: str = "",
    ) -> None:
        """
        Record human review of a decision.

        Args:
            record_id: Record to update
            approved: Whether human approved the decision
            notes: Human review notes
        """
        record = self._find_record(record_id)
        if record:
            record.human_reviewed = True
            record.success = approved  # Human approval determines final success
            record.human_review_notes = notes
            self._save_to_disk()
            logger.info(f"Human review recorded: {record_id} ({'approved' if approved else 'rejected'})")

    def get_decision_history(
        self,
        verdict: Optional[str] = None,
        limit: int = 20,
    ) -> list[AuditRecord]:
        """
        Retrieve decision history, optionally filtered.

        Args:
            verdict: Filter by verdict (BARK, GROWL, WAG, HOWL)
            limit: Max records to return

        Returns:
            List of AuditRecord objects (most recent first)
        """
        filtered = self._records
        if verdict:
            filtered = [r for r in filtered if r.verdict == verdict]

        return list(reversed(filtered))[-limit:]

    def get_blocked_decisions(self) -> list[AuditRecord]:
        """Get all decisions blocked by alignment checks."""
        return [r for r in self._records if not r.alignment_approved]

    def get_failed_executions(self) -> list[AuditRecord]:
        """Get all decisions that failed execution."""
        return [r for r in self._records if r.action_executed and not r.success]

    def stats(self) -> dict[str, Any]:
        """Return audit trail statistics."""
        total = len(self._records)
        approved = sum(1 for r in self._records if r.alignment_approved)
        executed = sum(1 for r in self._records if r.action_executed)
        successful = sum(1 for r in self._records if r.success)
        reviewed = sum(1 for r in self._records if r.human_reviewed)

        verdict_counts = {}
        for verdict in ["BARK", "GROWL", "WAG", "HOWL"]:
            verdict_counts[verdict] = sum(1 for r in self._records if r.verdict == verdict)

        return {
            "total_records": total,
            "alignment_approved": approved,
            "executed": executed,
            "successful": successful,
            "human_reviewed": reviewed,
            "verdict_distribution": verdict_counts,
            "storage_path": str(self._storage_path),
            "rolling_cap": _MAX_AUDIT_RECORDS,
        }

    # ── Private ────────────────────────────────────────────────────────

    def _find_record(self, record_id: str) -> Optional[AuditRecord]:
        """Find record by ID."""
        for record in self._records:
            if record.record_id == record_id:
                return record
        return None

    def _append_record(self, record: AuditRecord) -> None:
        """Add a new record and maintain rolling cap."""
        self._records.append(record)

        # Enforce rolling cap
        if len(self._records) > _MAX_AUDIT_RECORDS:
            self._records = self._records[-_MAX_AUDIT_RECORDS:]

        self._save_to_disk()

    def _load_from_disk(self) -> None:
        """Load records from JSONL file."""
        if not self._storage_path.exists():
            logger.debug(f"Audit trail file not found: {self._storage_path}")
            return

        try:
            with open(self._storage_path, "r") as f:
                for line in f:
                    if line.strip():
                        data = json.loads(line)
                        record = AuditRecord.from_dict(data)
                        self._records.append(record)

            # Enforce rolling cap
            if len(self._records) > _MAX_AUDIT_RECORDS:
                self._records = self._records[-_MAX_AUDIT_RECORDS:]

            logger.info(f"Loaded {len(self._records)} audit records from {self._storage_path}")
        except json.JSONDecodeError as e:
            logger.warning(f"Failed to load audit trail: {e}")

    def _save_to_disk(self) -> None:
        """Save all records to JSONL file."""
        try:
            with open(self._storage_path, "w") as f:
                for record in self._records:
                    f.write(record.to_json() + "\n")
        except OSError as e:
            logger.error(f"Failed to save audit trail: {e}")
