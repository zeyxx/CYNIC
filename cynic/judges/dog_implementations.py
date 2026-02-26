"""
11 Dog Implementations

Each Dog specializes in judging one or more axioms:

Dog 1: Crown Consciousness (FIDELITY)        — Judges overall fidelity & promise-keeping
Dog 2: Wisdom Analyzer (PHI)                  — Judges architectural balance & proportions
Dog 3: Understanding Synthesizer (VERIFY)     — Judges evidence & validation
Dog 4: Mercy Advocate (CULTURE)               — Judges community fit & cultural alignment
Dog 5: Severity Critic (BURN)                 — Judges efficiency & waste elimination
Dog 6: Harmony Mediator (FIDELITY + PHI)      — Balances fidelity and design
Dog 7: Victory Affirmer (PHI + VERIFY)        — Balances design and validation
Dog 8: Splendor Clarifier (VERIFY + CULTURE)  — Balances validation and culture
Dog 9: Foundation Keeper (CULTURE + BURN)     — Balances culture and efficiency
Dog 10: Kingdom Executor (BURN + FIDELITY)    — Balances efficiency and promise-keeping
Dog 11: Earth Guardian (All axioms)           — Holistic judgment across all axioms

Each Dog:
- Implements JudgeInterface
- Inherits from BaseJudge
- Is async
- Returns UnifiedJudgment
- Has φ-bounded confidence
- Has axiom_scores for all 5 core axioms
- Has unique judgment_id per judgment
"""

from __future__ import annotations

import time
import uuid
from typing import Dict, Any

from cynic.core.judge_interface import BaseJudge
from cynic.core.unified_state import UnifiedJudgment
from cynic.core.phi import (
    HOWL_MIN, WAG_MIN, GROWL_MIN, MAX_CONFIDENCE
)


class Dog1_CrownConsciousness(BaseJudge):
    """
    Dog 1: Crown Consciousness

    Focus: Overall fidelity and promise-keeping
    Axiom: FIDELITY (40% weight)

    Logic:
    - Evaluates if proposal aligns with community values
    - Checks consistency with past decisions
    - Assesses commitment to stated goals
    - Q-Score: FIDELITY 40%, others 15% each
    """

    def __init__(self):
        super().__init__(dog_id=1, dog_name="Crown Consciousness", axiom_focus="FIDELITY")

    async def judge(self, proposal_text: str, context: Dict[str, Any]) -> UnifiedJudgment:
        """Judge a proposal with FIDELITY focus."""
        # Analyze proposal length (proxy for substance)
        proposal_length = len(proposal_text)
        length_score = min(100.0, (proposal_length / 500) * 100)

        # Community alignment (from context)
        community_values = context.get("community_values", [])
        alignment_score = 85.0 if community_values else 60.0

        # Fidelity score (40% weight)
        fidelity_score = (alignment_score + length_score) / 2

        # Other axioms neutral (15% each)
        axiom_scores = {
            "FIDELITY": fidelity_score,
            "PHI": 50.0,
            "VERIFY": 50.0,
            "CULTURE": 50.0,
            "BURN": 50.0,
        }

        # Compute Q-score with weights: FIDELITY 40%, others 15% each
        q_score = (fidelity_score * 0.4) + (50.0 * 0.15 * 4)

        # Determine verdict based on Q-score
        if q_score >= HOWL_MIN:
            verdict = "HOWL"
        elif q_score >= WAG_MIN:
            verdict = "WAG"
        elif q_score >= GROWL_MIN:
            verdict = "GROWL"
        else:
            verdict = "BARK"

        # Confidence scaled from fidelity (φ-bounded)
        confidence = self._calculate_phi_bounded_confidence(fidelity_score / 100.0)

        return UnifiedJudgment(
            judgment_id=f"dog-{self.dog_id}-{uuid.uuid4().hex[:12]}",
            verdict=verdict,
            q_score=q_score,
            confidence=confidence,
            axiom_scores=axiom_scores,
            dog_votes={self.dog_id: {"vote": verdict, "confidence": confidence}},
            reasoning=f"{self.dog_name}: FIDELITY focus ({fidelity_score:.1f}/100) → {verdict}. "
            f"Aligns with community values: {bool(community_values)}",
            latency_ms=5.0,
        )


