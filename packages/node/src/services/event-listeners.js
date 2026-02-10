/**
 * Event Listeners - Closing Data Loops (AXE 2: PERSIST)
 *
 * "Le chien n'oublie jamais" - CYNIC persists everything.
 *
 * This module subscribes to key events and persists data to PostgreSQL.
 * It closes the "black holes" where data was emitted but never stored.
 *
 * BLACK HOLES CLOSED:
 * - JUDGMENT_CREATED: Now persisted to judgments table
 * - feedback:processed: Now creates feedback record + increments session counter
 * - SESSION_ENDED: Now consolidates SharedMemory
 * - DOG_EVENT: Now persisted to dog_events table (AXE 2+)
 * - CONSENSUS_COMPLETED: Now persisted to consensus_votes table (AXE 2+)
 * - DogSignal.*: Now persisted to dog_signals table (AXE 2+)
 * - CYNIC_STATE: Now sampled to collective_snapshots table (AXE 2+)
 * - THREAT_BLOCKED: Now persisted to dog_signals table (Fix #5)
 * - QUALITY_REPORT: Now persisted to dog_events table (Fix #5)
 * - VULNERABILITY_DETECTED: Now persisted to dog_signals table (Fix #5)
 *
 * @module @cynic/node/services/event-listeners
 */

'use strict';

import { createLogger, globalEventBus, EventType } from '@cynic/core';
import { DogSignal } from '../agents/collective/ambient-consensus.js';
import { getEventBus, EventType as AutomationEventType } from './event-bus.js';
import { AgentEvent } from '../agents/events.js';
import { getThermodynamicState } from '../organism/thermodynamics.js';
import { persistThermodynamics, persistConsciousnessTransition, createFileBackedRepo } from '@cynic/persistence';

const log = createLogger('EventListeners');

// ═══════════════════════════════════════════════════════════════════════════════
// STATE
// ═══════════════════════════════════════════════════════════════════════════════

/** @type {Function[]} Unsubscribe functions */
let _unsubscribers = [];

/** @type {boolean} Are listeners started? */
let _started = false;

/** @type {Object} Retry configuration */
const RETRY_CONFIG = {
  maxRetries: 3,
  initialDelayMs: 100,
  maxDelayMs: 2000,
  backoffMultiplier: 2,
};

/** @type {Object} Statistics */
const _stats = {
  judgmentsPersisted: 0,
  judgmentsFailed: 0,
  feedbackPersisted: 0,
  feedbackFailed: 0,
  sessionCountersIncremented: 0,
  sessionEndConsolidations: 0,
  dogEventsPersisted: 0,
  dogEventsFailed: 0,
  consensusPersisted: 0,
  consensusFailed: 0,
  dogSignalsPersisted: 0,
  dogSignalsFailed: 0,
  snapshotsPersisted: 0,
  snapshotsFailed: 0,
  thermoPersisted: 0,
  thermoFailed: 0,
  consciousnessPersisted: 0,
  consciousnessFailed: 0,
  blocksFinalizedPersisted: 0,
  blocksFinalizedFailed: 0,
  blocksAnchoredPersisted: 0,
  blocksAnchoredFailed: 0,
  anchorFailuresPersisted: 0,
  anchorFailuresFailed: 0,
  // RIGHT side stats
  codeDecisionsTriggered: 0,
  cynicAccountingOps: 0,
  codeAccountingOps: 0,
  humanActionsTriggered: 0,
  // Solana pipeline stats (C2.2-C2.7)
  solanaJudgments: 0,
  solanaDecisions: 0,
  solanaActionsRecorded: 0,
  solanaLearnings: 0,
  solanaAccountingOps: 0,
  solanaEmergencePatterns: 0,
  // Emergence pipeline stats (C1.7, C5.7)
  codeEmergencePatterns: 0,
  codeEmergenceChanges: 0,
  humanEmergenceSnapshots: 0,
  humanEmergencePatterns: 0,
  startedAt: null,
};

/** @type {number} Counter for sampling CYNIC_STATE emissions */
let _cynicStateCounter = 0;

/** @type {NodeJS.Timeout|null} Solana emergence analysis interval (F8=21min) */
let _solanaEmergenceInterval = null;

/** @type {NodeJS.Timeout|null} Code emergence analysis interval (F7=13min) */
let _codeEmergenceInterval = null;

/** @type {NodeJS.Timeout|null} Human emergence analysis interval (F9=34min) */
let _humanEmergenceInterval = null;

// ═══════════════════════════════════════════════════════════════════════════════
// RETRY UTILITY
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Retry an async operation with exponential backoff
 * "φ persists" - we don't give up easily
 *
 * @param {Function} operation - Async function to retry
 * @param {string} operationName - Name for logging
 * @param {Object} [options] - Retry options
 * @returns {Promise<any>} Operation result
 * @throws {Error} If all retries exhausted
 */
async function withRetry(operation, operationName, options = {}) {
  const {
    maxRetries = RETRY_CONFIG.maxRetries,
    initialDelayMs = RETRY_CONFIG.initialDelayMs,
    maxDelayMs = RETRY_CONFIG.maxDelayMs,
    backoffMultiplier = RETRY_CONFIG.backoffMultiplier,
  } = options;

  let lastError;
  let delay = initialDelayMs;

  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      return await operation();
    } catch (err) {
      lastError = err;
      log.warn(`${operationName} failed (attempt ${attempt}/${maxRetries})`, {
        error: err.message,
        nextRetryMs: attempt < maxRetries ? delay : null,
      });

      if (attempt < maxRetries) {
        await new Promise((resolve) => setTimeout(resolve, delay));
        delay = Math.min(delay * backoffMultiplier, maxDelayMs);
      }
    }
  }

  // All retries exhausted - throw loudly (FAIL LOUDLY principle)
  log.error(`${operationName} FAILED after ${maxRetries} retries`, {
    error: lastError.message,
  });
  throw lastError;
}

// ═══════════════════════════════════════════════════════════════════════════════
// JUDGMENT PERSISTENCE
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Handle JUDGMENT_CREATED event
 * Persists judgment to PostgreSQL judgments table
 *
 * @param {Object} event - CYNIC Event
 * @param {Object} repositories - Persistence repositories
 * @param {Object} context - Listener context (sessionId, userId)
 */
