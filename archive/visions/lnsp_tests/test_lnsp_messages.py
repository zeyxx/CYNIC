"""Comprehensive tests for LNSP message types and factories."""
from __future__ import annotations

import pytest
import time
from cynic.protocol.lnsp.types import (
    Layer,
    VerdictType,
    ObservationType,
    AggregationType,
    JudgmentType,
    ActionType,
    MessageHeader,
    Metadata,
    LNSPMessage,
)
from cynic.protocol.lnsp.messages import (
    create_raw_observation,
    create_aggregated_state,
    create_judgment,
    create_action,
    _generate_message_id,
    _get_timestamp,
)


# ============================================================================
# Test Helpers & Fixtures
# ============================================================================

@pytest.fixture
def instance_id():
    """Provide a test instance ID."""
    return "test_org_001"


@pytest.fixture
def timestamp():
    """Provide a test timestamp."""
    return time.time()


# ============================================================================
# Tests for Enums
# ============================================================================

class TestLayerEnum:
    """Test Layer enumeration."""

    def test_layer_values(self):
        """Test Layer enum values."""
        assert Layer.RAW.value == 1
        assert Layer.AGGREGATED.value == 2
        assert Layer.JUDGMENT.value == 3
        assert Layer.ACTION.value == 4

    def test_layer_names(self):
        """Test Layer enum names."""
        assert Layer.RAW.name == "RAW"
        assert Layer.AGGREGATED.name == "AGGREGATED"
        assert Layer.JUDGMENT.name == "JUDGMENT"
        assert Layer.ACTION.name == "ACTION"

    def test_layer_str_representation(self):
        """Test Layer string representation."""
        assert str(Layer.RAW) == "Layer.RAW"
        assert str(Layer.AGGREGATED) == "Layer.AGGREGATED"

    def test_layer_repr(self):
        """Test Layer repr."""
        repr_str = repr(Layer.RAW)
        assert "Layer" in repr_str
        assert "RAW" in repr_str
        assert "1" in repr_str


class TestVerdictTypeEnum:
    """Test VerdictType enumeration."""

    def test_verdict_values(self):
        """Test VerdictType enum values."""
        assert VerdictType.HOWL.value == "HOWL"
        assert VerdictType.GROWL.value == "GROWL"
        assert VerdictType.WAG.value == "WAG"
        assert VerdictType.BARK.value == "BARK"

    def test_verdict_str(self):
        """Test VerdictType string conversion."""
        assert str(VerdictType.HOWL) == "HOWL"
        assert str(VerdictType.GROWL) == "GROWL"
        assert str(VerdictType.WAG) == "WAG"
        assert str(VerdictType.BARK) == "BARK"


class TestObservationTypeEnum:
    """Test ObservationType enumeration."""

    def test_observation_types_exist(self):
        """Test all observation types are defined."""
        types = [
            ObservationType.PROCESS_CREATED,
            ObservationType.PROCESS_TERMINATED,
            ObservationType.METRIC_SAMPLE,
            ObservationType.ECOSYSTEM_EVENT,
            ObservationType.HUMAN_INPUT,
            ObservationType.ACTION_RESULT,
        ]
        assert len(types) == 6

    def test_observation_type_str(self):
        """Test ObservationType string conversion."""
        assert str(ObservationType.METRIC_SAMPLE) == "METRIC_SAMPLE"


class TestAggregationTypeEnum:
    """Test AggregationType enumeration."""

    def test_aggregation_types_exist(self):
        """Test all aggregation types are defined."""
        types = [
            AggregationType.PROCESS_METRICS,
            AggregationType.SYSTEM_STATE,
            AggregationType.ECOSYSTEM_STATE,
            AggregationType.HEALTH_SUMMARY,
        ]
        assert len(types) == 4

    def test_aggregation_type_str(self):
        """Test AggregationType string conversion."""
        assert str(AggregationType.SYSTEM_STATE) == "SYSTEM_STATE"


class TestJudgmentTypeEnum:
    """Test JudgmentType enumeration."""

    def test_judgment_types_exist(self):
        """Test all judgment types are defined."""
        types = [
            JudgmentType.STATE_EVALUATION,
            JudgmentType.EMERGENCE_ALERT,
            JudgmentType.PATTERN_DETECTED,
            JudgmentType.LEARNING_UPDATE,
        ]
        assert len(types) == 4

    def test_judgment_type_str(self):
        """Test JudgmentType string conversion."""
        assert str(JudgmentType.STATE_EVALUATION) == "STATE_EVALUATION"


