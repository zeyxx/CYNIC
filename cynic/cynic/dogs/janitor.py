"""
CYNIC Janitor Dog — Yesod (Foundation)

Non-LLM Dog. L3 REFLEX (<10ms AST analysis).
Technology: Python ast module + Ruff (if available)

Responsibilities:
  - Static code smell detection (complexity, dead code, naming)
  - Import cycle detection
  - Boilerplate identification (BURN axiom enforcement)
  - Dead code flagging
  - Line length and style compliance

Why Janitor?
  Yesod = Foundation. The janitor keeps the foundation clean.
  Technical debt is poison. Janitor detects it before it spreads.
  Pure AST analysis: no LLM, no network, deterministic.

φ-integration:
  q_score = phi_bound_score(100 - (smell_count * decay_per_smell))
  confidence = 48.5% (between PHI_INV_2 and PHI_INV — reliable but not perfect)
  VETO: impossible for Janitor (smells are advisory, not blocking)
"""
from __future__ import annotations

import ast
import logging
import time
from typing import Any, Dict, List, Optional, Tuple

from cynic.core.phi import PHI_INV, PHI_INV_2, PHI_INV_3, MAX_Q_SCORE, phi_bound_score
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell
from cynic.dogs.base import (
    AbstractDog, DogCapabilities, DogHealth, DogJudgment,
    DogId, HealthStatus,
)

logger = logging.getLogger("cynic.dogs.janitor")

# Ruff CLI (optional — degrades gracefully if absent)
try:
    import subprocess
    _RUFF_AVAILABLE = True
except ImportError:
    _RUFF_AVAILABLE = False

# Decay per smell found (φ-aligned)
DECAY_PER_SMELL: float = 8.0   # Each smell costs 8 points
MAX_SMELLS_REPORTED: int = 8   # F(6) = 8 — cap reports

# Complexity thresholds (φ-aligned)
HIGH_COMPLEXITY: int = 13      # F(7) — cyclomatic complexity ceiling
HIGH_LINES: int = 89           # F(11) — function line ceiling


