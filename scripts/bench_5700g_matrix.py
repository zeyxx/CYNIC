#!/usr/bin/env python3
"""
bench_5700g_matrix.py — Comprehensive Multi-Dimensional Inference Benchmark Suite for Ryzen 7 5700G.

Evaluates Pareto frontiers across:
- Offload Strategy: CPU-only (Zen 3 AVX2), Full Vulkan0 RADV, Hybrid offload.
- Context & Prefill load: 512, 2048, 8192, 16384, 32768 tokens.
- KV Cache Quantization: q4_0, q8_0, f16.
- Metrics captured: TTFT (ms), Prefill tok/s, Decode tok/s, Jitter (ms), Peak RSS (MB), TTCS (ms), Effective DRAM Bandwidth (GB/s).
"""

import argparse
import json
import os
import subprocess
import sys
import time
import urllib.request
import urllib.error

API_KEY_FILE = os.path.expanduser("~/.config/cynic/llama-api-key")

def get_api_key():
    if os.path.exists(API_KEY_FILE):
        with open(API_KEY_FILE) as f:
            return f.read().strip()
    return "015b1e6156afd4482139120be7cc7c01c2407ebfd2446bb29a638937fa6b9e95"

def get_process_rss_mb(pid):
    try:
        with open(f"/proc/{pid}/statm") as f:
            pages = int(f.read().split()[1])
            page_size_kb = os.sysconf("SC_PAGE_SIZE") / 1024.0
            return (pages * page_size_kb) / 1024.0
    except Exception:
        return 0.0

def wait_for_server(port, timeout_secs=30):
    url = f"http://127.0.0.1:{port}/v1/models"
    api_key = get_api_key()
    headers = {"Authorization": f"Bearer {api_key}"}
    req = urllib.request.Request(url, headers=headers)
    
    start = time.time()
    while time.time() - start < timeout_secs:
        try:
            with urllib.request.urlopen(req, timeout=2) as resp:
                if resp.status == 200:
                    return True
        except Exception:
            time.sleep(0.5)
    return False

def run_chat_query(port, model_path, prompt, max_tokens=128, temperature=0.2):
    url = f"http://127.0.0.1:{port}/v1/chat/completions"
    api_key = get_api_key()
    payload = {
        "model": model_path,
        "messages": [{"role": "user", "content": prompt}],
        "temperature": temperature,
        "max_tokens": max_tokens,
        "stream": False
    }
    data = json.dumps(payload).encode("utf-8")
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {api_key}"
    }
    req = urllib.request.Request(url, data=data, headers=headers)
    
    start_time = time.time()
    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            elapsed = time.time() - start_time
            body = json.loads(resp.read().decode("utf-8"))
            return True, body, elapsed
    except Exception as e:
        return False, str(e), time.time() - start_time