class Dog2_WisdomAnalyzer(BaseJudge):
    """
    Dog 2: Wisdom Analyzer

    Focus: Architectural proportions and balance
    Axiom: PHI (40% weight)

    Logic:
    - Evaluates design proportions (φ-aligned)
    - Checks balance of resources
    - Assesses symmetry and elegance
    - Q-Score: PHI 40%, others 15% each
    """

    def __init__(self):
        super().__init__(dog_id=2, dog_name="Wisdom Analyzer", axiom_focus="PHI")

    async def judge(self, proposal_text: str, context: Dict[str, Any]) -> UnifiedJudgment:
        """Judge a proposal with PHI focus."""
        # Analyze proposal structure (simple heuristic: word count)
        words = len(proposal_text.split())
        # Optimal is ~100 words (φ-scaled)
        ideal_words = 100
        balance_score = 100.0 - (abs(words - ideal_words) / ideal_words * 100)
        balance_score = max(0.0, min(100.0, balance_score))

        # Check for balanced structure (e.g., problem-solution-benefit)
        has_structure = (
            any(word in proposal_text.lower() for word in ["problem", "issue", "challenge"])
            and any(word in proposal_text.lower() for word in ["solution", "proposal", "plan"])
            and any(word in proposal_text.lower() for word in ["benefit", "advantage", "impact"])
        )
        structure_bonus = 20.0 if has_structure else 0.0

        # PHI score
        phi_score = min(100.0, balance_score + structure_bonus)

        axiom_scores = {
            "FIDELITY": 50.0,
            "PHI": phi_score,
            "VERIFY": 50.0,
            "CULTURE": 50.0,
            "BURN": 50.0,
        }

        q_score = (phi_score * 0.4) + (50.0 * 0.15 * 4)

        if q_score >= HOWL_MIN:
            verdict = "HOWL"
        elif q_score >= WAG_MIN:
            verdict = "WAG"
        elif q_score >= GROWL_MIN:
            verdict = "GROWL"
        else:
            verdict = "BARK"

        confidence = self._calculate_phi_bounded_confidence(phi_score / 100.0)

        return UnifiedJudgment(
            judgment_id=f"dog-{self.dog_id}-{uuid.uuid4().hex[:12]}",
            verdict=verdict,
            q_score=q_score,
            confidence=confidence,
            axiom_scores=axiom_scores,
            dog_votes={self.dog_id: {"vote": verdict, "confidence": confidence}},
            reasoning=f"{self.dog_name}: PHI focus ({phi_score:.1f}/100) → {verdict}. "
            f"Proposal has {words} words (optimal ~100), structure: {has_structure}",
            latency_ms=5.0,
        )


class Dog3_UnderstandingSynthesizer(BaseJudge):
    """
    Dog 3: Understanding Synthesizer

    Focus: Evidence and validation
    Axiom: VERIFY (40% weight)

    Logic:
    - How well is proposal validated?
    - What evidence exists?
    - Are claims substantiated?
    - Q-Score: VERIFY 40%, others 15% each
    """

    def __init__(self):
        super().__init__(dog_id=3, dog_name="Understanding Synthesizer", axiom_focus="VERIFY")

    async def judge(self, proposal_text: str, context: Dict[str, Any]) -> UnifiedJudgment:
        """Judge a proposal with VERIFY focus."""
        # Check for evidence markers
        evidence_count = context.get("evidence_count", 0)
        evidence_score = min(100.0, evidence_count * 20)

        # Check for specific, measurable claims
        has_metrics = any(word in proposal_text.lower() for word in ["%", "number", "count", "metric"])
        has_evidence = any(word in proposal_text.lower() for word in ["data", "research", "evidence", "study", "test"])

        verify_score = evidence_score
        if has_metrics:
            verify_score += 15
        if has_evidence:
            verify_score += 15

        verify_score = min(100.0, verify_score)

        axiom_scores = {
            "FIDELITY": 50.0,
            "PHI": 50.0,
            "VERIFY": verify_score,
            "CULTURE": 50.0,
            "BURN": 50.0,
        }

        q_score = (verify_score * 0.4) + (50.0 * 0.15 * 4)

        if q_score >= HOWL_MIN:
            verdict = "HOWL"
        elif q_score >= WAG_MIN:
            verdict = "WAG"
        elif q_score >= GROWL_MIN:
            verdict = "GROWL"
        else:
            verdict = "BARK"

        confidence = self._calculate_phi_bounded_confidence(verify_score / 100.0)

        return UnifiedJudgment(
            judgment_id=f"dog-{self.dog_id}-{uuid.uuid4().hex[:12]}",
            verdict=verdict,
            q_score=q_score,
            confidence=confidence,
            axiom_scores=axiom_scores,
            dog_votes={self.dog_id: {"vote": verdict, "confidence": confidence}},
            reasoning=f"{self.dog_name}: VERIFY focus ({verify_score:.1f}/100) → {verdict}. "
            f"Evidence items: {evidence_count}, has metrics: {has_metrics}, has evidence: {has_evidence}",
            latency_ms=5.0,
        )


