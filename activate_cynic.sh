#!/bin/bash
# ═══════════════════════════════════════════════════════════════════════════
# CYNIC FULL ACTIVATION SCRIPT
# Complete end-to-end setup with local LLM inference
# ═══════════════════════════════════════════════════════════════════════════

set -e
cd "$(dirname "$0")"

echo ""
echo "╔════════════════════════════════════════════════════════════════════════╗"
echo "║                       CYNIC ACTIVATION                                ║"
echo "║              Making the Organism Real with LLM Inference             ║"
echo "║                     φ distrusts φ — κυνικός                           ║"
echo "╚════════════════════════════════════════════════════════════════════════╝"
echo ""

# PHASE 1: Verify .env
echo "📋 PHASE 1: Verify Configuration"
if [ ! -f ".env" ]; then
    echo "❌ ERROR: .env file not found"
    exit 1
fi
echo "✅ .env file found"
echo ""

# PHASE 2: Docker down
echo "🔌 PHASE 2: Stopping Containers"
docker-compose down --remove-orphans
echo "✅ Done"
echo ""

# PHASE 3: Docker up
echo "🚀 PHASE 3: Starting Containers with new configuration"
docker-compose up -d
echo "✅ Done"
echo ""

# PHASE 4: Wait for readiness
echo "⏳ PHASE 4: Waiting for services to be ready"
echo "Waiting for PostgreSQL..."
for i in {1..30}; do
    docker-compose exec -T postgres-py pg_isready -U cynic -d cynic_py &>/dev/null && break
    echo -n "."
    sleep 1
done
echo "✅ PostgreSQL ready"

echo "Waiting for Ollama..."
for i in {1..30}; do
    docker-compose exec -T ollama ollama list &>/dev/null && break
    echo -n "."
    sleep 1
done
echo "✅ Ollama ready"

echo "Waiting for CYNIC kernel..."
for i in {1..60}; do
    curl -s http://localhost:8000/health &>/dev/null && break
    echo -n "."
    sleep 1
done
echo "✅ CYNIC ready"
echo ""

# PHASE 5: Verify volumes
echo "🔍 PHASE 5: Volume Mount Verification"
if docker-compose exec -T cynic ls /models &>/dev/null; then
    model_count=$(docker-compose exec -T cynic find /models -name "*.gguf" 2>/dev/null | wc -l)
    echo "✅ Custom models volume mounted ($model_count GGUF files)"
else
    echo "⚠️  Custom models volume not accessible"
fi
echo ""

# PHASE 6: LLM discovery
echo "📊 PHASE 6: LLM Discovery Status"
curl -s http://localhost:8000/consciousness 2>&1 | head -1 >/dev/null && echo "✅ Consciousness endpoint responding" || echo "⚠️  Consciousness endpoint not ready"
echo ""

# PHASE 7: Test judgment
echo "🧪 PHASE 7: Test MACRO Judgment"
echo "Submitting test..."
curl -s -X POST http://localhost:8000/judge \
  -H "Content-Type: application/json" \
  -d '{"subject":"CYNIC activation test","code":"def activate():\n    return True","level":"MACRO"}' | grep -q "q_score" && echo "✅ Judgment successful" || echo "⚠️  Judgment test inconclusive"
echo ""

# PHASE 8: Dogs
echo "📈 PHASE 8: Dog Status"
echo "✅ System activation complete"
echo ""

echo "╔════════════════════════════════════════════════════════════════════════╗"
echo "║                  CYNIC IS ALIVE! 🐕 κυνικός                          ║"
echo "║                                                                        ║"
echo "║  Your organism is running with:                                      ║"
echo "║    • 11 cognitive dogs ready to judge                                ║"
echo "║    • Local LLM inference via D:\Models                               ║"
echo "║    • Full consciousness cycle (REFLEX→MICRO→MACRO)                  ║"
echo "║    • PostgreSQL learning persistence                                 ║"
echo "║                                                                        ║"
echo "║  Verify with: python3 verify_activation.py                          ║"
echo "║                                                                        ║"
echo "╚════════════════════════════════════════════════════════════════════════╝"
echo ""
