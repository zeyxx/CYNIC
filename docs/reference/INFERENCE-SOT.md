# CYNIC Inference — Source of Truth

> Single document for inference infrastructure parameters.
> Updated: 2026-03-30. Derived from 14 experiments in `benchmarks/SOVEREIGN-INFERENCE.md`.

---

## 1. Fleet Topology

| Node | Tailscale IP | Hardware | Role | Latency |
|------|-------------|----------|------|---------|
| **cynic-core** | `<TAILSCALE_CORE>` | Ryzen 7 5700G, 27GB RAM, ROCm iGPU (0 VRAM) | kernel, db, embedding | — |
| **cynic-gpu** | `<TAILSCALE_GPU>` | i5-14400F, RTX 4060 Ti 16GB, 16GB RAM | **all inference** | ~3ms |

## 2. Inference Backends — Crystallized Config

### Dog 1: `sovereign-gpu` (PRIMARY — all CYNIC scoring)

```toml
# backends.toml — SINGLE SOURCE OF TRUTH
[backend.sovereign-gpu]
base_url = "http://<TAILSCALE_GPU>:8080/v1"
model = "Qwen3.5-9B-Q4_K_M.gguf"
auth_style = "none"
context_size = 131072          # INCREASED: 131K (11.5GB VRAM) to support root workspace context
timeout_secs = 180             # EXP-4: thinking can use 120s+ at T=0.7
max_tokens = 16384             # EXP-4: enough for thinking + response
temperature = 0.7              # EXP-4: 100% JSON valid vs 50% at T=0.1
disable_thinking = false       # EXP-13: thinking REQUIRED for score calibration
cost_input_per_mtok = 0.0
cost_output_per_mtok = 0.0

[backend.sovereign-gpu.remediation]
node = "<USER>@<TAILSCALE_GPU>"
restart_command = "schtasks /run /tn CynicSovereign"
max_retries = 3
cooldown_secs = 60
```

**GPU Server Launch Parameters:**

```bash
# On cynic-gpu (Windows) — schtasks "CynicSovereign" must match these EXACTLY
llama-server.exe ^
  --model Qwen3.5-9B-Q4_K_M.gguf ^
  --n-gpu-layers 33 ^
  --ctx-size 131072 ^
  -np 1 ^
  --threads 8 ^
  --host 0.0.0.0 --port 8080
  # FA auto-enabled by b8422+ (fused Gated Delta Net)
  # NO --cache-type-k/v  (Qwen3.5 crashes with q4_0/q8_0 — see §4)
  # NO --reasoning-budget 0 (crashes server)
```

### Dog 2: `deterministic-dog` (FREE — no LLM)

Built-in. No config needed. Always available.

### Embedding: `llama-embed` (on cynic-core)

```bash
# systemd: llama-embed.service
llama-server \
  --model Qwen3-Embedding-0.6B-Q8_0.gguf \
  --host <TAILSCALE_CORE> --port 8081 \
  --threads 4 --ctx-size 8192 --parallel 4 \
  --n-gpu-layers 0 --embedding \
  --api-key-file ~/.config/cynic/llama-api-key
```

### ❌ REMOVED: Local inference on cynic-core

**Rationale:** cynic-core has 0 usable VRAM (iGPU only). Running 2 inference models on CPU consumed 6.2GB RAM and saturated swap (8GB/8GB = 100%). All inference must go to `cynic-gpu` (16GB dedicated VRAM).

---

## 3. Performance Reference (from 14 experiments)

### RTX 4060 Ti 16GB — Qwen3.5-9B Q4_K_M

| Metric | Value | Source |
|--------|-------|--------|
| **Generation speed** | 43-45 tok/s | EXP-2,13,14 (constant across ctx sizes) |
| **Prompt processing (PP)** | 2686 tok/s @ 512, 1750 @ 32K, 1226 @ 65K | EXP-2 |
| **PP with Flash Attention** | +31% @ 32K (auto-enabled b8422+) | EXP-8 |
| **VRAM @ ctx 32K, -np 1** | ~8306 MiB (8.1 GB free) | EXP-14 |
| **VRAM @ ctx 131K** | ~11520 MiB (4.8 GB free) | EXP-14 |
| **Max verified context** | 131072 tokens | EXP-14 |
| **Model native context** | 262144 tokens (OOM on 16GB) | EXP-14 |
| **KV cache cost (f16)** | ~17 MiB per 1K tokens | EXP-3 |
| **-np 1 vs -np 4** | 45 vs 18 tok/s (150% speedup) | EXP-14, R12 |

### Domain Routing (Application-Level)

| Domain | Profile | Key Params | Expected Time |
|--------|---------|-----------|---------------|
| Code review (CI) | FAST | T=0.7, top_k=20 | ~37s |
| Research/fact-check | BALANCED | T=0.7, top_k=40 | ~57-120s |
| Trading/KAIROS | DEEP | T=0.9, repeat_penalty=1.1 | ~100s |

---

## 4. KV Cache — State of the Art (March 2026)

### Current Status on CYNIC