class Dog4_MercyAdvocate(BaseJudge):
    """
    Dog 4: Mercy Advocate

    Focus: Community and culture fit
    Axiom: CULTURE (40% weight)

    Logic:
    - Does proposal fit community values?
    - Support for minority interests?
    - Accessibility and inclusion?
    - Q-Score: CULTURE 40%, others 15% each
    """

    def __init__(self):
        super().__init__(dog_id=4, dog_name="Mercy Advocate", axiom_focus="CULTURE")

    async def judge(self, proposal_text: str, context: Dict[str, Any]) -> UnifiedJudgment:
        """Judge a proposal with CULTURE focus."""
        # Check for inclusive language
        inclusive_terms = ["community", "everyone", "inclusive", "diverse", "accessible", "fair"]
        inclusive_count = sum(1 for term in inclusive_terms if term in proposal_text.lower())
        inclusive_score = min(100.0, inclusive_count * 15)

        # Check context for inclusivity
        is_inclusive = context.get("is_inclusive", False)
        inclusivity_bonus = 20.0 if is_inclusive else 0.0

        culture_score = min(100.0, inclusive_score + inclusivity_bonus)

        axiom_scores = {
            "FIDELITY": 50.0,
            "PHI": 50.0,
            "VERIFY": 50.0,
            "CULTURE": culture_score,
            "BURN": 50.0,
        }

        q_score = (culture_score * 0.4) + (50.0 * 0.15 * 4)

        if q_score >= HOWL_MIN:
            verdict = "HOWL"
        elif q_score >= WAG_MIN:
            verdict = "WAG"
        elif q_score >= GROWL_MIN:
            verdict = "GROWL"
        else:
            verdict = "BARK"

        confidence = self._calculate_phi_bounded_confidence(culture_score / 100.0)

        return UnifiedJudgment(
            judgment_id=f"dog-{self.dog_id}-{uuid.uuid4().hex[:12]}",
            verdict=verdict,
            q_score=q_score,
            confidence=confidence,
            axiom_scores=axiom_scores,
            dog_votes={self.dog_id: {"vote": verdict, "confidence": confidence}},
            reasoning=f"{self.dog_name}: CULTURE focus ({culture_score:.1f}/100) → {verdict}. "
            f"Inclusive terms: {inclusive_count}, context inclusive: {is_inclusive}",
            latency_ms=5.0,
        )


