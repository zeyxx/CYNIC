"""
User Reputation Tracking System

Provides:
- Individual reputation metrics for users
- Reputation scoring and tracking
- Voting weight calculation based on reputation
- Metric updates and management
"""

from dataclasses import dataclass, field
from datetime import datetime


@dataclass
class ReputationScore:
    """
    User reputation metrics.

    Attributes:
        user_id: Unique user identifier
        governance_quality: Quality of proposals/votes [0-1]
        participation_rate: Frequency of participation [0-1]
        consensus_alignment: Alignment with community [0-1]
        expertise_level: Domain knowledge level [0-1]
        reliability: Consistency over time [0-1]
        community_feedback: Peer ratings [0-1]
        learning_ability: Rate of improvement [0-1]
    """
    user_id: str
    governance_quality: float = 0.5
    participation_rate: float = 0.5
    consensus_alignment: float = 0.5
    expertise_level: float = 0.5
    reliability: float = 0.5
    community_feedback: float = 0.5
    learning_ability: float = 0.5
    last_updated: datetime = field(default_factory=datetime.utcnow)

    def update_metric(self, metric: str, value: float):
        """
        Update a specific reputation metric.

        Args:
            metric: Metric name to update
            value: New value (will be bounded to [0, 1])
        """
        if hasattr(self, metric):
            # Bound value to [0, 1]
            bounded_value = max(0.0, min(1.0, value))
            setattr(self, metric, bounded_value)
            self.last_updated = datetime.utcnow()

    def get_metric(self, metric: str) -> float | None:
        """
        Get value of a specific metric.

        Args:
            metric: Metric name

        Returns:
            Metric value or None if not found
        """
        if hasattr(self, metric):
            return getattr(self, metric)
        return None

    def all_metrics(self) -> dict[str, float]:
        """
        Get all metrics as dictionary.

        Returns:
            Dictionary of all reputation metrics
        """
        return {
            "governance_quality": self.governance_quality,
            "participation_rate": self.participation_rate,
            "consensus_alignment": self.consensus_alignment,
            "expertise_level": self.expertise_level,
            "reliability": self.reliability,
            "community_feedback": self.community_feedback,
            "learning_ability": self.learning_ability,
        }

    def average_score(self) -> float:
        """
        Calculate average of all metrics.

        Returns:
            Arithmetic mean of all reputation metrics
        """
        metrics = self.all_metrics()
        if not metrics:
            return 0.5
        return sum(metrics.values()) / len(metrics)


class ReputationManager:
    """Manage user reputation scores and updates"""

    def __init__(self):
        """Initialize reputation manager"""
        self.scores: dict[str, ReputationScore] = {}

    def get_reputation(self, user_id: str) -> ReputationScore:
        """
        Get or create user reputation score.

        Args:
            user_id: User identifier

        Returns:
            ReputationScore for user
        """
        if user_id not in self.scores:
            self.scores[user_id] = ReputationScore(user_id=user_id)

        return self.scores[user_id]

    def has_user(self, user_id: str) -> bool:
        """
        Check if user has reputation record.

        Args:
            user_id: User identifier

        Returns:
            True if user exists, False otherwise
        """
        return user_id in self.scores

    def update_participation(self, user_id: str, activity_count: int, total_activities: int):
        """
        Update participation rate.

        Args:
            user_id: User identifier
            activity_count: Number of activities participated in
            total_activities: Total possible activities
        """
        if total_activities > 0:
            rate = min(1.0, activity_count / total_activities)
            score = self.get_reputation(user_id)
            score.update_metric("participation_rate", rate)

    def update_quality(self, user_id: str, quality_score: float):
        """
        Update governance quality score.

        Args:
            user_id: User identifier
            quality_score: Quality metric [0-1]
        """
        score = self.get_reputation(user_id)
        score.update_metric("governance_quality", quality_score)

    def update_consensus(self, user_id: str, alignment_score: float):
        """
        Update consensus alignment score.

        Args:
            user_id: User identifier
            alignment_score: Alignment with consensus [0-1]
        """
        score = self.get_reputation(user_id)
        score.update_metric("consensus_alignment", alignment_score)

    def update_expertise(self, user_id: str, expertise_score: float):
        """
        Update expertise level.

        Args:
            user_id: User identifier
            expertise_score: Expertise rating [0-1]
        """
        score = self.get_reputation(user_id)
        score.update_metric("expertise_level", expertise_score)

    def update_reliability(self, user_id: str, reliability_score: float):
        """
        Update reliability metric.

        Args:
            user_id: User identifier
            reliability_score: Reliability rating [0-1]
        """
        score = self.get_reputation(user_id)
        score.update_metric("reliability", reliability_score)

    def update_feedback(self, user_id: str, feedback_score: float):
        """
        Update community feedback score.

        Args:
            user_id: User identifier
            feedback_score: Feedback rating [0-1]
        """
        score = self.get_reputation(user_id)
        score.update_metric("community_feedback", feedback_score)

    def update_learning(self, user_id: str, learning_score: float):
        """
        Update learning ability score.

        Args:
            user_id: User identifier
            learning_score: Learning rate [0-1]
        """
        score = self.get_reputation(user_id)
        score.update_metric("learning_ability", learning_score)

    def calculate_voting_weight(self, user_id: str) -> float:
        """
        Calculate voting weight from reputation metrics.

        Uses geometric mean of key reputation metrics to determine
        how much weight this user's vote should carry.

        Args:
            user_id: User identifier

        Returns:
            Voting weight [0-1]
        """
        score = self.get_reputation(user_id)
        metrics = [
            score.governance_quality,
            score.participation_rate,
            score.consensus_alignment,
            score.expertise_level
        ]

        # Geometric mean for -bounded confidence
        import math
        if all(m > 0 for m in metrics):
            # Avoid taking nth root of 0
            product = math.prod(metrics)
            if product > 0:
                return product ** (1/len(metrics))

        return 0.5

    def get_top_users(self, limit: int = 10) -> list:
        """
        Get users with highest reputation.

        Args:
            limit: Maximum number of users to return

        Returns:
            List of (user_id, average_score) tuples sorted by score
        """
        user_scores = [
            (user_id, score.average_score())
            for user_id, score in self.scores.items()
        ]
        # Sort by score descending
        user_scores.sort(key=lambda x: x[1], reverse=True)
        return user_scores[:limit]

    def reset_user_reputation(self, user_id: str):
        """
        Reset user reputation to defaults.

        Args:
            user_id: User identifier
        """
        self.scores[user_id] = ReputationScore(user_id=user_id)

    def get_all_users(self) -> dict[str, ReputationScore]:
        """
        Get all user reputation scores.

        Returns:
            Dictionary of all user reputation scores
        """
        return dict(self.scores)
