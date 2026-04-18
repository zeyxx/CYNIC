#!/usr/bin/env python3
"""Sovereign Ouroboros runner — replaces hermes-agent with direct Tailscale calls.

Reads a plan.json (from scorecard.py), clones repos, analyzes them via qwen35
on sovereign GPU, persists findings via kernel POST /observe, writes report.json.

No third-party agent. No cloud provider. Bash + Python + curl + kernel.

Usage:
    python3 scripts/ouroboros_run.py --plan plan.json --report report.json
    python3 scripts/ouroboros_run.py --plan plan.json --report report.json --dry-run
"""

from __future__ import annotations

import argparse
import json
import logging
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from urllib.request import Request, urlopen
from urllib.error import URLError, HTTPError

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    datefmt="%H:%M:%S",
)
log = logging.getLogger("ouroboros")

# ── Config from env (set by ouroboros.sh) ──────────────────────────

INFERENCE_URL = os.environ.get("OPENAI_BASE_URL", "").rstrip("/")
INFERENCE_KEY = os.environ.get("OPENAI_API_KEY", "")
INFERENCE_MODEL = os.environ.get("HERMES_MODEL", "Qwen3.5-9B-Q4_K_M.gguf")
KERNEL_ADDR = os.environ.get("CYNIC_REST_ADDR", "")
KERNEL_KEY = os.environ.get("CYNIC_API_KEY", "")

ROOT = Path(__file__).resolve().parent.parent
TRADITIONS_PATH = ROOT / "docs" / "identity" / "CYNIC-PERENNIAL-EPISTEMOLOGY.md"
IDENTITY_PATH = ROOT / "docs" / "identity" / "HERMES-OUROBOROS.md"

GENEROUS_TIMEOUT = 300  # seconds — will be calibrated after first run

SYSTEM_PROMPT = """\
Tu es Hermes, le Dog nocturne de CYNIC. Tu analyses des repos externes
pour identifier ce qui est transférable à l'organisme. Tu tournes sur
GPU souverain. Pas de cloud, pas de raccourci.

## Posture épistémique
- φ⁻¹ = 0.618 max confidence. Tu ne claims jamais la certitude.
- Label chaque assertion : observed (lu dans le code), deduced (déduit),
  inferred (pattern), conjecture (hypothèse).
- Si tu ne peux pas dire ce qui FALSIFIERAIT ta conclusion, tu n'as pas
  de conclusion.

## Les 6 axiomes (grille d'analyse)
- FIDELITY : le repo fait-il ce qu'il claim ? Le README correspond-il au code ?
- PHI : la structure est-elle harmonieuse ou un patchwork ?
- VERIFY : y a-t-il des tests ? Sont-ils substantifs ou décoratifs ?
- CULTURE : les patterns sont-ils cohérents avec l'écosystème CYNIC (Rust, ports/adapters, hexagonal) ?
- BURN : est-ce plus simple que ce que CYNIC a déjà ? Ou est-ce de la complexité ajoutée ?
- SOVEREIGNTY : adopter ça crée-t-il une dépendance externe ? Peut-on le faire en local ?

## Output (JSON strict, pas de markdown, pas de commentaires)
{
  "repo_id": "...",
  "decision": "ADOPT|LEARN|STUDY|MIRROR|REJECT",
  "confidence": 0.0-0.618,
  "axiom_scores": {"fidelity": 0-1, "phi": 0-1, "verify": 0-1, "culture": 0-1, "burn": 0-1, "sovereignty": 0-1},
  "evidence": ["fichier:ligne — observation (epistemic_status)", ...],
  "transferable": "mécanisme précis transférable à CYNIC",
  "risk": "ce qui pourrait mal tourner si adopté",
  "falsification": "ce qui prouverait que cette évaluation est fausse",
  "notes": "synthèse 2-3 phrases"
}
"""


def load_traditions() -> str:
    """Load the 22 traditions as analytical enzymes."""
    for path in [TRADITIONS_PATH]:
        if path.exists():
            return path.read_text(encoding="utf-8")
    return ""


