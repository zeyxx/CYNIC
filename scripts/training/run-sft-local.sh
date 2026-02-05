#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════
# CYNIC Dog 0 — Local SFT Training (llama.cpp CPU LoRA)
#
# Wraps llama.cpp finetune CLI to train LoRA adapters on CPU.
# Reads config from training-config.mjs via environment variables.
#
# Prerequisites:
#   1. llama.cpp built with finetune support
#      git clone https://github.com/ggml-org/llama.cpp
#      cd llama.cpp && cmake -B build && cmake --build build --config Release
#
#   2. Base GGUF model downloaded:
#      huggingface-cli download Qwen/Qwen2.5-1.5B-Instruct-GGUF \
#        qwen2.5-1.5b-instruct-q4_k_m.gguf --local-dir models/
#
#   3. Training data split (run split-data.mjs first):
#      training-splits/train.txt  (ChatML plain text)
#
# Usage:
#   bash scripts/training/run-sft-local.sh [--model path.gguf] [--data path.txt]
#
# "φ distrusts φ" — even the training process doubts itself
# ═══════════════════════════════════════════════════════════════════════════

set -euo pipefail

# ── Defaults (overridable via env or flags) ──────────────────────────────

LLAMA_FINETUNE="${LLAMA_CPP_PATH:-llama-finetune}"
MODEL_PATH="${1:-models/qwen2.5-1.5b-instruct-q4_k_m.gguf}"
TRAIN_DATA="${2:-training-splits/train.txt}"
CHECKPOINT_DIR="training-checkpoints/sft-local"

# Hyperparameters (from training-config.mjs sftLocal)
THREADS="${CYNIC_TRAIN_THREADS:-14}"
LORA_R=8
LORA_ALPHA=16
ADAM_ALPHA="3e-4"
BATCH_SIZE=4
CTX_LENGTH=384
EPOCHS=3
SAVE_EVERY=100

# ── Parse flags ──────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
  case $1 in
    --model) MODEL_PATH="$2"; shift 2;;
    --data) TRAIN_DATA="$2"; shift 2;;
    --threads) THREADS="$2"; shift 2;;
    --epochs) EPOCHS="$2"; shift 2;;
    --checkpoint-dir) CHECKPOINT_DIR="$2"; shift 2;;
    *) shift;;
  esac
done

# ── Validation ───────────────────────────────────────────────────────────

echo "═══════════════════════════════════════════════════════════════"
echo "  CYNIC Dog 0 — SFT Local Training (llama.cpp CPU LoRA)"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Check llama-finetune binary
if ! command -v "$LLAMA_FINETUNE" &>/dev/null; then
  # Try common build paths
  for candidate in \
    "llama.cpp/build/bin/llama-finetune" \
    "llama.cpp/build/Release/llama-finetune" \
    "llama.cpp/build/bin/Release/llama-finetune.exe" \
    "$HOME/llama.cpp/build/bin/llama-finetune"; do
    if [[ -f "$candidate" ]]; then
      LLAMA_FINETUNE="$candidate"
      break
    fi
  done
fi

if ! command -v "$LLAMA_FINETUNE" &>/dev/null && [[ ! -f "$LLAMA_FINETUNE" ]]; then
  echo "[sft] ERROR: llama-finetune not found."
  echo "[sft] Build it:"
  echo "  git clone https://github.com/ggml-org/llama.cpp"
  echo "  cd llama.cpp && cmake -B build && cmake --build build --config Release"
  echo "[sft] Or set LLAMA_CPP_PATH=/path/to/llama-finetune"
  exit 1
fi

# Check model file
if [[ ! -f "$MODEL_PATH" ]]; then
  echo "[sft] ERROR: Model not found at $MODEL_PATH"
  echo "[sft] Download it:"
  echo "  pip install huggingface-hub"
  echo "  huggingface-cli download Qwen/Qwen2.5-1.5B-Instruct-GGUF \\"
  echo "    qwen2.5-1.5b-instruct-q4_k_m.gguf --local-dir models/"
  exit 1
fi

# Check training data
if [[ ! -f "$TRAIN_DATA" ]]; then
  echo "[sft] ERROR: Training data not found at $TRAIN_DATA"
  echo "[sft] Run: node scripts/training/split-data.mjs"
  exit 1
fi

# Create checkpoint directory
mkdir -p "$CHECKPOINT_DIR"

# ── Display config ───────────────────────────────────────────────────────

TRAIN_SIZE=$(wc -c < "$TRAIN_DATA" 2>/dev/null || echo "?")
echo "[sft] Configuration:"
echo "  Model:        $MODEL_PATH"
echo "  Train data:   $TRAIN_DATA ($TRAIN_SIZE bytes)"
echo "  Threads:      $THREADS"
echo "  LoRA rank:    $LORA_R"
echo "  LoRA alpha:   $LORA_ALPHA"
echo "  Adam alpha:   $ADAM_ALPHA"
echo "  Batch size:   $BATCH_SIZE"
echo "  Context:      $CTX_LENGTH"
echo "  Epochs:       $EPOCHS"
echo "  Checkpoints:  $CHECKPOINT_DIR"
echo ""
echo "[sft] Starting training..."
echo "───────────────────────────────────────────────────────────────"

# ── Run training ─────────────────────────────────────────────────────────

"$LLAMA_FINETUNE" \
  --model-base "$MODEL_PATH" \
  --train-data "$TRAIN_DATA" \
  --threads "$THREADS" \
  --lora-r "$LORA_R" \
  --lora-alpha "$LORA_ALPHA" \
  --adam-alpha "$ADAM_ALPHA" \
  --batch "$BATCH_SIZE" \
  --ctx "$CTX_LENGTH" \
  --epochs "$EPOCHS" \
  --save-every "$SAVE_EVERY" \
  --checkpoint-in "$CHECKPOINT_DIR/checkpoint.gguf" \
  --checkpoint-out "$CHECKPOINT_DIR/checkpoint.gguf" \
  --lora-out "$CHECKPOINT_DIR/cynic-dog0-lora.gguf" \
  --sample-start "<|im_start|>" \
  2>&1 | tee "$CHECKPOINT_DIR/training.log"

# ── Done ─────────────────────────────────────────────────────────────────

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  SFT Training Complete"
echo "═══════════════════════════════════════════════════════════════"
echo "  LoRA adapter: $CHECKPOINT_DIR/cynic-dog0-lora.gguf"
echo "  Training log: $CHECKPOINT_DIR/training.log"
echo ""
echo "  Next: run GRPO refinement or deploy directly:"
echo "    node scripts/training/run-grpo-local.mjs"
echo "    bash scripts/training/deploy-ollama.sh"
echo "═══════════════════════════════════════════════════════════════"
