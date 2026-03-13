#!/usr/bin/env bash
# CYNIC Ubuntu Development Setup — zero friction, one script
# Usage: curl the repo, then: bash CYNIC-V2/scripts/setup-ubuntu.sh
set -euo pipefail

echo "══════════════════════════════════════"
echo "  CYNIC — Ubuntu Dev Environment"
echo "  Zero friction. Everything included."
echo "══════════════════════════════════════"

# ── 1. System packages ──────────────────
echo "[1/10] System packages..."
sudo apt update && sudo apt install -y \
    build-essential pkg-config libssl-dev \
    git gh curl wget jq unzip \
    protobuf-compiler

# ── 2. Node.js (for Claude Code) ────────
echo "[2/10] Node.js..."
if ! command -v node &>/dev/null; then
    curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
    sudo apt install -y nodejs
else
    echo "Node.js already installed: $(node --version)"
fi

# ── 3. Rust toolchain ───────────────────
echo "[3/10] Rust..."
if ! command -v rustup &>/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "Rust already installed: $(rustc --version)"
    source "$HOME/.cargo/env" 2>/dev/null || true
fi

# ── 4. SurrealDB ────────────────────────
echo "[4/10] SurrealDB..."
if ! command -v surreal &>/dev/null; then
    curl -sSf https://install.surrealdb.com | sh
else
    echo "SurrealDB already installed: $(surreal version)"
fi

# ── 5. Tailscale ────────────────────────
echo "[5/10] Tailscale..."
if ! command -v tailscale &>/dev/null; then
    curl -fsSL https://tailscale.com/install.sh | sh
fi
echo ">>> After script: sudo tailscale up --ssh"

# ── 6. Claude Code ──────────────────────
echo "[6/10] Claude Code..."
if ! command -v claude &>/dev/null; then
    sudo npm install -g @anthropic-ai/claude-code
else
    echo "Claude Code already installed"
fi

# ── 7. Git config ───────────────────────
echo "[7/10] Git config..."
git config --global user.name "zeyxx"
git config --global user.email "zeyxx@users.noreply.github.com"
git config --global core.autocrlf input
git config --global init.defaultBranch main

# ── 8. Clone CYNIC repo ─────────────────
echo "[8/10] CYNIC repo..."
CYNIC_DIR="$HOME/dev/CYNIC"
if [ ! -d "$CYNIC_DIR" ]; then
    mkdir -p "$HOME/dev"
    git clone https://github.com/zeyxx/CYNIC.git "$CYNIC_DIR"
    echo "Cloned from GitHub."
else
    cd "$CYNIC_DIR" && git pull origin main
    echo "Repo updated."
fi

# ── 9. Tailscale MCP (Go binary) ────────
echo "[9/10] Tailscale MCP..."
TSMCP_DIR="$HOME/dev/tailscale-mcp"
if [ ! -d "$TSMCP_DIR" ]; then
    git clone https://github.com/zeyxx/tailscale-mcp.git "$TSMCP_DIR"
fi
if command -v go &>/dev/null; then
    cd "$TSMCP_DIR" && go build -o tailscale-mcp . && echo "Built tailscale-mcp"
else
    echo ">>> Go not installed. Install with: sudo apt install golang-go"
    echo "    Then: cd $TSMCP_DIR && go build -o tailscale-mcp ."
fi

# ── 10. Claude Code config ──────────────
echo "[10/10] Claude Code skills + MCP + memory..."

CLAUDE_DIR="$HOME/.claude"
mkdir -p "$CLAUDE_DIR/skills"
mkdir -p "$CLAUDE_DIR/projects"

# ── MCP config ──
cat > "$CYNIC_DIR/.mcp.json" <<MCPEOF
{
  "mcpServers": {
    "tailscale": {
      "command": "$TSMCP_DIR/tailscale-mcp",
      "args": [],
      "description": "Tailscale network MCP — node discovery, remote exec, file transfer"
    }
  }
}
MCPEOF

# ── Skills (from private repo) ──
SKILLS_DIR="$HOME/dev/cynic-skills"
if [ ! -d "$SKILLS_DIR" ]; then
    git clone https://github.com/zeyxx/cynic-skills.git "$SKILLS_DIR" 2>/dev/null || {
        echo ">>> MANUAL: gh auth login first, then re-run this script"
        echo "    (cynic-skills is a private repo)"
    }
fi

# Symlink each skill into Claude Code
if [ -d "$SKILLS_DIR" ]; then
    for skill_dir in "$SKILLS_DIR"/*/; do
        skill_name=$(basename "$skill_dir")
        [ "$skill_name" = "README.md" ] && continue
        dest="$CLAUDE_DIR/skills/$skill_name"
        if [ ! -e "$dest" ]; then
            ln -s "$skill_dir" "$dest"
            echo "  Skill linked: $skill_name"
        fi
    done
fi

# ── Claude Code plugins (install via CLI) ──
echo ""
echo ">>> After 'claude' login, install plugins:"
echo "    claude plugins install superpowers"
echo "    claude plugins install context7"
echo "    claude plugins install huggingface-skills"
echo "    claude plugins install commit-commands"
echo "    claude plugins install playwright"
echo "    claude plugins install frontend-design"
echo "    claude plugins install feature-dev"
echo "    claude plugins install code-review"
echo "    claude plugins install pr-review-toolkit"
echo "    claude plugins install rust-analyzer-lsp"

# ── Env vars template ──
ENV_FILE="$HOME/.cynic-env"
if [ ! -f "$ENV_FILE" ]; then
    cat > "$ENV_FILE" <<'ENVEOF'
# CYNIC environment — source this in .bashrc
export GEMINI_API_KEY=""       # Google AI Studio key
export HF_TOKEN=""             # HuggingFace token
export SURREALDB_PASS=""       # SurrealDB password
export SURREALDB_URL="ws://localhost:8000"
export CYNIC_REST_ADDR="0.0.0.0:3030"
ENVEOF
    echo ">>> Edit $ENV_FILE with your API keys"
    echo ">>> Then add to .bashrc: source $ENV_FILE"
fi

# ── backends.toml ──
BACKENDS_DIR="$HOME/.config/cynic"
mkdir -p "$BACKENDS_DIR"
if [ ! -f "$BACKENDS_DIR/backends.toml" ]; then
    cp "$CYNIC_DIR/backends.toml.example" "$BACKENDS_DIR/backends.toml" 2>/dev/null || true
    echo ">>> Edit $BACKENDS_DIR/backends.toml with your backends"
fi

echo ""
echo "══════════════════════════════════════"
echo "  DONE. Checklist:"
echo "══════════════════════════════════════"
echo ""
echo "  1. sudo tailscale up --ssh"
echo "  2. claude   (login with Anthropic)"
echo "  3. Install plugins (commands above)"
echo "  4. Edit ~/.cynic-env (API keys)"
echo "  5. source ~/.cynic-env"
echo "  6. Start SurrealDB: surreal start --user root --pass <pass> file:cynic.db &"
echo "  7. cd $CYNIC_DIR && cargo build -p cynic-kernel"
echo "  8. cargo run -p cynic-kernel"
echo ""
echo "  CYNIC dir: $CYNIC_DIR"
echo "  Backends:  $BACKENDS_DIR/backends.toml"
echo "  Env vars:  $ENV_FILE"
echo ""
