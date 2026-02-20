"""
CYNIC Cartographer Dog — Daat (Knowledge)

Daat = the hidden sefirot, the synthesis of Chokmah and Binah.
"Knowledge" — not wisdom alone, not understanding alone, but both united.

CARTOGRAPHER maps the topology of code. Where ARCHITECT sees structure
(coupling, nesting, cohesion within a module), CARTOGRAPHER sees the
GRAPH — what calls what, what imports what, how nodes relate across
the whole topology.

Technology: Python ast (stdlib) + NetworkX 3.x (graph analysis)

Responsibilities:
  - Build a directed dependency graph from code AST:
      import graph: module → dependency edges
      call graph:   caller → callee edges
      class graph:  class → base class edges (inheritance)
  - Score graph health via 4 topology metrics:
      DENSITY:    How tightly packed? High density → brittle
      CENTRALITY: Bottleneck nodes? High betweenness → fragile
      CYCLES:     Circular dependencies? → BARK
      ISOLATION:  Unreachable nodes? → dead code

Why CARTOGRAPHER?
  Daat is hidden — it doesn't appear on the Tree of Life by itself.
  It exists where Chokmah and Binah meet. Similarly, CARTOGRAPHER
  sees connections that no single-module analyzer can see.
  "The map is not the territory — but without a map, you're lost."

φ-integration:
  Graph thresholds use Fibonacci/Lucas:
    F(7)=13  → max edges before density concern
    F(6)=8   → max in-degree for a single node (bottleneck threshold)
    F(5)=5   → min cycle length to flag
    L(4)=7   → max strongly connected components before fragmentation concern

  Scoring: BASE_Q - penalties per violation (capped at [0, MAX_Q_SCORE])
  Confidence: 0.450 — graphs reveal structure but miss semantics
  VETO: impossible — topology is advisory
"""
from __future__ import annotations

import ast
import logging
import time
from dataclasses import dataclass, field
from typing import Any


import networkx as nx

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

logger = logging.getLogger("cynic.cognition.neurons.cartographer")

# ── Graph Thresholds (φ-aligned) ──────────────────────────────────────────

MAX_DENSITY_EDGES: int  = fibonacci(7)   # F(7)=13  — above this, graph is dense
MAX_IN_DEGREE: int      = fibonacci(6)   # F(6)=8   — bottleneck threshold
MAX_SCC_COUNT: int      = fibonacci(4)   # F(4)=3   — max SCCs before fragmentation
BASE_Q: float           = MAX_Q_SCORE    # 61.8 — start perfect, subtract penalties

# ── Penalties ─────────────────────────────────────────────────────────────
DENSITY_PENALTY: float     = 4.0   # per excess edge beyond MAX_DENSITY_EDGES
BOTTLENECK_PENALTY: float  = 5.0   # per node exceeding MAX_IN_DEGREE
CYCLE_PENALTY: float       = 8.0   # per cycle detected (circular dep = bad)
ISOLATION_PENALTY: float   = 2.0   # per isolated node (dead code)
SCC_PENALTY: float         = 3.0   # per fragmented component beyond MAX_SCC_COUNT

CARTOGRAPHER_CONFIDENCE: float = 0.450  # Between PHI_INV_2 and PHI_INV


@dataclass
class GraphSnapshot:
    """Extracted graph data from AST analysis."""
    imports: list[str] = field(default_factory=list)      # modules imported
    functions: list[str] = field(default_factory=list)    # function names
    classes: list[str] = field(default_factory=list)      # class names
    calls: list[tuple[str, str]] = field(default_factory=list)   # (caller, callee)
    bases: list[tuple[str, str]] = field(default_factory=list)   # (class, base)
    import_edges: list[tuple[str, str]] = field(default_factory=list)  # (module, dep)


