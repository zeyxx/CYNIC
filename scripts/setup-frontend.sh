#!/bin/bash
# Setup script for S.'s Windows machine (run in Git Bash or WSL)
# Installs: Node.js check, Claude Code check, project clone, frontend scaffold

set -e

echo "=== CYNIC Hackathon Setup — S. ==="

# Check Node.js
if command -v node &>/dev/null; then
    echo "[OK] Node.js $(node --version)"
else
    echo "[!!] Node.js not found. Install from https://nodejs.org/"
    exit 1
fi

# Check npm
if command -v npm &>/dev/null; then
    echo "[OK] npm $(npm --version)"
else
    echo "[!!] npm not found"
    exit 1
fi

# Check Claude Code
if command -v claude &>/dev/null; then
    echo "[OK] Claude Code $(claude --version 2>&1)"
else
    echo "[!!] Claude Code not found. Install: npm install -g @anthropic-ai/claude-code"
fi

# Check Tailscale
if command -v tailscale &>/dev/null; then
    echo "[OK] Tailscale installed"
    tailscale status 2>/dev/null | head -3 || echo "[WARN] Tailscale not connected"
else
    echo "[!!] Tailscale not found. Install from https://tailscale.com/download"
fi

# Check API access via Tailscale
echo ""
echo "=== Testing CYNIC API via Tailscale ==="
if curl -s --connect-timeout 5 http://<TAILSCALE_UBUNTU>:3030/health > /dev/null 2>&1; then
    echo "[OK] CYNIC kernel reachable at <TAILSCALE_UBUNTU>:3030"
    curl -s http://<TAILSCALE_UBUNTU>:3030/health | python3 -m json.tool 2>/dev/null || curl -s http://<TAILSCALE_UBUNTU>:3030/health
else
    echo "[!!] Cannot reach CYNIC kernel at <TAILSCALE_UBUNTU>:3030"
    echo "     Make sure T.'s machine is running and kernel is started"
fi

echo ""
echo "=== Setup Summary ==="
echo "1. Clone/pull CYNIC repo"
echo "2. cd cynic-ui && npm install"
echo "3. npm run dev (Vite dev server)"
echo "4. API target: http://<TAILSCALE_UBUNTU>:3030 (T. via Tailscale)"
echo "5. Slash commands available: /build, /run, /e2e, /status, /frontend-dev"
echo ""
echo "Ready for hackathon!"
