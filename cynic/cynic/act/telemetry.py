"""
SessionTelemetry — quantification of Claude Code sessions.

Every Claude Code session produces a rich telemetry record:
- Task type classification (7 categories from prompt keywords)
- Complexity estimation (4 tiers from tool sequence length)
- Multi-dimensional reward (success × efficiency × cost)
- CYNIC quality judgment of the session output (q_score, verdict)
- Full tool sequence with allow/deny rates

This turns binary "success/error" into a 28-state Q-Table
(7 task types × 4 complexity tiers) enabling task-specific learning.

For research (H1-H5): export sessions as JSONL → benchmark dataset.
"""
from __future__ import annotations

import json
import time
from collections import Counter, deque
from dataclasses import asdict, dataclass, field
from typing import Any, Dict, List


# ════════════════════════════════════════════════════════════════════════════
# TASK CLASSIFIER
# ════════════════════════════════════════════════════════════════════════════

_TASK_KEYWORDS: Dict[str, List[str]] = {
    "debug": [
        "fix", "bug", "error", "debug", "broken", "crash", "exception",
        "traceback", "failing", "issue", "wrong", "incorrect", "doesn't work",
    ],
    "refactor": [
        "refactor", "clean", "simplify", "restructure", "reorganize",
        "improve", "optimize", "rewrite", "redesign", "modernize",
    ],
    "test": [
        "test", "tests", "testing", "coverage", "spec", "unit", "pytest",
        "assertion", "mock", "fixture", "integration test",
    ],
    "review": [
        "review", "analyze", "check", "audit", "assess", "evaluate",
        "inspect", "scan", "look at", "what's wrong", "code review",
    ],
    "write": [
        "write", "create", "implement", "add", "build", "generate",
        "develop", "make", "new", "scaffold", "boilerplate",
    ],
    "explain": [
        "explain", "document", "describe", "comment", "understand",
        "how does", "what is", "why", "help me understand",
    ],
}


def classify_task(prompt: str) -> str:
    """
    Classify task type from prompt keywords.

    Returns one of: debug / refactor / test / review / write / explain / general

    Scoring: count keyword matches per category → pick the winner.
    Ties broken by category order (debug > refactor > test > ...).
    """
    if not prompt:
        return "general"

    prompt_lower = prompt.lower()
    best_type = "general"
    best_count = 0

    for task_type, keywords in _TASK_KEYWORDS.items():
        count = sum(1 for kw in keywords if kw in prompt_lower)
        if count > best_count:
            best_count = count
            best_type = task_type

    return best_type


# ════════════════════════════════════════════════════════════════════════════
# COMPLEXITY ESTIMATOR
# ════════════════════════════════════════════════════════════════════════════

def estimate_complexity(tool_sequence: List[str]) -> str:
    """
    Estimate task complexity from tool usage length.

    trivial: ≤2 tools  (echo hello, read a file)
    simple:  3-6 tools (small edit with context reads)
    medium:  7-15 tools (multi-file refactor)
    complex: >15 tools  (large feature implementation)
    """
    n = len(tool_sequence)
    if n <= 2:
        return "trivial"
    elif n <= 6:
        return "simple"
    elif n <= 15:
        return "medium"
    else:
        return "complex"


# ════════════════════════════════════════════════════════════════════════════
# REWARD FUNCTION
# ════════════════════════════════════════════════════════════════════════════

def compute_reward(
    is_error: bool,
    tool_count: int,
    cost_usd: float,
) -> float:
    """
    Multi-dimensional RL reward for SDK sessions.

    Base:           0.70 (success) / 0.20 (error)
    Efficiency:     ±0.00 to -0.15 (more tools = less efficient)
    Cost penalty:   0.00 to -0.10 (expensive sessions are less desirable)

    Result range: [0.10, 0.75] — φ-aligned, never hits 0 or 1.

    Examples:
      Success, 1 tool,  $0.001 → 0.70 + 0.06 - 0.00 = 0.76 → capped 0.75
      Success, 8 tools, $0.010 → 0.70 - 0.08 - 0.02 = 0.60
      Error,   3 tools, $0.005 → 0.20 - 0.02 - 0.01 = 0.17
    """
    base = 0.70 if not is_error else 0.20

    # Efficiency: each tool beyond 4 costs 0.02 (max penalty 0.15)
    efficiency = max(-0.15, (4 - tool_count) * 0.02)

    # Cost: penalize sessions > $0.01 (typical API cost threshold)
    cost_penalty = -min(cost_usd / 0.10, 0.10)

    reward = base + efficiency + cost_penalty
    return round(max(0.10, min(0.75, reward)), 3)


