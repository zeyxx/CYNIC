"""
Tests for Reputation and E-Score Systems

Tests:
- Reputation tracking and updates
- E-Score dimensions and calculations
- Voting weight calculations
- User ranking and analytics
"""

import pytest

pytestmark = pytest.mark.skip(reason="Old architecture removed in V5 - governance_bot module not found")

import math
from governance_bot.reputation import ReputationManager, ReputationScore
from governance_bot.e_score import EScore, EScoreManager


class TestReputationScore:
    """Test individual reputation scores"""

    def test_create_reputation_score(self):
        """Test creating reputation score with defaults"""
        score = ReputationScore(user_id="user_123")

        assert score.user_id == "user_123"
        assert score.governance_quality == 0.5
        assert score.participation_rate == 0.5
        assert score.consensus_alignment == 0.5
        assert score.expertise_level == 0.5
        assert score.reliability == 0.5
        assert score.community_feedback == 0.5
        assert score.learning_ability == 0.5

    def test_update_metric(self):
        """Test updating reputation metric"""
        score = ReputationScore(user_id="user_123")
        score.update_metric("governance_quality", 0.8)

        assert score.governance_quality == 0.8

    def test_metric_bounds_clamp_high(self):
        """Test metric values are clamped to [0, 1]"""
        score = ReputationScore(user_id="user_123")
        score.update_metric("governance_quality", 1.5)

        assert score.governance_quality == 1.0

    def test_metric_bounds_clamp_low(self):
        """Test metric values are clamped below 0"""
        score = ReputationScore(user_id="user_123")
        score.update_metric("governance_quality", -0.5)

        assert score.governance_quality == 0.0

    def test_get_metric(self):
        """Test retrieving metric value"""
        score = ReputationScore(user_id="user_123")
        score.update_metric("expertise_level", 0.7)

        assert score.get_metric("expertise_level") == 0.7
        assert score.get_metric("nonexistent") is None

    def test_all_metrics(self):
        """Test getting all metrics"""
        score = ReputationScore(user_id="user_123")
        metrics = score.all_metrics()

        assert len(metrics) == 7
        assert "governance_quality" in metrics
        assert "learning_ability" in metrics

    def test_average_score(self):
        """Test averaging all metrics"""
        score = ReputationScore(user_id="user_123")
        avg = score.average_score()

        # All default at 0.5, so average should be 0.5
        assert avg == 0.5

    def test_average_with_varied_scores(self):
        """Test average with different metric values"""
        score = ReputationScore(user_id="user_123")
        score.update_metric("governance_quality", 0.8)
        score.update_metric("participation_rate", 0.6)

        avg = score.average_score()
        assert 0.5 < avg < 0.8


class TestReputationManager:
    """Test reputation management system"""

    def test_get_reputation_creates_default(self):
        """Test getting reputation creates default if missing"""
        manager = ReputationManager()
        score = manager.get_reputation("user_456")

        assert score is not None
        assert score.user_id == "user_456"
        assert score.governance_quality == 0.5

    def test_has_user(self):
        """Test checking if user exists"""
        manager = ReputationManager()
        assert not manager.has_user("user_789")

        manager.get_reputation("user_789")
        assert manager.has_user("user_789")

    def test_update_participation(self):
        """Test updating participation rate"""
        manager = ReputationManager()
        manager.update_participation("user_123", 10, 15)

        score = manager.get_reputation("user_123")
        expected = 10 / 15
        assert score.participation_rate == pytest.approx(expected)

    def test_update_participation_max(self):
        """Test participation rate maxes at 1.0"""
        manager = ReputationManager()
        manager.update_participation("user_123", 20, 15)

        score = manager.get_reputation("user_123")
        assert score.participation_rate == 1.0

    def test_update_quality(self):
        """Test updating governance quality"""
        manager = ReputationManager()
        manager.update_quality("user_123", 0.85)

        score = manager.get_reputation("user_123")
        assert score.governance_quality == 0.85

    def test_update_consensus(self):
        """Test updating consensus alignment"""
        manager = ReputationManager()
        manager.update_consensus("user_123", 0.75)

        score = manager.get_reputation("user_123")
        assert score.consensus_alignment == 0.75

    def test_update_expertise(self):
        """Test updating expertise level"""
        manager = ReputationManager()
        manager.update_expertise("user_123", 0.9)

        score = manager.get_reputation("user_123")
        assert score.expertise_level == 0.9

    def test_voting_weight_calculation(self):
        """Test voting weight from reputation"""
        manager = ReputationManager()
        manager.update_quality("user_123", 0.8)
        manager.update_participation("user_123", 10, 15)

        weight = manager.calculate_voting_weight("user_123")

        assert 0.0 <= weight <= 1.0

    def test_voting_weight_default_user(self):
        """Test voting weight for user with default metrics"""
        manager = ReputationManager()
        weight = manager.calculate_voting_weight("user_new")

        # Default metrics are 0.5, geometric mean is 0.5
        assert weight == 0.5

    def test_get_top_users(self):
        """Test getting users sorted by reputation"""
        manager = ReputationManager()

        # Create users with different reputations
        manager.update_quality("user_1", 0.9)
        manager.update_quality("user_2", 0.7)
        manager.update_quality("user_3", 0.5)

        top = manager.get_top_users(2)

        assert len(top) == 2
        assert top[0][0] == "user_1"
        assert top[1][0] == "user_2"

    def test_reset_user_reputation(self):
        """Test resetting user reputation"""
        manager = ReputationManager()
        manager.update_quality("user_123", 0.8)

        manager.reset_user_reputation("user_123")
        score = manager.get_reputation("user_123")

        assert score.governance_quality == 0.5

    def test_get_all_users(self):
        """Test getting all user reputations"""
        manager = ReputationManager()
        manager.get_reputation("user_1")
        manager.get_reputation("user_2")

        all_users = manager.get_all_users()

        assert len(all_users) == 2
        assert "user_1" in all_users
        assert "user_2" in all_users


