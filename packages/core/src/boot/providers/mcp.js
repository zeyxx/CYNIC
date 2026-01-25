/**
 * MCP Server Provider
 *
 * Wraps MCPServer in the Lifecycle interface for bootCYNIC().
 * Provides graceful startup/shutdown and health checks.
 *
 * "The voice of the pack" - κυνικός
 *
 * @module @cynic/core/boot/providers/mcp
 */

'use strict';

import { createLifecycle, HealthStatus } from '../lifecycle.js';
import { registerProvider } from '../discovery.js';

/**
 * Register MCP-specific providers with the discovery system
 *
 * Call this before bootMCP() to enable MCP components.
 * Registers: migrations, mcp-server
 *
 * @param {Object} [options]
 * @param {number} [options.migrationTimeout=6180] - Migration timeout (φ⁻¹ × 10000)
 * @param {string} [options.mode] - MCP mode (stdio/http)
 * @param {number} [options.port] - HTTP port
 */
export function registerMCPProviders(options = {}) {
  const {
    migrationTimeout = 6180,
    mode = process.env.MCP_MODE || 'stdio',
    port = parseInt(process.env.PORT || '3000', 10),
  } = options;

  // Migrations provider
  registerProvider('migrations', {
    dependencies: ['config'],
    create: () => ({ applied: 0, error: null }),
    initialize: async (state) => {
      try {
        const { migrate } = await import('@cynic/persistence');

        const migrationPromise = migrate({ silent: false, exitOnError: false });
        const timeoutPromise = new Promise((_, reject) =>
          setTimeout(() => reject(new Error(`Migration timed out (${migrationTimeout}ms)`)), migrationTimeout)
        );

        const result = await Promise.race([migrationPromise, timeoutPromise]);
        state.applied = result.applied || 0;
      } catch (error) {
        console.warn('⚠️ Migration warning:', error.message);
        state.error = error.message;
      }
    },
    health: async (state) => ({
      status: state.error ? HealthStatus.DEGRADED : HealthStatus.HEALTHY,
      applied: state.applied,
      error: state.error,
    }),
  });

  // MCP Server provider
  registerProvider('mcp-server', {
    dependencies: ['config', 'migrations', 'engines'],
    create: () => ({ server: null, startTime: null, mode, port }),
    initialize: async (state) => {
      const { MCPServer } = await import('@cynic/mcp');
      state.server = new MCPServer({ mode: state.mode, port: state.port });
    },
    start: async (state) => {
      if (!state.server) {
        throw new Error('MCP Server not initialized');
      }
      state.startTime = Date.now();
      await state.server.start();
    },
    stop: async (state) => {
      if (state.server && typeof state.server.stop === 'function') {
        await state.server.stop();
      }
      state.server = null;
      state.startTime = null;
    },
    health: async (state) => {
      if (!state.server) {
        return { status: HealthStatus.UNHEALTHY, error: 'Server not running' };
      }
      return {
        status: HealthStatus.HEALTHY,
        mode: state.mode,
        port: state.mode === 'http' ? state.port : null,
        uptime: state.startTime ? Date.now() - state.startTime : 0,
      };
    },
  });
}

/**
 * Create MCP Server lifecycle provider
 *
 * @param {Object} options - MCP Server options
 * @param {string} [options.mode='stdio'] - Transport mode: 'stdio' or 'http'
 * @param {number} [options.port=3000] - HTTP port (only for http mode)
 * @param {Function} [options.serverFactory] - Custom server factory
 * @returns {Lifecycle}
 */
export function createMCPProvider(options = {}) {
  const {
    mode = process.env.MCP_MODE || 'stdio',
    port = parseInt(process.env.PORT || '3000', 10),
    serverFactory = null,
  } = options;

  let server = null;
  let startTime = null;

  const lifecycle = createLifecycle({
    name: 'mcp-server',
    dependencies: ['config'],

    initialize: async () => {
      // Dynamically import to avoid circular dependencies
      const { MCPServer } = await import('@cynic/mcp');

      // Create server instance
      if (serverFactory) {
        server = await serverFactory({ mode, port });
      } else {
        server = new MCPServer({ mode, port });
      }

      // Let MCPServer do its internal initialization
      // (ServiceInitializer runs during construction)
    },

    start: async () => {
      if (!server) {
        throw new Error('MCP Server not initialized');
      }
      startTime = Date.now();
      await server.start();
    },

    stop: async () => {
      if (server && typeof server.stop === 'function') {
        await server.stop();
      }
      server = null;
      startTime = null;
    },

    health: async () => {
      if (!server) {
        return {
          status: HealthStatus.UNHEALTHY,
          error: 'Server not running',
        };
      }

      return {
        status: HealthStatus.HEALTHY,
        mode,
        port: mode === 'http' ? port : null,
        uptime: startTime ? Date.now() - startTime : 0,
      };
    },
  });

  // Expose server instance for direct access
  Object.defineProperty(lifecycle, 'server', {
    get: () => server,
    enumerable: true,
  });

  return lifecycle;
}

/**
 * Create config provider (environment-based)
 *
 * @returns {Lifecycle}
 */
export function createConfigProvider() {
  let config = null;

  return createLifecycle({
    name: 'config',
    dependencies: [],

    initialize: async () => {
      // Load dotenv if available
      try {
        await import('dotenv/config');
      } catch {
        // dotenv not available, use process.env directly
      }

      config = {
        mcp: {
          mode: process.env.MCP_MODE || 'stdio',
          port: parseInt(process.env.PORT || '3000', 10),
        },
        database: {
          url: process.env.CYNIC_DATABASE_URL || process.env.DATABASE_URL,
        },
        redis: {
          url: process.env.CYNIC_REDIS_URL || process.env.REDIS_URL,
        },
        verbose: process.env.CYNIC_VERBOSE === 'true',
      };
    },

    health: async () => ({
      status: HealthStatus.HEALTHY,
      loaded: config !== null,
      hasDatabase: !!config?.database?.url,
      hasRedis: !!config?.redis?.url,
    }),
  });
}

/**
 * Create migrations provider
 *
 * @param {Object} [options]
 * @param {number} [options.timeout=6180] - Migration timeout (φ⁻¹ × 10000)
 * @returns {Lifecycle}
 */
export function createMigrationsProvider(options = {}) {
  const { timeout = 6180 } = options;
  let migrationResult = null;

  return createLifecycle({
    name: 'migrations',
    dependencies: ['config'],

    initialize: async () => {
      try {
        const { migrate } = await import('@cynic/persistence');

        const migrationPromise = migrate({ silent: false, exitOnError: false });
        const timeoutPromise = new Promise((_, reject) =>
          setTimeout(() => reject(new Error(`Migration timed out (${timeout}ms)`)), timeout)
        );

        migrationResult = await Promise.race([migrationPromise, timeoutPromise]);
      } catch (error) {
        // Don't fail boot on migration error - graceful degradation
        console.warn('⚠️ Migration warning:', error.message);
        migrationResult = { applied: 0, error: error.message };
      }
    },

    health: async () => ({
      status: migrationResult?.error ? HealthStatus.DEGRADED : HealthStatus.HEALTHY,
      applied: migrationResult?.applied || 0,
      error: migrationResult?.error || null,
    }),
  });
}
