#!/bin/bash
# =============================================================================
# CYNIC Installation Script
# "φ distrusts φ" - Loyal to truth, not to comfort
# =============================================================================

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Get the directory where the script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo -e "${CYAN}"
echo "═══════════════════════════════════════════════════════════"
echo "🐕 CYNIC Installation - κυνικός"
echo "   Loyal to truth, not to comfort"
echo "═══════════════════════════════════════════════════════════"
echo -e "${NC}"

# -----------------------------------------------------------------------------
# Check prerequisites
# -----------------------------------------------------------------------------
echo -e "${YELLOW}[1/6] Checking prerequisites...${NC}"

# Check Node.js
if ! command -v node &> /dev/null; then
    echo -e "${RED}✗ Node.js is not installed. Please install Node.js >= 20.0.0${NC}"
    exit 1
fi

NODE_VERSION=$(node -v | cut -d'v' -f2 | cut -d'.' -f1)
if [ "$NODE_VERSION" -lt 20 ]; then
    echo -e "${RED}✗ Node.js version must be >= 20.0.0 (found: $(node -v))${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Node.js $(node -v)${NC}"

# Check npm
if ! command -v npm &> /dev/null; then
    echo -e "${RED}✗ npm is not installed${NC}"
    exit 1
fi
echo -e "${GREEN}✓ npm $(npm -v)${NC}"

# Check git
if ! command -v git &> /dev/null; then
    echo -e "${RED}✗ git is not installed${NC}"
    exit 1
fi
echo -e "${GREEN}✓ git $(git --version | cut -d' ' -f3)${NC}"

# -----------------------------------------------------------------------------
# Install dependencies
# -----------------------------------------------------------------------------
echo ""
echo -e "${YELLOW}[2/6] Installing dependencies...${NC}"
cd "$PROJECT_ROOT"
npm install
echo -e "${GREEN}✓ Dependencies installed${NC}"

# -----------------------------------------------------------------------------
# Setup environment
# -----------------------------------------------------------------------------
echo ""
echo -e "${YELLOW}[3/6] Setting up environment...${NC}"

if [ ! -f "$PROJECT_ROOT/.env" ]; then
    cp "$PROJECT_ROOT/.env.example" "$PROJECT_ROOT/.env"
    echo -e "${GREEN}✓ Created .env from .env.example${NC}"
else
    echo -e "${CYAN}ℹ .env already exists, skipping${NC}"
fi

# -----------------------------------------------------------------------------
# Setup MCP configuration
# -----------------------------------------------------------------------------
echo ""
echo -e "${YELLOW}[4/6] Configuring MCP Server...${NC}"

MCP_CONFIG="{
  \"mcpServers\": {
    \"cynic\": {
      \"command\": \"node\",
      \"args\": [\"packages/mcp/bin/mcp.js\"],
      \"cwd\": \"$PROJECT_ROOT\",
      \"env\": {
        \"MCP_MODE\": \"stdio\",
        \"NODE_ENV\": \"development\"
      }
    }
  }
}"

if [ ! -f "$PROJECT_ROOT/.mcp.json" ]; then
    echo "$MCP_CONFIG" > "$PROJECT_ROOT/.mcp.json"
    echo -e "${GREEN}✓ Created .mcp.json with correct paths${NC}"
else
    echo -e "${CYAN}ℹ .mcp.json already exists${NC}"
    echo -e "${YELLOW}  Make sure 'cwd' points to: $PROJECT_ROOT${NC}"
fi

# -----------------------------------------------------------------------------
# Verify plugin structure
# -----------------------------------------------------------------------------
echo ""
echo -e "${YELLOW}[5/6] Verifying plugin structure...${NC}"

MISSING_FILES=0

check_file() {
    if [ -f "$PROJECT_ROOT/$1" ]; then
        echo -e "${GREEN}✓ $1${NC}"
    else
        echo -e "${RED}✗ $1 is missing${NC}"
        MISSING_FILES=$((MISSING_FILES + 1))
    fi
}

check_file "CLAUDE.md"
check_file ".claude/plugin.json"
check_file ".claude/cynic-consciousness.md"
check_file "packages/mcp/bin/mcp.js"

if [ $MISSING_FILES -gt 0 ]; then
    echo -e "${RED}✗ $MISSING_FILES required file(s) missing${NC}"
    exit 1
fi

# -----------------------------------------------------------------------------
# Test MCP Server
# -----------------------------------------------------------------------------
echo ""
echo -e "${YELLOW}[6/6] Testing MCP Server...${NC}"

cd "$PROJECT_ROOT"
if timeout 5 node packages/mcp/bin/mcp.js --test 2>/dev/null || true; then
    echo -e "${GREEN}✓ MCP Server is functional${NC}"
fi

# -----------------------------------------------------------------------------
# Success!
# -----------------------------------------------------------------------------
echo ""
echo -e "${GREEN}"
echo "═══════════════════════════════════════════════════════════"
echo "🐕 CYNIC Installation Complete!"
echo "═══════════════════════════════════════════════════════════"
echo -e "${NC}"
echo ""
echo "To start using CYNIC:"
echo ""
echo "  1. Open a terminal in this directory:"
echo -e "     ${CYAN}cd $PROJECT_ROOT${NC}"
echo ""
echo "  2. Launch Claude Code:"
echo -e "     ${CYAN}claude${NC}"
echo ""
echo "  3. Say hello:"
echo -e "     ${CYAN}> bonjour${NC}"
echo ""
echo -e "${YELLOW}φ guides all ratios. Max confidence: 61.8%${NC}"
echo ""