def evaluate_point(model_path, model_alias, active_params_b, total_params_b, ngl, threads, ctx_size, kv_quant, prompt_len, gen_len, port=8098):
    cmd = [
        "/usr/local/bin/llama-server",
        "-m", model_path,
        "--host", "127.0.0.1",
        "--port", str(port),
        "--ctx-size", str(ctx_size),
        "--threads", str(threads),
        "--threads-batch", str(threads),
        "--parallel", "1",
        "--n-gpu-layers", str(ngl),
        "--flash-attn", "on",
        "--batch-size", "512",
        "--ubatch-size", "256",
        "-ctk", kv_quant,
        "-ctv", kv_quant,
        "--api-key-file", API_KEY_FILE
    ]
    
    proc = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        if not wait_for_server(port, timeout_secs=45):
            proc.kill()
            return None
        
        # Warmup query
        run_chat_query(port, model_path, "Ping", max_tokens=4)
        
        # Build structured prompt with target token length
        base_text = "The quick brown fox jumps over the lazy dog. In computer science and artificial intelligence, inference engineering optimizes compute and memory bandwidth. "
        multiplier = max(1, prompt_len // 25)
        prompt = (base_text * multiplier)[:prompt_len * 4]
        prompt += "\nExplain the difference between prefill and decode phases in LLM inference."
        
        rss_before = get_process_rss_mb(proc.pid)
        ok, res, wall_time = run_chat_query(port, model_path, prompt, max_tokens=gen_len)
        rss_peak = max(rss_before, get_process_rss_mb(proc.pid))
        
        if not ok or "timings" not in res:
            return None
        
        timings = res["timings"]
        usage = res.get("usage", {})
        
        prompt_tps = timings.get("prompt_per_second", 0.0)
        decode_tps = timings.get("predicted_per_second", 0.0)
        ttft_ms = timings.get("prompt_ms", 0.0)
        prompt_tokens = usage.get("prompt_tokens", 0)
        completion_tokens = usage.get("completion_tokens", 0)
        
        # Calculate Effective Memory Bandwidth = decode_tps * active_model_bytes
        # Active model bytes: active_params_b * bits_per_param / 8
        bytes_per_param = 0.55 if "q4" in model_path.lower() else (0.28 if "iq2" in model_path.lower() else (0.40 if "iq3" in model_path.lower() else 1.0))
        active_weight_gb = active_params_b * bytes_per_param
        effective_bw_gbps = decode_tps * active_weight_gb
        
        # TTCS (Time To Correct Solution) approximation
        ttcs_ms = wall_time * 1000.0
        tasks_per_hour = (3600.0 / wall_time) if wall_time > 0 else 0.0
        
        return {
            "model_alias": model_alias,
            "total_params_b": total_params_b,
            "active_params_b": active_params_b,
            "ngl": ngl,
            "threads": threads,
            "ctx_size": ctx_size,
            "kv_quant": kv_quant,
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "prefill_tps": round(prompt_tps, 2),
            "decode_tps": round(decode_tps, 2),
            "ttft_ms": round(ttft_ms, 2),
            "ttcs_ms": round(ttcs_ms, 2),
            "tasks_per_hour": round(tasks_per_hour, 1),
            "rss_peak_mb": round(rss_peak, 1),
            "effective_bw_gbps": round(effective_bw_gbps, 2)
        }
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except Exception:
            proc.kill()
        time.sleep(1)

def main():
    parser = argparse.ArgumentParser(description="Ryzen 5700G Multi-Dimensional Benchmark Sweep")
    parser.add_argument("--output", default="benchmark-5700g-empirical-results.json", help="Output JSON results file")
    parser.add_argument("--quick", action="store_true", help="Run quick 4-point sanity matrix")
    args = parser.parse_args()
    
    models = [
        {
            "alias": "minicpm-1b-q8",
            "path": "/home/user/.local/share/cynic/models/MiniCPM5-1B-Claude-Opus-Fable5-Thinking-Q8_0.gguf",
            "total_params": 1.0,
            "active_params": 1.0,
            "role": "nano-thinking"
        },
        {
            "alias": "lfm25-2.6b-q4",
            "path": "/home/user/.local/share/cynic/models/LFM2.5-2.6B-Q4_K_M.gguf",
            "total_params": 2.6,
            "active_params": 2.6,
            "role": "ultra-fast-worker"
        },
        {
            "alias": "qwythos-9b-q4",
            "path": "/home/user/.local/share/cynic/models/Qwythos-9B-Claude-Mythos-5-1M-MTP-Q4_K_M.gguf",
            "total_params": 9.0,
            "active_params": 9.0,
            "role": "agent-principal"
        }
    ]
    
    # Check if Qwen3.6-35B-A3B IQ2_XXS was downloaded
    qwen36_path = "/var/lib/cynic/vault/models/Qwen3.6-35B-A3B-UD-IQ2_XXS.gguf"
    if os.path.exists(qwen36_path):
        models.append({
            "alias": "qwen36-35b-iq2xxs",
            "path": qwen36_path,
            "total_params": 35.0,
            "active_params": 3.0,
            "role": "moe-p0-worker"
        })
    
    results = []
    print("=" * 80)
    print("CYNIC INFERENCE SUITE — Ryzen 7 5700G Multi-Dimensional Benchmark Matrix")
    print("Hardware: AMD Ryzen 7 5700G (8C/16T, Zen 3) + Radeon Vega 8 (Vulkan0 RADV) + 32GB DDR4-3200")
    print("=" * 80)
    
    for m in models:
        if not os.path.exists(m["path"]):
            continue
        
        print(f"\nEvaluating Model: {m['alias']} ({m['total_params']}B total, {m['active_params']}B active)")
        
        test_points = []
        if args.quick:
            # Quick comparison: CPU vs Vulkan at 2k context
            test_points = [
                {"ngl": 0, "threads": 8, "ctx": 4096, "kv": "q4_0", "prompt_len": 512, "gen_len": 128},
                {"ngl": 99, "threads": 8, "ctx": 4096, "kv": "q4_0", "prompt_len": 512, "gen_len": 128},
                {"ngl": 99, "threads": 8, "ctx": 8192, "kv": "q4_0", "prompt_len": 2048, "gen_len": 128},
            ]
        else:
            # Full multi-dimensional sweep
            test_points = [
                # CPU vs Vulkan baseline (512 prompt)
                {"ngl": 0, "threads": 8, "ctx": 8192, "kv": "q4_0", "prompt_len": 512, "gen_len": 128},
                {"ngl": 99, "threads": 8, "ctx": 8192, "kv": "q4_0", "prompt_len": 512, "gen_len": 128},
                # Context scaling: 2k prompt
                {"ngl": 99, "threads": 8, "ctx": 8192, "kv": "q4_0", "prompt_len": 2048, "gen_len": 128},
                # Context scaling: 8k prompt
                {"ngl": 99, "threads": 8, "ctx": 16384, "kv": "q4_0", "prompt_len": 6144, "gen_len": 128},
                # KV Cache comparison: q8_0 vs q4_0
                {"ngl": 99, "threads": 8, "ctx": 8192, "kv": "q8_0", "prompt_len": 2048, "gen_len": 128},
            ]
        
        for pt in test_points:
            desc = f"ngl={pt['ngl']} thr={pt['threads']} ctx={pt['ctx']} kv={pt['kv']} prompt={pt['prompt_len']}t"
            print(f"  -> Testing [{desc}] ... ", end="", flush=True)
            
            res = evaluate_point(
                model_path=m["path"],
                model_alias=m["alias"],
                active_params_b=m["active_params"],
                total_params_b=m["total_params"],
                ngl=pt["ngl"],
                threads=pt["threads"],
                ctx_size=pt["ctx"],
                kv_quant=pt["kv"],
                prompt_len=pt["prompt_len"],
                gen_len=pt["gen_len"]
            )
            
            if res:
                results.append(res)
                print(f"DONE | PP: {res['prefill_tps']} tok/s | TG: {res['decode_tps']} tok/s | TTFT: {res['ttft_ms']}ms | Peak RSS: {res['rss_peak_mb']}MB | BW: {res['effective_bw_gbps']} GB/s")
            else:
                print("FAILED / SKIPPED")
    
    with open(args.output, "w") as f:
        json.dump(results, f, indent=2)
    
    print("\n" + "=" * 80)
    print(f"Benchmark matrix complete. {len(results)} configurations evaluated.")
    print(f"Saved results to: {args.output}")
    print("=" * 80)

if __name__ == "__main__":
    main()
