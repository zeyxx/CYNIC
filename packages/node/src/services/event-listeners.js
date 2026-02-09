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
  startedAt: null,
};

/** @type {number} Counter for sampling CYNIC_STATE emissions */
let _cynicStateCounter = 0;

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

    log.info('Automation bus orphan listeners wired (Fix #4 + 4b)');
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
