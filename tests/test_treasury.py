"""Tests for treasury management"""

import pytest

pytestmark = pytest.mark.skip(reason="Old architecture removed in V5 - governance_bot module not found")

from governance_bot.treasury import Treasury, TreasuryBudget


class TestTreasuryManagement:
    """Test treasury operations"""

    @pytest.fixture
    def treasury(self):
        """Create test treasury"""
        return Treasury(name="TestCommunity", initial_balance=1000.0)

    def test_deposit(self, treasury):
        """Test depositing funds"""
        result = treasury.deposit(500.0, "Community contribution", "admin")

        assert result is True
        assert treasury.balance == 1500.0
        assert len(treasury.transactions) == 1

    def test_withdraw(self, treasury):
        """Test withdrawing funds"""
        result = treasury.withdraw(300.0, "Operational expense", "admin")

        assert result is True
        assert treasury.balance == 700.0

    def test_insufficient_balance(self, treasury):
        """Test withdrawal with insufficient balance"""
        result = treasury.withdraw(2000.0, "Too much", "admin")

        assert result is False
        assert treasury.balance == 1000.0

    def test_negative_amount(self, treasury):
        """Test invalid amount"""
        result = treasury.deposit(-100.0, "Invalid", "admin")

        assert result is False
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

        assert result is True
        assert budget.spent == 1000.0
        assert budget.remaining == 4000.0

    def test_overspend_budget(self):
        """Test overspending budget"""
        budget = TreasuryBudget(category="Development", total_allocation=5000.0)
        result = budget.spend(6000.0)

        assert result is False
        assert budget.spent == 0.0
