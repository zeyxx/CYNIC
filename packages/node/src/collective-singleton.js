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

import { createLogger, PHI_INV, globalEventBus, EventType } from '@cynic/core';
import { createCollectivePack } from './agents/collective/index.js';
import { SharedMemory } from './memory/shared-memory.js';
import { getQLearningService } from './orchestration/learning-service.js';
import { createReasoningBank, TrajectoryType } from './learning/reasoning-bank.js';

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

  /**
   * FIX P3: Sync Fisher score to PostgreSQL patterns table
   * "φ persists" - EWC++ knowledge must survive restarts
   *
   * @param {string} patternId - Pattern ID
   * @param {number} fisherImportance - Fisher importance score (0-1)
   * @param {boolean} locked - Whether pattern should be consolidation locked
   */
  async syncFisherScore(patternId, fisherImportance, locked = false) {
    try {
      // Update patterns table if it exists
      await this._persistence.query?.(
        `UPDATE patterns SET
           fisher_importance = $2,
           consolidation_locked = $3,
           updated_at = NOW()
         WHERE id = $1 OR signature = $1`,
        [patternId, fisherImportance, locked]
      );
      log.trace(`Fisher synced: ${patternId} → ${fisherImportance}${locked ? ' (locked)' : ''}`);
    } catch (e) {
      // Non-fatal - pattern might not exist in patterns table yet
      log.trace(`Fisher sync failed: ${patternId} - ${e.message}`);
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
 * FIX P5: The global ReasoningBank instance - null until first access
 * "Le chien se souvient des chemins réussis"
 * @type {ReasoningBank|null}
 */
let _reasoningBank = null;

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
 * FIX P5: Get the ReasoningBank singleton
 *
 * ReasoningBank stores successful reasoning trajectories for replay.
 * "Le chien se souvient des chemins réussis"
 *
 * @param {Object} [options] - Options for ReasoningBank (only used on first call)
 * @param {Object} [options.persistence] - Persistence layer for trajectory storage
 * @returns {ReasoningBank} The singleton ReasoningBank instance
 */
export function getReasoningBank(options = {}) {
  if (!_reasoningBank) {
    _reasoningBank = createReasoningBank({
      persistence: options.persistence || null,
    });
    log.debug('ReasoningBank singleton created', { hasPersistence: !!options.persistence });
  }
  return _reasoningBank;
}

/**
 * Get the CollectivePack singleton (SYNC version)
 *
 * ⚠️ WARNING: This is the SYNC version. Persistence state may NOT be loaded.
 * Prefer getCollectivePackAsync() for most use cases.
 *
 * Use this only when:
 * - You're inside getCollectivePackAsync() (internal)
 * - You know persistence is already loaded
 * - You don't need persisted state
 *
 * @param {Object} [options] - Options (only used on first call)
 * @param {Object} [options.judge] - CYNICJudge instance
 * @param {Object} [options.persistence] - PersistenceManager instance
 * @param {Object} [options.sharedMemory] - SharedMemory instance (default: singleton)
 * @param {Object} [options.eventBus] - EventBus instance
 * @param {number} [options.consensusThreshold] - Consensus threshold (default: φ⁻¹)
 * @returns {CollectivePack} The singleton CollectivePack instance
 * @see getCollectivePackAsync - Preferred async version with persistence
 */
export function getCollectivePack(options = {}) {
  if (!_globalPack) {
    // Ensure SharedMemory exists with persistence wired
    // FIX: Pass options so PersistenceStorageAdapter is created
    const sharedMemory = options.sharedMemory || getSharedMemory(options);

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

    // FIX O3: Schedule background persistence initialization
    // "φ persiste" - persistence should be loaded even for sync calls
    if (finalOptions.persistence && !_initPromise) {
      log.debug('Scheduling background persistence initialization (sync path)');
      _initPromise = _initializeBackground(finalOptions.persistence).catch(err => {
        log.warn('Background persistence initialization failed', { error: err.message });
      });
    }
  }

  return _globalPack;
}

/**
 * FIX P5: Wire JUDGMENT_CREATED events to ReasoningBank
 * Converts judgments to trajectories for future replay
 * @private
 * @param {ReasoningBank} bank - The ReasoningBank singleton
 */
function _wireJudgmentToReasoningBank(bank) {
  // Only wire once
  if (bank._eventWired) return;
  bank._eventWired = true;

  globalEventBus.on(EventType.JUDGMENT_CREATED, async (event) => {
    try {
      const { id, payload } = event;
      const { qScore, verdict, dimensions, itemType, confidence } = payload;

      // Create a trajectory from the judgment
      const trajectory = bank.startTrajectory(TrajectoryType.JUDGMENT, {
        itemType,
        qScore,
        verdict,
      });

      // Add the judgment as an action
      trajectory.addAction({
        type: 'judgment',
        tool: 'CYNICJudge',
        confidence,
        input: { itemType, dimensions },
      });

      // Set outcome based on verdict
      const isSuccess = qScore >= 50; // WAG or HOWL
      trajectory.setOutcome({
        type: isSuccess ? 'success' : 'partial',
        success: isSuccess,
        metrics: { qScore, confidence },
      });

      // Store the trajectory (async, fire-and-forget)
      await bank.store(trajectory);
    } catch (err) {
      log.debug('Failed to store judgment trajectory', { error: err.message });
    }
  });

  log.debug('JUDGMENT_CREATED wired to ReasoningBank');
}

/**
 * Initialize persistence in background (called from sync getCollectivePack)
 * @private
 */
async function _initializeBackground(persistence) {
  // Create cynic_kv table if needed
  try {
    await persistence.query?.(
      `CREATE TABLE IF NOT EXISTS cynic_kv (
         key TEXT PRIMARY KEY,
         data JSONB NOT NULL DEFAULT '{}',
         updated_at TIMESTAMPTZ DEFAULT NOW()
       )`
    );
  } catch (err) {
    log.debug('cynic_kv table creation skipped', { error: err.message });
  }

  // Initialize SharedMemory from storage
  try {
    if (_sharedMemory?.initialize) {
      await _sharedMemory.initialize();
      log.debug('SharedMemory initialized from storage (background)');
    }
  } catch (err) {
    log.debug('SharedMemory background init failed', { error: err.message });
  }

  // Load patterns from PostgreSQL if SharedMemory is empty
  try {
    await loadPersistedState(persistence);
  } catch (err) {
    log.debug('Background pattern load failed', { error: err.message });
  }

  // Initialize Q-Learning
  try {
    await initializeQLearning(persistence);
  } catch (err) {
    log.debug('Background Q-Learning init failed', { error: err.message });
  }

  // FIX P5: Initialize ReasoningBank and load trajectories
  try {
    // Create reasoning_trajectories table if needed
    await persistence.query?.(
      `CREATE TABLE IF NOT EXISTS reasoning_trajectories (
         id TEXT PRIMARY KEY,
         type TEXT NOT NULL,
         data JSONB NOT NULL,
         reward REAL DEFAULT 0,
         confidence REAL DEFAULT 0.5,
         user_id TEXT,
         project_id TEXT,
         created_at TIMESTAMPTZ DEFAULT NOW(),
         updated_at TIMESTAMPTZ DEFAULT NOW()
       )`
    );
    const bank = getReasoningBank({ persistence });
    const loaded = await bank.load({ limit: 100 }); // Load top 100 trajectories
    log.debug('ReasoningBank initialized', { trajectoriesLoaded: loaded });

    // FIX P5: Wire JUDGMENT_CREATED → ReasoningBank for trajectory capture
    // "Le chien se souvient des chemins réussis"
    _wireJudgmentToReasoningBank(bank);
  } catch (err) {
    log.debug('Background ReasoningBank init failed', { error: err.message });
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

  // Use async version to ensure persistence is loaded
  const pack = await getCollectivePackAsync(context);

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
 * - Patterns from PostgreSQL (up to MAX_PATTERNS = 1597)
 * - Transform PostgreSQL format → SharedMemory format
 * - Prioritize by confidence × frequency (φ-weighted importance)
 *
 * @param {Object} persistence - PersistenceManager instance
 */
async function loadPersistedState(persistence) {
  if (!_sharedMemory) return;

  // Skip if SharedMemory already has patterns (loaded from cynic_kv)
  if (_sharedMemory._patterns?.size > 0) {
    log.debug('SharedMemory already has patterns, skipping PostgreSQL load', {
      count: _sharedMemory._patterns.size,
    });
    return;
  }

  const patternsRepo = persistence.getRepository?.('patterns');
  if (!patternsRepo) {
    log.debug('No patterns repository available');
    return;
  }

  try {
    // FIX: Use list() instead of non-existent findRecent()
    // Load top 1597 patterns (F17 = SharedMemory max) ordered by importance
    // Note: list() returns patterns ordered by confidence DESC, frequency DESC
    const patterns = await patternsRepo.list?.({ limit: 1597, offset: 0 });
    if (!patterns?.length) {
      log.debug('No patterns found in PostgreSQL');
      return;
    }

    // Transform PostgreSQL format → SharedMemory format and add
    let added = 0;
    for (const pgPattern of patterns) {
      const smPattern = transformPgPatternToSharedMemory(pgPattern);
      _sharedMemory.addPattern?.(smPattern);
      added++;
    }

    log.info('Loaded patterns from PostgreSQL → SharedMemory', {
      loaded: added,
      total: patterns.length,
    });
  } catch (err) {
    log.warn('Could not load persisted patterns', { error: err.message });
  }
}

/**
 * Transform PostgreSQL pattern format to SharedMemory format
 *
 * PostgreSQL: { pattern_id, category, name, description, confidence, frequency, tags, data, ... }
 * SharedMemory: { id, tags, applicableTo, description, weight, verified, fisherImportance, ... }
 *
 * @param {Object} pgPattern - Pattern from PostgreSQL
 * @returns {Object} Pattern in SharedMemory format
 */
function transformPgPatternToSharedMemory(pgPattern) {
  // Calculate φ-weighted importance from confidence × frequency
  // Higher confidence and frequency = higher weight and Fisher importance
  const confidence = parseFloat(pgPattern.confidence) || 0.5;
  const frequency = parseInt(pgPattern.frequency) || 1;

  // Weight: Scale by frequency (log to prevent runaway values)
  // Range: 0.5 to 2.618 (φ + 1)
  const weight = Math.min(2.618, 0.5 + Math.log10(frequency + 1) * 0.5);

  // Fisher importance: confidence acts as initial importance
  // High-confidence patterns are more resistant to forgetting
  const fisherImportance = Math.min(1.0, confidence * 1.2);

  // Verified: patterns with high confidence and frequency are considered verified
  const verified = confidence >= PHI_INV && frequency >= 3;

  // applicableTo: derive from category
  const applicableTo = pgPattern.category
    ? [pgPattern.category, '*']
    : ['*'];

  return {
    id: pgPattern.pattern_id,
    name: pgPattern.name,
    description: pgPattern.description || '',
    tags: pgPattern.tags || [],
    applicableTo,
    category: pgPattern.category,
    // φ-aligned weights
    weight,
    fisherImportance,
    verified,
    consolidationLocked: fisherImportance >= PHI_INV, // Lock high-importance patterns
    // Timestamps
    addedAt: pgPattern.created_at ? new Date(pgPattern.created_at).getTime() : Date.now(),
    lastUsed: pgPattern.updated_at ? new Date(pgPattern.updated_at).getTime() : null,
    // Usage stats
    useCount: frequency,
    // Original data preserved
    data: pgPattern.data || {},
    sourceJudgments: pgPattern.source_judgments || [],
    sourceCount: pgPattern.source_count || 0,
  };
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
  if (!persistence) return;

  let savedComponents = [];

  // ═══════════════════════════════════════════════════════════════════════════
  // 1. Save SharedMemory state (patterns, weights, procedures)
  // ═══════════════════════════════════════════════════════════════════════════
  if (_sharedMemory) {
    try {
      if (_sharedMemory.save) {
        await _sharedMemory.save();
        savedComponents.push('SharedMemory');
        log.debug('SharedMemory full state saved via storage adapter');
      }
    } catch (err) {
      log.debug('SharedMemory save() failed, falling back to pattern repo', { error: err.message });
    }

    // Fallback: also save high-Fisher patterns via patterns repo
    const patternsRepo = persistence.getRepository?.('patterns');
    if (patternsRepo) {
      try {
        const patterns = _sharedMemory.getLockedPatterns?.() || [];
        for (const pattern of patterns) {
          await patternsRepo.upsert?.(pattern);
        }
        if (patterns.length > 0) savedComponents.push(`Patterns(${patterns.length})`);
        log.debug('Saved locked patterns to repo', { patternsCount: patterns.length });
      } catch (err) {
        log.warn('Could not save patterns', { error: err.message });
      }
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // 2. FIX: Save Q-Learning state (routing weights, exploration stats)
  // "Le chien apprend" - Q-Learning state must survive sessions
  // ═══════════════════════════════════════════════════════════════════════════
  if (_qLearningService) {
    try {
      // Export Q-Learning state
      const qState = _qLearningService.export?.();
      if (qState && persistence.saveLearningState) {
        await persistence.saveLearningState(qState);
        savedComponents.push('Q-Learning');
        log.debug('Q-Learning state saved', { states: Object.keys(qState.qTable || {}).length });
      } else if (qState && persistence.getRepository?.('learning')) {
        // Fallback: use learning repository if available
        const learningRepo = persistence.getRepository('learning');
        await learningRepo.upsert?.({
          type: 'q_learning',
          state: qState,
          timestamp: Date.now(),
        });
        savedComponents.push('Q-Learning(repo)');
      }
    } catch (err) {
      log.warn('Could not save Q-Learning state', { error: err.message });
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // 3. Save CollectivePack consensus state (dog voting history)
  // ═══════════════════════════════════════════════════════════════════════════
  if (_globalPack) {
    try {
      const packStats = _globalPack.getStats?.();
      if (packStats && persistence.storeObservation) {
        await persistence.storeObservation({
          type: 'collective_state',
          content: JSON.stringify(packStats),
          confidence: 0.618,
          context: { component: 'CollectivePack', timestamp: Date.now() },
        });
        savedComponents.push('CollectivePack');
      }
    } catch (err) {
      log.debug('Could not save CollectivePack state', { error: err.message });
    }
  }

  log.info('Collective state saved', { components: savedComponents.join(', ') || 'none' });
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
  _reasoningBank = null; // FIX P5: Reset ReasoningBank
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
  getReasoningBank, // FIX P5: Export ReasoningBank singleton
  getQLearningServiceSingleton,
  initializeQLearning,
  awakenCynic,
  saveState,
  getSingletonStatus,
  isReady,
  _resetForTesting,
};