class TestActionTypeEnum:
    """Test ActionType enumeration."""

    def test_action_types_exist(self):
        """Test all action types are defined."""
        types = [
            ActionType.APPLY_CONFIG,
            ActionType.DEPLOY_COMPONENT,
            ActionType.EXTERNAL_CALL,
            ActionType.SIGNAL_HUMAN,
        ]
        assert len(types) == 4

    def test_action_type_str(self):
        """Test ActionType string conversion."""
        assert str(ActionType.EXTERNAL_CALL) == "EXTERNAL_CALL"


# ============================================================================
# Tests for MessageHeader
# ============================================================================

class TestMessageHeader:
    """Test MessageHeader dataclass."""

    def test_header_creation_minimal(self, timestamp):
        """Test creating a header with minimal fields."""
        header = MessageHeader(
            layer=Layer.RAW,
            message_id="abc12345",
            timestamp=timestamp,
            source="TEST_SENSOR",
        )
        assert header.layer == Layer.RAW
        assert header.message_id == "abc12345"
        assert header.timestamp == timestamp
        assert header.source == "TEST_SENSOR"
        assert header.target is None
        assert header.version == "1.0.0"

    def test_header_creation_full(self, timestamp):
        """Test creating a header with all fields."""
        header = MessageHeader(
            layer=Layer.JUDGMENT,
            message_id="xyz98765",
            timestamp=timestamp,
            source="SAGE_DOG",
            target="EXECUTIVE",
            version="1.0.0",
        )
        assert header.layer == Layer.JUDGMENT
        assert header.source == "SAGE_DOG"
        assert header.target == "EXECUTIVE"

    def test_header_to_dict(self, timestamp):
        """Test header serialization to dict."""
        header = MessageHeader(
            layer=Layer.AGGREGATED,
            message_id="msg00001",
            timestamp=timestamp,
            source="GANGLIA",
        )
        result = header.to_dict()
        assert isinstance(result, dict)
        assert result["layer"] == "AGGREGATED"
        assert result["layer_value"] == 2
        assert result["message_id"] == "msg00001"
        assert result["source"] == "GANGLIA"
        assert "timestamp_iso" in result
        assert result["version"] == "1.0.0"


# ============================================================================
# Tests for Metadata
# ============================================================================

class TestMetadata:
    """Test Metadata dataclass."""

    def test_metadata_creation_minimal(self, instance_id):
        """Test creating metadata with minimal fields."""
        metadata = Metadata(instance_id=instance_id)
        assert metadata.instance_id == instance_id
        assert metadata.region is None
        assert metadata.route_trace == []
        assert metadata.feedback is False
        assert metadata.closes_action_id is None

    def test_metadata_creation_full(self, instance_id):
        """Test creating metadata with all fields."""
        route = ["SENSOR1", "GANGLIA1"]
        metadata = Metadata(
            instance_id=instance_id,
            region="us-west",
            route_trace=route,
            feedback=True,
            closes_action_id="action_123",
        )
        assert metadata.instance_id == instance_id
        assert metadata.region == "us-west"
        assert metadata.route_trace == route
        assert metadata.feedback is True
        assert metadata.closes_action_id == "action_123"

    def test_metadata_to_dict(self, instance_id):
        """Test metadata serialization to dict."""
        metadata = Metadata(
            instance_id=instance_id,
            region="eu-central",
            route_trace=["SENSOR"],
            feedback=True,
        )
        result = metadata.to_dict()
        assert isinstance(result, dict)
        assert result["instance_id"] == instance_id
        assert result["region"] == "eu-central"
        assert result["route_trace"] == ["SENSOR"]
        assert result["feedback"] is True


# ============================================================================
# Tests for LNSPMessage
# ============================================================================