async function handleJudgmentCreated(event, repositories, context) {
  if (!repositories?.judgments) {
    log.debug('No judgments repository - skipping persistence');
    return;
  }

  const { id, payload, source, timestamp } = event;
  const {
    qScore,
    verdict,
    dimensions,
    dimensionScores,
    axiomScores,
    itemType,
    confidence,
    globalScore,
    weaknesses,
    item,
    context: judgmentContext,
    reasoningPath,
  } = payload || {};

  // Don't persist if no meaningful data
  if (qScore === undefined && !verdict) {
    log.debug('Judgment event has no score/verdict - skipping', { eventId: id });
    return;
  }

  try {
    await withRetry(async () => {
      const saved = await repositories.judgments.create({
        id: id, // Preserve event ID for PoJ traceability
        judgmentId: id,
        qScore,
        q_score: qScore,
        globalScore: globalScore || qScore,
        global_score: globalScore || qScore,
        verdict,
        dimensions: dimensionScores || dimensions,
        dimensionScores: dimensionScores || dimensions,
        axiomScores,
        axiom_scores: axiomScores,
        weaknesses: weaknesses || [],
        itemType,
        item: item || { type: itemType },
        confidence,
        sessionId: context.sessionId,
        userId: context.userId,
        context: judgmentContext || {
          source: source || 'event-listener',
          timestamp: timestamp || Date.now(),
        },
        reasoningPath: reasoningPath || [],
        reasoning_path: reasoningPath || [],
      });
      log.debug('Judgment persisted', {
        judgmentId: saved.judgment_id,
        qScore,
        verdict,
      });
      return saved;
    }, `Persist judgment ${id}`);

    _stats.judgmentsPersisted++;

    // Also increment session judgment counter
    if (context.sessionId && repositories.sessions) {
      try {
        await repositories.sessions.increment(context.sessionId, 'judgment_count');
        _stats.sessionCountersIncremented++;
      } catch (err) {
        // Non-fatal - session might not exist
        log.debug('Session counter increment failed', { error: err.message });
      }
    }
  } catch (err) {
    _stats.judgmentsFailed++;
    // Already logged by withRetry - emit error event for observability
    globalEventBus.publish(EventType.COMPONENT_ERROR, {
      component: 'EventListeners',
      operation: 'handleJudgmentCreated',
      error: err.message,
      eventId: id,
    });
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FEEDBACK PERSISTENCE
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Handle feedback:processed event from FeedbackProcessor
 * Persists feedback to PostgreSQL and increments session counter
 *
 * @param {Object} data - Feedback processing result
 * @param {Object} repositories - Persistence repositories
 * @param {Object} context - Listener context (sessionId, userId)
 * @param {Object} [feedbackData] - Original feedback data (if available)
 */
async function handleFeedbackProcessed(data, repositories, context, feedbackData = {}) {
  if (!repositories?.feedback) {
    log.debug('No feedback repository - skipping persistence');
    return;
  }

  const { scoreDelta, queueSize, immediateAdjustments } = data || {};

  try {
    await withRetry(async () => {
      const saved = await repositories.feedback.create({
        judgmentId: feedbackData.judgmentId || null, // Supports orphan feedback
        userId: context.userId,
        outcome: feedbackData.outcome || 'partial',
        actualScore: feedbackData.actualScore,
        reason: feedbackData.reason || `Score delta: ${scoreDelta}`,
        sourceType: feedbackData.source || 'system',
        sourceContext: {
          scoreDelta,
          queueSize,
          immediateAdjustments,
          ...feedbackData.sourceContext,
        },
      });
      log.debug('Feedback persisted', {
        feedbackId: saved.id,
        outcome: saved.outcome,
        judgmentId: saved.judgment_id,
      });
      return saved;
    }, 'Persist feedback');

    _stats.feedbackPersisted++;

    // Increment session feedback counter
    if (context.sessionId && repositories.sessions) {
      try {
        await repositories.sessions.increment(context.sessionId, 'feedback_count');
        _stats.sessionCountersIncremented++;
      } catch (err) {
        log.debug('Session feedback counter increment failed', { error: err.message });
      }
    }
  } catch (err) {
    _stats.feedbackFailed++;
    globalEventBus.publish(EventType.COMPONENT_ERROR, {
      component: 'EventListeners',
      operation: 'handleFeedbackProcessed',
      error: err.message,
    });
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SESSION END CONSOLIDATION
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Handle SESSION_ENDED event
 * Consolidates SharedMemory patterns and saves state
 *
 * @param {Object} event - CYNIC Event
 * @param {Object} dependencies - Dependencies
 * @param {Object} dependencies.sharedMemory - SharedMemory instance
 * @param {Object} dependencies.persistence - Persistence manager
 * @param {Function} [dependencies.saveState] - saveState function from collective-singleton
 */
async function handleSessionEnded(event, dependencies) {
  const { sharedMemory, persistence, saveState } = dependencies;

  log.info('Session ended - consolidating memory', {
    sessionId: event.payload?.sessionId,
    duration: event.payload?.duration,
  });

  try {
    // 1. Save SharedMemory state
    if (sharedMemory?.save) {
      await withRetry(
        () => sharedMemory.save(),
        'SharedMemory save'
      );
      log.debug('SharedMemory saved');
    }

    // 2. Call saveState if provided (saves patterns, Q-Learning, etc.)
    if (saveState && persistence) {
      await withRetry(
        () => saveState(persistence),
        'Collective state save'
      );
      log.debug('Collective state saved');
    }

    // 3. Run memory consolidation if available
    if (sharedMemory?.consolidate) {
      const consolidated = await sharedMemory.consolidate();
      log.debug('Memory consolidated', { consolidated });
    }

    _stats.sessionEndConsolidations++;
    log.info('Session end consolidation complete');
  } catch (err) {
    log.error('Session end consolidation failed', { error: err.message });
    globalEventBus.publish(EventType.COMPONENT_ERROR, {
      component: 'EventListeners',
      operation: 'handleSessionEnded',
      error: err.message,
    });
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// DOG EVENT PERSISTENCE (AXE 2+)
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Handle DOG_EVENT - individual dog invocations, blocks, warnings
 * Persists to dog_events table
 */
async function handleDogEvent(event, persistence, context) {
  const { dog, eventType, stats, health, details } = event.payload || {};
  if (!dog || !eventType) return;

  try {
    await withRetry(async () => {
      await persistence.query(
        `INSERT INTO dog_events (dog_name, event_type, stats, health, details, session_id)
         VALUES ($1, $2, $3, $4, $5, $6)`,
        [dog, eventType, JSON.stringify(stats || {}), health, JSON.stringify(details || {}), context.sessionId]
      );
    }, `Persist dog event ${dog}:${eventType}`);
    _stats.dogEventsPersisted++;
  } catch (err) {
    _stats.dogEventsFailed++;
    globalEventBus.publish(EventType.COMPONENT_ERROR, {
      component: 'EventListeners',
      operation: 'handleDogEvent',
      error: err.message,
    });
  }
}

/**
 * Handle CONSENSUS_COMPLETED - consensus results with vote breakdown
 * Persists to consensus_votes table
 */
async function handleConsensusCompleted(event, persistence, context) {
  const { consensusId, topic, approved, agreement, guardianVeto, votes, stats: voteStats, reason } = event.payload || {};
  if (!consensusId) return;

  try {
    await withRetry(async () => {
      await persistence.query(
        `INSERT INTO consensus_votes (consensus_id, topic, approved, agreement, guardian_veto, votes, stats, reason, session_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)`,
        [consensusId, topic, approved, agreement, guardianVeto || false, JSON.stringify(votes || {}), JSON.stringify(voteStats || {}), reason, context.sessionId]
      );
    }, `Persist consensus ${consensusId}`);
    _stats.consensusPersisted++;
  } catch (err) {
    _stats.consensusFailed++;
    globalEventBus.publish(EventType.COMPONENT_ERROR, {
      component: 'EventListeners',
      operation: 'handleConsensusCompleted',
      error: err.message,
    });
  }
}

/**
 * Handle DogSignal events - inter-dog communication
 * Persists to dog_signals table
 */
async function handleDogSignal(event, persistence, context) {
  const signalType = event.type;
  const { source, ...payload } = event.payload || {};

  try {
    await withRetry(async () => {
      await persistence.query(
        `INSERT INTO dog_signals (signal_type, source_dog, payload, session_id)
         VALUES ($1, $2, $3, $4)`,
        [signalType, source || null, JSON.stringify(payload), context.sessionId]
      );
    }, `Persist dog signal ${signalType}`);
    _stats.dogSignalsPersisted++;
  } catch (err) {
    _stats.dogSignalsFailed++;
    // Non-critical, don't emit error event for high-frequency signals
    log.debug('Dog signal persistence failed', { signalType, error: err.message });
  }
}

/**
 * Handle CYNIC_STATE - periodic collective health snapshots
 * Sampled: only persists every 5th emission to avoid table bloat
 */
async function handleCynicState(event, persistence, context, repositories) {
  _cynicStateCounter++;
  if (_cynicStateCounter % 5 !== 0) return; // Sample every 5th

  const { collective, memory } = event.payload || {};
  if (!collective) return;

  try {
    await withRetry(async () => {
      await persistence.query(
        `INSERT INTO collective_snapshots (active_dogs, dog_count, average_health, health_rating, pattern_count, memory_load, memory_freshness, snapshot_data, session_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)`,
        [
          collective.activeDogs, collective.dogCount, collective.averageHealth,
          collective.healthRating, memory?.patternCount || 0, memory?.load || 0,
          memory?.freshness || 0, JSON.stringify(event.payload), context.sessionId,
        ]
      );
    }, 'Persist collective snapshot');
    _stats.snapshotsPersisted++;
  } catch (err) {
    _stats.snapshotsFailed++;
    log.debug('Snapshot persistence failed', { error: err.message });
  }

  // Migration 033: Persist thermodynamic snapshot (piggyback on CYNIC_STATE sampling)
  try {
    const thermo = getThermodynamicState();
    if (thermo && persistence) {
      const stats = thermo.getStats();
      const ok = await persistThermodynamics(persistence, context.sessionId, {
        heat: stats.heat,
        work: stats.work,
        temperature: stats.temperature,
        efficiency: stats.efficiency,
        entropy: stats.events?.total || 0,
      });
      if (ok) _stats.thermoPersisted++;
      else _stats.thermoFailed++;
    }
  } catch (thermoErr) {
    _stats.thermoFailed++;
    log.debug('Thermodynamic snapshot persistence failed', { error: thermoErr.message });
  }

  // Migration 033: Persist consciousness state from collective data
  try {
    if (persistence && collective) {
      const ok = await persistConsciousnessTransition(persistence, context.sessionId, {
        awarenessLevel: collective.averageHealth || 0,
        state: collective.healthRating || 'DORMANT',
        avgConfidence: collective.averageHealth || 0,
        patternCount: event.payload?.memory?.patternCount || 0,
        predictionAccuracy: 0,
      });
      if (ok) _stats.consciousnessPersisted++;
      else _stats.consciousnessFailed++;
    }
  } catch (consErr) {
    _stats.consciousnessFailed++;
    log.debug('Consciousness transition persistence failed', { error: consErr.message });
  }

  // P-GAP-5: Bridge local psychology file → PostgreSQL psychology_snapshots
  // Uses PsychologyRepository.recordSnapshot() which handles user FK via _ensureUserExists().
  // Raw SQL was failing silently because context.userId is not a UUID.
  try {
    if (repositories?.psychology?.recordSnapshot) {
      const { readFileSync, existsSync } = await import('fs');
      const { join } = await import('path');
      const { homedir } = await import('os');
      const psyPath = join(homedir(), '.cynic', 'psychology', 'state.json');
      if (existsSync(psyPath)) {
        const psyState = JSON.parse(readFileSync(psyPath, 'utf8'));
        // Only persist if recently updated (within 10 min)
        if (psyState.updatedAt && (Date.now() - psyState.updatedAt) < 10 * 60 * 1000) {
          const dims = psyState.dimensions || {};
          await repositories.psychology.recordSnapshot(
            context.userId || 'default',
            {
              energy: dims.energy?.value ?? 0.5,
              focus: dims.focus?.value ?? 0.5,
              creativity: dims.creativity?.value ?? 0.5,
              frustration: dims.frustration?.value ?? 0.2,
            },
            {
              sessionId: context.sessionId || null,
              workDone: psyState.temporal?.sessionDuration || 0,
              heatGenerated: 0,
              errorCount: 0,
            }
          );
          _stats.burnoutPersisted = (_stats.burnoutPersisted || 0) + 1;
        }
      }
    }
  } catch (burnoutErr) {
    _stats.burnoutFailed = (_stats.burnoutFailed || 0) + 1;
    log.debug('Psychology snapshot persistence failed', { error: burnoutErr.message });
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// BLOCK ANCHORING PERSISTENCE (PHASE 2: DECENTRALIZE)
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Handle BLOCK_FINALIZED event
 * Persists finalized block to blocks table via BlockStore
 */
async function handleBlockFinalized(event, blockStore) {
  if (!blockStore) return;

  const { blockHash, slot, block } = event.payload || event;
  if (slot === undefined) return;

  try {
    await withRetry(async () => {
      await blockStore.storeBlock({
        slot,
        hash: blockHash || block?.hash,
        proposer: block?.proposer || 'unknown',
        merkle_root: block?.merkleRoot || block?.judgments_root,
        judgments: block?.judgments,
        judgment_count: block?.judgmentCount || block?.judgments?.length || 0,
        prev_hash: block?.parentHash || block?.prev_hash,
        timestamp: block?.timestamp || Date.now(),
      });
    }, `Persist finalized block ${slot}`);
    _stats.blocksFinalizedPersisted++;
  } catch (err) {
    _stats.blocksFinalizedFailed++;
    globalEventBus.publish(EventType.COMPONENT_ERROR, {
      component: 'EventListeners',
      operation: 'handleBlockFinalized',
      error: err.message,
      slot,
    });
  }
}

/**
 * Handle BLOCK_ANCHORED event
 * Persists successful anchor to block_anchors table via BlockStore
 */
async function handleBlockAnchored(event, blockStore) {
  if (!blockStore) return;

  const { slot, signature, merkleRoot, cluster } = event.payload || event;
  if (slot === undefined) return;

  try {
    await withRetry(async () => {
      await blockStore.storeAnchor({
        slot,
        txSignature: signature,
        status: 'confirmed',
        merkleRoot,
        cluster,
      });
    }, `Persist block anchor ${slot}`);
    _stats.blocksAnchoredPersisted++;
  } catch (err) {
    _stats.blocksAnchoredFailed++;
    globalEventBus.publish(EventType.COMPONENT_ERROR, {
      component: 'EventListeners',
      operation: 'handleBlockAnchored',
      error: err.message,
      slot,
    });
  }
}

/**
 * Handle anchor:failed event
 * Persists failed anchor to block_anchors table via BlockStore
 */
async function handleAnchorFailed(event, blockStore) {
  if (!blockStore) return;

  const { slot, retryCount } = event.payload || event;
  if (slot === undefined) return;

  try {
    await withRetry(async () => {
      await blockStore.storeAnchor({
        slot,
        txSignature: null,
        status: 'failed',
        retryCount,
      });
    }, `Persist anchor failure ${slot}`);
    _stats.anchorFailuresPersisted++;
  } catch (err) {
    _stats.anchorFailuresFailed++;
    log.debug('Anchor failure persistence failed', { slot, error: err.message });
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// DOG SPECIALIST EVENT PERSISTENCE (Fix #5)
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Handle THREAT_BLOCKED from AgentEventBus (Guardian dog)
 * Persists to dog_signals table for security audit trail
 */
async function handleThreatBlocked(event, persistence, context) {
  const { threatType, riskLevel, action, reason } = event.payload || {};
  try {
    await withRetry(async () => {
      await persistence.query(
        `INSERT INTO dog_signals (signal_type, source_dog, payload, session_id)
         VALUES ($1, $2, $3, $4)`,
        ['agent:threat:blocked', event.source || 'guardian',
         JSON.stringify({ threatType, riskLevel, action, reason }), context.sessionId]
      );
    }, 'Persist threat blocked');
    _stats.dogSignalsPersisted++;
  } catch (err) {
    _stats.dogSignalsFailed++;
    log.debug('Threat blocked persistence failed', { error: err.message });
  }
}

/**
 * Handle QUALITY_REPORT from AgentEventBus (Janitor dog)
 * Persists to dog_events table for code quality tracking
 */
async function handleQualityReport(event, persistence, context) {
  const { score, issues, suggestions, filesAnalyzed } = event.payload || {};
  try {
    await withRetry(async () => {
      await persistence.query(
        `INSERT INTO dog_events (dog_name, event_type, stats, health, details, session_id)
         VALUES ($1, $2, $3, $4, $5, $6)`,
        ['janitor', 'quality_report',
         JSON.stringify({ score, issueCount: (issues || []).length, filesAnalyzed }),
         score > 70 ? 'healthy' : 'degraded',
         JSON.stringify({ issues, suggestions }), context.sessionId]
      );
    }, 'Persist quality report');
    _stats.dogEventsPersisted++;
  } catch (err) {
    _stats.dogEventsFailed++;
    log.debug('Quality report persistence failed', { error: err.message });
  }
}

/**
 * Handle VULNERABILITY_DETECTED from AgentEventBus (Scout dog)
 * Persists to dog_signals table for security tracking
 */
async function handleVulnerabilityDetected(event, persistence, context) {
  const { severity, type, description, file, remediation, cveId } = event.payload || {};
  try {
    await withRetry(async () => {
      await persistence.query(
        `INSERT INTO dog_signals (signal_type, source_dog, payload, session_id)
         VALUES ($1, $2, $3, $4)`,
        ['agent:vulnerability:detected', event.source || 'scout',
         JSON.stringify({ severity, type, description, file, remediation, cveId }),
         context.sessionId]
      );
    }, 'Persist vulnerability detected');
    _stats.dogSignalsPersisted++;
  } catch (err) {
    _stats.dogSignalsFailed++;
    log.debug('Vulnerability persistence failed', { error: err.message });
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Start event listeners
 *
 * Subscribes to:
 * - JUDGMENT_CREATED (globalEventBus)
 * - SESSION_ENDED (globalEventBus)
 *
 * Also returns a function to wire FeedbackProcessor events.
 *
 * @param {Object} options - Configuration
 * @param {Object} options.persistence - PersistenceManager instance
 * @param {Object} options.repositories - Pre-created repositories (or will be created from persistence)
 * @param {Object} [options.sharedMemory] - SharedMemory instance
 * @param {Function} [options.saveState] - saveState function from collective-singleton
 * @param {Object} [options.judge] - CYNICJudge instance (for feeding feedback back to Bayesian trackers)
 * @param {string} [options.sessionId] - Current session ID
 * @param {string} [options.userId] - Current user ID
 * @returns {Object} Control object with wireFeedbackProcessor and stop functions
 */
export function startEventListeners(options = {}) {
  if (_started) {
    log.debug('Event listeners already started');
    return {
      wireFeedbackProcessor: () => {},
      stop: stopEventListeners,
      getStats: () => ({ ..._stats }),
    };
  }

  const {
    persistence,
    repositories: providedRepos,
    sharedMemory,
    saveState,
    judge,
    collectivePack,
    sessionId,
    userId,
    blockStore,
    // RIGHT side singletons (DECIDE/ACT/ACCOUNT)
    codeDecider,
    cynicAccountant,
    codeAccountant,
    humanActor,
    // Solana pipeline singletons (C2.2-C2.7)
    solanaJudge,
    solanaDecider,
    solanaActor,
    solanaLearner,
    solanaAccountant,
    solanaEmergence,
    // Emergence pipeline singletons (C1.7, C5.7)
    codeEmergence,
    humanEmergence,
  } = options;

  // Get or create repositories
  let repositories = providedRepos;
  if (!repositories && persistence) {
    try {
      // Try to get repositories from persistence factory
      if (typeof persistence.getRepository === 'function') {
        repositories = {
          judgments: persistence.getRepository('judgments'),
          feedback: persistence.getRepository('feedback'),
          sessions: persistence.getRepository('sessions'),
          psychology: persistence.getRepository('psychology'),
        };
      } else if (persistence.repositories) {
        repositories = persistence.repositories;
      }
    } catch (err) {
      log.warn('Could not get repositories from persistence', { error: err.message });
    }
  }

  if (!repositories) {
    // Fallback: file-backed repos so the full learning chain survives without PostgreSQL
    try {
      repositories = {
        judgments: createFileBackedRepo('judgments'),
        feedback: createFileBackedRepo('feedback'),
        patterns: createFileBackedRepo('patterns'),
        knowledge: createFileBackedRepo('knowledge'),
        patternEvolution: createFileBackedRepo('pattern-evolution'),
      };
      log.info('Using file-backed repos for learning pipeline (no PostgreSQL)');
    } catch (err) {
      log.warn('No repositories available - event listeners will be no-ops', { error: err.message });
      repositories = {};
    }
  }

  const context = { sessionId, userId };

  // ─────────────────────────────────────────────────────────────────────────────
  // Subscribe to JUDGMENT_CREATED
  // ─────────────────────────────────────────────────────────────────────────────
  const unsubJudgment = globalEventBus.subscribe(
    EventType.JUDGMENT_CREATED,
    (event) => {
      // Non-blocking - fire and forget with error handling
      handleJudgmentCreated(event, repositories, context).catch((err) => {
        log.error('Judgment handler threw unexpectedly', { error: err.message });
      });
    }
  );
  _unsubscribers.push(unsubJudgment);

  // ─────────────────────────────────────────────────────────────────────────────
  // Subscribe to SESSION_ENDED
  // ─────────────────────────────────────────────────────────────────────────────
  const unsubSession = globalEventBus.subscribe(
    EventType.SESSION_ENDED,
    (event) => {
      handleSessionEnded(event, { sharedMemory, persistence, saveState }).catch((err) => {
        log.error('Session end handler threw unexpectedly', { error: err.message });
      });
    }
  );
  _unsubscribers.push(unsubSession);

  // ─────────────────────────────────────────────────────────────────────────────
  // Wire USER_FEEDBACK events (from hook feedback)
  // WS1: Close the feedback loop — feedback now feeds BACK to Judge's Bayesian trackers
  // ─────────────────────────────────────────────────────────────────────────────
  const unsubUserFeedback = globalEventBus.subscribe(
    EventType.USER_FEEDBACK,
    (event) => {
      const feedbackData = event.payload || {};

      // 1. Persist feedback to DB (existing behavior)
      handleFeedbackProcessed(
        { scoreDelta: feedbackData.scoreDelta || 0 },
        repositories,
        context,
        feedbackData
      ).catch((err) => {
        log.error('User feedback handler threw unexpectedly', { error: err.message });
      });

      // 2. J-GAP-1: Update CalibrationTracker with actual outcome
      // Closes the loop: Judge.record(predicted) → Feedback → updateActual(outcome)
      if (judge?.calibrationTracker?.updateActual && feedbackData.judgmentId) {
        judge.calibrationTracker.updateActual(
          feedbackData.judgmentId,
          feedbackData.outcome || feedbackData.verdict || 'unknown'
        ).catch((err) => {
          log.debug('CalibrationTracker.updateActual failed (non-blocking)', { error: err.message });
        });
      }

      // 3. WS1: Feed back to Judge's Bayesian trackers
      // This closes the loop: Judge → Judgment → Feedback → Judge learns
      if (judge?.recordFeedback) {
        try {
          judge.recordFeedback({
            judgment: {
              id: feedbackData.judgmentId,
              item: feedbackData.item || { type: feedbackData.itemType || 'unknown' },
              dimensions: feedbackData.dimensions,
              dimensionScores: feedbackData.dimensionScores,
              qScore: feedbackData.qScore,
              global_score: feedbackData.qScore,
            },
            outcome: feedbackData.outcome || 'partial',
            actualScore: feedbackData.actualScore,
            dimensions: feedbackData.dimensionCorrections,
          });
          log.debug('Feedback fed to Judge Bayesian trackers', {
            judgmentId: feedbackData.judgmentId,
            outcome: feedbackData.outcome,
          });
        } catch (err) {
          log.debug('Judge.recordFeedback failed (non-blocking)', { error: err.message });
        }
      }
    }
  );
  _unsubscribers.push(unsubUserFeedback);

  // ─────────────────────────────────────────────────────────────────────────────
  // Subscribe to DOG_EVENT (AXE 2+)
  // ─────────────────────────────────────────────────────────────────────────────
  if (persistence?.query) {
    const unsubDogEvent = globalEventBus.subscribe(
      EventType.DOG_EVENT,
      (event) => {
        handleDogEvent(event, persistence, context).catch((err) => {
          log.error('Dog event handler threw unexpectedly', { error: err.message });
        });
      }
    );
    _unsubscribers.push(unsubDogEvent);

    // ─────────────────────────────────────────────────────────────────────────────
    // Subscribe to CONSENSUS_COMPLETED (AXE 2+)
    // ─────────────────────────────────────────────────────────────────────────────
    const unsubConsensus = globalEventBus.subscribe(
      EventType.CONSENSUS_COMPLETED,
      (event) => {
        handleConsensusCompleted(event, persistence, context).catch((err) => {
          log.error('Consensus handler threw unexpectedly', { error: err.message });
        });
      }
    );
    _unsubscribers.push(unsubConsensus);

    // ─────────────────────────────────────────────────────────────────────────────
    // Subscribe to DogSignal events (AXE 2+)
    // ─────────────────────────────────────────────────────────────────────────────
    for (const signalType of Object.values(DogSignal)) {
      const unsubSignal = globalEventBus.subscribe(
        signalType,
        (event) => {
          handleDogSignal(event, persistence, context).catch((err) => {
            log.debug('Dog signal handler error', { error: err.message });
          });
        }
      );
      _unsubscribers.push(unsubSignal);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Subscribe to CYNIC_STATE (AXE 2+ - sampled every 5th)
    // ─────────────────────────────────────────────────────────────────────────────
    const unsubCynicState = globalEventBus.subscribe(
      EventType.CYNIC_STATE,
      (event) => {
        handleCynicState(event, persistence, context, repositories).catch((err) => {
          log.debug('CYNIC state handler error', { error: err.message });
        });
      }
    );
    _unsubscribers.push(unsubCynicState);

    log.info('Dog collective event listeners wired (AXE 2+)');
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Subscribe to BLOCK_FINALIZED (PHASE 2: DECENTRALIZE)
  // ─────────────────────────────────────────────────────────────────────────────
  if (blockStore) {
    const unsubBlockFinalized = globalEventBus.subscribe(
      EventType.BLOCK_FINALIZED,
      (event) => {
        handleBlockFinalized(event, blockStore).catch((err) => {
          log.error('Block finalized handler threw unexpectedly', { error: err.message });
        });
      }
    );
    _unsubscribers.push(unsubBlockFinalized);

    // ─────────────────────────────────────────────────────────────────────────────
    // Subscribe to BLOCK_ANCHORED (PHASE 2: DECENTRALIZE)
    // ─────────────────────────────────────────────────────────────────────────────
    const unsubBlockAnchored = globalEventBus.subscribe(
      EventType.BLOCK_ANCHORED,
      (event) => {
        handleBlockAnchored(event, blockStore).catch((err) => {
          log.error('Block anchored handler threw unexpectedly', { error: err.message });
        });
      }
    );
    _unsubscribers.push(unsubBlockAnchored);

    // ─────────────────────────────────────────────────────────────────────────────
    // Subscribe to anchor:failed (PHASE 2: DECENTRALIZE)
    // ─────────────────────────────────────────────────────────────────────────────
    const unsubAnchorFailed = globalEventBus.subscribe(
      'anchor:failed',
      (event) => {
        handleAnchorFailed(event, blockStore).catch((err) => {
          log.debug('Anchor failed handler error', { error: err.message });
        });
      }
    );
    _unsubscribers.push(unsubAnchorFailed);

    log.info('Block anchoring event listeners wired (PHASE 2)');
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Subscribe to DIMENSION_CANDIDATE (WS6: Dimension Discovery Governance)
  // When ResidualDetector finds a candidate dimension, trigger Dog vote
  // ─────────────────────────────────────────────────────────────────────────────
  if (collectivePack?.consensus) {
    const unsubDimCandidate = globalEventBus.subscribe(
      EventType.DIMENSION_CANDIDATE,
      async (event) => {
        const candidate = event.candidate || event;
        try {
          const result = await collectivePack.consensus.triggerConsensus({
            topic: `dimension_governance:${candidate.key}`,
            context: {
              candidateKey: candidate.key,
              suggestedName: candidate.suggestedName,
              suggestedAxiom: candidate.suggestedAxiom,
              sampleCount: candidate.sampleCount,
              avgResidual: candidate.avgResidual,
              confidence: candidate.confidence,
              weakDimensions: candidate.weakDimensions,
            },
            reason: `New dimension candidate "${candidate.suggestedName}" detected (${candidate.sampleCount} samples, residual ${(candidate.avgResidual * 100).toFixed(1)}%)`,
          });

          // Apply governance decision
          if (result?.approved && collectivePack.judgeComponent?.residualDetector) {
            const detector = collectivePack.judgeComponent.residualDetector;
            detector.acceptCandidate(candidate.key, {
              name: candidate.suggestedName,
              axiom: candidate.suggestedAxiom,
              weight: 1.0,
            });
            log.info('Dimension candidate ACCEPTED by Dogs', { name: candidate.suggestedName, axiom: candidate.suggestedAxiom });
          } else if (collectivePack.judgeComponent?.residualDetector) {
            collectivePack.judgeComponent.residualDetector.rejectCandidate(candidate.key);
            log.info('Dimension candidate REJECTED by Dogs', { name: candidate.suggestedName });
          }
        } catch (err) {
          log.debug('Dimension governance failed (non-blocking)', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubDimCandidate);
    log.info('Dimension governance listener wired (WS6)');
  }

  // Fix #6: EmergenceDetector significant patterns → persistence
  // EmergenceDetector publishes PATTERN_DETECTED on globalEventBus (bridged via collective-singleton).
  // Persist HIGH/CRITICAL significance patterns to unified_signals for learning loops.
  if (persistence?.query) {
    const unsubPattern = globalEventBus.subscribe(EventType.PATTERN_DETECTED, (event) => {
      try {
        const d = event.data || event.payload || {};
        if (!d.source || d.source !== 'EmergenceDetector') return; // Only persist emergence patterns
        const significance = d.significance || 'low';
        if (significance !== 'high' && significance !== 'critical') return;

        const id = `ep_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
        persistence.query(`
          INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
          VALUES ($1, $2, $3, $4, $5, $6)
        `, [
          id,
          'emergence_pattern',
          context.sessionId || null,
          JSON.stringify({ category: d.category, key: d.key }),
          JSON.stringify({ status: significance, reason: d.subject }),
          JSON.stringify({ occurrences: d.occurrences, confidence: d.confidence }),
        ]).catch(err => {
          log.debug('Emergence pattern persistence failed', { error: err.message });
        });
        _stats.emergencePatternsPersisted = (_stats.emergencePatternsPersisted || 0) + 1;
        log.info('Emergence pattern persisted', { category: d.category, key: d.key, significance, occurrences: d.occurrences });
      } catch (e) { log.debug('Emergence pattern handler error', { error: e.message }); }
    });
    _unsubscribers.push(unsubPattern);
    log.info('Emergence pattern listener wired (Fix #6)');
  }

  // SOCIAL: Persist social capture events → unified_signals
  // XProxyService and XIngest emit SOCIAL_CAPTURE on globalEventBus.
  // Persist to unified_signals so learning loops can analyze social patterns.
  if (persistence?.query) {
    const unsubSocial = globalEventBus.subscribe(EventType.SOCIAL_CAPTURE, (event) => {
      try {
        const d = event.data || event.payload || {};
        const id = `sc_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
        persistence.query(`
          INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
          VALUES ($1, $2, $3, $4, $5, $6)
        `, [
          id,
          'social_capture',
          context.sessionId || null,
          JSON.stringify({ source: d.source, tweets: d.tweets, users: d.users }),
          JSON.stringify({ status: 'captured' }),
          JSON.stringify({ timestamp: Date.now() }),
        ]).catch(err => {
          log.debug('Social capture persistence failed', { error: err.message });
        });
        _stats.socialCapturesPersisted = (_stats.socialCapturesPersisted || 0) + 1;
      } catch (e) { log.debug('Social capture handler error', { error: e.message }); }
    });
    _unsubscribers.push(unsubSocial);
    log.info('Social capture listener wired');
  }

  // P2-B: Persist routing decisions from KabbalisticRouter → orchestration_log
  // Router emits ORCHESTRATION_COMPLETED after each routing decision.
  // This closes the data grave: orchestration_log was read by router but never written by it.
  if (persistence?.query) {
    const unsubOrch = globalEventBus.subscribe(EventType.ORCHESTRATION_COMPLETED, (event) => {
      try {
        const d = event.data || event.payload || {};
        persistence.query(`
          INSERT INTO orchestration_log
            (event_type, sefirah, outcome, domain, suggested_agent, trace, duration_ms, session_id)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        `, [
          'routing_decision',
          d.entrySefirah || 'unknown',
          d.success ? (d.blocked ? 'BLOCK' : 'ALLOW') : 'ERROR',
          d.taskType || 'unknown',
          d.entrySefirah || null,
          JSON.stringify({
            path: d.path,
            hasConsensus: d.hasConsensus,
            confidence: d.confidence,
            consultationCount: d.consultationCount,
            thompsonExplored: d.thompsonExplored,
          }),
          d.durationMs || null,
          context.sessionId || null,
        ]).catch(err => {
          log.debug('Orchestration log persistence failed', { error: err.message });
        });
        _stats.orchestrationDecisionsPersisted = (_stats.orchestrationDecisionsPersisted || 0) + 1;
      } catch (e) { log.debug('Orchestration completed handler error', { error: e.message }); }
    });
    _unsubscribers.push(unsubOrch);
    log.info('Orchestration completed listener wired (P2-B)');
  }

  // Fix #4: Automation Bus Orphan Logging
  // AutomationExecutor publishes events to getEventBus() with zero subscribers.
  // Add basic INFO-level logging for production visibility.
  try {
    const ab = getEventBus();
    _unsubscribers.push(ab.subscribe(AutomationEventType.LEARNING_CYCLE_COMPLETE, (event) => {
      try {
        const d = event.data || event.payload || {};
        log.info('Learning cycle completed (automation bus)', { trigger: d.trigger, cycleNumber: d.cycleNumber });
      } catch (e) { log.debug('Learning cycle log error', { error: e.message }); }
    }));
    _unsubscribers.push(ab.subscribe(AutomationEventType.TRIGGER_FIRED, (event) => {
      try {
        const d = event.data || event.payload || {};
        log.info('Trigger fired (automation bus)', { triggerId: d.triggerId, name: d.name });
      } catch (e) { log.debug('Trigger fired log error', { error: e.message }); }
    }));
    _unsubscribers.push(ab.subscribe(AutomationEventType.GOAL_COMPLETED, (event) => {
      try {
        const d = event.data || event.payload || {};
        log.info('Goal completed (automation bus)', { goal: d.goal });
      } catch (e) { log.debug('Goal completed log error', { error: e.message }); }
    }));
    // Fix #4b: Bridge data_grave_analysis findings → unified_signals
    // AutomationExecutor publishes findings as AUTOMATION_TICK with subType 'data_grave_analysis'.
    // Persist high-severity findings to unified_signals so they feed into learning loops.
    _unsubscribers.push(ab.subscribe(AutomationEventType.AUTOMATION_TICK, (event) => {
      try {
        const d = event.data || event.payload || {};
        if (d.subType !== 'data_grave_analysis' || !d.findings?.length) return;
        if (!persistence?.query) return;

        const highFindings = d.findings.filter(f => f.severity === 'high' || f.severity === 'critical');
        if (highFindings.length === 0) return;

        // Persist each high-severity finding as a unified signal
        for (const finding of highFindings) {
          const id = `dgf_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          persistence.query(`
            INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
          `, [
            id,
            'data_grave_analysis',
            context.sessionId || null,
            JSON.stringify({ findingType: finding.type }),
            JSON.stringify({ status: finding.severity, reason: finding.message }),
            JSON.stringify({ timestamp: d.timestamp, totalFindings: d.findings.length }),
          ]).catch(err => {
            log.debug('Data grave finding persistence failed', { error: err.message });
          });
        }
        _stats.dataGraveFindingsPersisted = (_stats.dataGraveFindingsPersisted || 0) + highFindings.length;
        log.info('Data grave findings persisted to unified_signals', { count: highFindings.length, types: highFindings.map(f => f.type) });
      } catch (e) { log.debug('Data grave findings bridge error', { error: e.message }); }
    }));
    // AXE 6: Bridge distribution_snapshot → distribution_snapshots table
    // AutomationExecutor check #11 publishes ecosystem health as AUTOMATION_TICK with subType 'distribution_snapshot'.
    _unsubscribers.push(ab.subscribe(AutomationEventType.AUTOMATION_TICK, (event) => {
      try {
        const d = event.data || event.payload || {};
        if (d.subType !== 'distribution_snapshot') return;
        if (!persistence?.query) return;

        const health = d.health || {};
        persistence.query(`
          INSERT INTO distribution_snapshots (health_report, e_score, total_sources, fetched_sources, stale_sources, error_sources)
          VALUES ($1, $2, $3, $4, $5, $6)
        `, [
          JSON.stringify(health),
          d.eScore || 0,
          health.total || 0,
          health.fetched || 0,
          health.stale || 0,
          health.errors || 0,
        ]).catch(err => {
          log.debug('Distribution snapshot persistence failed', { error: err.message });
        });
        _stats.distributionSnapshotsPersisted = (_stats.distributionSnapshotsPersisted || 0) + 1;
      } catch (e) { log.debug('Distribution snapshot bridge error', { error: e.message }); }
    }));

    // Fix #4c: Wire remaining automation bus orphans (LEARNING_CYCLE_START, TRIGGER_EVALUATED, AUTOMATION_START/STOP)
    _unsubscribers.push(ab.subscribe(AutomationEventType.LEARNING_CYCLE_START, (event) => {
      try {
        const d = event.data || event.payload || {};
        log.info('Learning cycle started (automation bus)', { trigger: d.trigger, scheduleType: d.scheduleType });
      } catch (e) { log.debug('Learning cycle start log error', { error: e.message }); }
    }));
    _unsubscribers.push(ab.subscribe(AutomationEventType.TRIGGER_EVALUATED, (event) => {
      try {
        const d = event.data || event.payload || {};
        log.info('Trigger evaluated (automation bus)', { triggerId: d.triggerId, fired: d.fired, reason: d.reason });
      } catch (e) { log.debug('Trigger evaluated log error', { error: e.message }); }
    }));
    _unsubscribers.push(ab.subscribe(AutomationEventType.AUTOMATION_START, (event) => {
      try {
        const d = event.data || event.payload || {};
        log.info('Automation executor started', { intervals: d.intervals, triggerCount: d.triggerCount });
      } catch (e) { log.debug('Automation start log error', { error: e.message }); }
    }));
    _unsubscribers.push(ab.subscribe(AutomationEventType.AUTOMATION_STOP, (event) => {
      try {
        log.info('Automation executor stopped');
      } catch (e) { log.debug('Automation stop log error', { error: e.message }); }
    }));

    log.info('Automation bus orphan listeners wired (Fix #4 + 4b + 4c + AXE 6)');
  } catch (err) {
    log.debug('Automation bus listeners skipped', { error: err.message });
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Fix #5: Dog Specialist Events -> Persistence
  // Subscribe to THREAT_BLOCKED, QUALITY_REPORT, VULNERABILITY_DETECTED on
  // AgentEventBus. These events are emitted by Guardian, Janitor, and Scout
  // dogs but had zero persistence subscribers.
  //
  // AgentEventBus.subscribe() requires a registered agent, so we register a
  // lightweight 'event-listeners' agent and subscribe through the formal API.
  // ─────────────────────────────────────────────────────────────────────────────
  if (persistence?.query && collectivePack?.eventBus) {
    try {
      const agentBus = collectivePack.eventBus;

      // Register a lightweight listener agent if not already registered
      if (!agentBus.isAgentRegistered('event-listeners')) {
        agentBus.registerAgent('event-listeners');
      }

      // THREAT_BLOCKED -> dog_signals
      const threatSubId = agentBus.subscribe(
        AgentEvent.THREAT_BLOCKED,
        'event-listeners',
        (event) => {
          handleThreatBlocked(event, persistence, context).catch((err) => {
            log.debug('Threat blocked handler error', { error: err.message });
          });
        }
      );
      _unsubscribers.push(() => agentBus.unsubscribe(threatSubId));

      // QUALITY_REPORT -> dog_events
      const qualitySubId = agentBus.subscribe(
        AgentEvent.QUALITY_REPORT,
        'event-listeners',
        (event) => {
          handleQualityReport(event, persistence, context).catch((err) => {
            log.debug('Quality report handler error', { error: err.message });
          });
        }
      );
      _unsubscribers.push(() => agentBus.unsubscribe(qualitySubId));

      // VULNERABILITY_DETECTED -> dog_signals
      const vulnSubId = agentBus.subscribe(
        AgentEvent.VULNERABILITY_DETECTED,
        'event-listeners',
        (event) => {
          handleVulnerabilityDetected(event, persistence, context).catch((err) => {
            log.debug('Vulnerability handler error', { error: err.message });
          });
        }
      );
      _unsubscribers.push(() => agentBus.unsubscribe(vulnSubId));

      log.info('Dog specialist event listeners wired (Fix #5)', {
        events: ['THREAT_BLOCKED', 'QUALITY_REPORT', 'VULNERABILITY_DETECTED'],
      });
    } catch (err) {
      log.debug('Dog specialist listeners skipped', { error: err.message });
    }
  }

  // ═════════════════════════════════════════════════════════════════════════════
  // SOLANA PIPELINE WIRING (C2.2-C2.7)
  // "On-chain is truth" — SolanaWatcher → Judge → Decide → Act → Learn → Emerge
  // ═════════════════════════════════════════════════════════════════════════════

  if (solanaJudge) {
    // 2a. perception:solana:slot → SolanaJudge (Fibonacci-sampled) + SolanaEmergence
    const unsubSolanaSlot = globalEventBus.subscribe(
      'perception:solana:slot',
      (event) => {
        try {
          const slotData = event.payload || event;
          // Judge network health on every 13th slot (Fibonacci sampling)
          if (slotData.slot % 13 === 0) {
            const judgment = solanaJudge.judgeNetwork(slotData);
            _stats.solanaJudgments++;
            globalEventBus.publish('solana:judgment', {
              type: 'network',
              judgment,
              slot: slotData.slot,
            }, { source: 'event-listeners' });
          }
          // Feed emergence with activity data
          if (solanaEmergence) {
            solanaEmergence.recordActivity({
              transactionCount: slotData.numTransactions || 1,
              slot: slotData.slot,
            });
          }
        } catch (err) {
          log.debug('Solana slot handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubSolanaSlot);

    // 2b. perception:solana:account → SolanaJudge + SolanaAccountant
    const unsubSolanaAccount = globalEventBus.subscribe(
      'perception:solana:account',
      (event) => {
        try {
          const accountData = event.payload || event;
          solanaJudge.judgeAccount(accountData);
          _stats.solanaJudgments++;
          if (solanaAccountant) {
            solanaAccountant.recordTransaction({
              type: 'account_change',
              ...accountData,
            });
            _stats.solanaAccountingOps++;
          }
        } catch (err) {
          log.debug('Solana account handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubSolanaAccount);

    // 2c. solana:judgment → SolanaDecider (only on GROWL/BARK verdicts)
    if (solanaDecider) {
      const unsubSolanaJudgment = globalEventBus.subscribe(
        'solana:judgment',
        (event) => {
          try {
            const { judgment } = event.payload || {};
            if (judgment?.verdict === 'GROWL' || judgment?.verdict === 'BARK') {
              const decision = solanaDecider.decide(judgment, { source: 'auto' });
              _stats.solanaDecisions++;
              globalEventBus.publish('solana:decision', {
                decision,
                judgment,
              }, { source: 'event-listeners' });
            }
          } catch (err) {
            log.debug('Solana judgment→decision handler error', { error: err.message });
          }
        }
      );
      _unsubscribers.push(unsubSolanaJudgment);
    }

    // 2d. solana:decision → SolanaActor (DRY RUN default) + SolanaLearner
    if (solanaActor || solanaLearner) {
      const unsubSolanaDecision = globalEventBus.subscribe(
        'solana:decision',
        (event) => {
          try {
            const { decision } = event.payload || {};
            if (!decision) return;

            // DRY RUN: Only execute if SOLANA_ACTOR_LIVE=true
            const isLive = process.env.SOLANA_ACTOR_LIVE === 'true';
            if (isLive && solanaActor) {
              solanaActor.execute(decision.type || decision.action, decision.params || {}).catch(err => {
                log.debug('SolanaActor.execute failed', { error: err.message });
              });
            }
            _stats.solanaActionsRecorded++;

            // Always record outcome for learning
            if (solanaLearner) {
              solanaLearner.recordOutcome({
                type: decision.type || decision.action,
                executed: isLive,
                decision,
                timestamp: Date.now(),
              });
              _stats.solanaLearnings++;
            }
          } catch (err) {
            log.debug('Solana decision→action handler error', { error: err.message });
          }
        }
      );
      _unsubscribers.push(unsubSolanaDecision);
    }

    // 2e. Fibonacci-triggered emergence analysis (F8=21min)
    if (solanaEmergence) {
      _solanaEmergenceInterval = setInterval(() => {
        try {
          const patterns = solanaEmergence.analyze();
          if (patterns?.length > 0) {
            for (const pattern of patterns) {
              globalEventBus.publish(EventType.PATTERN_DETECTED, {
                source: 'SolanaEmergence',
                key: pattern.type || pattern.key,
                significance: pattern.significance || 'medium',
                category: 'solana',
                ...pattern,
              }, { source: 'solana-emergence' });
            }
            _stats.solanaEmergencePatterns += patterns.length;
          }
        } catch (err) {
          log.debug('Solana emergence analysis error', { error: err.message });
        }
      }, 21 * 60 * 1000); // F8 = 21 minutes
      _solanaEmergenceInterval.unref();
    }

    // 2f. Persist solana:judgment and solana:decision → unified_signals
    if (persistence?.query) {
      const unsubSolanaJudgmentPersist = globalEventBus.subscribe(
        'solana:judgment',
        (event) => {
          try {
            const d = event.payload || {};
            const id = `sj_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            persistence.query(`
              INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
              VALUES ($1, $2, $3, $4, $5, $6)
            `, [
              id, 'solana_judgment', context.sessionId || null,
              JSON.stringify({ type: d.type, slot: d.slot }),
              JSON.stringify({ verdict: d.judgment?.verdict, qScore: d.judgment?.qScore }),
              JSON.stringify({ dimensions: d.judgment?.dimensions }),
            ]).catch(err => {
              log.debug('Solana judgment persistence failed', { error: err.message });
            });
          } catch (e) { log.debug('Solana judgment persist error', { error: e.message }); }
        }
      );
      _unsubscribers.push(unsubSolanaJudgmentPersist);

      const unsubSolanaDecisionPersist = globalEventBus.subscribe(
        'solana:decision',
        (event) => {
          try {
            const d = event.payload || {};
            const id = `sd_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            persistence.query(`
              INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
              VALUES ($1, $2, $3, $4, $5, $6)
            `, [
              id, 'solana_decision', context.sessionId || null,
              JSON.stringify({ type: d.decision?.type, action: d.decision?.action }),
              JSON.stringify({ verdict: d.judgment?.verdict }),
              JSON.stringify({ decision: d.decision }),
            ]).catch(err => {
              log.debug('Solana decision persistence failed', { error: err.message });
            });
          } catch (e) { log.debug('Solana decision persist error', { error: e.message }); }
        }
      );
      _unsubscribers.push(unsubSolanaDecisionPersist);
    }

    log.info('Solana pipeline event listeners wired (C2.2-C2.7)', {
      judge: !!solanaJudge,
      decider: !!solanaDecider,
      actor: !!solanaActor,
      learner: !!solanaLearner,
      accountant: !!solanaAccountant,
      emergence: !!solanaEmergence,
    });
  }

  // ═════════════════════════════════════════════════════════════════════════════
  // RIGHT SIDE WIRING (DECIDE/ACT/ACCOUNT/EMERGE)
  // "Le chien décide, agit, et rend des comptes"
  // ═════════════════════════════════════════════════════════════════════════════

  // 3a. DOG_EVENT → CynicAccountant: Track per-Dog economics
  if (cynicAccountant && persistence?.query) {
    const unsubCynicAccounting = globalEventBus.subscribe(
      EventType.DOG_EVENT,
      (event) => {
        try {
          const { dog, eventType, stats: dogStats } = event.payload || {};
          if (!dog) return;
          cynicAccountant.trackOperation(dog, eventType || 'invocation', dogStats || {});
          _stats.cynicAccountingOps++;

          // Publish accounting update for downstream consumers
          globalEventBus.publish(EventType.ACCOUNTING_UPDATE, {
            source: 'CynicAccountant',
            dogId: dog,
            type: eventType,
          }, { source: 'event-listeners' });
        } catch (err) {
          log.debug('CynicAccountant.trackOperation failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCynicAccounting);
  }

  // 3b. TOOL_COMPLETED → CodeAccountant: Track code change economics
  if (codeAccountant) {
    const unsubCodeAccounting = globalEventBus.subscribe(
      EventType.TOOL_COMPLETED,
      (event) => {
        try {
          const { tool, result } = event.payload || {};
          // Only track code-modifying tools (Write, Edit)
          if (!tool || !['Write', 'Edit'].includes(tool)) return;
          codeAccountant.trackChange({
            tool,
            linesAdded: result?.linesAdded || 0,
            linesRemoved: result?.linesRemoved || 0,
            filePath: result?.filePath || result?.path || 'unknown',
          }, { sessionId });
          _stats.codeAccountingOps++;
        } catch (err) {
          log.debug('CodeAccountant.trackChange failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCodeAccounting);
  }

  // 3c. JUDGMENT_CREATED → CodeDecider: Run decision pipeline for code judgments
  if (codeDecider) {
    const unsubCodeDecision = globalEventBus.subscribe(
      EventType.JUDGMENT_CREATED,
      (event) => {
        try {
          const { itemType, qScore, verdict, dimensions } = event.payload || {};
          // Only trigger for code-related judgments
          if (!itemType || !['code', 'commit', 'refactor', 'deploy'].includes(itemType)) return;

          const decision = codeDecider.decide({
            itemType,
            qScore,
            verdict,
            dimensions,
            judgmentId: event.id,
          });

          if (decision) {
            _stats.codeDecisionsTriggered++;
            globalEventBus.publish(EventType.CODE_DECISION, {
              source: 'CodeDecider',
              decision: decision.decision || decision.verdict,
              reason: decision.reason,
              judgmentId: event.id,
              qScore,
            }, { source: 'event-listeners' });

            // Persist to unified_signals (same pattern as PATTERN_DETECTED)
            if (persistence?.query) {
              const id = `cd_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
              persistence.query(`
                INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
                VALUES ($1, $2, $3, $4, $5, $6)
              `, [
                id, 'code_decision', context.sessionId || null,
                JSON.stringify({ itemType, qScore, judgmentId: event.id }),
                JSON.stringify({ decision: decision.decision || decision.verdict, reason: decision.reason }),
                JSON.stringify({ verdict }),
              ]).catch(err => {
                log.debug('Code decision persistence failed', { error: err.message });
              });
            }
          }
        } catch (err) {
          log.debug('CodeDecider.decide failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCodeDecision);
  }

  // 3d. CYNIC_STATE → HumanActor: Trigger intervention on burnout risk
  // Reuses existing 1:5 CYNIC_STATE sampling (fires after handleCynicState)
  if (humanActor) {
    const unsubHumanAction = globalEventBus.subscribe(
      EventType.CYNIC_STATE,
      (event) => {
        try {
          const { psychology, collective } = event.payload || {};
          // Only check every 5th emission (same sampling as snapshots)
          if (_cynicStateCounter % 5 !== 0) return;

          const burnoutRisk = psychology?.burnoutRisk || psychology?.frustration || 0;
          const cognitiveLoad = psychology?.cognitiveLoad || 0;

          // Trigger if burnout risk exceeds φ⁻¹ or cognitive load > Miller's Law (7)
          if (burnoutRisk > 0.618 || cognitiveLoad > 7) {
            const intervention = humanActor.act({
              type: burnoutRisk > 0.618 ? 'BURNOUT_WARNING' : 'COMPLEXITY_WARNING',
              burnoutRisk,
              cognitiveLoad,
              averageHealth: collective?.averageHealth || 0,
            });

            if (intervention) {
              _stats.humanActionsTriggered++;
              globalEventBus.publish(EventType.HUMAN_ACTION, {
                source: 'HumanActor',
                actionType: intervention.type || 'intervention',
                reason: intervention.reason || 'High burnout/load detected',
              }, { source: 'event-listeners' });

              // Persist to unified_signals
              if (persistence?.query) {
                const id = `ha_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
                persistence.query(`
                  INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
                  VALUES ($1, $2, $3, $4, $5, $6)
                `, [
                  id, 'human_action', context.sessionId || null,
                  JSON.stringify({ burnoutRisk, cognitiveLoad }),
                  JSON.stringify({ actionType: intervention.type }),
                  JSON.stringify({ averageHealth: collective?.averageHealth }),
                ]).catch(err => {
                  log.debug('Human action persistence failed', { error: err.message });
                });
              }
            }
          }
        } catch (err) {
          log.debug('HumanActor.act failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubHumanAction);
  }

  // AXE 4: PATTERN_DETECTED (CRITICAL) → HumanActor: Emerge→Action loop
  if (humanActor && persistence?.query) {
    const unsubPatternAction = globalEventBus.subscribe(
      EventType.PATTERN_DETECTED,
      (event) => {
        try {
          const d = event.data || event.payload || {};
          if (d.significance !== 'critical') return;

          const intervention = humanActor.act({
            type: 'COMPLEXITY_WARNING',
            pattern: d.key || d.category,
            occurrences: d.occurrences,
            significance: d.significance,
          });

          if (intervention) {
            _stats.humanActionsTriggered++;
            globalEventBus.publish(EventType.HUMAN_ACTION, {
              source: 'HumanActor',
              actionType: 'pattern_response',
              pattern: d.key,
              reason: `Critical pattern "${d.key}" detected (${d.occurrences}x)`,
            }, { source: 'event-listeners' });
          }
        } catch (err) {
          log.debug('Pattern→HumanActor failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubPatternAction);
  }

  if (codeDecider || cynicAccountant || codeAccountant || humanActor) {
    log.info('RIGHT side event listeners wired', {
      codeDecider: !!codeDecider,
      cynicAccountant: !!cynicAccountant,
      codeAccountant: !!codeAccountant,
      humanActor: !!humanActor,
    });
  }

  // ═════════════════════════════════════════════════════════════════════════════
  // EMERGE COLUMN WIRING (C1.7 CodeEmergence, C5.7 HumanEmergence)
  // "Le chien voit les patterns que l'humain ne voit pas"
  // ═════════════════════════════════════════════════════════════════════════════

  // 4a. JUDGMENT_CREATED → CodeEmergence: Feed code judgments as change data (C1.7)
  if (codeEmergence) {
    const unsubCodeEmergenceJudgment = globalEventBus.subscribe(
      EventType.JUDGMENT_CREATED,
      (event) => {
        try {
          const j = event.payload || {};
          // Only feed code-related judgments
          if (!j.itemType && !j.item_type) return;
          const itemType = j.itemType || j.item_type || '';
          if (!itemType.includes('code') && !itemType.includes('commit') && !itemType.includes('file')) return;

          codeEmergence.recordChange({
            filePath: j.query?.filePath || j.query?.path || j.itemId || 'unknown',
            linesAdded: j.query?.linesAdded || 0,
            linesRemoved: j.query?.linesRemoved || 0,
            imports: j.query?.imports || [],
            complexityDelta: j.score ? (j.score > 60 ? -0.05 : 0.05) : 0,
          }, {
            sessionId: j.sessionId,
            userId: j.userId,
          });
          _stats.codeEmergenceChanges++;
        } catch (err) {
          log.debug('CodeEmergence judgment handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCodeEmergenceJudgment);

    // 4b. Fibonacci-triggered code emergence analysis (F7=13min)
    _codeEmergenceInterval = setInterval(() => {
      try {
        const patterns = codeEmergence.getPatterns();
        if (patterns?.length > 0) {
          for (const pattern of patterns) {
            // Only publish NEW patterns (not already emitted)
            if (!pattern._emitted) {
              globalEventBus.publish(EventType.PATTERN_DETECTED, {
                source: 'CodeEmergence',
                key: pattern.type || pattern.key,
                significance: pattern.significance || 'medium',
                category: 'code',
                ...pattern,
              }, { source: 'code-emergence' });
              pattern._emitted = true;
              _stats.codeEmergencePatterns++;
            }
          }
        }
      } catch (err) {
        log.debug('Code emergence analysis error', { error: err.message });
      }
    }, 13 * 60 * 1000); // F7 = 13 minutes
    _codeEmergenceInterval.unref();

    log.info('CodeEmergence (C1.7) wired to JUDGMENT_CREATED + F7 interval');
  }

  // 4c. CYNIC_STATE → HumanEmergence: Feed psychology snapshots as daily data (C5.7)
  if (humanEmergence) {
    const unsubHumanEmergenceState = globalEventBus.subscribe(
      EventType.CYNIC_STATE,
      (event) => {
        try {
          // Sample at same rate as CYNIC_STATE persistence (1:5)
          // Use a separate counter to avoid coupling with main CYNIC_STATE handler
          if (Math.random() > 0.2) return; // ~1 in 5

          const state = event.payload || {};
          const psy = state.psychology || state.humanPsychology || {};
          const thermo = state.thermodynamics || {};

          humanEmergence.recordDailySnapshot({
            energy: psy.energy || psy.energyLevel || 50,
            focus: psy.focus || psy.focusLevel || 50,
            temperature: thermo.temperature || 0,
            efficiency: thermo.efficiency || 0,
            toolCalls: state.toolCalls || 0,
            sessionDuration: state.sessionDuration || 0,
            errors: state.errors || 0,
          });
          _stats.humanEmergenceSnapshots++;
        } catch (err) {
          log.debug('HumanEmergence state handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubHumanEmergenceState);

    // 4d. Fibonacci-triggered human emergence analysis (F9=34min)
    _humanEmergenceInterval = setInterval(() => {
      try {
        const analysis = humanEmergence.analyze();
        const patterns = analysis?.newPatterns || analysis?.patterns || [];
        if (patterns.length > 0) {
          for (const pattern of patterns) {
            globalEventBus.publish(EventType.PATTERN_DETECTED, {
              source: 'HumanEmergence',
              key: pattern.type || pattern.key,
              significance: pattern.significance?.label || pattern.significance || 'medium',
              category: 'human',
              ...pattern,
            }, { source: 'human-emergence' });
          }
          _stats.humanEmergencePatterns += patterns.length;
        }
      } catch (err) {
        log.debug('Human emergence analysis error', { error: err.message });
      }
    }, 34 * 60 * 1000); // F9 = 34 minutes
    _humanEmergenceInterval.unref();

    log.info('HumanEmergence (C5.7) wired to CYNIC_STATE + F9 interval');
  }

  _started = true;
  _stats.startedAt = Date.now();

  log.info('Event listeners started', {
    hasJudgmentsRepo: !!repositories.judgments,
    hasFeedbackRepo: !!repositories.feedback,
    hasSessionsRepo: !!repositories.sessions,
    hasPsychologyRepo: !!repositories?.psychology,
    hasSharedMemory: !!sharedMemory,
    hasPersistence: !!persistence?.query,
    hasBlockStore: !!blockStore,
  });

  /**
   * Wire a FeedbackProcessor instance to persist its events
   *
   * @param {FeedbackProcessor} feedbackProcessor - FeedbackProcessor instance
   * @param {Object} [feedbackContext] - Additional context
   */
  function wireFeedbackProcessor(feedbackProcessor, feedbackContext = {}) {
    if (!feedbackProcessor) return;

    const mergedContext = { ...context, ...feedbackContext };

    // Listen to feedback-processed event
    const handler = (result) => {
      handleFeedbackProcessed(result, repositories, mergedContext, feedbackContext).catch(
        (err) => {
          log.error('FeedbackProcessor handler threw unexpectedly', { error: err.message });
        }
      );
    };

    feedbackProcessor.on('feedback-processed', handler);

    // Track for cleanup
    _unsubscribers.push(() => {
      feedbackProcessor.off('feedback-processed', handler);
    });

    log.debug('FeedbackProcessor wired for persistence');
  }

  return {
    wireFeedbackProcessor,
    stop: stopEventListeners,
    getStats: () => ({ ..._stats }),
    updateContext: (updates) => {
      Object.assign(context, updates);
    },
  };
}

/**
 * Stop all event listeners
 */
export function stopEventListeners() {
  for (const unsub of _unsubscribers) {
    try {
      if (typeof unsub === 'function') {
        unsub();
      }
    } catch (err) {
      log.debug('Unsubscribe error', { error: err.message });
    }
  }

  // AXE 4: Clear emergence analysis intervals
  if (_solanaEmergenceInterval) {
    clearInterval(_solanaEmergenceInterval);
    _solanaEmergenceInterval = null;
  }
  if (_codeEmergenceInterval) {
    clearInterval(_codeEmergenceInterval);
    _codeEmergenceInterval = null;
  }
  if (_humanEmergenceInterval) {
    clearInterval(_humanEmergenceInterval);
    _humanEmergenceInterval = null;
  }

  _unsubscribers = [];
  _started = false;

  log.info('Event listeners stopped', { stats: _stats });
}

/**
 * Check if listeners are running
 * @returns {boolean}
 */
export function isRunning() {
  return _started;
}

/**
 * Get listener statistics
 * @returns {Object}
 */
export function getListenerStats() {
  return { ..._stats, running: _started };
}

// ═══════════════════════════════════════════════════════════════════════════════
// CLEANUP (AXE 2+: Prevent table bloat)
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Clean up old event data from dog_events and collective_snapshots tables.
 * These tables grow fast and need periodic cleanup.
 *
 * Default retention: 7 days for dog_events, 3 days for snapshots.
 *
 * @param {Object} persistence - PersistenceManager instance
 * @param {Object} [options] - Cleanup options
 * @param {number} [options.dogEventsRetentionDays=7] - Days to retain dog events
 * @param {number} [options.snapshotsRetentionDays=3] - Days to retain snapshots
 * @returns {Promise<{dogEventsDeleted: number, snapshotsDeleted: number}>}
 */
export async function cleanupOldEventData(persistence, options = {}) {
  const {
    dogEventsRetentionDays = 7,
    snapshotsRetentionDays = 3,
  } = options;

  const result = {
    dogEventsDeleted: 0,
    snapshotsDeleted: 0,
  };

  if (!persistence?.query) {
    log.debug('Cleanup skipped - no persistence query available');
    return result;
  }

  try {
    // Clean up old dog_events
    const dogEventsResult = await persistence.query(
      `DELETE FROM dog_events WHERE created_at < NOW() - INTERVAL '${dogEventsRetentionDays} days' RETURNING id`
    );
    result.dogEventsDeleted = dogEventsResult?.rowCount || 0;

    // Clean up old collective_snapshots
    const snapshotsResult = await persistence.query(
      `DELETE FROM collective_snapshots WHERE created_at < NOW() - INTERVAL '${snapshotsRetentionDays} days' RETURNING id`
    );
    result.snapshotsDeleted = snapshotsResult?.rowCount || 0;

    if (result.dogEventsDeleted > 0 || result.snapshotsDeleted > 0) {
      log.info('Event data cleanup complete', {
        dogEventsDeleted: result.dogEventsDeleted,
        snapshotsDeleted: result.snapshotsDeleted,
        dogEventsRetentionDays,
        snapshotsRetentionDays,
      });
    }
  } catch (err) {
    log.warn('Event data cleanup failed', { error: err.message });
  }

  return result;
}

export default {
  startEventListeners,
  stopEventListeners,
  isRunning,
  getListenerStats,
  cleanupOldEventData,
};
