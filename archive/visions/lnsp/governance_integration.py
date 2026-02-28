"""LNSP governance integration bridge."""
from __future__ import annotations

from typing import Any

from cynic.protocol.lnsp.governance_handlers import GovernanceVerdictHandler
from cynic.protocol.lnsp.governance_sensors import (
    ExecutionSensor,
    OutcomeSensor,
    ProposalSensor,
    VoteSensor,
)
from cynic.protocol.lnsp.manager import LNSPManager
from cynic.protocol.lnsp.types import LNSPMessage


class GovernanceLNSP:
    """Bridges CYNIC event bus to LNSP pipeline for governance."""

    def __init__(
        self,
        manager: LNSPManager,
        gasdf_executor: Any | None = None,
    ) -> None:
        """Initialize the governance LNSP bridge.

        Args:
            manager: LNSPManager instance for the LNSP pipeline
            gasdf_executor: Optional GASdfExecutor for on-chain execution
        """
        self.manager = manager
        self.gasdf_executor = gasdf_executor

        # Add verdict cache for feedback loop (proposal_id -> verdict info)
        self.verdict_cache: dict[str, dict[str, Any]] = {}

        # Create sensors
        self.proposal_sensor = ProposalSensor("sensor:proposal")
        self.vote_sensor = VoteSensor("sensor:vote")
        self.execution_sensor = ExecutionSensor("sensor:execution")
        self.outcome_sensor = OutcomeSensor("sensor:outcome")

        # Create handler
        self.governance_handler = GovernanceVerdictHandler("handler:governance")

    async def setup(self) -> None:
        """Initialize and wire all components."""
        # Register sensors with Layer 1
        self.manager.layer1.register_sensor(self.proposal_sensor)
        self.manager.layer1.register_sensor(self.vote_sensor)
        self.manager.layer1.register_sensor(self.execution_sensor)
        self.manager.layer1.register_sensor(self.outcome_sensor)

        # Register handler with Layer 4
        self.manager.layer4.register_handler(self.governance_handler)

        # Wire all layers together
        self.manager.wire_layers()

        # Register feedback handler for learning loop
        def capture_verdict(msg: LNSPMessage) -> None:
            # When a verdict is emitted, store it for feedback tracking
            data = msg.payload.get("data", {})
            proposal_id = data.get("proposal_id")
            if proposal_id:
                self.verdict_cache[proposal_id] = {
                    "verdict_type": data.get("verdict_type"),
                    "q_score": data.get("q_score"),
                    "timestamp": msg.header.timestamp,
                }

                # If GASdfExecutor is available, execute the verdict on-chain
                if self.gasdf_executor is not None:
                    # Schedule async execution (non-blocking)
                    import asyncio
                    asyncio.create_task(
                        self._execute_verdict_on_chain(data, proposal_id)
                    )

        self.manager.layer4.on_feedback(capture_verdict)

    async def process_proposal(self, proposal: dict[str, Any]) -> None:
        """Process a governance proposal."""
        from cynic.protocol.lnsp.governance_events import GovernanceProposalPayload

        payload = GovernanceProposalPayload(**proposal)
        self.proposal_sensor.pending_payloads.append(payload)
        await self.manager.run_cycle()

    async def process_vote(self, vote: dict[str, Any]) -> None:
        """Process a governance vote."""
        from cynic.protocol.lnsp.governance_events import GovernanceVotePayload

        payload = GovernanceVotePayload(**vote)
        self.vote_sensor.pending_payloads.append(payload)
        await self.manager.run_cycle()

    async def process_execution(self, execution: dict[str, Any]) -> None:
        """Process a governance execution outcome."""
        from cynic.protocol.lnsp.governance_events import GovernanceExecutionPayload

        payload = GovernanceExecutionPayload(**execution)
        self.execution_sensor.pending_payloads.append(payload)
        await self.manager.run_cycle()

    async def process_outcome(self, outcome: dict[str, Any]) -> None:
        """Process community outcome feedback."""
        from cynic.protocol.lnsp.governance_events import GovernanceOutcomePayload

        payload = GovernanceOutcomePayload(**outcome)
        self.outcome_sensor.pending_payloads.append(payload)
        await self.manager.run_cycle()

    async def on_execution_completed(self, execution_data: dict[str, Any]) -> None:
        """Handle execution completion and feed back to learning."""
        proposal_id = execution_data.get("proposal_id")
        success = execution_data.get("success", False)

        # Log: was the verdict correct?
        if proposal_id in self.verdict_cache:
            verdict_info = self.verdict_cache[proposal_id]
            # In a real system, this would update the Q-table
            # For now, just track it
            verdict_info["execution_success"] = success

    async def on_outcome_feedback(self, feedback_data: dict[str, Any]) -> None:
        """Handle community outcome feedback for learning."""
        proposal_id = feedback_data.get("proposal_id")
        accepted = feedback_data.get("accepted", False)

        # Log: was the community satisfied?
        if proposal_id in self.verdict_cache:
            verdict_info = self.verdict_cache[proposal_id]
            verdict_info["community_accepted"] = accepted
            # Q-table learning would happen here in full implementation

    async def _execute_verdict_on_chain(
        self, verdict_data: dict[str, Any], proposal_id: str
    ) -> None:
        """Execute a governance verdict on-chain via GASdf.

        This method is called asynchronously when a verdict is emitted
        to avoid blocking the feedback loop. It coordinates with the
        GASdfExecutor to perform the actual on-chain execution.

        Args:
            verdict_data: Verdict information from the LNSP message
            proposal_id: Proposal identifier
        """
        if self.gasdf_executor is None:
            return

        try:
            # Extract verdict info (mapped from LNSP VerdictType to CYNIC verdict)
            verdict = verdict_data.get("verdict", "UNKNOWN")

            # Execute the verdict on-chain
            # In a full implementation, this would:
            # 1. Construct the transaction based on the proposal
            # 2. Sign it (with appropriate keys from the proposal context)
            # 3. Call gasdf_executor.execute_verdict()
            #
            # For now, we log that execution would happen
            result = await self.gasdf_executor.execute_verdict(
                proposal_id=proposal_id,
                verdict=verdict,
                community_id=verdict_data.get("community_id", "unknown"),
                payment_token=verdict_data.get("payment_token", ""),
                user_pubkey=verdict_data.get("user_pubkey", ""),
                signed_transaction=verdict_data.get("signed_transaction", ""),
                payment_token_account=verdict_data.get("payment_token_account", ""),
            )

            # Track execution result
            if result is not None:
                if proposal_id in self.verdict_cache:
                    self.verdict_cache[proposal_id]["execution_result"] = {
                        "signature": result.signature,
                        "status": result.status,
                        "fee_amount": result.fee_amount,
                        "fee_token": result.fee_token,
                    }

        except Exception as e:
            # Log execution error but don't propagate
            if proposal_id in self.verdict_cache:
                self.verdict_cache[proposal_id]["execution_error"] = str(e)
