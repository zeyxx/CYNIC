#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════
# CYNIC Dog 0 — Deploy to Ollama
#
# Registers the fine-tuned model (base + LoRA or merged GGUF) in Ollama
# as "cynic-dog0", ready for CollectiveLearner to pick up.
#
# Supports two deployment modes:
#   A. LoRA adapter (base model + separate LoRA GGUF)
#   B. Merged GGUF (single file, pre-merged)
#
# Usage:
#   bash scripts/training/deploy-ollama.sh [--lora path] [--gguf path] [--base path]
#
# After deployment, set CYNIC_DOG0_MODEL=cynic-dog0 to activate.
#
# "φ distrusts φ" — deploy with caution
# ═══════════════════════════════════════════════════════════════════════════

set -euo pipefail

# ── Defaults ─────────────────────────────────────────────────────────────

MODEL_NAME="cynic-dog0"
OLLAMA_ENDPOINT="${OLLAMA_URL:-http://localhost:11434}"
BASE_MODEL="${1:-models/qwen2.5-1.5b-instruct-q4_k_m.gguf}"
LORA_PATH=""
GGUF_PATH=""
MODELFILE_DIR="training-checkpoints/deploy"

# ── Parse flags ──────────────────────────────────────────────────────────

shift 2>/dev/null || true
while [[ $# -gt 0 ]]; do
  case $1 in
    --lora) LORA_PATH="$2"; shift 2;;
    --gguf) GGUF_PATH="$2"; shift 2;;
    --base) BASE_MODEL="$2"; shift 2;;
    --name) MODEL_NAME="$2"; shift 2;;
    *) shift;;
  esac
done

# Auto-detect LoRA if not specified
if [[ -z "$LORA_PATH" && -z "$GGUF_PATH" ]]; then
  # Check common locations
  for candidate in \
    "training-checkpoints/sft-local/cynic-dog0-lora.gguf" \
    "training-checkpoints/grpo-local/cynic-dog0-lora.gguf"; do
    if [[ -f "$candidate" ]]; then
      LORA_PATH="$candidate"
      break
    fi
  done
fi

# ── Display ──────────────────────────────────────────────────────────────

echo "═══════════════════════════════════════════════════════════════"
echo "  CYNIC Dog 0 — Ollama Deployment"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "  Model name:   $MODEL_NAME"
echo "  Endpoint:     $OLLAMA_ENDPOINT"

if [[ -n "$GGUF_PATH" ]]; then
  echo "  Mode:         Merged GGUF"
  echo "  GGUF:         $GGUF_PATH"
elif [[ -n "$LORA_PATH" ]]; then
  echo "  Mode:         Base + LoRA adapter"
  echo "  Base:         $BASE_MODEL"
  echo "  LoRA:         $LORA_PATH"
else
  echo ""
  echo "  ERROR: No model artifact found."
  echo "  Provide one of:"
  echo "    --gguf path/to/merged.gguf"
  echo "    --lora path/to/lora.gguf"
  echo ""
  echo "  Or run training first:"
  echo "    bash scripts/training/run-sft-local.sh"
  exit 1
fi

echo ""

# ── Check Ollama ─────────────────────────────────────────────────────────

if ! command -v ollama &>/dev/null; then
  echo "[deploy] ERROR: 'ollama' CLI not found in PATH."
  echo "[deploy] Install: https://ollama.com/download"
  exit 1
fi

# Verify Ollama is running
if ! curl -s "${OLLAMA_ENDPOINT}/api/tags" >/dev/null 2>&1; then
  echo "[deploy] ERROR: Ollama not responding at ${OLLAMA_ENDPOINT}"
  echo "[deploy] Start it: ollama serve"
  exit 1
fi

echo "[deploy] Ollama is running."

# ── Create Modelfile ─────────────────────────────────────────────────────

mkdir -p "$MODELFILE_DIR"
MODELFILE="$MODELFILE_DIR/Modelfile"

if [[ -n "$GGUF_PATH" ]]; then
  # Mode A: Merged GGUF
  RESOLVED_PATH="$(realpath "$GGUF_PATH" 2>/dev/null || echo "$GGUF_PATH")"
  cat > "$MODELFILE" <<HEREDOC
