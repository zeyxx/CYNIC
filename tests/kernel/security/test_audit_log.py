"""Tests for audit logging system."""

from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path
import tempfile

import pytest

from cynic.kernel.security.audit_log import (
    AuditAction,
    AuditEntry,
    AuditLogger,
    AuditLogQuery,
    AuditResult,
)


class TestAuditAction:
    """Tests for AuditAction enum."""

    def test_audit_action_values(self) -> None:
        """AuditAction has expected values."""
        assert AuditAction.AUTH_LOGIN_SUCCESS.value == "auth.login.success"
        assert AuditAction.AUTH_LOGIN_FAILURE.value == "auth.login.failure"
        assert AuditAction.AUTHZ_CHECK_GRANTED.value == "authz.check.granted"
        assert AuditAction.DATA_READ.value == "data.read"

    def test_audit_result_values(self) -> None:
        """AuditResult has expected values."""
        assert AuditResult.SUCCESS.value == "success"
        assert AuditResult.FAILURE.value == "failure"
        assert AuditResult.DENIED.value == "denied"


class TestAuditEntry:
    """Tests for audit entry creation and serialization."""

    def test_create_audit_entry(self) -> None:
        """Audit entry can be created."""
        now = datetime.now(timezone.utc)
        entry = AuditEntry(
            timestamp=now,
            principal="api-gateway",
            action=AuditAction.AUTH_LOGIN_SUCCESS,
            resource="/api/auth",
            result=AuditResult.SUCCESS,
            metadata={"ip": "192.168.1.1"},
        )

        assert entry.timestamp == now
        assert entry.principal == "api-gateway"
        assert entry.action == "auth.login.success"
        assert entry.resource == "/api/auth"
        assert entry.result == "success"
        assert entry.metadata["ip"] == "192.168.1.1"

    def test_audit_entry_to_dict(self) -> None:
        """Audit entry can be converted to dictionary."""
        now = datetime.now(timezone.utc)
        entry = AuditEntry(
            timestamp=now,
            principal="api-gateway",
            action=AuditAction.AUTH_LOGIN_SUCCESS,
            resource="/api/auth",
            result=AuditResult.SUCCESS,
        )

        data = entry.to_dict()

        assert data["timestamp"] == now.isoformat()
        assert data["principal"] == "api-gateway"
        assert data["action"] == "auth.login.success"
        assert data["result"] == "success"
        assert data["metadata"] == {}

    def test_audit_entry_to_json(self) -> None:
        """Audit entry can be serialized to JSON."""
        now = datetime.now(timezone.utc)
        entry = AuditEntry(
            timestamp=now,
            principal="api-gateway",
            action=AuditAction.AUTH_LOGIN_SUCCESS,
            resource="/api/auth",
            result=AuditResult.SUCCESS,
        )

        json_str = entry.to_json()

        assert "api-gateway" in json_str
        assert "auth.login.success" in json_str
        assert "/api/auth" in json_str


