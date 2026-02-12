/**
 * CYNIC Daemon — Service Wiring
 *
 * Force-initializes daemon-essential singletons at boot time.
 * No lazy loading — the daemon wakes warm.
 *
 * Wires:
 *   - ModelIntelligence (Thompson Sampling)
 *   - CostLedger (budget tracking)
 *   - cost:update → ModelIntelligence budget awareness
 *   - Periodic state persistence (5 min, .unref())
 *
 * "Le chien se réchauffe avant la chasse" — CYNIC
 *
 * @module @cynic/node/daemon/service-wiring
 */

'use strict';

import { createLogger, globalEventBus, EventType } from '@cynic/core';
import { getModelIntelligence } from '../learning/model-intelligence.js';
import { getCostLedger } from '../accounting/cost-ledger.js';
import { getCollectivePackAsync } from '../collective-singleton.js';
import { createSONA } from '../learning/sona.js';
import { createBehaviorModifier } from '../learning/behavior-modifier.js';
import { getMetaCognition } from '../learning/meta-cognition.js';
import { getOrchestrator } from '../orchestration/unified-orchestrator.js';
import { createKabbalisticRouter } from '../orchestration/kabbalistic-router.js';
import { DogOrchestrator } from '../agents/orchestrator.js';
import { getQLearningService } from '../orchestration/learning-service.js';
import { wireQLearning, cleanupQLearning } from '../orchestration/q-learning-wiring.js';

const log = createLogger('ServiceWiring');

/** Persist interval: 5 min (φ × 3 ≈ 4.85, rounded to 5) */
const PERSIST_INTERVAL_MS = 5 * 60 * 1000;

let _persistTimer = null;
let _costListener = null;
let _wired = false;

// Learning system state
let _learningWired = false;
let _sona = null;
let _behaviorModifier = null;
let _metaCognition = null;
let _learningPersistTimer = null;
let _sonaListener = null;
let _feedbackListener = null;

// Orchestration system state
let _orchestratorWired = false;
let _orchestrator = null;
let _kabbalisticRouter = null;
let _dogOrchestrator = null;

/**
 * Wire daemon-essential services at boot.
 *
 * Force-initializes singletons so they're warm for hook requests.
 * Wires cross-service events and periodic persistence.
 *
 * @returns {{ modelIntelligence: Object, costLedger: Object }}
 */
export function wireDaemonServices() {
  if (_wired) {
    log.debug('Services already wired — skipping');
    return {
      modelIntelligence: getModelIntelligence(),
      costLedger: getCostLedger(),
    };
  }

  // 1. Force-initialize singletons (warm, not lazy)
  const mi = getModelIntelligence();
  const costLedger = getCostLedger();

  log.info('Singletons warm', {
    modelIntelligence: !!mi,
    costLedger: !!costLedger,
    thompsonMaturity: mi.getStats().samplerMaturity,
  });

  // 2. Wire cost:update → ModelIntelligence budget awareness
  _costListener = (data) => {
    try {
      const budget = data?.budget;
      if (budget && budget.level) {
        // Emit model recommendation when budget changes
        const rec = costLedger.recommendModel({
          taskType: 'moderate',
          needsReasoning: false,
        });
        globalEventBus.emit('model:recommendation', {
          model: rec.model,
          reason: rec.reason,
          budgetLevel: budget.level,
          timestamp: Date.now(),
        });
      }
    } catch (err) {
      log.debug('cost:update handler error', { error: err.message });
    }
  };

  globalEventBus.on('cost:update', _costListener);

  // 3. Periodic state persistence (.unref() so it doesn't block exit)
  _persistTimer = setInterval(() => {
    try {
      mi.persist();
      costLedger.persist();
      log.debug('Periodic persist completed');
    } catch (err) {
      log.debug('Periodic persist failed', { error: err.message });
    }
  }, PERSIST_INTERVAL_MS);
  _persistTimer.unref();

  _wired = true;

  log.info('Daemon services wired');
  return { modelIntelligence: mi, costLedger };
}

