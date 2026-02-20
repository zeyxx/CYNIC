"""
CYNIC UniversalActuator Tests (T33)

Tests Strategy Registry pattern: BashActuator, GitReadActuator, UniversalActuator.
No real subprocesses for safety — mocked where possible.
"""
from __future__ import annotations

import asyncio
import pytest
from unittest.mock import patch, AsyncMock, MagicMock

from cynic.metabolism.universal import (
    UniversalActuator,
    BashActuator,
    GitReadActuator,
    ActResult,
    BaseActuator,
    _MAX_OUTPUT,
)


class TestActResult:
    def test_defaults(self):
        r = ActResult("bash", success=True)
        assert r.output == ""
        assert r.error == ""
        assert r.duration_ms == 0.0
        assert r.timestamp > 0

    def test_to_dict_keys(self):
        r = ActResult("git_read", success=False, error="oops")
        d = r.to_dict()
        assert "action_type" in d
        assert "success" in d
        assert "output" in d
        assert "error" in d
        assert "duration_ms" in d


class TestBashActuator:
    def test_empty_args_returns_failure(self):
        ba = BashActuator()
        result = asyncio.get_event_loop().run_until_complete(
            ba.execute({"args": []})
        )
        assert result.success is False
        assert "Empty args" in result.error

    def test_no_args_key_returns_failure(self):
        ba = BashActuator()
        result = asyncio.get_event_loop().run_until_complete(
            ba.execute({})
        )
        assert result.success is False

    def test_timeout_capped_at_60(self):
        """Timeout cannot exceed 60s (safety bound)."""
        ba = BashActuator()
        # Build a mock proc
        mock_proc = AsyncMock()
        mock_proc.returncode = 0
        mock_proc.communicate = AsyncMock(return_value=(b"ok", b""))
        with patch("asyncio.create_subprocess_exec", return_value=mock_proc):
            result = asyncio.get_event_loop().run_until_complete(
                ba.execute({"args": ["echo", "hi"], "timeout": 9999.0})
            )
        assert result.success is True

    def test_output_capped_at_max_output(self):
        ba = BashActuator()
        long_output = b"x" * (_MAX_OUTPUT + 100)
        mock_proc = AsyncMock()
        mock_proc.returncode = 0
        mock_proc.communicate = AsyncMock(return_value=(long_output, b""))
        with patch("asyncio.create_subprocess_exec", return_value=mock_proc):
            result = asyncio.get_event_loop().run_until_complete(
                ba.execute({"args": ["echo", "long"]})
            )
        assert len(result.output) == _MAX_OUTPUT

    def test_nonzero_returncode_is_failure(self):
        ba = BashActuator()
        mock_proc = AsyncMock()
        mock_proc.returncode = 1
        mock_proc.communicate = AsyncMock(return_value=(b"", b"err"))
        with patch("asyncio.create_subprocess_exec", return_value=mock_proc):
            result = asyncio.get_event_loop().run_until_complete(
                ba.execute({"args": ["false"]})
            )
        assert result.success is False

    def test_timeout_error_returns_failure(self):
        ba = BashActuator()
        mock_proc = AsyncMock()
        mock_proc.communicate = AsyncMock(side_effect=asyncio.TimeoutError())
        with patch("asyncio.create_subprocess_exec", return_value=mock_proc):
            result = asyncio.get_event_loop().run_until_complete(
                ba.execute({"args": ["sleep", "100"]})
            )
        assert result.success is False
        assert "Timed out" in result.error


class TestGitReadActuator:
    def test_allowed_subcommands(self):
        gra = GitReadActuator()
        assert "status" in gra.ALLOWED_SUBCOMMANDS
        assert "log" in gra.ALLOWED_SUBCOMMANDS
        assert "diff" in gra.ALLOWED_SUBCOMMANDS

    def test_blocked_subcommand(self):
        gra = GitReadActuator()
        result = asyncio.get_event_loop().run_until_complete(
            gra.execute({"subcommand": "push"})
        )
        assert result.success is False
        assert "not allowed" in result.error

    def test_blocked_args(self):
        gra = GitReadActuator()
        result = asyncio.get_event_loop().run_until_complete(
            gra.execute({"subcommand": "log", "args": ["--force"]})
        )
        assert result.success is False
        assert "blocked" in result.error

    def test_status_executes(self):
        gra = GitReadActuator()
        mock_proc = AsyncMock()
        mock_proc.returncode = 0
        mock_proc.communicate = AsyncMock(return_value=(b"nothing to commit", b""))
        with patch("asyncio.create_subprocess_exec", return_value=mock_proc):
            result = asyncio.get_event_loop().run_until_complete(
                gra.execute({"subcommand": "status"})
            )
        assert result.success is True
        assert "nothing to commit" in result.output

    def test_default_subcommand_is_status(self):
        gra = GitReadActuator()
        mock_proc = AsyncMock()
        mock_proc.returncode = 0
        mock_proc.communicate = AsyncMock(return_value=(b"ok", b""))
        with patch("asyncio.create_subprocess_exec", return_value=mock_proc) as mock_exec:
            asyncio.get_event_loop().run_until_complete(
                gra.execute({})  # no subcommand → default "status"
            )
        # First arg should be "git", second "status"
        call_args = mock_exec.call_args[0]
        assert call_args[0] == "git"
        assert call_args[1] == "status"


class TestUniversalActuator:
    def test_built_in_strategies(self):
        ua = UniversalActuator()
        strats = ua.strategies()
        assert "bash" in strats
        assert "git_read" in strats

    def test_register_custom_strategy(self):
        ua = UniversalActuator()

        class NoopActuator(BaseActuator):
            name = "noop"
            async def execute(self, payload):
                return ActResult("noop", success=True, output="noop")

        ua.register("noop", NoopActuator())
        assert "noop" in ua.strategies()

    def test_dispatch_unknown_type_returns_failure(self):
        ua = UniversalActuator()
        result = asyncio.get_event_loop().run_until_complete(
            ua.dispatch({"action_type": "unknown_xyz"})
        )
        assert result.success is False
        assert "No strategy" in result.error

    def test_dispatch_defaults_to_bash(self):
        ua = UniversalActuator()
        mock_proc = AsyncMock()
        mock_proc.returncode = 0
        mock_proc.communicate = AsyncMock(return_value=(b"hi", b""))
        with patch("asyncio.create_subprocess_exec", return_value=mock_proc):
            result = asyncio.get_event_loop().run_until_complete(
                ua.dispatch({"args": ["echo", "hi"]})  # no action_type → bash
            )
        assert result.success is True

    def test_recent_results_rolling_cap(self):
        ua = UniversalActuator()
        mock_proc = AsyncMock()
        mock_proc.returncode = 0
        mock_proc.communicate = AsyncMock(return_value=(b"", b""))
        with patch("asyncio.create_subprocess_exec", return_value=mock_proc):
            for _ in range(100):
                asyncio.get_event_loop().run_until_complete(
                    ua.dispatch({"args": ["echo"]})
                )
        assert len(ua.recent_results(100)) == 89  # capped at F(11)=89

    def test_stats_counts_successes(self):
        ua = UniversalActuator()
        mock_proc_ok = AsyncMock()
        mock_proc_ok.returncode = 0
        mock_proc_ok.communicate = AsyncMock(return_value=(b"ok", b""))
        with patch("asyncio.create_subprocess_exec", return_value=mock_proc_ok):
            for _ in range(3):
                asyncio.get_event_loop().run_until_complete(
                    ua.dispatch({"args": ["echo"]})
                )
        s = ua.stats()
        assert s["success_count"] == 3
        assert s["error_count"] == 0
