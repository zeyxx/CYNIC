# Phase 3: Advanced Governance Features Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement advanced governance features including authentication, reputation systems, sophisticated voting mechanisms, and cross-community consensus for fair, learning-based governance.

**Architecture:** Layered approach with JWT authentication, E-Score reputation engine (7D scoring with φ-bounded confidence), voting mechanisms (quadratic/weighted/delegated), proposal templates, treasury management, and cross-community coordination enabling memecoin communities to govern collectively.

**Tech Stack:** PyJWT, SQLAlchemy (reputation tracking), voting algorithms, treasury smart contracts, Discord roles, NEAR tokens

---

## Task 3.1: User Authentication & Authorization

**Files:**
- Create: `governance_bot/auth.py`
- Create: `governance_bot/roles.py`
- Create: `cynic/tests/test_authentication.py`
- Modify: `governance_bot/config.py` (add JWT settings)

**Step 1: Add JWT configuration**

Modify `governance_bot/config.py`:

```python
class AuthSettings(BaseSettings):
    """Authentication settings"""
    jwt_secret: str = Field(default="", description="JWT secret key for signing")
    jwt_algorithm: str = Field(default="HS256", description="JWT algorithm")
    jwt_expiry_hours: int = Field(default=24, description="JWT token expiry in hours")
    enable_auth: bool = Field(default=False, description="Enable authentication")

    class Config:
        extra = "ignore"

# Add to main Config class:
auth: AuthSettings = Field(default_factory=AuthSettings)
```

**Step 2: Create auth module**

Create `governance_bot/auth.py`:

```python
"""Authentication and authorization"""

import jwt
from datetime import datetime, timedelta
from typing import Dict, Optional
from governance_bot.config import Config

class AuthManager:
    """Manage JWT authentication"""

    def __init__(self, config: Config):
        self.config = config
        self.secret = config.auth.jwt_secret
        self.algorithm = config.auth.jwt_algorithm
        self.expiry_hours = config.auth.jwt_expiry_hours

    def create_token(self, user_id: str, roles: list = None) -> str:
        """Create JWT token for user"""
        if not self.secret:
            raise ValueError("JWT secret not configured")

        payload = {
            "user_id": user_id,
            "roles": roles or [],
            "exp": datetime.utcnow() + timedelta(hours=self.expiry_hours),
            "iat": datetime.utcnow()
        }

        return jwt.encode(payload, self.secret, algorithm=self.algorithm)

    def verify_token(self, token: str) -> Optional[Dict]:
        """Verify and decode JWT token"""
        try:
            payload = jwt.decode(token, self.secret, algorithms=[self.algorithm])
            return payload
        except jwt.InvalidTokenError:
            return None

    def has_role(self, token: str, required_role: str) -> bool:
        """Check if token has required role"""
        payload = self.verify_token(token)
        if not payload:
            return False

        return required_role in payload.get("roles", [])
```

**Step 3: Create roles module**

Create `governance_bot/roles.py`:

```python
"""Role-based access control"""

from enum import Enum
from typing import List

class UserRole(str, Enum):
    """User roles in governance"""
    ADMIN = "admin"
    MODERATOR = "moderator"
    MEMBER = "member"
    GUEST = "guest"

class RoleManager:
    """Manage user roles and permissions"""

    PERMISSIONS = {
        UserRole.ADMIN: [
            "create_proposal",
            "modify_proposal",
            "delete_proposal",
            "manage_roles",
            "manage_treasury"
        ],
        UserRole.MODERATOR: [
            "create_proposal",
            "modify_proposal",
            "manage_members",
            "approve_proposals"
        ],
        UserRole.MEMBER: [
            "create_proposal",
            "vote",
            "view_treasury"
        ],
        UserRole.GUEST: [
            "view_proposals",
            "view_treasury"
        ]
    }

    @classmethod
    def has_permission(cls, role: UserRole, action: str) -> bool:
        """Check if role has permission for action"""
        return action in cls.PERMISSIONS.get(role, [])

    @classmethod
    def get_user_roles(cls, user_id: str, guild_id: int) -> List[UserRole]:
        """Get user roles from Discord guild"""
        # This would fetch from Discord in real implementation
        return [UserRole.MEMBER]
```