// ═══════════════════════════════════════════════════════════════════════════════
// LEARNING SYSTEM — collective-singleton + SONA + BehaviorModifier + MetaCognition
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Wire the learning system at daemon boot.
 *
 * Initializes collective-singleton (Q-Learning, LearningScheduler, UnifiedBridge,
 * EmergenceDetector, EventListeners, etc.) plus SONA, BehaviorModifier, and
 * MetaCognition. These are the organs that let CYNIC learn from every session.
 *
 * Non-blocking: daemon still runs if learning fails to initialize.
 *
 * @returns {Promise<{ sona: Object|null, behaviorModifier: Object|null, metaCognition: Object|null }>}
 */
export async function wireLearningSystem() {
  if (_learningWired) {
    log.debug('Learning system already wired — skipping');
    return { sona: _sona, behaviorModifier: _behaviorModifier, metaCognition: _metaCognition };
  }

  // 1. Boot collective-singleton (the big one — wakes everything)
  try {
    await getCollectivePackAsync();
    log.info('Collective-singleton initialized (Q-Learning, LearningScheduler, EventListeners, Emergence)');
  } catch (err) {
    log.warn('Collective-singleton init failed — learning degraded', { error: err.message });
  }

  // 2. SONA — real-time pattern adaptation
  try {
    _sona = createSONA();
    _sonaListener = async (data) => {
      try {
        if (data?.patternId && data?.dimensionScores) {
          _sona.observe({
            patternId: data.patternId,
            dimensionScores: data.dimensionScores,
            judgmentId: data.judgmentId,
          });

          // GAP-3: Record to learning_events table for G1.2 metric
          try {
            const { getPool } = await import('@cynic/persistence');
            const pool = getPool();
            await pool.query(`
              INSERT INTO learning_events (loop_type, event_type, judgment_id, pattern_id, metadata)
              VALUES ($1, $2, $3, $4, $5)
            `, [
              'sona',
              'observation',
              data.judgmentId || null,
              data.patternId || null,
              JSON.stringify({ dimensionCount: Object.keys(data.dimensionScores || {}).length })
            ]);
          } catch { /* non-blocking DB write */ }
        }
      } catch { /* non-blocking */ }
    };
    globalEventBus.on(EventType.JUDGMENT_CREATED || 'judgment:created', _sonaListener);
    log.info('SONA wired — observing judgments');
  } catch (err) {
    log.warn('SONA init failed', { error: err.message });
  }

  // 3. BehaviorModifier — feedback → behavior changes
  try {
    _behaviorModifier = createBehaviorModifier();
    _feedbackListener = async (data) => {
      try {
        if (_behaviorModifier && data) {
          _behaviorModifier.processFeedback(data);
        }
        if (_sona && data?.judgmentId) {
          _sona.processFeedback(data);
        }

        // GAP-3: Record to learning_events table for G1.2 metric
        try {
          const { getPool } = await import('@cynic/persistence');
          const pool = getPool();
          await pool.query(`
            INSERT INTO learning_events (loop_type, event_type, judgment_id, feedback_value, metadata)
            VALUES ($1, $2, $3, $4, $5)
          `, [
            'feedback-loop',
            'feedback',
            data?.judgmentId || null,
            data?.value || null,
            JSON.stringify({ source: 'behavior-modifier' })
          ]);
        } catch { /* non-blocking DB write */ }
      } catch { /* non-blocking */ }
    };
    globalEventBus.on(EventType.USER_FEEDBACK || 'feedback:processed', _feedbackListener);
    log.info('BehaviorModifier wired — processing feedback');
  } catch (err) {
    log.warn('BehaviorModifier init failed', { error: err.message });
  }

  // 4. MetaCognition — self-monitoring and strategy switching
  try {
    _metaCognition = getMetaCognition();

    // GAP-3: Record meta-cognition events
    const metaListener = async (data) => {
      try {
        const { getPool } = await import('@cynic/persistence');
        const pool = getPool();
        await pool.query(`
          INSERT INTO learning_events (loop_type, event_type, action_taken, metadata)
          VALUES ($1, $2, $3, $4)
        `, [
          'meta-cognition',
          data.type || 'strategy-switch',
          data.action || null,
          JSON.stringify(data)
        ]);
      } catch { /* non-blocking */ }
    };
    globalEventBus.on('metacognition:action', metaListener);

    log.info('MetaCognition wired — self-monitoring active');
  } catch (err) {
    log.warn('MetaCognition init failed', { error: err.message });
  }

  // 5. Periodic persist for learning singletons (3 min, offset from main persist)
  _learningPersistTimer = setInterval(() => {
    try {
      if (_sona?.getStats) {
        log.debug('Learning persist tick', { sonaPatterns: _sona.getStats().trackedPatterns || 0 });
      }
    } catch { /* non-blocking */ }
  }, 3 * 60 * 1000);
  _learningPersistTimer.unref();

  _learningWired = true;
  log.info('Learning system wired — organism breathing');

  return { sona: _sona, behaviorModifier: _behaviorModifier, metaCognition: _metaCognition };
}

