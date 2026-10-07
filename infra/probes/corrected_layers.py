#!/usr/bin/env python3
"""Corrected layer model, tested.

MY EARLIER MODEL WAS WRONG. I assigned domain=0, ports=1 and called every
`domain -> ports` edge an inversion. But in hexagonal architecture the ports ARE
the domain's own interface: `domain/shared.rs` declares `SubstrateDomainContract`
and `domain/{services,privilege,packages,principal}.rs` implement it. A module
cannot implement a trait without the trait in scope, so `domain -> ports` is
intra-layer, not an inversion. `ports/mod.rs` importing `domain` ten times is
the correct direction and confirms ports and domain are one layer.

THE CORRECT MODEL
    inside  : domain + ports        (the hexagon's interior and its own interface)
    app     : usecases              (application layer)
    outside : adapters + every infrastructure module (bridge, execution,
              runtime_layout, process_observation, transport, repo_access,
              network_actuator, network_mesh_plan, websocket, observe, ...)

An inversion is an INSIDE -> OUTSIDE edge, or an APP -> OUTSIDE edge. Nothing
else. Inside <-> inside is intra-layer and legal; outside -> inside is the
correct direction.

This script re-runs the falsification with the corrected model, so the earlier
negative result can be attributed to the right cause: were the SCCs resistant to
repair, or was the repair aimed at the wrong edges?
"""
import os
import re
from collections import defaultdict

ROOT = "crates"

INSIDE = {"domain", "ports"}
APP = {"usecases", "application"}
OUTSIDE = {
    "adapters", "bridge", "execution", "runtime_layout", "process_observation",
    "transport", "repo_access", "pre_os_control", "fleet_disk_observer",
    "service_supervisor_observation", "endpoint_observation", "agent_access",
    "network_actuator", "network_mesh_plan", "network_mesh_bundle",
    "network_mesh_apply_plan", "network_root_policy", "network_repair",
    "network_evidence", "network_shadow_proof", "network_runtime_status",
    "network_actuator_status", "network_activation_chain",
    "dns_activation_status", "dns_bundle", "websocket", "observe", "config",
    "provider_fallback_hosts", "provider_fallback_status", "watchdog",
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
                t = m.group(2)
                if t == mod:
                    continue
                edges[mod].add(t)
                evidence[(mod, t)].append(f"{rel}:{lineno}")
    return modules, edges, evidence


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


def is_inversion(a, b):
    """INSIDE or APP reaching OUTSIDE. Inside<->inside is legal."""
    if a in INSIDE or a in APP:
        return b in OUTSIDE
    return False


for crate in ["cynic-host", "cynic-node"]:
    modules, edges, evidence = build_graph(crate)
    before = sccs(modules, edges)

    inversions = [(a, b) for a in edges for b in edges[a] if is_inversion(a, b)]
    intra = [
        (a, b) for a in edges for b in edges[a]
        if a in INSIDE and b in INSIDE
    ]

    pruned = defaultdict(set)
    for a in edges:
        for b in edges[a]:
            if (a, b) not in inversions:
                pruned[a].add(b)
    after = sccs(modules, pruned)

    print(f"===== {crate} =====")
    print(f"  SCC before: {len(before)}  ->  after removing {len(inversions)} REAL inversion(s): {len(after)}")
    print(f"  (intra-layer inside<->inside edges, NOT inversions: {len(intra)}"
          + (f" -> {', '.join(f'{a}->{b}' for a, b in sorted(intra))}" if intra else "") + ")")
    print("  real inversions:")
    for a, b in sorted(inversions):
        print(f"    - {a} -> {b}   {', '.join(evidence[(a, b)][:3])}")
    if before and not after:
        print("  VERDICT: the real inversions ARE the cause")
    elif before:
        for comp in after:
            print(f"  VERDICT: residual SCC of {len(comp)}: {' <-> '.join(comp)}")
    print()
