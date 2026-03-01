"""
Metabolism Immune System — Guardrails and Safety

Components:
- alignment_checker.py: 5 axioms validation (γ2 loop)
- power_limiter.py: Resource budget enforcement (γ1 loop)
- human_approval_gate.py: Human-in-the-loop gating
- transparency_audit.py: Decision auditing
"""
from .alignment_checker import AlignmentSafetyChecker, AlignmentViolation
from .power_limiter import PowerLimiter
from .human_approval_gate import HumanApprovalGate
from .transparency_audit import TransparencyAuditTrail

__all__ = [
    "AlignmentSafetyChecker",
    "AlignmentViolation",
    "PowerLimiter",
    "HumanApprovalGate",
    "TransparencyAuditTrail",
]