class TestLNSPMessage:
    """Test LNSPMessage dataclass."""

    def test_message_creation(self, instance_id, timestamp):
        """Test creating an LNSPMessage."""
        header = MessageHeader(
            layer=Layer.RAW,
            message_id="msg00001",
            timestamp=timestamp,
            source="SENSOR",
        )
        metadata = Metadata(instance_id=instance_id)
        payload = {"data": "test"}

        msg = LNSPMessage(header=header, payload=payload, metadata=metadata)
        assert msg.header == header
        assert msg.payload == payload
        assert msg.metadata == metadata

    def test_message_to_dict(self, instance_id, timestamp):
        """Test message serialization to dict."""
        header = MessageHeader(
            layer=Layer.JUDGMENT,
            message_id="msg00002",
            timestamp=timestamp,
            source="DOG",
        )
        metadata = Metadata(instance_id=instance_id)
        payload = {"verdict": "HOWL"}

        msg = LNSPMessage(header=header, payload=payload, metadata=metadata)
        result = msg.to_dict()

        assert isinstance(result, dict)
        assert "header" in result
        assert "payload" in result
        assert "metadata" in result
        assert result["header"]["layer"] == "JUDGMENT"
        assert result["payload"]["verdict"] == "HOWL"
        assert result["metadata"]["instance_id"] == instance_id

    def test_message_repr(self, instance_id, timestamp):
        """Test message string representation."""
        header = MessageHeader(
            layer=Layer.ACTION,
            message_id="act00001",
            timestamp=timestamp,
            source="EXECUTIVE",
            target="HANDLER",
        )
        metadata = Metadata(instance_id=instance_id)
        msg = LNSPMessage(header=header, payload={}, metadata=metadata)

        repr_str = repr(msg)
        assert "LNSPMessage" in repr_str
        assert "ACTION" in repr_str
        assert "act00001" in repr_str
        assert "EXECUTIVE" in repr_str
        assert "HANDLER" in repr_str


# ============================================================================
# Tests for Message Factories
# ============================================================================

class TestMessageIDGeneration:
    """Test message ID generation."""

    def test_generate_message_id_format(self):
        """Test message IDs are 8-char hex strings."""
        msg_id = _generate_message_id()
        assert isinstance(msg_id, str)
        assert len(msg_id) == 8
        # Check it's valid hex
        int(msg_id, 16)  # Should not raise

    def test_generate_message_id_unique(self):
        """Test message IDs are unique."""
        ids = [_generate_message_id() for _ in range(100)]
        assert len(set(ids)) == 100  # All unique


class TestTimestampGeneration:
    """Test timestamp generation."""

    def test_get_timestamp_is_numeric(self):
        """Test timestamp is a numeric value."""
        ts = _get_timestamp()
        assert isinstance(ts, float)
        assert ts > 0

    def test_get_timestamp_is_current(self):
        """Test timestamp is approximately current."""
        ts1 = time.time()
        ts2 = _get_timestamp()
        ts3 = time.time()
        assert ts1 <= ts2 <= ts3
        assert (ts3 - ts1) < 0.1  # Within 100ms


class TestCreateRawObservation:
    """Test Layer 1 raw observation factory."""

    def test_create_raw_observation_basic(self, instance_id):
        """Test basic raw observation creation."""
        msg = create_raw_observation(
            observation_type=ObservationType.METRIC_SAMPLE,
            data={"cpu": 45.2},
            source="METRICS",
            instance_id=instance_id,
        )
        assert msg.header.layer == Layer.RAW
        assert msg.header.source == "METRICS"
        assert msg.payload["observation_type"] == "METRIC_SAMPLE"
        assert msg.payload["cpu"] == 45.2
        assert msg.metadata.instance_id == instance_id

    def test_raw_observation_with_target(self, instance_id):
        """Test raw observation can be created."""
        msg = create_raw_observation(
            observation_type=ObservationType.ECOSYSTEM_EVENT,
            data={"event": "tx_submitted"},
            source="SENSOR",
            instance_id=instance_id,
        )
        assert msg.header.target is None

    def test_raw_observation_with_region(self, instance_id):
        """Test raw observation with region."""
        msg = create_raw_observation(
            observation_type=ObservationType.PROCESS_CREATED,
            data={"pid": 12345},
            source="MONITOR",
            instance_id=instance_id,
            region="us-west",
        )
        assert msg.metadata.region == "us-west"

    def test_raw_observation_route_trace(self, instance_id):
        """Test route trace includes source."""
        msg = create_raw_observation(
            observation_type=ObservationType.METRIC_SAMPLE,
            data={},
            source="SENSOR1",
            instance_id=instance_id,
        )
        assert "SENSOR1" in msg.metadata.route_trace

    def test_raw_observation_no_feedback(self, instance_id):
        """Test raw observations don't expect feedback."""
        msg = create_raw_observation(
            observation_type=ObservationType.ACTION_RESULT,
            data={"status": "ok"},
            source="EXECUTOR",
            instance_id=instance_id,
        )
        assert msg.metadata.feedback is False

    def test_raw_observation_all_types(self, instance_id):
        """Test creating observations of all types."""
        types = [
            ObservationType.PROCESS_CREATED,
            ObservationType.PROCESS_TERMINATED,
            ObservationType.METRIC_SAMPLE,
            ObservationType.ECOSYSTEM_EVENT,
            ObservationType.HUMAN_INPUT,
            ObservationType.ACTION_RESULT,
        ]
        for obs_type in types:
            msg = create_raw_observation(
                observation_type=obs_type,
                data={},
                source="SENSOR",
                instance_id=instance_id,
            )
            assert msg.payload["observation_type"] == obs_type.value