| Cache Type | Qwen3.5 Status | Notes |
|------------|---------------|-------|
| **f16** (default) | ✅ Stable | Production config. 17 MiB/Kctx. |
| **bf16** | ✅ Stable | Equivalent to f16 on CUDA |
| **q8_0** | ❌ Crash/degrade | EXP-6: "failed to create context" on b8323/b8422 |
| **q4_0** | ❌ Crash/degrade | EXP-6: same failure |

### TurboQuant (Google DeepMind, ICLR 2026)

**What:** Training-free KV cache compression to 3-4 bits with near-lossless quality.

**How:** PolarQuant (Cartesian → polar coordinates) + QJL error correction (sign-bit residual projection).

**Impact:**
- 6x reduction in KV cache memory
- 8x faster attention logits computation (H100 benchmarks)
- ctx 32K → **192K+ on RTX 4060 Ti 16GB** (estimated: 17 MiB/Kctx ÷ 6 ≈ 2.8 MiB/Kctx)

**llama.cpp Integration Status:**
- PR #21089: CPU-based TBQ3_0/TBQ4_0 — **not yet merged**
- Community forks: `turboquant_plus` — available but experimental
- Expected timeline: weeks to months for upstream merge

### Upgrade Path

```
NOW (March 2026):     f16 cache → ctx 32K (8.3 GB VRAM) — APPLIED
NEXT (TurboQuant):    tbq4_0 cache → ctx 131K+ (same VRAM) — ACTIVE ON MERGE
FUTURE (NVFP4):       Hardware-native 4-bit → ctx 262K — Blackwell only
```

---

## 5. Experimental Rules (Crystallized)

| # | Rule | Source |
|---|------|--------|
| R1 | Test features in `llama-bench` BEFORE `llama-server` | EXP-1,2 |
| R2 | Use `schtasks` for persistent Windows processes via SSH | EXP-5 |
| R5 | `repeat_penalty` max 1.2 for Qwen3.5 (1.3 breaks JSON) | EXP-10 |
| R6 | Never use T≤0.2 with thinking mode (infinite loops) | EXP-4 |
| R7 | `top_p` ≥ 0.7 for Qwen3.5 (0.5 = timeout) | EXP-10 |
| R8 | `min_p=0.0` degrades quality significantly | EXP-10 |
| R9 | `--reasoning-budget N` for thinking control, not `enable_thinking` | EXP-12 |
| R11 | Measure before claiming — published specs ≠ local performance | All |
| R12 | `-np 1` for single-user inference (150% speedup) | EXP-14 |
| R13 | b8422+ auto-enables FA — do NOT pass `--flash-attn` manually | EXP-13 |
| R14 | Thinking REQUIRED for CYNIC score calibration | EXP-13 |

---

## 6. SoT File Hierarchy

```
# INFERENCE CONFIG (what the kernel reads)
~/.config/cynic/backends.toml       ← Dog definitions (model, URL, ctx, cost, remediation)

# ENVIRONMENT (secrets, addresses)
~/.cynic-env                        ← SINGLE source for all secrets + env vars
                                       Consumed by: systemd EnvironmentFile, scripts, shells

# SERVICE DEFINITIONS (how processes start)
~/.config/systemd/user/
├── cynic-kernel.service           ← Kernel (EnvironmentFile=~/.cynic-env)
├── surrealdb.service              ← Storage (EnvironmentFile=surrealdb.env)
├── llama-embed.service            ← Embedding (port 8081, CPU only)
└── [NO local llama-server.service] ← REMOVED: all inference on cynic-gpu

# GPU NODE (Windows, managed via schtasks)
CynicSovereign scheduled task      ← Must match backends.toml parameters EXACTLY
```

### ⚠ Deprecated (to remove)

| File | Why |
|------|-----|
| `~/.config/cynic/env` | Duplicate of `~/.cynic-env` — env.example says "deprecated" |
| `~/.config/cynic/remediation.toml` | Superseded by `[backend.*.remediation]` in backends.toml |
| `llama-server.service` (local) | No local inference — all goes to cynic-gpu |

---

## 7. Open Gaps

| # | Gap | Priority | Action |
|---|-----|----------|--------|
| G1 | GPU node runs ctx=2048, should be 32768 | 🔴 CRITICAL | Update `schtasks` CynicSovereign on Windows |
| G2 | TurboQuant not yet in upstream llama.cpp | ⏳ ACTIVE | Monitor PR #21089, test community fork when stable |
| G3 | Gemma orphan process on cynic-core (:8080) | 🔴 FIX NOW | `kill 3862`, disable llama-server.service |
| G4 | Swap 100% on cynic-core | 🟡 MEDIUM | Killing Gemma + llama-server frees ~6GB |
| G5 | Disk 86% on cynic-core | 🟡 MEDIUM | `cargo clean`, prune old backups |
| G6 | Domain tests not re-run with -np 1 | 🟡 MEDIUM | Re-run sovereign-bench.sh after G1 fix |
| G7 | `~/.config/cynic/env` still exists | 🟢 LOW | Delete after verifying nothing reads it |
