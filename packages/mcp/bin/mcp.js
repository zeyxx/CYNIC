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
import { logConfigStatus, getMcpConfig, detectEnvironment } from '@cynic/core';

// Log configuration status (never logs actual secrets)
logConfigStatus();

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
