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
import { startEventListeners, stopEventListeners, cleanupOldEventData } from './services/event-listeners.js';
import { getNetworkNode as getNetworkNodeSync, getNetworkNodeAsync, startNetworkNode, stopNetworkNode, isP2PEnabled } from './network-singleton.js';
import { BlockStore } from './network/block-store.js';
import { getErrorHandler } from './services/error-handler.js';
import { getSolanaWatcher, resetSolanaWatcher } from './perception/solana-watcher.js';
import { getEmergenceDetector } from './services/emergence-detector.js';
import { createConsciousnessMonitor } from '@cynic/emergence';
import { getHeartbeatService, createDefaultChecks } from './services/heartbeat-service.js';
import { getSLATracker } from './services/sla-tracker.js';
import { wireConsciousness } from './services/consciousness-bridge.js';

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

/**
 * PHASE 2 (DECENTRALIZE): The global NetworkNode instance
 * P2P networking, consensus, block production, Solana anchoring
 * "The pack hunts together" - κυνικός
 * @type {CYNICNetworkNode|null}
 */
let _networkNode = null;

/**
 * C2.1 (SOLANA x PERCEIVE): SolanaWatcher singleton
 * Watches Solana blockchain events when SOLANA_CLUSTER or SOLANA_RPC_URL is set
 * @type {import('./perception/solana-watcher.js').SolanaWatcher|null}
 */
let _solanaWatcher = null;

/**
 * AXE 6 (EMERGE): Global EmergenceDetector instance
 * Detects cross-session patterns from data graves (hourly analysis)
 * @type {import('./services/emergence-detector.js').EmergenceDetector|null}
 */
let _emergenceDetector = null;

/**
 * ConsciousnessBridge singleton
 * Connects HeartbeatService + SLATracker → ConsciousnessMonitor → ErrorHandler
 * AXE 9: System health → consciousness awareness
 * @type {import('./services/consciousness-bridge.js').ConsciousnessBridge|null}
 */
let _consciousnessBridge = null;

/**
 * HeartbeatService singleton
 * Continuous health monitoring for all system components
 * @type {import('./services/heartbeat-service.js').HeartbeatService|null}
 */
let _heartbeatService = null;

/**
 * EWC++ Consolidation Service singleton
 * Prevents catastrophic forgetting by locking high-Fisher patterns
 * Runs daily consolidation: RETRIEVE→JUDGE→DISTILL→CONSOLIDATE
 * @type {import('@cynic/persistence').EWCConsolidationService|null}
 */