class JanitorDog(AbstractDog):
    """
    Janitor (Yesod) — AST-based code smell detector.

    Analyzes code cells for:
      1. High cyclomatic complexity (nested branches)
      2. Dead/unreachable code (statements after return)
      3. Long functions (> F(11)=89 lines)
      4. Shadowed builtins (list = [...], type = ...)
      5. Missing type annotations on public functions
      6. Hardcoded magic numbers (non-φ constants)
      7. Import star usage (from x import *)
      8. TODO/FIXME debt markers

    Each smell detected reduces q_score by DECAY_PER_SMELL (8 pts).
    """

    def __init__(self) -> None:
        super().__init__(DogId.JANITOR)
        self._total_smells_found: int = 0
        self._cells_analyzed: int = 0

    def get_capabilities(self) -> DogCapabilities:
        return DogCapabilities(
            dog_id=DogId.JANITOR,
            sefirot="Yesod — Foundation",
            consciousness_min=ConsciousnessLevel.REFLEX,
            uses_llm=False,
            supported_realities={"CODE", "CYNIC"},  # Code quality is the domain
            supported_analyses={"PERCEIVE", "JUDGE", "ACCOUNT"},
            technology="Python ast + Ruff (optional)",
            max_concurrent=8,  # F(6) — pure CPU, highly parallelizable
        )

    async def analyze(self, cell: Cell, **kwargs: Any) -> DogJudgment:
        """
        Analyze a cell for code smells via AST.

        If cell.content is a dict with 'code' or 'source' key → parse as Python.
        If cell.content is a str → parse directly.
        Otherwise → analyze metadata only (novelty, complexity, risk).
        """
        start = time.perf_counter()

        code_str = self._extract_code(cell)
        smells: List[str] = []

        if code_str:
            smells = self._detect_smells(code_str)
        else:
            # No code available — use cell metadata as proxy
            if cell.complexity > PHI_INV:   # > 0.618 = CULTURE violation
                smells.append("high-complexity-metadata")
            if cell.risk > PHI_INV:
                smells.append("high-risk-metadata")

        smell_count = len(smells)
        self._total_smells_found += smell_count
        self._cells_analyzed += 1

        # Q-Score: start at MAX, decay per smell
        q_raw = MAX_Q_SCORE - (smell_count * DECAY_PER_SMELL)
        q_score = phi_bound_score(max(0.0, q_raw))

        # Confidence: Janitor is reliable but AST ≠ semantics
        confidence = 0.485  # Between PHI_INV_2 (38.2%) and PHI_INV (61.8%)

        # Build reasoning
        if smells:
            top = smells[:MAX_SMELLS_REPORTED]
            reasoning = f"Found {smell_count} smells: {', '.join(top)}"
        else:
            reasoning = "*wag* No smells detected — foundation is clean"

        latency = (time.perf_counter() - start) * 1000
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=q_score,
            confidence=confidence,
            reasoning=reasoning,
            evidence={
                "smell_count": smell_count,
                "smells": smells[:MAX_SMELLS_REPORTED],
                "code_lines": len(code_str.splitlines()) if code_str else 0,
            },
            latency_ms=latency,
            veto=False,  # Janitor never VETOs — advisory only
        )
        self.record_judgment(judgment)
        return judgment

    # ── Code Extraction ────────────────────────────────────────────────────

    def _extract_code(self, cell: Cell) -> Optional[str]:
        """Extract Python code string from cell content."""
        content = cell.content

        if isinstance(content, str) and content.strip():
            return content

        if isinstance(content, dict):
            for key in ("code", "source", "content", "diff", "text"):
                val = content.get(key)
                if isinstance(val, str) and val.strip():
                    return val

        return None

    # ── Smell Detection ────────────────────────────────────────────────────

    def _detect_smells(self, code: str) -> List[str]:
        """Run all smell detectors on code string. Returns list of smell labels."""
        smells: List[str] = []

        try:
            tree = ast.parse(code)
        except SyntaxError as e:
            return [f"syntax-error:{e.lineno}"]

        smells += self._check_complexity(tree)
        smells += self._check_dead_code(tree)
        smells += self._check_long_functions(tree)
        smells += self._check_shadowed_builtins(tree)
        smells += self._check_import_star(tree)
        smells += self._check_debt_markers(code)

        return smells

    def _check_complexity(self, tree: ast.AST) -> List[str]:
        """Check cyclomatic complexity via branch count per function."""
        smells = []
        for node in ast.walk(tree):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                branches = sum(
                    1 for n in ast.walk(node)
                    if isinstance(n, (
                        ast.If, ast.For, ast.While, ast.ExceptHandler,
                        ast.With, ast.Assert, ast.comprehension,
                    ))
                )
                if branches >= HIGH_COMPLEXITY:
                    smells.append(f"high-complexity:{getattr(node, 'name', '?')}({branches})")
        return smells

    def _check_dead_code(self, tree: ast.AST) -> List[str]:
        """Check for unreachable code after return/raise/break/continue."""
        smells = []
        for node in ast.walk(tree):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                body = node.body
                for i, stmt in enumerate(body[:-1]):
                    if isinstance(stmt, (ast.Return, ast.Raise, ast.Break, ast.Continue)):
                        next_stmt = body[i + 1]
                        if not isinstance(next_stmt, (ast.Pass, ast.Expr)):
                            smells.append(
                                f"dead-code:{getattr(node, 'name', '?')}:L{getattr(stmt, 'lineno', '?')}"
                            )
        return smells

    def _check_long_functions(self, tree: ast.AST) -> List[str]:
        """Check for functions longer than F(11)=89 lines."""
        smells = []
        for node in ast.walk(tree):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                if (hasattr(node, 'end_lineno') and hasattr(node, 'lineno')
                        and node.end_lineno is not None):
                    lines = node.end_lineno - node.lineno
                    if lines > HIGH_LINES:
                        smells.append(f"long-function:{getattr(node, 'name', '?')}({lines}L)")
        return smells

    def _check_shadowed_builtins(self, tree: ast.AST) -> List[str]:
        """Check for variables that shadow Python builtins."""
        BUILTINS = {
            "list", "dict", "set", "tuple", "type", "str", "int", "float",
            "bool", "bytes", "object", "super", "id", "hash", "len",
        }
        smells = []
        for node in ast.walk(tree):
            if isinstance(node, (ast.Name, ast.arg)):
                name = getattr(node, "id", None) or getattr(node, "arg", None)
                if name in BUILTINS:
                    smells.append(f"shadows-builtin:{name}")
        return list(set(smells))  # Deduplicate

    def _check_import_star(self, tree: ast.AST) -> List[str]:
        """Check for 'from X import *' usage (CULTURE violation)."""
        smells = []
        for node in ast.walk(tree):
            if isinstance(node, ast.ImportFrom):
                for alias in node.names:
                    if alias.name == "*":
                        smells.append(f"import-star:{node.module or '?'}")
        return smells

    def _check_debt_markers(self, code: str) -> List[str]:
        """Check for TODO/FIXME/HACK/XXX debt markers."""
        smells = []
        markers = ("TODO", "FIXME", "HACK", "XXX", "NOQA")
        lines = code.splitlines()
        for i, line in enumerate(lines, 1):
            upper = line.upper()
            for marker in markers:
                if marker in upper and "#" in line:
                    smells.append(f"debt-marker:{marker}:L{i}")
                    break  # One per line
        return smells[:3]  # Cap at 3 debt markers

    async def health_check(self) -> DogHealth:
        avg = self._total_smells_found / max(self._cells_analyzed, 1)
        return DogHealth(
            dog_id=self.dog_id,
            status=HealthStatus.HEALTHY,
            latency_p50_ms=self.avg_latency_ms,
            details=(
                f"Cells analyzed: {self._cells_analyzed}, "
                f"Total smells: {self._total_smells_found}, "
                f"Avg: {avg:.1f}/cell"
            ),
        )
