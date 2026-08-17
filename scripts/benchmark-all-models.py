#!/usr/bin/env python3
"""
CYNIC Sovereign Inference Benchmark Suite
Pushes local GGUF models in vault to their limits across coding & agentic tasks.
"""

import json
import time
import urllib.request
import subprocess

MODELS = [
    {"alias": "minicpm-1b-q8", "name": "MiniCPM-1B (Q8_0)", "ctx": 8192},
    {"alias": "qwen3-4b-instruct", "name": "Qwen3-4B-Instruct (Q4_K_M)", "ctx": 8192},
    {"alias": "qwythos-9b-q4", "name": "Qwythos-9B-Claude-Mythos (Q4_K_M)", "ctx": 8192},
    {"alias": "bonsai-27b-q1", "name": "Bonsai-27B (Q1_0)", "ctx": 65536},
]

PROMPTS = [
    {
        "kind": "coding",
        "prompt": "Écris une fonction Rust générique thread-safe avec RwLock et HashMap qui implémente un cache LRU.",
        "max_tokens": 150,
    },
    {
        "kind": "reasoning",
        "prompt": "Explique les principes de la souveraineté numérique et le fonctionnement d'un runtime souverain.",
        "max_tokens": 150,
    },
]

def run_command(cmd):
    res = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    return res.stdout, res.returncode

def start_model(alias):
    print(f"==> Starting model: {alias}...")
    out, code = run_command(f"cynic runtime inference-start {alias} --json")
    if code != 0:
        print(f"Error starting {alias}: {out}")
        return False
    return True

def query_completion(prompt, max_tokens):
    api_key_file = "/home/user/.config/cynic/llama-api-key"
    try:
        with open(api_key_file) as f:
            api_key = f.read().strip()
    except Exception:
        api_key = ""

    url = "http://127.0.0.1:8080/v1/chat/completions"
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {api_key}"
    }
    payload = {
        "model": "default",
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": max_tokens,
    }
    
    req = urllib.request.Request(url, data=json.dumps(payload).encode(), headers=headers)
    t0 = time.time()
    with urllib.request.urlopen(req) as resp:
        t1 = time.time()
        data = json.loads(resp.read().decode())
        data["wall_time_s"] = t1 - t0
        return data

def main():
    report = {
        "schema": "cynic.inference.benchmark_report.v1",
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "hardware": {
            "cpu": "AMD Ryzen 7 5700G (8C/16T)",
            "ram": "32 GB DDR4-3200 Dual-Channel (51.2 GB/s max bandwidth)",
            "gpu": "AMD Radeon Graphics (RADV RENOIR Vulkan)",
        },
        "results": []
    }

    for m in MODELS:
        alias = m["alias"]
        if not start_model(alias):
            continue

        model_res = {
            "alias": alias,
            "name": m["name"],
            "ctx_size": m["ctx"],
            "benchmarks": []
        }

        for p in PROMPTS:
            print(f"    Running prompt ({p['kind']})...")
            try:
                res = query_completion(p["prompt"], p["max_tokens"])
                timings = res.get("timings", {})
                usage = res.get("usage", {})
                bench = {
                    "kind": p["kind"],
                    "prompt_tokens": usage.get("prompt_tokens", 0),
                    "completion_tokens": usage.get("completion_tokens", 0),
                    "prompt_tps": timings.get("prompt_per_second", 0),
                    "eval_tps": timings.get("predicted_per_second", 0),
                    "wall_time_s": round(res.get("wall_time_s", 0), 2),
                    "sample_output": res["choices"][0]["message"].get("content", "")[:120]
                }
                model_res["benchmarks"].append(bench)
                print(f"    -> Prompt: {bench['prompt_tps']:.2f} t/s, Eval: {bench['eval_tps']:.2f} t/s")
            except Exception as e:
                print(f"    -> Error: {e}")

        report["results"].append(model_res)

    report_file = "infra/benchmark-report.json"
    with open(report_file, "w") as f:
        json.dump(report, f, indent=2)
    print(f"\n[+] Benchmark suite completed! Report saved to {report_file}")

if __name__ == "__main__":
    main()
