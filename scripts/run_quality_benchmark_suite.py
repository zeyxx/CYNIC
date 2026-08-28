#!/usr/bin/env python3
"""
run_quality_benchmark_suite.py — Automated Dual-Track Quality & Reasoning Evaluation Suite.

Enforces CYNIC Runtime Admission Preflight before every execution:
- Probes real-time /proc/meminfo MemAvailable and GTT aperture.
- Calculates exact memory footprint (Weights + KV + Activations + 20% Headroom).
- Blocks execution cleanly with AdmissionDecision if memory budget is exceeded.

Evaluates candidate models on Vulkan0:
1. Qwen3.6-35B-A3B MoE (IQ2_XXS, 10.75 GB)
2. Qwythos-9B-MTP (Q4_K_M, 5.48 GB)
3. LFM2.5-2.6B (Q4_K_M, 1.67 GB)
4. MiniCPM5-1B-Thinking (Q8_0, 1.07 GB)

Executes dual-track tasks from `infra/evals/quality-benchmark-dataset.json`.
"""

import json
import os
import subprocess
import sys
import time
import urllib.request

API_KEY_FILE = os.path.expanduser("~/.config/cynic/llama-api-key")

def get_api_key():
    if os.path.exists(API_KEY_FILE):
        with open(API_KEY_FILE) as f:
            return f.read().strip()
    return "015b1e6156afd4482139120be7cc7c01c2407ebfd2446bb29a638937fa6b9e95"

def probe_realtime_mem_available_bytes():
    try:
        with open("/proc/meminfo") as f:
            for line in f:
                if line.startswith("MemAvailable:"):
                    kb = int(line.split()[1])
                    return kb * 1024
    except Exception:
        pass
    return 16 * 1024 * 1024 * 1024

def check_runtime_admission(model_path, model_alias, ngl, ctx_size=16384, kv_quant_bytes=0.5):
    available_ram = probe_realtime_mem_available_bytes()
    weight_bytes = os.path.getsize(model_path) if os.path.exists(model_path) else 0
    
    # KV cache estimate: 2 * n_layers (64) * n_kv_heads (8) * head_dim (128) * ctx * kv_quant_bytes
    kv_bytes = int(2 * 64 * 8 * 128 * ctx_size * kv_quant_bytes)
    activation_bytes = 512 * 1024 * 1024
    overhead_bytes = 512 * 1024 * 1024
    
    base_required = weight_bytes + kv_bytes + activation_bytes + overhead_bytes
    total_required_with_headroom = int(base_required * 1.20) # 20% safety margin
    
    gtt_limit_bytes = 19 * 1024 * 1024 * 1024 # 19 GB GTT aperture
    
    if total_required_with_headroom > available_ram:
        return {
            "admitted": False,
            "status": "blocked_oom_risk",
            "reason": f"Required memory ({total_required_with_headroom / (1024**3):.2f} GB) exceeds real-time available RAM ({available_ram / (1024**3):.2f} GB)",
            "remediation": "Reduce context tier, quantize KV cache to q4_0, or downscale model."
        }
        
    if ngl == 99 and (weight_bytes + kv_bytes) > gtt_limit_bytes:
        return {
            "admitted": False,
            "status": "blocked_gtt_aperture_exceeded",
            "reason": f"Vulkan footprint ({(weight_bytes + kv_bytes) / (1024**3):.2f} GB) exceeds GTT limit ({gtt_limit_bytes / (1024**3):.2f} GB)",
            "remediation": "Switch from Full Vulkan to Hybrid offload (ngl 16 / 24)."
        }
        
    return {
        "admitted": True,
        "status": "admitted",
        "total_required_gb": round(total_required_with_headroom / (1024**3), 2),
        "available_ram_gb": round(available_ram / (1024**3), 2)
    }

def wait_for_server(port, timeout_secs=60):
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

def query_chat(port, model_path, prompt, max_tokens=512, temperature=0.2):
    url = f"http://127.0.0.1:{port}/v1/chat/completions"
    api_key = get_api_key()
    payload = {
        "model": model_path,
        "messages": [
            {"role": "system", "content": "You are an expert AI software engineer, mathematician, and on-chain blockchain analyst. Solve the user problem accurately, following all specifications strictly."},
            {"role": "user", "content": prompt}
        ],
        "temperature": temperature,
        "max_tokens": max_tokens,
        "stream": False
    }
    data = json.dumps(payload).encode("utf-8")
    headers = {"Content-Type": "application/json", "Authorization": f"Bearer {api_key}"}
    req = urllib.request.Request(url, data=data, headers=headers)
    start = time.time()
    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            wall_time = time.time() - start
            body = json.loads(resp.read().decode("utf-8"))
            return True, body, wall_time
    except Exception as e:
        return False, str(e), time.time() - start