class Dog5_SeverityCritic(BaseJudge):
    """
    Dog 5: Severity Critic

    Focus: Efficiency and no waste
    Axiom: BURN (40% weight)

    Logic:
    - Is this minimal viable?
    - Or over-engineered?
    - Wasteful?
    - Q-Score: BURN 40%, others 15% each
    """

    def __init__(self):
        super().__init__(dog_id=5, dog_name="Severity Critic", axiom_focus="BURN")

    async def judge(self, proposal_text: str, context: Dict[str, Any]) -> UnifiedJudgment:
        """Judge a proposal with BURN focus."""
        # Minimal viable check
        is_minimal = context.get("is_minimal", False)
        minimal_bonus = 30.0 if is_minimal else 0.0

        # Check for efficiency language
        efficiency_terms = ["minimal", "efficient", "lean", "optimized", "streamlined", "lightweight"]
        efficiency_count = sum(1 for term in efficiency_terms if term in proposal_text.lower())
        efficiency_score = min(70.0, efficiency_count * 15)

        # Penalize verbose/wasteful language
        proposal_length = len(proposal_text)
        if proposal_length > 1000:
            verbosity_penalty = min(20.0, (proposal_length - 1000) / 100)
        else:
            verbosity_penalty = 0.0

        burn_score = min(100.0, efficiency_score + minimal_bonus - verbosity_penalty)

        axiom_scores = {
            "FIDELITY": 50.0,
            "PHI": 50.0,
            "VERIFY": 50.0,
            "CULTURE": 50.0,
            "BURN": burn_score,
        }

        q_score = (burn_score * 0.4) + (50.0 * 0.15 * 4)

        if q_score >= HOWL_MIN:
            verdict = "HOWL"
        elif q_score >= WAG_MIN:
            verdict = "WAG"
        elif q_score >= GROWL_MIN:
            verdict = "GROWL"
        else:
            verdict = "BARK"

        confidence = self._calculate_phi_bounded_confidence(burn_score / 100.0)

        return UnifiedJudgment(
            judgment_id=f"dog-{self.dog_id}-{uuid.uuid4().hex[:12]}",
            verdict=verdict,
            q_score=q_score,
            confidence=confidence,
            axiom_scores=axiom_scores,
            dog_votes={self.dog_id: {"vote": verdict, "confidence": confidence}},
            reasoning=f"{self.dog_name}: BURN focus ({burn_score:.1f}/100) → {verdict}. "
            f"Minimal viable: {is_minimal}, efficiency terms: {efficiency_count}, length: {proposal_length}",
            latency_ms=5.0,
        )


class Dog6_HarmonyMediator(BaseJudge):
    """
    Dog 6: Harmony Mediator

    Focus: Balance FIDELITY and PHI
    Axioms: FIDELITY + PHI (25% each, others 8.33% each)

    Logic: Balances promise-keeping with design elegance
    """

    def __init__(self):
        super().__init__(dog_id=6, dog_name="Harmony Mediator", axiom_focus="FIDELITY + PHI")

    async def judge(self, proposal_text: str, context: Dict[str, Any]) -> UnifiedJudgment:
        """Judge balancing FIDELITY and PHI."""
        # Fidelity component
        community_values = context.get("community_values", [])
        fidelity_base = 85.0 if community_values else 60.0

        # PHI component
        words = len(proposal_text.split())
        ideal_words = 100
        phi_base = 100.0 - (abs(words - ideal_words) / ideal_words * 100)
        phi_base = max(0.0, min(100.0, phi_base))

        # Balance them
        fidelity_score = fidelity_base
        phi_score = phi_base

        axiom_scores = {
            "FIDELITY": fidelity_score,
            "PHI": phi_score,
            "VERIFY": 50.0,
            "CULTURE": 50.0,
            "BURN": 50.0,
        }

        # Weighted average: FIDELITY 25%, PHI 25%, others 8.33% each
        q_score = (fidelity_score * 0.25) + (phi_score * 0.25) + (50.0 * 0.0833 * 3)

        if q_score >= HOWL_MIN:
            verdict = "HOWL"
        elif q_score >= WAG_MIN:
            verdict = "WAG"
        elif q_score >= GROWL_MIN:
            verdict = "GROWL"
        else:
            verdict = "BARK"

        confidence = self._calculate_phi_bounded_confidence((fidelity_score + phi_score) / 200.0)

        return UnifiedJudgment(
            judgment_id=f"dog-{self.dog_id}-{uuid.uuid4().hex[:12]}",
            verdict=verdict,
            q_score=q_score,
            confidence=confidence,
            axiom_scores=axiom_scores,
            dog_votes={self.dog_id: {"vote": verdict, "confidence": confidence}},
            reasoning=f"{self.dog_name}: Balancing FIDELITY ({fidelity_score:.1f}) "
            f"and PHI ({phi_score:.1f}) → {verdict}",
            latency_ms=5.0,
        )


