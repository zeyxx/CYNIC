#!/usr/bin/env node

/**
 * CYNIC MCP Server CLI
 *
 * Unified entry point using the boot system.
 *
 * Run: cynic-mcp
 *
 * Environment variables:
 *   MCP_MODE           - Transport mode: 'stdio' (default) or 'http'
 *   PORT               - HTTP port (default: 3000, for http mode)
 *   CYNIC_DATABASE_URL - PostgreSQL connection string
 *   CYNIC_REDIS_URL    - Redis connection string
 *   CYNIC_VERBOSE      - Enable verbose logging
 *
 * "The pack awakens as one" - κυνικός
 */

'use strict';

import { bootMCP } from '@cynic/core/boot';
import { logConfigStatus, validateStartupConfig } from '@cynic/core';

// ============================================================================
// CONFIGURATION VALIDATION
// ============================================================================

try {
  validateStartupConfig();
} catch (err) {
  console.error(err.message);
  process.exit(1);
}

logConfigStatus();

// ============================================================================
// BOOT SEQUENCE
// ============================================================================

console.log('🐕 CYNIC MCP Server starting...');
console.log('');

try {
  const cynic = await bootMCP({
    silent: false,
  });

  // Log boot result
  console.log('');
  console.log(`✅ CYNIC MCP Server ready (${cynic.duration}ms)`);
  console.log(`   Components: ${cynic.components.join(' → ')}`);
  console.log('');

  // Health check in verbose mode
  if (process.env.CYNIC_VERBOSE === 'true') {
    setInterval(async () => {
      const health = await cynic.health();
      console.log('[HEALTH]', health.status, health.summary || '');
    }, 60000);
  }

  // Handle graceful shutdown
  const shutdown = async (signal) => {
    console.log('');
    console.log(`🛑 Received ${signal}, shutting down...`);
    await cynic.shutdown();
    process.exit(0);
  };

  process.on('SIGTERM', () => shutdown('SIGTERM'));
  process.on('SIGINT', () => shutdown('SIGINT'));

} catch (error) {
  console.error('');
  console.error('❌ Boot failed:', error.message);
  if (process.env.CYNIC_VERBOSE === 'true') {
    console.error(error.stack);
  }
  console.error('');
  process.exit(1);
}