FROM ${RESOLVED_PATH}
TEMPLATE """{{- if .System }}<|im_start|>system
{{ .System }}<|im_end|>
{{ end }}<|im_start|>user
{{ .Prompt }}<|im_end|>
<|im_start|>assistant
"""
PARAMETER temperature 0.3
PARAMETER num_predict 256
PARAMETER stop "<|im_end|>"
SYSTEM "You are CYNIC (κυνικός), a cynical judgment system. Score items on 25 dimensions across 4 axioms (PHI, VERIFY, CULTURE, BURN). Your confidence never exceeds 61.8% (φ⁻¹). Be direct, skeptical, honest. Respond with JSON: {\"score\": 0-100, \"confidence\": 0.0-0.618, \"verdict\": \"HOWL|WAG|GROWL|BARK\", \"reasoning\": \"brief\"}"
HEREDOC

elif [[ -n "$LORA_PATH" ]]; then
  # Mode B: Base + LoRA
  RESOLVED_BASE="$(realpath "$BASE_MODEL" 2>/dev/null || echo "$BASE_MODEL")"
  RESOLVED_LORA="$(realpath "$LORA_PATH" 2>/dev/null || echo "$LORA_PATH")"
  cat > "$MODELFILE" <<HEREDOC
FROM ${RESOLVED_BASE}
ADAPTER ${RESOLVED_LORA}
TEMPLATE """{{- if .System }}<|im_start|>system
{{ .System }}<|im_end|>
{{ end }}<|im_start|>user
{{ .Prompt }}<|im_end|>
<|im_start|>assistant
"""
PARAMETER temperature 0.3
PARAMETER num_predict 256
PARAMETER stop "<|im_end|>"
SYSTEM "You are CYNIC (κυνικός), a cynical judgment system. Score items on 25 dimensions across 4 axioms (PHI, VERIFY, CULTURE, BURN). Your confidence never exceeds 61.8% (φ⁻¹). Be direct, skeptical, honest. Respond with JSON: {\"score\": 0-100, \"confidence\": 0.0-0.618, \"verdict\": \"HOWL|WAG|GROWL|BARK\", \"reasoning\": \"brief\"}"
HEREDOC
fi

echo "[deploy] Modelfile written → $MODELFILE"
echo ""
cat "$MODELFILE"
echo ""

# ── Register in Ollama ───────────────────────────────────────────────────

echo "[deploy] Creating model '${MODEL_NAME}' in Ollama..."
ollama create "$MODEL_NAME" -f "$MODELFILE"

echo ""
echo "[deploy] Verifying model..."
ollama show "$MODEL_NAME" --modelfile 2>/dev/null || true

# ── Quick smoke test ─────────────────────────────────────────────────────

echo ""
echo "[deploy] Running smoke test..."
SMOKE_RESULT=$(curl -s "${OLLAMA_ENDPOINT}/api/generate" \
  -d "{\"model\": \"${MODEL_NAME}\", \"prompt\": \"<|im_start|>system\nYou are CYNIC. Respond with JSON.\n<|im_end|>\n<|im_start|>user\nJudge this test item: {\\\"type\\\": \\\"smoke_test\\\"}\n<|im_end|>\n<|im_start|>assistant\", \"stream\": false, \"options\": {\"num_predict\": 128}}" \
  2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin).get('response','ERROR'))" 2>/dev/null || echo "SMOKE TEST SKIPPED")

echo "  Response: ${SMOKE_RESULT:0:200}"

# ── Done ─────────────────────────────────────────────────────────────────

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  Deployment Complete"
echo "═══════════════════════════════════════════════════════════════"
echo "  Model: ${MODEL_NAME}"
echo "  Modelfile: ${MODELFILE}"
echo ""
echo "  Activate Dog 0:"
echo "    export CYNIC_DOG0_MODEL=${MODEL_NAME}"
echo ""
echo "  Test manually:"
echo "    ollama run ${MODEL_NAME}"
echo ""
echo "  Evaluate:"
echo "    node scripts/training/evaluate.mjs --model ${MODEL_NAME}"
echo "═══════════════════════════════════════════════════════════════"