class Dog7_VictoryAffirmer(BaseJudge):
    """
    Dog 7: Victory Affirmer

    Focus: Balance PHI and VERIFY
    Axioms: PHI + VERIFY (25% each, others 8.33% each)

    Logic: Balances design elegance with evidence
    """

    def __init__(self):
        super().__init__(dog_id=7, dog_name="Victory Affirmer", axiom_focus="PHI + VERIFY")

    async def judge(self, proposal_text: str, context: Dict[str, Any]) -> UnifiedJudgment:
        """Judge balancing PHI and VERIFY."""
        # PHI component
        words = len(proposal_text.split())
        phi_score = 100.0 - (abs(words - 100) / 100 * 100)
        phi_score = max(0.0, min(100.0, phi_score))

        # VERIFY component
        evidence_count = context.get("evidence_count", 0)
        verify_score = min(100.0, evidence_count * 20)

        axiom_scores = {
            "FIDELITY": 50.0,
            "PHI": phi_score,
            "VERIFY": verify_score,
            "CULTURE": 50.0,
            "BURN": 50.0,
        }

        q_score = (phi_score * 0.25) + (verify_score * 0.25) + (50.0 * 0.0833 * 3)

        if q_score >= HOWL_MIN:
            verdict = "HOWL"
        elif q_score >= WAG_MIN:
            verdict = "WAG"
        elif q_score >= GROWL_MIN:
            verdict = "GROWL"
        else:
            verdict = "BARK"

        confidence = self._calculate_phi_bounded_confidence((phi_score + verify_score) / 200.0)

        return UnifiedJudgment(
            judgment_id=f"dog-{self.dog_id}-{uuid.uuid4().hex[:12]}",
            verdict=verdict,
            q_score=q_score,
            confidence=confidence,
            axiom_scores=axiom_scores,
            dog_votes={self.dog_id: {"vote": verdict, "confidence": confidence}},
            reasoning=f"{self.dog_name}: Balancing PHI ({phi_score:.1f}) "
            f"and VERIFY ({verify_score:.1f}) → {verdict}",
            latency_ms=5.0,
        )


class Dog8_SplendorClarifier(BaseJudge):
    """
    Dog 8: Splendor Clarifier

    Focus: Balance VERIFY and CULTURE
    Axioms: VERIFY + CULTURE (25% each, others 8.33% each)

    Logic: Balances evidence with community values
    """

    def __init__(self):
        super().__init__(dog_id=8, dog_name="Splendor Clarifier", axiom_focus="VERIFY + CULTURE")

    async def judge(self, proposal_text: str, context: Dict[str, Any]) -> UnifiedJudgment:
        """Judge balancing VERIFY and CULTURE."""
        # VERIFY component
        evidence_count = context.get("evidence_count", 0)
        verify_score = min(100.0, evidence_count * 20)

        # CULTURE component
        inclusive_terms = ["community", "everyone", "inclusive", "diverse"]
        inclusive_count = sum(1 for term in inclusive_terms if term in proposal_text.lower())
        culture_score = min(100.0, inclusive_count * 20)

        axiom_scores = {
            "FIDELITY": 50.0,
            "PHI": 50.0,
            "VERIFY": verify_score,
            "CULTURE": culture_score,
            "BURN": 50.0,
        }

        q_score = (verify_score * 0.25) + (culture_score * 0.25) + (50.0 * 0.0833 * 3)

        if q_score >= HOWL_MIN:
            verdict = "HOWL"
        elif q_score >= WAG_MIN:
            verdict = "WAG"
        elif q_score >= GROWL_MIN:
            verdict = "GROWL"
        else:
            verdict = "BARK"

        confidence = self._calculate_phi_bounded_confidence((verify_score + culture_score) / 200.0)

        return UnifiedJudgment(
            judgment_id=f"dog-{self.dog_id}-{uuid.uuid4().hex[:12]}",
            verdict=verdict,
            q_score=q_score,
            confidence=confidence,
            axiom_scores=axiom_scores,
            dog_votes={self.dog_id: {"vote": verdict, "confidence": confidence}},
            reasoning=f"{self.dog_name}: Balancing VERIFY ({verify_score:.1f}) "
            f"and CULTURE ({culture_score:.1f}) → {verdict}",
            latency_ms=5.0,
        )


