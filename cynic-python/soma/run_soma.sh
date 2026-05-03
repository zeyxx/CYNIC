#!/bin/bash
# Soma Orchestrator — Quick start
# Usage: ./run_soma.sh [probe-only|monitor|help]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Check Python
if ! command -v python3 &> /dev/null; then
    echo "❌ python3 not found"
    exit 1
fi

# Check dependencies
python3 -c "import toml" 2>/dev/null || {
    echo "❌ Missing: pip install toml"
    exit 1
}

COMMAND="${1:-help}"

case "$COMMAND" in
    probe-only)
        echo "🔍 Running single health check..."
        python3 << 'EOF'
import asyncio
from soma_orchestrator import SomaOrchestrator
from pathlib import Path

async def main():
    orchestrator = SomaOrchestrator(Path("soma_manifest.toml"))
    print(f"✓ Loaded {len(orchestrator.backends)} backends")
    print("\nInitial health check:")
    await orchestrator.probe_all()
    await orchestrator.enforce_manifest()
    import json
    print(json.dumps(orchestrator.summary(), indent=2, default=str))

asyncio.run(main())
EOF
        ;;
    monitor)
        echo "📡 Starting Soma orchestrator (monitoring loop)..."
        echo "   Press Ctrl+C to stop"
        python3 -m soma.soma_orchestrator
        ;;
    help|*)
        cat << 'EOF'
Soma Layer 1 Orchestrator — Quick Start

Usage:
  ./run_soma.sh [command]

Commands:
  probe-only   Single health check, exit (test mode)
  monitor      Start monitoring loop (blocking, runs forever)
  help         Show this message

Examples:
  # Test the orchestrator (check all backends once)
  ./run_soma.sh probe-only

  # Run permanently (systemd can manage this)
  ./run_soma.sh monitor

Configuration:
  Edit soma_manifest.toml to customize backends, budgets, recovery strategies.

Logs:
  stdout: INFO/WARNING/ERROR messages
  (Structured logging ready for systemd integration)

Next:
  1. Run: ./run_soma.sh probe-only
  2. Verify all backends are alive
  3. Deploy: systemctl --user start soma-orchestrator (see README.md)
  4. Monitor for 7 days: measure OOM/timeout/context-drift
EOF
        ;;
esac
