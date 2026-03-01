"""NEAR Protocol integration types and dataclasses."""
from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class NEARError(Exception):
    """Exception raised for NEAR Protocol errors."""

    pass


class TxStatus(str, Enum):
    """Transaction status on NEAR."""

    PENDING = "pending"
    CONFIRMED = "confirmed"
    FAILED = "failed"


@dataclass
class NEARAccount:
    """NEAR blockchain account information.

    Attributes:
        account_id: NEAR account ID (e.g., cynic.near)
        public_key: Account public key for signing
        private_key: Account private key (keep secret!)
    """

    account_id: str
    public_key: str
    private_key: str = ""  # Should be loaded from secure storage


@dataclass
class NEARTransaction:
    """NEAR transaction details.

    Attributes:
        signer_id: Account signing the transaction
        receiver_id: Account receiving the transaction
        actions: List of actions to execute
        nonce: Transaction nonce (incremented per tx)
        block_hash: Hash of referenced block
    """

    signer_id: str
    receiver_id: str
    actions: list[dict]
    nonce: int
    block_hash: str


@dataclass
class NEARExecutionResult:
    """Result of a NEAR smart contract execution.

    Attributes:
        transaction_hash: On-chain transaction hash
        block_height: Block where transaction was included
        status: Execution status (pending/confirmed/failed)
        gas_used: Gas consumed by transaction
        outcome: Contract call outcome/return value
        cynic_verdict: CYNIC verdict that triggered this execution
        proposal_id: Governance proposal ID (if applicable)
    """

    transaction_hash: str
    block_height: int
    status: TxStatus
    gas_used: int
    outcome: dict
    cynic_verdict: str = ""
    proposal_id: str = ""


@dataclass
class NEARContractCall:
    """Smart contract call parameters.

    Attributes:
        method_name: Name of contract method to call
        args: Arguments to pass to method (as dict)
        gas: Gas to attach (nanoseconds)
        deposit: NEAR tokens to attach (yoctoNEAR)
    """

    method_name: str
    args: dict
    gas: int = 30_000_000_000_000  # 30 TGas (default)
    deposit: str = "0"  # 0 NEAR by default


@dataclass
class NEARGovernanceProposal:
    """Governance proposal stored on NEAR blockchain.

    Attributes:
        proposal_id: Unique proposal identifier
        title: Proposal title
        description: Proposal description
        cynic_verdict: CYNIC's judgment (HOWL/WAG/GROWL/BARK)
        q_score: CYNIC's confidence score (0-100)
        votes_for: Number of votes supporting
        votes_against: Number of votes opposing
        votes_abstain: Number of abstaining votes
        status: Proposal status (open/executed/failed)
        created_at: Timestamp when created
        expires_at: Timestamp when voting closes
    """

    proposal_id: str
    title: str
    description: str
    cynic_verdict: str
    q_score: float
    votes_for: int = 0
    votes_against: int = 0
    votes_abstain: int = 0
    status: str = "open"
    created_at: int = 0
    expires_at: int = 0


@dataclass
class NEARNetworkConfig:
    """NEAR network configuration.

    Attributes:
        network_id: Network identifier (mainnet/testnet)
        rpc_url: RPC endpoint URL
        contract_id: Governance contract account ID
        master_account: Account owning the contract
    """

    network_id: str
    rpc_url: str
    contract_id: str
    master_account: str