let _ewcService = null;

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
      // PHASE 2: Create BlockStore with pool for block anchoring persistence
      const blockStore = finalOptions.persistence.pool
        ? new BlockStore({ pool: finalOptions.persistence.pool })
        : (finalOptions.persistence.query ? new BlockStore({ pool: finalOptions.persistence }) : null);

      _eventListeners = startEventListeners({
        persistence: finalOptions.persistence,
        sharedMemory,
        saveState,
        judge: finalOptions.judge || _globalPack?.judge,
        collectivePack: _globalPack,
        sessionId: finalOptions.sessionId,
        userId: finalOptions.userId,
        blockStore,
      });
      log.info('EventListeners started - data loops closed (AXE 2)', { hasBlockStore: !!blockStore, hasJudge: !!(finalOptions.judge || _globalPack?.judge) });

      // Wire BlockStore to anchoring manager for retry sweeps
      if (blockStore && isP2PEnabled()) {
        try {
          const node = getNetworkNodeSync();
          if (node) {
            node.wireAnchoringStore(blockStore);
            log.info('BlockStore wired to anchoring manager for retry sweeps');
          }
        } catch (err) {
          log.debug('Could not wire BlockStore to anchoring manager', { error: err.message });
        }
      }
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
  // If already initialized but persistence provided and event listeners not started,
  // start them now (fixes the case where constructor called without persistence first)
  if (_globalPack) {
    if (options.persistence && !_eventListeners) {
      const blockStore = options.persistence.pool
        ? new BlockStore({ pool: options.persistence.pool })
        : (options.persistence.query ? new BlockStore({ pool: options.persistence }) : null);

      _eventListeners = startEventListeners({
        persistence: options.persistence,
        sharedMemory: _sharedMemory,
        saveState,
        judge: _globalPack?.judge,
        collectivePack: _globalPack,
        sessionId: options.sessionId,
        userId: options.userId,
        blockStore,
      });
      log.info('EventListeners started on subsequent call with persistence (AXE 2 fix)');

      // Wire BlockStore to anchoring manager for retry sweeps
      if (blockStore && isP2PEnabled()) {
        try {
          const node = getNetworkNodeSync();
          if (node) {
            node.wireAnchoringStore(blockStore);
            log.info('BlockStore wired to anchoring manager for retry sweeps');
          }
        } catch (err) {
          log.debug('Could not wire BlockStore to anchoring manager', { error: err.message });
        }
      }
    }
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

      // L-GAP-3: Wire PostgreSQL pool to UnifiedSignalStore
      // The store is created in sync init (getCollectivePack) without a pool.
      // Now that persistence is available, wire it so signals persist to unified_signals table.
      if (_unifiedBridge?.store && !_unifiedBridge.store._persistencePool) {
        _unifiedBridge.store._persistencePool = options.persistence;
        log.info('UnifiedSignalStore wired to PostgreSQL (L-GAP-3 closed)');
      }

      // Restore Dog track records from consensus_votes (AXE 2: close persistence loop)
      try {
        if (pack.ambientConsensus?.restoreFromDatabase) {
          const result = await pack.ambientConsensus.restoreFromDatabase(options.persistence);
          if (result.restored) {
            log.info('Dog track records restored', { dogs: result.dogs, votes: result.votes });
          }
        }
      } catch (err) {
        log.warn('Dog state restoration failed (non-blocking)', { error: err.message });
      }

      // THE_UNNAMEABLE: Load discovered dimensions from DB and register in DimensionRegistry
      // This is the gate that makes CYNIC remember what it learned to see
      let _residualStorage = null;
      try {
        const { createResidualStorage } = await import('@cynic/persistence');
        const { globalDimensionRegistry } = await import('./judge/dimension-registry.js');

        _residualStorage = createResidualStorage({ pool: options.persistence });

        // Load previously discovered dimensions
        const discovered = await _residualStorage.loadDiscoveredDimensions();
        for (const dim of discovered) {
          globalDimensionRegistry.register(dim.axiom, dim.name, {
            weight: dim.weight,
            threshold: dim.threshold,
            description: dim.description || `Discovered: ${dim.name}`,
          });
        }

        if (discovered.length > 0) {
          log.info('THE_UNNAMEABLE: Loaded discovered dimensions', { count: discovered.length });
        }

        // Wire storage to ResidualDetector in JudgeComponent
        if (pack.judge?.residualDetector) {
          pack.judge.residualDetector.storage = _residualStorage;
          await pack.judge.residualDetector.initialize();
          log.debug('ResidualDetector wired with PostgreSQL storage');
        }
      } catch (err) {
        log.warn('Could not initialize THE_UNNAMEABLE persistence', { error: err.message });
      }

      // C1.5 + C6.7: Start LearningScheduler (DPO + Governance)
      // Wire dependencies and start scheduler
      if (_learningScheduler) {
        try {
          // Try to get dependencies from judge package
          const { DPOOptimizer, CalibrationTracker, ResidualGovernance, LearningManager } = await import('./judge/index.js');

          const residualGovernance = new ResidualGovernance({
            pool: options.persistence,
            collectivePack: pack,
            residualStorage: _residualStorage,
          });

          // FIX: Create LearningManager instead of reading from pack.learner (was always null)
          // LearningManager auto-initializes DPOProcessor, DPOOptimizer, CalibrationTracker
          // L-GAP-FIX: Must include .feedback repo or pullFeedback() silently returns early
          let learningPersistence = null;
          if (options.persistence) {
            const { FeedbackRepository } = await import('@cynic/persistence');
            learningPersistence = {
              query: (sql, params) => options.persistence.query(sql, params),
              feedback: new FeedbackRepository(options.persistence),
            };
          } else {
            // File-backed fallback: learning survives without PostgreSQL
            const { createFileBackedRepo } = await import('@cynic/persistence');
            learningPersistence = {
              feedback: createFileBackedRepo('feedback'),
              patterns: createFileBackedRepo('patterns'),
              knowledge: createFileBackedRepo('knowledge'),
              patternEvolution: createFileBackedRepo('pattern-evolution'),
            };
            log.info('LearningManager using file-backed repos (no PostgreSQL)');
          }
          const learningManager = new LearningManager({
            persistence: learningPersistence,
            eventBus: globalEventBus,
          });
          try { await learningManager.initialize(); } catch { /* non-blocking */ }

          _learningScheduler.setDependencies({
            learningManager,
            dpoOptimizer: new DPOOptimizer({ pool: options.persistence }),
            calibrationTracker: new CalibrationTracker({ pool: options.persistence }),
            residualGovernance,
          });

          _learningScheduler.start();
          log.info('LearningScheduler started with dependencies');
        } catch (err) {
          log.warn('Could not start LearningScheduler', { error: err.message });
        }
      }
    }

    // EWC++ Consolidation: Prevent catastrophic forgetting
    // Fisher scores lock important patterns, daily consolidation cycle
    if (options.persistence && !_ewcService) {
      try {
        const { createEWCService } = await import('@cynic/persistence');
        _ewcService = createEWCService({ db: options.persistence });
        _ewcService.startScheduler();

        // Bridge: EWC consolidation → globalEventBus → LearningService
        // After consolidation, broadcast locked dimension Fisher scores
        // so Judge's LearningService can adjust dimension weight modifiers
        const dbPool = options.persistence;
        _ewcService.on('consolidation:completed', async (result) => {
          try {
            const { rows } = await dbPool.query(`
              SELECT name, fisher_importance
              FROM patterns
              WHERE category = 'dimension'
                AND consolidation_locked = TRUE
                AND fisher_importance >= 0.382
              ORDER BY fisher_importance DESC
            `);

            globalEventBus.emit(EventType.EWC_CONSOLIDATION_COMPLETED, {
              payload: {
                ...result,
                lockedDimensionPatterns: rows,
              },
            });

            if (rows.length > 0) {
              log.info('EWC Fisher scores broadcast to Judge', { lockedDimensions: rows.length });
            }
          } catch (err) {
            log.debug('EWC Fisher broadcast failed (non-blocking)', { error: err.message });
          }
        });

        log.info('EWC++ consolidation scheduler started (daily cycle)');
      } catch (err) {
        log.warn('EWC++ service startup failed (non-blocking)', { error: err.message });
        _ewcService = null;
      }
    }

    // PHASE 2 (DECENTRALIZE): Initialize NetworkNode if P2P enabled
    if (isP2PEnabled() && !_networkNode) {
      try {
        _networkNode = await getNetworkNodeAsync({
          persistence: options.persistence,
        });
        if (_networkNode) {
          await startNetworkNode();
          log.info('NetworkNode started (PHASE 2)', {
            nodeId: _networkNode.publicKey?.slice(0, 16),
            state: _networkNode.state,
          });
        }
      } catch (err) {
        log.warn('NetworkNode initialization failed (non-blocking)', { error: err.message });
      }
    }

    // C2.1 (SOLANA × PERCEIVE): Initialize SolanaWatcher if Solana env vars present
    if (!_solanaWatcher && (process.env.SOLANA_RPC_URL || process.env.SOLANA_CLUSTER)) {
      try {
        const cluster = process.env.SOLANA_CLUSTER || 'mainnet';
        _solanaWatcher = getSolanaWatcher({
          rpcUrl: process.env.SOLANA_RPC_URL || undefined,
          cluster,
          commitment: 'confirmed',
        });
        await _solanaWatcher.start();
        log.info('SolanaWatcher started (C2.1)', { cluster });
        try {
          await _solanaWatcher.watchSlots();
        } catch (slotErr) {
          log.warn('SolanaWatcher slot subscription failed (non-blocking)', { error: slotErr.message });
        }
      } catch (err) {
        log.warn('SolanaWatcher initialization failed (non-blocking)', { error: err.message });
        _solanaWatcher = null;
      }
    }

    // AXE 8 (AWARE): Activate ErrorHandler singleton with global error capture
    try {
      getErrorHandler({ captureGlobal: true });
      log.info('ErrorHandler activated (AXE 8: AWARE)');
    } catch (err) {
      log.warn('ErrorHandler activation failed (non-blocking)', { error: err.message });
    }

    // AXE 9 (CONSCIOUS): Wire ConsciousnessBridge — system health → awareness
    // Chain: HeartbeatService + SLATracker → ConsciousnessBridge → ConsciousnessMonitor
    // Also late-binds into ErrorHandler so errors feed consciousness
    if (!_consciousnessBridge) {
      try {
        // 1. Create ConsciousnessMonitor (the "inner eye")
        const consciousness = createConsciousnessMonitor({ windowSize: 100 });

        // 2. Create HeartbeatService with component health checks
        const pool = options.persistence?.pool || (options.persistence?.query ? options.persistence : null);
        const checks = createDefaultChecks({
          pool,
          collectivePack: pack,
        });
        _heartbeatService = getHeartbeatService({ components: checks });

        // 3. Create SLATracker wired to heartbeat
        const slaTracker = getSLATracker({ heartbeat: _heartbeatService });

        // 4. Wire everything into ConsciousnessBridge
        _consciousnessBridge = wireConsciousness({
          consciousness,
          heartbeat: _heartbeatService,
          slaTracker,
        });

        // 5. Late-bind into ErrorHandler so errors feed consciousness
        const errorHandler = getErrorHandler();
        if (errorHandler?.setDependencies) {
          errorHandler.setDependencies({ consciousness: _consciousnessBridge });
        }

        // 6. Start heartbeat monitoring
        _heartbeatService.start();

        log.info('ConsciousnessBridge wired (AXE 9: CONSCIOUS)', {
          components: Object.keys(checks),
          hasPool: !!pool,
          hasSLA: true,
        });
      } catch (err) {
        log.warn('ConsciousnessBridge initialization failed (non-blocking)', { error: err.message });
        _consciousnessBridge = null;
        _heartbeatService = null;
      }
    }

    // AXE 6 (EMERGE): Initialize EmergenceDetector for cross-session pattern analysis
    // Detects recurring mistakes, successful strategies, user preferences, workflow patterns
    // from data graves (dog_events, consensus_votes, collective_snapshots, judgments, etc.)
    if (options.persistence && !_emergenceDetector) {
      try {
        const pool = options.persistence.pool || (options.persistence.query ? options.persistence : null);
        if (pool) {
          _emergenceDetector = getEmergenceDetector({
            persistence: { pool },
            memoryRetriever: _sharedMemory || null,
          });
          _emergenceDetector.start();

          // Bridge significant patterns to globalEventBus for Dogs/listeners
          _emergenceDetector.on('significant_pattern', (pattern) => {
            globalEventBus.publish(EventType.PATTERN_DETECTED || 'pattern:detected', {
              source: 'EmergenceDetector',
              category: pattern.category,
              key: pattern.key,
              subject: pattern.subject,
              occurrences: pattern.occurrences,
              significance: pattern.significance,
              confidence: pattern.confidence,
            });
          });
          log.info('EmergenceDetector started (AXE 6: EMERGE)', { analysisInterval: '1h' });
        }
      } catch (err) {
        log.warn('EmergenceDetector initialization failed (non-blocking)', { error: err.message });
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
  _autoSaveInterval.unref();

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
    networkInitialized: !!_networkNode,
    solanaWatcherInitialized: !!_solanaWatcher,
    isAwakened: _isAwakened,
    sharedMemoryStats: _sharedMemory?.stats || null,
    packStats: _globalPack?.getStats?.() || null,
    qLearningStats: _qLearningService?.getStats?.() || null,
    networkState: _networkNode?.state || null,
    solanaWatcherRunning: _solanaWatcher?._isRunning || false,
    ewcServiceRunning: !!_ewcService?.consolidationTimer,
    ewcStats: _ewcService?.stats || null,
  };
}

