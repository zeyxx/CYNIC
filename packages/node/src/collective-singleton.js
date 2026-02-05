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
import { getDogStateEmitter } from './perception/dog-state-emitter.js';
import { getLearningScheduler } from './judge/learning-scheduler.js';
import { getUnifiedBridge } from './learning/unified-bridge.js';
import { getHumanAdvisor } from './symbiosis/human-advisor.js';
import { getHumanLearning } from './symbiosis/human-learning.js';
import { getHumanAccountant } from './symbiosis/human-accountant.js';
import { getHumanEmergence } from './symbiosis/human-emergence.js';
import { wireAmbientConsensus } from './agents/collective/ambient-consensus.js';
import { startEventListeners, stopEventListeners } from './services/event-listeners.js';

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
 * C6.1: The global DogStateEmitter instance - null until first access
 * "Le chien se voit lui-même"
 * @type {DogStateEmitter|null}
 */
let _dogStateEmitter = null;

/**
 * C1.5 + C6.7: The global LearningScheduler instance
 * Runs DPO training (C1.5) and ResidualGovernance (C6.7) daily
 * "φ learns while you sleep"
 * @type {LearningScheduler|null}
 */
let _learningScheduler = null;

/**
 * The global UnifiedBridge instance
 * Connects Judge events to UnifiedSignal for unified learning pipeline
 * "Tous les chemins mènent à l'apprentissage"
 * @type {UnifiedBridge|null}
 */
let _unifiedBridge = null;

/**
 * C5.3: The global HumanAdvisor instance
 * Proactive care for human wellbeing
 * "Le chien protège son humain"
 * @type {HumanAdvisor|null}
 */
let _humanAdvisor = null;

/**
 * C5.5: The global HumanLearning instance
 * Tracks human skill acquisition
 * "L'humain grandit, le chien observe"
 * @type {HumanLearning|null}
 */
let _humanLearning = null;

/**
 * C5.6: The global HumanAccountant instance
 * Tracks human activity and energy
 * "Le temps de l'humain est précieux"
 * @type {HumanAccountant|null}
 */
let _humanAccountant = null;

/**
 * C5.7: The global HumanEmergence instance
 * Detects human growth patterns
 * "L'humain émerge, CYNIC témoigne"
 * @type {HumanEmergence|null}
 */
let _humanEmergence = null;

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

/**
 * AXE 2 (PERSIST): The global EventListeners controller
 * Closes data loops by persisting judgments, feedback, and session state
 * "Le chien n'oublie jamais" - CYNIC persists everything
 * @type {Object|null}
 */
let _eventListeners = null;

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
 * C6.1: Get the DogStateEmitter singleton
 *
 * DogStateEmitter provides real-time visibility into Dog states.
 * "Le chien se voit lui-même" - CYNIC perceives its own collective
 *
 * @param {Object} [options] - Options for DogStateEmitter
 * @param {Object} [options.collectivePack] - CollectivePack reference
 * @param {Object} [options.sharedMemory] - SharedMemory reference
 * @param {boolean} [options.autoStart] - Start periodic emission (default: true when pack available)
 * @returns {DogStateEmitter} The singleton DogStateEmitter instance
 */
export function getDogStateEmitterSingleton(options = {}) {
  if (!_dogStateEmitter) {
    _dogStateEmitter = getDogStateEmitter({
      collectivePack: options.collectivePack || _globalPack,
      sharedMemory: options.sharedMemory || _sharedMemory,
      autoStart: false, // Don't auto-start yet - wait for pack
    });
    log.debug('DogStateEmitter singleton created');
  } else {
    // Update references if provided
    if (options.collectivePack) {
      _dogStateEmitter.setCollectivePack(options.collectivePack);
    }
    if (options.sharedMemory) {
      _dogStateEmitter.setSharedMemory(options.sharedMemory);
    }
  }
  return _dogStateEmitter;
}

/**
 * C1.5 + C6.7: Get the LearningScheduler singleton
 *
 * LearningScheduler runs DPO optimization (C1.5) and ResidualGovernance (C6.7) on schedule.
 * "φ learns while you sleep"
 *
 * Note: Scheduler is started automatically when getCollectivePackAsync completes.
 *
 * @returns {LearningScheduler} The singleton LearningScheduler instance
 */