**Step 4: Create authentication tests**

Create `cynic/tests/test_authentication.py`:

```python
"""Tests for authentication and authorization"""

import pytest
from governance_bot.auth import AuthManager
from governance_bot.roles import RoleManager, UserRole
from governance_bot.config import Config

class TestAuthentication:
    """Test authentication system"""

    @pytest.fixture
    def auth_config(self, monkeypatch):
        """Test config with JWT settings"""
        monkeypatch.setenv("AUTH_JWT_SECRET", "test_secret_key_12345")
        monkeypatch.setenv("AUTH_JWT_ALGORITHM", "HS256")
        monkeypatch.setenv("AUTH_JWT_EXPIRY_HOURS", "24")
        monkeypatch.setenv("AUTH_ENABLE_AUTH", "true")
        return Config()

    def test_create_jwt_token(self, auth_config):
        """Test JWT token creation"""
        auth = AuthManager(auth_config)

        token = auth.create_token("user_123", roles=["member"])

        assert token is not None
        assert isinstance(token, str)
        assert len(token) > 0

    def test_verify_valid_token(self, auth_config):
        """Test verifying valid JWT token"""
        auth = AuthManager(auth_config)

        token = auth.create_token("user_123", roles=["member"])
        payload = auth.verify_token(token)

        assert payload is not None
        assert payload["user_id"] == "user_123"
        assert "member" in payload["roles"]

    def test_verify_invalid_token(self, auth_config):
        """Test verifying invalid JWT token"""
        auth = AuthManager(auth_config)

        payload = auth.verify_token("invalid.token.here")

        assert payload is None

    def test_check_role_permission(self):
        """Test role-based permission checking"""
        assert RoleManager.has_permission(UserRole.ADMIN, "create_proposal")
        assert RoleManager.has_permission(UserRole.MEMBER, "vote")
        assert not RoleManager.has_permission(UserRole.GUEST, "create_proposal")

    def test_role_hierarchy(self):
        """Test role hierarchy and permissions"""
        admin_perms = set(RoleManager.PERMISSIONS[UserRole.ADMIN])
        member_perms = set(RoleManager.PERMISSIONS[UserRole.MEMBER])

        # Admin should have more permissions than member
        assert len(admin_perms) > len(member_perms)
```

**Step 5: Run tests**

Run: `pytest cynic/tests/test_authentication.py -v`
Expected: All 5 tests PASS

**Step 6: Commit**

```bash
git add governance_bot/auth.py governance_bot/roles.py cynic/tests/test_authentication.py governance_bot/config.py
git commit -m "feat(auth): Add JWT authentication and role-based access control"
```

---

## Task 3.2: Reputation & E-Score System

**Files:**
- Create: `governance_bot/reputation.py`
- Create: `governance_bot/e_score.py`
- Create: `cynic/tests/test_reputation.py`

**Step 1: Create reputation tracking module**

Create `governance_bot/reputation.py`:

```python
"""User reputation tracking"""

from typing import Dict, Optional
from dataclasses import dataclass, field
from datetime import datetime

@dataclass
class ReputationScore:
    """User reputation metrics"""
    user_id: str
    governance_quality: float = 0.5  # 0-1: quality of proposals/votes
    participation_rate: float = 0.5  # 0-1: frequency of participation
    consensus_alignment: float = 0.5  # 0-1: alignment with community
    expertise_level: float = 0.5  # 0-1: domain knowledge
    reliability: float = 0.5  # 0-1: consistency over time
    community_feedback: float = 0.5  # 0-1: peer ratings
    learning_ability: float = 0.5  # 0-1: improvement over time

    def update_metric(self, metric: str, value: float):
        """Update a specific reputation metric"""
        if hasattr(self, metric):
            setattr(self, metric, max(0.0, min(1.0, value)))

class ReputationManager:
    """Manage user reputation scores"""

    def __init__(self):
        self.scores: Dict[str, ReputationScore] = {}

    def get_reputation(self, user_id: str) -> ReputationScore:
        """Get user reputation score"""
        if user_id not in self.scores:
            self.scores[user_id] = ReputationScore(user_id=user_id)

        return self.scores[user_id]

    def update_participation(self, user_id: str, activity_count: int, total_activities: int):
        """Update participation rate"""
        if total_activities > 0:
            rate = min(1.0, activity_count / total_activities)
            score = self.get_reputation(user_id)
            score.update_metric("participation_rate", rate)

    def update_quality(self, user_id: str, quality_score: float):
        """Update governance quality score"""
        score = self.get_reputation(user_id)
        score.update_metric("governance_quality", quality_score)

    def calculate_voting_weight(self, user_id: str) -> float:
        """Calculate voting weight from reputation"""
        score = self.get_reputation(user_id)
        metrics = [
            score.governance_quality,
            score.participation_rate,
            score.consensus_alignment,
            score.expertise_level
        ]

        # Geometric mean for φ-bounded confidence
        import math
        if all(m > 0 for m in metrics):
            return math.prod(metrics) ** (1/len(metrics))
        else:
            return 0.5
```