class CartographerDog(LLMDog):
    """
    Cartographer (Daat) — NetworkX topology mapper.

    Heuristic path: AST + NetworkX graph scoring (density, centrality, cycles).
    Temporal MCTS path (when LLM available): 7-perspective temporal judgment.

    Graph dimensions scored:
      - DENSITY:    Edge count vs node count ratio
      - CENTRALITY: Bottleneck nodes with high in-degree
      - CYCLES:     Circular dependency detection
      - ISOLATION:  Unreachable / disconnected nodes
    """

    DOG_ID = DogId.CARTOGRAPHER

    def __init__(self) -> None:
        super().__init__(DogId.CARTOGRAPHER, task_type="topology")
        self._graphs_built: int = 0
        self._cycles_found: int = 0
        self._fallback_count: int = 0

    def get_capabilities(self) -> DogCapabilities:
        return DogCapabilities(
            dog_id=DogId.CARTOGRAPHER,
            sefirot="Daat — Knowledge (hidden sefirot)",
            consciousness_min=ConsciousnessLevel.MACRO,
            uses_llm=True,
            supported_realities={"CODE", "CYNIC"},
            supported_analyses={"PERCEIVE", "JUDGE", "EMERGE"},
            technology="NetworkX 3.x + Python ast (stdlib)",
            max_concurrent=4,
        )

    async def analyze(self, cell: Cell, **kwargs: Any) -> DogJudgment:
        """
        Route to temporal MCTS path (LLM available) or NetworkX heuristic path.
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
            f"reality:{cell.reality} analysis:{cell.analysis} "
            f"complexity:{cell.complexity:.2f} novelty:{cell.novelty:.2f}"
        )
        tj = await temporal_judgment(adapter, content)

        latency = (time.perf_counter() - start) * 1000
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=tj.phi_aggregate,
            confidence=tj.confidence,
            reasoning=(
                f"*sniff* Cartographer temporal MCTS: Q={tj.phi_aggregate:.1f} "
                f"from 7 topology perspectives"
            ),
            evidence=tj.to_dict(),
            latency_ms=latency,
            llm_id=tj.llm_id,
            veto=False,
        )
        self.record_judgment(judgment)
        return judgment

    async def _heuristic_path(self, cell: Cell, start: float) -> DogJudgment:
        """AST + NetworkX graph topology scoring."""
        code_str = self._extract_code(cell)
        violations: list[str] = []
        total_penalty: float = 0.0
        evidence: dict[str, Any] = {}

        if code_str:
            snapshot = self._parse_graph(code_str)
            if snapshot is not None:
                G = self._build_graph(snapshot)
                violations, total_penalty, evidence = self._score_graph(G, snapshot)
                self._graphs_built += 1
            else:
                # Syntax error path
                violations = ["unparseable-code"]
                total_penalty = 10.0
                evidence = {"parse_error": True}
                self._fallback_count += 1
        else:
            # No code — score via cell metadata
            violations, total_penalty, evidence = self._metadata_fallback(cell)
            self._fallback_count += 1

        q_raw = BASE_Q - total_penalty
        q_score = phi_bound_score(max(0.0, q_raw))

        if violations:
            top = violations[:4]
            reasoning = f"*sniff* Topology: {len(violations)} issue(s): {', '.join(top)}"
        else:
            reasoning = "*ears perk* Clean topology — Daat sees clear paths"

        latency = (time.perf_counter() - start) * 1000
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=q_score,
            confidence=CARTOGRAPHER_CONFIDENCE,
            reasoning=reasoning,
            evidence={**evidence, "violations": violations[:8], "total_penalty": round(total_penalty, 2)},
            latency_ms=latency,
            veto=False,
        )
        self.record_judgment(judgment)
        return judgment

    # ── Code Extraction ────────────────────────────────────────────────────

    def _extract_code(self, cell: Cell) -> str | None:
        """Extract Python code string from cell content."""
        content = cell.content
        if isinstance(content, str) and content.strip():
            return content
        if isinstance(content, dict):
            for key in ("code", "source", "content", "text"):
                val = content.get(key)
                if isinstance(val, str) and val.strip():
                    return val
        return None

    # ── AST → GraphSnapshot ────────────────────────────────────────────────

    def _parse_graph(self, code: str) -> GraphSnapshot | None:
        """
        Parse Python code into a GraphSnapshot.

        Returns None if the code has a syntax error.
        """
        try:
            tree = ast.parse(code)
        except SyntaxError:
            return None

        snap = GraphSnapshot()
        self._extract_imports(tree, snap)
        self._extract_definitions(tree, snap)
        self._extract_calls(tree, snap)
        return snap

    def _extract_imports(self, tree: ast.AST, snap: GraphSnapshot) -> None:
        """Extract all import statements as graph edges (module → dep)."""
        module_name = "__module__"  # Synthetic root node for this file

        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    dep = alias.name.split(".")[0]  # top-level package
                    snap.imports.append(dep)
                    snap.import_edges.append((module_name, dep))
            elif isinstance(node, ast.ImportFrom):
                if node.module:
                    dep = node.module.split(".")[0]
                    snap.imports.append(dep)
                    snap.import_edges.append((module_name, dep))

    def _extract_definitions(self, tree: ast.AST, snap: GraphSnapshot) -> None:
        """Extract function names, class names, and inheritance edges."""
        for node in ast.walk(tree):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                snap.functions.append(node.name)
            elif isinstance(node, ast.ClassDef):
                snap.classes.append(node.name)
                for base in node.bases:
                    base_name = self._name_from_expr(base)
                    if base_name:
                        snap.bases.append((node.name, base_name))

    def _extract_calls(self, tree: ast.AST, snap: GraphSnapshot) -> None:
        """
        Extract function call edges: (caller_function, callee_function).

        Only tracks calls where the callee name matches a known function
        in this module (cross-module calls become edges to external nodes).
        """
        known_funcs: set[str] = set(snap.functions)

        # Walk function bodies to find calls
        for node in ast.walk(tree):
            if not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                continue
            caller = node.name
            for child in ast.walk(node):
                if isinstance(child, ast.Call):
                    callee = self._name_from_expr(child.func)
                    if callee and callee != caller:
                        snap.calls.append((caller, callee))

    def _name_from_expr(self, expr: ast.expr) -> str | None:
        """Extract a simple name from a call expression."""
        if isinstance(expr, ast.Name):
            return expr.id
        if isinstance(expr, ast.Attribute):
            return expr.attr
        return None

    # ── Graph Construction ─────────────────────────────────────────────────

    def _build_graph(self, snap: GraphSnapshot) -> nx.DiGraph:
        """Build a directed graph from the GraphSnapshot."""
        G: nx.DiGraph = nx.DiGraph()

        # Add all known definitions as nodes
        for fn in snap.functions:
            G.add_node(fn, kind="function")
        for cls in snap.classes:
            G.add_node(cls, kind="class")

        # Import edges (module → dependency)
        for src, dst in snap.import_edges:
            G.add_edge(src, dst, kind="import")

        # Call edges (caller → callee)
        for caller, callee in snap.calls:
            G.add_edge(caller, callee, kind="call")

        # Inheritance edges (class → base)
        for cls, base in snap.bases:
            G.add_edge(cls, base, kind="inherits")

        return G

    # ── Graph Scoring ──────────────────────────────────────────────────────

    def _score_graph(
        self,
        G: nx.DiGraph,
        snap: GraphSnapshot,
    ) -> tuple[list[str], float, dict[str, Any]]:
        """
        Score the dependency graph on 4 topology dimensions.

        Returns: (violations, total_penalty, evidence)
        """
        violations: list[str] = []
        total_penalty: float = 0.0

        node_count = G.number_of_nodes()
        edge_count = G.number_of_edges()

        if node_count == 0:
            # Empty graph — neutral score
            return [], 0.0, {
                "nodes": 0, "edges": 0,
                "imports": len(snap.imports),
                "functions": len(snap.functions),
                "classes": len(snap.classes),
                "note": "no-graph-structure",
            }

        # ── Dimension 1: Density ─────────────────────────────────────────
        excess_edges = max(0, edge_count - MAX_DENSITY_EDGES)
        if excess_edges > 0:
            violations.append(f"high-density:{edge_count}-edges(>{MAX_DENSITY_EDGES})")
            total_penalty += excess_edges * DENSITY_PENALTY

        # ── Dimension 2: Bottleneck nodes (high in-degree) ───────────────
        bottlenecks: list[str] = []
        for node, in_deg in G.in_degree():
            if in_deg > MAX_IN_DEGREE:
                bottlenecks.append(f"{node}({in_deg})")
                total_penalty += BOTTLENECK_PENALTY
        if bottlenecks:
            violations.append(f"bottleneck-nodes:{','.join(bottlenecks[:3])}")

        # ── Dimension 3: Cycles (circular dependencies) ───────────────────
        try:
            cycles = list(nx.simple_cycles(G))
            meaningful_cycles = [c for c in cycles if len(c) >= 2]
            if meaningful_cycles:
                self._cycles_found += len(meaningful_cycles)
                violations.append(f"cycles:{len(meaningful_cycles)}-circular-deps")
                total_penalty += len(meaningful_cycles) * CYCLE_PENALTY
        except Exception:
            pass  # nx may fail on degenerate graphs — skip cycle check

        # ── Dimension 4: Isolation (disconnected components) ──────────────
        undirected = G.to_undirected()
        components = list(nx.connected_components(undirected))
        scc_excess = max(0, len(components) - MAX_SCC_COUNT)
        if scc_excess > 0:
            violations.append(f"fragmented:{len(components)}-components(>{MAX_SCC_COUNT})")
            total_penalty += scc_excess * SCC_PENALTY

        # Isolated nodes (no edges at all — potential dead code)
        isolated = [n for n in G.nodes() if G.degree(n) == 0]
        if isolated:
            violations.append(f"isolated-nodes:{len(isolated)}")
            total_penalty += len(isolated) * ISOLATION_PENALTY

        # ── Graph metrics ─────────────────────────────────────────────────
        density = nx.density(G)
        # Top betweenness centrality nodes (most connected hubs)
        try:
            centrality = nx.betweenness_centrality(G)
            top_central = sorted(centrality.items(), key=lambda kv: kv[1], reverse=True)[:3]
        except Exception:
            top_central = []

        evidence = {
            "nodes": node_count,
            "edges": edge_count,
            "density": round(density, 3),
            "imports": len(snap.imports),
            "functions": len(snap.functions),
            "classes": len(snap.classes),
            "cycles": len(meaningful_cycles) if "meaningful_cycles" in dir() else 0,
            "components": len(components),
            "isolated_nodes": len(isolated),
            "top_central": [(n, round(c, 3)) for n, c in top_central],
            "bottlenecks": bottlenecks[:5],
        }

        return violations, total_penalty, evidence

    # ── Metadata Fallback ─────────────────────────────────────────────────

    def _metadata_fallback(self, cell: Cell) -> tuple[list[str], float, dict[str, Any]]:
        """Score non-CODE cells via cell metadata (novelty, complexity, risk)."""
        violations: list[str] = []
        penalty = 0.0

        # High complexity → dense graph likely
        if cell.complexity > PHI_INV:
            violations.append(f"high-complexity:{cell.complexity:.2f}")
            penalty += cell.complexity * 5.0

        # High novelty → uncharted topology (uncertain)
        if cell.novelty > PHI_INV:
            violations.append(f"uncharted-topology:{cell.novelty:.2f}")
            penalty += cell.novelty * 3.0

        evidence = {
            "fallback": "no-code-extracted",
            "cell_complexity": cell.complexity,
            "cell_novelty": cell.novelty,
            "cell_reality": cell.reality,
        }
        return violations, penalty, evidence

    # ── Health ────────────────────────────────────────────────────────────

    async def health_check(self) -> DogHealth:
        total = self._judgment_count
        status = (
            HealthStatus.HEALTHY  if total > 0 or self._active
            else HealthStatus.UNKNOWN
        )
        return DogHealth(
            dog_id=self.dog_id,
            status=status,
            latency_p50_ms=self.avg_latency_ms,
            details=(
                f"Graphs built: {self._graphs_built}, "
                f"Cycles found: {self._cycles_found}, "
                f"Fallbacks: {self._fallback_count}"
            ),
        )
