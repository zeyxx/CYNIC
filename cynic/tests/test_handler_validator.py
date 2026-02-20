"""
Handler Validator Tests — Compile-time discovery and validation.

Tests that HandlerValidator detects:
1. Duplicate handler names
2. Orphan handlers (modules exist but not discovered)
3. Invalid subscriptions (non-CoreEvent types)
4. Invalid dependencies (not frozenset)
5. Duplicate subscriptions (same event, same handler, multiple groups)
"""

from __future__ import annotations

import pytest
from unittest.mock import MagicMock

from cynic.core.event_bus import CoreEvent
from cynic.api.handlers.base import HandlerGroup
from cynic.api.handlers.validator import ValidationIssue, HandlerValidator


# ── Fixtures ──────────────────────────────────────────────────────────────────

@pytest.fixture
def mock_handler_valid():
    """Valid handler with all correct properties."""
    handler = MagicMock(spec=HandlerGroup)
    handler.name = "valid_handler"
    handler.subscriptions.return_value = [
        (CoreEvent.JUDGMENT_CREATED, MagicMock()),
        (CoreEvent.EMERGENCE_DETECTED, MagicMock()),
    ]
    handler.dependencies.return_value = frozenset({"orchestrator", "qtable"})
    return handler


@pytest.fixture
def mock_handler_duplicate_name():
    """Handler with duplicate name."""
    handler = MagicMock(spec=HandlerGroup)
    handler.name = "valid_handler"  # Same as valid_handler
    handler.subscriptions.return_value = [(CoreEvent.JUDGMENT_CREATED, MagicMock())]
    handler.dependencies.return_value = frozenset()
    return handler


@pytest.fixture
def mock_handler_bad_subscriptions():
    """Handler with non-CoreEvent subscription."""
    handler = MagicMock(spec=HandlerGroup)
    handler.name = "bad_subs_handler"
    # Return subscriptions but with invalid event
    handler.subscriptions.return_value = [
        (MagicMock(value="FAKE_EVENT"), MagicMock()),
    ]
    handler.dependencies.return_value = frozenset()
    return handler


@pytest.fixture
def mock_handler_bad_dependencies():
    """Handler with invalid dependencies type."""
    handler = MagicMock(spec=HandlerGroup)
    handler.name = "bad_deps_handler"
    handler.subscriptions.return_value = [(CoreEvent.JUDGMENT_CREATED, MagicMock())]
    handler.dependencies.return_value = {"orchestrator", "qtable"}  # Set, not frozenset
    return handler


@pytest.fixture
def mock_handler_exception_subscriptions():
    """Handler that raises exception in subscriptions()."""
    handler = MagicMock(spec=HandlerGroup)
    handler.name = "exception_handler"
    handler.subscriptions.side_effect = RuntimeError("Subscription error")
    handler.dependencies.return_value = frozenset()
    return handler


@pytest.fixture
def validator():
    """Fresh HandlerValidator instance."""
    return HandlerValidator()


# ── Tests ─────────────────────────────────────────────────────────────────────

class TestValidatorBasics:
    """Basic validator functionality."""

    def test_no_errors_with_valid_handlers(self, validator, mock_handler_valid):
        """Validation has no ERROR issues with single valid handler."""
        issues = validator.validate([mock_handler_valid])
        # May have WARNING issues (orphan modules) but no ERRORS
        error_issues = [i for i in issues if i.severity == "ERROR"]
        assert len(error_issues) == 0

    def test_has_errors_when_errors_present(self, validator, mock_handler_valid, mock_handler_duplicate_name):
        """has_errors() returns True when ERROR issues present."""
        validator.validate([mock_handler_valid, mock_handler_duplicate_name])
        assert validator.has_errors() is True

    def test_has_errors_false_for_warnings_only(self, validator, mock_handler_bad_subscriptions):
        """has_errors() returns False when only WARNING issues."""
        validator.validate([mock_handler_bad_subscriptions])
        # Subscription to non-CoreEvent is WARNING, not ERROR
        assert validator.has_errors() is False

    def test_summary_includes_all_fields(self, validator, mock_handler_valid):
        """summary() returns dict with all required fields."""
        validator.validate([mock_handler_valid])
        summary = validator.summary()

        assert "total_issues" in summary
        assert "errors" in summary
        assert "warnings" in summary
        assert "infos" in summary
        assert "discovered_handlers" in summary
        assert "orphan_modules" in summary
        assert "issues" in summary


class TestDuplicateDetection:
    """Detect duplicate handler names."""

    def test_detect_duplicate_names(self, validator, mock_handler_valid, mock_handler_duplicate_name):
        """Detect when two handlers have the same name."""
        issues = validator.validate([mock_handler_valid, mock_handler_duplicate_name])

        duplicate_issues = [i for i in issues if i.category == "DUPLICATE"]
        assert len(duplicate_issues) > 0
        assert duplicate_issues[0].severity == "ERROR"

    def test_duplicate_issue_details(self, validator, mock_handler_valid, mock_handler_duplicate_name):
        """Duplicate issue has correct details."""
        issues = validator.validate([mock_handler_valid, mock_handler_duplicate_name])
        duplicate_issues = [i for i in issues if i.category == "DUPLICATE" and i.severity == "ERROR"]

        if duplicate_issues:
            issue = duplicate_issues[0]
            assert "valid_handler" in issue.message or issue.handler_name == "valid_handler"
            assert "multiple" in issue.message.lower()