/**
 * Get SONA singleton (if wired).
 * @returns {Object|null}
 */
export function getSONASingleton() { return _sona; }

/**
 * Get BehaviorModifier singleton (if wired).
 * @returns {Object|null}
 */
export function getBehaviorModifierSingleton() { return _behaviorModifier; }

/**
 * Get MetaCognition singleton (if wired).
 * @returns {Object|null}
 */
export function getMetaCognitionSingleton() { return _metaCognition; }

/**
 * Check if learning system is wired.
 * @returns {boolean}
 */
export function isLearningWired() { return _learningWired; }

/**
 * Cleanup wired services for graceful shutdown.
 *
 * Persists final state and removes event listeners.
 */
export async function cleanupDaemonServices() {
  if (!_wired) return;

  // Stop periodic persist
  if (_persistTimer) {
    clearInterval(_persistTimer);
    _persistTimer = null;
  }

  // Remove event listener
  if (_costListener) {
    globalEventBus.removeListener('cost:update', _costListener);
    _costListener = null;
  }

  // Final persist
  try {
    getModelIntelligence().persist();
    getCostLedger().persist();
    log.info('Final persist completed');
  } catch (err) {
    log.debug('Final persist failed', { error: err.message });
  }

  // Clean up learning system
  cleanupLearningSystem();

  // Clean up orchestrator
  cleanupOrchestrator();

  // Clean up watchers
  await cleanupWatchers();

  _wired = false;
  log.info('Daemon services cleaned up');
}

/**
 * Cleanup learning system resources.
 */
function cleanupLearningSystem() {
  if (!_learningWired) return;

  if (_learningPersistTimer) {
    clearInterval(_learningPersistTimer);
    _learningPersistTimer = null;
  }

  if (_sonaListener) {
    globalEventBus.removeListener(EventType.JUDGMENT_CREATED || 'judgment:created', _sonaListener);
    _sonaListener = null;
  }

  if (_feedbackListener) {
    globalEventBus.removeListener(EventType.USER_FEEDBACK || 'feedback:processed', _feedbackListener);
    _feedbackListener = null;
  }

  _sona = null;
  _behaviorModifier = null;
  _metaCognition = null;
  _learningWired = false;
  log.info('Learning system cleaned up');
}

