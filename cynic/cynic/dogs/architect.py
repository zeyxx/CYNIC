"""
CYNIC Architect Dog — Netzach (Victory)

Non-LLM Dog. L3 REFLEX (<20ms AST structural analysis).
Technology: Python ast module (stdlib only, zero dependencies)

Responsibilities:
  - Code structural quality (coupling, nesting, cohesion)
  - God Class detection (too many methods)
  - God Nesting detection (too deep branching)
  - Coupling measurement (excessive imports)
  - Module balance (logic vs data vs boilerplate ratio)

Why Architect?
  Netzach = Victory through proper form. Structure defeats chaos.
  JANITOR detects smells. ARCHITECT assesses DESIGN.
  A function can pass Janitor (no smells) yet be architecturally broken.
  Architect sees the whole building; Janitor sees each room.

φ-integration:
  Thresholds are Fibonacci/Lucas-aligned:
    F(7)=13 → max imports (coupling ceiling)
    L(4)=7  → max nesting depth (flatness law)
    L(5)=11 → max methods per class (cohesion law)
    F(6)=8  → max classes per module

  Penalties per violation (additive, capped at MAX_Q_SCORE):
    IMPORT_PENALTY  = 3.0 pts/excess import
    NESTING_PENALTY = 5.0 pts/excess level
    METHOD_PENALTY  = 2.0 pts/excess method
    CLASS_PENALTY   = 3.0 pts/excess class

  confidence = 0.500 (midpoint — structure matters but context-dependent)
  VETO: impossible — structural issues are advisory, not blocking
"""
from __future__ import annotations

import ast
import logging
import time
from typing import Any, Dict, List, Optional, Tuple

from cynic.core.phi import PHI_INV, PHI_INV_2, MAX_Q_SCORE, phi_bound_score
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell
from cynic.dogs.base import (
    AbstractDog, DogCapabilities, DogHealth, DogJudgment,
    DogId, HealthStatus,
)

logger = logging.getLogger("cynic.dogs.architect")

# ── Structural Thresholds (Fibonacci/Lucas-aligned) ──────────────────────────
MAX_IMPORTS: int = 13       # F(7) — max imports before coupling penalty
MAX_NESTING: int = 7        # L(4) — max nesting depth (flat > nested)
MAX_CLASS_METHODS: int = 11  # L(5) — max methods per class
MAX_CLASSES: int = 8         # F(6) — max classes per module

# ── Penalties per excess unit ──────────────────────────────────────────────
IMPORT_PENALTY: float = 3.0     # per excess import beyond MAX_IMPORTS
NESTING_PENALTY: float = 5.0    # per excess nesting level beyond MAX_NESTING
METHOD_PENALTY: float = 2.0     # per excess method beyond MAX_CLASS_METHODS
CLASS_PENALTY: float = 3.0      # per excess class beyond MAX_CLASSES

# Confidence: midpoint between PHI_INV_2 (0.382) and PHI_INV (0.618)
# Structure is important but intent matters — not as certain as JANITOR
ARCHITECT_CONFIDENCE: float = 0.500


