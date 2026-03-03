"""
CYNIC Auto-Surgery Sandbox - Sterile Operating Room.

Provides a completely isolated environment for the MCTS Scientist to test
code mutations without risking the active organism's survival.
Uses git worktrees for zero-overhead, isolated branching.

Lentille: Security / SRE
"""
import asyncio
import logging
import os
import shutil
import subprocess
from pathlib import Path
from typing import Dict, Optional

logger = logging.getLogger("cynic.kernel.organism.brain.cognition.cortex.surgery")

class AutoSurgeon:
    """
    Executes code mutations in an isolated git worktree sandbox.
    Guarantees that active kernel memory and processes are never corrupted.
    """
    def __init__(self, root_dir: str = "."):
        self.root = Path(root_dir).absolute()
        self.worktree_base = self.root / ".worktrees"
        self.worktree_base.mkdir(exist_ok=True)

    def _run_cmd(self, cmd: str, cwd: Optional[Path] = None) -> tuple[int, str, str]:
        """Runs a shell command and returns (exit_code, stdout, stderr)."""
        target_dir = cwd or self.root
        process = subprocess.Popen(
            cmd,
            shell=True,
            cwd=str(target_dir),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
        out, err = process.communicate()
        return process.returncode, out, err

    def prepare_sandbox(self, experiment_id: str) -> Path:
        """
        Creates a sterile environment (git worktree) for the experiment.
        """
        sandbox_path = self.worktree_base / f"surgery_{experiment_id}"
        branch_name = f"experiment/{experiment_id}"
        
        if sandbox_path.exists():
            self.cleanup_sandbox(experiment_id)

        # Create new branch and worktree
        code, out, err = self._run_cmd(f"git worktree add -b {branch_name} {sandbox_path} master")
        if code != 0:
            raise RuntimeError(f"Failed to create sandbox: {err}")
            
        logger.info(f"[*] Sterile sandbox prepared at {sandbox_path}")
        return sandbox_path

    def apply_mutations(self, sandbox_path: Path, mutations: Dict[str, str]) -> None:
        """
        Injects the hypothesized code changes into the sandbox.
        """
        for filepath, new_content in mutations.items():
            target_file = sandbox_path / filepath
            
            # Ensure parent directories exist
            target_file.parent.mkdir(parents=True, exist_ok=True)
            
            with open(target_file, "w", encoding="utf-8") as f:
                f.write(new_content)
                
            logger.debug(f"    - Mutated {filepath}")

    async def run_validation(self, sandbox_path: Path) -> bool:
        """
        Runs the test suite inside the sandbox.
        Returns True if the organism survives the mutation.
        """
        logger.info(f"[*] Validating mutation in sandbox...")
        
        # 1. Syntax & Linting (Ruff)
        code, out, err = self._run_cmd("ruff check .", cwd=sandbox_path)
        if code != 0:
            logger.warning(f"[!] Syntax validation failed in sandbox:\n{out}\n{err}")
            return False
            
        # 2. Survival Tests (pytest)
        # We run the core integration tests to ensure the organism still awakens
        code, out, err = self._run_cmd("pytest tests/test_integration_kernel_full_cycle.py", cwd=sandbox_path)
        if code != 0:
            logger.warning(f"[!] Survival validation failed in sandbox:\n{out[-500:]}")
            return False
            
        logger.info("[*] Mutation validation SUCCESS. Organism survives.")
        return True

    def suture(self, experiment_id: str) -> bool:
        """
        Merges the successful experiment back into the main organism (master branch).
        """
        branch_name = f"experiment/{experiment_id}"
        logger.info(f"[*] Commencing suture... merging {branch_name} into main.")
        
        # Merge the experiment branch
        code, out, err = self._run_cmd(f"git merge {branch_name} --no-ff -m 'feat(auto-surgery): apply successful mutation {experiment_id}'")
        
        if code != 0:
            logger.error(f"[!] Suture failed (merge conflict). Reverting.\n{err}")
            self._run_cmd("git merge --abort")
            return False
            
        logger.info("[*] Suture complete. Organism evolved.")
        return True

    def cleanup_sandbox(self, experiment_id: str) -> None:
        """
        Destroys the sterile environment after the operation.
        """
        sandbox_path = self.worktree_base / f"surgery_{experiment_id}"
        branch_name = f"experiment/{experiment_id}"
        
        self._run_cmd(f"git worktree remove --force {sandbox_path}")
        self._run_cmd(f"git branch -D {branch_name}")
        
        if sandbox_path.exists():
            shutil.rmtree(sandbox_path, ignore_errors=True)
            
        logger.debug(f"[*] Sandbox {experiment_id} incinerated.")
