"""
Integration tests for JSON logging in FastAPI middleware.

Verifies that HTTP requests are logged as JSON with all required fields.
"""
import json
import logging
from io import StringIO
import pytest
from fastapi import FastAPI
from starlette.testclient import TestClient
from cynic.observability.structured_logger import StructuredLogger


@pytest.fixture
def app_with_json_logging():
    """Create a simple FastAPI app with JSON logging middleware"""
    app = FastAPI()

    # Use StructuredLogger for the app logger
    json_logger = StructuredLogger("test.api")
    app_logger = json_logger.logger

    # Capture logs
    log_stream = StringIO()
    handler = logging.StreamHandler(log_stream)
    handler.setFormatter(json_logger.formatter)
    app_logger.addHandler(handler)
    app_logger.setLevel(logging.DEBUG)

    @app.middleware("http")
    async def log_requests(request, call_next):
        """Log requests as JSON"""
        app_logger.info(
            f"HTTP {request.method} {request.url.path}",
            extra={"method": request.method, "path": request.url.path}
        )
        response = await call_next(request)
        app_logger.info(
            f"Response {response.status_code}",
            extra={"status_code": response.status_code}
        )
        return response

    @app.get("/test")
    async def test_endpoint():
        return {"status": "ok"}

    return app, log_stream


def test_http_requests_logged_as_json(app_with_json_logging):
    """Verify HTTP requests are logged as JSON"""
    app, log_stream = app_with_json_logging
    client = TestClient(app)

    response = client.get("/test")
    assert response.status_code == 200

    # Get logs
    logs_text = log_stream.getvalue()
    log_lines = [line.strip() for line in logs_text.split('\n') if line.strip()]

    # Verify all logs are valid JSON
    parsed_logs = []
    for line in log_lines:
        parsed = json.loads(line)
        parsed_logs.append(parsed)

    # Should have at least request + response logs
    assert len(parsed_logs) >= 2


def test_json_logs_include_method_and_path(app_with_json_logging):
    """Verify JSON logs include method and path"""
    app, log_stream = app_with_json_logging
    client = TestClient(app)

    response = client.get("/test")
    assert response.status_code == 200

    logs_text = log_stream.getvalue()
    log_lines = [line.strip() for line in logs_text.split('\n') if line.strip()]

    # Find request log
    request_log = None
    for line in log_lines:
        parsed = json.loads(line)
        if "method" in parsed and parsed.get("method") == "GET":
            request_log = parsed
            break

    assert request_log is not None
    assert request_log["method"] == "GET"
    assert request_log["path"] == "/test"


def test_json_logs_include_status_code(app_with_json_logging):
    """Verify JSON logs include HTTP status codes"""
    app, log_stream = app_with_json_logging
    client = TestClient(app)

    response = client.get("/test")
    assert response.status_code == 200

    logs_text = log_stream.getvalue()
    log_lines = [line.strip() for line in logs_text.split('\n') if line.strip()]

    # Find response log
    response_log = None
    for line in log_lines:
        parsed = json.loads(line)
        if "status_code" in parsed:
            response_log = parsed
            break

    assert response_log is not None
    assert response_log["status_code"] == 200


def test_no_malformed_json_in_logs(app_with_json_logging):
    """Verify no malformed JSON in logs (all valid)"""
    app, log_stream = app_with_json_logging
    client = TestClient(app)

    response = client.get("/test")
    assert response.status_code == 200

    logs_text = log_stream.getvalue()
    log_lines = [line.strip() for line in logs_text.split('\n') if line.strip()]

    # Verify all lines are valid JSON
    for line in log_lines:
        try:
            json.loads(line)
        except json.JSONDecodeError:
            pytest.fail(f"Malformed JSON in log: {line}")
