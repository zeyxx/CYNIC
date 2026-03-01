"""NEAR Protocol executor for governance actions."""

from __future__ import annotations

import logging

from .rpc_client import NEARRPCClient
from .types import (
    NEARContractCall,
    NEARError,
    NEARExecutionResult,
    NEARGovernanceProposal,
    NEARNetworkConfig,
    TxStatus,
)

logger = logging.getLogger(__name__)


class NEARExecutor:
    """Execute governance actions on NEAR Protocol.

    Handles:
    - Submitting proposals to governance contract
    - Recording CYNIC verdicts on-chain
    - Executing approved governance decisions
    - Tracking proposal status and outcomes

    Attributes:
        config: NEAR network configuration
        rpc_client: RPC client for blockchain interaction
    """

    def __init__(self, config: NEARNetworkConfig):
        """Initialize NEAR executor.

        Args:
            config: Network configuration with contract addresses
        """
        self.config = config
        self.rpc_client = NEARRPCClient(config)

    async def health(self) -> bool:
        """Check if NEAR network is accessible.

        Returns:
            True if healthy, False otherwise
        """
        try:
            await self.rpc_client.health()
            return True
        except NEARError as e:
            logger.warning("NEAR health check failed: %s", e)
            return False

    async def submit_proposal(
        self,
        proposal_id: str,
        title: str,
        description: str,
        cynic_verdict: str,
        q_score: float,
        signer_id: str,
        expires_at: int,
    ) -> NEARExecutionResult:
        """Submit a governance proposal to NEAR.

        Creates a proposal on the governance contract with CYNIC's verdict
        attached. The proposal includes voting period and initial state.

        Args:
            proposal_id: Unique proposal identifier
            title: Proposal title
            description: Full proposal description
            cynic_verdict: CYNIC's verdict (HOWL/WAG/GROWL/BARK)
            q_score: CYNIC's confidence (0-100)
            signer_id: Account submitting proposal
            expires_at: Unix timestamp when voting closes

        Returns:
            Execution result with transaction hash and status

        Raises:
            NEARError: If submission fails
        """
        contract_call = NEARContractCall(
            method_name="create_proposal",
            args={
                "proposal_id": proposal_id,
                "title": title,
                "description": description,
                "cynic_verdict": cynic_verdict,
                "cynic_q_score": q_score,
                "expires_at": expires_at,
            },
            gas=300_000_000_000_000,  # 300 TGas
            deposit="1000000000000000000000000",  # 1 NEAR
        )

        return await self._execute_contract_call(
            signer_id, contract_call, proposal_id, cynic_verdict
        )

    async def record_vote(
        self,
        proposal_id: str,
        voter_id: str,
        vote: str,
        weight: int = 1,
    ) -> NEARExecutionResult:
        """Record a vote on a governance proposal.

        Args:
            proposal_id: Proposal to vote on
            voter_id: Account voting
            vote: Vote type (for/against/abstain)
            weight: Vote weight (default: 1)

        Returns:
            Execution result

        Raises:
            NEARError: If recording fails
        """
        if vote not in ("for", "against", "abstain"):
            raise NEARError(f"Invalid vote type: {vote}")

        contract_call = NEARContractCall(
            method_name="vote",
            args={
                "proposal_id": proposal_id,
                "vote": vote,
                "weight": weight,
            },
            gas=100_000_000_000_000,  # 100 TGas
        )

        return await self._execute_contract_call(
            voter_id, contract_call, proposal_id, f"vote:{vote}"
        )

    async def execute_proposal(
        self,
        proposal_id: str,
        executor_id: str,
    ) -> NEARExecutionResult:
        """Execute an approved governance proposal.

        Calls the governance contract to finalize a proposal that has
        passed voting. The contract enforces approval thresholds.

        Args:
            proposal_id: Proposal to execute
            executor_id: Account executing (can be any account)

        Returns:
            Execution result

        Raises:
            NEARError: If execution fails
        """
        contract_call = NEARContractCall(
            method_name="execute_proposal",
            args={"proposal_id": proposal_id},
            gas=200_000_000_000_000,  # 200 TGas
        )

        return await self._execute_contract_call(executor_id, contract_call, proposal_id, "execute")

    async def query_proposal(self, proposal_id: str) -> NEARGovernanceProposal:
        """Query proposal state from blockchain.

        Args:
            proposal_id: Proposal to query

        Returns:
            Governance proposal with current state

        Raises:
            NEARError: If query fails
        """
        try:
            result = await self.rpc_client.call_contract(
                self.config.contract_id,
                "get_proposal",
                {"proposal_id": proposal_id},
            )

            # Parse contract response
            result_data = result.get("result", {})
            if isinstance(result_data, bytes):
                import json

                result_data = json.loads(result_data.decode())

            return NEARGovernanceProposal(
                proposal_id=result_data.get("proposal_id", proposal_id),
                title=result_data.get("title", ""),
                description=result_data.get("description", ""),
                cynic_verdict=result_data.get("cynic_verdict", "UNKNOWN"),
                q_score=float(result_data.get("cynic_q_score", 0)),
                votes_for=int(result_data.get("votes_for", 0)),
                votes_against=int(result_data.get("votes_against", 0)),
                votes_abstain=int(result_data.get("votes_abstain", 0)),
                status=result_data.get("status", "open"),
                created_at=int(result_data.get("created_at", 0)),
                expires_at=int(result_data.get("expires_at", 0)),
            )
        except Exception as e:
            raise NEARError(f"Query proposal failed: {e}")

    async def _execute_contract_call(
        self,
        signer_id: str,
        contract_call: NEARContractCall,
        proposal_id: str,
        cynic_verdict: str,
    ) -> NEARExecutionResult:
        """Internal: Execute a contract call transaction.

        Args:
            signer_id: Account signing transaction
            contract_call: Contract method and args
            proposal_id: Associated proposal ID
            cynic_verdict: CYNIC verdict (for logging)

        Returns:
            Execution result

        Raises:
            NEARError: If execution fails
        """
        try:
            # Get nonce and block hash
            await self.rpc_client.get_nonce(signer_id)
            await self.rpc_client.get_block_hash()

            logger.info(
                "Executing contract call: %s.%s (proposal: %s, verdict: %s)",
                self.config.contract_id,
                contract_call.method_name,
                proposal_id,
                cynic_verdict,
            )

            # In production, would sign transaction here with private key
            # For now, we return a pending result
            # TODO: Implement proper transaction signing using near-api-py

            return NEARExecutionResult(
                transaction_hash="",  # Would be set after signing
                block_height=0,  # Would be set after confirmation
                status=TxStatus.PENDING,
                gas_used=0,
                outcome={
                    "method": contract_call.method_name,
                    "contract": self.config.contract_id,
                    "args": contract_call.args,
                },
                cynic_verdict=cynic_verdict,
                proposal_id=proposal_id,
            )

        except Exception as e:
            logger.error("Contract execution failed: %s (proposal: %s)", e, proposal_id)
            raise NEARError(f"Contract execution failed: {e}")

    async def wait_for_confirmation(
        self,
        tx_hash: str,
        signer_id: str,
        timeout_seconds: int = 60,
    ) -> NEARExecutionResult:
        """Wait for transaction confirmation on NEAR.

        Polls the transaction status until confirmed or timeout.

        Args:
            tx_hash: Transaction hash to wait for
            signer_id: Account that signed transaction
            timeout_seconds: Max seconds to wait

        Returns:
            Final execution result

        Raises:
            NEARError: If timeout or transaction fails
        """
        import asyncio
        import time

        start_time = time.time()

        while time.time() - start_time < timeout_seconds:
            try:
                result = await self.rpc_client.get_transaction_result(tx_hash, signer_id)

                # Check transaction status
                outcome = result.get("transaction_outcome", {})
                status_obj = outcome.get("outcome", {}).get("status", {})

                if "SuccessValue" in status_obj:
                    return NEARExecutionResult(
                        transaction_hash=tx_hash,
                        block_height=result.get("transaction", {}).get("block_height", 0),
                        status=TxStatus.CONFIRMED,
                        gas_used=outcome.get("outcome", {}).get("gas_burnt", 0),
                        outcome=outcome,
                    )
                elif "Failure" in status_obj:
                    return NEARExecutionResult(
                        transaction_hash=tx_hash,
                        block_height=result.get("transaction", {}).get("block_height", 0),
                        status=TxStatus.FAILED,
                        gas_used=outcome.get("outcome", {}).get("gas_burnt", 0),
                        outcome=outcome,
                    )

                # Still pending, wait a bit
                await asyncio.sleep(2)

            except NEARError as e:
                logger.debug("Transaction not yet finalized: %s", e)
                await asyncio.sleep(2)

        raise NEARError(f"Transaction confirmation timeout after {timeout_seconds}s")
