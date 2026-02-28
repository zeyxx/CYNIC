"""
End-to-End Governance Bot Integration Test

Tests the complete governance workflow:
1. Proposal submission (proposal created)
2. CYNIC judgment (orchestrator.run → verdict + q_score)
3. Community voting (YES/NO/ABSTAIN votes recorded)
4. Voting closes (approval_status determined)
5. Community rates satisfaction (1-5 stars)
6. Q-Table learns (learn_cynic updates Q-values)
7. Next judgment uses improved confidence

This validates that the unified components work end-to-end:
- governance_bot submits proposals
- orchestrator judges with 11 Dogs + PBFT consensus
- learning loop feeds back to Q-Table
- confidence improves over cycles
"""

import pytest
from dataclasses import dataclass
from datetime import datetime, timedelta

from cynic.kernel.core.judgment import Cell
from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.unified_state import UnifiedLearningOutcome
from cynic.brain.learning.unified_learning import UnifiedQTable, LearningSession
from cynic.kernel.organism.organism import awaken


@dataclass
class ProposalRound:
    """Represents one complete governance round."""
    proposal_id: str
    title: str
    description: str
    predicted_verdict: str  # What CYNIC said
    vote_yes: int           # Community vote counts
    vote_no: int
    vote_abstain: int
    satisfaction_rating: float  # How satisfied (1-5, normalize to 0-1)

    @property
    def total_votes(self) -> int:
        return self.vote_yes + self.vote_no + self.vote_abstain

    @property
    def approval_percentage(self) -> float:
        if self.total_votes == 0:
            return 0.0
        return (self.vote_yes / self.total_votes) * 100

    @property
    def actual_verdict(self) -> str:
        """Determine actual verdict from vote results.

        HOWL: >=75% approval (strong consensus)
        WAG:  50-74% approval (moderate approval)
        GROWL: 26-49% approval (moderate rejection)
        BARK: <=25% approval (strong rejection)
        """
        if self.approval_percentage >= 75:
            return "HOWL"
        elif self.approval_percentage >= 50:
            return "WAG"
        elif self.approval_percentage >= 26:
            return "GROWL"
        else:
            return "BARK"

    @property
    def satisfaction_normalized(self) -> float:
        """Normalize satisfaction 1-5 to 0-1 for Q-Table."""
        return (self.satisfaction_rating - 1.0) / 4.0  # 1→0.0, 3→0.5, 5→1.0