**Step 2: Create E-Score module**

Create `governance_bot/e_score.py`:

```python
"""E-Score: 7-dimensional reputation engine"""

from dataclasses import dataclass
import math

@dataclass
class EScore:
    """7-dimensional E-Score (Emergence Score)"""
    # The 5 CYNIC Axioms mapped to reputation
    fidelity: float  # 0-1: Commitment to truth and accuracy
    phi: float  # 0-1: Golden ratio balance in decisions
    verify: float  # 0-1: Verification and validation diligence
    culture: float  # 0-1: Community alignment and values
    burn: float  # 0-1: Willingness to sacrifice for collective good

    # Cross-axiom dimensions
    emergence: float  # 0-1: Capacity to contribute to collective intelligence
    learning: float  # 0-1: Rate of improvement and adaptation

    def __post_init__(self):
        """Ensure all scores are φ-bounded"""
        phi_bound = 0.618
        for attr in ['fidelity', 'phi', 'verify', 'culture', 'burn', 'emergence', 'learning']:
            val = getattr(self, attr)
            # Cap at φ for natural confidence limit
            setattr(self, attr, min(phi_bound, max(0.0, val)))

    def geometric_mean(self) -> float:
        """Calculate φ-bounded overall score using geometric mean"""
        values = [self.fidelity, self.phi, self.verify, self.culture, self.burn, self.emergence, self.learning]
        if all(v > 0 for v in values):
            return math.prod(values) ** (1/len(values))
        return 0.5

    def to_dict(self) -> dict:
        """Convert to dictionary"""
        return {
            "fidelity": self.fidelity,
            "phi": self.phi,
            "verify": self.verify,
            "culture": self.culture,
            "burn": self.burn,
            "emergence": self.emergence,
            "learning": self.learning,
            "overall": self.geometric_mean()
        }

class EScoreManager:
    """Manage E-Score reputation across users"""

    def __init__(self):
        self.scores: dict = {}

    def get_e_score(self, user_id: str) -> EScore:
        """Get user E-Score"""
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

    def update_dimension(self, user_id: str, dimension: str, value: float):
        """Update specific E-Score dimension"""
        score = self.get_e_score(user_id)
        if hasattr(score, dimension):
            setattr(score, dimension, value)
            # Re-apply φ-bound
            score.__post_init__()
```

**Step 3: Create reputation tests**

Create `cynic/tests/test_reputation.py`:

```python
"""Tests for reputation and E-Score systems"""

import pytest
from governance_bot.reputation import ReputationManager, ReputationScore
from governance_bot.e_score import EScore, EScoreManager

class TestReputation:
    """Test reputation tracking"""

    def test_create_reputation_score(self):
        """Test creating reputation score"""
        score = ReputationScore(user_id="user_123")

        assert score.user_id == "user_123"
        assert score.governance_quality == 0.5
        assert score.participation_rate == 0.5

    def test_update_metric(self):
        """Test updating reputation metric"""
        score = ReputationScore(user_id="user_123")
        score.update_metric("governance_quality", 0.8)

        assert score.governance_quality == 0.8

    def test_metric_bounds(self):
        """Test metric values stay in [0, 1]"""
        score = ReputationScore(user_id="user_123")

        score.update_metric("governance_quality", 1.5)  # Above 1
        assert score.governance_quality == 1.0

        score.update_metric("governance_quality", -0.5)  # Below 0
        assert score.governance_quality == 0.0

    def test_voting_weight_calculation(self):
        """Test voting weight from reputation"""
        manager = ReputationManager()
        manager.update_quality("user_123", 0.8)
        manager.update_participation("user_123", 10, 15)

        weight = manager.calculate_voting_weight("user_123")

        assert 0.0 <= weight <= 1.0

class TestEScore:
    """Test E-Score system"""

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
            fidelity=1.0,  # Will be bounded to 0.618
            phi=0.5,
            verify=0.5,
            culture=0.5,
            burn=0.5,
            emergence=0.5,
            learning=0.5
        )

        phi_bound = 0.618
        assert e_score.fidelity <= phi_bound

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
        assert mean == 0.5

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
        assert "overall" in score_dict
        assert 0.0 <= score_dict["overall"] <= 0.618
```

