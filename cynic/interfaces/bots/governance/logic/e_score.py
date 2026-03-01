"""
E-Score: 7-Dimensional Emergence & Reputation Engine

Maps CYNIC's core axiomes to reputation dimensions:
- Fidelity: Commitment to truth and accuracy
- Phi: Golden ratio balance in decisions
- Verify: Verification and validation diligence
- Culture: Community alignment and values
- Burn: Willingness to sacrifice for collective good
- Emergence: Capacity to contribute to collective intelligence
- Learning: Rate of improvement and adaptation

E-Scores are φ-bounded (max 0.618) for natural confidence limits.
"""

import math
from dataclasses import dataclass


@dataclass
class EScore:
    """
    7-dimensional E-Score for user reputation and emergence capability.

    Maps CYNIC's 5 Axiomes + 2 cross-axiom dimensions to reputation.
    All scores are φ-bounded (0.618 maximum) for natural confidence limits.

    Attributes:
        fidelity: Commitment to truth [0-0.618]
        phi: Golden ratio balance [0-0.618]
        verify: Verification diligence [0-0.618]
        culture: Community alignment [0-0.618]
        burn: Sacrifice for collective good [0-0.618]
        emergence: Collective intelligence contribution [0-0.618]
        learning: Rate of improvement [0-0.618]
    """
    fidelity: float
    phi: float
    verify: float
    culture: float
    burn: float
    emergence: float
    learning: float

    PHI_BOUND = 0.618  # Golden ratio bound for natural confidence limit

    def __post_init__(self):
        """Ensure all scores are φ-bounded after initialization"""
        for attr in ['fidelity', 'phi', 'verify', 'culture', 'burn', 'emergence', 'learning']:
            val = getattr(self, attr)
            # Clamp to [0, φ]
            clamped = max(0.0, min(self.PHI_BOUND, val))
            setattr(self, attr, clamped)

    def geometric_mean(self) -> float:
        """
        Calculate φ-bounded overall E-Score using geometric mean.

        Returns:
            Geometric mean of all 7 dimensions, φ-bounded
        """
        values = [
            self.fidelity,
            self.phi,
            self.verify,
            self.culture,
            self.burn,
            self.emergence,
            self.learning
        ]

        if all(v > 0 for v in values):
            product = math.prod(values)
            if product > 0:
                return math.prod(values) ** (1 / len(values))

        return 0.5

    def arithmetic_mean(self) -> float:
        """
        Calculate arithmetic mean of all dimensions.

        Returns:
            Simple average of all 7 dimensions
        """
        values = [
            self.fidelity,
            self.phi,
            self.verify,
            self.culture,
            self.burn,
            self.emergence,
            self.learning
        ]
        return sum(values) / len(values) if values else 0.5

    def to_dict(self) -> dict:
        """
        Convert E-Score to dictionary.

        Returns:
            Dictionary with all dimensions and overall scores
        """
        return {
            "fidelity": self.fidelity,
            "phi": self.phi,
            "verify": self.verify,
            "culture": self.culture,
            "burn": self.burn,
            "emergence": self.emergence,
            "learning": self.learning,
            "geometric_mean": self.geometric_mean(),
            "arithmetic_mean": self.arithmetic_mean(),
        }

    def get_dimension(self, name: str) -> float:
        """
        Get value of specific dimension.

        Args:
            name: Dimension name

        Returns:
            Dimension value, or None if not found
        """
        valid_dims = ['fidelity', 'phi', 'verify', 'culture', 'burn', 'emergence', 'learning']
        if name in valid_dims:
            return getattr(self, name)
        return None

    def all_dimensions(self) -> dict:
        """
        Get all dimensions as dictionary.

        Returns:
            Dictionary of all 7 dimensions
        """
        return {
            "fidelity": self.fidelity,
            "phi": self.phi,
            "verify": self.verify,
            "culture": self.culture,
            "burn": self.burn,
            "emergence": self.emergence,
            "learning": self.learning,
        }

    def is_fully_established(self) -> bool:
        """
        Check if user has strong E-Score across all dimensions.

        Returns:
            True if all dimensions > 0.4, False otherwise
        """
        threshold = 0.4
        return all(v >= threshold for v in [
            self.fidelity, self.phi, self.verify, self.culture,
            self.burn, self.emergence, self.learning
        ])