def clone_repo(full_name: str, dest: Path) -> tuple[bool, float]:
    """Shallow clone a GitHub repo. Returns (success, elapsed_seconds)."""
    url = f"https://github.com/{full_name}.git"
    t0 = time.time()
    try:
        subprocess.run(
            ["git", "clone", "--depth", "1", "--quiet", url, str(dest)],
            timeout=GENEROUS_TIMEOUT,
            capture_output=True,
            check=True,
        )
        elapsed = time.time() - t0
        log.info(f"  clone {full_name}: {elapsed:.1f}s")
        return True, elapsed
    except (subprocess.TimeoutExpired, subprocess.CalledProcessError) as e:
        elapsed = time.time() - t0
        log.warning(f"  clone {full_name} FAILED ({elapsed:.1f}s): {e}")
        return False, elapsed


def read_repo_content(repo_dir: Path, language: str) -> str:
    """Read key files from a cloned repo. Heuristic by language."""
    parts = []

    # README
    for name in ["README.md", "readme.md", "README.rst", "README"]:
        readme = repo_dir / name
        if readme.exists():
            text = readme.read_text(encoding="utf-8", errors="replace")[:4000]
            parts.append(f"=== {name} ===\n{text}")
            break

    # Tree structure
    try:
        tree = subprocess.run(
            ["find", str(repo_dir), "-maxdepth", "2", "-type", "f"],
            capture_output=True, text=True, timeout=10,
        )
        files = [
            line.replace(str(repo_dir) + "/", "")
            for line in tree.stdout.strip().split("\n")
            if line and not any(skip in line for skip in [".git/", "node_modules/", "__pycache__/", ".pyc"])
        ][:60]
        parts.append(f"=== Structure ({len(files)} files) ===\n" + "\n".join(files))
    except Exception:
        pass

    # Manifest + entry point by language
    key_files = {
        "Rust": ["Cargo.toml", "src/main.rs", "src/lib.rs"],
        "Python": ["setup.py", "pyproject.toml", "src/main.py", "app.py", "main.py"],
        "TypeScript": ["package.json", "src/index.ts", "index.ts"],
        "JavaScript": ["package.json", "src/index.js", "index.js"],
        "Go": ["go.mod", "main.go", "cmd/main.go"],
    }.get(language, ["README.md"])

    for kf in key_files:
        path = repo_dir / kf
        if path.exists():
            text = path.read_text(encoding="utf-8", errors="replace")[:3000]
            parts.append(f"=== {kf} ===\n{text}")

    combined = "\n\n".join(parts)
    # Rough token limit: ~10K tokens ≈ 40K chars
    return combined[:40000]


def call_inference(system: str, user: str) -> tuple[str | None, float]:
    """Call qwen35 via OpenAI-compatible API. Returns (response_text, elapsed)."""
    if not INFERENCE_URL:
        log.error("OPENAI_BASE_URL not set")
        return None, 0.0

    # INFERENCE_URL may or may not include /v1 — normalize
    base = INFERENCE_URL.rstrip("/")
    if base.endswith("/v1"):
        url = f"{base}/chat/completions"
    else:
        url = f"{base}/v1/chat/completions"
    payload = json.dumps({
        "model": INFERENCE_MODEL,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "max_tokens": 4096,
        "temperature": 0.3,
        "top_p": 0.9,
    }).encode()

    headers = {
        "Content-Type": "application/json",
    }
    if INFERENCE_KEY:
        headers["Authorization"] = f"Bearer {INFERENCE_KEY}"

    t0 = time.time()
    try:
        req = Request(url, data=payload, headers=headers, method="POST")
        with urlopen(req, timeout=GENEROUS_TIMEOUT) as resp:
            data = json.loads(resp.read())
        elapsed = time.time() - t0
        content = data.get("choices", [{}])[0].get("message", {}).get("content", "")
        log.info(f"  inference: {elapsed:.1f}s, ~{len(content)} chars response")
        return content, elapsed
    except Exception as e:
        elapsed = time.time() - t0
        log.error(f"  inference FAILED ({elapsed:.1f}s): {e}")
        return None, elapsed


