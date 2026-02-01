#!/usr/bin/env node
/**
 * CYNIC Example: MCP Server
 *
 * Demonstrates how to start and configure the CYNIC MCP Server.
 *
 * The MCP (Model Context Protocol) server exposes CYNIC's brain as tools
 * that can be used by AI assistants like Claude Code.
 *
 * Available Tools (60+):
 * - brain_cynic_judge: Evaluate items across 25 dimensions
 * - brain_cynic_refine: Improve judgments through RLHF
 * - brain_search: Semantic memory search
 * - brain_patterns: Detect emergent patterns
 * - brain_health: System health check
 * - ... and many more
 *
 * Run: node packages/node/examples/mcp-server.js
 *
 * Or use the CLI:
 *   npx cynic-mcp                    # stdio mode (for Claude Code)
 *   MCP_MODE=http npx cynic-mcp      # HTTP mode (for web dashboard)
 *
 * "The pack awakens as one" - κυνικός
 */

'use strict';

// =============================================================================
// EXAMPLE 1: Environment Configuration
// =============================================================================

console.log('═══════════════════════════════════════════════════════════════════');
console.log('🧠 CYNIC MCP Server Configuration');
console.log('═══════════════════════════════════════════════════════════════════\n');

console.log('── ENVIRONMENT VARIABLES ──────────────────────────────────────────');
console.log('');
console.log('Required:');
console.log('   MCP_MODE              stdio | http (default: stdio)');
console.log('');
console.log('Optional - Database:');
console.log('   CYNIC_DATABASE_URL    PostgreSQL connection string');
console.log('   CYNIC_REDIS_URL       Redis connection string');
console.log('');
console.log('Optional - AI:');
console.log('   ANTHROPIC_API_KEY     For Claude integration');
console.log('   OPENAI_API_KEY        For embeddings');
console.log('');
console.log('Optional - Services:');
console.log('   HELIUS_API_KEY        For Solana RPC');
console.log('   RENDER_API_KEY        For deployment');
console.log('');
console.log('Optional - Debug:');
console.log('   CYNIC_VERBOSE         Enable verbose logging');
console.log('   CYNIC_DEBUG           Enable debug mode');
console.log('');

// =============================================================================
// EXAMPLE 2: Starting the Server Programmatically
// =============================================================================

console.log('── PROGRAMMATIC STARTUP ───────────────────────────────────────────');
console.log('');

console.log(`
// Start MCP server programmatically:

import { bootMCP } from '@cynic/core/boot';

// Boot with options
const server = await bootMCP({
  mode: 'http',           // 'stdio' or 'http'
  port: 3000,             // HTTP port
  enableDashboard: true,  // Enable web dashboard
  persistence: {
    postgres: process.env.CYNIC_DATABASE_URL,
    redis: process.env.CYNIC_REDIS_URL,
  },
});

console.log('MCP server running on port', server.port);
`);

// =============================================================================
// EXAMPLE 3: Tool Categories
// =============================================================================

console.log('── AVAILABLE TOOL CATEGORIES ──────────────────────────────────────');
console.log('');

const categories = [
  { name: 'JUDGMENT',      tools: ['brain_cynic_judge', 'brain_cynic_refine', 'brain_cynic_feedback'] },
  { name: 'MEMORY',        tools: ['brain_memory_search', 'brain_memory_store', 'brain_memory_stats'] },
  { name: 'PATTERNS',      tools: ['brain_patterns', 'brain_timeline', 'brain_get_observations'] },
  { name: 'CODEBASE',      tools: ['brain_codebase', 'brain_lsp_symbols', 'brain_lsp_references'] },
  { name: 'ECOSYSTEM',     tools: ['brain_ecosystem', 'brain_ecosystem_monitor', 'brain_integrator'] },
  { name: 'PSYCHOLOGY',    tools: ['brain_psychology', 'brain_goals', 'brain_notifications'] },
  { name: 'BLOCKCHAIN',    tools: ['brain_poj_chain', 'brain_trace', 'brain_decisions'] },
  { name: 'META',          tools: ['brain_health', 'brain_meta', 'brain_consciousness'] },
];

for (const cat of categories) {
  console.log(`   ${cat.name.padEnd(14)} ${cat.tools.join(', ')}`);
}
console.log('');

// =============================================================================
// EXAMPLE 4: Using Tools via HTTP
// =============================================================================

console.log('── HTTP API USAGE ─────────────────────────────────────────────────');
console.log('');

console.log(`
// Judge an item via HTTP:

const response = await fetch('http://localhost:3000/api/tools/brain_cynic_judge', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    item: {
      type: 'code',
      content: 'function add(a, b) { return a + b; }',
      context: { language: 'javascript' }
    }
  })
});

const result = await response.json();
console.log(result);
// {
//   success: true,
//   result: {
//     Q: 72.5,
//     verdict: 'WAG',
//     dimensions: { ... },
//     weaknesses: [ ... ]
//   }
// }
`);

// =============================================================================
// EXAMPLE 5: Claude Code Integration
// =============================================================================

console.log('── CLAUDE CODE INTEGRATION ────────────────────────────────────────');
console.log('');

console.log(`
Add to your Claude Code settings (~/.claude.json):

{
  "mcpServers": {
    "cynic": {
      "command": "npx",
      "args": ["cynic-mcp"],
      "env": {
        "MCP_MODE": "stdio",
        "CYNIC_DATABASE_URL": "postgresql://...",
        "ANTHROPIC_API_KEY": "sk-ant-..."
      }
    }
  }
}

Then in Claude Code, you can use:
  - brain_cynic_judge to evaluate code
  - brain_search to find similar patterns
  - brain_patterns to detect issues
  - brain_health to check system status
`);

// =============================================================================
// EXAMPLE 6: Health Check
// =============================================================================

console.log('── HEALTH CHECK ───────────────────────────────────────────────────');
console.log('');

console.log(`
// Check server health:

const health = await fetch('http://localhost:3000/health');
const status = await health.json();

console.log(status);
// {
//   status: 'healthy',
//   tools: 60,
//   uptime: 3600,
//   services: {
//     postgres: 'connected',
//     redis: 'connected',
//     embeddings: 'ready'
//   }
// }
`);

// =============================================================================
// EXAMPLE 7: Starting Now
// =============================================================================

console.log('── QUICK START ────────────────────────────────────────────────────');
console.log('');
console.log('   # Install (if not already):');
console.log('   npm install -g @cynic/mcp');
console.log('');
console.log('   # Start in stdio mode (for Claude Code):');
console.log('   npx cynic-mcp');
console.log('');
console.log('   # Start in HTTP mode (for web access):');
console.log('   MCP_MODE=http PORT=3000 npx cynic-mcp');
console.log('');
console.log('   # With database (full features):');
console.log('   CYNIC_DATABASE_URL=postgresql://... npx cynic-mcp');
console.log('');

console.log('═══════════════════════════════════════════════════════════════════');
console.log('*tail wag* The pack awakens as one. CYNIC is ready.');
console.log('═══════════════════════════════════════════════════════════════════');