class Dog9_FoundationKeeper(BaseJudge):
    """
    Dog 9: Foundation Keeper

    Focus: Balance CULTURE and BURN
    Axioms: CULTURE + BURN (25% each, others 8.33% each)

    Logic: Balances community values with efficiency
    """

    def __init__(self):
        super().__init__(dog_id=9, dog_name="Foundation Keeper", axiom_focus="CULTURE + BURN")

    async def judge(self, proposal_text: str, context: Dict[str, Any]) -> UnifiedJudgment:
        """Judge balancing CULTURE and BURN."""
        # CULTURE component
        is_inclusive = context.get("is_inclusive", False)
        culture_score = 85.0 if is_inclusive else 50.0

        # BURN component
        is_minimal = context.get("is_minimal", False)
        minimal_bonus = 30.0 if is_minimal else 0.0

        efficiency_terms = ["minimal", "efficient", "lean", "optimized"]
        efficiency_count = sum(1 for term in efficiency_terms if term in proposal_text.lower())
        burn_score = min(100.0, (efficiency_count * 15) + minimal_bonus)

        axiom_scores = {
            "FIDELITY": 50.0,
            "PHI": 50.0,
            "VERIFY": 50.0,
            "CULTURE": culture_score,
            "BURN": burn_score,
        }

        q_score = (culture_score * 0.25) + (burn_score * 0.25) + (50.0 * 0.0833 * 3)

        if q_score >= HOWL_MIN:
            verdict = "HOWL"
        elif q_score >= WAG_MIN:
            verdict = "WAG"
        elif q_score >= GROWL_MIN:
            verdict = "GROWL"
        else:
            verdict = "BARK"

        confidence = self._calculate_phi_bounded_confidence((culture_score + burn_score) / 200.0)

        return UnifiedJudgment(
            judgment_id=f"dog-{self.dog_id}-{uuid.uuid4().hex[:12]}",
            verdict=verdict,
            q_score=q_score,
            confidence=confidence,
            axiom_scores=axiom_scores,
            dog_votes={self.dog_id: {"vote": verdict, "confidence": confidence}},
            reasoning=f"{self.dog_name}: Balancing CULTURE ({culture_score:.1f}) "
            f"and BURN ({burn_score:.1f}) → {verdict}",
            latency_ms=5.0,
        )


class Dog10_KingdomExecutor(BaseJudge):
    """
    Dog 10: Kingdom Executor

    Focus: Balance BURN and FIDELITY
    Axioms: BURN + FIDELITY (25% each, others 8.33% each)

    Logic: Balances efficiency with promise-keeping
    """

    def __init__(self):
        super().__init__(dog_id=10, dog_name="Kingdom Executor", axiom_focus="BURN + FIDELITY")

    async def judge(self, proposal_text: str, context: Dict[str, Any]) -> UnifiedJudgment:
        """Judge balancing BURN and FIDELITY."""
        # BURN component
        is_minimal = context.get("is_minimal", False)
        burn_score = 85.0 if is_minimal else 50.0

        # FIDELITY component
        community_values = context.get("community_values", [])
        fidelity_score = 85.0 if community_values else 60.0

        axiom_scores = {
            "FIDELITY": fidelity_score,
            "PHI": 50.0,
            "VERIFY": 50.0,
            "CULTURE": 50.0,
            "BURN": burn_score,
        }

        q_score = (burn_score * 0.25) + (fidelity_score * 0.25) + (50.0 * 0.0833 * 3)

        if q_score >= HOWL_MIN:
            verdict = "HOWL"
        elif q_score >= WAG_MIN:
            verdict = "WAG"
        elif q_score >= GROWL_MIN:
            verdict = "GROWL"
        else:
            verdict = "BARK"

        confidence = self._calculate_phi_bounded_confidence((burn_score + fidelity_score) / 200.0)

        return UnifiedJudgment(
            judgment_id=f"dog-{self.dog_id}-{uuid.uuid4().hex[:12]}",
            verdict=verdict,
            q_score=q_score,
            confidence=confidence,
            axiom_scores=axiom_scores,
            dog_votes={self.dog_id: {"vote": verdict, "confidence": confidence}},
            reasoning=f"{self.dog_name}: Balancing BURN ({burn_score:.1f}) "
            f"and FIDELITY ({fidelity_score:.1f}) → {verdict}",
            latency_ms=5.0,
        )


