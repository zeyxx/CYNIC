/**
 * Philosophy Engines Module
 *
 * 73 philosophical engines organized by domain.
 * Adapts legacy scripts/lib/ engines to the new Engine system.
 *
 * "The wisdom of ages, structured" - κυνικός
 *
 * @module @cynic/core/engines/philosophy
 */

'use strict';

// Adapter for legacy engines
export { adaptLegacyEngine, adaptLegacyEngines } from './adapter.js';

// Engine catalog (metadata for all 73 engines)
export {
  PHILOSOPHY_ENGINE_CATALOG,
  getCatalogByDomain,
  getCatalogByCapability,
  getCatalogByTradition,
  getCatalogStats,
} from './catalog.js';

// Loader for registering engines
export {
  loadPhilosophyEngines,
  loadPhilosophyEngine,
  loadEnginesByDomain,
  loadEnginesByTradition,
  areEnginesLoaded,
  getLoadStatus,
  unloadPhilosophyEngines,
} from './loader.js';
