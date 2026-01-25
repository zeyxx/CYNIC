/**
 * Philosophy Engine Loader
 *
 * Loads and registers all 73 philosophical engines from scripts/lib/.
 * Uses the catalog for metadata and adapter for wrapping.
 *
 * "Awaken the pack" - κυνικός
 *
 * @module @cynic/core/engines/philosophy/loader
 */

'use strict';

import { globalEngineRegistry } from '../registry.js';
import { adaptLegacyEngine } from './adapter.js';
import { PHILOSOPHY_ENGINE_CATALOG, getCatalogStats } from './catalog.js';

/**
 * Load status tracking
 */
const loadStatus = {
  loaded: false,
  enginesRegistered: 0,
  enginesFailed: 0,
  errors: [],
  startTime: null,
  endTime: null,
};

/**
 * Load all philosophy engines into the global registry
 *
 * @param {Object} [options] - Loading options
 * @param {EngineRegistry} [options.registry] - Registry to use (defaults to global)
 * @param {boolean} [options.silent] - Suppress console output
 * @param {string[]} [options.only] - Only load these engine IDs
 * @param {string[]} [options.exclude] - Exclude these engine IDs
 * @param {string[]} [options.domains] - Only load engines from these domains
 * @returns {Object} Load result
 *
 * @example
 * // Load all engines
 * const result = loadPhilosophyEngines();
 *
 * // Load only ethics engines
 * const result = loadPhilosophyEngines({ domains: ['ethics'] });
 *
 * // Exclude specific engines
 * const result = loadPhilosophyEngines({ exclude: ['decision-engine'] });
 */
export function loadPhilosophyEngines(options = {}) {
  const {
    registry = globalEngineRegistry,
    silent = false,
    only = null,
    exclude = [],
    domains = null,
  } = options;

  loadStatus.startTime = Date.now();
  loadStatus.enginesRegistered = 0;
  loadStatus.enginesFailed = 0;
  loadStatus.errors = [];

  // Filter catalog based on options
  let catalog = [...PHILOSOPHY_ENGINE_CATALOG];

  if (only && only.length > 0) {
    catalog = catalog.filter(e => only.includes(e.id));
  }

  if (exclude.length > 0) {
    catalog = catalog.filter(e => !exclude.includes(e.id));
  }

  if (domains && domains.length > 0) {
    catalog = catalog.filter(e =>
      domains.includes(e.domain) ||
      (e.subdomains && e.subdomains.some(d => domains.includes(d)))
    );
  }

  if (!silent) {
    console.log(`Loading ${catalog.length} philosophy engines...`);
  }

  // Load each engine
  for (const entry of catalog) {
    try {
      // Skip if already registered
      if (registry.has(entry.id)) {
        if (!silent) {
          console.log(`  [SKIP] ${entry.id} (already registered)`);
        }
        continue;
      }

      // Adapt legacy engine
      const engine = adaptLegacyEngine({
        id: entry.id,
        legacyPath: `${entry.file}.cjs`,
        domain: entry.domain,
        subdomains: entry.subdomains || [],
        capabilities: entry.capabilities,
        tradition: entry.tradition,
        description: entry.description,
      });

      // Register
      registry.register(engine);
      loadStatus.enginesRegistered++;

      if (!silent) {
        console.log(`  [OK] ${entry.id}`);
      }
    } catch (error) {
      loadStatus.enginesFailed++;
      loadStatus.errors.push({
        id: entry.id,
        error: error.message,
      });

      if (!silent) {
        console.log(`  [FAIL] ${entry.id}: ${error.message}`);
      }
    }
  }

  loadStatus.endTime = Date.now();
  loadStatus.loaded = true;

  const result = {
    success: loadStatus.enginesFailed === 0,
    registered: loadStatus.enginesRegistered,
    failed: loadStatus.enginesFailed,
    errors: loadStatus.errors,
    duration: loadStatus.endTime - loadStatus.startTime,
    registryStats: registry.getStats(),
  };

  if (!silent) {
    console.log(`\nLoaded ${result.registered} engines in ${result.duration}ms`);
    if (result.failed > 0) {
      console.log(`  ${result.failed} failed to load`);
    }
  }

  return result;
}

/**
 * Load a single philosophy engine by ID
 *
 * @param {string} id - Engine ID
 * @param {Object} [options] - Options
 * @returns {Engine|null}
 */
export function loadPhilosophyEngine(id, options = {}) {
  const { registry = globalEngineRegistry } = options;

  // Check if already loaded
  if (registry.has(id)) {
    return registry.get(id);
  }

  // Find in catalog
  const entry = PHILOSOPHY_ENGINE_CATALOG.find(e => e.id === id);
  if (!entry) {
    throw new Error(`Unknown engine: ${id}`);
  }

  // Adapt and register
  const engine = adaptLegacyEngine({
    id: entry.id,
    legacyPath: `${entry.file}.cjs`,
    domain: entry.domain,
    subdomains: entry.subdomains || [],
    capabilities: entry.capabilities,
    tradition: entry.tradition,
    description: entry.description,
  });

  registry.register(engine);
  return engine;
}

/**
 * Load engines by domain
 *
 * @param {string} domain - Domain to load
 * @param {Object} [options] - Options
 * @returns {Object} Load result
 */
export function loadEnginesByDomain(domain, options = {}) {
  return loadPhilosophyEngines({
    ...options,
    domains: [domain],
  });
}

/**
 * Load engines by tradition
 *
 * @param {string} tradition - Tradition to load
 * @param {Object} [options] - Options
 * @returns {Object} Load result
 */
export function loadEnginesByTradition(tradition, options = {}) {
  const traditionEngines = PHILOSOPHY_ENGINE_CATALOG
    .filter(e => e.tradition === tradition)
    .map(e => e.id);

  return loadPhilosophyEngines({
    ...options,
    only: traditionEngines,
  });
}

/**
 * Check if engines are loaded
 *
 * @returns {boolean}
 */
export function areEnginesLoaded() {
  return loadStatus.loaded;
}

/**
 * Get load status
 *
 * @returns {Object}
 */
export function getLoadStatus() {
  return { ...loadStatus };
}

/**
 * Unload all philosophy engines from registry
 *
 * @param {Object} [options] - Options
 * @returns {number} Number of engines unloaded
 */
export function unloadPhilosophyEngines(options = {}) {
  const { registry = globalEngineRegistry } = options;

  let unloaded = 0;
  for (const entry of PHILOSOPHY_ENGINE_CATALOG) {
    if (registry.unregister(entry.id)) {
      unloaded++;
    }
  }

  loadStatus.loaded = false;
  loadStatus.enginesRegistered = 0;

  return unloaded;
}

/**
 * Get philosophy engine catalog stats
 *
 * @returns {Object}
 */
export { getCatalogStats };

export default loadPhilosophyEngines;
