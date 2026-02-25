"""Integration tests for GASdf with CYNIC governance."""
from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from cynic.integrations.gasdf.executor import GASdfExecutor
from cynic.integrations.gasdf.types import GASdfQuote
from cynic.protocol.lnsp.governance_integration import GovernanceLNSP
from cynic.protocol.lnsp.types import LNSPMessage, Layer, MessageHeader, Metadata


class TestGASdfGovernanceIntegration:
    """Test suite for GASdf integration with CYNIC governance."""

    @pytest.fixture
    def mock_manager(self) -> AsyncMock:
        """Create a mock LNSPManager."""
        manager = AsyncMock()
        manager.layer1 = MagicMock()
        manager.layer4 = MagicMock()
        return manager

    @pytest.fixture
    def mock_gasdf_executor(self) -> AsyncMock:
        """Create a mock GASdfExecutor."""
        return AsyncMock(spec=GASdfExecutor)

    def test_governance_lnsp_initialization_with_executor(
        self, mock_manager: AsyncMock, mock_gasdf_executor: AsyncMock
    ) -> None:
        """Test that GovernanceLNSP initializes with optional GASdfExecutor."""
        governance = GovernanceLNSP(
            manager=mock_manager,
            gasdf_executor=mock_gasdf_executor,
        )

        assert governance.manager is mock_manager
        assert governance.gasdf_executor is mock_gasdf_executor

    def test_governance_lnsp_initialization_without_executor(
        self, mock_manager: AsyncMock
    ) -> None:
        """Test that GovernanceLNSP works without GASdfExecutor."""
        governance = GovernanceLNSP(manager=mock_manager)

        assert governance.manager is mock_manager
        assert governance.gasdf_executor is None

    async def test_governance_with_gasdf_executor(
        self, mock_manager: AsyncMock, mock_gasdf_executor: AsyncMock
    ) -> None:
        """Test full governance flow with GASdfExecutor mock."""
        mock_gasdf_executor.execute_verdict.return_value = None

        governance = GovernanceLNSP(
            manager=mock_manager,
            gasdf_executor=mock_gasdf_executor,
        )

        # Mock the layer4.on_feedback to immediately call the callback
        feedback_callback = None

        def capture_on_feedback(cb):
            nonlocal feedback_callback
            feedback_callback = cb

        mock_manager.layer4.on_feedback.side_effect = capture_on_feedback

        # Setup
        await governance.setup()

        # Verify that on_feedback was called
        mock_manager.layer4.on_feedback.assert_called_once()

        # Create a mock verdict message
        header = MessageHeader(
            layer=Layer.JUDGMENT,
            message_id="msg_123",
            timestamp=1234567890.0,
            source="handler:governance",
        )
        metadata = Metadata(
            instance_id="instance:local",
            region=None,
            route_trace=["handler:governance"],
            feedback=False,
        )
        payload = {
            "observation_type": "JUDGMENT_EMITTED",
            "data": {
                "proposal_id": "prop_123",
                "verdict_type": "BARK",
                "verdict": "APPROVED",
                "q_score": 0.85,
                "community_id": "com_1",
                "payment_token": "usdc",
                "user_pubkey": "user_pub",
                "signed_transaction": "tx_data",
                "payment_token_account": "token_acc",
            },
        }
        verdict_msg = LNSPMessage(
            header=header,
            payload=payload,
            metadata=metadata,
        )

        # Simulate verdict emission through callback
        if feedback_callback:
            feedback_callback(verdict_msg)

        # Allow async task to complete
        import asyncio
        await asyncio.sleep(0.1)

        # Verify verdict was cached
        assert "prop_123" in governance.verdict_cache
        assert governance.verdict_cache["prop_123"]["verdict_type"] == "BARK"

    async def test_verdict_execution_called_for_approved(
        self, mock_manager: AsyncMock, mock_gasdf_executor: AsyncMock
    ) -> None:
        """Test that executor is called for APPROVED verdicts."""
        from cynic.integrations.gasdf.types import GASdfExecutionResult

        # Mock execution result
        exec_result = GASdfExecutionResult(
            signature="sig_123",
            status="confirmed",
            fee_amount=5000,
            fee_token="usdc",
            quote_id="q_123",
        )
        mock_gasdf_executor.execute_verdict.return_value = exec_result

        governance = GovernanceLNSP(
            manager=mock_manager,
            gasdf_executor=mock_gasdf_executor,
        )

        feedback_callback = None

        def capture_on_feedback(cb):
            nonlocal feedback_callback
            feedback_callback = cb

        mock_manager.layer4.on_feedback.side_effect = capture_on_feedback
        await governance.setup()

        # Create verdict message with APPROVED verdict
        header = MessageHeader(
            layer=Layer.JUDGMENT,
            message_id="msg_456",
            timestamp=1234567890.0,
            source="handler:governance",
        )
        metadata = Metadata(
            instance_id="instance:local",
            region=None,
            route_trace=["handler:governance"],
            feedback=False,
        )
        payload = {
            "observation_type": "JUDGMENT_EMITTED",
            "data": {
                "proposal_id": "prop_456",
                "verdict": "APPROVED",
                "verdict_type": "BARK",
                "q_score": 0.9,
                "community_id": "com_1",
                "payment_token": "usdc",
                "user_pubkey": "user_pub",
                "signed_transaction": "tx_data",
                "payment_token_account": "token_acc",
            },
        }
        verdict_msg = LNSPMessage(
            header=header,
            payload=payload,
            metadata=metadata,
        )

        # Trigger callback
        if feedback_callback:
            feedback_callback(verdict_msg)

        # Allow async execution
        import asyncio
        await asyncio.sleep(0.1)

        # Verify executor was called with correct parameters
        mock_gasdf_executor.execute_verdict.assert_called_once()
        call_args = mock_gasdf_executor.execute_verdict.call_args
        assert call_args[1]["proposal_id"] == "prop_456"
        assert call_args[1]["verdict"] == "APPROVED"

    def test_verdict_cache_structure(
        self, mock_manager: AsyncMock
    ) -> None:
        """Test that verdict cache has correct structure."""
        governance = GovernanceLNSP(manager=mock_manager)

        # Manually add to cache
        governance.verdict_cache["prop_test"] = {
            "verdict_type": "BARK",
            "q_score": 0.85,
            "timestamp": 1234567890.0,
        }

        assert "prop_test" in governance.verdict_cache
        assert governance.verdict_cache["prop_test"]["q_score"] == 0.85