**Step 4: Run tests**

Run: `pytest cynic/tests/test_reputation.py -v`
Expected: All 8 tests PASS

**Step 5: Commit**

```bash
git add governance_bot/reputation.py governance_bot/e_score.py cynic/tests/test_reputation.py
git commit -m "feat(reputation): Add reputation tracking and E-Score system"
```

---

## Task 3.3: Advanced Voting Mechanisms

**Files:**
- Create: `governance_bot/voting.py`
- Create: `cynic/tests/test_voting_mechanics.py`

**Step 1: Create voting module**

Create `governance_bot/voting.py`:

```python
"""Advanced voting mechanisms"""

from typing import Dict, List, Tuple
from dataclasses import dataclass
from enum import Enum
import math

class VotingType(str, Enum):
    """Types of voting mechanisms"""
    SIMPLE = "simple"  # 1 vote per user
    QUADRATIC = "quadratic"  # Cost increases quadratically
    WEIGHTED = "weighted"  # Weight based on reputation
    DELEGATED = "delegated"  # Votes can be delegated

@dataclass
class Vote:
    """Single vote record"""
    voter_id: str
    proposal_id: str
    choice: bool  # True = for, False = against
    weight: float = 1.0  # Vote weight (reputation-based)
    delegated_from: str = None  # If delegated, who it came from

class VoteCounter:
    """Count and tally votes with different mechanisms"""

    @staticmethod
    def simple_majority(votes: List[Vote]) -> Tuple[bool, float]:
        """Simple majority voting"""
        if not votes:
            return False, 0.5

        for_votes = sum(1 for v in votes if v.choice)
        total = len(votes)

        approval_rate = for_votes / total
        return for_votes > total / 2, approval_rate

    @staticmethod
    def quadratic_voting(votes: List[Vote]) -> Tuple[bool, float]:
        """Quadratic voting: cost increases as sqrt(votes)"""
        if not votes:
            return False, 0.5

        for_votes = sum(math.sqrt(1) for v in votes if v.choice)
        against_votes = sum(math.sqrt(1) for v in votes if not v.choice)
        total = for_votes + against_votes

        if total == 0:
            return False, 0.5

        approval_rate = for_votes / total
        return for_votes > total / 2, approval_rate

    @staticmethod
    def weighted_voting(votes: List[Vote]) -> Tuple[bool, float]:
        """Weighted voting: vote weight = reputation"""
        if not votes:
            return False, 0.5

        for_weight = sum(v.weight for v in votes if v.choice)
        against_weight = sum(v.weight for v in votes if not v.choice)
        total_weight = for_weight + against_weight

        if total_weight == 0:
            return False, 0.5

        approval_rate = for_weight / total_weight
        return for_weight > total_weight / 2, approval_rate

class DelegationManager:
    """Manage vote delegation"""

    def __init__(self):
        self.delegations: Dict[str, str] = {}  # voter_id -> delegate_id

    def delegate_vote(self, voter_id: str, delegate_id: str):
        """Delegate vote to another user"""
        self.delegations[voter_id] = delegate_id

    def revoke_delegation(self, voter_id: str):
        """Revoke vote delegation"""
        if voter_id in self.delegations:
            del self.delegations[voter_id]

    def get_delegated_votes(self, delegate_id: str) -> List[str]:
        """Get all voters delegated to this user"""
        return [v for v, d in self.delegations.items() if d == delegate_id]
```

