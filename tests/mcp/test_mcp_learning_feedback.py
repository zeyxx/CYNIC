"""
Tests for Phase 2: MCP Learning Feedback.

Verifies that:
1. ask_cynic â†’ orchestrator.run() returns real Judgment result
2. learn endpoint â†’ emits USER_FEEDBACK event on CORE bus
3. Fallback behavior when orchestrator unavailable
4. observe_cynic â†’ returns state snapshot
"""

import pytest

pytest.skip("Old architecture: module removed in V5", allow_module_level=True)
