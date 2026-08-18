#!/bin/bash
# CYNIC benchmark runner for qwen38-27b via llama-server API
# Usage: run_qwen38_bench.sh <kv_type> <ctx_size> <threads> <batch_threads> <label>
# KV types: f16, q8_0, q4_0, iq4_nl

set -uo pipefail

MODEL="/var/lib/cynic/.local/share/cynic/models/Qwen3.8-27B-IQ3_XXS.gguf"
PORT=8090
KBENCH_DIR="/tmp/bench-results"
mkdir -p "$KBENCH_DIR"

KV_TYPE="${1:?usage: kv_type [f16|q8_0|q4_0|iq4_nl]}"
CTX_SIZE="${2:?usage: ctx_size}"
THREADS="${3:?usage: threads}"
THREADS_BATCH="${4:?usage: threads_batch}"
LABEL="${5:?usage: label}"

# Map KV type to ctk/ctv value
case "$KV_TYPE" in
  f16)    CTK="f16" ;;
  q8_0)   CTK="q8_0" ;;
  q4_0)   CTK="q4_0" ;;
  iq4_nl) CTK="iq4_nl" ;;
  *) echo "Unknown KV type: $KV_TYPE"; exit 1 ;;
esac

echo "[cynic-bench] START $LABEL: kv=$KV_TYPE ctx=$CTX_SIZE t=$THREADS tb=$THREADS_BATCH ctk=$CTK"

# Stop CYNIC-managed server if running (safe kill)
/home/user/.local/bin/cynic runtime inference-stop qwen38-27b --json 2>/dev/null || true
# Kill any orphaned llama-server on the port
PIDS=$(pgrep -f "llama-server.*:${PORT}" 2>/dev/null || true)
if [ -n "$PIDS" ]; then
  echo "$PIDS" | xargs kill 2>/dev/null || true
  sleep 2
fi
PIDS=$(pgrep -f "llama-server.*:${PORT}" 2>/dev/null || true)
if [ -n "$PIDS" ]; then
  echo "$PIDS" | xargs kill -9 2>/dev/null || true
  sleep 1
fi

# Get API key
KEY=$(sudo cat /home/user/.config/cynic/llama-api-key 2>/dev/null || echo "testkey")

# Start llama-server
/usr/local/bin/llama-server \
  --model "$MODEL" \
  --host 127.0.0.1 --port $PORT \
  --threads "$THREADS" \
  --threads-batch "$THREADS_BATCH" \
  --parallel 1 \
  --n-gpu-layers 99 \
  --device Vulkan0 \
  -ctk "$CTK" -ctv "$CTK" \
  --ctx-size "$CTX_SIZE" \
  --flash-attn auto \
  --batch-size 512 \
  --ubatch-size 256 \
  2>/tmp/bench-server-${LABEL}.err &

SERVER_PID=$!
echo "[cynic-bench] server pid=$SERVER_PID, waiting for health..."

# Wait for health (up to 120s)
HEALTHY=0
for i in $(seq 1 60); do
  if curl -sf "http://127.0.0.1:${PORT}/health" 2>/dev/null | grep -q "ok"; then
    echo "[cynic-bench] server healthy after $((i*2))s"
    HEALTHY=1
    break
  fi
  sleep 2
  if ! kill -0 $SERVER_PID 2>/dev/null; then
    echo "[cynic-bench] ERROR: server died (pid=$SERVER_PID)"
    echo "=== stderr ===" && cat /tmp/bench-server-${LABEL}.err 2>/dev/null | tail -20
    HEALTHY=0
    break
  fi
done

if [ "$HEALTHY" != "1" ]; then
  echo "[cynic-bench] FATAL: server never became healthy"
  cat /tmp/bench-server-${LABEL}.err 2>/dev/null | tail -30
  exit 1
fi

sleep 10  # warm-up

PROMPT="Explique-moi la théorie de la relativité en 3 phrases simples."
N_GEN=128

for rep in 1 2 3; do
  RESP=$(curl -sf -X POST "http://127.0.0.1:${PORT}/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $KEY" \
    -d "{
      \"model\": \"qwen38-27b\",
      \"messages\": [{\"role\": \"user\", \"content\": \"$PROMPT\"}],
      \"max_tokens\": $N_GEN,
      \"temperature\": 0.5
    }" 2>&1)

  echo "$RESP" | python3 -c "
import json, sys
try:
    d = json.load(sys.stdin)
    u = d.get('usage', {})
    t = d.get('timings', {})
    pp_tps = t.get('prompt_per_second', 0)
    tg_ppm = t.get('predicted_per_token_ms', 0)
    tg_tps = round(1000/tg_ppm, 2) if tg_ppm > 0 else 0
    result = {
        'label': '$LABEL',
        'kv_type': '$KV_TYPE',
        'ctk': '$CTK',
        'ctx_size': int('$CTX_SIZE'),
        'threads': int('$THREADS'),
        'threads_batch': int('$THREADS_BATCH'),
        'rep': int('$rep'),
        'prompt_tokens': u.get('prompt_tokens', 0),
        'completion_tokens': u.get('completion_tokens', 128),
        'pp_toks_per_sec': round(pp_tps, 2),
        'tg_toks_per_sec': tg_tps,
        'pp_ms': round(t.get('prompt_ms', 0), 2),
        'tg_ms_per_token': round(t.get('predicted_per_token_ms', 0), 3),
        'n_gpu_layers': 99,
        'cached_tokens': u.get('prompt_tokens_details', {}).get('cached_tokens', 0),
        'finish_reason': d.get('choices', [{}])[0].get('finish_reason', '')
    }
    print(json.dumps(result))
except Exception as e:
    print(json.dumps({'error': str(e), 'label': '$LABEL', 'rep': int('$rep')}), file=sys.stderr)
" >> "$KBENCH_DIR/qwen38-full-matrix.jsonl"

  echo "[cynic-bench] rep=$rep done"
  sleep 3
done

# Cleanup
kill $SERVER_PID 2>/dev/null || true
sleep 2
kill -9 $SERVER_PID 2>/dev/null || true

echo "[cynic-bench] DONE $LABEL"
