"""
CYNIC HumanApprovalGate — Guardrail 4: Human-in-the-Loop

Routes high-impact decisions to human reviewers before execution.

High-impact triggers:
  1. BARK verdicts (severity=CRITICAL)
  2. Novel pattern violations (CULTURE warnings)
  3. Low alignment approval (<50% checks passing)
  4. Large action prompts (potential for extraction)
  5. Multiple consecutive failures

Blocks execution until human:
  - Approves the decision
  - Provides optional notes
  - Confirms understanding of implications
"""
from __future__ import annotations

import logging
import json
import uuid
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Optional
from enum import Enum

from cynic.core.phi import fibonacci

logger = logging.getLogger("cynic.immune.human_approval_gate")

# Threshold for blocking
_BARK_VERDICT_REQUIRES_APPROVAL = True
_LOW_ALIGNMENT_THRESHOLD = 0.5
_LARGE_PROMPT_CHARS = 800
_MAX_CONSECUTIVE_FAILURES = 2


class ApprovalStatus(Enum):
    """Status of a human approval request."""
    PENDING = "pending"
    APPROVED = "approved"
    REJECTED = "rejected"
    ESCALATED = "escalated"


@dataclass
class ApprovalRequest:
    """Request for human approval of a decision."""
    request_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    record_id: str = ""  # From audit trail

    # Decision context
    verdict: str = ""
    confidence: float = 0.0
    q_score: float = 0.0
    action_prompt: str = ""

    # Why approval is needed
    reason: str = ""
    blocking_violations: list[str] = field(default_factory=list)
    risk_level: str = "MEDIUM"  # LOW, MEDIUM, HIGH, CRITICAL

    # Approval tracking
    status: ApprovalStatus = ApprovalStatus.PENDING
    human_reviewer: str = ""  # Who reviewed it
    approval_notes: str = ""
    timestamp_created: float = 0.0
    timestamp_reviewed: Optional[float] = None

    def to_dict(self) -> dict[str, Any]:
        """Convert to JSON-serializable dict."""
        d = asdict(self)
        d["status"] = self.status.value
        return d

    def to_json(self) -> str:
        """Serialize to JSON."""
        return json.dumps(self.to_dict(), default=str)


