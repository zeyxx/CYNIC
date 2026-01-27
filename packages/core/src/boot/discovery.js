/**
 * CYNIC Component Discovery
 *
 * Auto-discovers and registers CYNIC components.
 * Scans packages, detects capabilities, wires dependencies.
 *
 * "The pack finds its own" - κυνικός
 *
 * @module @cynic/core/boot/discovery
 */

'use strict';

import { createLifecycle, HealthStatus } from './lifecycle.js';
import { createLogger } from '../logger.js';
import { EngineRegistry } from '../engines/registry.js';
import { loadPhilosophyEngines } from '../engines/philosophy/loader.js';

/**
 * Component registry for discovered components
 */
const discoveredComponents = new Map();

/**
 * Component providers - functions that create lifecycle-wrapped components
 */
const componentProviders = new Map();

/**
 * Cached module imports (lazy loaded)
 * @type {Map<string, any>}
 */
const moduleCache = new Map();

/**
 * Lazy load a module from @cynic packages
 * @param {string} modulePath - Module path
 * @returns {Promise<any>}
 */
async function lazyImport(modulePath) {
  if (moduleCache.has(modulePath)) {
    return moduleCache.get(modulePath);
  }

  const mod = await import(modulePath);
  moduleCache.set(modulePath, mod);
  return mod;
}

/**
 * Register a component provider
 *
 * @param {string} name - Component name
 * @param {Object} config - Provider configuration
 * @param {string[]} [config.dependencies] - Component dependencies
 * @param {Function} config.create - Factory function that creates the component (can be async)
 * @param {Function} [config.initialize] - Initialize callback
 * @param {Function} [config.start] - Start callback
 * @param {Function} [config.stop] - Stop callback
 * @param {Function} [config.health] - Health check callback
 *
 * @example
 * registerProvider('postgres', {
 *   dependencies: ['config'],
 *   create: async (deps) => {
 *     const { PostgresClient } = await import('@cynic/persistence');
 *     return new PostgresClient(deps.config.database);
 *   },
 *   initialize: async (client) => await client.connect(),
 *   stop: async (client) => await client.close(),
 *   health: async (client) => ({
 *     status: client.isConnected ? 'healthy' : 'unhealthy'
 *   }),
 * });
 */
export function registerProvider(name, config) {
  componentProviders.set(name, config);
}

/**
 * Discover and create all components from registered providers
 * Now async to support dynamic imports
 *
 * @param {Object} [context] - Shared context (config, etc.)
 * @returns {Promise<Map<string, Lifecycle>>} Discovered components
 */
export async function discoverComponents(context = {}) {
  const resolved = new Map();
  const resolving = new Set();

  const resolve = async (name) => {
    if (resolved.has(name)) {
      return resolved.get(name);
    }

    if (resolving.has(name)) {
      throw new Error(`Circular dependency detected: ${name}`);
    }

    const provider = componentProviders.get(name);
    if (!provider) {
      throw new Error(`Unknown component: ${name}`);
    }

    resolving.add(name);

    // Resolve dependencies first
    const deps = {};
    for (const depName of (provider.dependencies || [])) {
      deps[depName] = await resolve(depName);
      // Get actual instance from lifecycle wrapper
      if (deps[depName]?.instance !== undefined) {
        deps[depName] = deps[depName].instance;
      }
    }

    // Create the component instance (now supports async)
    const instance = await Promise.resolve(provider.create(deps, context));

    // Wrap in lifecycle
    const lifecycle = createLifecycle({
      name,
      dependencies: provider.dependencies || [],
      initialize: provider.initialize
        ? () => provider.initialize(instance, deps, context)
        : undefined,
      start: provider.start
        ? () => provider.start(instance, deps, context)
        : undefined,
      stop: provider.stop
        ? () => provider.stop(instance, deps, context)
        : undefined,
      health: provider.health
        ? () => provider.health(instance, deps, context)
        : undefined,
    });

    // Store instance for access
    lifecycle.instance = instance;

    resolving.delete(name);
    resolved.set(name, lifecycle);
    discoveredComponents.set(name, lifecycle);

    return lifecycle;
  };

  // Resolve all registered providers
  for (const name of componentProviders.keys()) {
    await resolve(name);
  }

  return resolved;
}

/**
 * Get a discovered component's instance
 *
 * @param {string} name - Component name
 * @returns {any} Component instance
 */
