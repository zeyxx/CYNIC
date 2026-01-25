/**
 * CYNIC Boot System
 *
 * Unified boot sequence for all CYNIC components.
 * Auto-discovers, wires, and starts all services in dependency order.
 *
 * "The pack awakens" - κυνικός
 *
 * @module @cynic/core/boot
 */

'use strict';

import { globalBootManager, BootManager, BootState, BootEvent } from './boot-manager.js';
import { Lifecycle, LifecycleState, HealthStatus, createLifecycle } from './lifecycle.js';
import {
  registerProvider,
  discoverComponents,
  getComponent,
  clearDiscovery,
  registerStandardProviders,
} from './discovery.js';

// Re-export everything
export {
  // Boot Manager
  BootManager,
  BootState,
  BootEvent,
  globalBootManager,

  // Lifecycle
  Lifecycle,
  LifecycleState,
  HealthStatus,
  createLifecycle,

  // Discovery
  registerProvider,
  discoverComponents,
  getComponent,
  clearDiscovery,
  registerStandardProviders,
};

/**
 * Boot CYNIC with auto-discovery
 *
 * This is the main entry point for starting CYNIC.
 * It discovers all registered components, resolves dependencies,
 * and boots everything in the correct order.
 *
 * @param {Object} [options] - Boot options
 * @param {Object} [options.config] - Override configuration
 * @param {string[]} [options.only] - Only boot these components
 * @param {string[]} [options.exclude] - Exclude these components
 * @param {boolean} [options.silent] - Suppress console output
 * @param {Lifecycle[]} [options.additional] - Additional components to register
 * @returns {Promise<Object>} Boot result with component handles
 *
 * @example
 * // Boot with defaults
 * const cynic = await bootCYNIC();
 * console.log(`CYNIC running with ${cynic.components.length} components`);
 *
 * // Access components
 * const judge = cynic.get('judge');
 * await judge.judge({ type: 'code', data: ... });
 *
 * // Shutdown gracefully
 * await cynic.shutdown();
 *
 * @example
 * // Boot with custom config
 * const cynic = await bootCYNIC({
 *   config: {
 *     database: { url: 'postgres://...' },
 *   },
 * });
 *
 * @example
 * // Boot only specific components
 * const cynic = await bootCYNIC({
 *   only: ['config', 'postgres', 'judge'],
 * });
 */
export async function bootCYNIC(options = {}) {
  const {
    config,
    only,
    exclude = [],
    silent = false,
    additional = [],
  } = options;

  const log = silent
    ? () => {}
    : (msg) => console.log(`[CYNIC] ${msg}`);

  log('Initializing boot sequence...');

  // Create a fresh boot manager for this boot
  const bootManager = new BootManager();

  // Register standard providers if not already done
  registerStandardProviders();

  // Discover all components (now async for dynamic imports)
  const context = { config: config || {} };
  const discovered = await discoverComponents(context);

  // Filter components
  let componentNames = Array.from(discovered.keys());

  if (only && only.length > 0) {
    componentNames = componentNames.filter(name => only.includes(name));
  }

  if (exclude.length > 0) {
    componentNames = componentNames.filter(name => !exclude.includes(name));
  }

  // Register discovered components
  for (const name of componentNames) {
    const component = discovered.get(name);
    if (component) {
      bootManager.register(component);
    }
  }

  // Register additional components
  for (const component of additional) {
    bootManager.register(component);
  }

  log(`Discovered ${bootManager.getComponentNames().length} components`);

  // Setup event listeners for logging
  if (!silent) {
    bootManager.on(BootEvent.COMPONENT_INITIALIZING, ({ name }) => {
      log(`  [INIT] ${name}...`);
    });

    bootManager.on(BootEvent.COMPONENT_STARTED, ({ name }) => {
      log(`  [OK] ${name}`);
    });

    bootManager.on(BootEvent.COMPONENT_FAILED, ({ name, error, phase }) => {
      log(`  [FAIL] ${name} (${phase}): ${error.message}`);
    });
  }

  // Boot all components
  log('Starting boot sequence...');
  const result = await bootManager.boot();

  log(`Boot completed in ${result.duration}ms`);
  log(`Components: ${result.components.join(', ')}`);

  // Return a handle to interact with CYNIC
  return {
    /**
     * Boot result
     */
    ...result,

    /**
     * Boot manager reference
     */
    bootManager,

    /**
     * Get a component instance by name
     */
    get: (name) => getComponent(name),

    /**
     * Get all component instances
     */
    getAll: () => {
      const instances = {};
      for (const name of result.components) {
        instances[name] = getComponent(name);
      }
      return instances;
    },

    /**
     * Check health of all components
     */
    health: () => bootManager.health(),

    /**
     * Shutdown CYNIC gracefully
     */
    shutdown: () => bootManager.shutdown(),

    /**
     * Wait for shutdown signal
     */
    waitForShutdown: () => {
      return new Promise((resolve) => {
        bootManager.on(BootEvent.SHUTDOWN_COMPLETED, resolve);
      });
    },
  };
}

/**
 * Quick boot for development/testing
 * Boots minimal components without external services
 *
 * @param {Object} [options] - Boot options
 * @returns {Promise<Object>}
 */
export async function bootMinimal(options = {}) {
  return bootCYNIC({
    ...options,
    only: ['config', 'logger', 'engines'],
  });
}

/**
 * Boot for MCP server mode
 *
 * Registers MCP-specific providers (migrations, mcp-server)
 * then boots the system excluding P2P components.
 *
 * @param {Object} [options] - Boot options
 * @param {number} [options.migrationTimeout=6180] - Migration timeout (φ⁻¹ × 10000)
 * @param {string} [options.mode] - MCP transport mode
 * @param {number} [options.port] - HTTP port for http mode
 * @returns {Promise<Object>}
 */
export async function bootMCP(options = {}) {
  const { migrationTimeout, mode, port, ...restOptions } = options;

  // Register MCP-specific providers
  const { registerMCPProviders } = await import('./providers/mcp.js');
  registerMCPProviders({ migrationTimeout, mode, port });

  return bootCYNIC({
    ...restOptions,
    exclude: ['node', 'transport', 'consensus', ...(restOptions.exclude || [])],
  });
}

/**
 * Boot for P2P node mode
 *
 * @param {Object} [options] - Boot options
 * @returns {Promise<Object>}
 */
export async function bootNode(options = {}) {
  return bootCYNIC({
    ...options,
    exclude: ['mcp'],
  });
}

// Default export
export default bootCYNIC;
