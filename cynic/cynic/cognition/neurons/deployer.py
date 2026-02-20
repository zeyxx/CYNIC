"""
CYNIC Deployer Dog — Hod (Splendor)

Hod = the left pillar of the Tree of Life, opposing Netzach (Victory).
"Splendor" — the glory of precise execution, disciplined communication.
DEPLOYER judges deployment readiness: is this code safe to ship?

Technology: Python ast (stdlib) + re (stdlib) — zero external dependencies.
Heuristic path (Phase 1). LLM path (Phase 2) will add semantic analysis.

Responsibilities:
  - Detect debug artifacts: print(), pdb.set_trace(), breakpoint()
  - Detect hardcoded secrets: password=, api_key=, secret= string literals
  - Detect deployment blockers: TODO/FIXME/HACK/XXX comments
  - Detect stubs: raise NotImplementedError (unfinished code shipped)
  - Detect sys.exit() outside __main__ blocks (library anti-pattern)
  - Detect missing __main__ guard in executable scripts

Why DEPLOYER?
  Hod = communication, precise form, Mercury's discipline.
  Before code ships, it must be honest about what it is.
  Debug artifacts are lies — they say "this is production" while
  whispering "I was hacked together at 3am."
  DEPLOYER calls the bluff.

φ-integration:
  F(5)=5  → max debug statements before penalty (generous — debugging is real)
  F(3)=2  → max hardcoded secrets (2 is already 1 too many, but code has context)
  F(4)=3  → max TODO/FIXME markers in production code
  F(3)=2  → max NotImplementedError stubs

  Penalties per excess unit (deployment failures are expensive):
    DEBUG_PENALTY  = 6.0   — debug artifacts are prod liabilities
    SECRET_PENALTY = 12.0  — hardcoded secrets are catastrophic
    TODO_PENALTY   = 2.0   — mild: todos are known debt
    STUB_PENALTY   = 8.0   — stubs in prod are broken contracts
    SYS_EXIT_PENALTY = 3.0 — library exit calls are subtle bugs

  Confidence: 0.500 — structural checks solid, semantic context missing
  VETO: impossible — advisory only (GUARDIAN handles hard blocks)
"""
from __future__ import annotations

import ast
import logging
import re
import time
from typing import Any


from cynic.core.phi import (
    PHI_INV, PHI_INV_2, MAX_Q_SCORE, MAX_CONFIDENCE,
    phi_bound_score, fibonacci,
)
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell
from cynic.cognition.neurons.base import (
    AbstractDog, LLMDog, DogCapabilities, DogHealth, DogJudgment,
    DogId, HealthStatus,
)

logger = logging.getLogger("cynic.cognition.neurons.deployer")

# ── Thresholds (Fibonacci-aligned) ────────────────────────────────────────
MAX_DEBUG_ARTIFACTS: int  = fibonacci(5)   # F(5)=5 — max print/pdb/breakpoint
MAX_SECRETS: int          = fibonacci(3)   # F(3)=2 — max hardcoded secret patterns
MAX_TODOS: int            = fibonacci(4)   # F(4)=3 — max TODO/FIXME/HACK/XXX
MAX_STUBS: int            = fibonacci(3)   # F(3)=2 — max NotImplementedError stubs

BASE_Q: float = MAX_Q_SCORE

# ── Penalties ─────────────────────────────────────────────────────────────
DEBUG_PENALTY: float   = 6.0   # per debug artifact beyond threshold
SECRET_PENALTY: float  = 12.0  # per hardcoded secret (catastrophic)
TODO_PENALTY: float    = 2.0   # per TODO/FIXME/HACK/XXX beyond threshold
STUB_PENALTY: float    = 8.0   # per NotImplementedError stub beyond threshold
SYS_EXIT_PENALTY: float = 3.0  # per sys.exit() outside __main__

DEPLOYER_CONFIDENCE: float = 0.500  # midpoint — structural but context-dependent

# ── Regex patterns ────────────────────────────────────────────────────────
# Debug artifact patterns (line-level)
_DEBUG_PATTERNS = [
    re.compile(r'\bprint\s*\(', re.IGNORECASE),           # print() calls
    re.compile(r'\bpdb\.set_trace\s*\('),                  # pdb debugger
    re.compile(r'\bbreakpoint\s*\('),                      # Python 3.7+ breakpoint()
    re.compile(r'\bipdb\.set_trace\s*\('),                 # ipdb
    re.compile(r'\bpudb\.set_trace\s*\('),                 # pudb
]