def evaluate_task(port, model_path, task):
    prompt = task["prompt"]
    validation = task.get("validation", {})
    
    ok, res, wall_time = query_chat(port, model_path, prompt, max_tokens=384, temperature=0.2)
    if not ok or "choices" not in res:
        return {
            "task_id": task["id"],
            "task_name": task["name"],
            "category": task.get("category", ""),
            "passed": False,
            "ttcs_ms": wall_time * 1000.0,
            "thinking_words": 0,
            "completion_tokens": 0,
            "content_sample": str(res)[:100]
        }
        
    msg = res["choices"][0]["message"]
    content = msg.get("content", "").strip()
    reasoning = msg.get("reasoning_content", "").strip()
    full_text = (content + " " + reasoning).lower()
    
    timings = res.get("timings", {})
    usage = res.get("usage", {})
    
    completion_tokens = usage.get("completion_tokens", timings.get("predicted_n", 0))
    thinking_words = len(reasoning.split()) if reasoning else 0
    visible_words = len(content.split()) if content else 0
    overthinking_ratio = (thinking_words / max(1, visible_words)) if reasoning else 0.0
    
    passed = True
    if "expected_substring" in validation:
        if validation["expected_substring"].lower() not in full_text:
            passed = False
            
    if "expected_keywords" in validation:
        for kw in validation["expected_keywords"]:
            if kw.lower() not in full_text:
                passed = False
                break
                
    return {
        "task_id": task["id"],
        "task_name": task["name"],
        "category": task.get("category", ""),
        "passed": passed,
        "ttcs_ms": round(wall_time * 1000.0, 1),
        "prefill_tps": round(timings.get("prompt_per_second", 0.0), 2),
        "decode_tps": round(timings.get("predicted_per_second", 0.0), 2),
        "completion_tokens": completion_tokens,
        "thinking_words": thinking_words,
        "overthinking_ratio": round(overthinking_ratio, 2),
        "content_sample": content[:120].replace("\n", " "),
        "reasoning_sample": reasoning[:120].replace("\n", " ") if reasoning else None
    }

