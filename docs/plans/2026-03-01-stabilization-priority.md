# Stabilization Priority: Fractal Architecture Integrity

> **BLOCKING PRIORITY** — Must complete before P8-P11
>
> **For Claude:** Use superpowers:subagent-driven-development to execute task-by-task.

**Goal:** Harden CYNIC's fractal architecture for 10k TPS capacity by eliminating Unicode corruption, circular imports, missing wiring, and broken CI/CD gatekeeping.

**Context:** 120+ files corrupted by UTF-8 encoding during recent CI/CD (likely bad formatter). Core φ constants, event routing, and import chains are vulnerable. The infinite, interconnected fractal design requires *perfect* precision at every scale.

**Architecture:** Five stabilization axes:
1. **Encoding Pipeline** — UTF-8 validation in pre-commit hooks + CI/CD
2. **Import Health** — Detect circular deps + visualize dependency graph
3. **Factory Wiring** — Verify all components initialized + injected correctly
4. **API Router Integrity** — Health checks for all 20+ routers
5. **CI/CD Gatekeeping** — Fail fast on encoding, imports, wiring, tests

**Tech Stack:** Python AST analysis, pytest, pre-commit hooks, GitHub Actions

---

## Phase 1: Encoding Pipeline (Prevent Future Corruption)

### SPA-1: Add UTF-8 Encoding Validation Hook

**Files:**
- Create: `.pre-commit-config.yaml` (if missing) or update
- Create: `scripts/validate_encoding.py`
- Modify: `pyproject.toml` (add pre-commit config)

**Step 1: Write validation script**

Create `scripts/validate_encoding.py`:

```python
"""
Validate all Python files are valid UTF-8 with correct φ, ×, √, → symbols.

Exit code 1 if any file has encoding issues.
"""
import sys
from pathlib import Path


def validate_file(path: Path) -> list[str]:
    """Check file for UTF-8 validity and mathematical symbol correctness."""
    errors = []
    try:
        content = path.read_text(encoding='utf-8')
    except UnicodeDecodeError as e:
        errors.append(f"{path}: UTF-8 decode error at position {e.start}: {e.reason}")
        return errors

    # Check for common corruption patterns
    if 'Ã—' in content or 'â€š' in content or 'â†'' in content or 'Ï†' in content:
        errors.append(f"{path}: Found Unicode corruption (corrupted ×, √, →, or φ)")

    # Check φ usage in phi.py
    if 'phi.py' in str(path):
        if 'PHI' in content and 'φ' not in content:
            # φ symbol should be defined in module docstring/comments
            if '"""' in content and 'PHI' in content:
                # Allow if it's just constants without the symbol
                pass

    return errors


def main():
    cynic_dir = Path("cynic")
    test_dir = Path("tests")
    all_errors = []

    for py_file in list(cynic_dir.rglob("*.py")) + list(test_dir.rglob("*.py")):
        all_errors.extend(validate_file(py_file))

    if all_errors:
        print("❌ Encoding validation failed:")
        for error in all_errors:
            print(f"  {error}")
        sys.exit(1)
    else:
        print("✅ All files have valid UTF-8 encoding")
        sys.exit(0)


if __name__ == "__main__":
    main()
```

**Step 2: Run validation**

```bash
python scripts/validate_encoding.py
```

Expected: `✅ All files have valid UTF-8 encoding`

**Step 3: Add pre-commit hook**

Update `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: local
    hooks:
      - id: validate-encoding
        name: Validate UTF-8 encoding
        entry: python scripts/validate_encoding.py
        language: system
        types: [python]
        pass_filenames: false
        stages: [commit]

  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.5.0
    hooks:
      - id: check-merge-conflict
      - id: check-yaml
      - id: end-of-file-fixer
      - id: trailing-whitespace
```

**Step 4: Commit**

```bash
git add scripts/validate_encoding.py .pre-commit-config.yaml
git commit -m "feat(stabilization): Add UTF-8 encoding validation hook"
```

---

## Phase 2: Import Health (Detect Circular Dependencies)

### SPA-2: Detect & Map Circular Imports

**Files:**
- Create: `scripts/analyze_imports.py`
- Create: `docs/import_graph.txt`

**Step 1: Write import analyzer**

Create `scripts/analyze_imports.py`:

