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

    def __init__(self, manager: LNSPManager) -> None:
        """Initialize the governance LNSP bridge."""
        self.manager = manager

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