// ═══════════════════════════════════════════════════════════════════════════════
// ORCHESTRATION SYSTEM — UnifiedOrchestrator + KabbalisticRouter + DogOrchestrator
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Wire the orchestration system at daemon boot.
 *
 * Creates and wires UnifiedOrchestrator with all routing dependencies:
 * - KabbalisticRouter (Lightning Flash routing through Tree of Life)
 * - DogOrchestrator (11 dogs consensus system)
 * - LearningService (Q-Learning for route optimization)
 * - CostLedger (budget-aware routing)
 * - CollectivePack (shared memory + dogs)
 *
 * This enables event routing through:
 * globalEventBus → KabbalisticRouter → Dogs → Consensus → Learning
 *
 * Metrics enabled:
 * - G1.4: ≥20 KabbalisticRouter calls logged
 * - G1.5: ≥10 LLMRouter routes to non-Anthropic
 *
 * @returns {Promise<{ orchestrator: Object|null, kabbalisticRouter: Object|null, dogOrchestrator: Object|null }>}
 */
export async function wireOrchestrator() {
  if (_orchestratorWired) {
    log.debug('Orchestrator already wired — skipping');
    return {
      orchestrator: _orchestrator,
      kabbalisticRouter: _kabbalisticRouter,
      dogOrchestrator: _dogOrchestrator,
    };
  }

  try {
    // 1. Get CollectivePack (must be initialized first — contains dogs + shared memory)
    const pack = await getCollectivePackAsync();
    const sharedMemory = pack.sharedMemory;
    log.debug('CollectivePack loaded for orchestrator wiring');

    // 1.5. Wire Q-Learning (load Q-table from DB, track updates for G1.3)
    // This MUST happen before KabbalisticRouter creation so weights are loaded
    const { learningService: qLearningService, loaded: qTableLoaded } = await wireQLearning();
    if (qTableLoaded) {
      log.info('Q-Learning wired — weights loaded from DB and ready for routing');
    }

    // 2. Get LearningService (Q-Learning for route optimization)
    const learningService = qLearningService || getQLearningService(); // Use initialized Q-Learning service
    log.debug('LearningService ready');

    // 3. Get CostLedger (budget-aware routing)
    const costLedger = getCostLedger();
    log.debug('CostLedger ready');

    // 4. Create KabbalisticRouter (Lightning Flash routing)
    _kabbalisticRouter = createKabbalisticRouter({
      collectivePack: pack,
      learningService,
      costOptimizer: costLedger, // CostLedger acts as cost optimizer
    });
    log.info('KabbalisticRouter created');

    // 5. Create DogOrchestrator (11 dogs consensus)
    _dogOrchestrator = new DogOrchestrator({
      collectivePack: pack,
      sharedMemory,
      mode: 'parallel', // Run all dogs in parallel for full consensus
      consensusThreshold: 0.618, // φ⁻¹ consensus threshold
      useSwarmConsensus: true, // Enable P2.4 swarm consensus
    });
    log.info('DogOrchestrator created');

    // 6. Create UnifiedOrchestrator (central coordination facade)
    _orchestrator = getOrchestrator({
      kabbalisticRouter: _kabbalisticRouter,
      dogOrchestrator: _dogOrchestrator,
      learningService,
      costOptimizer: costLedger,
      eventBus: globalEventBus,
    });
    log.info('UnifiedOrchestrator created');

    // 7. Wire globalEventBus events to orchestrator routing
    // This enables automatic routing for key events
    const eventTypes = [
      'judgment:request',
      'tool:use',
      'hook:guard',
      'hook:observe',
      'consensus:needed',
    ];

    for (const eventType of eventTypes) {
      globalEventBus.on(eventType, async (data) => {
        try {
          // Route event through UnifiedOrchestrator
          const result = await _orchestrator.process({
            eventType,
            content: data.content || data.input || '',
            context: data.context || {},
            userContext: data.userContext || {},
          });

          // Emit result for downstream consumers
          globalEventBus.emit(`${eventType}:routed`, {
            original: data,
            routing: result.routing,
            outcome: result.outcome,
            judgment: result.judgment,
          });
        } catch (err) {
          log.debug(`Event routing failed for ${eventType}`, { error: err.message });
        }
      });
    }

    log.info('Event routing wired — globalEventBus → UnifiedOrchestrator');

    _orchestratorWired = true;

    return {
      orchestrator: _orchestrator,
      kabbalisticRouter: _kabbalisticRouter,
      dogOrchestrator: _dogOrchestrator,
    };
  } catch (err) {
    log.warn('Orchestrator wiring failed — daemon still operational', { error: err.message });
    return {
      orchestrator: null,
      kabbalisticRouter: null,
      dogOrchestrator: null,
    };
  }
}

