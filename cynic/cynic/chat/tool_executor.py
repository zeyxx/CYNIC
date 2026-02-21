"""
CYNIC Chat Tool Executor — Execute tools with REFLEX judgment safety.

Every dangerous tool (bash, write, edit) gets a REFLEX judgment from the
GuardianDog before execution. BARK → blocked. Everything else → proceed.

Read-only tools (read, glob, grep) execute without judgment overhead.

NOTE: The bash tool intentionally uses create_subprocess_shell because
it must support pipes, redirects, and other shell features. CYNIC's
GuardianDog REFLEX judgment provides the safety layer (BARK → blocked).
"""
from __future__ import annotations

import asyncio
import logging
import os
import pathlib
import re
import time
from typing import Any

from cynic.chat.tools import ToolCall, ToolResult, DANGEROUS_TOOLS
from cynic.core.formulas import BASH_OUTPUT_CAP, READ_FILE_CAP, GLOB_MATCH_CAP, GREP_OUTPUT_CAP

logger = logging.getLogger("cynic.chat.executor")

# Output caps imported from formulas.py (centralized constants)
_BASH_CAP = BASH_OUTPUT_CAP
_READ_CAP = READ_FILE_CAP
_GLOB_CAP = GLOB_MATCH_CAP
_GREP_CAP = GREP_OUTPUT_CAP