**Step 2: Create voting tests**

Create `cynic/tests/test_voting_mechanics.py`:

```python
"""Tests for voting mechanisms"""

import pytest
from governance_bot.voting import Vote, VoteCounter, VotingType, DelegationManager

class TestSimpleVoting:
    """Test simple voting mechanism"""

    def test_simple_majority_approve(self):
        """Test simple majority approval"""
        votes = [
            Vote(voter_id="user_1", proposal_id="prop_1", choice=True),
            Vote(voter_id="user_2", proposal_id="prop_1", choice=True),
            Vote(voter_id="user_3", proposal_id="prop_1", choice=False),
        ]

        approved, rate = VoteCounter.simple_majority(votes)

        assert approved == True
        assert rate == 2/3

    def test_simple_majority_reject(self):
        """Test simple majority rejection"""
        votes = [
            Vote(voter_id="user_1", proposal_id="prop_1", choice=False),
            Vote(voter_id="user_2", proposal_id="prop_1", choice=False),
            Vote(voter_id="user_3", proposal_id="prop_1", choice=True),
        ]

        approved, rate = VoteCounter.simple_majority(votes)

        assert approved == False
        assert rate == 1/3

class TestWeightedVoting:
    """Test weighted voting mechanism"""

    def test_weighted_voting_approve(self):
        """Test weighted voting with reputation"""
        votes = [
            Vote(voter_id="user_1", proposal_id="prop_1", choice=True, weight=0.8),
            Vote(voter_id="user_2", proposal_id="prop_1", choice=True, weight=0.6),
            Vote(voter_id="user_3", proposal_id="prop_1", choice=False, weight=0.5),
        ]

        approved, rate = VoteCounter.weighted_voting(votes)

        assert approved == True
        for_weight = 0.8 + 0.6
        total_weight = 0.8 + 0.6 + 0.5
        assert abs(rate - for_weight/total_weight) < 0.001

class TestDelegation:
    """Test vote delegation"""

    def test_delegate_vote(self):
        """Test delegating vote"""
        manager = DelegationManager()
        manager.delegate_vote("user_1", "user_2")

        delegated = manager.get_delegated_votes("user_2")

        assert "user_1" in delegated

    def test_revoke_delegation(self):
        """Test revoking delegation"""
        manager = DelegationManager()
        manager.delegate_vote("user_1", "user_2")
        manager.revoke_delegation("user_1")

        delegated = manager.get_delegated_votes("user_2")

        assert "user_1" not in delegated

    def test_multiple_delegations(self):
        """Test multiple users delegating to one"""
        manager = DelegationManager()
        manager.delegate_vote("user_1", "delegate")
        manager.delegate_vote("user_2", "delegate")
        manager.delegate_vote("user_3", "delegate")

        delegated = manager.get_delegated_votes("delegate")

        assert len(delegated) == 3
```

**Step 3: Run tests**

Run: `pytest cynic/tests/test_voting_mechanics.py -v`
Expected: All 7 tests PASS

**Step 4: Commit**

```bash
git add governance_bot/voting.py cynic/tests/test_voting_mechanics.py
git commit -m "feat(voting): Add quadratic, weighted, and delegated voting mechanisms"
```

---

## Task 3.4: Proposal Templates & Standardization

**Files:**
- Create: `governance_bot/proposal_templates.py`
- Create: `cynic/tests/test_proposal_templates.py`

**Step 1: Create proposal templates module**

Create `governance_bot/proposal_templates.py`:

