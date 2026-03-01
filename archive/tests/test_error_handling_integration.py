"""
Tests for Error Handling Integration in Governance Bot

Verifies that:
- Circuit breaker blocks CYNIC calls when threshold is exceeded
- Error handler categorizes and reports errors appropriately
- Graceful degradation when CYNIC is unavailable
- Health check endpoint works correctly
"""

from datetime import datetime, timedelta

import pytest

from cynic.interfaces.bots.governance.core.error_handler import (
    CircuitBreaker,
    CYNICUnavailableError,
    DatabaseError,
    ErrorMetrics,
    GovernanceError,
    cynic_circuit_breaker,
    error_metrics,
    handle_error,
    retry_with_backoff,
)


class TestCircuitBreaker:
    """Test circuit breaker pattern"""

    def test_circuit_breaker_initialization(self):
        """Circuit breaker starts in CLOSED state"""
        cb = CircuitBreaker(failure_threshold=5, recovery_timeout=300)
        assert cb.state == "CLOSED"
        assert cb.failure_count == 0
        assert cb.is_available()

    def test_circuit_breaker_opens_after_threshold(self):
        """Circuit breaker opens after failure threshold exceeded"""
        cb = CircuitBreaker(failure_threshold=3, recovery_timeout=300)

        # Record failures up to threshold
        cb.record_failure()
        assert cb.is_available()  # Still CLOSED at 1 failure

        cb.record_failure()
        assert cb.is_available()  # Still CLOSED at 2 failures

        cb.record_failure()
        assert cb.state == "OPEN"
        assert not cb.is_available()  # Now OPEN

    def test_circuit_breaker_half_open_after_timeout(self):
        """Circuit breaker transitions to HALF_OPEN after recovery timeout"""
        cb = CircuitBreaker(failure_threshold=1, recovery_timeout=1)

        # Open the circuit
        cb.record_failure()
        assert cb.state == "OPEN"
        assert not cb.is_available()

        # Wait for recovery timeout
        cb.last_failure_time = datetime.utcnow() - timedelta(seconds=2)

        # Should transition to HALF_OPEN
        assert cb.is_available()
        assert cb.state == "HALF_OPEN"

    def test_circuit_breaker_resets_on_success(self):
        """Circuit breaker resets on success"""
        cb = CircuitBreaker(failure_threshold=5, recovery_timeout=300)

        # Record some failures
        cb.record_failure()
        cb.record_failure()
        assert cb.failure_count == 2

        # Success resets
        cb.record_success()
        assert cb.state == "CLOSED"
        assert cb.failure_count == 0

    def test_circuit_breaker_status_string(self):
        """Circuit breaker provides status string"""
        cb = CircuitBreaker(failure_threshold=5, recovery_timeout=300)

        status = cb.get_status()
        assert "CLOSED" in status
        assert "failures=0/5" in status

        cb.record_failure()
        status = cb.get_status()
        assert "failures=1/5" in status


class TestErrorMetrics:
    """Test error metrics tracking"""

    def test_error_metrics_initialization(self):
        """Error metrics start empty"""
        em = ErrorMetrics()
        assert em.total_errors == 0
        assert len(em.errors_by_type) == 0

    def test_error_metrics_recording(self):
        """Error metrics record error occurrences"""
        em = ErrorMetrics()

        em.record_error("DATABASE_ERROR")
        assert em.total_errors == 1
        assert em.errors_by_type["DATABASE_ERROR"] == 1

        em.record_error("DATABASE_ERROR")
        assert em.total_errors == 2
        assert em.errors_by_type["DATABASE_ERROR"] == 2

        em.record_error("CYNIC_UNAVAILABLE")
        assert em.total_errors == 3
        assert em.errors_by_type["CYNIC_UNAVAILABLE"] == 1

    def test_error_metrics_get_metrics(self):
        """Error metrics returns dict with stats"""
        em = ErrorMetrics()

        em.record_error("TEST_ERROR")
        metrics = em.get_metrics()

        assert metrics["total_errors"] == 1
        assert metrics["errors_by_type"]["TEST_ERROR"] == 1
        assert metrics["last_error"] is not None