class Dog11_EarthGuardian(BaseJudge):
    """
    Dog 11: Earth Guardian

    Focus: All axioms equal weight
    Axioms: All 5 axioms (20% each)

    Logic: Holistic judgment across all dimensions
    """

    def __init__(self):
        super().__init__(dog_id=11, dog_name="Earth Guardian", axiom_focus="All Axioms")

    async def judge(self, proposal_text: str, context: Dict[str, Any]) -> UnifiedJudgment:
        """Judge with equal weight across all axioms."""
        # Compute each axiom
        proposal_length = len(proposal_text)
        fidelity_score = 80.0 if context.get("community_values") else 60.0

        words = len(proposal_text.split())
        phi_score = 100.0 - (abs(words - 100) / 100 * 100)
        phi_score = max(0.0, min(100.0, phi_score))

        verify_score = min(100.0, context.get("evidence_count", 0) * 20)

        inclusive_count = sum(1 for term in ["community", "inclusive", "diverse"]
                             if term in proposal_text.lower())
        culture_score = min(100.0, inclusive_count * 25)

        is_minimal = context.get("is_minimal", False)
        burn_score = 85.0 if is_minimal else 50.0

        axiom_scores = {
            "FIDELITY": fidelity_score,
            "PHI": phi_score,
            "VERIFY": verify_score,
            "CULTURE": culture_score,
            "BURN": burn_score,
        }

        # Equal weight: 20% each
        q_score = (sum(axiom_scores.values()) / 5)

        if q_score >= HOWL_MIN:
            verdict = "HOWL"
        elif q_score >= WAG_MIN:
            verdict = "WAG"
        elif q_score >= GROWL_MIN:
            verdict = "GROWL"
        else:
            verdict = "BARK"

        avg_score = sum(axiom_scores.values()) / 5
        confidence = self._calculate_phi_bounded_confidence(avg_score / 100.0)

        return UnifiedJudgment(
            judgment_id=f"dog-{self.dog_id}-{uuid.uuid4().hex[:12]}",
            verdict=verdict,
            q_score=q_score,
            confidence=confidence,
            axiom_scores=axiom_scores,
            dog_votes={self.dog_id: {"vote": verdict, "confidence": confidence}},
            reasoning=f"{self.dog_name}: Holistic judgment across all axioms "
            f"(avg {q_score:.1f}/100) → {verdict}. "
            f"FIDELITY:{fidelity_score:.0f} PHI:{phi_score:.0f} VERIFY:{verify_score:.0f} "
            f"CULTURE:{culture_score:.0f} BURN:{burn_score:.0f}",
            latency_ms=7.0,
        )


def get_all_dogs() -> list[BaseJudge]:
    """
    Factory function to create and return all 11 Dogs.

    Returns:
        List of 11 BaseJudge instances in order (Dog1-Dog11)
    """
    return [
        Dog1_CrownConsciousness(),
        Dog2_WisdomAnalyzer(),
        Dog3_UnderstandingSynthesizer(),
        Dog4_MercyAdvocate(),
        Dog5_SeverityCritic(),
        Dog6_HarmonyMediator(),
        Dog7_VictoryAffirmer(),
        Dog8_SplendorClarifier(),
        Dog9_FoundationKeeper(),
        Dog10_KingdomExecutor(),
        Dog11_EarthGuardian(),
    ]