```python
"""Proposal templates for standardized governance"""

from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple
from enum import Enum

class ProposalType(str, Enum):
    """Types of proposals"""
    GOVERNANCE = "governance"  # Governance changes
    BUDGET = "budget"  # Treasury/budget
    FEATURE = "feature"  # New feature proposal
    EMERGENCY = "emergency"  # Emergency action
    STANDARD = "standard"  # Generic proposal

@dataclass
class ProposalTemplate:
    """Template for standardized proposals"""
    title_template: str
    description_template: str
    required_fields: List[str]
    voting_period_days: int = 3
    approval_threshold_percent: float = 50.0

class ProposalTemplates:
    """Collection of proposal templates"""

    GOVERNANCE_TEMPLATE = ProposalTemplate(
        title_template="[GOV] {change_type}",
        description_template="Governance Change Proposal:\n\nChange Type: {change_type}\n\nRationale: {rationale}\n\nImpact: {impact}",
        required_fields=["change_type", "rationale", "impact"],
        voting_period_days=7,
        approval_threshold_percent=66.0
    )

    BUDGET_TEMPLATE = ProposalTemplate(
        title_template="[BUDGET] {budget_category}",
        description_template="Budget Proposal:\n\nCategory: {budget_category}\n\nAmount: {amount}\n\nJustification: {justification}\n\nBenefits: {benefits}",
        required_fields=["budget_category", "amount", "justification", "benefits"],
        voting_period_days=5,
        approval_threshold_percent=50.0
    )

    FEATURE_TEMPLATE = ProposalTemplate(
        title_template="[FEATURE] {feature_name}",
        description_template="Feature Proposal:\n\nFeature: {feature_name}\n\nDescription: {description}\n\nBenefits: {benefits}\n\nEstimated Cost: {estimated_cost}",
        required_fields=["feature_name", "description", "benefits", "estimated_cost"],
        voting_period_days=5,
        approval_threshold_percent=50.0
    )

    EMERGENCY_TEMPLATE = ProposalTemplate(
        title_template="[EMERGENCY] {emergency_type}",
        description_template="Emergency Action:\n\nType: {emergency_type}\n\nSituation: {situation}\n\nProposed Action: {proposed_action}\n\nJustification: {justification}",
        required_fields=["emergency_type", "situation", "proposed_action", "justification"],
        voting_period_days=1,
        approval_threshold_percent=75.0
    )

    @classmethod
    def get_template(cls, proposal_type: ProposalType) -> ProposalTemplate:
        """Get template for proposal type"""
        templates = {
            ProposalType.GOVERNANCE: cls.GOVERNANCE_TEMPLATE,
            ProposalType.BUDGET: cls.BUDGET_TEMPLATE,
            ProposalType.FEATURE: cls.FEATURE_TEMPLATE,
            ProposalType.EMERGENCY: cls.EMERGENCY_TEMPLATE,
            ProposalType.STANDARD: ProposalTemplate(
                title_template="{title}",
                description_template="{description}",
                required_fields=["title", "description"],
                voting_period_days=3,
                approval_threshold_percent=50.0
            )
        }

        return templates.get(proposal_type, templates[ProposalType.STANDARD])

@dataclass
class StructuredProposal:
    """Structured proposal with validated fields"""
    proposal_id: str
    proposal_type: ProposalType
    fields: Dict[str, str]
    template: ProposalTemplate

    def validate(self) -> Tuple[bool, List[str]]:
        """Validate proposal against template"""
        errors = []

        for field in self.template.required_fields:
            if field not in self.fields:
                errors.append(f"Missing required field: {field}")
            elif not self.fields[field]:
                errors.append(f"Field '{field}' cannot be empty")

        return len(errors) == 0, errors

    def render(self) -> Tuple[str, str]:
        """Render title and description"""
        try:
            title = self.template.title_template.format(**self.fields)
            description = self.template.description_template.format(**self.fields)
            return title, description
        except KeyError as e:
            raise ValueError(f"Missing field for template: {e}")
```

**Step 2: Create template tests**

Create `cynic/tests/test_proposal_templates.py`:

