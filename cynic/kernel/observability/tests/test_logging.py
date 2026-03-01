"""
Test suite for structured JSON logging system.

All logs must be valid JSON for machine parsing by monitoring systems.
No raw text output allowed.
"""
import json
import logging

from cynic.kernel.observability.structured_logger import StructuredLogger


def test_logs_are_valid_json():
    """All logs should be valid JSON"""
    logger = StructuredLogger("test")

    # Capture logs
    logs = []
    class MockHandler(logging.Handler):
        def emit(self, record):
            logs.append(self.format(record))

    handler = MockHandler()
    handler.setFormatter(logger.formatter)
    logger.logger.addHandler(handler)

    logger.info("test message")

    # Should be valid JSON
    assert len(logs) > 0
    parsed = json.loads(logs[0])
    assert parsed["level"] == "INFO"
    assert parsed["message"] == "test message"


def test_json_includes_timestamp():
    """All JSON logs should have timestamps"""
    logger = StructuredLogger("test")
    json_str = logger._format_json("INFO", "test", {})
    parsed = json.loads(json_str)

    assert "timestamp" in parsed
    assert "T" in parsed["timestamp"]  # ISO8601 format


def test_extra_fields_included():
    """Extra fields should be included in JSON"""
    logger = StructuredLogger("test")
    json_str = logger._format_json("INFO", "test", {"user_id": "123", "action": "query"})
    parsed = json.loads(json_str)

    assert parsed["user_id"] == "123"
    assert parsed["action"] == "query"


def test_error_level_logging():
    """Should support error level"""
    logger = StructuredLogger("test")
    json_str = logger._format_json("ERROR", "something failed", {"error_code": 500})
    parsed = json.loads(json_str)

    assert parsed["level"] == "ERROR"
    assert parsed["error_code"] == 500


def test_logger_name_included():
    """Logger name should be in output"""
    logger = StructuredLogger("myapp.module")
    json_str = logger._format_json("INFO", "test", {})
    parsed = json.loads(json_str)

    assert "logger" in parsed or "name" in parsed
    assert "myapp" in parsed.get("logger", parsed.get("name", ""))


def test_no_raw_text_output():
    """Logging should never output raw text (only JSON)"""
    logger = StructuredLogger("test")
    json_str = logger._format_json("INFO", "test", {})

    # Should be JSON, not text
    assert json_str.startswith("{")
    assert json.loads(json_str)  # Should parse without error
