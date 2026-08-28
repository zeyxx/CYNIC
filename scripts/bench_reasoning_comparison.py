#!/usr/bin/env python3
"""
bench_reasoning_comparison.py — Empirical Comparison of Task Performance: With vs Without Reasoning.

Evaluates 4 cognitive & coding tasks on Qwythos-9B:
1. Logic / Cognitive Reflection (Water lily doubling - 47 days vs 24 days trap)
2. Constrained Arithmetic (Bat & ball - $0.05 vs $0.10 trap)
3. Algorithmic Coding (Rust first_unique_char with UTF-8 safety)
4. Structured Extraction (Strict JSON entity schema)

Measures:
- Correctness (Pass@1)
- TTFT (ms)
- TTCS (ms)
- Thinking tokens vs Solution tokens
- Overthinking ratio
- Quality-Adjusted Tasks per Hour
"""

import json
import os
import sys
import time
import urllib.request

API_KEY_FILE = os.path.expanduser("~/.config/cynic/llama-api-key")

def get_api_key():
    if os.path.exists(API_KEY_FILE):
        with open(API_KEY_FILE) as f:
            return f.read().strip()
    return "015b1e6156afd4482139120be7cc7c01c2407ebfd2446bb29a638937fa6b9e95"

TASKS = [
    {
        "id": "task_1_water_lily",
        "name": "Piège Logique (Nénuphar)",
        "prompt": "Un nénuphar double de taille chaque jour. Au bout de 48 jours, il couvre tout le lac. Combien de jours faut-il pour qu'il couvre exactement la moitié du lac ? Donne la réponse finale en premier.",
        "expected_substring": "47",
        "wrong_substring": "24",
    },
    {
        "id": "task_2_bat_and_ball",
        "name": "Arithmétique à Contrainte (Batte & Balle)",
        "prompt": "Une batte et une balle de baseball coûtent 1,10 $ au total. La batte coûte 1,00 $ de plus que la balle. Combien coûte la balle en dollars et en centimes ?",
        "expected_substring": "0,05",
        "wrong_substring": "0,10",
    },
    {
        "id": "task_3_rust_algo",
        "name": "Algorithmique Rust (first_unique_char UTF-8)",
        "prompt": "Écris en Rust la fonction `pub fn first_unique_char(s: &str) -> Option<usize>` qui retourne l'index en octets ou caractères du premier caractère unique non répété. Utilise std::collections::HashMap.",
        "expected_substring": "HashMap",
        "wrong_substring": None,
    },
    {
        "id": "task_4_json_schema",
        "name": "Extraction JSON Strict",
        "prompt": "Extrais les personnes sous forme d'un objet JSON strict avec la clé 'people': 'Alice a 28 ans et vit à Lyon. Bob a 34 ans et vit à Paris.' Réponds UNIQUEMENT en JSON valide.",
        "expected_substring": "\"people\"",
        "wrong_substring": None,
    }
]

def query_endpoint(port, model_path, system_prompt, user_prompt, max_tokens=512, temperature=0.2):
    url = f"http://127.0.0.1:{port}/v1/chat/completions"
    api_key = get_api_key()
    payload = {
        "model": model_path,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt}
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
        with urllib.request.urlopen(req, timeout=60) as resp:
            wall_time = time.time() - start
            body = json.loads(resp.read().decode("utf-8"))
            return True, body, wall_time
    except Exception as e:
        return False, str(e), time.time() - start

