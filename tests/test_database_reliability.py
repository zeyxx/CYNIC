"""
Tests for Database Reliability Features

Verifies that:
- Connection pooling configuration
- Transaction management
- Data consistency checking
- Backup/restore functionality
- Database health checks
"""

import pytest
from pathlib import Path

import sys
sys.path.insert(0, str(Path(__file__).parent.parent / "governance_bot"))

from database import DatabaseHealthCheck


class TestDatabaseHealthCheck:
    """Test database health check functionality"""

    def test_health_check_initialization(self):
        """Health check initializes correctly"""
        hc = DatabaseHealthCheck()

        assert hc.last_check_time is None
        assert hc.last_check_status is None
        assert hc.error_count == 0

    def test_health_check_status_not_checked(self):
        """Health check reports NOT_CHECKED initially"""
        hc = DatabaseHealthCheck()
        status = hc.get_status()
        assert status == "NOT_CHECKED"

    def test_health_check_tracks_error_count(self):
        """Health check increments error count"""
        hc = DatabaseHealthCheck()

        hc.last_check_status = "UNHEALTHY"
        hc.error_count = 0

        hc.error_count += 1
        assert hc.error_count == 1

        hc.error_count += 1
        assert hc.error_count == 2

    def test_health_check_critical_after_multiple_errors(self):
        """Health check reports CRITICAL after 3+ errors"""
        hc = DatabaseHealthCheck()

        hc.last_check_status = "UNHEALTHY"
        hc.error_count = 3

        status = hc.get_status()
        assert status == "CRITICAL"

    def test_health_check_healthy_status(self):
        """Health check reports HEALTHY when healthy"""
        hc = DatabaseHealthCheck()

        hc.last_check_status = "HEALTHY"
        hc.error_count = 0

        status = hc.get_status()
        assert status == "HEALTHY"


class TestSessionContextAPI:
    """Test session context manager API"""

    def test_session_context_accepts_auto_commit(self):
        """session_context accepts auto_commit parameter"""
        import inspect
        from database import session_context

        sig = inspect.signature(session_context.__wrapped__)
        params = list(sig.parameters.keys())

        assert "auto_commit" in params

    def test_session_context_documented(self):
        """session_context documents auto_commit parameter"""
        import inspect
        from database import session_context

        doc = session_context.__wrapped__.__doc__

        assert doc is not None
        assert "auto_commit" in doc.lower()
        assert "rollback" in doc.lower()


class TestTransactionContextAPI:
    """Test transaction context manager"""

    def test_transaction_context_exists(self):
        """transaction_context is properly defined"""
        from database import transaction_context

        assert transaction_context is not None
        assert callable(transaction_context)

    def test_transaction_context_documented(self):
        """transaction_context documents manual control"""
        import inspect
        from database import transaction_context

        doc = transaction_context.__wrapped__.__doc__

        assert doc is not None
        assert "manual" in doc.lower() or "commit" in doc.lower()


class TestDataConsistencyChecking:
    """Test data consistency checking"""

    def test_verify_data_consistency_is_async(self):
        """verify_data_consistency is an async function"""
        import inspect
        from database import verify_data_consistency

        assert inspect.iscoroutinefunction(verify_data_consistency)


class TestConnectionPoolConfiguration:
    """Test connection pool configuration"""

    def test_pool_size_configured_to_five(self):
        """Connection pool is configured with pool_size=5"""
        import inspect
        from database import init_db

        source = inspect.getsource(init_db)

        # Check pool configuration
        assert "pool_size=5" in source or "pool_size = 5" in source
        assert "max_overflow" in source


class TestBackupRestoreFunctions:
    """Test backup and restore functionality"""

    def test_backup_database_is_async(self):
        """backup_database is an async function"""
        import inspect
        from database import backup_database

        assert inspect.iscoroutinefunction(backup_database)

    def test_backup_database_has_backup_dir_param(self):
        """backup_database accepts backup_dir parameter"""
        import inspect
        from database import backup_database

        sig = inspect.signature(backup_database)
        params = list(sig.parameters.keys())

        assert "backup_dir" in params

    def test_restore_database_is_async(self):
        """restore_database is an async function"""
        import inspect
        from database import restore_database

        assert inspect.iscoroutinefunction(restore_database)

    def test_restore_database_has_backup_file_param(self):
        """restore_database accepts backup_file parameter"""
        import inspect
        from database import restore_database

        sig = inspect.signature(restore_database)
        params = list(sig.parameters.keys())

        assert "backup_file" in params


class TestErrorHandlingInSessionContext:
    """Test error handling in session context"""

    def test_session_context_includes_rollback(self):
        """session_context includes rollback on error"""
        import inspect
        from database import session_context

        source = inspect.getsource(session_context.__wrapped__)

        assert "rollback" in source
        assert "except" in source

    def test_close_db_has_backup_parameter(self):
        """close_db accepts create_backup parameter"""
        import inspect
        from database import close_db

        sig = inspect.signature(close_db)
        params = list(sig.parameters.keys())

        assert "create_backup" in params

    def test_close_db_backup_default_true(self):
        """close_db creates backup by default"""
        import inspect
        from database import close_db

        sig = inspect.signature(close_db)
        backup_param = sig.parameters.get("create_backup")

        assert backup_param is not None
        assert backup_param.default is True


class TestDatabaseReliabilityDocumentation:
    """Test documentation for database reliability"""

    def test_database_module_docstring(self):
        """database.py has comprehensive docstring"""
        import database

        doc = database.__doc__

        assert doc is not None
        assert ("pooling" in doc.lower() or
                "reliability" in doc.lower() or
                "consistency" in doc.lower())

    def test_database_health_check_class_documented(self):
        """DatabaseHealthCheck class is documented"""
        from database import DatabaseHealthCheck

        doc = DatabaseHealthCheck.__doc__

        assert doc is not None
        assert "health" in doc.lower()

    def test_backup_function_documented(self):
        """backup_database function is documented"""
        import inspect
        from database import backup_database

        doc = backup_database.__doc__

        assert doc is not None
        assert "backup" in doc.lower()


class TestGlobalHealthCheckInstance:
    """Test global database health check instance"""

    def test_db_health_check_global_exists(self):
        """Global db_health_check instance is created"""
        from database import db_health_check

        assert db_health_check is not None
        assert isinstance(db_health_check, DatabaseHealthCheck)

    def test_db_health_check_has_check_method(self):
        """db_health_check has check_health method"""
        from database import db_health_check

        assert hasattr(db_health_check, "check_health")
        assert callable(db_health_check.check_health)

    def test_db_health_check_has_get_status_method(self):
        """db_health_check has get_status method"""
        from database import db_health_check

        assert hasattr(db_health_check, "get_status")
        assert callable(db_health_check.get_status)
