import subprocess
from pathlib import Path
from ..core.entities import RepoState, Branch
from ..core.ports import IGitPerceiver

class LocalGitPerceiver(IGitPerceiver):
    def __init__(self, repo_path: Path):
        self.repo_path = repo_path

    def _run_git(self, *args) -> str:
        cmd = ["git", "-C", str(self.repo_path)] + list(args)
        res = subprocess.run(cmd, capture_output=True, text=True)
        return res.stdout.strip()

    def gather_state(self) -> RepoState:
        current_branch = self._run_git("rev-parse", "--abbrev-ref", "HEAD")
        
        status_out = self._run_git("status", "--porcelain")
        dirty = sum(1 for line in status_out.splitlines() if not line.startswith("??"))
        untracked = sum(1 for line in status_out.splitlines() if line.startswith("??"))
        
        # Get branches
        branches = []
        branch_out = self._run_git("for-each-ref", "--format=%(refname:short)|%(objectname)|%(authorname)|%(authordate:short)", "refs/heads/")
        for line in branch_out.splitlines():
            if not line: continue
            parts = line.split("|")
            if len(parts) >= 4:
                branches.append(Branch(name=parts[0], commit_hash=parts[1], author=parts[2], date=parts[3]))
                
        # Check gate markers
        gate_markers = {
            "gate-0": (self.repo_path / ".gate-0").exists(),
            "gate-1": (self.repo_path / ".gate-1").exists(),
            "gate-2": (self.repo_path / ".gate-2").exists()
        }
        
        return RepoState(
            current_branch=current_branch,
            dirty_files=dirty,
            untracked_files=untracked,
            branches=branches,
            gate_markers=gate_markers
        )
