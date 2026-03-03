"""
Metabolism Immune System " Guardrails and Safety

Components:
- alignment_checker.py: 5 axioms validation (2 loop)
- power_limiter.py: Resource budget enforcement (1 loop)
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
