#!/usr/bin/env python3
"""The residual entanglement: short cycles among infrastructure modules.

The previous run falsified the hypothesis that removing the 12 clean inversions
would produce a DAG. It does not: cynic-host splits from 2 SCCs into 4, and
cynic-node barely moves (23 -> 20 members).

So the SCCs are not caused by a few type misplacements. This script shows what
they ARE caused by: the short cycles among infrastructure modules, where no
layer assignment exists yet and every module reaches sideways.

Also computes a greedy feedback arc set: the minimum number of edges whose
removal makes the graph acyclic. That number is the size of the job, and it is
reported as an estimate, not as a proven minimum (minimum FAS is NP-hard; this
is the standard greedy approximation and says so).
"""
import os
import re
from collections import defaultdict

ROOT = "crates"

USE_STMT = re.compile(r"^\s*(?:pub\s+)?use\s+(crate|super|self)\s*::\s*([A-Za-z_][A-Za-z0-9_]*)")


def module_of(rel_path):
    p = rel_path[:-3] if rel_path.endswith(".rs") else rel_path
    parts = p.split("/")
    if parts[-1] in ("mod", "lib", "main"):
        parts = parts[:-1]
    return parts[0] if parts else None


def strip_test_regions(text):
    lines = text.split("\n")
    out, i = [], 0
    while i < len(lines):
        if "cfg(test)" in lines[i]:
            depth, started, j = 0, False, i
            while j < len(lines):
                depth += lines[j].count("{") - lines[j].count("}")
                if "{" in lines[j]:
                    started = True
                if started and depth <= 0:
                    break
                j += 1
            i = j + 1
            continue
        out.append(lines[i])
        i += 1
    return "\n".join(out)


def build_graph(crate):
    src = os.path.join(ROOT, crate, "src")
    edges = defaultdict(set)
    modules = set()
    for dirpath, _, filenames in os.walk(src):
        for fn in filenames:
            if not fn.endswith(".rs"):
                continue
            full = os.path.join(dirpath, fn)
            rel = os.path.relpath(full, src)
            mod = module_of(rel)
            if not mod:
                continue
            modules.add(mod)
            try:
                text = strip_test_regions(open(full, encoding="utf-8", errors="ignore").read())
            except OSError:
                continue
            for line in text.split("\n"):
                if line.lstrip().startswith("//"):
                    continue
                m = USE_STMT.search(line)
                if not m or m.group(1) != "crate":
                    continue
                t = m.group(2)
                if t != mod:
                    edges[mod].add(t)
    return modules, edges


def sccs(modules, edges):
    index, low, on_stack, stack, comps, counter = {}, {}, {}, [], [], [0]

    def sc(v):
        index[v] = low[v] = counter[0]
        counter[0] += 1
        stack.append(v)
        on_stack[v] = True
        for w in sorted(edges.get(v, ())):
            if w not in modules:
                continue
            if w not in index:
                sc(w)
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
                comps.append(sorted(comp))

    for v in sorted(modules):
        if v not in index:
            sc(v)
    return comps


def short_cycles(members, edges, max_len=4):
    """All simple cycles up to max_len within the member set."""
    member = set(members)
    found = []
    adj = {a: sorted(b for b in edges.get(a, ()) if b in member) for a in member}

    def walk(start, path, seen):
        if len(path) > max_len:
            return
        for nxt in adj.get(path[-1], ()):
            if nxt == start and len(path) >= 2:
                found.append(list(path))
            elif nxt not in seen and nxt > start and len(path) < max_len:
                seen.add(nxt)
                path.append(nxt)
                walk(start, path, seen)
                path.pop()
                seen.remove(nxt)

    for s in sorted(member):
        walk(s, [s], {s})
    return found


def greedy_fas(modules, edges):
    """Greedy feedback arc set: how many edges must go to make it acyclic."""
    work = {a: set(edges.get(a, ())) for a in modules}
    removed = []
    for _ in range(400):
        comps = sccs(modules, work)
        if not comps:
            break
        comp = max(comps, key=len)
        member = set(comp)
        # count how many short cycles each internal edge participates in
        cycles = short_cycles(comp, work, max_len=4)
        participation = defaultdict(int)
        for cyc in cycles:
            for i in range(len(cyc)):
                participation[(cyc[i], cyc[(i + 1) % len(cyc)])] += 1
        internal = [
            (participation[(a, b)], a, b)
            for a in member
            for b in work.get(a, ())
            if b in member
        ]
        if not internal:
            # no short cycle found but SCC persists: fall back to any internal edge
            a = sorted(member)[0]
            b = sorted(x for x in work[a] if x in member)[0]
            internal = [(0, a, b)]
        internal.sort(key=lambda t: (-t[0], t[1], t[2]))
        _, a, b = internal[0]
        work[a].discard(b)
        removed.append(f"{a} -> {b}")
    return removed, work


for crate in ["cynic-host", "cynic-node"]:
    modules, edges = build_graph(crate)
    comps = sccs(modules, edges)
    print(f"===== {crate} =====")
    for comp in sorted(comps, key=len, reverse=True):
        cycles = short_cycles(comp, edges, max_len=3)
        print(f"  SCC of {len(comp)}: {len(cycles)} cycles of length <= 3")
        seen_pairs = set()
        for cyc in cycles[:14]:
            key = tuple(sorted(cyc))
            if key in seen_pairs:
                continue
            seen_pairs.add(key)
            print(f"      {' -> '.join(cyc)} -> {cyc[0]}")
    removed, _ = greedy_fas(modules, edges)
    print(f"  greedy feedback arc set: {len(removed)} edge(s) must be re-pointed")
    for r in removed[:20]:
        print(f"      - {r}")
    print()
