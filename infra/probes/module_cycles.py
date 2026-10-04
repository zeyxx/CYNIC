#!/usr/bin/env python3
"""Cartograph the module-level cycles in the largest crates.

WHY. The crate-level DAG is clean (49 crates, 0 cycles), but the module level is
not: 27 two-module cycles across cynic-host (14), cynic-node (10), cynic-cli (1),
cynic-protocol (2). A `domain` <-> `ports` cycle means the domain and its own port
declarations are mutually entangled -- the hexagon is not a hexagon. Moving files
to fix this without knowing the causal chain would be destructive, so the first
step is the chain, not the move.

WHAT. A real strongly-connected-component analysis (Tarjan) over the module
graph, not a 2-cycle scan. Each cycle is reported with its member modules and the
directed edges that close it, so the correct direction can be chosen from
evidence rather than from the file layout.

HOW. Module identity is the top-level module of the file (`crate::a::b` -> `a`).
Edges come from `crate::`, `super::`, and `self::` path references in source.
Test modules (`#[cfg(test)]` regions) are excluded: a test that reaches across
modules is not an architectural dependency.

Output: JSON to stdout, one object per crate.
"""
import json
import os
import re
import sys
from collections import defaultdict

CRATES = sys.argv[1:] or ["cynic-host", "cynic-node", "cynic-cli", "cynic-protocol"]
ROOT = "crates"

# A path reference: crate::a::b, super::a, self::a
PATH_REF = re.compile(r"\b(crate|super|self)\s*::\s*([A-Za-z_][A-Za-z0-9_]*)")
USE_STMT = re.compile(r"^\s*(?:pub\s+)?use\s+(crate|super|self)\s*::\s*([A-Za-z_][A-Za-z0-9_]*)")


def module_of(rel_path):
    """Top-level module name for a file relative to src/.

    src/lib.rs and src/main.rs are the crate root itself, not a module.
    src/a/mod.rs and src/a.rs both yield `a`.
    """
    p = rel_path[:-3] if rel_path.endswith(".rs") else rel_path
    parts = p.split("/")
    if parts[-1] in ("mod", "lib", "main"):
        parts = parts[:-1]
    if not parts:
        return None  # crate root
    return parts[0]


def strip_test_regions(text):
    """Remove `#[cfg(test)] mod ...` blocks and `#[cfg(test)]` items.

    A test module that reaches across modules is not an architectural
    dependency; counting it would invent cycles that the compiled library does
    not have.
    """
    lines = text.split("\n")
    out = []
    i = 0
    while i < len(lines):
        line = lines[i]
        if "cfg(test)" in line:
            # Skip to the end of the attribute target: brace matching from the
            # next line that opens a block.
            depth = 0
            started = False
            j = i
            while j < len(lines):
                depth += lines[j].count("{") - lines[j].count("}")
                if "{" in lines[j]:
                    started = True
                if started and depth <= 0:
                    break
                j += 1
            i = j + 1
            continue
        out.append(line)
        i += 1
    return "\n".join(out)


def analyse(crate):
    src = os.path.join(ROOT, crate, "src")
    if not os.path.isdir(src):
        return None

    # file -> module, and module -> files
    mod_files = defaultdict(list)
    for dirpath, _, filenames in os.walk(src):
        for fn in filenames:
            if not fn.endswith(".rs"):
                continue
            full = os.path.join(dirpath, fn)
            rel = os.path.relpath(full, src)
            mod = module_of(rel)
            if mod:
                mod_files[mod].append((rel, full))

    # directed edges module -> module, with evidence
    edges = defaultdict(lambda: defaultdict(list))  # a -> b -> [evidence]
    for mod, files in mod_files.items():
        for rel, full in files:
            try:
                text = strip_test_regions(open(full, encoding="utf-8", errors="ignore").read())
            except OSError:
                continue
            for lineno, line in enumerate(text.split("\n"), 1):
                if line.lstrip().startswith("//"):
                    continue
                for m in USE_STMT.finditer(line):
                    kind, first = m.group(1), m.group(2)
                    target = first if kind == "crate" else None
                    if kind in ("super", "self"):
                        # `super::x` from module m refers to a sibling at the
                        # parent level; treat only the top-level case, where
                        # super:: from a top-level module is the crate root.
                        target = None
                    if target and target != mod:
                        edges[mod][target].append(f"{rel}:{lineno}: {line.strip()[:100]}")

    # Tarjan SCC
    nodes = sorted(mod_files.keys())
    index = {}
    low = {}
    on_stack = {}
    stack = []
    sccs = []
    counter = [0]

    def strongconnect(v):
        index[v] = low[v] = counter[0]
        counter[0] += 1
        stack.append(v)
        on_stack[v] = True
        for w in sorted(edges.get(v, {})):
            if w not in nodes:
                continue
            if w not in index:
                strongconnect(w)
                low[v] = min(low[v], low[w])
            elif on_stack.get(w):
                low[v] = min(low[v], index[w])
        if low[v] == index[v]:
            comp = []
            while True:
                w = stack.pop()
                on_stack[w] = False
                comp.append(w)
                if w == v:
                    break
            if len(comp) > 1:
                sccs.append(sorted(comp))

    for v in nodes:
        if v not in index:
            strongconnect(v)

    # describe each cycle
    cycles = []
    for comp in sccs:
        member = set(comp)
        internal = []
        for a in comp:
            for b, ev in edges.get(a, {}).items():
                if b in member:
                    internal.append(
                        {"from": a, "to": b, "count": len(ev), "evidence": ev[:3]}
                    )
        cycles.append(
            {
                "members": comp,
                "size": len(comp),
                "internal_edges": len(internal),
                "edges": sorted(internal, key=lambda e: (e["from"], e["to"])),
            }
        )

    return {
        "crate": crate,
        "modules": len(nodes),
        "edges": sum(len(v) for v in edges.values()),
        "cycles": len(cycles),
        "cycle_details": sorted(cycles, key=lambda c: (-c["size"], c["members"])),
    }


results = [r for r in (analyse(c) for c in CRATES) if r]
print(json.dumps(results, indent=1))