export function getLearningSchedulerSingleton() {
  if (!_learningScheduler) {
    _learningScheduler = getLearningScheduler();
  }
  return _learningScheduler;
}

/**
 * Get the UnifiedBridge singleton
 *
 * UnifiedBridge connects JUDGMENT_CREATED events to UnifiedSignalStore.
 * This enables unified learning from all judgment outputs.
 * "Tous les chemins mènent à l'apprentissage" - κυνικός
 *
 * @returns {UnifiedBridge} The singleton UnifiedBridge instance
 */
export function getUnifiedBridgeSingleton() {
  if (!_unifiedBridge) {
    _unifiedBridge = getUnifiedBridge();
  }
  return _unifiedBridge;
}

// ═══════════════════════════════════════════════════════════════════════════════
// SYMBIOSIS SINGLETONS (C5.* - Human Layer)
// "Le chien amplifie l'humain, l'humain guide le chien"
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * C5.3: Get the HumanAdvisor singleton
 *
 * HumanAdvisor provides proactive care for human wellbeing.
 * "Le chien protège son humain" - CYNIC cares for its human
 *
 * @param {Object} [options] - Options for HumanAdvisor
 * @param {Object} [options.psychology] - Psychology tracker reference
 * @returns {HumanAdvisor} The singleton HumanAdvisor instance
 */
export function getHumanAdvisorSingleton(options = {}) {
  if (!_humanAdvisor) {
    _humanAdvisor = getHumanAdvisor(options);
    log.debug('HumanAdvisor singleton created (C5.3)');
  }
  return _humanAdvisor;
}

/**
 * C5.5: Get the HumanLearning singleton
 *
 * HumanLearning tracks skill acquisition and growth over time.
 * "L'humain grandit, le chien observe"
 *
 * @param {Object} [options] - Options for HumanLearning
 * @returns {HumanLearning} The singleton HumanLearning instance
 */
export function getHumanLearningSingleton(options = {}) {
  if (!_humanLearning) {
    _humanLearning = getHumanLearning(options);
    log.debug('HumanLearning singleton created (C5.5)');
  }
  return _humanLearning;
}

/**
 * C5.6: Get the HumanAccountant singleton
 *
 * HumanAccountant tracks human activity, energy, and time investment.
 * "Le temps de l'humain est précieux"
 *
 * @param {Object} [options] - Options for HumanAccountant
 * @returns {HumanAccountant} The singleton HumanAccountant instance
 */
export function getHumanAccountantSingleton(options = {}) {
  if (!_humanAccountant) {
    _humanAccountant = getHumanAccountant(options);
    log.debug('HumanAccountant singleton created (C5.6)');
  }
  return _humanAccountant;
}

/**
 * C5.7: Get the HumanEmergence singleton
 *
 * HumanEmergence detects patterns in human growth and development.
 * "L'humain émerge, CYNIC témoigne"
 *
 * @param {Object} [options] - Options for HumanEmergence
 * @returns {HumanEmergence} The singleton HumanEmergence instance
 */
