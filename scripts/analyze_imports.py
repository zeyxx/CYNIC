"""
Analyze Python import graph and detect circular dependencies.

Usage: python scripts/analyze_imports.py [--graph] [--fail-on-cycle]
"""
import sys
import argparse
import ast
from pathlib import Path
from collections import defaultdict, deque

# Ensure stdout uses UTF-8 encoding
if sys.stdout.encoding and sys.stdout.encoding.lower() != 'utf-8':
    sys.stdout.reconfigure(encoding='utf-8', errors='replace')


class ImportAnalyzer(ast.NodeVisitor):
    def __init__(self, filepath: Path, content: str):
        self.filepath = filepath
        self.module_name = self._path_to_module(filepath)
        self.imports = set()
        self.from_imports = defaultdict(set)
        self.in_type_checking = False
        self.in_lazy_import = False
        # Track line numbers that are within TYPE_CHECKING or function bodies
        self.type_checking_lines = self._find_type_checking_blocks(content)

    def _path_to_module(self, path: Path) -> str:
        """Convert file path to module name."""
        # Convert to string and normalize path separators
        path_str = str(path).replace("\\", "/")
        # Remove .py extension and convert to module notation
        if path_str.endswith(".py"):
            path_str = path_str[:-3]
        # Replace slashes with dots
        module_name = path_str.replace("/", ".")
        # Remove __init__ from the end if present
        if module_name.endswith(".__init__"):
            module_name = module_name[:-9]
        return module_name

    def _find_type_checking_blocks(self, content: str) -> set:
        """Find line numbers inside TYPE_CHECKING blocks."""
        lines = set()
        in_block = False
        indent_level = 0
        for i, line in enumerate(content.split('\n'), 1):
            stripped = line.lstrip()
            if 'if TYPE_CHECKING:' in line:
                in_block = True
                indent_level = len(line) - len(stripped)
            elif in_block:
                if stripped and not stripped.startswith('#'):
                    current_indent = len(line) - len(stripped)
                    if current_indent <= indent_level and stripped:
                        in_block = False
                    else:
                        lines.add(i)
        return lines

    def visit_Import(self, node):
        # Skip if in TYPE_CHECKING block
        if node.lineno not in self.type_checking_lines:
            # Check if this is a lazy import (inside a function)
            if not self._is_lazy_import(node):
                for alias in node.names:
                    self.imports.add(alias.name)
        self.generic_visit(node)

    def visit_ImportFrom(self, node):
        # Skip if in TYPE_CHECKING block
        if node.lineno not in self.type_checking_lines:
            # Check if this is a lazy import (inside a function)
            if not self._is_lazy_import(node):
                if node.module:
                    for alias in node.names:
                        self.from_imports[node.module].add(alias.name)
        self.generic_visit(node)

    def _is_lazy_import(self, node) -> bool:
        """Check if import is inside a function (lazy import)."""
        # This is a heuristic - we check if the node is deeply nested
        # A proper check would need to track the AST context
        # For now, we'll consider imports at module level as real imports
        return getattr(node, 'col_offset', 0) > 0  # Indented imports are lazy


def build_import_graph():
    """Build complete import dependency graph."""
    graph = defaultdict(set)

    for py_file in Path("cynic").rglob("*.py"):
        try:
            content = py_file.read_text(encoding='utf-8')
            tree = ast.parse(content)
        except (SyntaxError, UnicodeDecodeError):
            continue

        analyzer = ImportAnalyzer(py_file, content)
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
    parser = argparse.ArgumentParser(
        description='Analyze Python import graph and detect circular dependencies'
    )
    parser.add_argument(
        '--graph',
        action='store_true',
        help='Display the import dependency graph'
    )
    parser.add_argument(
        '--fail-on-cycle',
        action='store_true',
        help='Exit with code 1 if any circular imports are found'
    )
    args = parser.parse_args()

    graph = build_import_graph()
    cycles = find_cycles(graph)

    if cycles:
        print(f"❌ Found {len(cycles)} circular import chain(s):")
        for cycle in cycles:
            print(f"  {' → '.join(cycle)}")
        if args.fail_on_cycle:
            sys.exit(1)
    else:
        print("✅ No circular imports detected")

    if args.graph:
        print("\nImport dependency graph:")
        for module, deps in sorted(graph.items()):
            if deps:
                print(f"  {module}:")
                for dep in sorted(deps):
                    print(f"    → {dep}")

    sys.exit(0)


if __name__ == "__main__":
    main()
