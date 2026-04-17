#!/usr/bin/env python3
"""
CYNIC Inference Profiler — measures real prompt_tokens / completion_tokens / latency / json_valid
per domain × Dog. Sends the EXACT same prompts the kernel builds.

Usage: python3 scripts/profile-dogs.py [--domain chess] [--dog gemma-4-e4b-core]
       Defaults to all 5 domains × 3 HTTP Dogs.
"""

import json
import os
import sys
import time
import argparse
from pathlib import Path

# ── Load secrets ──────────────────────────────────────────
def load_env():
    env_file = Path.home() / ".cynic-env"
    if env_file.exists():
        for line in env_file.read_text().splitlines():
            line = line.strip()
            if line and not line.startswith("#") and "=" in line:
                key, _, val = line.partition("=")
                val = val.strip().strip('"').strip("'")
                os.environ.setdefault(key.strip(), val)

load_env()

# ── Dog configs ───────────────────────────────────────────
DOGS = {
    "gemma-4-e4b-core": {
        "url": "http://100.74.31.10:8080/v1/chat/completions",
        "model": "gemma-4-E4B-it-Q4_K_M.gguf",
        "api_key": os.environ.get("GEMMA_CORE_API_KEY", ""),
        "context_size": 8192,
        "timeout": 120,
        "max_tokens": 2048,
        "temperature": 0.3,
        "disable_thinking": True,
        "json_mode": True,
    },
    "qwen35-9b-gpu": {
        "url": "http://100.119.192.107:8080/v1/chat/completions",
        "model": "Qwen3.5-9B-Q4_K_M.gguf",
        "api_key": os.environ.get("QWEN35_API_KEY", ""),
        "context_size": 131072,
        "timeout": 120,
        "max_tokens": 4096,
        "temperature": 0.3,
        "disable_thinking": True,
        "json_mode": True,
    },
    "qwen-7b-hf": {
        "url": "https://router.huggingface.co/v1/chat/completions",
        "model": "Qwen/Qwen2.5-7B-Instruct",
        "api_key": os.environ.get("HF_TOKEN", ""),
        "context_size": 32768,
        "timeout": 30,
        "max_tokens": 2048,
        "temperature": 0.3,
        "disable_thinking": False,
        "json_mode": False,
    },
}

# ── System prompt (exact copy from inference.rs:49-51) ────
SYSTEM_PROMPT = (
    "You are CYNIC, a sovereign epistemic judge. You evaluate THE SUBJECT MATTER "
    "described in the stimulus — not the quality of its description. In chess, judge "
    "the MOVE or STRATEGY, not the text. In science, judge the CLAIM, not the writing. "
    "Your axioms measure the SUBSTANCE, not the FORM.\n\n"
    "VERIFY means 'does this SURVIVE testing?' — not 'can you test it?' A strategy "
    "easily refuted by analysis scores LOW on VERIFY. A claim disproven by evidence "
    "scores LOW on VERIFY.\n\n"
    "Be harsh. Be honest. Overconfidence is the enemy. Most things deserve 0.3-0.6, "
    "not 0.8-0.9. The value 0.618 (phi^-1) is your absolute upper limit for confidence "
    "in any standard evaluation.\n\n"
    "SCORING RANGE: 0.05 to 1.0. Never return exactly 0.0 for any axiom. The minimum "
    "possible score is 0.05. A score of 0.0 means you failed to evaluate, not that the "
    "content is bad. Terrible content scores 0.05-0.15, not 0.0."
)

# ── Domain prompts (loaded from domains/*.md) ─────────────
DOMAIN_DIR = Path(__file__).resolve().parent.parent / "cynic-kernel" / "domains"

def load_domain_prompts():
    prompts = {}
    if not DOMAIN_DIR.exists():
        print(f"WARN: {DOMAIN_DIR} not found", file=sys.stderr)
        return prompts
    for md in DOMAIN_DIR.glob("*.md"):
        domain = md.stem
        prompts[domain] = md.read_text().strip()
    return prompts