```python
"""
Analyze Python import graph and detect circular dependencies.

Usage: python scripts/analyze_imports.py [--graph]
"""
import sys
import ast
from pathlib import Path
from collections import defaultdict, deque


class ImportAnalyzer(ast.NodeVisitor):
    def __init__(self, filepath: Path):
        self.filepath = filepath
        self.module_name = self._path_to_module(filepath)
        self.imports = set()
        self.from_imports = defaultdict(set)

    def _path_to_module(self, path: Path) -> str:
        """Convert file path to module name."""
        parts = path.relative_to(Path.cwd()).parts
        return ".".join(parts).replace(".py", "").replace("__init__", "")

    def visit_Import(self, node):
        for alias in node.names:
            self.imports.add(alias.name)
        self.generic_visit(node)

    def visit_ImportFrom(self, node):
        if node.module:
            for alias in node.names:
                self.from_imports[node.module].add(alias.name)
        self.generic_visit(node)


def build_import_graph():
    """Build complete import dependency graph."""
    graph = defaultdict(set)

    for py_file in Path("cynic").rglob("*.py"):
        try:
            content = py_file.read_text(encoding='utf-8')
            tree = ast.parse(content)
        except (SyntaxError, UnicodeDecodeError):
            continue

        analyzer = ImportAnalyzer(py_file)
        analyzer.visit(tree)

        # Map to cynic.* imports only
        for imp in analyzer.imports:
            if imp.startswith("cynic"):
                graph[analyzer.module_name].add(imp)

        for module, names in analyzer.from_imports.items():
            if module and module.startswith("cynic"):
                graph[analyzer.module_name].add(module)

    return graph


def find_cycles(graph):
    """Detect circular imports using DFS."""
    visited = set()
    rec_stack = set()
    cycles = []

    def dfs(node, path):
        visited.add(node)
        rec_stack.add(node)
        path.append(node)

        for neighbor in graph.get(node, []):
            if neighbor not in visited:
                dfs(neighbor, path.copy())
            elif neighbor in rec_stack and len(path) > 1:
                cycle_start = path.index(neighbor)
                cycle = path[cycle_start:] + [neighbor]
                cycles.append(cycle)

        rec_stack.discard(node)

    for node in graph:
        if node not in visited:
            dfs(node, [])

    return cycles


def main():
    graph = build_import_graph()
    cycles = find_cycles(graph)

    if cycles:
        print(f"❌ Found {len(cycles)} circular import chain(s):")
        for cycle in cycles:
            print(f"  {' → '.join(cycle)}")
        sys.exit(1)
    else:
        print("✅ No circular imports detected")

    if "--graph" in sys.argv:
        print("\nImport dependency graph:")
        for module, deps in sorted(graph.items()):
            if deps:
                print(f"  {module}:")
                for dep in sorted(deps):
                    print(f"    → {dep}")

    sys.exit(0)


if __name__ == "__main__":
    main()
```

**Step 2: Run analyzer**

```bash
python scripts/analyze_imports.py --graph
```

Expected: `✅ No circular imports detected`

**Step 3: Add to CI/CD**

Update `.github/workflows/ci.yml` (create if missing):

```yaml
name: CI/CD Gatekeeping

on: [push, pull_request]

jobs:
  encoding:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-python@v4
        with:
          python-version: '3.13'
      - name: Validate UTF-8 Encoding
        run: python scripts/validate_encoding.py

  imports:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-python@v4
        with:
          python-version: '3.13'
      - name: Detect Circular Imports
        run: python scripts/analyze_imports.py

  tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-python@v4
        with:
          python-version: '3.13'
      - run: pip install -e . -q
      - run: python -m pytest tests/ -v --tb=short -x
```

**Step 4: Commit**

```bash
git add scripts/analyze_imports.py .github/workflows/ci.yml
git commit -m "feat(stabilization): Add import cycle detection + CI/CD gatekeeping"
```

---

## Phase 3: Factory Wiring Audit

### SPA-3: Verify All Components Are Wired

**Files:**
- Create: `scripts/audit_factory_wiring.py`
- Modify: `cynic/kernel/organism/factory.py` (if wiring gaps found)

**Step 1: Write factory audit**

Create `scripts/audit_factory_wiring.py`:

```python
"""
Audit factory.py to ensure all components are initialized and passed to Organism.

Checks:
  - All components initialized (bus, journal, tracer, collector, etc.)
  - All components injected into ArchiveCore
  - All service setters called (set_qtable, set_metrics_collector, etc.)
  - No missing wiring steps
"""
import re
from pathlib import Path


def audit_factory():
    """Check factory.py for completeness."""
    factory_path = Path("cynic/kernel/organism/factory.py")
    content = factory_path.read_text(encoding='utf-8')

    issues = []

    # Required initializations
    required_inits = [
        "instance_bus = EventBus",
        "self.journal = EventJournal",
        "self.tracer = DecisionTracer",
        "self.loop_validator = LoopClosureValidator",
        "self.reconstructor = StateReconstructor",
        "self.metrics_collector = EventMetricsCollector",
    ]

    for init_pattern in required_inits:
        if init_pattern not in content:
            issues.append(f"Missing initialization: {init_pattern}")

    # Required service injections (setters)
    required_setters = [
        "prober.set_qtable",
        "prober.set_residual_detector",
        "prober.set_escore_tracker",
        "prober.set_metrics_collector",
    ]

    for setter in required_setters:
        if setter not in content:
            issues.append(f"Missing service injection: {setter}")

    # Required ArchiveCore fields
    required_archive_fields = [
        "journal=self.journal",
        "loop_validator=self.loop_validator",
        "reconstructor=self.reconstructor",
        "metrics_collector=self.metrics_collector",
    ]

    for field in required_archive_fields:
        if field not in content:
            issues.append(f"Missing ArchiveCore field: {field}")

    # Required bus handlers
    required_handlers = [
        'instance_bus.on("*", self._journal_adapter.on_event)',
        'instance_bus.on("*", self._loop_adapter.on_event)',
        'instance_bus.on("*", self._metrics_adapter.on_event)',
    ]

    for handler in required_handlers:
        # Normalize whitespace for matching
        handler_normalized = re.sub(r'\s+', ' ', handler)
        content_normalized = re.sub(r'\s+', ' ', content)
        if handler_normalized not in content_normalized:
            issues.append(f"Missing bus handler: {handler}")

    if issues:
        print(f"❌ Factory wiring audit found {len(issues)} issues:")
        for issue in issues:
            print(f"  - {issue}")
        return False
    else:
        print("✅ Factory wiring is complete")
        return True


if __name__ == "__main__":
    import sys
    if not audit_factory():
        sys.exit(1)
```

**Step 2: Run audit**

```bash
python scripts/audit_factory_wiring.py
```

Expected: `✅ Factory wiring is complete`

**Step 3: Commit**

```bash
git add scripts/audit_factory_wiring.py
git commit -m "feat(stabilization): Add factory wiring audit script"
```

---

## Phase 4: API Router Health Checks

### SPA-4: Verify All 20+ Routers Are Mounted

**Files:**
- Create: `scripts/audit_api_routers.py`
- Modify: `cynic/interfaces/api/server.py` (if routers unmounted)

**Step 1: Write router audit**

Create `scripts/audit_api_routers.py`:

```python
"""
Audit API router mounting. Verify all routers in cynic/interfaces/api/routers/
are imported and mounted in server.py.
"""
from pathlib import Path
import re


def audit_routers():
    """Check that all routers are mounted."""
    routers_dir = Path("cynic/interfaces/api/routers")
    server_path = Path("cynic/interfaces/api/server.py")

    # Find all router modules
    router_modules = sorted([
        f.stem for f in routers_dir.glob("*.py")
        if f.stem != "__init__" and not f.stem.startswith("_")
    ])

    server_content = server_path.read_text(encoding='utf-8')

    issues = []

    # Check each router is imported
    for router in router_modules:
        import_pattern = f"from.*{router}.*import"
        if not re.search(import_pattern, server_content, re.IGNORECASE):
            issues.append(f"Router not imported: {router}")

    # Check each router is mounted (include_router or app.mount)
    for router in router_modules:
        # Look for include_router or direct mounting
        mount_pattern = f"(include_router|mount).*{router}"
        if not re.search(mount_pattern, server_content, re.IGNORECASE):
            issues.append(f"Router not mounted: {router}")

    if issues:
        print(f"❌ API router audit found {len(issues)} issues:")
        for issue in issues:
            print(f"  - {issue}")
        print(f"\nAvailable routers: {', '.join(router_modules)}")
        return False
    else:
        print(f"✅ All {len(router_modules)} API routers are mounted")
        print(f"   Routers: {', '.join(router_modules[:5])}... (+{len(router_modules)-5} more)")
        return True


if __name__ == "__main__":
    import sys
    if not audit_routers():
        sys.exit(1)
```

