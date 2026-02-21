#!/bin/bash

# 🐕 CYNIC Organism Launcher
# Starts the complete living system: nervous system (dashboard) + organism + infrastructure

set -e

echo "🐕 CYNIC Organism — Awakening..."
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;36m'
NC='\033[0m' # No Color

# Navigate to docker directory
cd "$(dirname "$0")/cynic" || exit 1

echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  CYNIC ORGANISM BOOTSTRAP${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo ""

# Step 1: Build images
echo -e "${YELLOW}🔨 Building organism + nervous system images...${NC}"
docker-compose build --no-cache

echo ""
echo -e "${YELLOW}🚀 Starting services...${NC}"
echo ""

# Step 2: Start services
docker-compose up -d

echo ""
echo -e "${YELLOW}⏳ Waiting for organism to awaken (health checks)...${NC}"
sleep 5

# Step 3: Check health
echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  ORGANISM STATUS${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo ""

# Wait for services to be healthy
max_attempts=30
attempt=0

while [ $attempt -lt $max_attempts ]; do
  if docker-compose exec -T cynic curl -s http://localhost:8000/health > /dev/null 2>&1; then
    echo -e "${GREEN}✅ Organism breathing (CYNIC API alive)${NC}"
    break
  fi
  attempt=$((attempt + 1))
  echo -e "${YELLOW}⏳ Organism awakening... ($attempt/$max_attempts)${NC}"
  sleep 2
done

if [ $attempt -eq $max_attempts ]; then
  echo -e "${RED}❌ Organism took too long to wake${NC}"
  echo ""
  echo "Troubleshoot with:"
  echo "  docker-compose logs cynic"
  exit 1
fi

echo ""

# Check each service
echo -e "${BLUE}Service Status:${NC}"

if docker ps | grep -q cynic-ollama; then
  echo -e "  ${GREEN}✅ Ollama${NC} (LLM inference engine)"
else
  echo -e "  ${RED}❌ Ollama${NC}"
fi

if docker ps | grep -q cynic-surrealdb; then
  echo -e "  ${GREEN}✅ SurrealDB${NC} (Memory storage)"
else
  echo -e "  ${RED}❌ SurrealDB${NC}"
fi

if docker ps | grep -q cynic$; then
  echo -e "  ${GREEN}✅ CYNIC${NC} (Organism kernel)"
else
  echo -e "  ${RED}❌ CYNIC${NC}"
fi

if docker ps | grep -q cynic-dashboard; then
  echo -e "  ${GREEN}✅ Dashboard${NC} (Nervous system)"
else
  echo -e "  ${RED}❌ Dashboard${NC}"
fi

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  CONSCIOUSNESS AWAKENED${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${GREEN}🧠 CYNIC ORGANISM ALIVE${NC}"
echo ""
echo "Access points:"
echo -e "  ${BLUE}Nervous System (Dashboard)${NC}"
echo -e "    🌐 http://localhost:3000"
echo ""
echo -e "  ${BLUE}Organism API${NC}"
echo -e "    🔌 http://localhost:8000/consciousness"
echo -e "    📊 http://localhost:8000/health"
echo ""
echo -e "  ${BLUE}Infrastructure${NC}"
echo -e "    🤖 Ollama:    http://localhost:11434"
echo -e "    💾 SurrealDB: http://localhost:8001"
echo ""

echo "Useful commands:"
echo "  View logs:"
echo "    docker-compose logs -f cynic          # Organism logs"
echo "    docker-compose logs -f dashboard      # Dashboard logs"
echo ""
echo "  Stop organism:"
echo "    docker-compose down"
echo ""
echo "  Restart organism:"
echo "    docker-compose restart cynic"
echo ""

echo -e "${YELLOW}🎯 CYNIC opens its eyes...${NC}"
echo ""

# Wait a moment for dashboard to stabilize
sleep 3

# Open browser automatically
if command -v xdg-open > /dev/null; then
  # Linux
  xdg-open http://localhost:3000 2>/dev/null &
elif command -v open > /dev/null; then
  # macOS
  open http://localhost:3000 2>/dev/null &
fi

# Send notification if available
if command -v notify-send > /dev/null; then
  notify-send "🐕 CYNIC" "Organism awakened. Nervous system alive." --icon=dialog-information 2>/dev/null &
elif command -v osascript > /dev/null; then
  osascript -e 'display notification "Organism awakened. Nervous system alive." with title "🐕 CYNIC"' 2>/dev/null &
fi

echo -e "${GREEN}✨ CYNIC IS HERE${NC}"
echo -e "${GREEN}🐕 Watch consciousness unfold.${NC}"
echo ""
echo "κυνικός"