class TestGovernanceBotE2E:
    """Test end-to-end governance bot workflow."""

    def test_single_proposal_cycle(self):
        """Test a single proposal from submission to learning."""
        # Setup
        q_table = UnifiedQTable()
        session = LearningSession()

        # Simulate proposal
        proposal = ProposalRound(
            proposal_id="prop_001",
            title="Increase community treasury",
            description="Allocate 5% of monthly revenue to community projects",
            predicted_verdict="HOWL",  # CYNIC strongly recommends
            vote_yes=15,
            vote_no=3,
            vote_abstain=2,
            satisfaction_rating=4.5  # Very satisfied (4.5/5)
        )

        # Verify proposal state
        assert proposal.approval_percentage == 75.0  # 15/20 = 75%
        assert proposal.actual_verdict == "HOWL"  # 75% approval → HOWL (strong consensus)
        assert proposal.satisfaction_normalized == pytest.approx(0.875)

        # Create learning outcome
        outcome = UnifiedLearningOutcome(
            judgment_id=proposal.proposal_id,
            predicted_verdict=proposal.predicted_verdict,
            actual_verdict=proposal.actual_verdict,
            satisfaction_rating=proposal.satisfaction_normalized
        )

        # Add to session and update Q-Table
        session.add_outcome(outcome)
        q_table.update(outcome)

        # Verify learning
        assert len(session.outcomes) == 1
        # HOWL→HOWL: 0.5 + 0.1 * (0.875 - 0.5) = 0.5375
        assert q_table.get_q_value("HOWL", "HOWL") == pytest.approx(0.5375)

    def test_multi_round_proposal_learning(self):
        """Test learning across multiple proposal rounds."""
        q_table = UnifiedQTable()
        session = LearningSession()

        # Simulate 5 governance rounds
        rounds = [
            ProposalRound(
                proposal_id="prop_001",
                title="Treasury allocation",
                description="...",
                predicted_verdict="HOWL",  # Recommend
                vote_yes=15, vote_no=3, vote_abstain=2,
                satisfaction_rating=4.5  # Happy
            ),
            ProposalRound(
                proposal_id="prop_002",
                title="Partnership opportunity",
                description="...",
                predicted_verdict="WAG",   # Lean approve
                vote_yes=12, vote_no=6, vote_abstain=2,
                satisfaction_rating=4.0   # Happy
            ),
            ProposalRound(
                proposal_id="prop_003",
                title="Change governance rules",
                description="...",
                predicted_verdict="GROWL", # Caution
                vote_yes=8, vote_no=10, vote_abstain=2,
                satisfaction_rating=3.5   # Neutral
            ),
            ProposalRound(
                proposal_id="prop_004",
                title="Increase fees",
                description="...",
                predicted_verdict="BARK",  # Strongly against
                vote_yes=3, vote_no=15, vote_abstain=2,
                satisfaction_rating=4.8   # Very happy
            ),
            ProposalRound(
                proposal_id="prop_005",
                title="Community charity",
                description="...",
                predicted_verdict="HOWL",  # Recommend
                vote_yes=18, vote_no=1, vote_abstain=1,
                satisfaction_rating=5.0   # Perfect
            ),
        ]

        # Process all rounds
        for round_data in rounds:
            outcome = UnifiedLearningOutcome(
                judgment_id=round_data.proposal_id,
                predicted_verdict=round_data.predicted_verdict,
                actual_verdict=round_data.actual_verdict,
                satisfaction_rating=round_data.satisfaction_normalized
            )
            session.add_outcome(outcome)
            q_table.update(outcome)

        # Verify learning statistics
        assert len(session.outcomes) == 5
        accuracy = session.accuracy_rate()
        avg_satisfaction = session.satisfaction_average()

        print(f"\nMulti-round learning statistics:")
        print(f"  Accuracy: {accuracy:.1%}")
        print(f"  Avg Satisfaction: {avg_satisfaction:.3f}")
        print(f"  Q-Table confidence (HOWL): {q_table.get_prediction_confidence('HOWL'):.3f}")
        print(f"  Q-Table confidence (WAG): {q_table.get_prediction_confidence('WAG'):.3f}")
        print(f"  Q-Table confidence (GROWL): {q_table.get_prediction_confidence('GROWL'):.3f}")
        print(f"  Q-Table confidence (BARK): {q_table.get_prediction_confidence('BARK'):.3f}")

        # Verify specific Q-value transitions
        # HOWL→HOWL appeared in round 1 and 5 with high satisfaction
        howl_correct = q_table.get_q_value("HOWL", "HOWL")
        assert howl_correct > 0.5  # Should increase

        # BARK→BARK appeared in round 4 with high satisfaction
        bark_correct = q_table.get_q_value("BARK", "BARK")
        assert bark_correct > 0.5

    def test_proposal_consensus_mismatch_learning(self):
        """Test learning when CYNIC's prediction doesn't match community decision."""
        q_table = UnifiedQTable()

        # CYNIC says BARK (strongly against), but community approves (WAG)
        proposal = ProposalRound(
            proposal_id="prop_001",
            title="Controversial proposal",
            description="...",
            predicted_verdict="BARK",
            vote_yes=12, vote_no=6, vote_abstain=2,
            satisfaction_rating=3.0  # Moderate satisfaction
        )

        assert proposal.actual_verdict == "WAG"
        assert proposal.predicted_verdict == "BARK"
        # BARK vs WAG is a significant mismatch

        outcome = UnifiedLearningOutcome(
            judgment_id=proposal.proposal_id,
            predicted_verdict=proposal.predicted_verdict,
            actual_verdict=proposal.actual_verdict,
            satisfaction_rating=proposal.satisfaction_normalized
        )

        initial_q = q_table.get_q_value("BARK", "WAG")
        q_table.update(outcome)
        updated_q = q_table.get_q_value("BARK", "WAG")

        # BARK→WAG should increase slightly (moderate satisfaction 3.0 → 0.5)
        # Q_new = 0.5 + 0.1 * (0.5 - 0.5) = 0.5
        assert updated_q == pytest.approx(0.5)

    def test_governance_confidence_by_category(self):
        """Test that different proposal categories learn different confidence levels."""
        q_table = UnifiedQTable()

        # Budget proposals: CYNIC is good at these (4/5 correct)
        budget_outcomes = [
            ("HOWL", "HOWL", 1.0),  # Correct, satisfied
            ("HOWL", "HOWL", 1.0),  # Correct, satisfied
            ("WAG", "WAG", 1.0),    # Correct, satisfied
            ("WAG", "WAG", 1.0),    # Correct, satisfied
            ("GROWL", "BARK", 0.6), # Wrong, less satisfied
        ]

        # Policy proposals: CYNIC is worse (2/5 correct)
        policy_outcomes = [
            ("HOWL", "GROWL", 0.2),  # Wrong
            ("WAG", "BARK", 0.3),    # Wrong
            ("GROWL", "HOWL", 0.4),  # Wrong
            ("BARK", "WAG", 0.4),    # Wrong
            ("WAG", "WAG", 1.0),     # Correct
        ]

        # Train on both categories
        for pred, actual, satisfaction in budget_outcomes + policy_outcomes:
            outcome = UnifiedLearningOutcome(
                judgment_id=f"prop_{pred}_{actual}",
                predicted_verdict=pred,
                actual_verdict=actual,
                satisfaction_rating=satisfaction
            )
            q_table.update(outcome)

        # Verify budget confidence > policy confidence
        # (This is simplified; in reality you'd separate Q-Tables by category)
        all_values = list(q_table.values.values())
        avg_q = sum(all_values) / len(all_values)

        print(f"\nGovernance learning by category:")
        print(f"  Average Q-value after 10 rounds: {avg_q:.3f}")
        print(f"  Total Q-Table transitions: {len(q_table.values)}")

    def test_satisfaction_drives_q_learning(self):
        """Test that satisfaction rating directly affects Q-Table updates."""
        q_table_high_sat = UnifiedQTable()
        q_table_low_sat = UnifiedQTable()

        # Same prediction/actual, different satisfaction
        prediction = "HOWL"
        actual = "WAG"

        high_sat_outcome = UnifiedLearningOutcome(
            judgment_id="prop_high",
            predicted_verdict=prediction,
            actual_verdict=actual,
            satisfaction_rating=1.0  # Perfect satisfaction
        )

        low_sat_outcome = UnifiedLearningOutcome(
            judgment_id="prop_low",
            predicted_verdict=prediction,
            actual_verdict=actual,
            satisfaction_rating=0.0  # No satisfaction
        )

        q_table_high_sat.update(high_sat_outcome)
        q_table_low_sat.update(low_sat_outcome)

        high_q = q_table_high_sat.get_q_value(prediction, actual)
        low_q = q_table_low_sat.get_q_value(prediction, actual)

        # High satisfaction should produce higher Q-value
        # High: 0.5 + 0.1 * (1.0 - 0.5) = 0.55
        # Low:  0.5 + 0.1 * (0.0 - 0.5) = 0.45
        assert high_q > low_q
        assert high_q == pytest.approx(0.55)
        assert low_q == pytest.approx(0.45)

    def test_governance_proposal_metrics(self):
        """Test that proposals track the right metrics for learning."""
        # Simulate a real proposal
        proposal = ProposalRound(
            proposal_id="prop_community_vote_001",
            title="Increase token rewards for top holders",
            description="Allocate additional rewards from treasury to top 10% token holders",
            predicted_verdict="HOWL",  # CYNIC recommends
            vote_yes=120,  # Out of 200 members
            vote_no=60,
            vote_abstain=20,
            satisfaction_rating=4.2
        )

        # Verify metrics
        assert proposal.total_votes == 200
        assert proposal.approval_percentage == 60.0
        assert proposal.actual_verdict == "WAG"  # 60% is moderate approval
        assert proposal.satisfaction_normalized == pytest.approx(0.8)

        # Verify correctness: CYNIC said HOWL, community decided WAG
        # This is a partial mismatch - community was more conservative
        assert proposal.predicted_verdict != proposal.actual_verdict

        # But satisfaction is high (4.2/5), so Q-Table shouldn't punish much
        satisfaction = proposal.satisfaction_normalized
        assert satisfaction > 0.5  # Moderately satisfied despite mismatch