def parse_analysis(raw: str) -> dict | None:
    """Extract JSON from LLM response. Tolerates thinking tags, markdown fences."""
    if not raw:
        return None
    text = raw.strip()

    # Strip Qwen3.5 thinking tags (thinking mode outputs <think>...</think> before response)
    import re
    text = re.sub(r"<think>.*?</think>", "", text, flags=re.DOTALL).strip()

    # Strip markdown fences
    if text.startswith("```"):
        lines = text.split("\n")
        lines = [l for l in lines if not l.strip().startswith("```")]
        text = "\n".join(lines).strip()

    # Try direct parse
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        pass

    # Try to find JSON object in the text
    start = text.find("{")
    end = text.rfind("}") + 1
    if start >= 0 and end > start:
        try:
            return json.loads(text[start:end])
        except json.JSONDecodeError:
            pass

    log.warning(f"  JSON parse failed, raw[:300]={text[:300]}...")
    return None


def post_observe(content: str, context: str, domain: str = "research") -> tuple[bool, float]:
    """POST /observe to the kernel."""
    if not KERNEL_ADDR:
        log.warning("  CYNIC_REST_ADDR not set, skipping /observe")
        return False, 0.0

    # Ensure protocol prefix
    addr = KERNEL_ADDR if KERNEL_ADDR.startswith("http") else f"http://{KERNEL_ADDR}"
    url = f"{addr}/observe"
    payload = json.dumps({
        "tool": "ouroboros_run",
        "target": content,
        "context": context,
        "domain": domain,
        "status": "completed",
    }).encode()

    headers = {
        "Content-Type": "application/json",
    }
    if KERNEL_KEY:
        headers["Authorization"] = f"Bearer {KERNEL_KEY}"

    t0 = time.time()
    try:
        req = Request(url, data=payload, headers=headers, method="POST")
        with urlopen(req, timeout=60) as resp:
            resp.read()
        elapsed = time.time() - t0
        log.info(f"  /observe: {elapsed:.1f}s")
        return True, elapsed
    except Exception as e:
        elapsed = time.time() - t0
        log.warning(f"  /observe FAILED ({elapsed:.1f}s): {e}")
        return False, elapsed


def analyze_repo(repo: dict, traditions: str) -> dict | None:
    """Full pipeline for one repo: clone → read → infer → observe → result."""
    repo_id = repo["repo_id"]
    full_name = repo["full_name"]
    language = repo.get("language", "Unknown")
    track = repo.get("track", "general")
    question = repo.get("nightly_question", f"What can CYNIC learn from {full_name}?")

    log.info(f"Analyzing {full_name} ({language}, track={track})")
    timings = {}

    # 1. Clone
    with tempfile.TemporaryDirectory(prefix=f"ouroboros-{repo_id}-") as tmpdir:
        dest = Path(tmpdir) / repo_id
        ok, t = clone_repo(full_name, dest)
        timings["clone"] = t
        if not ok:
            return None

        # 2. Read content
        t0 = time.time()
        content = read_repo_content(dest, language)
        timings["read"] = time.time() - t0

    # 3. Build prompt
    user_prompt = f"""## Les 22 traditions (enzymes d'analyse)

{traditions}

## Repo à analyser

Repo: {full_name} ({language})
Track: {track}
Question: {question}

{content}

Analyse ce repo selon les 6 axiomes et les traditions. Réponds en JSON strict."""

    # 4. Inference
    response, t = call_inference(SYSTEM_PROMPT, user_prompt)
    timings["inference"] = t

    # 5. Parse
    analysis = parse_analysis(response)
    if analysis is None and response:
        # Retry once with explicit JSON request
        log.info("  Retrying with explicit JSON instruction...")
        response2, t2 = call_inference(
            SYSTEM_PROMPT,
            user_prompt + "\n\nIMPORTANT: ta réponse doit être UN SEUL objet JSON, rien d'autre.",
        )
        timings["inference_retry"] = t2
        analysis = parse_analysis(response2)

    if analysis is None:
        log.warning(f"  {repo_id}: no valid analysis produced")
        return None

    # Ensure repo_id is set
    analysis["repo_id"] = repo_id
    analysis["full_name"] = full_name
    analysis["track"] = track
    analysis["language"] = language
    analysis["timings"] = timings
    analysis["total_elapsed"] = sum(timings.values())

    # 6. POST /observe
    observe_content = f"Hermes analysis: {full_name} → {analysis.get('decision', '?')} (confidence {analysis.get('confidence', 0):.2f})"
    observe_context = json.dumps({
        "agent": "hermes-ouroboros",
        "repo": full_name,
        "decision": analysis.get("decision"),
        "transferable": analysis.get("transferable", ""),
        "evidence_count": len(analysis.get("evidence", [])),
    })
    ok, t = post_observe(observe_content, observe_context, domain="research")
    timings["observe"] = t

    log.info(f"  {repo_id}: {analysis.get('decision', '?')} (confidence={analysis.get('confidence', 0):.2f}, total={analysis['total_elapsed']:.1f}s)")
    return analysis


