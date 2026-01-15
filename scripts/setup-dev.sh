#!/bin/bash
# CYNIC Development Environment Setup
# Generates local configuration files from templates

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "🐕 CYNIC Development Setup"
echo "================================"

# Check for .env
if [ ! -f "$PROJECT_ROOT/.env" ]; then
  echo "Creating .env from template..."
  cp "$PROJECT_ROOT/.env.example" "$PROJECT_ROOT/.env"
  echo "✓ Created .env - please configure with your values"
else
  echo "✓ .env exists"
fi

# Generate .mcp.json for Claude Code
if [ ! -f "$PROJECT_ROOT/.mcp.json" ]; then
  echo "Creating .mcp.json..."
  cat > "$PROJECT_ROOT/.mcp.json" << EOF
{
  "mcpServers": {
    "cynic": {
      "command": "node",
      "args": ["packages/mcp/bin/mcp.js"],
      "cwd": "$PROJECT_ROOT",
      "env": {
        "MCP_MODE": "stdio",
        "NODE_ENV": "development"
      }
    }
  }
}
EOF
  echo "✓ Created .mcp.json"
else
  echo "✓ .mcp.json exists"
fi

echo ""
echo "================================"
echo "For local development with Docker:"
echo ""
echo "  # PostgreSQL"
echo "  docker run -d --name cynic-postgres \\"
echo "    -p 5432:5432 \\"
echo "    -e POSTGRES_USER=cynic \\"
echo "    -e POSTGRES_PASSWORD=cynic \\"
echo "    -e POSTGRES_DB=cynic \\"
echo "    postgres:16"
echo ""
echo "  # Redis"
echo "  docker run -d --name cynic-redis \\"
echo "    -p 6379:6379 \\"
echo "    redis:7"
echo ""
echo "================================"
echo "Setup complete! Edit .env with your configuration."
echo ""
echo "For production credentials:"
echo "  - Get CYNIC_DATABASE_URL from Render PostgreSQL dashboard"
echo "  - Get CYNIC_REDIS_URL from Render Key-Value dashboard"
echo "  - NEVER commit real credentials to git"