/**
 * Get UnifiedOrchestrator singleton (if wired).
 * @returns {Object|null}
 */
export function getOrchestratorSingleton() {
  return _orchestrator;
}

/**
 * Get KabbalisticRouter singleton (if wired).
 * @returns {Object|null}
 */
export function getKabbalisticRouterSingleton() {
  return _kabbalisticRouter;
}

/**
 * Get DogOrchestrator singleton (if wired).
 * @returns {Object|null}
 */
export function getDogOrchestratorSingleton() {
  return _dogOrchestrator;
}

/**
 * Check if orchestrator is wired.
 * @returns {boolean}
 */
export function isOrchestratorWired() {
  return _orchestratorWired;
}

/**
 * Cleanup orchestrator resources.
 */
function cleanupOrchestrator() {
  if (!_orchestratorWired) return;

  // Remove event listeners
  const eventTypes = [
    'judgment:request',
    'tool:use',
    'hook:guard',
    'hook:observe',
    'consensus:needed',
  ];

  for (const eventType of eventTypes) {
    globalEventBus.removeAllListeners(eventType);
  }

  _orchestrator = null;
  _kabbalisticRouter = null;
  _dogOrchestrator = null;

  // Cleanup Q-Learning wiring
  cleanupQLearning();
  _orchestratorWired = false;

  log.info('Orchestrator cleaned up');
}

// ═══════════════════════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════════════════════
// CONSCIOUSNESS REFLECTION — meta-cognition self-reflection loop
// ═══════════════════════════════════════════════════════════════════════════════

let _consciousnessWired = false;
let _consciousnessReader = null;
let _reflectionTimer = null;

const REFLECTION_INTERVAL_MS = 60 * 60 * 1000; // 60 min (φ × 100 ≈ 62 min)

/**
 * Wire consciousness reflection — periodic self-reflection loop.
 *
 * Reads consciousness state from DB (judgments, patterns, learning metrics)
 * and generates self-reflections for meta-cognition.
 *
 * @returns {Promise<{ consciousnessReader: Object|null }>}
 */
export async function wireConsciousnessReflection() {
  if (_consciousnessWired) {
    log.debug("Consciousness reflection already wired — skipping");
    return { consciousnessReader: _consciousnessReader };
  }

  try {
    const { getConsciousnessReader } = await import("../orchestration/consciousness-reader.js");
    _consciousnessReader = getConsciousnessReader();
    await _consciousnessReader.initialize();

    log.info("ConsciousnessReader initialized — starting periodic reflection");

    // Periodic reflection task
    _reflectionTimer = setInterval(async () => {
      try {
        const result = await _consciousnessReader.reflect();
        log.info("Self-reflection completed", {
          reflectionId: result.id,
          confidence: result.reflection.overallConfidence.toFixed(3),
          concerns: result.reflection.prompts.filter(p => p.concern).length,
        });
      } catch (err) {
        log.warn("Self-reflection failed", { error: err.message });
      }
    }, REFLECTION_INTERVAL_MS);
    _reflectionTimer.unref();

    _consciousnessWired = true;
    log.info("Consciousness reflection wired — φ observes φ");

    return { consciousnessReader: _consciousnessReader };
  } catch (err) {
    log.warn("ConsciousnessReader init failed", { error: err.message });
    return { consciousnessReader: null };
  }
}

/**
 * Cleanup consciousness reflection.
 */
export async function cleanupConsciousnessReflection() {
  if (!_consciousnessWired) return;

  if (_reflectionTimer) {
    clearInterval(_reflectionTimer);
    _reflectionTimer = null;
  }

  _consciousnessReader = null;
  _consciousnessWired = false;
  log.info("Consciousness reflection cleaned up");
}

