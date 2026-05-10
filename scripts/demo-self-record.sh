#!/usr/bin/env bash
# CYNIC Self-Recording Demo — launches ffmpeg + demo script in one command
# Output: ~/.cynic/recordings/YYYY-MM-DD_HHhMM_vX.Y.Z.mp4
# Usage: ./scripts/demo-self-record.sh [DEMO_SPEED]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DEMO_SCRIPT="${SCRIPT_DIR}/demo-organic.sh"
SPEED="${1:-1}"

# ── Versioning + horodatage ────────────────────────────────────
TIMESTAMP="$(date +%Y-%m-%d_%Hh%M)"
source "${HOME}/.cynic-env" 2>/dev/null || true
VERSION=$(curl -s -m 3 "${CYNIC_REST_ADDR:-localhost:3030}/health" \
    -H "Authorization: Bearer ${CYNIC_API_KEY:-}" 2>/dev/null \
    | python3 -c "import sys,json; print(json.load(sys.stdin).get('version','unknown'))" 2>/dev/null \
    || echo "unknown")

RECORDINGS_DIR="${HOME}/.cynic/recordings"
mkdir -p "${RECORDINGS_DIR}"
OUTPUT="${RECORDINGS_DIR}/${TIMESTAMP}_${VERSION}.mp4"

# ── Detect primary monitor ─────────────────────────────────────
PRIMARY=$(xrandr 2>/dev/null | grep " primary" | grep -oP '\d+x\d+\+\d+\+\d+' | head -1)
if [ -n "${PRIMARY}" ]; then
    SIZE=$(echo "${PRIMARY}" | grep -oP '^\d+x\d+')
    OFFSET_X=$(echo "${PRIMARY}" | grep -oP '\+\K\d+' | head -1)
    OFFSET_Y=$(echo "${PRIMARY}" | grep -oP '\+\K\d+' | tail -1)
else
    SIZE="1920x1080"
    OFFSET_X="0"
    OFFSET_Y="0"
fi

echo "╔══════════════════════════════════════════════╗"
echo "║  CYNIC Self-Recording Demo                   ║"
echo "║                                              ║"
echo "║  Version:  ${VERSION}"
echo "║  Output:   ${OUTPUT}"
echo "║  Screen:   ${SIZE}+${OFFSET_X},${OFFSET_Y}"
echo "║  Speed:    ${SPEED}x"
echo "║                                              ║"
echo "║  Recording starts in 3s...                   ║"
echo "╚══════════════════════════════════════════════╝"
sleep 3

# ── Start ffmpeg in background ─────────────────────────────────
ffmpeg -y \
    -f x11grab -video_size "${SIZE}" -framerate 30 -i ":0+${OFFSET_X},${OFFSET_Y}" \
    -f pulse -i default \
    -c:v libx264 -preset fast -crf 18 -pix_fmt yuv420p \
    -c:a aac -b:a 128k \
    -movflags +faststart \
    "${OUTPUT}" &
FFMPEG_PID=$!

# Wait for ffmpeg to initialize
sleep 2

# ── Run the demo ───────────────────────────────────────────────
DEMO_SPEED="${SPEED}" bash "${DEMO_SCRIPT}" || true

# ── Stop recording ─────────────────────────────────────────────
sleep 2
kill -SIGINT "${FFMPEG_PID}" 2>/dev/null || true
wait "${FFMPEG_PID}" 2>/dev/null || true

# ── Summary ────────────────────────────────────────────────────
DURATION=$(ffprobe -v error -show_entries format=duration \
    -of default=noprint_wrappers=1:nokey=1 "${OUTPUT}" 2>/dev/null || echo "?")
FILESIZE=$(du -h "${OUTPUT}" 2>/dev/null | cut -f1)

echo ""
echo "╔══════════════════════════════════════════════╗"
echo "║  Recording complete                          ║"
echo "║                                              ║"
echo "║  File:     ${OUTPUT}"
echo "║  Duration: ${DURATION}s"
echo "║  Size:     ${FILESIZE}"
echo "║  Version:  ${VERSION}"
echo "║                                              ║"
echo "║  Recordings index:                           ║"
ls -1t "${RECORDINGS_DIR}"/*.mp4 2>/dev/null | head -5 | while read f; do
    echo "║    $(basename "$f")"
done
echo "╚══════════════════════════════════════════════╝"
