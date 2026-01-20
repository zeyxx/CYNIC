#!/usr/bin/env node

/**
 * CYNIC MCP Server CLI
 *
 * Run: cynic-mcp
 *
 * Environment variables:
 *   MCP_MODE           - Transport mode: 'stdio' (default) or 'http'
 *   PORT               - HTTP port (default: 3000, for http mode)
 *   CYNIC_DATABASE_URL - PostgreSQL connection string
 *   CYNIC_REDIS_URL    - Redis connection string
 *
 * "φ distrusts φ" - κυνικός
 */

'use strict';

// Load environment variables from .env file
import 'dotenv/config';

import { MCPServer } from '../src/server.js';
import { logConfigStatus, getMcpConfig, validateStartupConfig } from '@cynic/core';
import { migrate } from '@cynic/persistence';

// Validate configuration at startup (throws in production if misconfigured)
try {
  validateStartupConfig();
} catch (err) {
  console.error(err.message);
  process.exit(1);
}

// Log configuration status (never logs actual secrets)
logConfigStatus();

// Run database migrations before starting (auto-migrate on deploy)
try {
  console.log('🐕 Running auto-migrations...');
  const result = await migrate({ silent: false, exitOnError: false });
  if (result.applied > 0) {
    console.log(`✅ Applied ${result.applied} migration(s)`);
  } else {
    console.log('✅ Database schema up to date');
  }
} catch (err) {
  console.error('⚠️ Migration warning:', err.message);
  console.error('   Server will start but some features may not work');
  // Don't exit - let the server start anyway (graceful degradation)
}

// Get MCP configuration
const { mode: configMode, port: configPort } = getMcpConfig();

// Determine mode: http if PORT is set or MCP_MODE=http
const port = configPort;
const mode = configMode || (process.env.PORT ? 'http' : 'stdio');

// Start MCP server
const server = new MCPServer({
  mode,
  port,
});

server.start().catch(err => {
  console.error('Failed to start MCP server:', err);
  process.exit(1);
});

// Handle graceful shutdown
process.on('SIGTERM', async () => {
  console.error('Received SIGTERM, shutting down...');
  await server.stop();
  process.exit(0);
});

process.on('SIGINT', async () => {
  console.error('Received SIGINT, shutting down...');
  await server.stop();
  process.exit(0);
});