class EScoreManager:
    """Manage E-Score reputation across users"""

    def __init__(self):
        """Initialize E-Score manager"""
        self.scores: dict = {}

    def get_e_score(self, user_id: str) -> EScore:
        """
        Get or create E-Score for user.

        Args:
            user_id: User identifier

        Returns:
            EScore for user
        """
        if user_id not in self.scores:
            self.scores[user_id] = EScore(
                fidelity=0.5,
                phi=0.5,
                verify=0.5,
                culture=0.5,
                burn=0.5,
                emergence=0.5,
                learning=0.5
            )

        return self.scores[user_id]

    def has_user(self, user_id: str) -> bool:
        """
        Check if user has E-Score.

        Args:
            user_id: User identifier

        Returns:
            True if user has E-Score, False otherwise
        """
        return user_id in self.scores

    def update_dimension(self, user_id: str, dimension: str, value: float):
        """
        Update specific E-Score dimension.

        Args:
            user_id: User identifier
            dimension: Dimension name
            value: New value (will be φ-bounded)
        """
        score = self.get_e_score(user_id)
        valid_dims = ['fidelity', 'phi', 'verify', 'culture', 'burn', 'emergence', 'learning']

        if dimension in valid_dims:
            setattr(score, dimension, value)
            # Re-apply φ-bound
            score.__post_init__()

    def update_multiple(self, user_id: str, updates: dict):
        """
        Update multiple dimensions at once.

        Args:
            user_id: User identifier
            updates: Dictionary of {dimension: value}
        """
        self.get_e_score(user_id)
        for dimension, value in updates.items():
            self.update_dimension(user_id, dimension, value)

    def reset_user_score(self, user_id: str):
        """
        Reset user E-Score to defaults.

        Args:
            user_id: User identifier
        """
        self.scores[user_id] = EScore(
            fidelity=0.5,
            phi=0.5,
            verify=0.5,
            culture=0.5,
            burn=0.5,
            emergence=0.5,
            learning=0.5
        )

    def get_top_users(self, limit: int = 10) -> list:
        """
        Get users with highest overall E-Score.

        Args:
            limit: Maximum number of users to return

        Returns:
            List of (user_id, overall_score) tuples sorted by score
        """
        user_scores = [
            (user_id, score.geometric_mean())
            for user_id, score in self.scores.items()
        ]
        # Sort by score descending
        user_scores.sort(key=lambda x: x[1], reverse=True)
        return user_scores[:limit]

    def get_all_users(self) -> dict:
        """
        Get all user E-Scores.

        Returns:
            Dictionary of all user E-Scores
        """
        return dict(self.scores)

    def calculate_community_average(self) -> dict:
        """
        Calculate average E-Score across all users.

        Returns:
            Dictionary with average for each dimension
        """
        if not self.scores:
            return {d: 0.5 for d in ['fidelity', 'phi', 'verify', 'culture', 'burn', 'emergence', 'learning']}

        dimension_sums = {
            'fidelity': 0, 'phi': 0, 'verify': 0, 'culture': 0,
            'burn': 0, 'emergence': 0, 'learning': 0
        }

        for score in self.scores.values():
            for dim in dimension_sums.keys():
                dimension_sums[dim] += getattr(score, dim)

        user_count = len(self.scores)
        return {dim: val / user_count for dim, val in dimension_sums.items()}

    def get_users_by_establishment(self, fully_established: bool = True) -> list:
        """
        Get users by establishment status.

        Args:
            fully_established: If True, return fully established users,
                             if False, return emerging users

        Returns:
            List of user IDs
        """
        return [
            user_id for user_id, score in self.scores.items()
            if score.is_fully_established() == fully_established
        ]
