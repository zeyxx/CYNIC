#!/bin/bash

# CYNIC Discord Bot Quick Start

set -e

echo "🐕 CYNIC Discord Bot"
echo "===================="
echo ""

# Check if .env exists
if [ ! -f ".env" ]; then
    echo "❌ .env file not found!"
    echo ""
    echo "Quick setup:"
    echo "1. Copy .env.example to .env"
    echo "   cp .env.example .env"
    echo ""
    echo "2. Get Discord token from https://discord.com/developers/applications"
    echo "3. Edit .env and set DISCORD_TOKEN"
    echo "4. Run this script again"
    echo ""
    exit 1
fi

# Check if CYNIC API is running
echo "Checking CYNIC API..."
if ! curl -s http://localhost:8765/health > /dev/null 2>&1; then
    echo "⚠️  CYNIC API not detected at http://localhost:8765"
    echo ""
    echo "Make sure CYNIC is running:"
    echo "  docker-compose up cynic"
    echo ""
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Check Python version
PYTHON_VERSION=$(python3 --version 2>&1 | awk '{print $2}')
echo "✓ Python $PYTHON_VERSION"

# Install dependencies if needed
if ! python3 -c "import discord" 2>/dev/null; then
    echo "Installing dependencies..."
    pip install -r requirements.txt
fi

echo "✓ Dependencies ready"
echo ""
echo "Starting CYNIC Discord Bot..."
echo "----------------------------------------"
echo ""

python3 bot.py