/**
 * Check if consciousness reflection is wired.
 * @returns {boolean}
 */
export function isConsciousnessWired() {
  return _consciousnessWired;
}

// WATCHERS — perception polling (FileWatcher, SolanaWatcher)
// ═══════════════════════════════════════════════════════════════════════════════

let _watchersWired = false;
let _fileWatcherPoll = null;
let _solanaWatcherPoll = null;
let _fsWatcher = null;
let _solanaWatcher = null;

/** Polling intervals (φ-aligned) */
const FILE_POLL_INTERVAL = 5000; // 5s
const SOLANA_POLL_INTERVAL = 30000; // 30s
const HEARTBEAT_INTERVAL = 60000; // 1 min

/**
 * Wire watchers — FileWatcher + SolanaWatcher polling.
 *
 * Starts polling loops that:
 * 1. Poll for changes (filesystem via chokidar watch, Solana via slot checks)
 * 2. Emit events to globalEventBus
 * 3. Record heartbeats to watcher_heartbeats table
 *
 * Handles 429 rate limits gracefully (backs off, logs, continues).
 *
 * @param {Object} [options]
 * @param {Object} [options.db] - PostgreSQL client for heartbeats
 * @param {string[]} [options.watchPaths] - Paths to watch for filesystem
 * @param {string} [options.solanaRpcUrl] - Solana RPC URL
 * @returns {Promise<{ fileWatcher: Object, solanaWatcher: Object }>}
 */
export async function wireWatchers(options = {}) {
  if (_watchersWired) {
    log.debug('Watchers already wired — skipping');
    return { fileWatcher: _fsWatcher, solanaWatcher: _solanaWatcher };
  }

  const { createFilesystemWatcher } = await import('../perception/filesystem-watcher.js');
  const { getSolanaWatcher } = await import('../perception/solana-watcher.js');
  const { getPostgresClient } = await import('@cynic/persistence');

  const db = options.db || getPostgresClient();

  // 1. Initialize FilesystemWatcher
  try {
    _fsWatcher = createFilesystemWatcher({
      paths: options.watchPaths || [process.cwd()],
      eventBus: globalEventBus,
    });
    _fsWatcher.start();
    log.info('FilesystemWatcher started');
  } catch (err) {
    log.warn('FilesystemWatcher init failed', { error: err.message });
  }

  // 2. Initialize SolanaWatcher
  try {
    _solanaWatcher = getSolanaWatcher({
      rpcUrl: options.solanaRpcUrl || process.env.SOLANA_RPC_URL,
      eventBus: globalEventBus,
    });
    await _solanaWatcher.start();

    // Subscribe to slots for active polling
    await _solanaWatcher.watchSlots();

    log.info('SolanaWatcher started');
  } catch (err) {
    log.warn('SolanaWatcher init failed (degraded mode)', { error: err.message });
  }

  // 3. Start heartbeat polling (records to DB)
  _fileWatcherPoll = setInterval(async () => {
    if (!_fsWatcher) return;

    try {
      const stats = _fsWatcher.getStats();
      const status = _fsWatcher.isRunning() ? 'active' : 'stopped';

      await _recordHeartbeat(db, {
        watcher_name: 'FilesystemWatcher',
        events_polled: stats.eventsEmitted || 0,
        status,
        metadata: { filesWatched: stats.filesWatched, uptime: stats.uptime },
      });

      log.debug('FilesystemWatcher heartbeat', { status, events: stats.eventsEmitted });
    } catch (err) {
      log.debug('FilesystemWatcher heartbeat failed', { error: err.message });
    }
  }, HEARTBEAT_INTERVAL);
  _fileWatcherPoll.unref();

  _solanaWatcherPoll = setInterval(async () => {
    if (!_solanaWatcher) return;

    try {
      const stats = _solanaWatcher.getStats();
      const health = _solanaWatcher.getHealth();
      const status = health.status === 'offline' ? 'stopped'
        : health.status === 'critical' ? 'error'
        : 'active';

      await _recordHeartbeat(db, {
        watcher_name: 'SolanaWatcher',
        events_polled: stats.eventsEmitted || 0,
        status,
        error_message: health.status === 'critical' ? `Error rate: ${(health.errorRate * 100).toFixed(1)}%` : null,
        metadata: {
          lastSlot: stats.lastSlot,
          uptime: stats.uptime,
          eventsPerMinute: health.eventsPerMinute,
        },
      });

      log.debug('SolanaWatcher heartbeat', { status, events: stats.eventsEmitted, slot: stats.lastSlot });
    } catch (err) {
      // Handle 429 rate limits gracefully
      if (err.message?.includes('429')) {
        log.debug('SolanaWatcher rate limited (429) — backing off');
        await _recordHeartbeat(db, {
          watcher_name: 'SolanaWatcher',
          events_polled: 0,
          status: 'idle',
          error_message: 'Rate limited (429)',
          metadata: { backoff: true },
        }).catch(() => {}); // Non-blocking
      } else {
        log.debug('SolanaWatcher heartbeat failed', { error: err.message });
      }
    }
  }, HEARTBEAT_INTERVAL);
  _solanaWatcherPoll.unref();

  _watchersWired = true;
  log.info('Watchers wired — perception layer active');

  return { fileWatcher: _fsWatcher, solanaWatcher: _solanaWatcher };
}

