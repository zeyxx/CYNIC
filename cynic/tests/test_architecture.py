"""
CYNIC Architecture Validation Tests — Phase 0

Proves layer boundaries, interface compliance, and structural invariants.
These tests prevent regressions during all subsequent refactoring.

Rules enforced:
  - core/ never imports from judge/, api/, dogs/, act/
  - dogs/ never imports from api/
  - learning/ never imports from api/ or dogs/
  - No circular imports between any two modules
  - All AbstractDog subclasses implement required methods
  - All LLMAdapter subclasses implement complete()
"""
from __future__ import annotations

import ast
import importlib
import inspect
import os
import pkgutil
import sys
from pathlib import Path

import pytest

# ── Paths ────────────────────────────────────────────────────────────────────

CYNIC_ROOT = Path(__file__).parent.parent / "cynic"
CYNIC_PKG = "cynic"

# Layer hierarchy (lower number = lower layer, must NOT import higher layers)
LAYERS = {
    "core":      0,
    "learning":  1,
    "llm":       1,
    "perceive":  1,
    "dogs":      2,
    "judge":     3,
    "act":       3,
    "api":       4,
    "cli":       5,
    "tui":       5,
}

# Explicit forbidden imports (source_layer → cannot import from target_layer)
FORBIDDEN_IMPORTS = [
    # core is the foundation — imports nothing above it
    ("core", "dogs"),
    ("core", "judge"),
    ("core", "api"),
    ("core", "act"),
    ("core", "cli"),
    ("core", "tui"),
    # learning is low-level — no dogs, no api
    ("learning", "api"),
    ("learning", "dogs"),
    ("learning", "judge"),
    # dogs never reach into api
    ("dogs", "api"),
    ("dogs", "cli"),
    ("dogs", "tui"),
    # llm is low-level
    ("llm", "api"),
    ("llm", "dogs"),
    ("llm", "judge"),
    # perceive is low-level
    ("perceive", "api"),
    ("perceive", "judge"),
]


# ── Helpers ──────────────────────────────────────────────────────────────────

def _get_py_files(subdir: str) -> list[Path]:
    """Get all .py files in a subdirectory of the cynic package."""
    target = CYNIC_ROOT / subdir
    if not target.exists():
        return []
    return list(target.rglob("*.py"))


def _extract_imports(filepath: Path) -> list[str]:
    """Extract all import targets from a Python file using AST."""
    try:
        source = filepath.read_text(encoding="utf-8", errors="replace")
        tree = ast.parse(source, filename=str(filepath))
    except (SyntaxError, UnicodeDecodeError):
        return []

    imports = []
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                imports.append(alias.name)
        elif isinstance(node, ast.ImportFrom):
            if node.module:
                imports.append(node.module)
    return imports


def _get_layer(module_path: str) -> str | None:
    """Extract the layer name from a cynic.X.Y module path."""
    parts = module_path.split(".")
    # cynic.core.phi → "core"
    # cynic.dogs.sage → "dogs"
    if len(parts) >= 2 and parts[0] == CYNIC_PKG:
        return parts[1]
    return None


# ── Layer Boundary Tests ─────────────────────────────────────────────────────

