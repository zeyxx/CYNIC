"""
Metabolism Immune System â€" Guardrails and Safety

Components:
- alignment_checker.py: 5 axioms validation (Î³2 loop)
- power_limiter.py: Resource budget enforcement (Î³1 loop)
- human_approval_gate.py: Human-in-the-loop gating
- transparency_audit.py: Decision auditing
"""

from .alignment_checker import AlignmentSafetyChecker, AlignmentViolation
from .human_approval_gate import HumanApprovalGate
from .power_limiter import PowerLimiter
from .transparency_audit import TransparencyAuditTrail

__all__ = [
    "AlignmentSafetyChecker",
    "AlignmentViolation",
    "PowerLimiter",
    "HumanApprovalGate",
    "TransparencyAuditTrail",
]