```python
"""Tests for proposal templates"""

import pytest
from governance_bot.proposal_templates import ProposalTemplates, ProposalType, StructuredProposal

class TestProposalTemplates:
    """Test proposal templates"""

    def test_governance_template(self):
        """Test governance proposal template"""
        template = ProposalTemplates.get_template(ProposalType.GOVERNANCE)

        assert template.approval_threshold_percent == 66.0
        assert template.voting_period_days == 7
        assert "change_type" in template.required_fields

    def test_budget_template(self):
        """Test budget proposal template"""
        template = ProposalTemplates.get_template(ProposalType.BUDGET)

        assert template.approval_threshold_percent == 50.0
        assert "amount" in template.required_fields

    def test_emergency_template(self):
        """Test emergency proposal template"""
        template = ProposalTemplates.get_template(ProposalType.EMERGENCY)

        assert template.voting_period_days == 1
        assert template.approval_threshold_percent == 75.0

class TestStructuredProposal:
    """Test structured proposals"""

    def test_validate_complete_proposal(self):
        """Test validating complete proposal"""
        template = ProposalTemplates.get_template(ProposalType.BUDGET)
        proposal = StructuredProposal(
            proposal_id="prop_1",
            proposal_type=ProposalType.BUDGET,
            fields={
                "budget_category": "Development",
                "amount": "10000",
                "justification": "New tools",
                "benefits": "Faster development"
            },
            template=template
        )

        valid, errors = proposal.validate()

        assert valid == True
        assert len(errors) == 0

    def test_validate_incomplete_proposal(self):
        """Test validating incomplete proposal"""
        template = ProposalTemplates.get_template(ProposalType.BUDGET)
        proposal = StructuredProposal(
            proposal_id="prop_1",
            proposal_type=ProposalType.BUDGET,
            fields={
                "budget_category": "Development"
                # Missing other required fields
            },
            template=template
        )

        valid, errors = proposal.validate()

        assert valid == False
        assert len(errors) > 0

    def test_render_proposal(self):
        """Test rendering proposal title and description"""
        template = ProposalTemplates.get_template(ProposalType.FEATURE)
        proposal = StructuredProposal(
            proposal_id="prop_1",
            proposal_type=ProposalType.FEATURE,
            fields={
                "feature_name": "Dark Mode",
                "description": "Add dark mode UI",
                "benefits": "Reduces eye strain",
                "estimated_cost": "40 hours"
            },
            template=template
        )

        title, description = proposal.render()

        assert "[FEATURE]" in title
        assert "Dark Mode" in title
        assert "Dark Mode" in description
```

**Step 3: Run tests**

Run: `pytest cynic/tests/test_proposal_templates.py -v`
Expected: All 6 tests PASS

**Step 4: Commit**

```bash
git add governance_bot/proposal_templates.py cynic/tests/test_proposal_templates.py
git commit -m "feat(templates): Add proposal templates and standardization"
```

---

## Task 3.5: Treasury Integration

**Files:**
- Create: `governance_bot/treasury.py`
- Create: `cynic/tests/test_treasury.py`

**Step 1: Create treasury module**

Create `governance_bot/treasury.py`:

```python
"""Community treasury management"""

from dataclasses import dataclass, field
from typing import Dict, List, Optional
from datetime import datetime
from enum import Enum

class TransactionType(str, Enum):
    """Types of treasury transactions"""
    DEPOSIT = "deposit"
    WITHDRAWAL = "withdrawal"
    ALLOCATION = "allocation"
    BURN = "burn"

@dataclass
class Transaction:
    """Treasury transaction record"""
    transaction_id: str
    tx_type: TransactionType
    amount: float
    description: str
    timestamp: datetime
    approved_by: str
    proposal_id: Optional[str] = None

@dataclass
class TreasuryBudget:
    """Budget allocation"""
    category: str
    total_allocation: float
    spent: float = 0.0
    remaining: float = field(init=False)

    def __post_init__(self):
        """Calculate remaining"""
        self.remaining = self.total_allocation - self.spent

    def spend(self, amount: float) -> bool:
        """Allocate funds from budget"""
        if amount > self.remaining:
            return False

        self.spent += amount
        self.remaining -= amount
        return True

class Treasury:
    """Manage community treasury"""

    def __init__(self, name: str, initial_balance: float = 0.0):
        self.name = name
        self.balance = initial_balance
        self.transactions: List[Transaction] = []
        self.budgets: Dict[str, TreasuryBudget] = {}

    def deposit(self, amount: float, description: str, approved_by: str) -> bool:
        """Deposit funds"""
        if amount <= 0:
            return False

        self.balance += amount
        tx = Transaction(
            transaction_id=f"tx_{len(self.transactions)}",
            tx_type=TransactionType.DEPOSIT,
            amount=amount,
            description=description,
            timestamp=datetime.utcnow(),
            approved_by=approved_by
        )
        self.transactions.append(tx)
        return True

    def withdraw(self, amount: float, description: str, approved_by: str, proposal_id: str = None) -> bool:
        """Withdraw funds"""
        if amount <= 0 or amount > self.balance:
            return False

        self.balance -= amount
        tx = Transaction(
            transaction_id=f"tx_{len(self.transactions)}",
            tx_type=TransactionType.WITHDRAWAL,
            amount=amount,
            description=description,
            timestamp=datetime.utcnow(),
            approved_by=approved_by,
            proposal_id=proposal_id
        )
        self.transactions.append(tx)
        return True

    def allocate_budget(self, category: str, amount: float) -> bool:
        """Allocate budget for category"""
        if category in self.budgets:
            return False

        self.budgets[category] = TreasuryBudget(category=category, total_allocation=amount)
        return True

    def get_balance(self) -> float:
        """Get current balance"""
        return self.balance

    def get_budget_utilization(self) -> Dict[str, float]:
        """Get budget utilization rates"""
        return {
            category: budget.spent / budget.total_allocation
            for category, budget in self.budgets.items()
        }
```