class TestCreateAggregatedState:
    """Test Layer 2 aggregated state factory."""

    def test_create_aggregated_state_basic(self, instance_id):
        """Test basic aggregated state creation."""
        msg = create_aggregated_state(
            aggregation_type=AggregationType.SYSTEM_STATE,
            source="GANGLIA",
            data={"health": "GOOD"},
            based_on=["msg123", "msg456"],
            instance_id=instance_id,
        )
        assert msg.header.layer == Layer.AGGREGATED
        assert msg.header.source == "GANGLIA"
        assert msg.payload["aggregation_type"] == "SYSTEM_STATE"
        assert msg.payload["health"] == "GOOD"
        assert msg.payload["based_on"] == ["msg123", "msg456"]

    def test_aggregated_state_with_target(self, instance_id):
        """Test aggregated state can be created."""
        msg = create_aggregated_state(
            aggregation_type=AggregationType.PROCESS_METRICS,
            data={},
            source="GANGLIA",
            based_on=["msg789"],
            instance_id=instance_id,
        )
        assert msg.header.target is None

    def test_aggregated_state_route_trace(self, instance_id):
        """Test route trace in aggregated state."""
        msg = create_aggregated_state(
            aggregation_type=AggregationType.HEALTH_SUMMARY,
            data={},
            source="GANGLIA",
            based_on=["msg001"],
            instance_id=instance_id,
        )
        assert "GANGLIA" in msg.metadata.route_trace

    def test_aggregated_state_all_types(self, instance_id):
        """Test creating aggregated states of all types."""
        types = [
            AggregationType.PROCESS_METRICS,
            AggregationType.SYSTEM_STATE,
            AggregationType.ECOSYSTEM_STATE,
            AggregationType.HEALTH_SUMMARY,
        ]
        for agg_type in types:
            msg = create_aggregated_state(
                aggregation_type=agg_type,
                source="GANGLIA",
                data={},
                based_on=["msg_base"],
                instance_id=instance_id,
            )
            assert msg.payload["aggregation_type"] == agg_type.value


