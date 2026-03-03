"""Audit logging for comprehensive security and compliance.

Records all significant actions for forensics, compliance, and debugging.

Architecture:
- Immutable append-only logs
- Records: timestamp, principal, action, resource, result, metadata
- Audit DB: PostgreSQL audit schema (separate from operational DB)
- Local fallback: rotating file logs if DB unavailable
- Retention policy: 66 weeks per GDPR/SOC2 requirements (Task 1.7)
- Real-time: logged immediately, not batched

Action categories:
- Authentication: login attempts, MFA, token generation
- Authorization: permission checks, access decisions
- Data access: read/write/delete operations
- Configuration: settings changes, secrets rotation
- Security: anomalies, policy violations, incidents
- Compliance: GDPR requests, audit findings

Integration points:
- API gateway: log all HTTP requests (via middleware)
- EventBus: log event emissions
- AuthN/AuthZ: log auth decisions
- Config layer: log all changes
- Secrets manager: log rotations
"""

from __future__ import annotations

import json
import logging
from datetime import datetime, timezone
from enum import Enum
from pathlib import Path
from typing import Any

logger = logging.getLogger(__name__)


class AuditAction(str, Enum):
    """Standard audit actions."""

    # Authentication
    AUTH_LOGIN_SUCCESS = "auth.login.success"
    AUTH_LOGIN_FAILURE = "auth.login.failure"
    AUTH_LOGOUT = "auth.logout"
    AUTH_MFA_SUCCESS = "auth.mfa.success"
    AUTH_MFA_FAILURE = "auth.mfa.failure"
    AUTH_TOKEN_CREATED = "auth.token.created"
    AUTH_TOKEN_REVOKED = "auth.token.revoked"

    # Authorization
    AUTHZ_CHECK_GRANTED = "authz.check.granted"
    AUTHZ_CHECK_DENIED = "authz.check.denied"
    AUTHZ_PERMISSION_GRANTED = "authz.permission.granted"
    AUTHZ_PERMISSION_REVOKED = "authz.permission.revoked"

    # Data operations
    DATA_READ = "data.read"
    DATA_CREATE = "data.create"
    DATA_UPDATE = "data.update"
    DATA_DELETE = "data.delete"
    DATA_EXPORT = "data.export"

    # Configuration
    CONFIG_CHANGE = "config.change"
    CONFIG_READ = "config.read"
    SECRET_ROTATED = "secret.rotated"
    SECRET_ACCESSED = "secret.accessed"

    # Security events
    SECURITY_ANOMALY = "security.anomaly"
    SECURITY_VIOLATION = "security.violation"
    SECURITY_INCIDENT = "security.incident"

    # Compliance
    COMPLIANCE_REQUEST = "compliance.request"
    COMPLIANCE_AUDIT = "compliance.audit"


class AuditResult(str, Enum):
    """Result of audited action."""

    SUCCESS = "success"
    FAILURE = "failure"
    DENIED = "denied"
    ERROR = "error"


class AuditEntry:
    """A single audit log entry (immutable)."""

    def __init__(
        self,
        timestamp: datetime,
        principal: str,
        action: AuditAction | str,
        resource: str,
        result: AuditResult | str,
        metadata: dict[str, Any] | None = None,
    ):
        """Create immutable audit entry.

        Args:
            timestamp: When the action occurred
            principal: Who performed the action (service ID, user ID, etc.)
            action: What action was performed (see AuditAction enum)
            resource: What resource was affected (e.g., "/api/users/123")
            result: How it ended (success/failure/denied/error)
            metadata: Additional context (error message, affected fields, etc.)
        """
        self.timestamp = timestamp
        self.principal = principal
        self.action = str(action.value if isinstance(action, Enum) else action)
        self.resource = resource
        self.result = str(result.value if isinstance(result, Enum) else result)
        self.metadata = metadata or {}

    def to_dict(self) -> dict[str, Any]:
        """Convert to dictionary for storage/transmission."""
        return {
            "timestamp": self.timestamp.isoformat(),
            "principal": self.principal,
            "action": self.action,
            "resource": self.resource,
            "result": self.result,
            "metadata": self.metadata,
        }

    def to_json(self) -> str:
        """Convert to JSON string."""
        return json.dumps(self.to_dict())


