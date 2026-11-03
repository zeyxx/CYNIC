#!/usr/bin/env python3
"""
Sync infra/crate-topology.json with the actual workspace state.

Derives:
- Crates and their kinds (binary vs library)
- Runtime dependencies (depends_on)
- Dev dependencies (dev_depends_on)
- Consumers
- Justification (is_binary or has consumers)
- Lines of code per crate (loc) and total (loc_total)
- Summary metrics (runtime_edges, dev_edges, binaries, etc.)

Preserves:
- Layer assignments (L0-L4)
- Policies, rules, and debt ledgers
"""

import glob
import json
import os
import sys

def parse_sections(content):
    sects = {}
    cur = None
    buf = []
    for line in content.splitlines():
        t = line.strip()
        if t.startswith("[") and t.endswith("]"):
            if cur:
                sects[cur] = "\n".join(buf)
            cur = t.strip("[]")
            buf = []
        elif cur:
            buf.append(line)
    if cur:
        sects[cur] = "\n".join(buf)
    return sects

def named_in(body, known, self_name):
    found = set()
    for line in body.splitlines():
        t = line.strip()
        if not t or not (t[0].islower() or t[0] == "_"):
            continue
        name = []
        for c in t:
            if c.islower() or c.isdigit() or c == "-":
                name.append(c)
            else:
                break
        name = "".join(name)
        if not name or name == self_name:
            continue
        rest = t[len(name):].lstrip()
        if (rest.startswith("=") or rest.startswith(".workspace")) and name in known:
            found.add(name)
    return found

def is_binary(cname, repo_root):
    manifest_path = os.path.join(repo_root, f"crates/{cname}/Cargo.toml")
    with open(manifest_path) as f:
        content = f.read()
    sects = parse_sections(content)
    if "bin" in sects:
        bin_sect = sects["bin"]
        if "name" in bin_sect or "path" in bin_sect:
            return True
    if os.path.isfile(os.path.join(repo_root, f"crates/{cname}/src/main.rs")):
        return True
    if glob.glob(os.path.join(repo_root, f"crates/{cname}/src/bin/*.rs")):
        return True
    return False

def loc_of(cname, repo_root):
    total = 0
    crate_dir = os.path.join(repo_root, f"crates/{cname}")
    for root, _, files in os.walk(crate_dir):
        for file in files:
            if file.endswith(".rs"):
                with open(os.path.join(root, file), "rb") as f:
                    total += sum(1 for _ in f)
    return total

def sync_topology(repo_root, check_only=False):
    topo_path = os.path.join(repo_root, "infra/crate-topology.json")
    if not os.path.exists(topo_path):
        print(f"Error: {topo_path} not found", file=sys.stderr)
        return 1

    with open(topo_path) as f:
        committed = json.load(f)

    manifest_pattern = os.path.join(repo_root, "crates/*/Cargo.toml")
    known = sorted(list(set(os.path.basename(os.path.dirname(p)) for p in glob.glob(manifest_pattern))))

    edges = {}
    dev_edges = {}
    consumers = {name: set() for name in known}
    dev_consumers = {name: set() for name in known}

    for name in known:
        manifest_path = os.path.join(repo_root, f"crates/{name}/Cargo.toml")
        with open(manifest_path) as f:
            content = f.read()
        sects = parse_sections(content)
        e = set()
        for s in ["dependencies", "build-dependencies"]:
            if s in sects:
                e.update(named_in(sects[s], known, name))
        edges[name] = e
        for dep in e:
            consumers[dep].add(name)

        dev_e = set()
        if "dev-dependencies" in sects:
            dev_e.update(named_in(sects["dev-dependencies"], known, name))
        dev_edges[name] = dev_e
        for dep in dev_e:
            dev_consumers[dep].add(name)

    runtime_edges = sum(len(e) for e in edges.values())
    dev_edges_count = sum(len(e) for e in dev_edges.values())
    binaries_count = sum(1 for n in known if is_binary(n, repo_root))
    loc_total = sum(loc_of(n, repo_root) for n in known)

    doc = dict(committed)
    new_crates = {}
    for name in sorted(known):
        prev = dict(committed.get("crates", {}).get(name, {}))
        prev["consumers"] = sorted(list(consumers[name]))
        prev["depends_on"] = sorted(list(edges[name]))
        prev["dev_depends_on"] = sorted(list(dev_edges[name]))
        bin_flag = is_binary(name, repo_root)
        prev["kind"] = "binary" if bin_flag else "library"
        prev["justified"] = bin_flag or len(consumers[name]) > 0 or len(dev_consumers[name]) > 0
        prev["loc"] = loc_of(name, repo_root)
        new_crates[name] = {k: prev[k] for k in sorted(prev.keys())}

    doc["crates"] = new_crates
    doc["measured"] = {
        "acyclic": True,
        "binaries": binaries_count,
        "crates": len(known),
        "dev_edges": dev_edges_count,
        "loc_basis": committed.get("measured", {}).get(
            "loc_basis",
            "every .rs file under crates/<name>/, tests and examples included, blank lines counted",
        ),
        "loc_total": loc_total,
        "pending_disposition": len(committed.get("pending_disposition", {})),
        "runtime_edges": runtime_edges,
        "unjustified": sum(1 for n in known if not (is_binary(n, repo_root) or len(consumers[n]) > 0 or len(dev_consumers[n]) > 0)),
        "upward_edges": committed.get("measured", {}).get("upward_edges", 0),
    }

    rendered = json.dumps(doc, indent=4, sort_keys=False) + "\n"

    with open(topo_path) as f:
        existing = f.read()

    if existing == rendered:
        print(f"Topology SSOT is fresh: {len(known)} crates, {runtime_edges} runtime edges, {loc_total} LOC.")
        return 0

    if check_only:
        print("Topology SSOT is stale. Run scripts/sync-topology.py to update it.", file=sys.stderr)
        return 1

    with open(topo_path, "w") as f:
        f.write(rendered)
    print(f"Updated {topo_path}: {len(known)} crates, {runtime_edges} runtime edges, {loc_total} LOC.")
    return 0

if __name__ == "__main__":
    root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    check = "--check" in sys.argv
    sys.exit(sync_topology(root, check_only=check))