# ── Representative stimuli per domain ─────────────────────
STIMULI = {
    "general": {
        "content": "LLMs are fundamentally incapable of reasoning — they merely pattern-match on training data. Any appearance of logical deduction is an illusion created by statistical correlation over vast text corpora.",
        "context": "(no additional context)",
    },
    "chess": {
        "content": "1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 5. O-O Be7 — Ruy Lopez, Closed. White develops harmoniously, castles early, maintains central tension. The bishop retreat to a4 keeps pressure on c6 while avoiding exchange.",
        "context": "Opening theory evaluation",
    },
    "dev": {
        "content": "Refactored evaluate_progressive() from 450 lines to 3 extracted functions: process_dog_result (handles circuit breaker + score accumulation), build_verdict (hash chain + consensus), emit_events (telemetry). Each function <100 lines. Tests cover all 3 individually. Main function now 80 lines of orchestration.",
        "context": "Kernel refactoring commit",
    },
    "trading": {
        "content": "BTC 4h: Price $67,200 (+2.1% 24h). RSI(14)=62 neutral-bullish. OI +$1.2B (24h), funding rate +0.012% (slight long bias). Volume 24h $38B (above 7d avg $31B). Hypothesis: continuation above $67,500 targets $69,000 with stop at $66,100. R:R = 1.8:1.",
        "context": "Binance perpetual futures analysis",
    },
    "token-analysis": {
        "content": "Token: $BONK (DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263). Supply: 93.5T (56.2T burned). Holders: 872,431. Top-1 wallet: 4.2% (Binance hot wallet). HHI: 0.008. Mint authority: REVOKED. Freeze authority: REVOKED. LP: Raydium, burned. Age: 782 days. 24h volume: $89M. Market cap: $1.8B.",
        "context": "Solana SPL token analysis from on-chain data",
    },
}

# ── Build user prompt (exact copy of inference.rs logic) ──
def build_user_prompt(domain, content, context, domain_prompts):
    ctx = context or "(no additional context)"

    if domain in domain_prompts:
        axioms_section = f"DOMAIN-SPECIFIC EVALUATION CRITERIA:\n{domain_prompts[domain]}"
    else:
        axioms_section = (
            "AXIOMS:\n"
            "1. FIDELITY — Is the SUBJECT MATTER itself sound? Judge the THING, not the accuracy of its description.\n"
            "2. PHI — Is this structurally harmonious? Well-coordinated? Proportional?\n"
            "3. VERIFY — Does this SURVIVE scrutiny? When tested against the strongest counterarguments, does it hold?\n"
            "4. CULTURE — Does this honor existing traditions, conventions, and established patterns?\n"
            "5. BURN — Is this efficient? Minimal waste? Could excess be destroyed without loss?\n"
            "6. SOVEREIGNTY — Does this preserve individual agency and freedom of choice?"
        )

    return (
        f"DOMAIN: {domain}\n"
        f"STIMULUS: {content}\n"
        f"CONTEXT: {ctx}\n\n"
        "IMPORTANT: Evaluate THE SUBJECT MATTER described — not how well it is described. "
        "A well-written description of a bad strategy is still a bad strategy. A poorly written "
        "description of a brilliant idea is still brilliant.\n\n"
        f"{axioms_section}\n\n"
        "STEP 1: In 2-3 sentences, analyze the QUALITY of the subject matter itself. "
        "What is strong? What is weak? Be specific and harsh.\n"
        "STEP 2: Based on your analysis, score each axiom 0.05-1.0. Your scores MUST reflect "
        "your analysis — if you identified weaknesses, the scores must be LOW.\n\n"
        "Output your analysis, then this exact JSON inside <json> tags "
        "(keep each reason under 15 words):\n"
        "<json>\n"
        '{"fidelity": 0.XX, "phi": 0.XX, "verify": 0.XX, "culture": 0.XX, "burn": 0.XX, '
        '"sovereignty": 0.XX, "fidelity_reason": "...", "phi_reason": "...", "verify_reason": "...", '
        '"culture_reason": "...", "burn_reason": "...", "sovereignty_reason": "..."}\n'
        "</json>"
    )


def estimate_tokens(text):
    """Rough estimate: ~4 chars/token (same as kernel)."""
    return len(text) // 4