class HumanApprovalGate:
    """
    Routes high-impact decisions to human reviewers.

    Blocks execution until human approval. Provides CLI and REST endpoints
    for human review workflow.

    Usage:
        gate = HumanApprovalGate()
        gate.start()

        # Check if decision needs approval
        if gate.requires_approval(decision, alignment_violations):
            # Create approval request
            req = gate.create_approval_request(
                verdict="BARK",
                confidence=conf,
                action_prompt=prompt,
                reason="BARK verdict detected",
            )

            # Block until human reviews
            # (via /approve/{request_id} or /reject/{request_id})

            # Check approval status
            status = gate.get_approval_status(req.request_id)
            if status == ApprovalStatus.APPROVED:
                execute_action()
    """

    def __init__(self, storage_path: str = "~/.cynic/approval_requests.jsonl") -> None:
        """
        Args:
            storage_path: Path to JSONL file for approval requests
        """
        self._storage_path = Path(storage_path).expanduser()
        self._storage_path.parent.mkdir(parents=True, exist_ok=True)
        self._requests: list[ApprovalRequest] = []
        self._consecutive_failures = 0
        self._load_from_disk()

    def start(self) -> None:
        """Start approval gate monitoring."""
        logger.info(f"HumanApprovalGate started — requests saved to {self._storage_path}")

    def requires_approval(
        self,
        decision: dict[str, Any],
        alignment_violations: list[dict[str, Any] | "AlignmentViolation"],
    ) -> bool:
        """
        Determine if decision requires human approval.

        Args:
            decision: Decision dict from DecideAgent
            alignment_violations: Results from AlignmentSafetyChecker (list of AlignmentViolation or dicts)

        Returns:
            True if human approval needed before execution
        """
        verdict = decision.get("verdict", "")
        action_prompt = decision.get("action_prompt", "")
        confidence = float(decision.get("confidence", 0.0))

        # BARK verdicts always require approval
        if _BARK_VERDICT_REQUIRES_APPROVAL and verdict == "BARK":
            return True

        # Low alignment approval rate
        if alignment_violations:
            # Handle both AlignmentViolation objects and dicts
            def is_blocking(v: Any) -> bool:
                if hasattr(v, "blocking"):  # AlignmentViolation object
                    return v.blocking
                else:  # dict
                    return v.get("blocking", False)

            pass_rate = sum(1 for v in alignment_violations if not is_blocking(v)) / len(alignment_violations)
            if pass_rate < _LOW_ALIGNMENT_THRESHOLD:
                return True

        # Large action prompts (potential extraction)
        if len(action_prompt) > _LARGE_PROMPT_CHARS:
            return True

        # Multiple consecutive failures
        if self._consecutive_failures >= _MAX_CONSECUTIVE_FAILURES:
            return True

        return False

    def create_approval_request(
        self,
        record_id: str,
        verdict: str,
        confidence: float,
        q_score: float,
        action_prompt: str,
        reason: str,
        blocking_violations: Optional[list[str]] = None,
        risk_level: str = "MEDIUM",
    ) -> ApprovalRequest:
        """
        Create a human approval request.

        Args:
            record_id: ID from audit trail
            verdict: BARK, GROWL, WAG, HOWL
            confidence: Decision confidence (0-1)
            q_score: Q-score (0-100)
            action_prompt: Prompt to execute
            reason: Why approval is needed
            blocking_violations: Axioms that failed
            risk_level: CRITICAL, HIGH, MEDIUM, LOW

        Returns:
            ApprovalRequest with request_id
        """
        import time
        request = ApprovalRequest(
            record_id=record_id,
            verdict=verdict,
            confidence=confidence,
            q_score=q_score,
            action_prompt=action_prompt,
            reason=reason,
            blocking_violations=blocking_violations or [],
            risk_level=risk_level,
            timestamp_created=time.time(),
        )
        self._append_request(request)
        logger.warning(f"Approval request created: {request.request_id} ({risk_level})")
        return request

    def approve_request(
        self,
        request_id: str,
        reviewer: str,
        notes: str = "",
    ) -> Optional[ApprovalRequest]:
        """
        Human approves a decision.

        Args:
            request_id: ID of approval request
            reviewer: Human reviewer name/ID
            notes: Optional review notes

        Returns:
            Updated ApprovalRequest or None if not found
        """
        import time
        request = self._find_request(request_id)
        if request:
            request.status = ApprovalStatus.APPROVED
            request.human_reviewer = reviewer
            request.approval_notes = notes
            request.timestamp_reviewed = time.time()
            self._save_to_disk()
            logger.info(f"Approval granted: {request_id} by {reviewer}")
            return request
        return None

    def reject_request(
        self,
        request_id: str,
        reviewer: str,
        notes: str = "",
    ) -> Optional[ApprovalRequest]:
        """
        Human rejects a decision.

        Args:
            request_id: ID of approval request
            reviewer: Human reviewer name/ID
            notes: Rejection reason

        Returns:
            Updated ApprovalRequest or None if not found
        """
        import time
        request = self._find_request(request_id)
        if request:
            request.status = ApprovalStatus.REJECTED
            request.human_reviewer = reviewer
            request.approval_notes = notes
            request.timestamp_reviewed = time.time()
            self._consecutive_failures += 1
            self._save_to_disk()
            logger.warning(f"Approval rejected: {request_id} by {reviewer}")
            return request
        return None

    def escalate_request(
        self,
        request_id: str,
        reason: str,
    ) -> Optional[ApprovalRequest]:
        """
        Escalate request to higher-level review.

        Args:
            request_id: ID of approval request
            reason: Reason for escalation

        Returns:
            Updated ApprovalRequest or None if not found
        """
        request = self._find_request(request_id)
        if request:
            request.status = ApprovalStatus.ESCALATED
            request.approval_notes = f"ESCALATED: {reason}"
            self._save_to_disk()
            logger.error(f"Approval escalated: {request_id} — {reason}")
            return request
        return None

    def get_approval_status(self, request_id: str) -> Optional[ApprovalStatus]:
        """Get the current status of an approval request."""
        request = self._find_request(request_id)
        return request.status if request else None

    def get_pending_approvals(self) -> list[ApprovalRequest]:
        """Get all pending approval requests."""
        return [r for r in self._requests if r.status == ApprovalStatus.PENDING]

    def record_execution_result(
        self,
        request_id: str,
        success: bool,
    ) -> None:
        """
        Record execution result after approval.

        Args:
            request_id: ID of approval request
            success: Whether execution succeeded
        """
        if success:
            self._consecutive_failures = 0
        else:
            self._consecutive_failures += 1

    def stats(self) -> dict[str, Any]:
        """Return approval gate statistics."""
        total = len(self._requests)
        pending = sum(1 for r in self._requests if r.status == ApprovalStatus.PENDING)
        approved = sum(1 for r in self._requests if r.status == ApprovalStatus.APPROVED)
        rejected = sum(1 for r in self._requests if r.status == ApprovalStatus.REJECTED)
        escalated = sum(1 for r in self._requests if r.status == ApprovalStatus.ESCALATED)

        return {
            "total_requests": total,
            "pending": pending,
            "approved": approved,
            "rejected": rejected,
            "escalated": escalated,
            "consecutive_failures": self._consecutive_failures,
            "approval_rate": approved / (approved + rejected) if (approved + rejected) > 0 else 0.0,
        }

    # ── Private ────────────────────────────────────────────────────────

    def _find_request(self, request_id: str) -> Optional[ApprovalRequest]:
        """Find request by ID."""
        for request in self._requests:
            if request.request_id == request_id:
                return request
        return None

    def _append_request(self, request: ApprovalRequest) -> None:
        """Add a new request with rolling cap."""
        self._requests.append(request)

        # Enforce rolling cap: F(9)=34 pending + resolved
        if len(self._requests) > fibonacci(9):
            self._requests = self._requests[-fibonacci(9):]

        self._save_to_disk()

    def _load_from_disk(self) -> None:
        """Load requests from JSONL file."""
        if not self._storage_path.exists():
            logger.debug(f"Approval requests file not found: {self._storage_path}")
            return

        try:
            with open(self._storage_path, "r") as f:
                for line in f:
                    if line.strip():
                        data = json.loads(line)
                        data["status"] = ApprovalStatus(data.get("status", "pending"))
                        request = ApprovalRequest(**data)
                        self._requests.append(request)

            logger.info(f"Loaded {len(self._requests)} approval requests")
        except json.JSONDecodeError as e:
            logger.warning(f"Failed to load approval requests: {e}")

    def _save_to_disk(self) -> None:
        """Save all requests to JSONL file."""
        try:
            with open(self._storage_path, "w") as f:
                for request in self._requests:
                    f.write(request.to_json() + "\n")
        except OSError as e:
            logger.error(f"Failed to save approval requests: {e}")
