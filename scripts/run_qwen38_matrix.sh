#!/bin/bash
# Run full qwen38-27b benchmark matrix sequentially
# Uses run_qwen38_bench.sh for each config

MATRIX_SCRIPT="/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/scripts/run_qwen38_bench.sh"

# Phase 1: KV type sweep at 64K ctx, 8 threads (conservative baseline)
CONFIGS=(
  "q4_0 65536 8 16 q4_64k_8t"
  "q8_0 65536 8 16 q8_64k_8t"
  "f16 65536 8 16 f16_64k_8t"
  "iq4_nl 65536 8 16 iq4nl_64k_8t"
  # Phase 2: thread scaling at 64K q4_0
  "q4_0 65536 4 8 q4_64k_4t"
  "q4_0 65536 6 12 q4_64k_6t"
  "q4_0 65536 10 16 q4_64k_10t"
  "q4_0 65536 12 16 q4_64k_12t"
  # Phase 3: ctx size sweep at 12 threads, q4_0
  "q4_0 131072 12 16 q4_128k_12t"
  "iq4_nl 131072 12 16 iq4nl_128k_12t"
  # Phase 4: 200K stress test
  "q4_0 200000 12 16 q4_200k_12t"
)

TOTAL=${#CONFIGS[@]}
echo "=== Running $TOTAL benchmark configs for qwen38-27b ==="
echo "Start time: $(date)"
echo ""

i=0
for cfg in "${CONFIGS[@]}"; do
  i=$((i + 1))
  echo "=== [$i/$TOTAL] Launching: $cfg ==="
  bash "$MATRIX_SCRIPT" $cfg 2>&1
  echo ""
  echo "=== [$i/$TOTAL] Complete ==="
  echo ""
done

echo "=== All benchmarks done at $(date) ==="
