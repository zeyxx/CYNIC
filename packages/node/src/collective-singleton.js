/**
 * Collective Pack Singleton
 *
 * "One pack, one truth" - κυνικός
 *
 * This module guarantees a SINGLE CollectivePack instance across the entire system.
 * All orchestrators, routers, and hooks must use this singleton.
 *
 * WHY SINGLETON?
 * - Dogs share memory (SharedMemory)
 * - Dogs build consensus (need same state)
 * - Q-Learning needs consistent routing decisions
 * - Avoids "two packs" problem where hooks and MCP have different Dogs
 *
 * USAGE:
 *   import { getCollectivePack, getSharedMemory } from '@cynic/node';
 *   const pack = getCollectivePack();  // Always returns same instance
 *
 * φ-aligned: First call initializes, subsequent calls return cached instance.
 *
 * @module @cynic/node/collective-singleton
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';
import { createCollectivePack } from './agents/collective/index.js';
import { SharedMemory } from './memory/shared-memory.js';
import { getQLearningService } from './orchestration/learning-service.js';

const log = createLogger('CollectiveSingleton');

// ═══════════════════════════════════════════════════════════════════════════════
// SHARED MEMORY STORAGE ADAPTER (D5: bridges persistence → SharedMemory get/set)
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Adapts PersistenceManager to SharedMemory's storage interface (get/set).
 * Uses a dedicated PostgreSQL table row for SharedMemory state.
 */
class PersistenceStorageAdapter {
  constructor(persistence) {
    this._persistence = persistence;
  }

  async get(key) {
    try {
      const result = await this._persistence.query?.(
        'SELECT data FROM cynic_kv WHERE key = $1',
        [key]
      );
      if (result?.rows?.[0]?.data) {
        return typeof result.rows[0].data === 'string'
          ? JSON.parse(result.rows[0].data)
          : result.rows[0].data;
      }
      return null;
    } catch (e) {
      log.debug(`Storage adapter get(${key}) failed: ${e.message}`);
      return null;
    }
  }