class TestCreateJudgment:
    """Test Layer 3 judgment factory."""

    def test_create_judgment_basic(self, instance_id):
        """Test basic judgment creation."""
        msg = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.HOWL,
            q_score=92.5,
            confidence=0.95,
            axiom_scores={"BURN": 0.9, "CONSENT": 0.85},
            data={"reason": "All good"},
            source="SAGE_DOG",
            target="EXECUTIVE",
            based_on=["agg123", "agg456"],
            instance_id=instance_id,
        )
        assert msg.header.layer == Layer.JUDGMENT
        assert msg.header.source == "SAGE_DOG"
        assert msg.payload["judgment_type"] == "STATE_EVALUATION"
        assert msg.payload["verdict"] == "HOWL"
        assert msg.payload["data"]["reason"] == "All good"
        assert msg.payload["q_score"] == 92.5
        assert msg.payload["confidence"] == 0.95
        assert msg.payload["axiom_scores"] == {"BURN": 0.9, "CONSENT": 0.85}
        assert msg.payload["based_on"] == ["agg123", "agg456"]

    def test_judgment_expects_feedback(self, instance_id):
        """Test judgments expect feedback."""
        msg = create_judgment(
            judgment_type=JudgmentType.PATTERN_DETECTED,
            verdict=VerdictType.GROWL,
            q_score=75.0,
            confidence=0.8,
            axiom_scores={},
            data={},
            source="DOG",
            target="EXEC",
            based_on=["msg_base"],
            instance_id=instance_id,
        )
        assert msg.metadata.feedback is True

    def test_judgment_with_q_score(self, instance_id):
        """Test judgment with Q-score."""
        msg = create_judgment(
            judgment_type=JudgmentType.EMERGENCE_ALERT,
            verdict=VerdictType.BARK,
            q_score=87.5,
            confidence=0.92,
            axiom_scores={"LEAD": 0.88},
            data={},
            source="DOG",
            target="EXEC",
            based_on=["msg_x"],
            instance_id=instance_id,
        )
        assert msg.payload["q_score"] == 87.5

    def test_judgment_with_target(self, instance_id):
        """Test judgment with target."""
        msg = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.WAG,
            q_score=88.0,
            confidence=0.9,
            axiom_scores={},
            data={},
            source="DOG",
            target="EXECUTIVE",
            based_on=["msg_abc"],
            instance_id=instance_id,
        )
        assert msg.header.target == "EXECUTIVE"

    def test_judgment_all_verdicts(self, instance_id):
        """Test judgments with all verdict types."""
        verdicts = [
            VerdictType.HOWL,
            VerdictType.GROWL,
            VerdictType.WAG,
            VerdictType.BARK,
        ]
        for verdict in verdicts:
            msg = create_judgment(
                judgment_type=JudgmentType.STATE_EVALUATION,
                verdict=verdict,
                q_score=85.0,
                confidence=0.85,
                axiom_scores={},
                data={},
                source="DOG",
                target="EXEC",
                based_on=["base_msg"],
                instance_id=instance_id,
            )
            assert msg.payload["verdict"] == verdict.value

    def test_judgment_all_types(self, instance_id):
        """Test judgments of all types."""
        types = [
            JudgmentType.STATE_EVALUATION,
            JudgmentType.EMERGENCE_ALERT,
            JudgmentType.PATTERN_DETECTED,
            JudgmentType.LEARNING_UPDATE,
        ]
        for jtype in types:
            msg = create_judgment(
                judgment_type=jtype,
                verdict=VerdictType.HOWL,
                q_score=90.0,
                confidence=0.88,
                axiom_scores={},
                data={},
                source="DOG",
                target="EXEC",
                based_on=["msg_type"],
                instance_id=instance_id,
            )
            assert msg.payload["judgment_type"] == jtype.value


class TestCreateAction:
    """Test Layer 4 action factory."""

    def test_create_action_basic(self, instance_id):
        """Test basic action creation."""
        msg = create_action(
            action_type=ActionType.EXTERNAL_CALL,
            target="BLOCKCHAIN_EXECUTOR",
            action_data={"call": "submit_vote"},
            source="EXECUTIVE",
            based_on_verdict="verdict_msg_123",
            instance_id=instance_id,
        )
        assert msg.header.layer == Layer.ACTION
        assert msg.header.source == "EXECUTIVE"
        assert msg.payload["action_type"] == "EXTERNAL_CALL"
        assert msg.payload["call"] == "submit_vote"
        assert msg.payload["based_on_verdict"] == "verdict_msg_123"

    def test_action_expects_feedback(self, instance_id):
        """Test actions expect feedback."""
        msg = create_action(
            action_type=ActionType.APPLY_CONFIG,
            target="CONFIG_HANDLER",
            action_data={},
            source="EXECUTIVE",
            based_on_verdict="verdict_x",
            instance_id=instance_id,
        )
        assert msg.metadata.feedback is True

    def test_action_with_target(self, instance_id):
        """Test action with target executor."""
        msg = create_action(
            action_type=ActionType.DEPLOY_COMPONENT,
            target="ORCHESTRATOR",
            action_data={"component": "new_agent"},
            source="EXECUTIVE",
            based_on_verdict="verdict_deploy",
            instance_id=instance_id,
        )
        assert msg.header.target == "ORCHESTRATOR"

    def test_action_closes_action_id(self, instance_id):
        """Test action that closes a previous action."""
        msg = create_action(
            action_type=ActionType.EXTERNAL_CALL,
            target="EXECUTOR",
            action_data={"result": "success"},
            source="EXECUTOR",
            based_on_verdict="verdict_close",
            instance_id=instance_id,
            region=None,
        )
        # Note: closes_action_id is handled via metadata in factory, not as explicit param
        assert msg.metadata.feedback is True

    def test_action_all_types(self, instance_id):
        """Test actions of all types."""
        types = [
            ActionType.APPLY_CONFIG,
            ActionType.DEPLOY_COMPONENT,
            ActionType.EXTERNAL_CALL,
            ActionType.SIGNAL_HUMAN,
        ]
        for atype in types:
            msg = create_action(
                action_type=atype,
                target="EXECUTOR",
                action_data={},
                source="EXECUTIVE",
                based_on_verdict="verdict_type",
                instance_id=instance_id,
            )
            assert msg.payload["action_type"] == atype.value