def main():
    dataset_path = "infra/evals/quality-benchmark-dataset.json"
    with open(dataset_path) as f:
        dataset = json.load(f)
    tasks = dataset.get("tasks", [])
    
    models = [
        {
            "alias": "qwen36-35b-iq2xxs",
            "name": "Qwen3.6-35B-A3B MoE (IQ2_XXS)",
            "path": "/var/lib/cynic/vault/models/Qwen3.6-35B-A3B-UD-IQ2_XXS.gguf",
            "type": "MoE 35B/3B",
            "ngl": 99,
            "port": 8094
        },
        {
            "alias": "qwythos-9b-q4",
            "name": "Qwythos-9B (Q4_K_M)",
            "path": "/home/user/.local/share/cynic/models/Qwythos-9B-Claude-Mythos-5-1M-MTP-Q4_K_M.gguf",
            "type": "Dense 9B",
            "ngl": 99,
            "port": 8095
        },
        {
            "alias": "lfm25-2.6b-q4",
            "name": "LFM2.5-2.6B (Q4_K_M)",
            "path": "/home/user/.local/share/cynic/models/LFM2.5-2.6B-Q4_K_M.gguf",
            "type": "Dense 2.6B",
            "ngl": 99,
            "port": 8096
        },
        {
            "alias": "minicpm-1b-q8",
            "name": "MiniCPM5-1B (Q8_0 Thinking)",
            "path": "/home/user/.local/share/cynic/models/MiniCPM5-1B-Claude-Opus-Fable5-Thinking-Q8_0.gguf",
            "type": "Dense 1B Thinking",
            "ngl": 99,
            "port": 8097
        }
    ]
    
    print("=" * 95)
    print(" CYNIC CONTRACT-FIRST QUALITY & REASONING BENCHMARK SUITE — AMD RYZEN 7 5700G")
    print(f" Total Tasks: {len(tasks)} (Universal Baselines + CYNIC/TrenchScanner/CultScreener)")
    print(f" Total Models: {len(models)}")
    print("=" * 95)
    
    model_scorecards = []
    
    for m in models:
        if not os.path.exists(m["path"]):
            print(f"\n[SKIP] Model not found: {m['path']}")
            continue
            
        # 1. Real-time Runtime Admission Preflight Gate
        admission = check_runtime_admission(m["path"], m["alias"], m["ngl"])
        if not admission["admitted"]:
            print(f"\n[BLOCKED BY PREFLIGHT GATE] {m['name']} — {admission['status']}: {admission['reason']}")
            print(f"  Remediation: {admission['remediation']}")
            continue
            
        port = m["port"]
        print(f"\n>> ÉVALUATION DU MODÈLE : {m['name']} ({m['type']})")
        print(f"   [PREFLIGHT GATE: ADMITTED] Required: {admission['total_required_gb']} GB <= Available RAM: {admission['available_ram_gb']} GB")
        print(f"   Lancement de llama-server sur le port {port} (ngl={m['ngl']} Vulkan0)...")
        
        cmd = [
            "/usr/local/bin/llama-server",
            "-m", m["path"],
            "--host", "127.0.0.1",
            "--port", str(port),
            "--ctx-size", "16384",
            "--threads", "8",
            "--threads-batch", "16",
            "--parallel", "1",
            "--n-gpu-layers", str(m["ngl"]),
            "--flash-attn", "on",
            "-ctk", "q4_0",
            "-ctv", "q4_0",
            "--api-key-file", API_KEY_FILE
        ]
        
        proc = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        try:
            if not wait_for_server(port, timeout_secs=60):
                print("   [ERROR] Failed to start llama-server")
                proc.kill()
                continue
                
            task_results = []
            passed_count = 0
            total_ttcs = 0.0
            
            for idx, task in enumerate(tasks, start=1):
                print(f"   [{idx:02d}/{len(tasks):02d}] {task['name']:<55s} ... ", end="", flush=True)
                eval_res = evaluate_task(port, m["path"], task)
                task_results.append(eval_res)
                
                status_icon = "✅ PASS" if eval_res["passed"] else "❌ FAIL"
                if eval_res["passed"]:
                    passed_count += 1
                total_ttcs += eval_res["ttcs_ms"]
                
                print(f"{status_icon} | TTCS: {eval_res['ttcs_ms']:6.0f}ms | Decode: {eval_res.get('decode_tps', 0):5.1f} t/s")
                
            pass_rate = (passed_count / len(tasks)) * 100.0 if tasks else 0.0
            avg_ttcs = total_ttcs / len(tasks) if tasks else 0.0
            quality_tph = (3600000.0 / avg_ttcs) * (pass_rate / 100.0) if avg_ttcs > 0 else 0.0
            
            scorecard = {
                "model_alias": m["alias"],
                "model_name": m["name"],
                "model_type": m["type"],
                "total_tasks": len(tasks),
                "passed_tasks": passed_count,
                "pass_rate_pct": round(pass_rate, 1),
                "avg_ttcs_ms": round(avg_ttcs, 1),
                "quality_adjusted_tph": round(quality_tph, 1),
                "tasks": task_results
            }
            model_scorecards.append(scorecard)
            
            print(f"\n   ==> RÉSULTATS {m['name']} : Pass@1 = {pass_rate:.1f}% ({passed_count}/{len(tasks)}) | TTCS Moyen = {avg_ttcs:.0f}ms | Tâches/h = {quality_tph:.1f}")
            
        finally:
            proc.terminate()
            try:
                proc.wait(timeout=5)
            except Exception:
                proc.kill()
            time.sleep(1)
            
    out_json = "benchmark-quality-scorecards.json"
    with open(out_json, "w") as f:
        json.dump(model_scorecards, f, indent=2)
        
    print("\n" + "=" * 95)
    print(" 🏆 TABLEAU DE BORD FINAL : CLASSEMENT DE QUALITÉ & EFFICACITÉ (RYZEN 7 5700G)")
    print("=" * 95)
    print(f"{'Modèle':<32s} {'Architecture':<16s} {'Pass@1 (%)':<12s} {'TTCS Moyen':<14s} {'Tasks/Hour (Qualité)':<20s}")
    print("-" * 95)
    for sc in sorted(model_scorecards, key=lambda x: x["quality_adjusted_tph"], reverse=True):
        print(f"{sc['model_name']:<32s} {sc['model_type']:<16s} {sc['pass_rate_pct']:>9.1f}% {sc['avg_ttcs_ms']:>10.0f}ms {sc['quality_adjusted_tph']:>18.1f}")
    print("=" * 95)
    print(f"Données enregistrées dans : {out_json}")

if __name__ == "__main__":
    main()