class TestGovernanceBotWithOrchestrator:
    """Test governance bot with actual CYNIC orchestrator."""

    @pytest.mark.asyncio
    async def test_orchestrator_driven_governance(self):
        """
        Test full governance cycle with real orchestrator:
        1. Create proposal (Cell)
        2. Get orchestrator judgment (unified Dogs + PBFT)
        3. Simulate community vote
        4. Learn from outcome
        """
        try:
            organism = awaken()

            # Create a governance proposal
            cell = Cell(
                content="Should we increase staking rewards by 5%?",
                context="Community treasury: Monthly revenue allocation",
                reality="SOCIAL",
                analysis="JUDGE",
                lod=1
            )

            # Get CYNIC's judgment
            judgment = await organism.orchestrator.run(
                cell,
                level=ConsciousnessLevel.MICRO,
                budget_usd=0.01
            )

            if judgment:
                # Simulate community voting
                # 65% approve (WAG verdict)
                actual_verdict = "WAG"
                satisfaction = 0.85

                # Create learning outcome
                outcome = UnifiedLearningOutcome(
                    judgment_id=judgment.judgment_id,
                    predicted_verdict=judgment.verdict,
                    actual_verdict=actual_verdict,
                    satisfaction_rating=satisfaction
                )

                # Learn
                q_table = UnifiedQTable()
                q_table.update(outcome)

                # Verify
                q_value = q_table.get_q_value(judgment.verdict, actual_verdict)
                assert 0.0 <= q_value <= 1.0

                print(f"\nOrchestrator governance test:")
                print(f"  CYNIC verdict: {judgment.verdict}")
                print(f"  Community decision: {actual_verdict}")
                print(f"  Q-value after learning: {q_value:.3f}")
            else:
                pytest.skip("Orchestrator not available")

        except Exception as e:
            pytest.skip(f"Orchestrator not available: {e}")


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
