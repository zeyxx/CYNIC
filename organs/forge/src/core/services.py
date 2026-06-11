from .ports import IHealthAuditor
from .entities import RepoState

class HealthAuditor(IHealthAuditor):
    def audit(self, state: RepoState) -> RepoState:
        score = 100.0
        state.alerts = [] # Clear old alerts
        
        # 1. Dirty State
        if state.dirty_files > 0:
            score -= 5
            state.add_alert("WARN", f"Repo is dirty ({state.dirty_files} modified files).")
            
        if state.untracked_files > 0:
            score -= 2
            state.add_alert("INFO", f"Repo has {state.untracked_files} untracked files.")
            
        # 2. Gate Markers
        if not state.gate_markers.get("gate-0", False):
            score -= 20
            state.add_alert("CRITICAL", "Gate-0 is missing. Basic grep checks and formatting have failed.")
            
        if not state.gate_markers.get("gate-1", False):
            score -= 10
            state.add_alert("WARN", "Gate-1 (clippy) is missing.")
            
        if not state.gate_markers.get("gate-2", False):
            score -= 10
            state.add_alert("INFO", "Gate-2 (tests) is missing. Deploy is blocked.")
            
        # 3. Branch sprawl
        num_branches = len(state.branches)
        if num_branches > 5:
            penalty = (num_branches - 5) * 2
            score -= penalty
            state.add_alert("WARN", f"Branch sprawl detected: {num_branches} branches active.")
            
        state.health_score = max(0.0, score)
        return state
