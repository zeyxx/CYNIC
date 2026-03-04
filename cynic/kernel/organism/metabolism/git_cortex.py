"""
CYNIC Git Cortex - Sovereign Version Control & GitOps Workflow.
Respects SRE, Security & Solutions Architect Lenses.

This cortex prevents CYNIC from modifying the 'master' branch directly.
It enforces the 'Straight Corridor':
1. Branch -> 2. Sandbox Edit -> 3. Local Test -> 4. Commit -> 5. Pull Request.
"""

from __future__ import annotations

import asyncio
import logging
import shutil
from pathlib import Path
from typing import Optional, Tuple

logger = logging.getLogger("cynic.motor.git_cortex")


class GitCortex:
    """
    Manages isolated branches, worktrees, and Pull Requests.
    """

    def __init__(self, base_repo_path: str = "."):
        self.base_repo = Path(base_repo_path).absolute()

    async def _run_git(self, *args, cwd: Optional[Path] = None) -> Tuple[bool, str]:
        cmd = ["git"] + list(args)
        proc = await asyncio.create_subprocess_exec(
            *cmd,
            cwd=cwd or self.base_repo,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, stderr = await proc.communicate()
        if proc.returncode == 0:
            return True, stdout.decode().strip()
        return False, stderr.decode().strip()

    async def create_sandbox_branch(self, branch_name: str) -> Optional[Path]:
        """
        Creates an isolated Git Worktree.
        This is a 'Local OpenSandbox' that prevents dirtying the main working directory.
        """
        sandbox_dir = self.base_repo.parent / f"cynic-sandbox-{branch_name}"

        # 1. Clean up old sandbox if it exists
        if sandbox_dir.exists():
            shutil.rmtree(sandbox_dir, ignore_errors=True)
            await self._run_git("worktree", "prune")

        # 2. Check if branch already exists
        exists, _ = await self._run_git(
            "show-ref", "--verify", f"refs/heads/{branch_name}"
        )
        if exists:
            await self._run_git(
                "branch", "-D", branch_name
            )  # Force delete for fresh start

        # 3. Create the Worktree
        success, out = await self._run_git(
            "worktree", "add", "-b", branch_name, str(sandbox_dir)
        )
        if not success:
            logger.error(f"Failed to create sandbox worktree: {out}")
            return None

        logger.info(f"Sandbox created at {sandbox_dir} on branch {branch_name}")
        return sandbox_dir

    async def commit_and_push(self, sandbox_dir: Path, commit_msg: str) -> bool:
        """Commits changes inside the sandbox and pushes to remote."""
        # Add all
        success, out = await self._run_git("add", ".", cwd=sandbox_dir)
        if not success:
            logger.error(f"Failed to stage files: {out}")
            return False

        # Check if there is anything to commit
        status_ok, status_out = await self._run_git(
            "status", "--porcelain", cwd=sandbox_dir
        )
        if not status_out:
            logger.warning("No changes to commit in sandbox.")
            return False

        # Commit
        success, out = await self._run_git("commit", "-m", commit_msg, cwd=sandbox_dir)
        if not success:
            logger.error(f"Commit failed: {out}")
            return False

        # Push (assumes current branch name)
        branch_name = sandbox_dir.name.replace("cynic-sandbox-", "")
        logger.info(f"Pushing branch {branch_name} to origin...")
        success, out = await self._run_git(
            "push", "-u", "origin", branch_name, cwd=sandbox_dir
        )

        if not success:
            logger.error(f"Push failed: {out}")
            return False

        return True

    async def create_pull_request(
        self, sandbox_dir: Path, title: str, body: str
    ) -> bool:
        """
        Uses GitHub CLI (gh) to open a PR.
        This is the final step of the Straight Corridor.
        """
        cmd = ["gh", "pr", "create", "--title", title, "--body", body]
        proc = await asyncio.create_subprocess_exec(
            *cmd,
            cwd=sandbox_dir,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, stderr = await proc.communicate()

        if proc.returncode == 0:
            pr_url = stdout.decode().strip()
            logger.info(f"✅ Pull Request successfully created: {pr_url}")
            return True
        else:
            logger.error(
                f"Failed to create PR (Is `gh` installed and authenticated?): {stderr.decode().strip()}"
            )
            return False

    async def destroy_sandbox(self, sandbox_dir: Path):
        """Removes the isolated environment to maintain UPLINK Rigor."""
        logger.info(f"Destroying sandbox: {sandbox_dir}")
        shutil.rmtree(sandbox_dir, ignore_errors=True)
        await self._run_git("worktree", "prune")