/**
 * Get SolanaWatcher singleton (if initialized)
 *
 * @returns {import('./perception/solana-watcher.js').SolanaWatcher|null} Watcher or null
 */
export function getSolanaWatcherSingleton() {
  return _solanaWatcher;
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

  // PHASE 2: Stop network node
  if (_networkNode) {
    stopNetworkNode().catch(() => {});
    _networkNode = null;
  }

  // C2.1: Reset SolanaWatcher
  if (_solanaWatcher) {
    resetSolanaWatcher();
    _solanaWatcher = null;
  }

  // Auto-save: Stop interval
  if (_autoSaveInterval) {
    clearInterval(_autoSaveInterval);
    _autoSaveInterval = null;
  }

  // LearningScheduler: Stop DPO + Governance crons
  if (_learningScheduler) {
    _learningScheduler.stop();
    _learningScheduler = null;
  }

  // EWC++: Stop consolidation scheduler
  if (_ewcService) {
    _ewcService.stopScheduler();
    _ewcService = null;
  }

  // AXE 6: Stop EmergenceDetector
  if (_emergenceDetector) {
    _emergenceDetector.stop();
    _emergenceDetector = null;
  }

  // C6.1: Stop DogStateEmitter (has setInterval that prevents process exit)
  if (_dogStateEmitter) {
    _dogStateEmitter.stop();
    _dogStateEmitter = null;
  }

  // UnifiedBridge: Stop event subscriptions
  if (_unifiedBridge) {
    _unifiedBridge.stop();
    _unifiedBridge = null;
  }

  // AXE 9: Stop HeartbeatService + ConsciousnessBridge
  if (_heartbeatService) {
    _heartbeatService.stop();
    _heartbeatService = null;
  }
  _consciousnessBridge = null;

  // Human symbiosis singletons
  _humanAdvisor = null;
  _humanLearning = null;
  _humanAccountant = null;
  _humanEmergence = null;

  log.warn('Singletons reset (testing only)');
}

// ═══════════════════════════════════════════════════════════════════════════════
// EXPORTS
// ═══════════════════════════════════════════════════════════════════════════════

// AXE 2: Re-export event listener functions for external use
export { startEventListeners, stopEventListeners, cleanupOldEventData } from './services/event-listeners.js';
export { getListenerStats } from './services/event-listeners.js';

// PHASE 2: Re-export network singleton for external use
export { getNetworkNodeAsync as getNetworkNode, startNetworkNode, stopNetworkNode, getNetworkStatus, isP2PEnabled } from './network-singleton.js';

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
  cleanupOldEventData,
  // PHASE 2: Network
  getNetworkNodeAsync,
  startNetworkNode,
  stopNetworkNode,
  // C2.1: Solana perception
  getSolanaWatcherSingleton,
};