/**
 * Record heartbeat to watcher_heartbeats table.
 * @private
 */
async function _recordHeartbeat(db, data) {
  if (!db) return;

  await db.query(`
    INSERT INTO watcher_heartbeats
      (watcher_name, events_polled, status, error_message, metadata)
    VALUES ($1, $2, $3, $4, $5)
  `, [
    data.watcher_name,
    data.events_polled || 0,
    data.status || 'active',
    data.error_message || null,
    JSON.stringify(data.metadata || {}),
  ]);
}

/**
 * Stop watchers and cleanup.
 */
export async function cleanupWatchers() {
  if (!_watchersWired) return;

  if (_fileWatcherPoll) {
    clearInterval(_fileWatcherPoll);
    _fileWatcherPoll = null;
  }

  if (_solanaWatcherPoll) {
    clearInterval(_solanaWatcherPoll);
    _solanaWatcherPoll = null;
  }

  if (_fsWatcher) {
    await _fsWatcher.stop();
    _fsWatcher = null;
  }

  if (_solanaWatcher) {
    await _solanaWatcher.stop();
    _solanaWatcher = null;
  }

  _watchersWired = false;
  log.info('Watchers cleaned up');
}

/**
 * Check if watchers are wired.
 * @returns {boolean}
 */
export function isWatchersWired() {
  return _watchersWired;
}

/**
 * Check if services are wired.
 * @returns {boolean}
 */
export function isWired() {
  return _wired;
}

/**
 * Reset for testing — clears all state without persisting.
 */
export async function _resetForTesting() {
  if (_persistTimer) {
    clearInterval(_persistTimer);
    _persistTimer = null;
  }
  if (_costListener) {
    globalEventBus.removeListener('cost:update', _costListener);
    _costListener = null;
  }
  _wired = false;

  // Reset learning system
  if (_learningPersistTimer) {
    clearInterval(_learningPersistTimer);
    _learningPersistTimer = null;
  }
  if (_sonaListener) {
    globalEventBus.removeListener(EventType.JUDGMENT_CREATED || 'judgment:created', _sonaListener);
    _sonaListener = null;
  }
  if (_feedbackListener) {
    globalEventBus.removeListener(EventType.USER_FEEDBACK || 'feedback:processed', _feedbackListener);
    _feedbackListener = null;
  }
  _sona = null;
  _behaviorModifier = null;
  _metaCognition = null;
  _learningWired = false;

  // Reset orchestrator
  cleanupOrchestrator();

  // Reset watchers
  await cleanupWatchers();
}