# Hardcoded secret patterns (very conservative — only obvious literals)
_SECRET_PATTERNS = [
    re.compile(r'(?:password|passwd|pwd)\s*=\s*["\'][^"\']{4,}["\']', re.IGNORECASE),
    re.compile(r'(?:api_key|apikey|secret_key|secret)\s*=\s*["\'][^"\']{8,}["\']', re.IGNORECASE),
    re.compile(r'(?:token|auth_token|access_token)\s*=\s*["\'][^"\']{8,}["\']', re.IGNORECASE),
    re.compile(r'(?:private_key|priv_key)\s*=\s*["\'][^"\']{4,}["\']', re.IGNORECASE),
]

# Deployment blocker comments
_TODO_PATTERNS = [
    re.compile(r'#\s*(?:TODO|FIXME|HACK|XXX|BUG|TEMP)\b', re.IGNORECASE),
]

# sys.exit outside __main__
_SYS_EXIT_PATTERN = re.compile(r'\bsys\.exit\s*\(')

# __main__ guard
_MAIN_GUARD_PATTERN = re.compile(r'if\s+__name__\s*==\s*["\']__main__["\']')


class DeployerDog(LLMDog):
    """
    Deployer (Hod) — deployment readiness scorer.

    Heuristic path: regex + AST scan (debug artifacts, secrets, stubs).
    Temporal MCTS path (when LLM available): 7-perspective temporal judgment.

    Scans code for:
      - Debug leftovers (print, pdb, breakpoint)
      - Hardcoded secrets (password=, api_key=)
      - Known debt markers (TODO, FIXME, HACK)
      - Broken contracts (NotImplementedError stubs)
      - Library anti-patterns (sys.exit outside __main__)
    """

    DOG_ID = DogId.DEPLOYER

    def __init__(self) -> None:
        super().__init__(DogId.DEPLOYER, task_type="deployment")
        self._secrets_found: int = 0
        self._stubs_found: int = 0
        self._debug_found: int = 0

    def get_capabilities(self) -> DogCapabilities:
        return DogCapabilities(
            dog_id=DogId.DEPLOYER,
            sefirot="Hod — Splendor (execution discipline)",
            consciousness_min=ConsciousnessLevel.MACRO,
            uses_llm=True,
            supported_realities={"CODE", "CYNIC", "SOLANA"},
            supported_analyses={"JUDGE", "DECIDE", "ACT"},
            technology="Python ast + re (stdlib)",
            max_concurrent=8,
        )

    async def analyze(self, cell: Cell, **kwargs: Any) -> DogJudgment:
        """
        Route to temporal MCTS path (LLM available) or heuristic scan path.
        """
        start = time.perf_counter()
        adapter = await self.get_llm()
        if adapter is not None:
            return await self._temporal_path(cell, adapter, start)
        return await self._heuristic_path(cell, start)

    async def _temporal_path(
        self,
        cell: Cell,
        adapter: Any,
        start: float,
    ) -> DogJudgment:
        """7-perspective temporal MCTS judgment via Ollama."""
        from cynic.llm.temporal import temporal_judgment

        code_str = self._extract_code(cell)
        content = code_str[:2000] if code_str else (
            f"reality:{cell.reality} risk:{cell.risk:.2f} novelty:{cell.novelty:.2f}"
        )
        tj = await temporal_judgment(adapter, content)

        latency = (time.perf_counter() - start) * 1000
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=tj.phi_aggregate,
            confidence=tj.confidence,
            reasoning=(
                f"*sniff* Deployer temporal MCTS: Q={tj.phi_aggregate:.1f} "
                f"— deploy-readiness from 7 perspectives"
            ),
            evidence=tj.to_dict(),
            latency_ms=latency,
            llm_id=tj.llm_id,
            veto=False,
        )
        self.record_judgment(judgment)
        return judgment

    async def _heuristic_path(self, cell: Cell, start: float) -> DogJudgment:
        """Regex + AST scan for deployment anti-patterns."""
        code_str = self._extract_code(cell)
        violations: list[str] = []
        total_penalty: float = 0.0
        evidence: dict[str, Any] = {}

        if code_str:
            violations, total_penalty, evidence = self._score_deployability(code_str)
        else:
            violations, total_penalty, evidence = self._metadata_fallback(cell)

        q_raw = BASE_Q - total_penalty
        q_score = phi_bound_score(max(0.0, q_raw))

        if violations:
            top = violations[:3]
            reasoning = f"*GROWL* Deploy risk: {len(violations)} issue(s): {', '.join(top)}"
        else:
            reasoning = "*tail wag* Deploy-ready — Hod sees clean execution path"

        latency = (time.perf_counter() - start) * 1000
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=q_score,
            confidence=DEPLOYER_CONFIDENCE,
            reasoning=reasoning,
            evidence={
                **evidence,
                "violations": violations[:8],
                "total_penalty": round(total_penalty, 2),
            },
            latency_ms=latency,
            veto=False,
        )
        self.record_judgment(judgment)
        return judgment

    # ── Code Extraction ────────────────────────────────────────────────────

    def _extract_code(self, cell: Cell) -> str | None:
        """Extract code string from cell content."""
        content = cell.content
        if isinstance(content, str) and content.strip():
            return content
        if isinstance(content, dict):
            for key in ("code", "source", "content", "text"):
                val = content.get(key)
                if isinstance(val, str) and val.strip():
                    return val
        return None

    # ── Deployment Readiness Scoring ───────────────────────────────────────

    def _score_deployability(
        self,
        code: str,
    ) -> tuple[list[str], float, dict[str, Any]]:
        """
        Scan code for deployment anti-patterns.

        Returns: (violations, total_penalty, evidence)
        """
        violations: list[str] = []
        total_penalty: float = 0.0

        lines = code.splitlines()

        # ── Check 1: Debug artifacts ──────────────────────────────────────
        debug_hits = self._find_debug_artifacts(lines)
        self._debug_found += len(debug_hits)
        excess_debug = max(0, len(debug_hits) - MAX_DEBUG_ARTIFACTS)
        if excess_debug > 0:
            violations.append(
                f"debug-artifacts:{len(debug_hits)}(>{MAX_DEBUG_ARTIFACTS})"
            )
            total_penalty += excess_debug * DEBUG_PENALTY
        # Any pdb/breakpoint is always flagged regardless of threshold
        hard_debug = [h for h in debug_hits if "pdb" in h or "breakpoint" in h or "ipdb" in h]
        if hard_debug:
            violations.append(f"debugger-active:{len(hard_debug)}")
            total_penalty += len(hard_debug) * DEBUG_PENALTY

        # ── Check 2: Hardcoded secrets ────────────────────────────────────
        secret_hits = self._find_secrets(lines)
        self._secrets_found += len(secret_hits)
        excess_secrets = max(0, len(secret_hits) - MAX_SECRETS)
        if excess_secrets > 0:
            violations.append(f"hardcoded-secrets:{len(secret_hits)}(>{MAX_SECRETS})")
            total_penalty += excess_secrets * SECRET_PENALTY
        elif secret_hits:
            # Under threshold but non-zero — still penalize
            violations.append(f"hardcoded-secrets:{len(secret_hits)}")
            total_penalty += len(secret_hits) * SECRET_PENALTY

        # ── Check 3: TODO/FIXME/HACK markers ─────────────────────────────
        todo_hits = self._find_todos(lines)
        excess_todos = max(0, len(todo_hits) - MAX_TODOS)
        if excess_todos > 0:
            violations.append(f"known-debt:{len(todo_hits)}-markers(>{MAX_TODOS})")
            total_penalty += excess_todos * TODO_PENALTY

        # ── Check 4: NotImplementedError stubs ────────────────────────────
        stub_hits = self._find_stubs(code)
        self._stubs_found += len(stub_hits)
        excess_stubs = max(0, len(stub_hits) - MAX_STUBS)
        if excess_stubs > 0:
            violations.append(f"unfinished-stubs:{len(stub_hits)}(>{MAX_STUBS})")
            total_penalty += excess_stubs * STUB_PENALTY

        # ── Check 5: sys.exit outside __main__ ────────────────────────────
        sys_exit_count = self._find_sys_exit_outside_main(code)
        if sys_exit_count > 0:
            violations.append(f"sys-exit-outside-main:{sys_exit_count}")
            total_penalty += sys_exit_count * SYS_EXIT_PENALTY

        # ── Check 6: Missing __main__ guard (if script-like) ──────────────
        has_main_guard = bool(_MAIN_GUARD_PATTERN.search(code))
        has_sys_exit = bool(_SYS_EXIT_PATTERN.search(code))
        if has_sys_exit and not has_main_guard:
            violations.append("missing-main-guard")
            total_penalty += 4.0

        evidence = {
            "debug_artifacts": len(debug_hits),
            "hardcoded_secrets": len(secret_hits),
            "todo_markers": len(todo_hits),
            "stubs": len(stub_hits),
            "sys_exit_outside_main": sys_exit_count,
            "has_main_guard": has_main_guard,
            "lines": len(lines),
        }
        return violations, total_penalty, evidence

    # ── Pattern Finders ────────────────────────────────────────────────────

    def _find_debug_artifacts(self, lines: list[str]) -> list[str]:
        """Find debug statements in code lines."""
        hits: list[str] = []
        for line in lines:
            stripped = line.strip()
            if stripped.startswith("#"):
                continue  # skip comments
            for pat in _DEBUG_PATTERNS:
                if pat.search(stripped):
                    hits.append(stripped[:60])
                    break  # one hit per line
        return hits

    def _find_secrets(self, lines: list[str]) -> list[str]:
        """Find hardcoded secret patterns."""
        hits: list[str] = []
        for line in lines:
            stripped = line.strip()
            if stripped.startswith("#"):
                continue
            for pat in _SECRET_PATTERNS:
                if pat.search(stripped):
                    # Redact the actual value in the hit
                    hits.append(re.sub(r'=\s*["\'][^"\']*["\']', '=<REDACTED>', stripped[:80]))
                    break
        return hits

    def _find_todos(self, lines: list[str]) -> list[str]:
        """Find TODO/FIXME/HACK/XXX comment markers."""
        hits: list[str] = []
        for line in lines:
            for pat in _TODO_PATTERNS:
                if pat.search(line):
                    hits.append(line.strip()[:60])
                    break
        return hits

    def _find_stubs(self, code: str) -> list[str]:
        """Find NotImplementedError raises via AST."""
        hits: list[str] = []
        try:
            tree = ast.parse(code)
            for node in ast.walk(tree):
                if isinstance(node, ast.Raise):
                    exc = node.exc
                    if exc is None:
                        continue
                    if isinstance(exc, ast.Call):
                        func = exc.func
                    elif isinstance(exc, ast.Name):
                        func = exc
                    else:
                        continue
                    name = func.id if isinstance(func, ast.Name) else (
                        func.attr if isinstance(func, ast.Attribute) else None
                    )
                    if name == "NotImplementedError":
                        hits.append(f"line:{getattr(node, 'lineno', '?')}")
        except SyntaxError:
            pass
        return hits

    def _find_sys_exit_outside_main(self, code: str) -> int:
        """
        Count sys.exit() calls that are NOT inside a __main__ block.

        Conservative: if there's no main guard at all and sys.exit exists,
        count all sys.exit calls. If main guard exists, count 0 (we trust placement).
        """
        if _MAIN_GUARD_PATTERN.search(code):
            return 0  # Trust the developer's __main__ block structure
        return len(_SYS_EXIT_PATTERN.findall(code))

    # ── Metadata Fallback ─────────────────────────────────────────────────

    def _metadata_fallback(self, cell: Cell) -> tuple[list[str], float, dict[str, Any]]:
        """Score non-CODE cells via cell metadata (risk)."""
        violations: list[str] = []
        penalty = 0.0

        # High risk → not deploy-ready
        if cell.risk > PHI_INV:
            violations.append(f"high-risk-cell:{cell.risk:.2f}")
            penalty += cell.risk * 8.0

        # High novelty → uncertain deployment territory
        if cell.novelty > PHI_INV:
            violations.append(f"high-novelty-deployment:{cell.novelty:.2f}")
            penalty += cell.novelty * 4.0

        evidence = {
            "fallback": "no-code-extracted",
            "cell_risk": cell.risk,
            "cell_novelty": cell.novelty,
            "cell_reality": cell.reality,
        }
        return violations, penalty, evidence

    # ── Health ────────────────────────────────────────────────────────────

    async def health_check(self) -> DogHealth:
        total = self._judgment_count
        status = (
            HealthStatus.HEALTHY if total > 0 or self._active
            else HealthStatus.UNKNOWN
        )
        return DogHealth(
            dog_id=self.dog_id,
            status=status,
            latency_p50_ms=self.avg_latency_ms,
            details=(
                f"Debug found: {self._debug_found}, "
                f"Secrets found: {self._secrets_found}, "
                f"Stubs found: {self._stubs_found}"
            ),
        )
