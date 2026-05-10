#!/usr/bin/env bash
# CYNIC Demo Recorder — tmux split-screen + ffmpeg capture
# Left pane:  demo-organic.sh (narrated organism demo)
# Right pane: journalctl -f (live kernel logs)
#
# Usage: ./scripts/demo-record.sh
# Stop:  press 'q' in the left pane or Ctrl-C, then type 'exit'
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DEMO_SCRIPT="${SCRIPT_DIR}/demo-organic.sh"
OUTPUT="${SCRIPT_DIR}/../cynic-demo.mp4"
SESSION="cynic-demo"

# Kill previous session if exists
tmux kill-session -t "${SESSION}" 2>/dev/null || true

# ── Create tmux session ────────────────────────────────────────
# Left pane (65%): demo script
tmux new-session -d -s "${SESSION}" -x 220 -y 55
tmux set -t "${SESSION}" status off  # clean look

# Right pane (35%): kernel logs
tmux split-window -t "${SESSION}" -h -p 35

# Right pane: live kernel logs with color
tmux send-keys -t "${SESSION}:0.1" \
    'echo -e "\033[38;5;245m━━━ KERNEL LIVE ━━━\033[0m" && journalctl --user -u cynic-kernel.service -f --no-pager -n 0 | grep --line-buffered -v "^$"' Enter

# Give journalctl a moment to start
sleep 1

# Left pane: run the demo
tmux send-keys -t "${SESSION}:0.0" \
    "bash ${DEMO_SCRIPT}" Enter

# ── Start recording ────────────────────────────────────────────
echo "╔══════════════════════════════════════════╗"
echo "║  CYNIC Demo Recording                   ║"
echo "║                                         ║"
echo "║  tmux session: ${SESSION}              ║"
echo "║  Output: ${OUTPUT}                      ║"
echo "║                                         ║"
echo "║  To attach:  tmux attach -t ${SESSION}  ║"
echo "║  To stop:    Ctrl-C here                ║"
echo "╚══════════════════════════════════════════╝"
echo ""
echo "Starting ffmpeg screen capture in 3s..."
sleep 3

# Capture primary monitor
ffmpeg -y \
    -f x11grab -video_size 1920x1080 -framerate 30 -i :0+1920,0 \
    -f pulse -i default \
    -c:v libx264 -preset fast -crf 18 -pix_fmt yuv420p \
    -c:a aac -b:a 128k \
    -movflags +faststart \
    "${OUTPUT}"

# Cleanup
tmux kill-session -t "${SESSION}" 2>/dev/null || true
echo ""
echo "Recording saved: ${OUTPUT}"
echo "Duration: $(ffprobe -v error -show_entries format=duration -of csv=p=0 "${OUTPUT}" 2>/dev/null || echo '?')s"
