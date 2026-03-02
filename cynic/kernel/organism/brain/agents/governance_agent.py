"""
CYNIC Governance Agents - AI Members of Governance Communities

Agents participate in governance voting to:
1. Create rich voting patterns for emergence detection
2. Test autonomous decision-making capabilities
3. Accelerate learning (agents learn faster than humans)
4. Demonstrate governance improvement (agent + human > human alone)
5. Generate emergence signals for measurement

Agent Types:
- Optimal: Follows CYNIC judgment (11 Dogs + PBFT)
- Exploration: Tests alternatives (contrarian voting)
- Learning: Learns from community outcomes
- Community: Adopts community-specific culture
- Chaos: Random voting (baseline for comparison)
"""

from __future__ import annotations

import logging
import random
from dataclasses import dataclass

from cynic.kernel.organism.brain.learning.qlearning import LearningSignal, QTable

logger = logging.getLogger(__name__)


@dataclass
class AgentVote:
    """Record of an agent's vote."""

    agent_id: str
    proposal_id: str
    vote: str  # "YES" | "NO" | "ABSTAIN"
    agent_type: str
    confidence: float  # 0-1, how confident in this vote
    reasoning: str


@dataclass
class AgentLearningRecord:
    """Record of learning from a governance outcome."""

    agent_id: str
    proposal_id: str
    predicted_vote: str
    actual_outcome: str  # Did proposal pass or fail?
    satisfaction: float  # Community satisfaction with outcome
    prediction_correct: bool