def main():
    parser = argparse.ArgumentParser(description="Sovereign Ouroboros runner")
    parser.add_argument("--plan", required=True, help="Path to plan.json from scorecard")
    parser.add_argument("--report", required=True, help="Path to write report.json")
    parser.add_argument("--dry-run", action="store_true", help="Print plan without executing")
    args = parser.parse_args()

    # Load plan
    with open(args.plan) as f:
        plan = json.load(f)

    repos = plan.get("selected_repos", [])
    log.info(f"Plan: {plan.get('run_id', '?')} — {len(repos)} repos")

    if args.dry_run:
        for r in repos:
            print(f"  {r['full_name']} ({r['language']}) — {r['track']}")
        return

    # Validate env
    if not INFERENCE_URL:
        log.error("OPENAI_BASE_URL not set. Cannot run inference.")
        sys.exit(1)

    # Load traditions
    traditions = load_traditions()
    log.info(f"Traditions loaded: {len(traditions)} chars (~{len(traditions)//4} tokens)")

    # Run analysis
    run_start = time.time()
    results = []
    errors = 0

    for repo in repos:
        try:
            analysis = analyze_repo(repo, traditions)
            if analysis:
                results.append(analysis)
            else:
                errors += 1
        except Exception as e:
            log.error(f"Unexpected error on {repo.get('full_name', '?')}: {e}")
            errors += 1

    run_elapsed = time.time() - run_start

    # Build report (compatible with existing persist.py format)
    report = {
        "run": {
            "run_id": plan.get("run_id", "unknown"),
            "status": "completed" if results else "failed",
            "started_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(run_start)),
            "finished_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "duration_s": round(run_elapsed, 1),
            "agent_id": "hermes-ouroboros",
            "model": INFERENCE_MODEL,
            "backend_id": "qwen35-9b-gpu",
            "repos_attempted": len(repos),
            "repos_completed": len(results),
            "tool_failures": errors,
            "tests_run": 0,
            "tests_passed": 0,
        },
        "repo_results": [
            {
                "repo_id": r["repo_id"],
                "full_name": r["full_name"],
                "track": r["track"],
                "task_profile": "sovereign-analysis",
                "outcome": "analyzed",
                "decision": r.get("decision", "?"),
                "effort": "low",
                "confidence": r.get("confidence", 0),
                "evidence_count": len(r.get("evidence", [])),
                "exact_files_cited": len(r.get("evidence", [])),
                "stale_repo": False,
                "elapsed_s": r.get("total_elapsed", 0),
                "notes": r.get("notes", ""),
                "axiom_scores": r.get("axiom_scores", {}),
                "transferable": r.get("transferable", ""),
                "risk": r.get("risk", ""),
                "falsification": r.get("falsification", ""),
            }
            for r in results
        ],
    }

    # Write report
    with open(args.report, "w") as f:
        json.dump(report, f, indent=2, ensure_ascii=False)
    log.info(f"Report written to {args.report}")

    # Summary
    log.info(f"=== DONE === {len(results)}/{len(repos)} repos in {run_elapsed:.0f}s")
    for r in results:
        log.info(f"  {r['full_name']:40s} → {r.get('decision', '?'):6s} (conf={r.get('confidence', 0):.2f}, {r.get('total_elapsed', 0):.0f}s)")

    sys.exit(0 if results else 1)


if __name__ == "__main__":
    main()