def send_request(dog_name, dog_cfg, system, user):
    """Send chat completion request, return (response_dict, latency_ms) or (error_dict, latency_ms)."""
    import urllib.request
    import urllib.error

    messages = [
        {"role": "system", "content": system},
        {"role": "user", "content": user},
    ]

    # Kernel uses InferenceProfile::Scoring which caps max_tokens at 1024,
    # regardless of backend config. Match kernel behavior.
    scoring_max_tokens = 1024
    body = {
        "model": dog_cfg["model"],
        "messages": messages,
        "temperature": dog_cfg["temperature"],
        "max_tokens": scoring_max_tokens,
    }

    if dog_cfg.get("json_mode"):
        body["response_format"] = {"type": "json_object"}

    # Thinking disable for Qwen3 family (prepend /no_think)
    if dog_cfg.get("disable_thinking") and "qwen" in dog_cfg["model"].lower():
        messages[1]["content"] = "/no_think\n" + messages[1]["content"]

    # Gemma thinking disable via chat_template_kwargs
    if dog_cfg.get("disable_thinking") and "gemma" in dog_cfg["model"].lower():
        body["chat_template_kwargs"] = {"enable_thinking": False}

    data = json.dumps(body).encode("utf-8")
    req = urllib.request.Request(
        dog_cfg["url"],
        data=data,
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {dog_cfg['api_key']}",
            "User-Agent": "CYNIC-Profiler/1.0",
        },
        method="POST",
    )

    t0 = time.monotonic()
    try:
        with urllib.request.urlopen(req, timeout=dog_cfg["timeout"]) as resp:
            result = json.loads(resp.read().decode("utf-8"))
        latency_ms = int((time.monotonic() - t0) * 1000)
        return result, latency_ms
    except urllib.error.HTTPError as e:
        latency_ms = int((time.monotonic() - t0) * 1000)
        body_text = e.read().decode("utf-8", errors="replace")[:200]
        return {"error": f"HTTP {e.code}: {body_text}"}, latency_ms
    except Exception as e:
        latency_ms = int((time.monotonic() - t0) * 1000)
        return {"error": str(e)}, latency_ms


def extract_json_from_text(text):
    """Extract JSON from <json> tags or bare braces."""
    # Try <json> tags first
    start = text.find("<json>")
    if start != -1:
        end = text.find("</json>")
        if end != -1:
            return text[start + 6:end].strip()
        return text[start + 6:].strip()
    # Fallback: brace matching
    start = text.find("{")
    if start == -1:
        return None
    depth = 0
    for i, ch in enumerate(text[start:], start):
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return text[start:i + 1]
    return None


def validate_json(text):
    """Check if response contains valid axiom scores JSON."""
    json_str = extract_json_from_text(text)
    if not json_str:
        return False, "no_json"
    try:
        obj = json.loads(json_str)
    except json.JSONDecodeError:
        return False, "invalid_json"
    required = ["fidelity", "phi", "verify", "culture", "burn", "sovereignty"]
    missing = [k for k in required if k not in obj]
    if missing:
        return False, f"missing:{','.join(missing)}"
    for k in required:
        v = obj[k]
        if not isinstance(v, (int, float)):
            return False, f"non_numeric:{k}={v}"
        if v < 0.0 or v > 1.0:
            return False, f"out_of_range:{k}={v}"
    return True, "ok"


def profile_one(dog_name, dog_cfg, domain, stimulus, domain_prompts):
    """Profile a single domain × dog combination."""
    user = build_user_prompt(domain, stimulus["content"], stimulus.get("context"), domain_prompts)
    system = SYSTEM_PROMPT

    est_prompt = estimate_tokens(system) + estimate_tokens(user)
    ctx_budget = dog_cfg["context_size"]

    result = {
        "dog": dog_name,
        "domain": domain,
        "est_prompt_tokens": est_prompt,
        "ctx_budget": ctx_budget,
        "headroom": ctx_budget - est_prompt - dog_cfg["max_tokens"],
    }

    # Pre-flight check
    if est_prompt + 200 > ctx_budget:  # 200 = minimal completion
        result["status"] = "SKIP_OVERFLOW"
        result["prompt_tokens"] = 0
        result["completion_tokens"] = 0
        result["latency_ms"] = 0
        result["json_valid"] = False
        result["note"] = f"est {est_prompt} + 200 > ctx {ctx_budget}"
        return result

    resp, latency_ms = send_request(dog_name, dog_cfg, system, user)
    result["latency_ms"] = latency_ms

    if "error" in resp:
        result["status"] = "ERROR"
        result["prompt_tokens"] = 0
        result["completion_tokens"] = 0
        result["json_valid"] = False
        result["note"] = str(resp["error"])[:100]
        return result

    # Extract usage
    usage = resp.get("usage", {})
    result["prompt_tokens"] = usage.get("prompt_tokens", 0)
    result["completion_tokens"] = usage.get("completion_tokens", 0)

    # Extract text
    text = ""
    choices = resp.get("choices", [])
    if choices:
        msg = choices[0].get("message", {})
        text = msg.get("content", "")

    result["text_len"] = len(text)
    valid, reason = validate_json(text)
    result["json_valid"] = valid
    result["json_status"] = reason
    result["status"] = "OK" if valid else f"FAIL:{reason}"

    # Tokenizer ratio check
    if result["prompt_tokens"] > 0:
        actual_chars = len(system) + len(user)
        result["chars_per_token"] = round(actual_chars / result["prompt_tokens"], 2)

    return result


