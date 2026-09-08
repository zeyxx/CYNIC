#!/home/user/.local/bin/python3.11
"""TEMPORARY field adapter: seal / verify IntentTransmission records.

STATUS: transitional, NOT runtime authority. Expires the day
`cynic intent seal|verify` lands (see protocols/intent-cli-surface.md) —
this file is then deleted, not maintained. Never import into Rust, never
grow features.

Why python exists at all: field teams (bootcamp) have no cynic binary yet.
This adapter gives them sealing with the SAME canonical JSON as
cynic-protocol integrity::canonical_json_bytes (sorted keys, compact
separators, UTF-8). Any divergence is a bug HERE — the Rust side wins.
Cross-proven byte-identical on trap vector (accents, quotes, newline,
slash, backslash): f752c3…8fbfc9.

Usage:
  intent-seal-TEMP.py seal record.json
  intent-seal-TEMP.py seal --in-place record.json
  intent-seal-TEMP.py verify sealed.json   # exit 0 iff digest matches
"""

import hashlib
import json
import sys


def canonical_bytes(record: dict) -> bytes:
    payload = {k: v for k, v in record.items() if k != "digest"}
    return json.dumps(
        payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")


def seal(record: dict) -> str:
    return "sha3-512:" + hashlib.sha3_512(canonical_bytes(record)).hexdigest()


def cmd_seal(path: str, in_place: bool) -> int:
    with open(path, encoding="utf-8") as f:
        record = json.load(f)
    digest = seal(record)
    if in_place:
        record["digest"] = digest
        with open(path, "w", encoding="utf-8") as f:
            json.dump(record, f, ensure_ascii=False, indent=2)
            f.write("\n")
    else:
        print(digest)
    return 0


def cmd_verify(path: str) -> int:
    with open(path, encoding="utf-8") as f:
        record = json.load(f)
    claimed = record.get("digest", "")
    if seal(record) == claimed:
        print("OK " + claimed)
        return 0
    print("MISMATCH claimed=" + claimed + " computed=" + seal(record))
    return 1


def main(argv: list) -> int:
    if len(argv) < 3 or argv[1] not in ("seal", "verify"):
        print(__doc__)
        return 2
    if argv[1] == "seal":
        in_place = argv[2] == "--in-place"
        path = argv[3] if in_place and len(argv) > 3 else argv[2]
        return cmd_seal(path, in_place)
    return cmd_verify(argv[2])


if __name__ == "__main__":
    sys.exit(main(sys.argv))