export function getHumanEmergenceSingleton(options = {}) {
  if (!_humanEmergence) {
    _humanEmergence = getHumanEmergence(options);
    log.debug('HumanEmergence singleton created (C5.7)');
  }
  return _humanEmergence;
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

    // C6.1: Wire DogStateEmitter to the pack for real-time self-perception
    const emitter = getDogStateEmitterSingleton({
      collectivePack: _globalPack,
      sharedMemory,
    });
    // Start periodic emission now that pack is available
    emitter.start();
    log.debug('DogStateEmitter wired and started');

    // C1.5 + C6.7: Wire LearningScheduler (DPO + Governance)
    // Will be started in background by getCollectivePackAsync
    _learningScheduler = getLearningScheduler();
    log.debug('LearningScheduler singleton created (not started yet)');

    // Wire UnifiedBridge for unified learning pipeline
    // Connects JUDGMENT_CREATED events to UnifiedSignalStore
    _unifiedBridge = getUnifiedBridge();
    _unifiedBridge.start();

    // AXE 3: Wire Ambient Consensus - Dogs vote automatically
    // "Le pack décide ensemble" - triggers consensus on low confidence / danger
    const ambientConsensus = wireAmbientConsensus(_globalPack);
    _globalPack.ambientConsensus = ambientConsensus;
    log.info('AmbientConsensus wired - Dogs vote automatically');
    log.debug('UnifiedBridge wired and started');

    // C5.*: Wire Symbiosis Layer (Human × CYNIC)
    // "Le chien amplifie l'humain, l'humain guide le chien"
    _humanAdvisor = getHumanAdvisor();
    _humanLearning = getHumanLearning();
    _humanAccountant = getHumanAccountant();
    _humanEmergence = getHumanEmergence();
    log.debug('Symbiosis layer wired (C5.3-C5.7)');

    // AXE 2 (PERSIST): Wire Event Listeners to close data loops
    // "Le chien n'oublie jamais" - persists judgments, feedback, session state
    if (finalOptions.persistence && !_eventListeners) {
      _eventListeners = startEventListeners({
        persistence: finalOptions.persistence,
        sharedMemory,
        saveState,
        sessionId: finalOptions.sessionId,
        userId: finalOptions.userId,
      });
      log.info('EventListeners started - data loops closed (AXE 2)');
    }

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

      // C1.5 + C6.7: Start LearningScheduler (DPO + Governance)
      // Wire dependencies and start scheduler
      if (_learningScheduler) {
        try {
          // Try to get dependencies from judge package
          const { DPOOptimizer, CalibrationTracker, ResidualGovernance, LearningManager } = await import('./judge/index.js');

          _learningScheduler.setDependencies({
            learningManager: pack.learner?.learningManager || null,
            dpoOptimizer: new DPOOptimizer({ pool: options.persistence }),
            calibrationTracker: new CalibrationTracker({ pool: options.persistence }),
            residualGovernance: new ResidualGovernance({
              pool: options.persistence,
              collectivePack: pack,
            }),
          });

          _learningScheduler.start();
          log.info('LearningScheduler started with dependencies');
        } catch (err) {
          log.warn('Could not start LearningScheduler', { error: err.message });
        }
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
 * Auto-save interval handle
 * @type {NodeJS.Timeout|null}
 */
let _autoSaveInterval = null;

/**
 * Last auto-save timestamp
 * @type {number}
 */
let _lastAutoSave = 0;

/**
 * Auto-save configuration
 */
const AUTO_SAVE_CONFIG = {
  INTERVAL_MS: 10 * 60 * 1000, // 10 minutes
  MIN_PATTERNS_CHANGED: 5,      // Minimum pattern changes before auto-save
};

/**
 * Start auto-save interval for patterns
 * FIX W1.1: Ensures patterns are saved periodically
 *
 * @param {Object} persistence - PersistenceManager instance
 */
export function startAutoSave(persistence) {
  if (_autoSaveInterval) {
    log.debug('Auto-save already running');
    return;
  }

  _autoSaveInterval = setInterval(async () => {
    try {
      const now = Date.now();

      // Skip if not enough time has passed (failsafe)
      if (now - _lastAutoSave < AUTO_SAVE_CONFIG.INTERVAL_MS * 0.9) {
        return;
      }

      // Save with onlyModified=true to reduce DB load
      await saveState(persistence, { onlyModified: true });
      _lastAutoSave = now;

      log.debug('Auto-save completed');
    } catch (err) {
      log.warn('Auto-save failed', { error: err.message });
    }
  }, AUTO_SAVE_CONFIG.INTERVAL_MS);

  log.info('Auto-save started', { intervalMs: AUTO_SAVE_CONFIG.INTERVAL_MS });
}

/**
 * Stop auto-save interval
 */
export function stopAutoSave() {
  if (_autoSaveInterval) {
    clearInterval(_autoSaveInterval);
    _autoSaveInterval = null;
    log.debug('Auto-save stopped');
  }
}

/**
 * Load persisted state into SharedMemory
 *
 * FIX W1.1 + W1.5: Uses new loadFromPostgres method
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

  // FIX W1.5: Use new loadFromPostgres method if available
  if (_sharedMemory.loadFromPostgres) {
    try {
      const result = await _sharedMemory.loadFromPostgres(patternsRepo, {
        limit: 500,        // Load top 500 patterns (reduced for faster startup)
        minConfidence: 0.1,
      });
      log.info('Patterns loaded via loadFromPostgres', result);

      // Start auto-save after loading
      startAutoSave(persistence);
      return;
    } catch (err) {
      log.warn('loadFromPostgres failed, falling back to legacy load', { error: err.message });
    }
  }

  // Fallback: Legacy loading method
  try {
    const patterns = await patternsRepo.list?.({ limit: 1597, offset: 0 });
    if (!patterns?.length) {
      log.debug('No patterns found in PostgreSQL');
      return;
    }

    let added = 0;
    for (const pgPattern of patterns) {
      const smPattern = transformPgPatternToSharedMemory(pgPattern);
      _sharedMemory.addPattern?.(smPattern);
      added++;
    }

    log.info('Loaded patterns from PostgreSQL → SharedMemory (legacy)', {
      loaded: added,
      total: patterns.length,
    });

    // Start auto-save after loading
    startAutoSave(persistence);
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
 * - ALL patterns to PostgreSQL (batch save)
 * - High-Fisher patterns (EWC++ locked)
 * - Dimension weights
 *
 * FIX W1.1: Closes BLACK HOLE #1 - Patterns no longer lost on restart
 *
 * @param {Object} persistence - PersistenceManager instance
 * @param {Object} [options={}] - Save options
 * @param {boolean} [options.onlyModified=false] - Only save modified patterns
 */
export async function saveState(persistence, options = {}) {
  if (!persistence) return;

  let savedComponents = [];

  // ═══════════════════════════════════════════════════════════════════════════
  // 1. Save SharedMemory state (patterns, weights, procedures)
  // FIX W1.1: Use batch saveToPostgres for all patterns
  // ═══════════════════════════════════════════════════════════════════════════
  if (_sharedMemory) {
    // First, try the new batch persistence to PostgreSQL
    const patternsRepo = persistence.getRepository?.('patterns');
    if (patternsRepo && _sharedMemory.saveToPostgres) {
      try {
        const result = await _sharedMemory.saveToPostgres(patternsRepo, {
          onlyModified: options.onlyModified ?? false,
        });
        if (result.saved > 0) {
          savedComponents.push(`Patterns(${result.saved}/${result.total})`);
        }
        if (result.errors > 0) {
          log.warn('Some patterns failed to save', { errors: result.errors });
        }
        log.debug('Batch pattern save complete', result);
      } catch (err) {
        log.warn('Batch pattern save failed', { error: err.message });
      }
    }

    // Also save full state via storage adapter (includes weights, procedures, feedback)
    try {
      if (_sharedMemory.save) {
        await _sharedMemory.save();
        savedComponents.push('SharedMemory');
        log.debug('SharedMemory full state saved via storage adapter');
      }
    } catch (err) {
      log.debug('SharedMemory save() failed', { error: err.message });
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // 2. FIX: Save Q-Learning state (routing weights, exploration stats)
  // "Le chien apprend" - Q-Learning state must survive sessions
  // ═══════════════════════════════════════════════════════════════════════════
  if (_qLearningService) {
    try {
      // Flush Q-Learning state to PostgreSQL immediately
      // The service already handles persistence via _doPersist() to qlearning_state table
      await _qLearningService.flush();
      savedComponents.push('Q-Learning');
      log.debug('Q-Learning state flushed', { stats: _qLearningService.getStats?.() });
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

  // AXE 2: Stop event listeners
  if (_eventListeners) {
    stopEventListeners();
    _eventListeners = null;
  }

  log.warn('Singletons reset (testing only)');
}

// ═══════════════════════════════════════════════════════════════════════════════
// EXPORTS
// ═══════════════════════════════════════════════════════════════════════════════

// AXE 2: Re-export event listener functions for external use
export { startEventListeners, stopEventListeners } from './services/event-listeners.js';
export { getListenerStats } from './services/event-listeners.js';

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
  // AXE 2: Event listeners
  startEventListeners,
  stopEventListeners,
};