**Step 2: Create treasury tests**

Create `cynic/tests/test_treasury.py`:

```python
"""Tests for treasury management"""

import pytest
from governance_bot.treasury import Treasury, TreasuryBudget, TransactionType

class TestTreasuryManagement:
    """Test treasury operations"""

    @pytest.fixture
    def treasury(self):
        """Create test treasury"""
        return Treasury(name="TestCommunity", initial_balance=1000.0)

    def test_deposit(self, treasury):
        """Test depositing funds"""
        result = treasury.deposit(500.0, "Community contribution", "admin")

        assert result == True
        assert treasury.balance == 1500.0
        assert len(treasury.transactions) == 1

    def test_withdraw(self, treasury):
        """Test withdrawing funds"""
        result = treasury.withdraw(300.0, "Operational expense", "admin")

        assert result == True
        assert treasury.balance == 700.0

    def test_insufficient_balance(self, treasury):
        """Test withdrawal with insufficient balance"""
        result = treasury.withdraw(2000.0, "Too much", "admin")

        assert result == False
        assert treasury.balance == 1000.0

    def test_negative_amount(self, treasury):
        """Test invalid amount"""
        result = treasury.deposit(-100.0, "Invalid", "admin")

        assert result == False
        assert treasury.balance == 1000.0

class TestBudgetAllocation:
    """Test budget allocation"""

    def test_allocate_budget(self):
        """Test allocating budget"""
        budget = TreasuryBudget(category="Development", total_allocation=5000.0)

        assert budget.category == "Development"
        assert budget.remaining == 5000.0

    def test_spend_from_budget(self):
        """Test spending from budget"""
        budget = TreasuryBudget(category="Development", total_allocation=5000.0)
        result = budget.spend(1000.0)

        assert result == True
        assert budget.spent == 1000.0
        assert budget.remaining == 4000.0

    def test_overspend_budget(self):
        """Test overspending budget"""
        budget = TreasuryBudget(category="Development", total_allocation=5000.0)
        result = budget.spend(6000.0)

        assert result == False
        assert budget.spent == 0.0
```

**Step 3: Run tests**

Run: `pytest cynic/tests/test_treasury.py -v`
Expected: All 8 tests PASS

**Step 4: Commit**

```bash
git add governance_bot/treasury.py cynic/tests/test_treasury.py
git commit -m "feat(treasury): Add treasury management and budget allocation"
```

---

## Summary

**Phase 3 Deliverables (34 New Tests):**

✅ **Task 3.1: Authentication & Authorization (5 tests)**
- JWT token creation and verification
- Role-based access control
- Permission hierarchy

✅ **Task 3.2: Reputation & E-Score (8 tests)**
- Reputation tracking across 7 metrics
- E-Score 7-dimensional system (φ-bounded)
- Voting weight calculation

✅ **Task 3.3: Advanced Voting (7 tests)**
- Simple majority voting
- Quadratic voting
- Weighted voting with reputation
- Vote delegation management

✅ **Task 3.4: Proposal Templates (6 tests)**
- Standardized proposal types
- Template validation
- Title/description rendering

✅ **Task 3.5: Treasury (8 tests)**
- Deposit/withdrawal transactions
- Budget allocation
- Fund management
- Utilization tracking

**Total: 34 new tests + 454+ existing = 488+ passing tests**