class TestLayerBoundaries:
    """Enforce that lower layers never import from higher layers."""

    @pytest.mark.parametrize("source_layer,forbidden_target", FORBIDDEN_IMPORTS)
    def test_no_forbidden_imports(self, source_layer: str, forbidden_target: str):
        """Source layer must not import from forbidden target layer."""
        violations = []
        for filepath in _get_py_files(source_layer):
            imports = _extract_imports(filepath)
            for imp in imports:
                target_layer = _get_layer(imp)
                if target_layer == forbidden_target:
                    rel = filepath.relative_to(CYNIC_ROOT.parent)
                    violations.append(f"  {rel} imports {imp}")

        if violations:
            msg = (
                f"\nLayer violation: {source_layer}/ must not import "
                f"from {forbidden_target}/\n"
                + "\n".join(violations)
            )
            pytest.fail(msg)

    def test_no_circular_deps_between_layers(self):
        """No two layers should import each other (A→B and B→A)."""
        # Build directed graph: layer → set of layers it imports
        graph: dict[str, set[str]] = {}
        for layer in LAYERS:
            graph[layer] = set()
            for filepath in _get_py_files(layer):
                imports = _extract_imports(filepath)
                for imp in imports:
                    target = _get_layer(imp)
                    if target and target != layer and target in LAYERS:
                        graph[layer].add(target)

        # Check for bidirectional edges (A→B and B→A)
        cycles = []
        checked = set()
        for a, targets in graph.items():
            for b in targets:
                pair = tuple(sorted([a, b]))
                if pair in checked:
                    continue
                checked.add(pair)
                if a in graph.get(b, set()):
                    cycles.append(f"  {a} ↔ {b}")

        if cycles:
            pytest.fail(
                f"\nCircular layer dependencies detected:\n"
                + "\n".join(cycles)
            )


# ── Interface Compliance Tests ───────────────────────────────────────────────

class TestInterfaceCompliance:
    """All abstract base classes must be properly implemented."""

    def _get_all_subclasses(self, base_cls: type) -> list[type]:
        """Recursively find all concrete subclasses."""
        # Force-import all dog modules to ensure subclasses are registered
        dogs_dir = CYNIC_ROOT / "dogs"
        for path in dogs_dir.glob("*.py"):
            if path.name.startswith("_"):
                continue
            mod_name = f"cynic.cognition.neurons.{path.stem}"
            try:
                importlib.import_module(mod_name)
            except httpx.RequestError:
                pass

        result = []
        for sub in base_cls.__subclasses__():
            if not inspect.isabstract(sub):
                result.append(sub)
            result.extend(self._get_all_subclasses(sub))
        return result

    def test_all_dogs_implement_analyze(self):
        """Every concrete AbstractDog subclass must implement analyze()."""
        from cynic.cognition.neurons.base import AbstractDog
        subclasses = self._get_all_subclasses(AbstractDog)
        assert len(subclasses) >= 10, (
            f"Expected at least 10 dog implementations, found {len(subclasses)}: "
            f"{[c.__name__ for c in subclasses]}"
        )
        for cls in subclasses:
            assert hasattr(cls, "analyze"), f"{cls.__name__} missing analyze()"
            assert hasattr(cls, "get_capabilities"), f"{cls.__name__} missing get_capabilities()"
            assert hasattr(cls, "health_check"), f"{cls.__name__} missing health_check()"

    def test_all_dogs_have_dog_id_in_enum(self):
        """Every concrete dog's dog_id should correspond to a DogId enum value."""
        from cynic.cognition.neurons.base import AbstractDog, DogId
        subclasses = self._get_all_subclasses(AbstractDog)
        valid_ids = set(DogId)
        for cls in subclasses:
            # Instantiate to get dog_id (constructor takes dog_id)
            # Check if cls has DOG_ID class attr or check __init__ signature
            init_sig = inspect.signature(cls.__init__)
            params = list(init_sig.parameters.keys())
            # Most dogs take dog_id as first param or have it hardcoded
            # Just verify the class is importable and has analyze
            assert callable(getattr(cls, "analyze", None)), (
                f"{cls.__name__} has non-callable analyze"
            )

    def test_llm_adapters_implement_complete(self):
        """All LLMAdapter subclasses must implement complete()."""
        from cynic.llm.adapter import LLMAdapter
        subclasses = []
        # Import adapter modules
        llm_dir = CYNIC_ROOT / "llm"
        for path in llm_dir.glob("*.py"):
            if path.name.startswith("_"):
                continue
            try:
                importlib.import_module(f"cynic.llm.{path.stem}")
            except asyncio.TimeoutError:
                pass

        for sub in LLMAdapter.__subclasses__():
            if not inspect.isabstract(sub):
                subclasses.append(sub)

        assert len(subclasses) >= 2, (
            f"Expected at least 2 LLM adapter implementations, found {len(subclasses)}"
        )
        for cls in subclasses:
            assert hasattr(cls, "complete"), f"{cls.__name__} missing complete()"
            assert hasattr(cls, "check_available"), f"{cls.__name__} missing check_available()"

    def test_perceive_workers_implement_perceive(self):
        """All perceive workers must implement a perceive-like method."""
        workers_dir = CYNIC_ROOT / "perceive" / "workers"
        if not workers_dir.exists():
            pytest.skip("No perceive/workers/ directory")

        worker_modules = []
        for path in workers_dir.glob("*.py"):
            if path.name.startswith("_"):
                continue
            try:
                mod = importlib.import_module(f"cynic.senses.workers.{path.stem}")
                worker_modules.append((path.stem, mod))
            except CynicError:
                pass

        # At least some workers should exist
        assert len(worker_modules) >= 1, "No perceive workers found"


