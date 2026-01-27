/**
 * Engine System Provider
 *
 * Provides the 73 philosophical engines as a bootable component.
 * Auto-discovers and loads all engines during initialization.
 *
 * "The wisdom pack awakens" - κυνικός
 *
 * @module @cynic/core/boot/providers/engines
 */

'use strict';

import { EngineRegistry, globalEngineRegistry } from '../../engines/registry.js';
import { createOrchestrator } from '../../engines/orchestrator.js';
import { loadPhilosophyEngines, getLoadStatus } from '../../engines/philosophy/loader.js';
import { createLifecycle, HealthStatus } from '../lifecycle.js';
import { createLogger } from '../../logger.js';

const log = createLogger('EngineProvider');

/**
 * Create engine registry lifecycle component
 *
 * @param {Object} [options] - Options
 * @param {EngineRegistry} [options.registry] - Use existing registry (defaults to global)
 * @param {string[]} [options.domains] - Only load engines from these domains
 * @param {string[]} [options.exclude] - Exclude these engine IDs
 * @param {boolean} [options.silent] - Suppress console output
 * @returns {Lifecycle}
 */
export function createEngineRegistryProvider(options = {}) {
  const {
    registry = globalEngineRegistry,
    domains = null,
    exclude = [],
    silent = true,
  } = options;

  let loadResult = null;

  return createLifecycle({
    name: 'engine-registry',
    dependencies: [],

    initialize: async () => {
      // Load all philosophy engines
      loadResult = loadPhilosophyEngines({
        registry,
        domains,
        exclude,
        silent,
      });

      if (!loadResult.success) {
        log.warn('Some engines failed to load', { failed: loadResult.failed });
      }
    },

    health: async () => {
      const status = getLoadStatus();
      return {
        status: status.loaded && loadResult?.failed === 0
          ? HealthStatus.HEALTHY
          : status.loaded
            ? HealthStatus.DEGRADED
            : HealthStatus.UNHEALTHY,
        engines: {
          total: registry.size,
          registered: status.enginesRegistered,
          failed: status.enginesFailed,
          loadDuration: status.endTime - status.startTime,
        },
        stats: registry.getStats(),
      };
    },
  });
}

/**
 * Create engine orchestrator lifecycle component
 *
 * @param {Object} [options] - Options
 * @param {EngineRegistry} [options.registry] - Registry to use
 * @returns {Lifecycle}
 */
export function createEngineOrchestratorProvider(options = {}) {
  const {
    registry = globalEngineRegistry,
  } = options;

  let orchestrator = null;

  const lifecycle = createLifecycle({
    name: 'engine-orchestrator',
    dependencies: ['engine-registry'],

    initialize: async () => {
      orchestrator = createOrchestrator({ registry });
    },

    health: async () => ({
      status: orchestrator ? HealthStatus.HEALTHY : HealthStatus.UNHEALTHY,
      initialized: !!orchestrator,
    }),
  });

  // Expose orchestrator instance
  Object.defineProperty(lifecycle, 'orchestrator', {
    get: () => orchestrator,
  });

  return lifecycle;
}

/**
 * Create a combined engines provider that includes both registry and orchestrator
 *
 * @param {Object} [options] - Options
 * @returns {Lifecycle}
 */
export function createEnginesProvider(options = {}) {
  const {
    domains = null,
    exclude = [],
    silent = true,
  } = options;

  const registry = new EngineRegistry();
  let orchestrator = null;
  let loadResult = null;

  const lifecycle = createLifecycle({
    name: 'engines',
    dependencies: [],

    initialize: async () => {
      // Load philosophy engines
      loadResult = loadPhilosophyEngines({
        registry,
        domains,
        exclude,
        silent,
      });

      // Create orchestrator
      orchestrator = createOrchestrator({ registry });

      if (loadResult.failed > 0) {
        log.warn('Some engines failed to load', { failed: loadResult.failed });
      }
    },

    health: async () => {
      const stats = registry.getStats();
      const totalEngines = stats.totalEngines || 0;
      const hasFailures = loadResult?.failed > 0;

      return {
        status: hasFailures ? HealthStatus.DEGRADED : HealthStatus.HEALTHY,
        engines: {
          total: totalEngines,
          loaded: loadResult?.registered || totalEngines,
          failed: loadResult?.failed || 0,
          duration: loadResult?.duration || 0,
        },
        domains: stats.domains || 0,
        capabilities: stats.capabilities || 0,
      };
    },
  });

  // Expose instances directly on the lifecycle object
  lifecycle.registry = registry;

  // Orchestrator is created during init, so use getter
  Object.defineProperty(lifecycle, 'orchestrator', {
    get: () => orchestrator,
    enumerable: true,
  });

  return lifecycle;
}