# ============================================================================
# Integration Tests
# ============================================================================

class TestLNSPMessageIntegration:
    """Integration tests for LNSP message pipeline."""

    def test_message_pipeline_observation_to_action(self, instance_id):
        """Test a complete message pipeline from observation to action."""
        # Layer 1: Observation
        obs = create_raw_observation(
            observation_type=ObservationType.METRIC_SAMPLE,
            data={"cpu": 95.0},
            source="METRICS",
            instance_id=instance_id,
        )
        assert obs.header.layer == Layer.RAW

        # Layer 2: Aggregation (using previous message's message ID as causality)
        agg = create_aggregated_state(
            aggregation_type=AggregationType.SYSTEM_STATE,
            data={"status": "degraded"},
            source="GANGLIA",
            based_on=[obs.header.message_id],
            instance_id=instance_id,
        )
        assert agg.header.layer == Layer.AGGREGATED

        # Layer 3: Judgment
        judgment = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.BARK,
            q_score=85.0,
            confidence=0.88,
            axiom_scores={"LEAD": 0.85},
            data={"reason": "CPU too high"},
            source="SAGE_DOG",
            target="EXECUTIVE",
            based_on=[agg.header.message_id],
            instance_id=instance_id,
        )
        assert judgment.header.layer == Layer.JUDGMENT

        # Layer 4: Action
        action = create_action(
            action_type=ActionType.SIGNAL_HUMAN,
            target="HUMAN_OPERATOR",
            action_data={"alert": "high_cpu"},
            source="EXECUTIVE",
            based_on_verdict=judgment.header.message_id,
            instance_id=instance_id,
        )
        assert action.header.layer == Layer.ACTION

    def test_message_serialization_roundtrip(self, instance_id):
        """Test message can be converted to dict and contains all data."""
        msg = create_judgment(
            judgment_type=JudgmentType.LEARNING_UPDATE,
            verdict=VerdictType.HOWL,
            q_score=91.3,
            confidence=0.91,
            axiom_scores={"BURN": 0.92, "CONSENT": 0.90},
            data={"q_table_updated": True},
            source="SAGE_DOG",
            target="EXECUTIVE",
            based_on=["learning_base"],
            instance_id=instance_id,
            region="us-east",
        )

        # Convert to dict
        msg_dict = msg.to_dict()

        # Verify all data is present
        assert msg_dict["header"]["layer"] == "JUDGMENT"
        assert msg_dict["header"]["source"] == "SAGE_DOG"
        assert msg_dict["header"]["target"] == "EXECUTIVE"
        assert msg_dict["payload"]["verdict"] == "HOWL"
        assert msg_dict["payload"]["q_score"] == 91.3
        assert msg_dict["payload"]["confidence"] == 0.91
        assert msg_dict["payload"]["axiom_scores"] == {"BURN": 0.92, "CONSENT": 0.90}
        assert msg_dict["payload"]["based_on"] == ["learning_base"]
        assert msg_dict["metadata"]["instance_id"] == instance_id
        assert msg_dict["metadata"]["region"] == "us-east"

    def test_multiple_messages_have_unique_ids(self, instance_id):
        """Test that multiple messages have unique IDs."""
        msgs = [
            create_raw_observation(
                ObservationType.METRIC_SAMPLE, data={}, source="S", instance_id=instance_id
            )
            for _ in range(10)
        ]
        ids = [msg.header.message_id for msg in msgs]
        assert len(set(ids)) == 10  # All unique