class ArchitectDog(AbstractDog):
    """
    Architect (Netzach) — AST-based code structure analyzer.

    Analyzes CODE cells for structural quality:
      1. Coupling: import count (> F(7)=13 → over-coupled)
      2. Nesting: max depth (> L(4)=7 → god nesting)
      3. Cohesion: methods per class (> L(5)=11 → god class)
      4. Balance: classes per module (> F(6)=8 → structural sprawl)

    Difference from JANITOR:
      JANITOR: function-level smells (complexity, dead code, long functions)
      ARCHITECT: module-level structure (coupling, nesting, cohesion, balance)
    """

    def __init__(self) -> None:
        super().__init__(DogId.ARCHITECT)
        self._cells_analyzed: int = 0
        self._violations_total: int = 0

    def get_capabilities(self) -> DogCapabilities:
        return DogCapabilities(
            dog_id=DogId.ARCHITECT,
            sefirot="Netzach — Victory",
            consciousness_min=ConsciousnessLevel.REFLEX,
            uses_llm=False,
            supported_realities={"CODE", "CYNIC"},
            supported_analyses={"PERCEIVE", "JUDGE", "ACCOUNT"},
            technology="Python ast (stdlib)",
            max_concurrent=8,  # F(6) — pure CPU, parallelizable
        )

    async def analyze(self, cell: Cell, **kwargs: Any) -> DogJudgment:
        """
        Analyze code structure via AST.

        Extracts code string → parses AST → scores structural metrics.
        Falls back to cell metadata if no code extractable.
        """
        start = time.perf_counter()

        code_str = self._extract_code(cell)
        violations: list[str] = []
        total_penalty: float = 0.0
        evidence: dict[str, Any] = {}

        if code_str:
            violations, total_penalty, evidence = self._assess_structure(code_str)
        else:
            # Metadata fallback: use novelty/complexity/risk proxy
            if cell.complexity > PHI_INV:
                violations.append("high-complexity-metadata")
                total_penalty += 5.0
            evidence = {"fallback": "no_code_extracted", "cell_complexity": cell.complexity}

        self._cells_analyzed += 1
        self._violations_total += len(violations)

        # Q-Score: start at MAX, decay by total penalties
        q_raw = MAX_Q_SCORE - total_penalty
        q_score = phi_bound_score(max(0.0, q_raw))

        # Reasoning
        if violations:
            top = violations[:5]
            reasoning = f"Structural issues ({len(violations)}): {', '.join(top)}"
        else:
            reasoning = "*ears perk* Clean structure — Netzach approves"

        latency = (time.perf_counter() - start) * 1000
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=q_score,
            confidence=ARCHITECT_CONFIDENCE,
            reasoning=reasoning,
            evidence={**evidence, "violations": violations[:8], "total_penalty": total_penalty},
            latency_ms=latency,
            veto=False,  # Structural issues are advisory
        )
        self.record_judgment(judgment)
        return judgment

    # ── Code Extraction ────────────────────────────────────────────────────

    def _extract_code(self, cell: Cell) -> str | None:
        """Extract Python code string from cell content (same pattern as JANITOR)."""
        content = cell.content

        if isinstance(content, str) and content.strip():
            return content

        if isinstance(content, dict):
            for key in ("code", "source", "content", "diff", "text"):
                val = content.get(key)
                if isinstance(val, str) and val.strip():
                    return val

        return None

    # ── Structural Assessment ──────────────────────────────────────────────

    def _assess_structure(self, code: str) -> tuple[list[str], float, dict[str, Any]]:
        """
        Parse code and assess 4 structural dimensions.

        Returns: (violations, total_penalty, evidence)
        """
        violations: list[str] = []
        total_penalty: float = 0.0

        try:
            tree = ast.parse(code)
        except SyntaxError as e:
            return [f"syntax-error:L{e.lineno}"], MAX_Q_SCORE, {"parse_error": str(e)}

        # Dimension 1: Coupling (import count)
        import_violations, import_penalty, import_count = self._check_coupling(tree)
        violations += import_violations
        total_penalty += import_penalty

        # Dimension 2: God Nesting (max depth)
        nesting_violations, nesting_penalty, max_depth = self._check_nesting(tree)
        violations += nesting_violations
        total_penalty += nesting_penalty

        # Dimension 3: God Class (methods per class)
        class_violations, class_penalty, class_stats = self._check_class_cohesion(tree)
        violations += class_violations
        total_penalty += class_penalty

        # Dimension 4: Module Balance (class count)
        balance_violations, balance_penalty, class_count = self._check_module_balance(tree)
        violations += balance_violations
        total_penalty += balance_penalty

        evidence = {
            "import_count": import_count,
            "max_nesting_depth": max_depth,
            "class_stats": class_stats,
            "class_count": class_count,
            "code_lines": len(code.splitlines()),
        }
        return violations, total_penalty, evidence

    def _check_coupling(self, tree: ast.AST) -> tuple[list[str], float, int]:
        """Count imports. > F(7)=13 → coupling violation."""
        violations: list[str] = []
        import_names: list[str] = []

        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    import_names.append(alias.name)
            elif isinstance(node, ast.ImportFrom):
                import_names.append(node.module or "?")

        count = len(import_names)
        excess = max(0, count - MAX_IMPORTS)
        if excess > 0:
            violations.append(f"high-coupling:{count}-imports(>{MAX_IMPORTS})")

        penalty = excess * IMPORT_PENALTY
        return violations, penalty, count

    def _check_nesting(self, tree: ast.AST) -> tuple[list[str], float, int]:
        """Measure max nesting depth. > L(4)=7 → god nesting."""
        violations: list[str] = []
        max_depth = 0

        def _depth(node: ast.AST, current: int) -> None:
            nonlocal max_depth
            if current > max_depth:
                max_depth = current
            NESTING_NODES = (ast.If, ast.For, ast.While, ast.With, ast.Try,
                             ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef,
                             ast.ExceptHandler)
            for child in ast.iter_child_nodes(node):
                if isinstance(child, NESTING_NODES):
                    _depth(child, current + 1)
                else:
                    _depth(child, current)

        _depth(tree, 0)

        excess = max(0, max_depth - MAX_NESTING)
        if excess > 0:
            violations.append(f"god-nesting:depth={max_depth}(>{MAX_NESTING})")

        penalty = excess * NESTING_PENALTY
        return violations, penalty, max_depth

    def _check_class_cohesion(self, tree: ast.AST) -> tuple[list[str], float, list[dict]]:
        """Count methods per class. > L(5)=11 → god class."""
        violations: list[str] = []
        class_stats: list[dict] = []
        total_penalty = 0.0

        for node in ast.walk(tree):
            if isinstance(node, ast.ClassDef):
                methods = [
                    n for n in ast.iter_child_nodes(node)
                    if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef))
                ]
                method_count = len(methods)
                class_stats.append({"name": node.name, "methods": method_count})

                excess = max(0, method_count - MAX_CLASS_METHODS)
                if excess > 0:
                    violations.append(f"god-class:{node.name}({method_count}methods)")
                    total_penalty += excess * METHOD_PENALTY

        return violations, total_penalty, class_stats

    def _check_module_balance(self, tree: ast.AST) -> tuple[list[str], float, int]:
        """Count top-level classes. > F(6)=8 → structural sprawl."""
        violations: list[str] = []
        class_count = sum(
            1 for node in ast.iter_child_nodes(tree)
            if isinstance(node, ast.ClassDef)
        )

        excess = max(0, class_count - MAX_CLASSES)
        if excess > 0:
            violations.append(f"structural-sprawl:{class_count}-classes(>{MAX_CLASSES})")

        penalty = excess * CLASS_PENALTY
        return violations, penalty, class_count

    async def health_check(self) -> DogHealth:
        avg = self._violations_total / max(self._cells_analyzed, 1)
        return DogHealth(
            dog_id=self.dog_id,
            status=HealthStatus.HEALTHY,
            latency_p50_ms=self.avg_latency_ms,
            details=(
                f"Cells analyzed: {self._cells_analyzed}, "
                f"Total violations: {self._violations_total}, "
                f"Avg: {avg:.1f}/cell"
            ),
        )