# ════════════════════════════════════════════════════════════════════════════
# SESSION TELEMETRY RECORD
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class SessionTelemetry:
    """
    Rich record of one completed Claude Code session.

    The measurement atom for CYNIC's learning and research benchmarks.
    Covers all data needed to test H1-H5 hypotheses.
    """
    session_id: str
    task: str                       # prompt (≤500 chars)
    task_type: str                  # debug/refactor/test/review/write/explain/general
    complexity: str                 # trivial/simple/medium/complex

    model: str
    tools_sequence: List[str]       # ordered: ["Read", "Edit", "Bash", ...]
    tools_allowed: int
    tools_denied: int
    tool_allow_rate: float          # 0.0-1.0

    input_tokens: int               # accumulated across all assistant messages
    output_tokens: int
    total_cost_usd: float
    duration_s: float               # from Claude's result.duration_ms
    is_error: bool
    result_text: str                # Claude's result description (≤500 chars)

    # CYNIC quality judgment of the session output (REFLEX level)
    output_q_score: float           # 0-61.8 (φ-bounded)
    output_verdict: str             # BARK/GROWL/WAG/HOWL
    output_confidence: float        # 0-0.618

    # Learning signal used
    state_key: str                  # "SDK:{model}:{task_type}:{complexity}"
    reward: float                   # compute_reward() result

    timestamp: float = field(default_factory=time.time)


# ════════════════════════════════════════════════════════════════════════════
# TELEMETRY STORE
# ════════════════════════════════════════════════════════════════════════════

class TelemetryStore:
    """
    In-memory ring buffer for session telemetry.

    Holds up to maxlen records (oldest discarded). Supports:
    - stats() → aggregate metrics (verdicts, task_types, error_rate, cost)
    - recent(n) → last N records as dicts
    - export() → all records as dicts (for benchmark analysis)
    - save_jsonl(path) → persist to file (one record per line)
    """

    def __init__(self, maxlen: int = 1000):
        self._records: deque = deque(maxlen=maxlen)

    def __len__(self) -> int:
        return len(self._records)

    def add(self, record: SessionTelemetry) -> None:
        self._records.append(record)

    def recent(self, n: int = 10) -> List[Dict[str, Any]]:
        records = list(self._records)
        return [asdict(r) for r in records[-n:]]

    def export(self) -> List[Dict[str, Any]]:
        return [asdict(r) for r in self._records]

    def stats(self) -> Dict[str, Any]:
        records = list(self._records)
        if not records:
            return {
                "count": 0,
                "message": "No sessions recorded yet",
            }

        costs = [r.total_cost_usd for r in records]
        q_scores = [r.output_q_score for r in records]
        rewards = [r.reward for r in records]

        return {
            "count": len(records),
            "total_cost_usd": round(sum(costs), 6),
            "mean_cost_usd": round(sum(costs) / len(costs), 6),
            "error_rate": round(
                sum(1 for r in records if r.is_error) / len(records), 3
            ),
            "mean_q_score": round(sum(q_scores) / len(q_scores), 2),
            "mean_reward": round(sum(rewards) / len(rewards), 3),
            "verdicts": dict(Counter(r.output_verdict for r in records)),
            "task_types": dict(Counter(r.task_type for r in records)),
            "complexities": dict(Counter(r.complexity for r in records)),
        }

    def save_jsonl(self, path: str) -> int:
        """Save all records as JSONL (one JSON per line). Returns record count."""
        records = list(self._records)
        with open(path, "w", encoding="utf-8") as fh:
            for record in records:
                fh.write(json.dumps(asdict(record)) + "\n")
        return len(records)