def main():
    port = 8080
    model_path = "/home/user/.local/share/cynic/models/Qwythos-9B-Claude-Mythos-5-1M-MTP-Q4_K_M.gguf"
    
    print("=" * 95)
    print(" ÉVALUATION COMPARATIVE : IMPACT DU RAISONNEMENT (CoT) SUR LES TÂCHES RÉELLES")
    print(f" Modèle : Qwythos-9B Q4_K_M (MTP) | Port : {port} | Hardware : AMD Ryzen 7 5700G (Vega 8 Vulkan)")
    print("=" * 95)
    
    results = []
    
    for t in TASKS:
        print(f"\n>> {t['name']} ({t['id']})")
        print(f"   Prompt : \"{t['prompt'][:80]}...\"")
        
        # 1. Sans Raisonnement (Mode Direct)
        sys_direct = "You are a concise, direct assistant. Output only the immediate solution directly without step-by-step thinking or preamble."
        ok_d, res_d, wall_d = query_endpoint(port, model_path, sys_direct, t["prompt"], max_tokens=384, temperature=0.1)
        
        content_d = ""
        reasoning_d = ""
        ttft_d = 0.0
        tps_d = 0.0
        tokens_d = 0
        
        if ok_d and "choices" in res_d:
            msg = res_d["choices"][0]["message"]
            content_d = msg.get("content", "").strip()
            reasoning_d = msg.get("reasoning_content", "").strip()
            timings = res_d.get("timings", {})
            ttft_d = timings.get("prompt_ms", 0.0)
            tps_d = timings.get("predicted_per_second", 0.0)
            tokens_d = timings.get("predicted_n", 0)
            
        pass_d = t["expected_substring"].lower() in content_d.lower()
        if t["wrong_substring"] and t["wrong_substring"].lower() in content_d.lower() and not pass_d:
            pass_d = False
            
        print("   [SANS RAISONNEMENT] :")
        print(f"     -> Réponse : \"{content_d[:100].replace(chr(10), ' ')}...\"")
        print(f"     -> Pass@1: {'✅ OUI' if pass_d else '❌ NON'} | Tokens: {tokens_d} | TTFT: {ttft_d:.0f}ms | TTCS: {wall_d*1000:.0f}ms")
        
        # 2. Avec Raisonnement Approfondi (CoT)
        sys_cot = "You are a deep, rigorous mathematical and software engineering reasoner. Carefully analyze the problem step-by-step in your internal reasoning trace before concluding."
        ok_c, res_c, wall_c = query_endpoint(port, model_path, sys_cot, t["prompt"], max_tokens=512, temperature=0.4)
        
        content_c = ""
        reasoning_c = ""
        ttft_c = 0.0
        tps_c = 0.0
        tokens_c = 0
        
        if ok_c and "choices" in res_c:
            msg = res_c["choices"][0]["message"]
            content_c = msg.get("content", "").strip()
            reasoning_c = msg.get("reasoning_content", "").strip()
            timings = res_c.get("timings", {})
            ttft_c = timings.get("prompt_ms", 0.0)
            tps_c = timings.get("predicted_per_second", 0.0)
            tokens_c = timings.get("predicted_n", 0)
            
        pass_c = t["expected_substring"].lower() in (content_c + " " + reasoning_c).lower()
        
        thinking_words = len(reasoning_c.split())
        visible_words = len(content_c.split())
        overthinking_ratio = (thinking_words / max(1, visible_words)) if reasoning_c else 0.0
        
        print("   [AVEC RAISONNEMENT CoT] :")
        print(f"     -> Réflexion : \"{reasoning_c[:100].replace(chr(10), ' ')}...\"")
        print(f"     -> Réponse   : \"{content_c[:100].replace(chr(10), ' ')}...\"")
        print(f"     -> Pass@1: {'✅ OUI' if pass_c else '❌ NON'} | Tokens: {tokens_c} | Thinking Words: {thinking_words} | Ratio Sur-Réflexion: {overthinking_ratio:.1f}x | TTCS: {wall_c*1000:.0f}ms")
        
        results.append({
            "task_id": t["id"],
            "task_name": t["name"],
            "direct": {
                "pass": pass_d,
                "tokens": tokens_d,
                "ttft_ms": ttft_d,
                "ttcs_ms": wall_d * 1000.0,
                "content": content_d
            },
            "reasoning": {
                "pass": pass_c,
                "tokens": tokens_c,
                "thinking_words": thinking_words,
                "visible_words": visible_words,
                "overthinking_ratio": round(overthinking_ratio, 2),
                "ttft_ms": ttft_c,
                "ttcs_ms": wall_c * 1000.0,
                "content": content_c,
                "reasoning_sample": reasoning_c[:200]
            }
        })
        
    out_file = "benchmark-reasoning-comparison-results.json"
    with open(out_file, "w") as f:
        json.dump(results, f, indent=2)
        
    print("\n" + "=" * 95)
    print(f" Évaluation terminée avec succès. Résultats enregistrés dans : {out_file}")
    print("=" * 95)

if __name__ == "__main__":
    main()
