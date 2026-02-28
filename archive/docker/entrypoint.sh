#!/bin/bash

# =============================================================================
# CYNIC Governance Bot - Docker Entrypoint
# =============================================================================
# Handles initialization and graceful startup/shutdown

set -e

echo "=== CYNIC Governance Bot ==="
echo "Environment: $BOT_ENVIRONMENT"
echo ""

# Validate required environment variables
if [ -z "$DISCORD_TOKEN" ]; then
    echo "ERROR: DISCORD_TOKEN environment variable is not set"
    echo "Set it with: docker run -e DISCORD_TOKEN=your_token ..."
    exit 1
fi

echo "✓ Discord token configured"

# Validate database if using file-based
if [[ "$DATABASE_URL" == sqlite* ]]; then
    DB_FILE=$(echo $DATABASE_URL | sed 's/sqlite:\/\/\///g')
    DB_DIR=$(dirname "$DB_FILE")

    if [ ! -d "$DB_DIR" ]; then
        echo "Creating database directory: $DB_DIR"
        mkdir -p "$DB_DIR"
    fi

    echo "✓ Database directory ready: $DB_DIR"
fi

# Create backups directory if needed
if [ ! -d "/app/backups" ]; then
    mkdir -p /app/backups
    echo "✓ Backups directory created"
fi

# Log directory
if [ ! -d "/app/logs" ]; then
    mkdir -p /app/logs
    echo "✓ Logs directory created"
fi

echo ""
echo "Starting governance bot..."
echo ""

# Handle graceful shutdown
trap 'echo "Received SIGTERM, shutting down gracefully..."; exit 0' SIGTERM SIGINT

# Execute the command
exec "$@"
