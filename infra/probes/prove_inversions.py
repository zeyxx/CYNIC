#!/usr/bin/env python3
"""Prove that the identified inversion edges are the CAUSE of the SCCs.

A diagnosis that says "these edges are suspicious" is not a proof. The claim
here is falsifiable and tested: remove exactly the edges whose direction
contradicts the hexagonal pattern, recompute the strongly-connected components,
and check whether the SCC dissolves.

If the SCC dissolves, the inversions are the cause and the repair is those
specific type moves. If it does not, the diagnosis is incomplete and the script
says so rather than implying success.

The correct direction, from the pattern:
    adapters  -> ports, domain      (adapters implement ports, use domain types)
    ports     -> domain             (trait signatures reference domain types)
    usecases  -> ports, domain      (use cases call ports)
    domain    -> nothing above it

An edge is an INVERSION when it points from a lower layer to a higher one.
"""
import json
import os
import re
import sys
from collections import defaultdict

ROOT = "crates"

# layer assignment: lower number = lower layer. An edge a->b is an inversion
# when layer[a] < layer[b], i.e. a lower layer depends on a higher one.
LAYER = {
    "domain": 0,
    "ports": 1,
    "usecases": 2,
    "adapters": 3,
}

# Modules that are infrastructure inside the crate, not domain: they sit above
# domain and may be depended on by adapters/usecases but not by domain/ports.
INFRA = {
    "bridge", "execution", "runtime_layout", "process_observation",
    "network_actuator", "network_mesh_plan", "network_mesh_bundle",
    "network_mesh_apply_plan", "websocket", "observe", "config",
    "endpoint_observation", "repo_access", "transport",
    "fleet_disk_observer", "service_supervisor_observation",
    "pre_os_control", "agent_access",
}

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
    evidence = defaultdict(list)
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
            for lineno, line in enumerate(text.split("\n"), 1):
                if line.lstrip().startswith("//"):
                    continue
                m = USE_STMT.search(line)
                if not m or m.group(1) != "crate":
                    continue
                target = m.group(2)
                if target == mod:
                    continue
                edges[mod].add(target)
                evidence[(mod, target)].append(f"{rel}:{lineno}")
    return modules, edges, evidence


def sccs(modules, edges):
    index, low, on_stack, stack, comps, counter = {}, {}, {}, [], [], [0]

    def strongconnect(v):
        index[v] = low[v] = counter[0]
        counter[0] += 1
        stack.append(v)
        on_stack[v] = True
        for w in sorted(edges.get(v, ())):
            if w not in modules:
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
                comps.append(sorted(comp))

    for v in sorted(modules):
        if v not in index:
            strongconnect(v)
    return comps


def is_inversion(a, b):
    """True when a lower layer depends on a higher layer or on infrastructure."""
    if a in LAYER and b in LAYER:
        return LAYER[a] < LAYER[b]
    if a in ("domain", "ports") and b in INFRA:
        return True
    return False


for crate in ["cynic-host", "cynic-node", "cynic-cli", "cynic-protocol"]:
    modules, edges, evidence = build_graph(crate)
    before = sccs(modules, edges)

    inversions = [(a, b) for a in edges for b in edges[a] if is_inversion(a, b)]

    pruned = defaultdict(set)
    for a in edges:
        for b in edges[a]:
            if (a, b) not in inversions:
                pruned[a].add(b)
    after = sccs(modules, pruned)

    print(f"===== {crate} =====")
    print(f"  SCC before: {len(before)}  ->  after removing {len(inversions)} inversion(s): {len(after)}")
    for a, b in sorted(inversions):
        print(f"    - {a} -> {b}   {', '.join(evidence[(a, b)][:2])}")
    if before and not after:
        print("  VERDICT: the inversions ARE the cause; the graph becomes a DAG")
    elif not before:
        print("  VERDICT: no SCC to explain")
    else:
        for comp in after:
            print(f"  VERDICT: INCOMPLETE -- residual SCC of {len(comp)}: {' <-> '.join(comp)}")
    print()