class ToolExecutor:
    """
    Execute tool calls with optional CYNIC REFLEX judgment.

    If an orchestrator is wired, dangerous tools get judged before execution.
    Without an orchestrator, all tools execute freely (standalone mode).
    """

    def __init__(
        self,
        cwd: Optional[str] = None,
        orchestrator: Any = None,
    ) -> None:
        self.cwd = cwd or os.getcwd()
        self.orchestrator = orchestrator  # JudgeOrchestrator (optional)

    def _resolve_path(self, path: str) -> str:
        """Resolve relative paths against cwd."""
        p = pathlib.Path(path)
        if not p.is_absolute():
            p = pathlib.Path(self.cwd) / p
        return str(p)

    async def execute(self, call: ToolCall) -> ToolResult:
        """Execute a single tool call, optionally with REFLEX judgment."""
        start = time.time()

        if not call.is_valid:
            return ToolResult(
                call=call,
                error=f"Unknown tool: {call.name}",
                duration_ms=(time.time() - start) * 1000,
            )

        # REFLEX judgment for dangerous tools
        if call.is_dangerous and self.orchestrator is not None:
            result = await self._judge_before_execute(call)
            if result is not None:
                result.duration_ms = (time.time() - start) * 1000
                return result

        # Execute the tool
        try:
            output = await self._dispatch(call)
            return ToolResult(
                call=call,
                output=output,
                duration_ms=(time.time() - start) * 1000,
            )
        except CynicError as exc:
            return ToolResult(
                call=call,
                error=str(exc),
                duration_ms=(time.time() - start) * 1000,
            )

    async def _judge_before_execute(self, call: ToolCall) -> Optional[ToolResult]:
        """Run REFLEX judgment on dangerous tools. Returns ToolResult if blocked, None if OK."""
        try:
            from cynic.core.judgment import Cell
            from cynic.core.consciousness import ConsciousnessLevel

            content = f"Tool: {call.name}\nArgs: {call.preview(200)}"
            cell = Cell(
                reality="CODE",
                analysis="ACT",
                content=content,
                context=f"CYNIC Code tool execution: {call.name}",
            )
            judgment = await self.orchestrator.run(cell, level=ConsciousnessLevel.REFLEX)

            if judgment.verdict == "BARK":
                return ToolResult(
                    call=call,
                    blocked=True,
                    verdict=judgment.verdict,
                    q_score=judgment.q_score,
                    error=f"Blocked by GuardianDog: {judgment.verdict} Q={judgment.q_score:.1f}",
                )
            # Not blocked — proceed with execution
            return None
        except OSError as exc:
            logger.debug("REFLEX judgment skipped: %s", exc)
            return None  # Fail open — don't block on judgment errors

    async def _dispatch(self, call: ToolCall) -> str:
        """Dispatch to the appropriate tool handler."""
        name = call.name
        args = call.arguments

        if name == "bash":
            return await self._exec_bash(args.get("command", ""), args.get("timeout", 120))
        if name == "read":
            return await self._exec_read(args.get("path", ""), args.get("offset"), args.get("limit"))
        if name == "write":
            return await self._exec_write(args.get("path", ""), args.get("content", ""))
        if name == "edit":
            return await self._exec_edit(args.get("path", ""), args.get("old_string", ""), args.get("new_string", ""))
        if name == "glob":
            return await self._exec_glob(args.get("pattern", ""), args.get("path"))
        if name == "grep":
            return await self._exec_grep(args.get("pattern", ""), args.get("path"), args.get("glob"))

        return f"Unknown tool: {name}"

    # ── Individual tool implementations ───────────────────────────────────

    async def _exec_bash(self, command: str, timeout: int = 120) -> str:
        """Execute shell command. Uses subprocess_shell intentionally for pipe/redirect support.
        Safety: GuardianDog REFLEX judgment blocks dangerous commands before this runs."""
        if not command.strip():
            raise ValueError("Empty command")

        proc = await asyncio.create_subprocess_shell(
            command,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            cwd=self.cwd,
        )
        try:
            stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=timeout)
        except TimeoutError:
            proc.kill()
            raise TimeoutError(f"Command timed out after {timeout}s")

        out = stdout.decode("utf-8", errors="replace") if stdout else ""
        err = stderr.decode("utf-8", errors="replace") if stderr else ""

        result = ""
        if out:
            result += out
        if err:
            result += f"\n[stderr]\n{err}" if result else err
        if proc.returncode and proc.returncode != 0:
            result += f"\n[exit code: {proc.returncode}]"

        if len(result) > _BASH_CAP:
            result = result[:_BASH_CAP] + f"\n... (truncated, {len(result)} total chars)"
        return result or "(no output)"

    async def _exec_read(self, path: str, offset: Optional[int] = None, limit: Optional[int] = None) -> str:
        if not path:
            raise ValueError("Path required")
        resolved = self._resolve_path(path)

        loop = asyncio.get_running_loop()
        content = await loop.run_in_executor(None, self._sync_read, resolved, offset, limit)
        if len(content) > _READ_CAP:
            content = content[:_READ_CAP] + f"\n... (truncated, {len(content)} total chars)"
        return content

    @staticmethod
    def _sync_read(path: str, offset: Optional[int], limit: Optional[int]) -> str:
        with open(path, encoding="utf-8", errors="replace") as fh:
            lines = fh.readlines()

        start = max((offset or 1) - 1, 0)
        end = start + limit if limit else len(lines)
        selected = lines[start:end]

        # Number lines (like cat -n)
        numbered = []
        for i, line in enumerate(selected, start=start + 1):
            numbered.append(f"{i:>6}\t{line.rstrip()}")
        return "\n".join(numbered) if numbered else "(empty file)"

    async def _exec_write(self, path: str, content: str) -> str:
        if not path:
            raise ValueError("Path required")
        resolved = self._resolve_path(path)

        loop = asyncio.get_running_loop()
        await loop.run_in_executor(None, self._sync_write, resolved, content)
        lines = content.count("\n") + 1
        return f"Wrote {lines} lines to {path}"

    @staticmethod
    def _sync_write(path: str, content: str) -> None:
        os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
        with open(path, "w", encoding="utf-8") as fh:
            fh.write(content)

    async def _exec_edit(self, path: str, old_string: str, new_string: str) -> str:
        if not path:
            raise ValueError("Path required")
        if not old_string:
            raise ValueError("old_string required")
        if old_string == new_string:
            raise ValueError("old_string and new_string are identical")
        resolved = self._resolve_path(path)

        loop = asyncio.get_running_loop()
        return await loop.run_in_executor(None, self._sync_edit, resolved, old_string, new_string)

    @staticmethod
    def _sync_edit(path: str, old_string: str, new_string: str) -> str:
        with open(path, encoding="utf-8") as fh:
            content = fh.read()

        count = content.count(old_string)
        if count == 0:
            raise ValueError(f"old_string not found in {path}")
        if count > 1:
            raise ValueError(f"old_string found {count} times in {path} — must be unique")

        new_content = content.replace(old_string, new_string, 1)
        with open(path, "w", encoding="utf-8") as fh:
            fh.write(new_content)
        return f"Edited {path} (1 replacement)"

    async def _exec_glob(self, pattern: str, path: Optional[str] = None) -> str:
        if not pattern:
            raise ValueError("Pattern required")
        base = pathlib.Path(self._resolve_path(path) if path else self.cwd)

        loop = asyncio.get_running_loop()
        matches = await loop.run_in_executor(None, lambda: sorted(base.glob(pattern)))

        if not matches:
            return "(no matches)"

        lines = [str(m.relative_to(base)) for m in matches]
        result = "\n".join(lines)
        if len(result) > _GLOB_CAP:
            result = result[:_GLOB_CAP] + f"\n... ({len(matches)} total matches)"
        return result

    async def _exec_grep(self, pattern: str, path: Optional[str] = None, glob_filter: Optional[str] = None) -> str:
        if not pattern:
            raise ValueError("Pattern required")
        base = self._resolve_path(path) if path else self.cwd

        # Try ripgrep first, fallback to Python regex
        try:
            return await self._grep_rg(pattern, base, glob_filter)
        except FileNotFoundError:
            return await self._grep_python(pattern, base, glob_filter)

    async def _grep_rg(self, pattern: str, path: str, glob_filter: Optional[str]) -> str:
        cmd = ["rg", "--no-heading", "-n", "--max-count", "50", pattern, path]
        if glob_filter:
            cmd.extend(["--glob", glob_filter])

        proc = await asyncio.create_subprocess_exec(
            *cmd,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, _ = await asyncio.wait_for(proc.communicate(), timeout=30)
        result = stdout.decode("utf-8", errors="replace") if stdout else "(no matches)"
        if len(result) > _GREP_CAP:
            result = result[:_GREP_CAP] + "\n... (truncated)"
        return result

    async def _grep_python(self, pattern: str, path: str, glob_filter: Optional[str]) -> str:
        """Fallback grep using Python re module."""
        base = pathlib.Path(path)
        regex = re.compile(pattern)
        results: list[str] = []
        total_len = 0

        if base.is_file():
            files = [base]
        else:
            file_pattern = glob_filter or "**/*"
            files = sorted(base.glob(file_pattern))

        loop = asyncio.get_running_loop()

        def _search() -> list[str]:
            nonlocal total_len
            for fp in files:
                if not fp.is_file():
                    continue
                try:
                    text = fp.read_text(encoding="utf-8", errors="replace")
                except httpx.RequestError:
                    continue
                for i, line in enumerate(text.splitlines(), 1):
                    if regex.search(line):
                        entry = f"{fp}:{i}:{line.rstrip()}"
                        results.append(entry)
                        total_len += len(entry) + 1
                        if total_len > _GREP_CAP or len(results) >= 50:
                            return results
            return results

        await loop.run_in_executor(None, _search)
        return "\n".join(results) if results else "(no matches)"