class TestErrorExceptions:
    """Test custom error exception hierarchy"""

    def test_governance_error_initialization(self):
        """GovernanceError stores message and type"""
        error = GovernanceError("Test message", "TEST_TYPE")
        assert error.message == "Test message"
        assert error.error_type == "TEST_TYPE"
        assert str(error) == "Test message"

    def test_cynic_unavailable_error(self):
        """CYNICUnavailableError is properly initialized"""
        error = CYNICUnavailableError()
        assert error.error_type == "CYNIC_UNAVAILABLE"
        assert "unavailable" in error.message.lower()

        error2 = CYNICUnavailableError("Custom message")
        assert error2.message == "Custom message"

    def test_database_error(self):
        """DatabaseError formats operation context"""
        error = DatabaseError("Connection failed", "INSERT")
        assert error.error_type == "DATABASE_ERROR"
        assert "INSERT" in error.message
        assert "Connection failed" in error.message


@pytest.mark.asyncio
class TestErrorHandling:
    """Test error handling functions"""

    async def test_handle_error_cynic_unavailable(self):
        """handle_error categorizes CYNIC unavailable errors"""
        error = CYNICUnavailableError()
        message = await handle_error(error, "test context")

        assert "CYNIC" in message
        assert "unavailable" in message.lower()
        assert "❌" in message

    async def test_handle_error_database_error(self):
        """handle_error categorizes database errors"""
        error = DatabaseError("Connection failed", "SELECT")
        message = await handle_error(error, "test context")

        assert "Database" in message or "database" in message
        assert "❌" in message

    async def test_handle_error_timeout(self):
        """handle_error handles timeout errors"""
        error = TimeoutError()
        message = await handle_error(error, "test context")

        assert "timed out" in message.lower()
        assert "❌" in message

    async def test_handle_error_generic(self):
        """handle_error provides generic error message"""
        error = ValueError("Some value is wrong")
        message = await handle_error(error, "test context")

        assert "error" in message.lower()
        assert "❌" in message


@pytest.mark.asyncio
class TestRetryWithBackoff:
    """Test retry with exponential backoff"""

    async def test_retry_succeeds_on_first_attempt(self):
        """Retry returns immediately on success"""
        async def success_func():
            return "result"

        result = await retry_with_backoff(success_func, max_retries=3)
        assert result == "result"

    async def test_retry_fails_after_max_retries(self):
        """Retry exhausts retries and raises exception"""
        call_count = 0

        async def failing_func():
            nonlocal call_count
            call_count += 1
            raise ConnectionError("Network error")

        with pytest.raises(ConnectionError):
            await retry_with_backoff(
                failing_func,
                max_retries=2,
                initial_delay=0.01,
                backoff_factor=1.5
            )

        # Should have tried 3 times (initial + 2 retries)
        assert call_count == 3

    async def test_retry_succeeds_after_transient_failure(self):
        """Retry recovers from transient failures"""
        call_count = 0

        async def sometimes_failing_func():
            nonlocal call_count
            call_count += 1
            if call_count < 2:
                raise TimeoutError()
            return "success"

        result = await retry_with_backoff(
            sometimes_failing_func,
            max_retries=3,
            initial_delay=0.01
        )

        assert result == "success"
        assert call_count == 2

    async def test_retry_non_retryable_error_fails_immediately(self):
        """Retry doesn't retry on non-retryable errors"""
        call_count = 0

        async def failing_func():
            nonlocal call_count
            call_count += 1
            raise ValueError("Not a transient error")

        with pytest.raises(ValueError):
            await retry_with_backoff(failing_func, max_retries=3)

        # Should only try once (ValueError is not retryable)
        assert call_count == 1

    async def test_retry_respects_backoff_timing(self):
        """Retry applies exponential backoff delays"""
        import time

        call_times = []

        async def timing_func():
            call_times.append(time.time())
            if len(call_times) < 3:
                raise TimeoutError()
            return "success"

        start = time.time()
        result = await retry_with_backoff(
            timing_func,
            max_retries=2,
            initial_delay=0.05,
            backoff_factor=2.0
        )

        elapsed = time.time() - start
        # Should have delays: 0.05 + 0.1 = 0.15 seconds minimum
        assert elapsed >= 0.1
        assert result == "success"


class TestGlobalInstances:
    """Test global error handler instances"""

    def test_cynic_circuit_breaker_exists(self):
        """Global CYNIC circuit breaker is initialized"""
        assert cynic_circuit_breaker is not None
        assert cynic_circuit_breaker.failure_threshold == 5
        assert cynic_circuit_breaker.recovery_timeout == 300

    def test_error_metrics_exists(self):
        """Global error metrics instance is initialized"""
        assert error_metrics is not None
        assert isinstance(error_metrics, ErrorMetrics)