class AuditLogger:
    """Records immutable audit logs with optional database persistence."""

    def __init__(
        self,
        principal: str,
        db_connection: Any | None = None,
        local_log_dir: Path | str | None = None,
    ):
        """Initialize audit logger.

        Args:
            principal: Service/module name that's logging (e.g., "api-gateway", "auth-service")
            db_connection: Optional database connection for audit DB (PostgSQL)
            local_log_dir: Optional local directory for fallback file logging
        """
        self.principal = principal
        self.db_connection = db_connection
        self.local_log_dir = Path(local_log_dir) if local_log_dir else None
        self._local_logger: logging.Logger | None = None
        self._file_handler: logging.FileHandler | None = None

        # Setup local file logging if directory provided
        if self.local_log_dir:
            self.local_log_dir.mkdir(parents=True, exist_ok=True)
            self._setup_local_logging()

    def _setup_local_logging(self) -> None:
        """Setup local file-based logging as fallback."""
        log_file = self.local_log_dir / f"audit-{self.principal}.jsonl"

        # Create file handler
        handler = logging.FileHandler(log_file)
        handler.setFormatter(logging.Formatter("%(message)s"))

        self._local_logger = logging.getLogger(f"audit.{self.principal}")
        self._local_logger.setLevel(logging.INFO)
        self._local_logger.addHandler(handler)
        self._local_logger.propagate = False

        # Store handler for cleanup
        self._file_handler = handler

    def shutdown(self) -> None:
        """Close file handlers and cleanup."""
        if self._file_handler:
            self._file_handler.close()
            if self._local_logger:
                self._local_logger.removeHandler(self._file_handler)

    async def log(
        self,
        action: AuditAction | str,
        resource: str,
        result: AuditResult | str = AuditResult.SUCCESS,
        actor: str | None = None,
        metadata: dict[str, Any] | None = None,
    ) -> None:
        """Log an audit entry.

        Args:
            action: Action performed (see AuditAction enum)
            resource: Resource affected (e.g., "/api/proposals")
            result: Result of action (success/failure/denied/error)
            actor: Who performed the action (defaults to principal)
            metadata: Additional context
        """
        entry = AuditEntry(
            timestamp=datetime.now(timezone.utc),
            principal=actor or self.principal,
            action=action,
            resource=resource,
            result=result,
            metadata=metadata or {},
        )

        # Try to store in database
        if self.db_connection:
            await self._store_in_db(entry)
        else:
            logger.debug(
                f"No database connection, using local fallback for audit: {entry.to_json()}"
            )

        # Always log locally as well (immutable record)
        if self._local_logger:
            self._local_logger.info(entry.to_json())

    async def _store_in_db(self, entry: AuditEntry) -> None:
        """Store entry in audit database.

        Expected schema:
            CREATE TABLE audit_log (
                id BIGSERIAL PRIMARY KEY,
                timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
                principal VARCHAR(255) NOT NULL,
                action VARCHAR(255) NOT NULL,
                resource TEXT NOT NULL,
                result VARCHAR(32) NOT NULL,
                metadata JSONB DEFAULT '{}',
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            );
            CREATE INDEX idx_audit_timestamp ON audit_log(timestamp DESC);
            CREATE INDEX idx_audit_principal ON audit_log(principal);
            CREATE INDEX idx_audit_action ON audit_log(action);
        """
        try:
            # Using asyncpg or similar async database interface
            query = """
                INSERT INTO audit_log (timestamp, principal, action, resource, result, metadata)
                VALUES ($1, $2, $3, $4, $5, $6)
            """
            await self.db_connection.execute(
                query,
                entry.timestamp,
                entry.principal,
                entry.action,
                entry.resource,
                entry.result,
                json.dumps(entry.metadata),
            )
        except Exception as e:
            logger.error(
                f"Failed to store audit entry in database: {e}. "
                f"Falling back to local log."
            )
            # Fall back to local log
            if self._local_logger:
                self._local_logger.info(entry.to_json())

    async def log_auth_attempt(
        self,
        actor: str,
        resource: str,
        success: bool,
        metadata: dict[str, Any] | None = None,
    ) -> None:
        """Log authentication attempt."""
        action = (
            AuditAction.AUTH_LOGIN_SUCCESS
            if success
            else AuditAction.AUTH_LOGIN_FAILURE
        )
        result = AuditResult.SUCCESS if success else AuditResult.FAILURE
        await self.log(action, resource, result, actor, metadata or {})

    async def log_authz_decision(
        self,
        actor: str,
        resource: str,
        granted: bool,
        reason: str | None = None,
    ) -> None:
        """Log authorization decision."""
        action = (
            AuditAction.AUTHZ_CHECK_GRANTED
            if granted
            else AuditAction.AUTHZ_CHECK_DENIED
        )
        result = AuditResult.SUCCESS if granted else AuditResult.DENIED
        metadata = {"reason": reason} if reason else {}
        await self.log(action, resource, result, actor, metadata)

    async def log_data_access(
        self,
        actor: str,
        operation: str,  # read/create/update/delete
        resource: str,
        success: bool,
        metadata: dict[str, Any] | None = None,
    ) -> None:
        """Log data access operation."""
        action_map = {
            "read": AuditAction.DATA_READ,
            "create": AuditAction.DATA_CREATE,
            "update": AuditAction.DATA_UPDATE,
            "delete": AuditAction.DATA_DELETE,
        }
        action = action_map.get(operation, AuditAction.DATA_READ)
        result = AuditResult.SUCCESS if success else AuditResult.FAILURE
        await self.log(action, resource, result, actor, metadata or {})

    async def log_config_change(
        self,
        actor: str,
        resource: str,
        changes: dict[str, Any],
    ) -> None:
        """Log configuration change."""
        metadata = {"changes": changes}
        await self.log(
            AuditAction.CONFIG_CHANGE,
            resource,
            AuditResult.SUCCESS,
            actor,
            metadata,
        )

    async def log_secret_rotation(
        self,
        actor: str,
        secret_name: str,
    ) -> None:
        """Log secret rotation."""
        await self.log(
            AuditAction.SECRET_ROTATED,
            f"/secrets/{secret_name}",
            AuditResult.SUCCESS,
            actor,
            {"secret": secret_name},
        )

    async def log_security_event(
        self,
        event_type: str,  # anomaly/violation/incident
        resource: str,
        severity: str,  # low/medium/high/critical
        description: str,
    ) -> None:
        """Log security event."""
        action_map = {
            "anomaly": AuditAction.SECURITY_ANOMALY,
            "violation": AuditAction.SECURITY_VIOLATION,
            "incident": AuditAction.SECURITY_INCIDENT,
        }
        action = action_map.get(event_type, AuditAction.SECURITY_ANOMALY)
        metadata = {
            "severity": severity,
            "description": description,
        }
        await self.log(action, resource, AuditResult.ERROR, "system", metadata)