class TestInvalidSubscriptions:
    """Detect invalid event subscriptions."""

    def test_invalid_subscription_detection(self, validator, mock_handler_bad_subscriptions):
        """Detect subscription to non-CoreEvent."""
        issues = validator.validate([mock_handler_bad_subscriptions])

        invalid_issues = [i for i in issues if i.category == "INVALID"]
        assert len(invalid_issues) > 0

    def test_subscription_exception_handling(self, validator, mock_handler_exception_subscriptions):
        """Handle exception in subscriptions() gracefully."""
        issues = validator.validate([mock_handler_exception_subscriptions])

        error_issues = [i for i in issues if i.severity == "ERROR"]
        assert len(error_issues) > 0
        assert any("subscriptions" in i.message.lower() for i in error_issues)


class TestInvalidDependencies:
    """Detect invalid dependency declarations."""

    def test_invalid_dependency_type(self, validator, mock_handler_bad_dependencies):
        """Detect dependencies() returning non-frozenset."""
        issues = validator.validate([mock_handler_bad_dependencies])

        invalid_issues = [i for i in issues if i.category == "INVALID"]
        assert len(invalid_issues) > 0
        assert invalid_issues[0].severity == "WARNING"


class TestOrphanDetection:
    """Detect orphan handlers (modules exist but not discovered)."""

    def test_orphan_detection_logic(self, validator):
        """Orphan detection identifies modules not in discovered set."""
        # Create minimal handler
        handler = MagicMock(spec=HandlerGroup)
        handler.name = "minimal"
        handler.subscriptions.return_value = []
        handler.dependencies.return_value = frozenset()

        # After validation, orphan_modules should be populated
        issues = validator.validate([handler])
        summary = validator.summary()

        # orphan_modules will depend on what actually exists in handlers/
        # but the structure should be present
        assert isinstance(summary["orphan_modules"], list)


class TestReporting:
    """Test reporting and output formatting."""

    def test_report_no_errors(self, validator, mock_handler_valid):
        """Report output for valid handlers (no ERROR issues)."""
        issues = validator.validate([mock_handler_valid])
        report = validator.report()

        # If only warnings/infos, report should be readable
        error_issues = [i for i in issues if i.severity == "ERROR"]
        if not error_issues:
            # No errors = OK
            assert "(" in report  # Has issue counts
        # Report is always present
        assert len(report) > 0

    def test_report_with_errors(self, validator, mock_handler_valid, mock_handler_duplicate_name):
        """Report output includes error section."""
        validator.validate([mock_handler_valid, mock_handler_duplicate_name])
        report = validator.report()

        assert "ERROR" in report or "🔴" in report
        assert "issues" in report.lower()

    def test_report_format_readability(self, validator, mock_handler_bad_subscriptions):
        """Report is human-readable."""
        validator.validate([mock_handler_bad_subscriptions])
        report = validator.report()

        # Should have line breaks
        assert "\n" in report
        # Should have structured sections
        assert "[" in report and "]" in report


class TestValidationCombinations:
    """Test combinations of validation issues."""

    def test_multiple_issue_types(self, validator):
        """Handler with multiple problems generates multiple issues."""
        handler = MagicMock(spec=HandlerGroup)
        handler.name = "multi_issue"
        # Bad subscriptions AND bad dependencies
        handler.subscriptions.return_value = [(MagicMock(value="INVALID"), MagicMock())]
        handler.dependencies.return_value = {"not", "a", "frozenset"}

        issues = validator.validate([handler])

        # Should have at least 2 issues (one for each problem)
        assert len(issues) >= 2

    def test_many_valid_handlers(self, validator):
        """Validation with many valid handlers."""
        handlers = []
        for i in range(10):
            handler = MagicMock(spec=HandlerGroup)
            handler.name = f"handler_{i}"
            handler.subscriptions.return_value = [(CoreEvent.JUDGMENT_CREATED, MagicMock())]
            handler.dependencies.return_value = frozenset()
            handlers.append(handler)

        issues = validator.validate(handlers)

        # Should have no ERROR issues (all handlers valid)
        # May have WARNING issues (orphan modules) but no ERRORS
        error_issues = [i for i in issues if i.severity == "ERROR"]
        assert len(error_issues) == 0
        assert validator.has_errors() is False


class TestDiscoveredHandlerTracking:
    """Test tracking of discovered handlers."""

    def test_discovered_handlers_list(self, validator):
        """Summary includes list of discovered handlers."""
        handler1 = MagicMock(spec=HandlerGroup)
        handler1.name = "handler_one"
        handler1.subscriptions.return_value = []
        handler1.dependencies.return_value = frozenset()

        handler2 = MagicMock(spec=HandlerGroup)
        handler2.name = "handler_two"
        handler2.subscriptions.return_value = []
        handler2.dependencies.return_value = frozenset()

        validator.validate([handler1, handler2])
        summary = validator.summary()

        discovered = summary["discovered_handlers"]
        assert "handler_one" in discovered
        assert "handler_two" in discovered


class TestValidationIssueDataclass:
    """Test ValidationIssue dataclass."""

    def test_validation_issue_creation(self):
        """Create and verify ValidationIssue."""
        issue = ValidationIssue(
            severity="ERROR",
            category="DUPLICATE",
            handler_name="test_handler",
            message="Test message",
        )

        assert issue.severity == "ERROR"
        assert issue.category == "DUPLICATE"
        assert issue.handler_name == "test_handler"
        assert issue.message == "Test message"

    def test_validation_issue_in_report(self, validator):
        """ValidationIssue appears in summary."""
        handler = MagicMock(spec=HandlerGroup)
        handler.name = "test"
        handler.subscriptions.side_effect = RuntimeError("Test error")
        handler.dependencies.return_value = frozenset()

        validator.validate([handler])
        summary = validator.summary()

        issues_dicts = summary["issues"]
        assert len(issues_dicts) > 0
        # Each issue should be a dict with required fields
        assert all("severity" in i and "category" in i for i in issues_dicts)