class TestEScore:
    """Test E-Score dimensions"""

    def test_create_e_score(self):
        """Test creating E-Score"""
        e_score = EScore(
            fidelity=0.6,
            phi=0.5,
            verify=0.7,
            culture=0.5,
            burn=0.4,
            emergence=0.5,
            learning=0.6
        )

        assert e_score.fidelity == 0.6
        assert e_score.learning == 0.6

    def test_phi_bounding(self):
        """Test φ-bounding of E-Score"""
        e_score = EScore(
            fidelity=1.0,
            phi=0.5,
            verify=0.5,
            culture=0.5,
            burn=0.5,
            emergence=0.5,
            learning=0.5
        )

        phi_bound = 0.618
        assert e_score.fidelity <= phi_bound

    def test_all_dimensions_bounded(self):
        """Test all dimensions are φ-bounded"""
        e_score = EScore(
            fidelity=2.0,
            phi=2.0,
            verify=2.0,
            culture=2.0,
            burn=2.0,
            emergence=2.0,
            learning=2.0
        )

        phi_bound = 0.618
        for dim in [e_score.fidelity, e_score.phi, e_score.verify,
                    e_score.culture, e_score.burn, e_score.emergence,
                    e_score.learning]:
            assert dim <= phi_bound

    def test_geometric_mean_calculation(self):
        """Test E-Score geometric mean"""
        e_score = EScore(
            fidelity=0.5,
            phi=0.5,
            verify=0.5,
            culture=0.5,
            burn=0.5,
            emergence=0.5,
            learning=0.5
        )

        mean = e_score.geometric_mean()

        assert 0.0 <= mean <= 0.618
        assert mean == pytest.approx(0.5)

    def test_arithmetic_mean_calculation(self):
        """Test arithmetic mean of dimensions"""
        e_score = EScore(
            fidelity=0.4,
            phi=0.6,
            verify=0.5,
            culture=0.5,
            burn=0.5,
            emergence=0.5,
            learning=0.5
        )

        mean = e_score.arithmetic_mean()
        expected = (0.4 + 0.6 + 0.5 + 0.5 + 0.5 + 0.5 + 0.5) / 7
        assert mean == pytest.approx(expected)

    def test_e_score_to_dict(self):
        """Test E-Score serialization"""
        e_score = EScore(
            fidelity=0.6,
            phi=0.5,
            verify=0.7,
            culture=0.5,
            burn=0.4,
            emergence=0.5,
            learning=0.6
        )

        score_dict = e_score.to_dict()

        assert "fidelity" in score_dict
        assert "geometric_mean" in score_dict
        assert "arithmetic_mean" in score_dict
        assert 0.0 <= score_dict["geometric_mean"] <= 0.618

    def test_get_dimension(self):
        """Test getting specific dimension"""
        e_score = EScore(
            fidelity=0.6,
            phi=0.5,
            verify=0.7,
            culture=0.5,
            burn=0.4,
            emergence=0.5,
            learning=0.6
        )

        assert e_score.get_dimension("fidelity") == 0.6
        assert e_score.get_dimension("learning") == 0.6
        assert e_score.get_dimension("invalid") is None

    def test_all_dimensions_dict(self):
        """Test getting all dimensions as dict"""
        e_score = EScore(
            fidelity=0.6,
            phi=0.5,
            verify=0.7,
            culture=0.5,
            burn=0.4,
            emergence=0.5,
            learning=0.6
        )

        dims = e_score.all_dimensions()

        assert len(dims) == 7
        assert dims["fidelity"] == 0.6

    def test_is_fully_established(self):
        """Test establishment status"""
        # All dimensions > 0.4
        e_score_high = EScore(
            fidelity=0.5,
            phi=0.5,
            verify=0.5,
            culture=0.5,
            burn=0.5,
            emergence=0.5,
            learning=0.5
        )
        assert e_score_high.is_fully_established()

        # One dimension < 0.4
        e_score_low = EScore(
            fidelity=0.3,
            phi=0.5,
            verify=0.5,
            culture=0.5,
            burn=0.5,
            emergence=0.5,
            learning=0.5
        )
        assert not e_score_low.is_fully_established()