  async set(key, value) {
    try {
      await this._persistence.query?.(
        `INSERT INTO cynic_kv (key, data, updated_at) VALUES ($1, $2, NOW())
         ON CONFLICT (key) DO UPDATE SET data = $2, updated_at = NOW()`,
        [key, JSON.stringify(value)]
      );
    } catch (e) {
      log.debug(`Storage adapter set(${key}) failed: ${e.message}`);
    }
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON STATE
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * The global CollectivePack instance - null until first getCollectivePack() call
 * @type {CollectivePack|null}
 */
let _globalPack = null;

/**
 * The global SharedMemory instance - null until first access
 * @type {SharedMemory|null}
 */
let _sharedMemory = null;

/**
 * The global QLearningService instance - null until first access
 * @type {QLearningService|null}
 */
let _qLearningService = null;

/**
 * Initialization promise to prevent race conditions
 * @type {Promise|null}
 */
let _initPromise = null;

/**
 * Track if pack has been awakened
 * @type {boolean}
 */
let _isAwakened = false;

// ═══════════════════════════════════════════════════════════════════════════════
// DEFAULT OPTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Default options for CollectivePack creation
 * These are φ-aligned values that should rarely be overridden
 */
const DEFAULT_OPTIONS = Object.freeze({
  consensusThreshold: PHI_INV,  // 61.8% - φ⁻¹
  mode: 'parallel',
  profileLevel: 3,
  enableEventBus: true,
});

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON ACCESSORS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Get the SharedMemory singleton
 *
 * SharedMemory is created lazily and shared across all Dogs.
 * It contains:
 * - Patterns (max 1597 - F17)
 * - JudgmentIndex (similarity search)
 * - DimensionWeights (learned per dimension)
 * - Procedures (per item type)
 * - EWC++ Fisher scores (prevent catastrophic forgetting)
 *
 * @param {Object} [options] - Options for SharedMemory (only used on first call)
 * @returns {SharedMemory} The singleton SharedMemory instance
 */
export function getSharedMemory(options = {}) {
  if (!_sharedMemory) {
    // D5: Wire storage adapter if persistence is available
    const storageOptions = { ...options };
    if (options.persistence && !options.storage) {
      storageOptions.storage = new PersistenceStorageAdapter(options.persistence);
    }
    _sharedMemory = new SharedMemory(storageOptions);
    log.debug('SharedMemory singleton created', { hasStorage: !!storageOptions.storage });
  }
  return _sharedMemory;
}

/**
 * Get the CollectivePack singleton
 *
 * This is the ONLY way to get the CollectivePack.
 * Never use createCollectivePack() directly in production code.
 *
 * First call initializes the pack with provided options.
 * Subsequent calls return the same instance (options ignored).
 *
 * @param {Object} [options] - Options (only used on first call)
 * @param {Object} [options.judge] - CYNICJudge instance
 * @param {Object} [options.persistence] - PersistenceManager instance
 * @param {Object} [options.sharedMemory] - SharedMemory instance (default: singleton)
 * @param {Object} [options.eventBus] - EventBus instance
 * @param {number} [options.consensusThreshold] - Consensus threshold (default: φ⁻¹)
 * @returns {CollectivePack} The singleton CollectivePack instance
 */
export function getCollectivePack(options = {}) {
  if (!_globalPack) {
    // Ensure SharedMemory exists
    const sharedMemory = options.sharedMemory || getSharedMemory();

    // Merge with defaults
    const finalOptions = {
      ...DEFAULT_OPTIONS,
      ...options,
      sharedMemory,
    };

    // Create the pack
    _globalPack = createCollectivePack(finalOptions);

    log.info('CollectivePack singleton created', {
      consensusThreshold: finalOptions.consensusThreshold,
      mode: finalOptions.mode,
      profileLevel: finalOptions.profileLevel,
      hasPersistence: !!finalOptions.persistence,
      hasJudge: !!finalOptions.judge,
    });
  }

  return _globalPack;
}

/**
 * Get the CollectivePack singleton asynchronously
 *
 * Use this when you need to ensure the pack is fully initialized
 * before proceeding (e.g., during startup).
 *
 * @param {Object} [options] - Options (only used on first call)
 * @returns {Promise<CollectivePack>} The singleton CollectivePack instance
 */
export async function getCollectivePackAsync(options = {}) {
  // If already initialized, return immediately
  if (_globalPack) {
    return _globalPack;
  }

  // If initialization is in progress, wait for it
  if (_initPromise) {
    return _initPromise;
  }

  // Start initialization
  _initPromise = (async () => {
    const pack = getCollectivePack(options);

    // If persistence is provided, try to load persisted state
    if (options.persistence) {
      // D5: Ensure cynic_kv table exists for SharedMemory storage
      try {
        await options.persistence.query?.(
          `CREATE TABLE IF NOT EXISTS cynic_kv (
             key TEXT PRIMARY KEY,
             data JSONB NOT NULL DEFAULT '{}',
             updated_at TIMESTAMPTZ DEFAULT NOW()
           )`
        );
      } catch (err) {
        log.debug('cynic_kv table creation skipped', { error: err.message });
      }

      // D5: Initialize SharedMemory from storage (loads patterns, weights, procedures)
      try {
        if (_sharedMemory?.initialize) {
          await _sharedMemory.initialize();
          log.debug('SharedMemory initialized from storage');
        }
      } catch (err) {
        log.warn('SharedMemory initialization failed, falling back to pattern repo', { error: err.message });
      }

      // Fallback: load from patterns repo if SharedMemory storage is empty
      try {
        await loadPersistedState(options.persistence);
      } catch (err) {
        log.warn('Could not load persisted collective state', { error: err.message });
      }

      // Initialize Q-Learning with persistence
      try {
        await initializeQLearning(options.persistence);
      } catch (err) {
        log.warn('Could not initialize Q-Learning', { error: err.message });
      }
    }

    return pack;
  })();

  return _initPromise;
}

/**
 * Awaken CYNIC for this session
 *
 * Should be called once per session (e.g., from SessionStart hook).
 * Safe to call multiple times - subsequent calls are no-ops.
 *
 * @param {Object} context - Session context
 * @param {string} context.sessionId - Session ID
 * @param {string} context.userId - User ID
 * @param {string} [context.project] - Project name
 * @returns {Promise<Object>} Awakening result with greeting
 */
export async function awakenCynic(context = {}) {
  if (_isAwakened) {
    log.debug('CYNIC already awakened this session');
    return { success: true, alreadyAwake: true };
  }

  const pack = getCollectivePack();

  if (!pack.awakenCynic) {
    log.warn('CollectivePack does not have awakenCynic method');
    return { success: false, error: 'awakenCynic not available' };
  }

  try {
    const result = await pack.awakenCynic({
      sessionId: context.sessionId || `session_${Date.now()}`,
      userId: context.userId || 'unknown',
      project: context.project || 'unknown',
    });

    _isAwakened = true;
    log.info('CYNIC awakened', { greeting: result.greeting });

    return result;
  } catch (err) {
    log.error('Failed to awaken CYNIC', { error: err.message });
    return { success: false, error: err.message };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Q-LEARNING SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Get the Q-Learning Service singleton
 *
 * Creates a QLearningService with persistence wired.
 * Call initializeQLearning() after to load state from DB.
 *
 * @param {Object} [options] - Options (only used on first call)
 * @param {Object} [options.persistence] - Persistence layer with query() method
 * @param {string} [options.serviceId] - Service identifier (default: 'collective')
 * @returns {Object} QLearningService instance
 */
export function getQLearningServiceSingleton(options = {}) {
  if (!_qLearningService) {
    _qLearningService = getQLearningService({
      persistence: options.persistence || null,
      serviceId: options.serviceId || 'collective',
    });

    log.debug('QLearningService singleton created', {
      hasPersistence: !!options.persistence,
      serviceId: options.serviceId || 'collective',
    });
  }

  return _qLearningService;
}

/**
 * Initialize Q-Learning from PostgreSQL
 *
 * Loads persisted Q-table, exploration rate, and stats.
 * Safe to call multiple times — only loads on first call.
 *
 * @param {Object} [persistence] - Persistence layer with query()
 * @returns {Promise<boolean>} True if loaded from DB
 */
export async function initializeQLearning(persistence) {
  const service = getQLearningServiceSingleton({ persistence });

  // Wire persistence if provided and not already set
  if (persistence && !service.persistence) {
    service.persistence = persistence;
  }

  try {
    const loaded = await service.initialize();
    if (loaded) {
      const stats = service.getStats();
      log.info('Q-Learning state loaded from DB', {
        states: stats.qTableStats?.states || 0,
        episodes: stats.episodes || 0,
      });
    }
    return loaded;
  } catch (err) {
    log.warn('Q-Learning initialization failed', { error: err.message });
    return false;
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// PERSISTENCE HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Load persisted state into SharedMemory
 *
 * Called during async initialization to restore:
 * - Patterns from PostgreSQL
 * - Dimension weights
 * - Fisher scores (EWC++)
 *
 * @param {Object} persistence - PersistenceManager instance
 */
async function loadPersistedState(persistence) {
  if (!_sharedMemory) return;

  const patternsRepo = persistence.getRepository?.('patterns');
  if (!patternsRepo) {
    log.debug('No patterns repository available');
    return;
  }

  try {
    // Load top patterns (by Fisher score)
    const patterns = await patternsRepo.findRecent?.({ limit: 100 });
    if (patterns?.length) {
      for (const p of patterns) {
        _sharedMemory.addPattern?.(p);
      }
      log.debug('Loaded persisted patterns', { count: patterns.length });
    }
  } catch (err) {
    log.warn('Could not load persisted patterns', { error: err.message });
  }
}

/**
 * Save current state to persistence
 *
 * Called periodically or on shutdown to persist:
 * - High-Fisher patterns
 * - Dimension weights
 *
 * @param {Object} persistence - PersistenceManager instance
 */
export async function saveState(persistence) {
  if (!_sharedMemory || !persistence) return;

  // D5: Use SharedMemory's own save() for full state (patterns, weights, procedures)
  try {
    if (_sharedMemory.save) {
      await _sharedMemory.save();
      log.debug('SharedMemory full state saved via storage adapter');
    }
  } catch (err) {
    log.debug('SharedMemory save() failed, falling back to pattern repo', { error: err.message });
  }

  // Fallback: also save high-Fisher patterns via patterns repo
  const patternsRepo = persistence.getRepository?.('patterns');
  if (!patternsRepo) return;

  try {
    const patterns = _sharedMemory.getLockedPatterns?.() || [];

    for (const pattern of patterns) {
      await patternsRepo.upsert?.(pattern);
    }

    log.debug('Saved locked patterns to repo', { patternsCount: patterns.length });
  } catch (err) {
    log.warn('Could not save collective state', { error: err.message });
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// STATUS & DIAGNOSTICS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Get singleton status for diagnostics
 *
 * @returns {Object} Status information
 */
export function getSingletonStatus() {
  return {
    packInitialized: !!_globalPack,
    sharedMemoryInitialized: !!_sharedMemory,
    qLearningInitialized: !!_qLearningService?._initialized,
    isAwakened: _isAwakened,
    sharedMemoryStats: _sharedMemory?.stats || null,
    packStats: _globalPack?.getStats?.() || null,
    qLearningStats: _qLearningService?.getStats?.() || null,
  };
}

/**
 * Check if the singleton is ready for use
 *
 * @returns {boolean} True if pack is initialized
 */
export function isReady() {
  return !!_globalPack;
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTING UTILITIES
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Reset singletons for testing
 *
 * WARNING: Only use in tests! Never call in production.
 * This breaks the singleton contract and can cause inconsistent state.
 */
export function _resetForTesting() {
  if (process.env.NODE_ENV === 'production') {
    throw new Error('Cannot reset singletons in production');
  }

  _globalPack = null;
  _sharedMemory = null;
  _qLearningService = null;
  _initPromise = null;
  _isAwakened = false;

  log.warn('Singletons reset (testing only)');
}

// ═══════════════════════════════════════════════════════════════════════════════
// EXPORTS
// ═══════════════════════════════════════════════════════════════════════════════

export default {
  getCollectivePack,
  getCollectivePackAsync,
  getSharedMemory,
  getQLearningServiceSingleton,
  initializeQLearning,
  awakenCynic,
  saveState,
  getSingletonStatus,
  isReady,
  _resetForTesting,
};