class AuditLogQuery:
    """Query and retrieve audit logs."""

    def __init__(self, db_connection: Any | None = None):
        """Initialize query interface.

        Args:
            db_connection: Database connection for querying audit logs
        """
        self.db_connection = db_connection

    async def get_logs(
        self,
        principal: str | None = None,
        action: str | None = None,
        resource: str | None = None,
        start_time: datetime | None = None,
        end_time: datetime | None = None,
        limit: int = 100,
    ) -> list[dict[str, Any]]:
        """Query audit logs with optional filters.

        Args:
            principal: Filter by who performed the action
            action: Filter by action type
            resource: Filter by affected resource
            start_time: Start of time range
            end_time: End of time range
            limit: Maximum results to return

        Returns:
            List of audit entries matching criteria
        """
        if not self.db_connection:
            return []

        conditions = []
        params = []
        param_idx = 1

        if principal:
            conditions.append(f"principal = ${param_idx}")
            params.append(principal)
            param_idx += 1

        if action:
            conditions.append(f"action = ${param_idx}")
            params.append(action)
            param_idx += 1

        if resource:
            conditions.append(f"resource LIKE ${param_idx}")
            params.append(f"%{resource}%")
            param_idx += 1

        if start_time:
            conditions.append(f"timestamp >= ${param_idx}")
            params.append(start_time)
            param_idx += 1

        if end_time:
            conditions.append(f"timestamp <= ${param_idx}")
            params.append(end_time)
            param_idx += 1

        where_clause = " AND ".join(conditions) if conditions else "TRUE"
        query = f"""
            SELECT timestamp, principal, action, resource, result, metadata
            FROM audit_log
            WHERE {where_clause}
            ORDER BY timestamp DESC
            LIMIT ${param_idx}
        """
        params.append(limit)

        try:
            return await self.db_connection.fetch(query, *params)
        except Exception as e:
            logger.error(f"Failed to query audit logs: {e}")
            return []

    async def get_action_history(
        self,
        resource: str,
        limit: int = 50,
    ) -> list[dict[str, Any]]:
        """Get all actions affecting a specific resource.

        Args:
            resource: Resource to query
            limit: Maximum results

        Returns:
            Ordered list of actions on resource
        """
        return await self.get_logs(resource=resource, limit=limit)

    async def get_user_actions(
        self,
        principal: str,
        limit: int = 50,
    ) -> list[dict[str, Any]]:
        """Get all actions performed by a principal.

        Args:
            principal: Principal (user/service) to query
            limit: Maximum results

        Returns:
            Ordered list of actions by principal
        """
        return await self.get_logs(principal=principal, limit=limit)
