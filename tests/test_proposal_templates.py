"""Tests for proposal templates"""

import pytest

pytestmark = pytest.mark.skip(reason="Old architecture removed in V5 - governance_bot module not found")

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

        assert valid is True
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

        assert valid is False
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
