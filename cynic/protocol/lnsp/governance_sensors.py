"""Governance sensors for LNSP Layer 1 observation."""
from __future__ import annotations

import dataclasses
from typing import Any

from cynic.protocol.lnsp.governance_events import (
    GovernanceExecutionPayload,
    GovernanceOutcomePayload,
    GovernanceProposalPayload,
    GovernanceVotePayload,
)
from cynic.protocol.lnsp.layer1 import Sensor
from cynic.protocol.lnsp.messages import create_raw_observation
from cynic.protocol.lnsp.types import LNSPMessage, ObservationType


class ProposalSensor(Sensor):
    """Sensor that converts GovernanceProposalPayload into LNSP observations.

    Observation type: HUMAN_INPUT (governance proposals are community-driven input).

    Attributes:
        pending_payloads: Queue of proposal payloads awaiting conversion.
    """

    def __init__(self, sensor_id: str, instance_id: str = "instance:local") -> None:
        """Initialize ProposalSensor.

        Args:
            sensor_id: Unique identifier for this sensor.
            instance_id: Organism instance ID (default: "instance:local").
        """
        super().__init__(sensor_id, instance_id)
        self.pending_payloads: list[GovernanceProposalPayload] = []

    async def observe(self) -> LNSPMessage | None:
        """Convert first pending proposal payload to a RAW LNSP observation.

        Returns:
            LNSPMessage with HUMAN_INPUT observation type, or None if no payloads.
        """
        if not self.pending_payloads:
            return None
        payload = self.pending_payloads.pop(0)
        data: dict[str, Any] = {"data": dataclasses.asdict(payload)}
        return create_raw_observation(
            observation_type=ObservationType.HUMAN_INPUT,
            data=data,
            source=self.sensor_id,
            instance_id=self.instance_id,
        )


class VoteSensor(Sensor):
    """Sensor that converts GovernanceVotePayload into LNSP observations.

    Observation type: HUMAN_INPUT (votes are community-driven input).

    Attributes:
        pending_payloads: Queue of vote payloads awaiting conversion.
    """

    def __init__(self, sensor_id: str, instance_id: str = "instance:local") -> None:
        """Initialize VoteSensor.

        Args:
            sensor_id: Unique identifier for this sensor.
            instance_id: Organism instance ID (default: "instance:local").
        """
        super().__init__(sensor_id, instance_id)
        self.pending_payloads: list[GovernanceVotePayload] = []

    async def observe(self) -> LNSPMessage | None:
        """Convert first pending vote payload to a RAW LNSP observation.

        Returns:
            LNSPMessage with HUMAN_INPUT observation type, or None if no payloads.
        """
        if not self.pending_payloads:
            return None
        payload = self.pending_payloads.pop(0)
        data: dict[str, Any] = {"data": dataclasses.asdict(payload)}
        return create_raw_observation(
            observation_type=ObservationType.HUMAN_INPUT,
            data=data,
            source=self.sensor_id,
            instance_id=self.instance_id,
        )


class ExecutionSensor(Sensor):
    """Sensor that converts GovernanceExecutionPayload into LNSP observations.

    Observation type: ACTION_RESULT (execution is the result of a governance action).

    Attributes:
        pending_payloads: Queue of execution payloads awaiting conversion.
    """

    def __init__(self, sensor_id: str, instance_id: str = "instance:local") -> None:
        """Initialize ExecutionSensor.

        Args:
            sensor_id: Unique identifier for this sensor.
            instance_id: Organism instance ID (default: "instance:local").
        """
        super().__init__(sensor_id, instance_id)
        self.pending_payloads: list[GovernanceExecutionPayload] = []

    async def observe(self) -> LNSPMessage | None:
        """Convert first pending execution payload to a RAW LNSP observation.

        Returns:
            LNSPMessage with ACTION_RESULT observation type, or None if no payloads.
        """
        if not self.pending_payloads:
            return None
        payload = self.pending_payloads.pop(0)
        data: dict[str, Any] = {"data": dataclasses.asdict(payload)}
        return create_raw_observation(
            observation_type=ObservationType.ACTION_RESULT,
            data=data,
            source=self.sensor_id,
            instance_id=self.instance_id,
        )


class OutcomeSensor(Sensor):
    """Sensor that converts GovernanceOutcomePayload into LNSP observations.

    Observation type: ECOSYSTEM_EVENT (outcomes are external ecosystem feedback).

    Attributes:
        pending_payloads: Queue of outcome payloads awaiting conversion.
    """

    def __init__(self, sensor_id: str, instance_id: str = "instance:local") -> None:
        """Initialize OutcomeSensor.

        Args:
            sensor_id: Unique identifier for this sensor.
            instance_id: Organism instance ID (default: "instance:local").
        """
        super().__init__(sensor_id, instance_id)
        self.pending_payloads: list[GovernanceOutcomePayload] = []

    async def observe(self) -> LNSPMessage | None:
        """Convert first pending outcome payload to a RAW LNSP observation.

        Returns:
            LNSPMessage with ECOSYSTEM_EVENT observation type, or None if no payloads.
        """
        if not self.pending_payloads:
            return None
        payload = self.pending_payloads.pop(0)
        data: dict[str, Any] = {"data": dataclasses.asdict(payload)}
        return create_raw_observation(
            observation_type=ObservationType.ECOSYSTEM_EVENT,
            data=data,
            source=self.sensor_id,
            instance_id=self.instance_id,
        )
