#!/bin/bash
# CYNIC Askesis — Claude Code SessionEnd Hook
# Capture la mémoire cortex et l'injecte dans le Proof-of-History.

PROJECT_ROOT="$CLAUDE_PROJECT_DIR"
CORTEX_HISTORY="$PROJECT_ROOT/.cynic/memory/cortex-history.md"
ASKESIS_LOG="$PROJECT_ROOT/.cynic/memory/logs/human-kernel.jsonl"
ASKESIS_BIN="$PROJECT_ROOT/target/debug/cynic-askesis"

# 1. Vérifier le binaire
if [ ! -f "$ASKESIS_BIN" ]; then
    ASKESIS_BIN=$(which cynic-askesis 2>/dev/null)
fi

# 2. Ingestion
if [ -f "$CORTEX_HISTORY" ] && [ -x "$ASKESIS_BIN" ]; then
    "$ASKESIS_BIN" ingest "$CORTEX_HISTORY" --logfile "$ASKESIS_LOG" --domain session_end_claude > /dev/null 2>&1
    echo "[CYNIC] Askesis: Claude session history sealed." >&2
fi

exit 0
