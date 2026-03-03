"""Proposal templates for standardized governance"""

from dataclasses import dataclass
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
    required_fields: list[str]
    voting_period_days: int = 3
    approval_threshold_percent: float = 50.0


class ProposalTemplates:
    """Collection of proposal templates"""

    GOVERNANCE_TEMPLATE = ProposalTemplate(
        title_template="[GOV] {change_type}",
        description_template="Governance Change Proposal:\n\nChange Type: {change_type}\n\nRationale: {rationale}\n\nImpact: {impact}",
        required_fields=["change_type", "rationale", "impact"],
        voting_period_days=7,
        approval_threshold_percent=66.0,
    )

    BUDGET_TEMPLATE = ProposalTemplate(
        title_template="[BUDGET] {budget_category}",
        description_template="Budget Proposal:\n\nCategory: {budget_category}\n\nAmount: {amount}\n\nJustification: {justification}\n\nBenefits: {benefits}",
        required_fields=["budget_category", "amount", "justification", "benefits"],
        voting_period_days=5,
        approval_threshold_percent=50.0,
    )

    FEATURE_TEMPLATE = ProposalTemplate(
        title_template="[FEATURE] {feature_name}",
        description_template="Feature Proposal:\n\nFeature: {feature_name}\n\nDescription: {description}\n\nBenefits: {benefits}\n\nEstimated Cost: {estimated_cost}",
        required_fields=["feature_name", "description", "benefits", "estimated_cost"],
        voting_period_days=5,
        approval_threshold_percent=50.0,
    )

    EMERGENCY_TEMPLATE = ProposalTemplate(
        title_template="[EMERGENCY] {emergency_type}",
        description_template="Emergency Action:\n\nType: {emergency_type}\n\nSituation: {situation}\n\nProposed Action: {proposed_action}\n\nJustification: {justification}",
        required_fields=[
            "emergency_type",
            "situation",
            "proposed_action",
            "justification",
        ],
        voting_period_days=1,
        approval_threshold_percent=75.0,
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
                approval_threshold_percent=50.0,
            ),
        }

        return templates.get(proposal_type, templates[ProposalType.STANDARD])


@dataclass
class StructuredProposal:
    """Structured proposal with validated fields"""

    proposal_id: str
    proposal_type: ProposalType
    fields: dict[str, str]
    template: ProposalTemplate

    def validate(self) -> tuple[bool, list[str]]:
        """Validate proposal against template"""
        errors = []

        for field in self.template.required_fields:
            if field not in self.fields:
                errors.append(f"Missing required field: {field}")
            elif not self.fields[field]:
                errors.append(f"Field '{field}' cannot be empty")

        return len(errors) == 0, errors

    def render(self) -> tuple[str, str]:
        """Render title and description"""
        try:
            title = self.template.title_template.format(**self.fields)
            description = self.template.description_template.format(**self.fields)
            return title, description
        except KeyError as e:
            raise ValueError(f"Missing field for template: {e}")
