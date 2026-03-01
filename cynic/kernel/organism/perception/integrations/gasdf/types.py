"""GASdf integration types and dataclasses."""

from __future__ import annotations

from dataclasses import dataclass


class GASdfError(Exception):
    """Exception raised for GASdf API errors."""

    pass


@dataclass
class GASdfExecutionResult:
    """Result of a GASdf execution.

    Attributes:
        signature: Transaction signature on-chain
        status: Execution status (e.g., "confirmed", "pending")
        fee_amount: Fee charged for execution
        fee_token: Token used for fee payment
        quote_id: Quote ID from GASdf
    """

    signature: str
    status: str
    fee_amount: int
    fee_token: str
    quote_id: str


@dataclass
class GASdfQuote:
    """Fee quote from GASdf.

    Attributes:
        quote_id: Unique quote identifier
        payment_token: Token to use for payment
        fee_amount: Fee in lamports
        burn_amount: Amount burned (76.4% of fee)
        user_pubkey: User's public key
    """

    quote_id: str
    payment_token: str
    fee_amount: int
    burn_amount: int
    user_pubkey: str


@dataclass
class GASdfStats:
    """Burn statistics from GASdf.

    Attributes:
        total_burned: Total amount burned to $ASDF (lamports)
        total_transactions: Number of transactions processed
        burned_formatted: Human-readable burned amount
        treasury: Treasury information dict
    """

    total_burned: int
    total_transactions: int
    burned_formatted: str
    treasury: dict[str, object]
