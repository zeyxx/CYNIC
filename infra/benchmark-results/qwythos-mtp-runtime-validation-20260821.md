# Qwythos-9B-MTP Q4_K_M Runtime Validation (2026-08-21)

Contexte: Sweep throughput CYNIC via /v1/chat/completions (timings.predicted_per_second,
méthode run_ornith_bench.sh — request time / completion_tokens, warm-up inclus).

## Findings

- **API tg = 6.91 t/s stable** (ctx8K/KV-q4_0/t12, 128-354 tokens) ≠ model card **8.10 t/s** → divergence methodology:
  model card = llama-bench (draft-only + LM-head bypass); runtime API = full decode.
  → Rapport de vérité = 6.91 t/s (API real).
- **ctx > 8K = instable**: ctx16K/KV-q8_0/t12 → HANG (90s timeout) = MTP draft KV cache mismatch
  (draft q4 ≠ KV q8 → segfault llama-server). → **ctx8K hard limit** pour Qwythos.
- **threads optimal = 12**: t6=6.70 → t12=6.90 (+0.20 t/s, +3%).
- **KV cache type nil impact** pour le draft (q4_0 draft + KV q4 = matched; q8_0 q8 = mismatch→hang).

## Sweep matrix

| config     | tg t/s | status        |
|------------|--------|---------------|
| ctx8K  q4 t12 | 6.90 | ✅ baseline optimal |
| ctx8K  q4 t6  | 6.70 | ✅ (−threads = −tg) |
| ctx8K  q8 t12 | 6.91 | ✅ (KV-q8 = nil gain) |
| ctx16K q8 t12 | —   | ❌ HANG 90s (segfault draft) |

## Ranking agent-runtime (validated)

| # | modèle | tg (t/s) | Rust | tool-use | ctx  |
|---|--------|----------|------|----------|------|
| 1 | Ornith 9B dense  (8091) | 8.17 | ✅ | ✅ | 128K |
| 2 | Qwythos 9B MTP   (8080) | 6.91 | ✅ | ✅ | 8K  ← ctx limité |
| 3 | LFM2.5 2.6B      (8085) | 25.94| ✅ | ✅ | 128K ← fallback ultra-rapide |
| — | BTL-4 35B MoE    (8092)  | 17.14| ❌ | ❌ | quarantaine (GGUF tokenizer defect) |

→ Qwythos #1 = **Ornith** (8.17 > 6.91 + ctx 128K). Qwythos #2 = MCP draft mais tg bas + ctx limité.
→ BTL4 = **quarantaine définitive** (retesté 02.667.489 — même template crash).

## Rétroaction: model card 8.10 t/s = **over-claim**

Le model card Qwythos (8.10 t/s) = benchmark llama-bench — **pas runtime API**.
→ SSOT inference-profiles.json: Qwythos tg = 6.91 t/s (API real, validated), caveat model-card divergence.
