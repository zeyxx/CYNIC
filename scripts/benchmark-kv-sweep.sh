#!/usr/bin/env bash
# CYNIC inference benchmark — Multi-parameter KV Cache & Context Sweep
# Policy: infra/benchmark-kv-sweep-policy.json
#
# Usage:
#   ./scripts/benchmark-kv-sweep.sh                    # full sweep according to policy
#   ./scripts/benchmark-kv-sweep.sh --model qwen3-4b   # single model sweep
#   ./scripts/benchmark-kv-sweep.sh --ctx 65536         # 64K baseline only
#   ./scripts/benchmark-kv-sweep.sh --dry-run          # inspect commands without executing
# ─────────────────────────────────────────────────────────────────────

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo .)"
VAULT="${CYNIC_MODEL_VAULT:-$HOME/.local/share/cynic/models}"
RESULTS_DIR="$REPO_ROOT/infra/benchmark-results"
DATE=$(date +%Y-%m-%d)
BENCH_BIN="${LLAMA_BENCH_BIN:-llama-bench}"
DEVICE="${LLAMA_BENCH_DEVICE:-Vulkan0}"
GIT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
MESA_VERSION=$(glxinfo 2>/dev/null | grep -i "opengl version" | head -1 || echo "Vulkan RADV RENOIR")
CPU_GOV=$(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo "unknown")

DRY_RUN=0
MODEL_FILTER=""
CTX_FILTER=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=1 ;;
    --model)   MODEL_FILTER="$2"; shift ;;
    --ctx)     CTX_FILTER="$2"; shift ;;
    *)         echo "Unknown arg: $1"; exit 1 ;;
  esac
  shift
done

mkdir -p "$RESULTS_DIR"
OUTFILE="$RESULTS_DIR/kv-ctx-sweep-${DATE}.jsonl"
SUMMARY="$RESULTS_DIR/kv-ctx-sweep-${DATE}-summary.md"

echo "# CYNIC Multi-Objective Benchmark — $DATE" | tee "$SUMMARY"
echo "# Policy: infra/benchmark-kv-sweep-policy.json" | tee -a "$SUMMARY"
echo "# System: Git=$GIT_COMMIT | CPU_Gov=$CPU_GOV | Driver=$MESA_VERSION" | tee -a "$SUMMARY"
echo "# Vault: $VAULT" | tee -a "$SUMMARY"
echo "" | tee -a "$SUMMARY"

# Context sizes to sweep
CONTEXTS=(65536 131072 262144) # 64K baseline, 128K, 256K

# Models registry with parameter sizes
declare -A MODEL_FILES=(
  [qwythos-9b-q4]="Qwythos-9B-Claude-Mythos-5-1M-MTP-Q4_K_M.gguf"
  [qwen3-4b-instruct]="Qwen_Qwen3-4B-Instruct-2507-Q4_K_M.gguf"
  [minicpm-1b-q8]="MiniCPM5-1B-Claude-Opus-Fable5-Thinking-Q8_0.gguf"
  [bonsai-27b-q1]="Bonsai-27B-Q1_0.gguf"
  [shieldstral-3b-q8]="Shieldstral-1.0-3B-Q8_0.gguf"
  [lfm25-2b-q4]="LFM2.5-2.6B-Q4_K_M.gguf"
  [qwen36-35b-iq2]="Qwen3.6-35B-A3B-Uncensored-HauhauCS-Aggressive-IQ2_M.gguf"
  [vibethinker-3b-q4]="VibeThinker-3B-Q4_K_M.gguf"
  [qwen35-2b-q4]="Qwen3.5-2B-Q4_K_M.gguf"
  [qwen36-27b-iq2]="Qwen3.6-27B-Fable-Fus-711-UnHeretic-NM-DAU-NEO-MAX-NEO-MTP-IQ2_M.gguf"
)

# Standard KV types for all models
STD_KV=(f16 q8_0 q4_0 q4_1 iq4_nl q5_0 q5_1)
# Extra KV types for micro models <= 2B
MICRO_KV=(bf16 f32)

run_bench_combo() {
  local alias="$1"
  local model_file="$2"
  local ctx="$3"
  local kv="$4"
  local batch="$5"
  local ubatch="$6"
  local threads="$7"
  local model_path="$VAULT/$model_file"
  local ctx_k=$((ctx / 1024))

  if [[ ! -f "$model_path" ]]; then
    return 0
  fi

  if [[ -n "$CTX_FILTER" ]] && [[ "$ctx" != "$CTX_FILTER" ]]; then
    return 0
  fi

  local cmd=(
    "$BENCH_BIN"
    -m "$model_path"
    -p "$ctx"
    -n 128
    -b "$batch"
    -ub "$ubatch"
    -ctk "$kv"
    -ctv "$kv"
    -ngl 99
    -fa on
    -dev "$DEVICE"
    -t "$threads"
    -r 2
    -o jsonl
  )

  echo "▶ $alias | ctx=${ctx_k}K | kv=$kv | b=$batch ub=$ubatch t=$threads" | tee -a "$SUMMARY"

  if [[ "$DRY_RUN" == "1" ]]; then
    echo "  [DRY] ${cmd[*]}"
    return 0
  fi

  local meta
  meta=$(printf '{"cynic_alias":"%s","ctx_k":%d,"kv_type":"%s","batch":%d,"ubatch":%d,"threads":%d,"device":"%s","git_commit":"%s","date":"%s"}' \
    "$alias" "$ctx_k" "$kv" "$batch" "$ubatch" "$threads" "$DEVICE" "$GIT_COMMIT" "$DATE")

  if "${cmd[@]}" 2>&1 | while IFS= read -r line; do
    if [[ "$line" == "{"* ]]; then
      python3 -c "
import sys, json
try:
  d = json.loads('''$line''')
  meta = json.loads('''$meta''')
  d.update(meta)
  print(json.dumps(d))
except:
  pass
" >> "$OUTFILE"
    fi
  done; then
    echo "  ✅ completed" | tee -a "$SUMMARY"
  else
    echo "  ❌ failed (exit $?)" | tee -a "$SUMMARY"
  fi
}

for alias in "${!MODEL_FILES[@]}"; do
  if [[ -n "$MODEL_FILTER" ]] && [[ "$alias" != *"$MODEL_FILTER"* ]]; then
    continue
  fi

  model_file="${MODEL_FILES[$alias]}"

  # Determine KV types for model
  KV_LIST=("${STD_KV[@]}")
  if [[ "$alias" == *"1b"* ]] || [[ "$alias" == *"2b"* ]]; then
    KV_LIST+=("${MICRO_KV[@]}")
  fi

  for ctx in "${CONTEXTS[@]}"; do
    for kv in "${KV_LIST[@]}"; do
      # Test representative batch & thread combinations
      for batch in 512 2048; do
        for ubatch in 256; do
          for threads in 8 16; do
            run_bench_combo "$alias" "$model_file" "$ctx" "$kv" "$batch" "$ubatch" "$threads"
          done
        done
      done
    done
  done
done

echo "" | tee -a "$SUMMARY"
echo "Benchmark sweep finished. Results logged to $OUTFILE" | tee -a "$SUMMARY"
