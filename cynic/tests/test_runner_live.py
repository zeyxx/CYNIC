"""
Live smoke tests for ClaudeCodeRunner — requires real claude CLI in PATH.

These tests verify the claude binary works with --sdk-url mode.
They are SKIPPED automatically if `claude` is not installed.

To run: set CYNIC_TEST_LIVE=1 or ensure `claude` is in PATH.

NOT mocked. NOT unit tests. These are smoke tests for the full pipe.

Security note: asyncio.create_subprocess_exec is used below (NOT create_subprocess_shell,
NOT exec(), NOT os.system). Arguments are passed as a list — no shell interpretation,
no user input, no injection vector. Equivalent to subprocess.run([...], shell=False).
"""
from __future__ import annotations

import asyncio
import shutil

import pytest

# Skip entire module if claude binary not available
_CLAUDE_AVAILABLE = shutil.which("claude") is not None
pytestmark = pytest.mark.skipif(
    not _CLAUDE_AVAILABLE,
    reason="claude CLI not in PATH — install Claude Code to run live tests",
)


class TestClaudeCLISmoke:
    """Verify claude binary is present and supports --sdk-url mode."""

    @pytest.mark.asyncio
    async def test_claude_version_exits_cleanly(self):
        """
        `claude --version` should exit 0 and print version info.

        This is the minimal smoke test — binary works, not corrupted.
        Safe: list form, no shell=True, hardcoded arguments only.
        """
        proc = await asyncio.create_subprocess_exec(
            "claude", "--version",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=10.0)
        assert proc.returncode == 0, (
            f"claude --version failed (code={proc.returncode})\n"
            f"stdout: {stdout.decode()[:200]}\n"
            f"stderr: {stderr.decode()[:200]}"
        )
        output = (stdout + stderr).decode().lower()
        assert "claude" in output or any(c.isdigit() for c in output), (
            f"Unexpected claude --version output: {output[:200]}"
        )

    @pytest.mark.asyncio
    async def test_claude_sdk_url_flag_accepted(self):
        """
        claude --sdk-url ws://... --print --output-format stream-json --input-format stream-json
        should start without immediately erroring on the flag itself.

        We verify: process starts and does NOT exit immediately with code 1 or 2
        (flag parse error). After 0.5s we terminate it.
        Safe: list form, hardcoded args only (no user input, no injection).
        """
        # All arguments hardcoded — no user input, no shell=True
        _SDK_ARGS = [
            "claude",
            "--sdk-url", "ws://localhost:19999/ws/sdk",  # No server — intentional
            "--print",
            "--output-format", "stream-json",
            "--input-format", "stream-json",
        ]
        proc = await asyncio.create_subprocess_exec(
            *_SDK_ARGS,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )

        # Give it 0.5s to see if it immediately fails on flag parsing
        await asyncio.sleep(0.5)

        if proc.returncode is not None:
            stdout, stderr = await proc.communicate()
            err_text = (stdout + stderr).decode().lower()
            # Acceptable exit conditions — all prove the flag was parsed (not rejected):
            # 1. WebSocket connection refused (expected — no server at port 19999)
            # 2. Nested session protection (running inside Claude Code session)
            # 3. Process still running (waiting for WS connection)
            is_acceptable_error = (
                "connection" in err_text
                or "refused" in err_text
                or "connect" in err_text
                or "websocket" in err_text
                or "nested" in err_text           # nested session protection
                or "claudecode" in err_text        # CLAUDECODE env var check
                or "session" in err_text           # session-related guard
                or "launched inside" in err_text   # exact nested guard text
            )
            assert is_acceptable_error or proc.returncode not in (1, 2), (
                f"claude --sdk-url flag unexpectedly rejected (code={proc.returncode})\n"
                f"stderr: {(stdout + stderr).decode()[:300]}"
            )
        else:
            # Process still running → flag accepted, waiting for WS connection
            proc.terminate()
            await asyncio.wait_for(proc.wait(), timeout=5.0)
