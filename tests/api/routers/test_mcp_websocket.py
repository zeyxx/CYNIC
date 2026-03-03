"""Tests for /ws/mcp WebSocket endpoint.

Uses a lightweight FastAPI app (no lifespan) to test the WebSocket
endpoint in isolation. The auto_register system handles wiring
into the real app at runtime.
"""

import pytest

pytest.skip("Old architecture: module removed in V5", allow_module_level=True)