export function getComponent(name) {
  const lifecycle = discoveredComponents.get(name);
  return lifecycle?.instance;
}

/**
 * Clear all discovered components (for testing)
 */
export function clearDiscovery() {
  discoveredComponents.clear();
  componentProviders.clear();
  moduleCache.clear();
}

// ============================================================================
// CYNIC Standard Component Providers
// ============================================================================

/**
 * Register standard CYNIC component providers
 *
 * These are the built-in components that CYNIC auto-discovers.
 * Additional components can be registered by other packages.
 */
export function registerStandardProviders() {
  // Config provider (always first)
  registerProvider('config', {
    dependencies: [],
    create: (deps, context) => {
      return context.config || loadDefaultConfig();
    },
    health: async (_config) => ({
      status: HealthStatus.HEALTHY,
      loaded: true,
    }),
  });

  // Logger provider
  registerProvider('logger', {
    dependencies: ['config'],
    create: (deps) => {
      return createLogger('CYNIC', deps.config.log);
    },
    health: async () => ({ status: HealthStatus.HEALTHY }),
  });

  // PostgreSQL provider (optional - graceful degradation if not configured)
  registerProvider('postgres', {
    dependencies: ['config', 'logger'],
    create: async (deps) => {
      if (!deps.config.database?.url) {
        deps.logger?.info?.('PostgreSQL not configured - using in-memory fallback');
        return null; // PostgreSQL is optional
      }
      const { PostgresClient } = await lazyImport('@cynic/persistence');
      return new PostgresClient(deps.config.database);
    },
    initialize: async (client) => {
      if (client) await client.connect();
    },
    stop: async (client) => {
      if (client) await client.close();
    },
    health: async (client) => ({
      status: client?.isConnected?.() ? HealthStatus.HEALTHY : HealthStatus.UNHEALTHY,
      poolSize: client?.getPoolStats?.(),
    }),
  });

  // Redis provider (optional)
  registerProvider('redis', {
    dependencies: ['config', 'logger'],
    create: async (deps) => {
      if (!deps.config.redis?.url) {
        return null; // Redis is optional
      }
      const { RedisClient } = await lazyImport('@cynic/persistence');
      return new RedisClient(deps.config.redis);
    },
    initialize: async (client) => {
      if (client) await client.connect();
    },
    stop: async (client) => {
      if (client) await client.close();
    },
    health: async (client) => {
      if (!client) {
        return { status: HealthStatus.HEALTHY, available: false };
      }
      return {
        status: client.isReady?.() ? HealthStatus.HEALTHY : HealthStatus.UNHEALTHY,
        available: true,
      };
    },
  });

  // Engine Registry provider
  registerProvider('engines', {
    dependencies: ['config', 'logger'],
    create: () => {
      return new EngineRegistry();
    },
    initialize: async (registry, deps) => {
      // Load philosophy engines
      loadPhilosophyEngines({ registry, silent: !deps.config.verbose });
    },
    health: async (registry) => ({
      status: HealthStatus.HEALTHY,
      engineCount: registry.size,
    }),
  });

  // Judge provider
  registerProvider('judge', {
    dependencies: ['config', 'postgres', 'redis', 'engines'],
    create: async (deps) => {
      const { CYNICJudge } = await lazyImport('@cynic/node');
      const { createEngineIntegration } = await lazyImport('@cynic/node/judge/engine-integration.js');

      // Create engine integration from loaded engines
      const engineIntegration = deps.engines
        ? createEngineIntegration({ registry: deps.engines })
        : null;

      return new CYNICJudge({
        postgres: deps.postgres,
        redis: deps.redis,
        engineIntegration,
        consultEngines: true, // Enable engine consultation by default
      });
    },
    health: async (judge) => ({
      status: HealthStatus.HEALTHY,
      initialized: true,
      consultEngines: judge?.consultEngines || false,
    }),
  });
}

/**
 * Load default config from environment
 */
function loadDefaultConfig() {
  return {
    database: {
      url: process.env.CYNIC_DATABASE_URL,
      poolSize: parseInt(process.env.CYNIC_POOL_SIZE || '21', 10),
    },
    redis: {
      url: process.env.CYNIC_REDIS_URL,
    },
    log: {
      level: process.env.CYNIC_LOG_LEVEL || 'info',
    },
    verbose: process.env.CYNIC_VERBOSE === 'true',
  };
}