class GovernanceAgent:
    """AI agent that participates in governance voting and learns."""

    def __init__(
        self,
        agent_id: str,
        agent_type: str,  # optimal|exploration|learning|community|chaos
        community_id: str,
    ):
        """Initialize a governance agent.

        Args:
            agent_id: Unique agent identifier
            agent_type: Type of agent (determines voting strategy)
            community_id: Community this agent belongs to
        """
        self.agent_id = agent_id
        self.agent_type = agent_type
        self.community_id = community_id

        # Voting history
        self.vote_history: list[AgentVote] = []
        self.learning_history: list[AgentLearningRecord] = []

        # Learning systems
        self.q_table = QTable()
        self.prediction_accuracy = 0.5  # Start neutral

        # Agent-specific state
        self.voting_patterns: dict[str, int] = {
            "YES": 0,
            "NO": 0,
            "ABSTAIN": 0,
        }

        logger.info(
            f"Agent {self.agent_id} ({self.agent_type}) created for community {community_id}"
        )

    async def vote_on_proposal(self, proposal) -> AgentVote:
        """Vote on a governance proposal.

        Args:
            proposal: Proposal with cynic_verdict and metadata

        Returns:
            AgentVote with the agent's voting decision
        """

        if self.agent_type == "optimal":
            vote, confidence, reasoning = await self._vote_optimal(proposal)

        elif self.agent_type == "exploration":
            vote, confidence, reasoning = await self._vote_exploration(proposal)

        elif self.agent_type == "learning":
            vote, confidence, reasoning = await self._vote_learning(proposal)

        elif self.agent_type == "community":
            vote, confidence, reasoning = await self._vote_community(proposal)

        elif self.agent_type == "chaos":
            vote, confidence, reasoning = await self._vote_chaos(proposal)

        else:
            vote, confidence, reasoning = "ABSTAIN", 0.3, "Unknown agent type"

        # Record vote
        agent_vote = AgentVote(
            agent_id=self.agent_id,
            proposal_id=proposal.proposal_id,
            vote=vote,
            agent_type=self.agent_type,
            confidence=confidence,
            reasoning=reasoning,
        )

        self.vote_history.append(agent_vote)
        self.voting_patterns[vote] += 1

        logger.info(
            f"Agent {self.agent_id} ({self.agent_type}) voted {vote} on {proposal.proposal_id} "
            f"(confidence={confidence:.2f}): {reasoning}"
        )

        return agent_vote

    async def _vote_optimal(self, proposal) -> tuple[str, float, str]:
        """Optimal: Follow CYNIC judgment exactly."""

        cynic_verdict = proposal.cynic_verdict
        q_score = proposal.q_score / 100.0  # Normalize to 0-1

        if cynic_verdict == "HOWL":
            return "YES", 0.9, f"CYNIC HOWL verdict (q={q_score:.2f}) â’ strong approval"

        elif cynic_verdict == "WAG":
            # WAG: Moderate approval, but could go either way
            vote = "YES" if random.random() < 0.75 else "ABSTAIN"
            conf = 0.7
            return vote, conf, f"CYNIC WAG verdict (q={q_score:.2f}) â’ lean approve"

        elif cynic_verdict == "GROWL":
            # GROWL: Moderate caution
            vote = "ABSTAIN" if random.random() < 0.75 else "NO"
            conf = 0.6
            return vote, conf, f"CYNIC GROWL verdict (q={q_score:.2f}) â’ proceed cautiously"

        elif cynic_verdict == "BARK":
            return "NO", 0.9, f"CYNIC BARK verdict (q={q_score:.2f}) â’ strong rejection"

        else:
            return "ABSTAIN", 0.3, f"Unknown CYNIC verdict: {cynic_verdict}"

    async def _vote_exploration(self, proposal) -> tuple[str, float, str]:
        """Exploration: Sometimes contradict CYNIC (test alternatives)."""

        cynic_verdict = proposal.cynic_verdict

        # 30% of the time, vote opposite to CYNIC
        if random.random() < 0.3:
            if cynic_verdict in ["HOWL", "WAG"]:
                return "NO", 0.4, f"Exploring: Contrary to CYNIC {cynic_verdict} (test alternative)"
            else:
                return (
                    "YES",
                    0.4,
                    f"Exploring: Contrary to CYNIC {cynic_verdict} (test alternative)",
                )

        # Otherwise follow CYNIC with lower confidence
        else:
            if cynic_verdict == "HOWL":
                return "YES", 0.7, f"Exploring: CYNIC {cynic_verdict} but questioning"
            elif cynic_verdict == "WAG":
                return "YES" if random.random() < 0.6 else "ABSTAIN", 0.6, "Exploring WAG sentiment"
            elif cynic_verdict == "GROWL":
                return "ABSTAIN" if random.random() < 0.6 else "NO", 0.5, "Exploring GROWL caution"
            elif cynic_verdict == "BARK":
                return "NO", 0.7, f"Exploring: CYNIC {cynic_verdict} but questioning"
            else:
                return "ABSTAIN", 0.3, "Exploring unknown verdict"

    async def _vote_learning(self, proposal) -> tuple[str, float, str]:
        """Learning: Adjust strategy based on past community outcomes."""

        # Use past prediction accuracy to inform confidence
        cynic_verdict = proposal.cynic_verdict

        if self.prediction_accuracy > 0.7:
            # High confidence in past learning â’ follow CYNIC closely
            if cynic_verdict == "HOWL":
                return (
                    "YES",
                    0.8 + (self.prediction_accuracy - 0.7),
                    "Learning: Past accuracy high, follow CYNIC",
                )
            elif cynic_verdict == "WAG":
                return (
                    "YES" if random.random() < 0.75 else "ABSTAIN",
                    0.65,
                    "Learning-adjusted WAG strategy",
                )
            elif cynic_verdict == "GROWL":
                return (
                    "ABSTAIN" if random.random() < 0.75 else "NO",
                    0.6,
                    "Learning-adjusted GROWL strategy",
                )
            elif cynic_verdict == "BARK":
                return (
                    "NO",
                    0.8 + (self.prediction_accuracy - 0.7),
                    "Learning: Past accuracy high, follow CYNIC",
                )

        else:
            # Low confidence â’ more exploration
            return await self._vote_exploration(proposal)

    async def _vote_community(self, proposal) -> tuple[str, float, str]:
        """Community: Adopt community-specific governance culture."""

        # Vote based on what past proposals in this community did
        community_approval_rate = sum(1 for v in self.vote_history if v.vote == "YES") / max(
            1, len(self.vote_history)
        )

        # If community tends to approve, approve this
        if community_approval_rate > 0.6:
            vote = "YES"
            conf = min(community_approval_rate, 0.85)
            reasoning = f"Community culture: {community_approval_rate:.0%} approval rate â’ align with community"

        elif community_approval_rate < 0.4:
            vote = "NO"
            conf = min(1 - community_approval_rate, 0.85)
            reasoning = f"Community culture: {community_approval_rate:.0%} approval rate â’ skeptical with community"

        else:
            vote = "ABSTAIN"
            conf = 0.5
            reasoning = "Community culture: neutral, let humans decide"

        return vote, conf, reasoning

    async def _vote_chaos(self, proposal) -> tuple[str, float, str]:
        """Chaos: Vote semi-randomly (baseline for comparison)."""

        choices = ["YES", "NO", "ABSTAIN"]
        vote = random.choice(choices)
        confidence = random.uniform(0.2, 0.5)

        return vote, confidence, "Random voting (baseline comparison)"

    async def learn_from_outcome(
        self,
        proposal_id: str,
        actual_outcome: str,  # "APPROVED" or "REJECTED"
        community_satisfaction: float,  # 0-1
    ) -> AgentLearningRecord:
        """Learn from a governance outcome.

        Args:
            proposal_id: ID of proposal that was voted on
            actual_outcome: What actually happened (APPROVED/REJECTED)
            community_satisfaction: Community satisfaction with outcome (0-1)

        Returns:
            Learning record documenting the learning
        """

        # Find the agent's vote on this proposal
        agent_vote = None
        for vote in self.vote_history:
            if vote.proposal_id == proposal_id:
                agent_vote = vote
                break

        if not agent_vote:
            logger.warning(f"Agent {self.agent_id} has no vote record for {proposal_id}")
            return None

        # Determine if agent's vote was "correct" (aligned with actual outcome)
        expected_vote = "YES" if actual_outcome == "APPROVED" else "NO"
        prediction_correct = (agent_vote.vote == expected_vote) or (agent_vote.vote == "ABSTAIN")

        # Create learning record
        record = AgentLearningRecord(
            agent_id=self.agent_id,
            proposal_id=proposal_id,
            predicted_vote=agent_vote.vote,
            actual_outcome=actual_outcome,
            satisfaction=community_satisfaction,
            prediction_correct=prediction_correct,
        )

        self.learning_history.append(record)

        # Update prediction accuracy
        correct_count = sum(1 for r in self.learning_history if r.prediction_correct)
        self.prediction_accuracy = correct_count / len(self.learning_history)

        # Update Q-Table with learning outcome
        # Map vote to CYNIC verdict space for learning
        vote_to_verdict = {
            "YES": "HOWL" if community_satisfaction > 0.7 else "WAG",
            "NO": "BARK" if community_satisfaction < 0.3 else "GROWL",
            "ABSTAIN": "WAG",
        }

        predicted_verdict = vote_to_verdict[agent_vote.vote]

        # Convert outcome to LearningSignal
        state_key = f"AGENT:{self.agent_id}:{self.agent_type}"
        signal = LearningSignal(
            state_key=state_key,
            action=predicted_verdict,
            reward=community_satisfaction,
            judgment_id=proposal_id,
            loop_name=f"AGENT_{self.agent_id}",
        )
        self.q_table.update(signal)

        logger.info(
            f"Agent {self.agent_id} learned from {proposal_id}: "
            f"voted {agent_vote.vote}, actual {actual_outcome}, "
            f"accuracy={self.prediction_accuracy:.1%}, satisfaction={community_satisfaction:.2f}"
        )

        return record

    def get_voting_profile(self) -> dict:
        """Get agent's voting profile (statistics)."""

        if not self.vote_history:
            return {
                "agent_id": self.agent_id,
                "agent_type": self.agent_type,
                "total_votes": 0,
                "accuracy": 0.5,
                "profile": "No votes yet",
            }

        state_key = f"AGENT:{self.agent_id}:{self.agent_type}"
        return {
            "agent_id": self.agent_id,
            "agent_type": self.agent_type,
            "total_votes": len(self.vote_history),
            "yes_votes": self.voting_patterns["YES"],
            "no_votes": self.voting_patterns["NO"],
            "abstain_votes": self.voting_patterns["ABSTAIN"],
            "yes_rate": self.voting_patterns["YES"] / len(self.vote_history),
            "accuracy": self.prediction_accuracy,
            "q_table_confidence": self.q_table.confidence(state_key),
        }

    def __str__(self) -> str:
        """String representation."""
        profile = self.get_voting_profile()
        return (
            f"Agent({self.agent_id}, {self.agent_type}, "
            f"votes={profile['total_votes']}, accuracy={profile['accuracy']:.1%})"
        )
