import pytest
from cynic.api.error_handler import format_error_for_user, ErrorCategory


def test_database_error_friendly():
    """Database errors should be user-friendly"""
    error_msg = "asyncpg.errors.UndefinedTableError: table users not found"
    formatted = format_error_for_user(error_msg)

    assert "database" in formatted.lower()
    assert "try again" in formatted.lower() or "temporary" in formatted.lower()
    assert "UndefinedTableError" not in formatted  # No raw Python


def test_timeout_error_recoverable():
    """Timeout errors should suggest retry"""
    error_msg = "asyncio.TimeoutError: Ollama request timed out after 30s"
    formatted = format_error_for_user(error_msg)

    assert "long" in formatted.lower() or "timeout" in formatted.lower() or "busy" in formatted.lower()
    assert "retry" in formatted.lower() or "try again" in formatted.lower()


def test_validation_error_actionable():
    """Validation errors should tell user what to fix"""
    error_msg = "ValidationError: field 'score' must be 0-100, got 150"
    formatted = format_error_for_user(error_msg)

    assert "invalid" in formatted.lower() or "field" in formatted.lower()
    # Should include the range extracted from error message
    assert "0-100" in formatted or ("0" in formatted and "100" in formatted)


def test_auth_error_clear():
    """Auth errors should be clear and actionable"""
    error_msg = "AuthenticationError: JWT token expired"
    formatted = format_error_for_user(error_msg)

    assert "session" in formatted.lower() or "login" in formatted.lower()
    # Don't require re-login to be absent, just that it handles auth clearly


def test_unknown_error_safe():
    """Unknown errors should be safe"""
    error_msg = "SomeWeirdException: random internal error"
    formatted = format_error_for_user(error_msg)

    assert "unexpected" in formatted.lower()
    assert "SomeWeirdException" not in formatted  # No raw Python


def test_error_code_included():
    """Error should include a code for support reference"""
    error_msg = "TestError: something failed"
    formatted = format_error_for_user(error_msg)

    # Should have an error code
    assert "#" in formatted or "code" in formatted.lower()


def test_error_not_exposing_internals():
    """Error should not expose internal paths or secrets"""
    error_msg = "DBError: Connection to postgresql://user:password@db:5432"
    formatted = format_error_for_user(error_msg)

    assert "password" not in formatted.lower()
    assert "postgresql://" not in formatted


def test_error_suggests_contact_support():
    """Critical errors should suggest contacting support"""
    error_msg = "CriticalInternalError: something really broke"
    formatted = format_error_for_user(error_msg)

    assert "support" in formatted.lower() or "contact" in formatted.lower()
