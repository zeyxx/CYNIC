"""Veille technologique on LLM model benchmarks via open APIs.

Sources:
  - LLMBenchmark.ai (free, no key, 500 req/day)
  - modelgrep.com (free, no key)
"""
import argparse
import json
import os
import re
import sys
import urllib.request
import urllib.error

LLMBENCH_URL = "https://api.llmbenchmark.ai/v1"
MODELGREP_URL = "https://modelgrep.com/api/v1"


def fetch_json(url, timeout=15):
    """Fetch JSON from URL."""
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "cynic-veille/1.0"})
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return json.loads(resp.read().decode())
    except (urllib.error.URLError, json.JSONDecodeError) as e:
        print(f"Warning: {url} failed: {e}", file=sys.stderr)
        return None


def get_llmbench_models():
    """Get model list from LLMBenchmark.ai."""
    data = fetch_json(f"{LLMBENCH_URL}/models")
    if not data:
        return []
    if isinstance(data, list):
        return data
    if isinstance(data, dict):
        return data.get("models", data.get("data", []))
    return []


def get_llmbench_scores(model_id):
    """Get scores for a specific model from LLMBenchmark.ai."""
    data = fetch_json(f"{LLMBENCH_URL}/scores?model={model_id}")
    if not data:
        return {}
    if isinstance(data, dict):
        return data
    return {}


def get_modelgrep_models():
    """Get model list from modelgrep.com."""
    data = fetch_json(f"{MODELGREP_URL}/models")
    if not data:
        return []
    if isinstance(data, list):
        return data
    if isinstance(data, dict):
        return data.get("models", data.get("data", []))
    return []


def normalize_name(name):
    """Normalize model name for matching."""
    if not name:
        return ""
    name = name.lower().strip()
    if "/" in name:
        name = name.split("/")[-1]
    name = name.split(":")[0]
    name = re.sub(r"[-_](free|latest|preview|beta|alpha)$", "", name)
    name = name.replace("_", "-")
    return name


def fuzzy_match(name, lookup):
    """Try exact match, then normalized match, then substring match."""
    if not name:
        return None
    if name in lookup:
        return lookup[name]
    norm = normalize_name(name)
    for key, val in lookup.items():
        if normalize_name(key) == norm:
            return val
    for key, val in lookup.items():
        nkey = normalize_name(key)
        if norm in nkey or nkey in norm:
            return val
    return None


def cross_reference(hermes_models):
    """Cross-reference Hermes models with benchmark data."""
    results = []
    llm_models = get_llmbench_models()
    mg_models = get_modelgrep_models()

    llm_lookup = {}
    for m in llm_models:
        name = (m.get("id") or m.get("name") or m.get("model") or "").lower()
        if name:
            llm_lookup[name] = m

    mg_lookup = {}
    for m in mg_models:
        name = (m.get("id") or m.get("name") or m.get("model") or "").lower()
        if name:
            mg_lookup[name] = m

    for hm in hermes_models:
        model_name = hm.get("model", "").lower()
        entry = {
            "model": hm.get("model"),
            "provider": hm.get("provider"),
            "free": hm.get("free", False),
            "llmbench": fuzzy_match(model_name, llm_lookup),
            "modelgrep": fuzzy_match(model_name, mg_lookup),
        }
        results.append(entry)

    return results


def main():
    parser = argparse.ArgumentParser(description="Veille technologique on LLM benchmarks")
    parser.add_argument("--json", action="store_true", help="Output as JSON")
    parser.add_argument("--free-only", action="store_true", help="Show only free models")
    parser.add_argument("--hermes-models", type=str, help="Path to hermes-model-detector JSON output")
    args = parser.parse_args()

    if args.hermes_models:
        with open(args.hermes_models) as f:
            hermes_models = json.load(f)
    else:
        sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "hermes-model-detector"))
        try:
            from detect import detect_models
            hermes_models = detect_models()
        except ImportError:
            print("Warning: could not import detect.py", file=sys.stderr)
            hermes_models = []

    if args.free_only:
        hermes_models = [m for m in hermes_models if m.get("free")]

    results = cross_reference(hermes_models)

    if args.json:
        print(json.dumps(results, indent=2))
    else:
        for r in results:
            print(f"\n{r['model']} ({r['provider']}){' [FREE]' if r['free'] else ''}")
            if r["llmbench"]:
                print(f"  LLMBenchmark: {json.dumps(r['llmbench'], indent=2)[:200]}")
            if r["modelgrep"]:
                print(f"  modelgrep: {json.dumps(r['modelgrep'], indent=2)[:200]}")
            if not r["llmbench"] and not r["modelgrep"]:
                print("  No benchmark data found")


if __name__ == "__main__":
    main()