class TestEScoreManager:
    """Test E-Score management system"""

    def test_get_e_score_creates_default(self):
        """Test getting E-Score creates default"""
        manager = EScoreManager()
        score = manager.get_e_score("user_123")

        assert score is not None
        assert score.fidelity == 0.5

    def test_has_user(self):
        """Test checking if user has E-Score"""
        manager = EScoreManager()
        assert not manager.has_user("user_456")

        manager.get_e_score("user_456")
        assert manager.has_user("user_456")

    def test_update_dimension(self):
        """Test updating single dimension"""
        manager = EScoreManager()
        manager.update_dimension("user_123", "fidelity", 0.6)

        score = manager.get_e_score("user_123")
        assert score.fidelity == 0.6

    def test_update_multiple(self):
        """Test updating multiple dimensions"""
        manager = EScoreManager()
        updates = {
            "fidelity": 0.6,
            "phi": 0.5,
            "verify": 0.4
        }
        manager.update_multiple("user_123", updates)

        score = manager.get_e_score("user_123")
        assert score.fidelity == 0.6
        assert score.phi == 0.5
        assert score.verify == 0.4

    def test_reset_user_score(self):
        """Test resetting user score"""
        manager = EScoreManager()
        manager.update_dimension("user_123", "fidelity", 0.8)

        manager.reset_user_score("user_123")
        score = manager.get_e_score("user_123")

        assert score.fidelity == 0.5

    def test_get_top_users(self):
        """Test getting top users by E-Score"""
        manager = EScoreManager()

        # Create users with different scores
        manager.update_dimension("user_1", "fidelity", 0.6)
        manager.update_dimension("user_2", "fidelity", 0.4)

        top = manager.get_top_users(2)

        assert len(top) == 2
        assert top[0][0] == "user_1"

    def test_get_all_users(self):
        """Test getting all user E-Scores"""
        manager = EScoreManager()
        manager.get_e_score("user_1")
        manager.get_e_score("user_2")

        all_users = manager.get_all_users()

        assert len(all_users) == 2

    def test_calculate_community_average(self):
        """Test calculating community average"""
        manager = EScoreManager()
        manager.update_dimension("user_1", "fidelity", 0.6)
        manager.update_dimension("user_2", "fidelity", 0.4)

        avg = manager.calculate_community_average()

        assert "fidelity" in avg
        assert avg["fidelity"] == pytest.approx(0.5)

    def test_get_users_by_establishment(self):
        """Test filtering users by establishment status"""
        manager = EScoreManager()

        # Create established user
        manager.update_multiple("user_1", {
            "fidelity": 0.5,
            "phi": 0.5,
            "verify": 0.5,
            "culture": 0.5,
            "burn": 0.5,
            "emergence": 0.5,
            "learning": 0.5
        })

        # Create emerging user
        manager.update_dimension("user_2", "fidelity", 0.3)

        established = manager.get_users_by_establishment(True)
        emerging = manager.get_users_by_establishment(False)

        assert "user_1" in established
        assert "user_2" in emerging