**Step 2: Run audit**

```bash
python scripts/audit_api_routers.py
```

Expected: `✅ All 20+ API routers are mounted`

**Step 3: Commit**

```bash
git add scripts/audit_api_routers.py
git commit -m "feat(stabilization): Add API router mounting audit"
```

---

## Phase 5: Comprehensive Stability Test

### SPA-5: Create Stability Test Suite

**Files:**
- Create: `tests/test_stabilization.py`

**Step 1: Write stability tests**

Create `tests/test_stabilization.py`:

```python
"""
Stabilization tests — verify foundational integrity for 10k TPS.

Tests that the codebase is ready for high-throughput operation.
"""
import subprocess
import sys
from pathlib import Path


def test_no_encoding_errors():
    """Encoding validation passes."""
    result = subprocess.run(
        [sys.executable, "scripts/validate_encoding.py"],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, f"Encoding validation failed:\n{result.stdout}\n{result.stderr}"


def test_no_circular_imports():
    """No circular import chains."""
    result = subprocess.run(
        [sys.executable, "scripts/analyze_imports.py"],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, f"Circular import detected:\n{result.stdout}\n{result.stderr}"


def test_factory_wiring_complete():
    """All components wired in factory."""
    result = subprocess.run(
        [sys.executable, "scripts/audit_factory_wiring.py"],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, f"Factory wiring incomplete:\n{result.stdout}\n{result.stderr}"


def test_api_routers_mounted():
    """All API routers are mounted."""
    result = subprocess.run(
        [sys.executable, "scripts/audit_api_routers.py"],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, f"Router mounting incomplete:\n{result.stdout}\n{result.stderr}"


def test_priority_tests_pass():
    """All Priority 5-7 tests pass (no regressions)."""
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "pytest",
            "tests/test_priority5_event_protocol.py",
            "tests/test_priority6_state_reconstruction.py",
            "tests/test_priority7_event_metrics.py",
            "-v",
            "--tb=short",
        ],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, f"Priority tests failed:\n{result.stdout}\n{result.stderr}"


if __name__ == "__main__":
    import pytest
    pytest.main([__file__, "-v"])
```

**Step 2: Run stability tests**

```bash
pytest tests/test_stabilization.py -v
```

Expected: All 5 tests pass.

**Step 3: Commit**

```bash
git add tests/test_stabilization.py
git commit -m "test(stabilization): Add comprehensive stability test suite"
```

---

## Verification Checklist

- [ ] UTF-8 encoding validation passes
- [ ] No circular imports detected
- [ ] Factory wiring is complete
- [ ] All API routers mounted
- [ ] Priority 5-7 tests still pass (88/88)
- [ ] Stabilization tests pass (5/5)
- [ ] Pre-commit hooks configured
- [ ] CI/CD gatekeeping active

---

## Success Criteria for 10k TPS

After Stabilization Priority completes:

✅ **Encoding Pipeline:** No future Unicode corruption
✅ **Import Health:** Fractal has no circular dependencies (scales infinitely)
✅ **Factory Wiring:** All components properly injected (testable, composable)
✅ **API Integrity:** All routers mounted + health-checkable
✅ **CI/CD Gatekeeping:** Fail fast on regressions
✅ **Test Coverage:** 93+ tests passing, zero flakes

---

## Then: Priority 8 Unblocked

Once Stabilization completes, Priority 8 (SelfProber Metrics Integration) can be executed confidently on a solid foundation. The fractal architecture will be:

- **Encoding-safe** (no corruption)
- **Structurally-sound** (no cycles)
- **Fully-wired** (complete DI)
- **Observable** (health checks everywhere)
- **CI/CD-protected** (gatekeeping active)

This is the prerequisite for reaching 10k TPS with Solana/NEAR anchoring + predictive capabilities.

