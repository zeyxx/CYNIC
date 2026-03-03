"""Community treasury management"""

from dataclasses import dataclass, field
from datetime import UTC, datetime
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
    proposal_id: str | None = None


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
        self.transactions: list[Transaction] = []
        self.budgets: dict[str, TreasuryBudget] = {}

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
            timestamp=datetime.now(UTC),
            approved_by=approved_by,
        )
        self.transactions.append(tx)
        return True

    def withdraw(
        self, amount: float, description: str, approved_by: str, proposal_id: str = None
    ) -> bool:
        """Withdraw funds"""
        if amount <= 0 or amount > self.balance:
            return False

        self.balance -= amount
        tx = Transaction(
            transaction_id=f"tx_{len(self.transactions)}",
            tx_type=TransactionType.WITHDRAWAL,
            amount=amount,
            description=description,
            timestamp=datetime.now(UTC),
            approved_by=approved_by,
            proposal_id=proposal_id,
        )
        self.transactions.append(tx)
        return True

    def allocate_budget(self, category: str, amount: float) -> bool:
        """Allocate budget for category"""
        if category in self.budgets:
            return False

        self.budgets[category] = TreasuryBudget(
            category=category, total_allocation=amount
        )
        return True

    def get_balance(self) -> float:
        """Get current balance"""
        return self.balance

    def get_budget_utilization(self) -> dict[str, float]:
        """Get budget utilization rates"""
        return {
            category: budget.spent / budget.total_allocation
            for category, budget in self.budgets.items()
        }
