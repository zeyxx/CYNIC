"""
CYNIC Motor System " Sovereign Actuators.

Actuators that do not depend on external AI services, 
ensuring the organism can maintain itself using local tools.
"""
from __future__ import annotations

import asyncio
import logging
import os
import time
from typing import Any, Dict
from pathlib import Path

from cynic.kernel.organism.metabolism.actuators import AbstractActuator, ActResult

logger = logging.getLogger("cynic.metabolism.actuators.sovereign")

class LocalCodeActuator(AbstractActuator):
    """
    Sovereign Code Editor.
    Applies changes using local file operations and validates with local tests.
    NO external API dependency.
    """
    async def execute(self, payload: Dict[str, Any]) -> ActResult:
        start = time.perf_counter()
        path = payload.get("path")
        content = payload.get("content")
        test_cmd = payload.get("test_command", "pytest")
        
        if not path or content is None:
            return ActResult(False, None, 0.0, "Missing path or content")

        file_path = Path(path)
        backup_path = file_path.with_suffix(".bak")
        
        try:
            # 1. Backup current state
            if file_path.exists():
                file_path.rename(backup_path)
            
            # 2. Apply change
            file_path.write_text(content, encoding="utf-8")
            
            # 3. VERIFY (The Sovereign Loop)
            # Run local tests to ensure we didn't break the matter
            process = await asyncio.create_subprocess_shell(
                f"export PYTHONPATH=$PYTHONPATH:. && {test_cmd} {path}",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, stderr = await process.communicate()
            
            if process.returncode == 0:
                # Success! Remove backup
                if backup_path.exists(): os.remove(backup_path)
                return ActResult(
                    success=True,
                    output=stdout.decode(),
                    duration_ms=(time.perf_counter() - start) * 1000
                )
            else:
                # Failure! Rollback Matter
                if backup_path.exists():
                    if file_path.exists(): os.remove(file_path)
                    backup_path.rename(file_path)
                
                return ActResult(
                    success=False,
                    output=stdout.decode(),
                    error=stderr.decode() or f"Tests failed with code {process.returncode}",
                    duration_ms=(time.perf_counter() - start) * 1000
                )

        except Exception as e:
            # Emergency Rollback
            if backup_path.exists():
                backup_path.rename(file_path)
            return ActResult(False, None, 0.0, str(e))

class ShellActuator(AbstractActuator):
    """Allows CYNIC to execute local system commands."""
    async def execute(self, payload: Dict[str, Any]) -> ActResult:
        start = time.perf_counter()
        command = payload.get("command")
        if not command:
            return ActResult(False, None, 0.0, "No command provided")

        try:
            process = await asyncio.create_subprocess_shell(
                command,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, stderr = await process.communicate()
            
            return ActResult(
                success=(process.returncode == 0),
                output=stdout.decode(),
                error=stderr.decode() if process.returncode != 0 else None,
                duration_ms=(time.perf_counter() - start) * 1000
            )
        except Exception as e:
            return ActResult(False, None, 0.0, str(e))
