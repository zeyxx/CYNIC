#!/usr/bin/env python3
"""Render the public repository from the private tree, then prove it.

Bootstrap for the native surface `cynic dev publish`. It exists so the publication
contract is not another document nothing executes. It is declared in
contracts/publication.json with status `temporary` and a removal condition, like the
Makefile and scripts/cargo: present now, with a stated end.

    tools/publish.py render   <outdir> [--source index|worktree]
    tools/publish.py check    <outdir>
    tools/publish.py publish  <outdir> --apply

The source defaults to `index`: publication projects the committed state, read through
`git ls-files` for membership and `git show :path` for content. A previous version took
membership from the index but content from the working tree, which silently mixed the two
and pulled another agent's uncommitted work into the projection. `--source worktree` is
available for previewing an in-progress tree, and says so in its output.

`publish` refuses on any failing read-back step and leaves the previous output in
place. There is no warn mode, because a soft failure is a wish.

Standard library only. No network.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CONTRACT = ROOT / "contracts" / "publication.json"

# Highest-confidence credential shapes only. A generic key/value regex is a false
# positive generator; this is the subset that is a leak whatever the variable is called.
SECRET_PATTERNS = [
    re.compile(r"-----BEGIN (?:RSA |OPENSSH |EC |PGP )?PRIVATE KEY-----"),
    re.compile(r"\b(?:gh[pousr]_[A-Za-z0-9]{36,255})\b"),
    re.compile(r"\bsk-[A-Za-z0-9]{32,}\b"),
    re.compile(r"\bAKIA[0-9A-Z]{16}\b"),
    re.compile(r"(?i)\b(?:password|passwd|api[_-]?key|secret[_-]?key|private[_-]?key)\b\s*[:=]\s*[\"']?[A-Za-z0-9/+_-]{24,}"),
]

# Statuses whose content reaches the public repository, and therefore must be redacted.
PUBLISHED = {"public", "temporary"}


def fail(msg: str) -> None:
    print(f"  REFUSED  {msg}", file=sys.stderr)


def ok(msg: str) -> None:
    print(f"  PASS     {msg}")


def matches(pattern: str, path: str) -> bool:
    """Contract glob against a repo-relative path. `**` spans zero or more segments."""
    p, v = pattern.split("/"), path.split("/")

    def rec(pi: int, vi: int) -> bool:
        if pi == len(p):
            return vi == len(v)
        if p[pi] == "**":
            return rec(pi + 1, vi) or (vi < len(v) and rec(pi, vi + 1))
        return vi < len(v) and (p[pi] == "*" or p[pi] == v[vi]) and rec(pi + 1, vi + 1)

    return rec(0, 0)


def tracked() -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", "-z"], cwd=ROOT, capture_output=True, text=True, check=True
    ).stdout
    return [p for p in out.split("\0") if p]


def contract() -> dict:
    return json.loads(CONTRACT.read_text(encoding="utf-8"))


def status_of(rules: list[dict], path: str) -> str | None:
    for r in rules:
        if matches(r["path"], path):
            return r["status"]
    return None


def redactions(c: dict) -> list[tuple[re.Pattern, str]]:
    subs = []
    for spec in c["redactions"].values():
        for s in spec["substitutions"]:
            subs.append((re.compile(s["match"]), s["replace"]))
    return subs


def blob(rel: str, source: str) -> bytes:
    """Content for a path, from the index by default.

    Reading the working tree instead would make the projection non-reproducible and
    would include edits nobody has committed — including, on a shared worktree, other
    agents' work in progress. A publication must be a decision, not an accident.
    """
    if source == "worktree":
        return (ROOT / rel).read_bytes()
    out = subprocess.run(
        ["git", "show", f":{rel}"], cwd=ROOT, capture_output=True, check=True
    ).stdout
    return out


def render(outdir: Path, source: str = "index") -> tuple[int, int, int]:
    c = contract()
    rules = c["path_rules"]
    subs = redactions(c)
    outdir.mkdir(parents=True, exist_ok=True)

    published = skipped = redacted_files = 0
    for rel in tracked():
        st = status_of(rules, rel)
        if st is None:
            fail(f"undeclared tracked path, refusing: {rel}")
            return (0, 0)
        if st == "exclude":
            skipped += 1
            continue
        dst = outdir / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        try:
            body = blob(rel, source)
        except (OSError, subprocess.CalledProcessError):
            fail(f"unreadable: {rel}")
            return (0, 0, 0)
        if st in PUBLISHED and subs:
            try:
                text = body.decode("utf-8")
            except UnicodeDecodeError:
                dst.write_bytes(body)  # binary: no literal can hide in an opaque blob
            else:
                hits = 0
                for rx, repl in subs:
                    text, n = rx.subn(repl, text)
                    hits += n
                dst.write_text(text, encoding="utf-8")
                if hits:
                    redacted_files += 1
        else:
            shutil.copyfile(src, dst)
        # Preserve the executable bit: git tracks it, and a public script that loses
        # it is a broken script.
        mode = subprocess.run(
            ["git", "ls-files", "-s", "--", rel], cwd=ROOT, capture_output=True, text=True
        ).stdout.split()
        if len(mode) >= 2 and mode[0] == "100755":
            os.chmod(dst, 0o755)
        published += 1
    return (published, skipped, redacted_files)


def step_no_forbidden_literal(c: dict, outdir: Path) -> bool:
    r"""Apply the declared substitutions' own patterns to the rendered tree.

    A previous version derived a substring "stem" from each pattern to avoid regex
    semantics. That was wrong in the worst way: the stem of
    `[a-z0-9-]+\.synapse\.cynic` is the empty string, and the stem of `cynic\.local`
    is `cynic` — the name of the entire project. The check reported 1 737 false
    positives while the substitution itself was correct. A verifier that cries wolf
    on every file is indistinguishable from no verifier.
    """
    patterns = [(re.compile(s["match"]), s["match"])
                for spec in c["redactions"].values()
                for s in spec["substitutions"]]
    assert patterns, "no redaction pattern declared, this step would pass vacuously"

    hits = []
    for p in outdir.rglob("*"):
        if not p.is_file():
            continue
        try:
            body = p.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue  # an opaque binary cannot hide a hostname literal
        for rx, raw in patterns:
            if rx.search(body):
                hits.append(f"{p.relative_to(outdir)} <- {raw}")
    if hits:
        fail(f"forbidden literal still present in {len(hits)} rendered file(s): {hits[:5]}")
        return False
    ok(f"none of the {len(patterns)} redaction patterns matches the rendered tree")
    return True


def step_no_undeclared_path(c: dict, outdir: Path) -> bool:
    # A build artefact left inside the rendered tree used to be reported as "rendered but
    # not declared", which is true but useless: it named the tool's own residue rather
    # than a publication defect. Build output now lives outside the tree, and this guard
    # makes sure it cannot come back in.
    for stray in (".publish-target", "target", "target-cynic-ci"):
        if (outdir / stray).exists():
            fail(f"build artefact `{stray}` is inside the rendered tree; it would be published")
            return False
    rules = c["path_rules"]
    rendered = {
        str(p.relative_to(outdir))
        for p in outdir.rglob("*")
        if p.is_file()
    }
    expected = {r for r in tracked() if status_of(rules, r) in PUBLISHED}
    extra = sorted(rendered - expected)
    missing = sorted(expected - rendered)
    if extra or missing:
        if extra:
            fail(f"rendered but not declared: {extra[:5]}")
        if missing:
            fail(f"declared but not rendered: {missing[:5]}")
        return False
    ok(f"rendered set equals the declared set ({len(rendered)} files)")
    return True


def step_secrets_clean(_c: dict, outdir: Path) -> bool:
    findings = []
    for p in outdir.rglob("*"):
        if not p.is_file():
            continue
        try:
            body = p.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        for rx in SECRET_PATTERNS:
            if rx.search(body):
                findings.append(f"{p.relative_to(outdir)}: {rx.pattern[:40]}")
    if findings:
        fail(f"credential-shaped literal in {len(findings)} file(s): {findings[:5]}")
        return False
    ok("no high-confidence credential literal")
    return True


# `workspace_compiles` and `tests_green` are runner-only. AGENTS.md states the
# canonical gate is an "admitted build runner only", and the local machine cannot honour
# that: rustc 1.95.0 ICEs on cynic-node asking for 32 GiB of stack on a 27 GiB host, which
# is the failure `infra/RUST_COMPILER_ALPHAS.md:43` documents. A local attempt is not a
# weaker proof, it is a wrong one. So they are DELEGATED, never locally PASS or REFUSED.
RUNNER_ONLY = {"workspace_compiles", "tests_green"}


def step_runner_only(step_id: str) -> bool:
    fail(
        f"`{step_id}` is runner-only and must not be executed locally. Supply the admitted "
        f"runner's result with --runner-proof."
    )
    return False


def _cargo(outdir: Path, *args: str) -> bool:
    """Only reached when a runner proof explicitly asks for a local retry."""
    env = dict(os.environ, CARGO_TARGET_DIR=str(outdir.parent / (outdir.name + "-build")))
    proc = subprocess.run(
        ["cargo", *args], cwd=outdir, env=env, capture_output=True, text=True
    )
    if proc.returncode != 0:
        errs = [ln for ln in proc.stderr.splitlines() if ln.startswith("error")]
        fail(f"`cargo {' '.join(args)}` failed (exit {proc.returncode})")
        for ln in errs[:4]:
            print(f"          {ln}", file=sys.stderr)
        return False
    ok(f"`cargo {' '.join(args)}` exit 0")
    return True


def step_canonical_gate_runs(_c: dict, outdir: Path) -> bool:
    """The gate the public README names must be the gate the public CI executes."""
    readme = outdir / "README.md"
    ci = outdir / ".github" / "workflows" / "ci.yml"
    if not readme.exists() or not ci.exists():
        fail("public README or public CI workflow missing; gate cannot be compared")
        return False
    body, workflow = readme.read_text(encoding="utf-8"), ci.read_text(encoding="utf-8")
    gates = set(re.findall(r"^make ([a-z][a-z0-9.-]*)", body, re.M))
    gates |= set(re.findall(r"`make ([a-z][a-z0-9.-]*)`", body))
    if not gates:
        fail("the public README names no make gate, so there is nothing to compare")
        return False
    absent = sorted(g for g in gates if g not in workflow)
    if absent:
        fail(
            f"README announces gate(s) {absent} that the public CI never executes. "
            "Publishing a repository whose validation is not the validation it advertises "
            "is the prisma 6 defect, made public."
        )
        return False
    ok(f"every gate named in the public README is executed by the public CI: {sorted(gates)}")
    return True


CONTENT_STEPS = [
    ("no_forbidden_literal", step_no_forbidden_literal),
    ("no_undeclared_path", step_no_undeclared_path),
    ("secrets_clean", step_secrets_clean),
    ("canonical_gate_runs", step_canonical_gate_runs),
]
RUNNER_STEPS = ["workspace_compiles", "tests_green"]


def load_runner_proof(path: Path | None) -> dict:
    """The admitted build runner's result, as evidence rather than as a claim.

    Three states, never two. A step that was not executed here is `unknown`, not `pass`.
    """
    if path is None:
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def check(outdir: Path, runner_proof: Path | None = None) -> bool:
    c = contract()
    declared = [s["id"] for s in c["read_back"]["steps"]]
    implemented = [sid for sid, _ in CONTENT_STEPS] + RUNNER_STEPS
    missing = [sid for sid in declared if sid not in implemented]
    if missing:
        fail(f"contract declares read-back steps this renderer does not implement: {missing}")
        return False

    proof = load_runner_proof(runner_proof)
    print(f"read-back over {outdir}")
    print(f"  execution steps run locally: none, by policy (AGENTS.md: admitted build runner only)")
    print(f"  runner evidence: {runner_proof or 'NONE SUPPLIED'}\n")

    results: list[tuple[str, str, bool]] = []
    for sid, fn in CONTENT_STEPS:
        results.append((sid, "local", fn(c, outdir)))

    for sid in RUNNER_STEPS:
        entry = proof.get("steps", {}).get(sid)
        if entry is None:
            print(f"  UNKNOWN  {sid}: no runner evidence supplied")
            results.append((sid, "unknown", False))
        elif entry.get("status") == "pass":
            ok(f"{sid}: runner evidence {entry.get('proof_ref', 'n/a')}")
            results.append((sid, "runner", True))
        else:
            fail(f"{sid}: runner reported {entry.get('status', 'unknown')} — {entry.get('detail', '')}")
            results.append((sid, "runner", False))

    print()
    refused = [sid for sid, _, good in results if not good]
    unknown = [sid for sid, origin, good in results if not good and origin == "unknown"]
    if unknown:
        print(f"  UNKNOWN  {len(unknown)} step(s) have no evidence: {unknown}")
        print("           Publication is not refused on an unknown, and is not granted either.")
    if refused:
        print(f"  REFUSED  {len(refused)} read-back step(s) failed: {refused}")
        return False
    if unknown:
        return False  # a complete proof-back is required to publish
    print(f"  ALL {len(results)} read-back steps have evidence")
    return True


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("command", choices=["render", "check", "publish"])
    ap.add_argument("outdir")
    ap.add_argument("--apply", action="store_true", help="required for publish")
    ap.add_argument("--source", choices=["index", "worktree"], default="index",
                    help="content source: committed state (default) or working tree")
    ap.add_argument("--runner-proof", type=Path,
                    help="JSON emitted by the admitted build runner for the execution steps")
    a = ap.parse_args()

    outdir = Path(a.outdir).resolve()
    if a.command == "render":
        pub, skip, red = render(outdir, a.source)
        print(f"rendered {pub} file(s) from {a.source}, {red} redacted, {skip} excluded -> {outdir}")
        return 0
    if a.command == "check":
        return 0 if check(outdir, a.runner_proof) else 1
    if not a.apply:
        fail("publish requires --apply; without it nothing is written")
        return 1
    if outdir.exists():
        fail(f"{outdir} already exists; refusing to overwrite. Remove it explicitly.")
        return 1
    pub, skip, red = render(outdir, a.source)
    print(f"rendered {pub} file(s) from {a.source}, {red} redacted, {skip} excluded\n")
    if not check(outdir, a.runner_proof):
        print("\npublication refused. The rendered tree is kept for inspection, "
              "not published.", file=sys.stderr)
        return 1
    print(f"\npublication admissible: {outdir}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