def main():
    parser = argparse.ArgumentParser(description="CYNIC Dog Profiler")
    parser.add_argument("--domain", help="Single domain to test")
    parser.add_argument("--dog", help="Single dog to test")
    parser.add_argument("--json", action="store_true", help="Output raw JSON")
    args = parser.parse_args()

    domain_prompts = load_domain_prompts()
    print(f"Loaded {len(domain_prompts)} domain prompts: {list(domain_prompts.keys())}", file=sys.stderr)

    domains = [args.domain] if args.domain else list(STIMULI.keys())
    dogs = {args.dog: DOGS[args.dog]} if args.dog else DOGS

    results = []

    for domain in domains:
        stimulus = STIMULI[domain]
        for dog_name, dog_cfg in dogs.items():
            label = f"{domain:16s} × {dog_name:20s}"
            print(f"  ▸ {label} ...", end="", flush=True, file=sys.stderr)

            result = profile_one(dog_name, dog_cfg, domain, stimulus, domain_prompts)
            results.append(result)

            status = result["status"]
            pt = result.get("prompt_tokens", 0)
            ct = result.get("completion_tokens", 0)
            lat = result.get("latency_ms", 0)
            print(f" {status:20s} pt={pt:5d} ct={ct:4d} lat={lat:5d}ms", file=sys.stderr)

    if args.json:
        print(json.dumps(results, indent=2))
        return

    # ── Pretty table ──────────────────────────────────────
    print()
    print(f"{'Domain':<18} {'Dog':<22} {'Status':<22} {'est_pt':>7} {'real_pt':>8} {'ct':>5} "
          f"{'lat_ms':>7} {'c/t':>5} {'json':>5} {'headroom':>9}")
    print("─" * 120)
    for r in results:
        print(
            f"{r['domain']:<18} {r['dog']:<22} {r['status']:<22} "
            f"{r['est_prompt_tokens']:>7} {r.get('prompt_tokens',0):>8} {r.get('completion_tokens',0):>5} "
            f"{r.get('latency_ms',0):>7} {r.get('chars_per_token','?'):>5} "
            f"{'✓' if r.get('json_valid') else '✗':>5} {r.get('headroom','?'):>9}"
        )

    # ── Summary stats ─────────────────────────────────────
    print()
    ok = [r for r in results if r["status"] == "OK"]
    total = len(results)
    print(f"Total: {total} probes, {len(ok)} OK, {total - len(ok)} failed/skipped")

    if ok:
        pts = [r["prompt_tokens"] for r in ok if r["prompt_tokens"] > 0]
        cts = [r["completion_tokens"] for r in ok if r["completion_tokens"] > 0]
        cpts = [r["chars_per_token"] for r in ok if "chars_per_token" in r]
        if pts:
            print(f"  prompt_tokens:  min={min(pts)} max={max(pts)} avg={sum(pts)//len(pts)}")
        if cts:
            print(f"  completion_tok: min={min(cts)} max={max(cts)} avg={sum(cts)//len(cts)} p95≈{sorted(cts)[int(len(cts)*0.95)]}")
        if cpts:
            print(f"  chars/token:    min={min(cpts)} max={max(cpts)} avg={sum(cpts)/len(cpts):.2f}")


if __name__ == "__main__":
    main()