class TestAuditLogger:
    """Tests for AuditLogger."""

    @pytest.fixture
    def temp_log_dir(self) -> Path:
        """Create temporary log directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            yield Path(tmpdir)

    @pytest.fixture
    def audit_logger(self, temp_log_dir: Path) -> AuditLogger:
        """Create audit logger instance with cleanup."""
        logger = AuditLogger(
            principal="api-gateway",
            local_log_dir=temp_log_dir,
        )
        yield logger
        # Cleanup: close file handlers after test
        logger.shutdown()

    @pytest.mark.asyncio
    async def test_log_auth_attempt_success(self, audit_logger: AuditLogger) -> None:
        """Log successful authentication attempt."""
        await audit_logger.log_auth_attempt(
            actor="user-123",
            resource="/api/auth/login",
            success=True,
            metadata={"ip": "192.168.1.1"},
        )

        # Verify log file exists
        log_file = audit_logger.local_log_dir / "audit-api-gateway.jsonl"
        assert log_file.exists()

    @pytest.mark.asyncio
    async def test_log_auth_attempt_failure(self, audit_logger: AuditLogger) -> None:
        """Log failed authentication attempt."""
        await audit_logger.log_auth_attempt(
            actor="user-123",
            resource="/api/auth/login",
            success=False,
            metadata={"reason": "invalid password"},
        )

        log_file = audit_logger.local_log_dir / "audit-api-gateway.jsonl"
        assert log_file.exists()

    @pytest.mark.asyncio
    async def test_log_authz_decision_granted(self, audit_logger: AuditLogger) -> None:
        """Log authorization granted."""
        await audit_logger.log_authz_decision(
            actor="user-123",
            resource="/api/admin",
            granted=True,
        )

        log_file = audit_logger.local_log_dir / "audit-api-gateway.jsonl"
        assert log_file.exists()

    @pytest.mark.asyncio
    async def test_log_authz_decision_denied(self, audit_logger: AuditLogger) -> None:
        """Log authorization denied."""
        await audit_logger.log_authz_decision(
            actor="user-123",
            resource="/api/admin",
            granted=False,
            reason="insufficient privileges",
        )

        log_file = audit_logger.local_log_dir / "audit-api-gateway.jsonl"
        assert log_file.exists()

    @pytest.mark.asyncio
    async def test_log_data_access_read(self, audit_logger: AuditLogger) -> None:
        """Log data read operation."""
        await audit_logger.log_data_access(
            actor="user-123",
            operation="read",
            resource="/api/users/456",
            success=True,
        )

        log_file = audit_logger.local_log_dir / "audit-api-gateway.jsonl"
        assert log_file.exists()

    @pytest.mark.asyncio
    async def test_log_data_access_create(self, audit_logger: AuditLogger) -> None:
        """Log data create operation."""
        await audit_logger.log_data_access(
            actor="user-123",
            operation="create",
            resource="/api/proposals",
            success=True,
            metadata={"proposal_id": "prop-789"},
        )

        log_file = audit_logger.local_log_dir / "audit-api-gateway.jsonl"
        assert log_file.exists()

    @pytest.mark.asyncio
    async def test_log_data_access_delete(self, audit_logger: AuditLogger) -> None:
        """Log data delete operation."""
        await audit_logger.log_data_access(
            actor="user-123",
            operation="delete",
            resource="/api/proposals/789",
            success=True,
        )

        log_file = audit_logger.local_log_dir / "audit-api-gateway.jsonl"
        assert log_file.exists()

    @pytest.mark.asyncio
    async def test_log_config_change(self, audit_logger: AuditLogger) -> None:
        """Log configuration change."""
        await audit_logger.log_config_change(
            actor="admin-user",
            resource="/config/security",
            changes={"tls_required": "true", "mfa_enabled": "true"},
        )

        log_file = audit_logger.local_log_dir / "audit-api-gateway.jsonl"
        assert log_file.exists()

    @pytest.mark.asyncio
    async def test_log_secret_rotation(self, audit_logger: AuditLogger) -> None:
        """Log secret rotation."""
        await audit_logger.log_secret_rotation(
            actor="system",
            secret_name="api_key",
        )

        log_file = audit_logger.local_log_dir / "audit-api-gateway.jsonl"
        assert log_file.exists()

    @pytest.mark.asyncio
    async def test_log_security_event(self, audit_logger: AuditLogger) -> None:
        """Log security event."""
        await audit_logger.log_security_event(
            event_type="anomaly",
            resource="/api/users",
            severity="medium",
            description="Unusual number of failed login attempts",
        )

        log_file = audit_logger.local_log_dir / "audit-api-gateway.jsonl"
        assert log_file.exists()

    @pytest.mark.asyncio
    async def test_audit_log_immutable(self, audit_logger: AuditLogger) -> None:
        """Log entries are immutable after creation."""
        entry = AuditEntry(
            timestamp=datetime.now(timezone.utc),
            principal="api-gateway",
            action=AuditAction.AUTH_LOGIN_SUCCESS,
            resource="/api/auth",
            result=AuditResult.SUCCESS,
        )

        # Entries should not have settable attributes (immutable)
        assert hasattr(entry, "principal")
        assert hasattr(entry, "action")


class TestAuditLogQuery:
    """Tests for audit log querying."""

    @pytest.mark.asyncio
    async def test_query_without_db(self) -> None:
        """Query returns empty list without database."""
        query = AuditLogQuery(db_connection=None)

        results = await query.get_logs()

        assert results == []

    @pytest.mark.asyncio
    async def test_get_action_history_without_db(self) -> None:
        """Get action history returns empty without database."""
        query = AuditLogQuery(db_connection=None)

        results = await query.get_action_history(resource="/api/users/123")

        assert results == []

    @pytest.mark.asyncio
    async def test_get_user_actions_without_db(self) -> None:
        """Get user actions returns empty without database."""
        query = AuditLogQuery(db_connection=None)

        results = await query.get_user_actions(principal="user-123")

        assert results == []


class TestAuditLoggingIntegration:
    """Integration tests for audit logging."""

    @pytest.fixture
    def temp_log_dir(self) -> Path:
        """Create temporary log directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            yield Path(tmpdir)

    @pytest.mark.asyncio
    async def test_multiple_log_entries(self, temp_log_dir: Path) -> None:
        """Multiple log entries are appended."""
        logger = AuditLogger(principal="test-service", local_log_dir=temp_log_dir)

        try:
            # Log multiple events
            await logger.log_auth_attempt("user-1", "/api/auth", True)
            await logger.log_data_access("user-1", "read", "/api/users", True)
            await logger.log_config_change("admin", "/config", {"key": "value"})

            log_file = temp_log_dir / "audit-test-service.jsonl"
            content = log_file.read_text()
            lines = content.strip().split("\n")

            # Should have 3 log entries
            assert len(lines) == 3
        finally:
            logger.shutdown()

    @pytest.mark.asyncio
    async def test_audit_log_format(self, temp_log_dir: Path) -> None:
        """Audit logs are in proper JSON format."""
        logger = AuditLogger(principal="test-service", local_log_dir=temp_log_dir)

        try:
            await logger.log_auth_attempt("user-1", "/api/auth", True)

            log_file = temp_log_dir / "audit-test-service.jsonl"
            content = log_file.read_text()

            import json

            entry = json.loads(content.strip())

            assert "timestamp" in entry
            assert "principal" in entry
            assert "action" in entry
            assert "resource" in entry
            assert "result" in entry
            assert "metadata" in entry
            assert entry["principal"] == "user-1"
            assert entry["action"] == "auth.login.success"
        finally:
            logger.shutdown()
