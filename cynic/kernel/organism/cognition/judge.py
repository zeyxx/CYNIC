"""
CYNIC Axiomatic Judge - The Ethical & Architectural Guardian.
Uses Wisdom Nodes to evaluate proposed changes before they reach the codebase.
"""
from __future__ import annotations

import logging
from typing import List, Dict, Any, Optional
from cynic.kernel.organism.cognition.base import get_cognitive_base, WisdomNode

logger = logging.getLogger("cynic.organism.cognition.judge")

class AxiomaticJudge:
    def __init__(self):
        self.base = get_cognitive_base()

    def evaluate_proposal(self, axiom: str, proposal_description: str, diff: str) -> Dict[str, Any]:
        """
        Evaluates a code proposal against industry wisdom nodes for a specific axiom.
        """
        relevant_principles = self.base.get_principles_for_axiom(axiom)
        findings = []
        is_valid = True
        phi_score = 1.0

        for node in relevant_principles:
            # Simple heuristic matching (would be LLM-driven in full production)
            # Here we simulate the logic: "Does this change violate the principle?"
            if node.id == "unified_pipelines" and "os.getenv" in diff:
                findings.append(f"VIOLATION: {node.principle} - Direct env access detected in pipeline logic.")
                is_valid = False
                phi_score -= 0.3
            
            if node.id == "data_drift_monitoring" and "try:" in diff and "except:" in diff and "pass" in diff:
                findings.append(f"VIOLATION: {node.principle} - Silent failure detected (except: pass).")
                is_valid = False
                phi_score -= 0.4

        return {
            "is_valid": is_valid,
            "phi_score": max(0.0, phi_score),
            "findings": findings,
            "axiom": axiom
        }

_judge: Optional[AxiomaticJudge] = None

def get_judge() -> AxiomaticJudge:
    global _judge
    if _judge is None:
        _judge = AxiomaticJudge()
    return _judge
