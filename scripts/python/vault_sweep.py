#!/usr/bin/env python3
"""
D2 baseline sweep — sha256 manifest for the model artifact vault.

Read-only over the artifacts; writes the manifest to a JSON file plus
a .sha256 seal of the manifest itself. First pass establishes the
integrity baseline: future sweeps diff against it to detect bitrot.

Usage:
  vault_sweep.py                 # full hash of every file (~15-20 min)
  vault_sweep.py --quick         # size-only inventory (no hashing)
"""
import hashlib
import json
import os
import sys
import datetime

VAULT = "/var/lib/cynic/vault/models"
OUT = "/var/lib/cynic/evidence/artifact_manifest.json"

CHUNK = 4 * 1024 * 1024


def sha256_of(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while True:
            chunk = f.read(CHUNK)
            if not chunk:
                break
            h.update(chunk)
    return h.hexdigest()


def main():
    quick = "--quick" in sys.argv

    entries = []
    total_bytes = 0
    errors = []

    for root, _, files in os.walk(VAULT):
        for fn in sorted(files):
            path = os.path.join(root, fn)
            rel = os.path.relpath(path, VAULT)
            try:
                size = os.path.getsize(path)
                total_bytes += size
                entry = {"path": rel, "size": size}
                if not quick:
                    entry["sha256"] = sha256_of(path)
                # source_url unknown at this stage (residual unknown):
                # recorded as null so later fills are explicit migrations.
                entry.setdefault("source_url", None)
                entries.append(entry)
                done = sum(1 for e in entries if "sha256" in e or quick)
                sys.stderr.write(f"\r[{done}] {rel} ({size // (1024*1024)} MB)")
            except OSError as e:
                errors.append({"path": rel, "error": str(e)})

    sys.stderr.write("\n")
    manifest = {
        "schema": "cynic.artifact.manifest.v1",
        "generated_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds"),
        "vault_root": VAULT,
        "mode": "inventory" if quick else "baseline_digest",
        "file_count": len(entries),
        "total_bytes": total_bytes,
        "entries": sorted(entries, key=lambda e: e["path"]),
        "errors": errors,
    }
    blob = json.dumps(manifest, sort_keys=True, indent=2).encode()
    digest = hashlib.sha256(blob).hexdigest()

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    tmp = OUT + ".tmp"
    with open(tmp, "w") as f:
        f.write(json.dumps(manifest, indent=2))
    os.replace(tmp, OUT)
    with open(OUT + ".sha256", "w") as f:
        f.write(f"{digest}  artifact_manifest.json\n")

    print(f"files={len(entries)} total={total_bytes // (1024**3)} GiB "
          f"({total_bytes // (1024**2)} MiB)")
    print(f"manifest -> {OUT}")
    print(f"manifest sha256: {digest[:16]}…")
    if errors:
        print(f"WARNINGS: {len(errors)} unreadable paths")


if __name__ == "__main__":
    main()