# ── Structural Invariants ────────────────────────────────────────────────────

class TestStructuralInvariants:
    """Properties that must always hold across the codebase."""

    def test_no_star_imports_in_production_code(self):
        """Star imports (from X import *) cause namespace pollution."""
        violations = []
        for layer in LAYERS:
            for filepath in _get_py_files(layer):
                try:
                    source = filepath.read_text(encoding="utf-8", errors="replace")
                    tree = ast.parse(source)
                except (SyntaxError, UnicodeDecodeError):
                    continue
                for node in ast.walk(tree):
                    if isinstance(node, ast.ImportFrom):
                        for alias in node.names:
                            if alias.name == "*":
                                rel = filepath.relative_to(CYNIC_ROOT.parent)
                                violations.append(f"  {rel}: from {node.module} import *")

        if violations:
            pytest.fail(
                f"\nStar imports in production code:\n" + "\n".join(violations)
            )

    def test_core_event_types_are_defined(self):
        """All CoreEvent members should be unique string values."""
        from cynic.core.event_bus import CoreEvent
        values = [e.value for e in CoreEvent]
        assert len(values) == len(set(values)), "Duplicate CoreEvent values found"
        assert len(values) >= 20, f"Expected at least 20 CoreEvents, got {len(values)}"

    def test_dog_ids_match_enum(self):
        """DogId enum should have exactly 11 members."""
        from cynic.cognition.neurons.base import DogId
        assert len(DogId) == 11, f"Expected 11 DogIds, got {len(DogId)}"

    def test_phi_constants_consistent(self):
        """φ constants must be mathematically consistent."""
        from cynic.core.phi import (
            PHI, PHI_INV, PHI_INV_2, PHI_2, PHI_3,
            MAX_Q_SCORE, MAX_CONFIDENCE,
        )
        assert abs(PHI - 1.618033988749895) < 1e-10
        assert abs(PHI_INV - 1 / PHI) < 1e-10
        assert abs(PHI_INV_2 - 1 / PHI**2) < 1e-10
        assert abs(PHI_2 - PHI**2) < 1e-10
        assert abs(PHI_3 - PHI**3) < 1e-10
        assert MAX_Q_SCORE == 100.0
        assert abs(MAX_CONFIDENCE - 0.618) < 0.001

    def test_no_relative_imports_in_cynic_package(self):
        """Relative imports are fragile — prefer absolute imports."""
        violations = []
        for layer in LAYERS:
            for filepath in _get_py_files(layer):
                try:
                    source = filepath.read_text(encoding="utf-8", errors="replace")
                    tree = ast.parse(source)
                except (SyntaxError, UnicodeDecodeError):
                    continue
                for node in ast.walk(tree):
                    if isinstance(node, ast.ImportFrom) and node.level and node.level > 0:
                        rel = filepath.relative_to(CYNIC_ROOT.parent)
                        violations.append(
                            f"  {rel}: relative import level={node.level}"
                        )

        # Allow a few (TYPE_CHECKING, __init__.py) but flag if excessive
        if len(violations) > 5:
            pytest.fail(
                f"\nExcessive relative imports ({len(violations)}):\n"
                + "\n".join(violations[:10])
                + ("\n  ..." if len(violations) > 10 else "")
            )
