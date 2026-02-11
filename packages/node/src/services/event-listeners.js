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
  codeDpoPairs: 0,
  codeDecisionsTriggered: 0,
  codeActionsTriggered: 0,
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
  // Cosmos pipeline stats (C7.2-C7.5)
  cosmosJudgments: 0,
  cosmosDecisions: 0,
  cosmosActions: 0,
  cosmosLearnings: 0,
  // Emergence pipeline stats (C1.7, C4.7, C5.7, C6.7, C7.7)
  codeEmergencePatterns: 0,
  codeEmergenceChanges: 0,
  humanEmergenceSnapshots: 0,
  humanEmergencePatterns: 0,
  cynicEmergenceDogEvents: 0,
  cynicEmergenceConsensus: 0,
  cynicEmergencePatterns: 0,
  socialEmergenceCaptures: 0,
  socialEmergencePatterns: 0,
  cosmosEmergenceSnapshots: 0,
  cosmosEmergencePatterns: 0,
  // Self-awareness stats (C6.1, C6.3, C6.4, C5.6)
  cynicSelfHealActions: 0,
  cynicDecisions: 0,
  homeostasisObservations: 0,
  humanSessionsTracked: 0,
  humanActivitiesRecorded: 0,
  startedAt: null,
};

/** @type {number} Counter for sampling CYNIC_STATE emissions */
let _cynicStateCounter = 0;

/** @type {NodeJS.Timeout|null} Solana emergence analysis interval (F8=21min) */
let _solanaEmergenceInterval = null;

/** @type {boolean} Guard: Solana event subscriptions already wired? */
let _solanaWired = false;

/** @type {NodeJS.Timeout|null} Code emergence analysis interval (F7=13min) */
let _codeEmergenceInterval = null;

/** @type {NodeJS.Timeout|null} Human emergence analysis interval (F9=34min) */
let _humanEmergenceInterval = null;

/** @type {NodeJS.Timeout|null} Cynic emergence analysis interval (F8=21min) */
let _cynicEmergenceInterval = null;

/** @type {NodeJS.Timeout|null} Social emergence analysis interval (F10=55min) */
let _socialEmergenceInterval = null;

/** @type {NodeJS.Timeout|null} Cosmos emergence analysis interval (F11=89min) */
let _cosmosEmergenceInterval = null;

/** @type {NodeJS.Timeout|null} HumanJudge periodic assessment interval (F8=21min) */
let _humanJudgeInterval = null;

/** @type {NodeJS.Timeout|null} CosmosJudge periodic health snapshot interval (F9=34min) */
let _cosmosJudgeInterval = null;

/** @type {NodeJS.Timeout|null} SocialJudge periodic assessment interval (F10=55min) */
let _socialJudgeInterval = null;

/** @type {NodeJS.Timeout|null} CynicJudge periodic self-assessment interval (F7=13min) */
let _cynicJudgeInterval = null;

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
    codeActor,
    codeLearner,
    cynicAccountant,
    codeAccountant,
    socialAccountant,
    socialDecider,
    socialActor,
    cosmosAccountant,
    humanActor,
    // Cosmos pipeline singletons (C7.2-C7.5)
    cosmosJudge,
    cosmosDecider,
    cosmosActor,
    cosmosLearner,
    // Solana pipeline singletons (C2.2-C2.7)
    solanaJudge,
    solanaDecider,
    solanaActor,
    solanaLearner,
    solanaAccountant,
    solanaEmergence,
    // Emergence pipeline singletons (C1.7, C4.7, C5.7, C6.7, C7.7)
    codeEmergence,
    humanEmergence,
    cynicEmergence,
    socialEmergence,
    cosmosEmergence,
    // Self-awareness singletons (C6.1, C6.3, C6.4, C5.6)
    cynicActor,
    cynicDecider,
    humanAccountant,
    homeostasis,
    consciousnessMonitor,
    // Human pipeline singletons (C5.2)
    humanJudge,
    // Cross-cutting cost accounting
    costLedger,
    // Model intelligence (Thompson over LLM models)
    modelIntelligence,
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

  // Fix #6½: CALIBRATION_DRIFT_DETECTED → PATTERN_DETECTED: Bridge learning→emergence
  // When CalibrationTracker detects confidence drift, republish as a pattern for the emergence pipeline.
  {
    const unsubCalibDrift = globalEventBus.subscribe(
      EventType.CALIBRATION_DRIFT_DETECTED,
      (event) => {
        try {
          const d = event.payload || event;
          globalEventBus.publish(EventType.PATTERN_DETECTED, {
            source: 'CalibrationTracker',
            key: 'calibration_drift',
            category: 'learning',
            significance: d.ece > 0.15 ? 'high' : 'medium',
            subject: `Calibration drift: ECE=${(d.ece || 0).toFixed(3)} (threshold ${(d.threshold || 0).toFixed(3)})`,
            data: { ece: d.ece, threshold: d.threshold, totalSamples: d.totalSamples },
          }, { source: 'calibration-drift-bridge' });
        } catch (err) {
          log.debug('calibration:drift → pattern:detected bridge failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCalibDrift);
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
  // Uses wireSolanaEventListeners() — can also be called late from collective-singleton
  // after C2.2-C2.7 singletons are initialized (they're null in the sync path).
  // ═════════════════════════════════════════════════════════════════════════════
  wireSolanaEventListeners({
    solanaJudge, solanaDecider, solanaActor, solanaLearner, solanaAccountant, solanaEmergence,
    persistence, sessionId: context.sessionId,
  });

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

  // 3c. TOOL_COMPLETED → CostLedger: Record real token costs + feed CynicAccountant
  //     This is THE bridge that feeds real tokensUsed values into the system.
  //     Previously CynicAccountant.trackOperation() received tokensUsed: 0 on every call.
  if (costLedger) {
    const unsubCostLedger = globalEventBus.subscribe(
      EventType.TOOL_COMPLETED,
      (event) => {
        try {
          const { tool, result, input } = event.payload || {};
          if (!tool) return;

          // Estimate tokens from tool input/output
          const inputText = typeof input === 'string' ? input : JSON.stringify(input || '');
          const outputText = typeof result === 'string' ? result : JSON.stringify(result || '');

          const costRecord = costLedger.record({
            type: 'tool_call',
            inputText,
            outputText,
            source: `tool:${tool}`,
            durationMs: event.payload?.durationMs || 0,
          });

          // Feed CynicAccountant with REAL token values
          if (cynicAccountant && costRecord) {
            cynicAccountant.trackOperation('system', 'action', {
              tokensUsed: costRecord.totalTokens,
              latencyMs: costRecord.durationMs,
              wasSuccessful: !event.payload?.error,
              confidenceProduced: 0.382, // φ⁻² default for tool calls
            });
          }

          _stats.costLedgerOps = (_stats.costLedgerOps || 0) + 1;
        } catch (err) {
          log.debug('CostLedger.record failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCostLedger);

    // 3c½. SESSION_ENDED → CostLedger.persist (save lifetime stats)
    const unsubCostSessionEnd = globalEventBus.subscribe(
      EventType.SESSION_ENDED,
      () => {
        try {
          costLedger.endSession();
        } catch (err) {
          log.debug('CostLedger.endSession failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCostSessionEnd);
  }

  // 3d. COST_UPDATE → pattern:detected: Burn rate spike detection
  //     When token velocity crosses φ⁻¹, emit as anomaly pattern for emergence/learning.
  if (costLedger) {
    let _lastCostAlertLevel = 'abundant';
    const unsubCostSpike = globalEventBus.subscribe(
      EventType.COST_UPDATE,
      (event) => {
        try {
          const { burnRate, budget } = event.payload || {};
          const level = budget?.level || 'abundant';
          const velocity = burnRate?.velocity || 0;

          // Emit pattern when budget transitions to critical/exhausted (one-shot per transition)
          if ((level === 'critical' || level === 'exhausted') && _lastCostAlertLevel !== level) {
            globalEventBus.publish(EventType.PATTERN_DETECTED, {
              category: 'cost_spike',
              significance: level === 'exhausted' ? 'critical' : 'high',
              subject: `Token burn rate spike: budget=${level}, velocity=${(velocity * 100).toFixed(1)}%`,
              data: { level, velocity, tokensPerMinute: burnRate?.tokensPerMinute || 0 },
            }, { source: 'cost-update-bridge' });
            log.info('Cost spike pattern emitted', { level, velocity });
          }

          _lastCostAlertLevel = level;
          _stats.costUpdateEvents = (_stats.costUpdateEvents || 0) + 1;
        } catch (err) {
          log.debug('cost:update → pattern:detected bridge failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCostSpike);
  }

  // 3e. TOOL_COMPLETED → ModelIntelligence: Record model outcomes for Thompson learning
  //     Every tool completion = evidence about whether the current model works for this task.
  if (modelIntelligence) {
    const unsubModelOutcome = globalEventBus.subscribe(
      EventType.TOOL_COMPLETED,
      (event) => {
        try {
          const { tool, result, error, durationMs } = event.payload || {};
          if (!tool) return;

          // Detect current model (best effort from environment)
          const detected = modelIntelligence.detectCurrentModel();

          modelIntelligence.recordOutcome({
            taskType: event.payload?.taskType || 'default',
            model: detected.tier,
            success: !error,
            tool,
            durationMs: durationMs || 0,
          });

          _stats.modelOutcomesRecorded = (_stats.modelOutcomesRecorded || 0) + 1;
        } catch (err) {
          log.debug('ModelIntelligence.recordOutcome failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubModelOutcome);

    // Persist on session end
    const unsubModelSessionEnd = globalEventBus.subscribe(
      EventType.SESSION_ENDED,
      () => {
        try {
          modelIntelligence.persist();
        } catch (err) {
          log.debug('ModelIntelligence.persist failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubModelSessionEnd);
  }

  // 3b½. SOCIAL_CAPTURE → SocialAccountant: Track social interaction economics (C4.6)
  if (socialAccountant) {
    const unsubSocialAccounting = globalEventBus.subscribe(
      EventType.SOCIAL_CAPTURE,
      (event) => {
        try {
          const d = event.data || event.payload || {};
          const tweets = d.tweets || [];
          const users = d.users || [];

          // Track each tweet as an interaction
          for (const tweet of tweets.slice(0, 21)) { // Fib(8) cap
            socialAccountant.trackInteraction({
              type: tweet.isRetweet ? 'retweet' : tweet.isReply ? 'reply' : 'post',
              reach: tweet.impressions || tweet.likes || 0,
              engagement: (tweet.likes || 0) + (tweet.retweets || 0) + (tweet.replies || 0),
              sentiment: tweet.sentiment || 0,
              isInfluencer: (tweet.followers || 0) > 10000,
              platform: 'x',
            }, { sessionId });
          }

          // Track new follows/users as interactions
          for (const user of users.slice(0, 13)) { // Fib(7) cap
            socialAccountant.trackInteraction({
              type: 'follow',
              reach: user.followers || 0,
              engagement: 1,
              sentiment: 0,
              isInfluencer: (user.followers || 0) > 10000,
              platform: 'x',
            }, { sessionId });
          }

          _stats.socialAccountingOps = (_stats.socialAccountingOps || 0) + 1;

          // Publish accounting update
          globalEventBus.publish(EventType.ACCOUNTING_UPDATE, {
            source: 'SocialAccountant',
            type: 'social_capture',
            interactions: tweets.length + users.length,
          }, { source: 'event-listeners' });
        } catch (err) {
          log.debug('SocialAccountant.trackInteraction failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubSocialAccounting);
  }

  // 3b¾. PATTERN_DETECTED → CosmosAccountant: Track ecosystem value flows (C7.6)
  if (cosmosAccountant) {
    const unsubCosmosAccounting = globalEventBus.subscribe(
      EventType.PATTERN_DETECTED,
      (event) => {
        try {
          const d = event.payload || {};
          const category = d.category || d.source || 'unknown';

          // Determine flow type based on pattern category
          let flowType = 'ecosystem_event';
          if (category === 'cosmos' || d.source === 'CosmosEmergence') flowType = 'emergence_signal';
          else if (d.domains && d.domains.length > 1) flowType = 'cross_domain_sync';
          else flowType = 'pattern_detected';

          cosmosAccountant.trackValueFlow({
            type: flowType,
            direction: 'in',
            magnitude: d.confidence || d.significance || 0.3,
            domain: category,
            targetDomain: d.targetDomain || null,
            significance: d.significance === 'HIGH' || d.significance === 'CRITICAL' ? 0.8 : 0.4,
          }, { sessionId });

          _stats.cosmosAccountingOps = (_stats.cosmosAccountingOps || 0) + 1;

          // Persist high-significance value flows to unified_signals (C7.6 persistence)
          if (persistence?.query && (d.significance === 'HIGH' || d.significance === 'CRITICAL')) {
            const id = `cac_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            persistence.query(`
              INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
              VALUES ($1, $2, $3, $4, $5, $6)
            `, [
              id, 'cosmos_accounting', context.sessionId || null,
              JSON.stringify({ flowType, category, significance: d.significance }),
              JSON.stringify({ magnitude: d.confidence || 0.3 }),
              JSON.stringify({ domain: category, targetDomain: d.targetDomain || null }),
            ]).catch(err => {
              log.debug('Cosmos accounting persistence failed', { error: err.message });
            });
          }
        } catch (err) {
          log.debug('CosmosAccountant.trackValueFlow failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCosmosAccounting);
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

    // 3c⅛. CodeDecider.outcome_recorded → persist calibration to unified_signals (C1.3 calibration loop)
    codeDecider.on('outcome_recorded', (entry) => {
      try {
        if (persistence?.query) {
          const id = `cdo_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          persistence.query(`
            INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
          `, [
            id, 'code_calibration', context.sessionId || null,
            JSON.stringify({ decisionType: entry.decisionType, judgmentId: entry.judgmentId }),
            JSON.stringify({ result: entry.result, successRate: entry.successRate }),
            JSON.stringify({ confidence: entry.confidence, adjustment: entry.adjustment }),
          ]).catch(err => {
            log.debug('Code calibration persistence failed', { error: err.message });
          });
        }
      } catch (err) {
        log.debug('CodeDecider outcome_recorded handler error', { error: err.message });
      }
    });
  }

  // 3c½. CODE_DECISION → CodeActor: Execute advisory action from code decisions (C1.4)
  if (codeActor) {
    const unsubCodeAction = globalEventBus.subscribe(
      EventType.CODE_DECISION,
      (event) => {
        try {
          const { decision, reason, judgmentId, qScore } = event.payload || {};
          if (!decision) return;

          const result = codeActor.act(
            { type: decision, decision, reasoning: reason, reason },
            { judgmentId, qScore, source: 'event-pipeline' }
          );

          if (result) {
            _stats.codeActionsTriggered++;
            globalEventBus.publish(EventType.CODE_ACTION, {
              source: 'CodeActor',
              actionType: result.type,
              urgency: result.urgency,
              message: result.message,
              judgmentId,
            }, { source: 'event-listeners' });

            // Persist to unified_signals
            if (persistence?.query) {
              const id = `ca_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
              persistence.query(`
                INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
                VALUES ($1, $2, $3, $4, $5, $6)
              `, [
                id, 'code_action', context.sessionId || null,
                JSON.stringify({ decision, judgmentId, qScore }),
                JSON.stringify({ actionType: result.type, urgency: result.urgency }),
                JSON.stringify({ message: result.message }),
              ]).catch(err => {
                log.debug('Code action persistence failed', { error: err.message });
              });
            }
          }
        } catch (err) {
          log.debug('CodeActor.act failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCodeAction);

    // 3c⅜. CodeActor debt + response → persist to unified_signals (C1.4 feedback loop)
    codeActor.on('action', (result) => {
      try {
        if (result.type === 'log_debt' && persistence?.query) {
          const id = `cdt_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          persistence.query(`
            INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
          `, [
            id, 'code_debt', context.sessionId || null,
            JSON.stringify({ actionType: result.type, urgency: result.urgency }),
            JSON.stringify({ message: result.message }),
            JSON.stringify({ judgmentId: result.context?.judgmentId }),
          ]).catch(err => {
            log.debug('Code debt persistence failed', { error: err.message });
          });
        }
      } catch (err) {
        log.debug('CodeActor action handler error', { error: err.message });
      }
    });

    codeActor.on('response', (resp) => {
      try {
        if (persistence?.query) {
          const id = `car_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          persistence.query(`
            INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
          `, [
            id, 'code_action_response', context.sessionId || null,
            JSON.stringify({ actionType: resp.type }),
            JSON.stringify({ response: resp.response }),
            JSON.stringify({}),
          ]).catch(err => {
            log.debug('Code action response persistence failed', { error: err.message });
          });
        }
      } catch (err) {
        log.debug('CodeActor response handler error', { error: err.message });
      }
    });
  }

  // 3c½b. CODE_ACTION → CodeAccountant: Track economic value of code actions (C1.6)
  if (codeAccountant && codeActor) {
    const unsubCodeActionAccounting = globalEventBus.subscribe(
      EventType.CODE_ACTION,
      (event) => {
        try {
          const d = event.payload || {};
          // Map action to accounting trackChange format
          codeAccountant.trackChange({
            filePath: d.judgmentId || 'advisory-action',
            linesAdded: 0,
            linesRemoved: 0,
          }, {
            source: 'code:action',
            actionType: d.actionType,
            urgency: d.urgency,
          });
          _stats.codeAccountingOps++;
        } catch (err) {
          log.debug('code:action → CodeAccountant failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCodeActionAccounting);
  }

  // 3c¾. CODE_DECISION → CodeLearner: Register decisions for feedback matching (C1.5)
  if (codeLearner) {
    const unsubCodeLearnRegister = globalEventBus.subscribe(
      EventType.CODE_DECISION,
      (event) => {
        try {
          const { decision, reason, judgmentId, qScore } = event.payload || {};
          if (!decision) return;

          codeLearner.registerDecision({
            judgmentId,
            type: decision,
            decision,
            riskLevel: event.payload.riskLevel || 'unknown',
            riskScore: event.payload.riskScore || 0,
            confidence: event.payload.confidence || 0,
            riskFactors: event.payload.riskFactors || [],
          });
        } catch (err) {
          log.debug('CodeLearner.registerDecision failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCodeLearnRegister);

    // USER_FEEDBACK → CodeLearner: Match feedback to code decisions
    const unsubCodeLearnFeedback = globalEventBus.subscribe(
      EventType.USER_FEEDBACK,
      (event) => {
        try {
          const { type, context: feedbackContext, reason, judgmentId } = event.payload || {};
          if (!type) return;

          const result = codeLearner.processFeedback({
            type,
            context: feedbackContext,
            reason,
            judgmentId,
          });

          if (result) {
            _stats.codeLearningOutcomes = (_stats.codeLearningOutcomes || 0) + 1;

            // Persist learning outcome to unified_signals
            if (persistence?.query) {
              const id = `cl_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
              persistence.query(`
                INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
                VALUES ($1, $2, $3, $4, $5, $6)
              `, [
                id, 'code_learning', context.sessionId || null,
                JSON.stringify({ decisionId: result.decisionId, feedback: result.feedback }),
                JSON.stringify({ outcome: result.outcome, decisionType: result.decisionType }),
                JSON.stringify({ riskLevel: result.riskLevel, confidence: result.confidence }),
              ]).catch(err => {
                log.debug('Code learning persistence failed', { error: err.message });
              });
            }
          }
        } catch (err) {
          log.debug('CodeLearner.processFeedback failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCodeLearnFeedback);

    // 3c⅞. CodeLearner DPO pairs → preference_pairs + unified_signals (C1.5 DPO loop closure)
    // DPO pairs land in preference_pairs (DPOOptimizer reads these → routing_weights)
    // Also archived in unified_signals for analytics
    codeLearner.on('dpo_pair', (pair) => {
      try {
        _stats.codeDpoPairs = (_stats.codeDpoPairs || 0) + 1;

        if (persistence?.query) {
          // PRIMARY: Insert into preference_pairs (DPOOptimizer consumes these)
          persistence.query(`
            INSERT INTO preference_pairs (chosen, rejected, context, context_type, confidence)
            VALUES ($1, $2, $3, $4, $5)
          `, [
            JSON.stringify(pair.preferred || {}),
            JSON.stringify(pair.rejected || {}),
            JSON.stringify({ riskLevel: pair.riskLevel, source: 'CodeLearner' }),
            pair.context || 'code_decision',
            pair.confidence || 0.5,
          ]).catch(err => {
            log.debug('Code DPO preference_pairs insert failed', { error: err.message });
          });

          // SECONDARY: Archive in unified_signals for analytics
          const id = `dpo_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          persistence.query(`
            INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
          `, [
            id, 'code_dpo', context.sessionId || null,
            JSON.stringify({ preferred: pair.preferred, rejected: pair.rejected }),
            JSON.stringify({ context: pair.context || 'code_decision', riskLevel: pair.riskLevel }),
            JSON.stringify({ ts: pair.ts || Date.now() }),
          ]).catch(err => {
            log.debug('Code DPO unified_signals archive failed', { error: err.message });
          });
        }
      } catch (err) {
        log.debug('CodeLearner dpo_pair handler error', { error: err.message });
      }
    });

    // 3c⅞b. code:learning → homeostasis + unified_signals (C1.5 downstream)
    const unsubCodeLearning = globalEventBus.subscribe(
      'code:learning',
      (event) => {
        try {
          const d = event.payload || event;

          // Feed learning rate to homeostasis
          if (homeostasis) {
            const matchRate = codeLearner.getHealth?.()?.matchRate ?? 0.5;
            homeostasis.update('codeLearningRate', matchRate);
            _stats.homeostasisObservations++;
          }

          // Wire to CodeActor.recordResponse() — feedback on recent actions
          if (codeActor?.recordResponse && d.decisionType) {
            const response = d.outcome === 'success' ? 'acted' : 'dismiss';
            codeActor.recordResponse(d.decisionType, response);
          }
        } catch (err) {
          log.debug('code:learning downstream handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCodeLearning);

    log.info('CodeLearner (C1.5) wired: decisions + feedback + DPO persistence + downstream');
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

  // 3d½. HUMAN_ACTION → HumanAccountant: Track human intervention costs (C5.6)
  if (humanAccountant && humanActor) {
    const unsubHumanActionAccounting = globalEventBus.subscribe(
      EventType.HUMAN_ACTION,
      (event) => {
        try {
          const d = event.payload || {};
          // Record intervention as a task completion
          humanAccountant.recordTask(true, {
            source: 'human:action',
            actionType: d.actionType,
            reason: d.reason,
          });
          _stats.humanAccountingOps = (_stats.humanAccountingOps || 0) + 1;
        } catch (err) {
          log.debug('human:action → HumanAccountant failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubHumanActionAccounting);
  }

  // ═════════════════════════════════════════════════════════════════════════════
  // SELF-AWARENESS WIRING (C6.1 PERCEIVE, C6.4 ACT, C5.6 ACCOUNT)
  // "Le chien se connaît, se soigne, et compte les pas de son maître"
  // ═════════════════════════════════════════════════════════════════════════════

  // 3f. consciousness:changed → CynicActor: Self-healing from degradation (C6.4)
  if (cynicActor) {
    const unsubCynicSelfHeal = globalEventBus.subscribe(
      EventType.CONSCIOUSNESS_CHANGED,
      (event) => {
        try {
          const actions = cynicActor.processConsciousnessChange(event.payload || event);

          for (const action of actions) {
            _stats.cynicSelfHealActions++;

            // Persist self-healing actions to unified_signals
            if (persistence?.query) {
              const id = `sh_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
              persistence.query(`
                INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
                VALUES ($1, $2, $3, $4, $5, $6)
              `, [
                id, 'cynic_self_heal', context.sessionId || null,
                JSON.stringify({ healthState: action.healthState, reason: action.context?.reason }),
                JSON.stringify({ action: action.type }),
                JSON.stringify(action.context || {}),
              ]).catch(err => {
                log.debug('Self-heal persistence failed', { error: err.message });
              });
            }
          }
        } catch (err) {
          log.debug('CynicActor.processConsciousnessChange failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCynicSelfHeal);
    log.info('CynicActor (C6.4) wired to consciousness:changed');
  }

  // 3g. JUDGMENT_CREATED + CYNIC_STATE → HomeostasisTracker: Feed metrics (C6.1 boost)
  if (homeostasis) {
    // Feed qScore from judgments into homeostasis baseline
    const unsubHomeostasisJudgment = globalEventBus.subscribe(
      EventType.JUDGMENT_CREATED,
      (event) => {
        try {
          const j = event.payload || {};
          const qScore = j.qScore || j.global_score || j.score;
          if (typeof qScore === 'number') {
            homeostasis.update('qScore', qScore / 100); // Normalize to 0-1
            _stats.homeostasisObservations++;
          }
        } catch (err) {
          log.debug('Homeostasis judgment handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubHomeostasisJudgment);

    // C6.2: Judge self-evaluation — feed confidence + skepticism to homeostasis
    const unsubJudgeSelfEval = globalEventBus.subscribe(
      EventType.JUDGMENT_CREATED,
      (event) => {
        try {
          const j = event.payload || {};
          const confidence = j.confidence;

          // Feed judgment confidence as homeostasis metric
          if (typeof confidence === 'number') {
            homeostasis.update('judgmentConfidence', confidence);
            _stats.homeostasisObservations++;
          }

          // If judge has selfSkeptic, track doubt quality
          if (judge?.selfSkeptic) {
            const skepticStats = judge.selfSkeptic.getStats?.();
            if (skepticStats) {
              // Compute skepticism ratio: doubts / total judgments
              const ratio = skepticStats.judgmentsDoubled > 0
                ? skepticStats.confidenceReductions / skepticStats.judgmentsDoubled
                : 0;
              homeostasis.update('judgmentSkepticism', ratio);
            }
          }

          // Persist low-confidence judgments to unified_signals for meta-analysis
          if (persistence?.query && typeof confidence === 'number' && confidence < 0.3) {
            const id = `jse_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            persistence.query(`
              INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
              VALUES ($1, $2, $3, $4, $5, $6)
            `, [
              id, 'judge_self_evaluation', context.sessionId || null,
              JSON.stringify({ qScore: j.qScore, verdict: j.verdict, itemType: j.itemType }),
              JSON.stringify({ confidence, weaknesses: j.weaknesses }),
              JSON.stringify({ cell: 'C6.2', reason: 'low_confidence_judgment' }),
            ]).catch(err => {
              log.debug('Judge self-evaluation persistence failed', { error: err.message });
            });
          }
        } catch (err) {
          log.debug('Judge self-evaluation handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubJudgeSelfEval);

    // Feed thermodynamic metrics (errorRate, latency)
    const unsubHomeostasisState = globalEventBus.subscribe(
      EventType.CYNIC_STATE,
      (event) => {
        try {
          // Sample 1:5 to avoid flooding
          if (Math.random() > 0.2) return;

          const state = event.payload || {};
          const thermo = state.thermodynamics || {};
          if (thermo.errorRate !== undefined) homeostasis.update('errorRate', thermo.errorRate);
          if (thermo.temperature !== undefined) homeostasis.update('latency', thermo.temperature / 100);
          if (thermo.efficiency !== undefined) homeostasis.update('successRate', thermo.efficiency);
          _stats.homeostasisObservations++;

          // Detect perturbation and feed to CynicActor
          if (cynicActor && homeostasis.isPerturbed?.()) {
            const metrics = homeostasis.getMetrics?.() || {};
            for (const [metric, data] of Object.entries(metrics)) {
              if (data?.perturbed) {
                cynicActor.processPerturbation({
                  metric,
                  deviation: data.deviation || 3,
                });
              }
            }
          }

          // Feed homeostasis score to ConsciousnessMonitor as observation
          if (consciousnessMonitor) {
            const score = homeostasis.getHomeostasis?.() ?? 0.5;
            consciousnessMonitor.observe('HOMEOSTASIS', {
              score,
              status: homeostasis.getStatus?.() || 'unknown',
            }, score, 'homeostasis-bridge');
          }
        } catch (err) {
          log.debug('Homeostasis state handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubHomeostasisState);

    // Feed emergence rate — pattern detection frequency as self-perception
    const unsubHomeostasisEmergence = globalEventBus.subscribe(
      EventType.PATTERN_DETECTED,
      (event) => {
        try {
          const p = event.payload || {};
          const sig = p.significance || 'low';
          const value = sig === 'critical' ? 1.0
            : sig === 'high' ? 0.7
              : sig === 'medium' ? 0.4
                : 0.2;
          homeostasis.update('emergenceRate', value);
          _stats.homeostasisObservations++;
        } catch (err) {
          log.debug('Homeostasis emergence handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubHomeostasisEmergence);

    // Feed awareness level — consciousness state as self-perception
    const unsubHomeostasisAwareness = globalEventBus.subscribe(
      EventType.CONSCIOUSNESS_CHANGED,
      (event) => {
        try {
          const payload = event.payload || event;
          const state = payload.newState || payload.state || 'DORMANT';
          const value = state === 'TRANSCENDENT' ? 0.9
            : state === 'HEIGHTENED' ? 0.7
              : state === 'AWARE' ? 0.5
                : state === 'AWAKENING' ? 0.3
                  : 0.1;
          homeostasis.update('awarenessLevel', value);
          _stats.homeostasisObservations++;
        } catch (err) {
          log.debug('Homeostasis awareness handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubHomeostasisAwareness);

    // Feed consensus quality — agreement rate as collective self-perception
    const unsubHomeostasisConsensus = globalEventBus.subscribe(
      EventType.CONSENSUS_COMPLETED,
      (event) => {
        try {
          const p = event.payload || event;
          const agreement = p.agreement ?? p.inference?.weightedAgreement ?? 0.5;
          homeostasis.update('consensusQuality', agreement);
          _stats.homeostasisObservations++;
        } catch (err) {
          log.debug('Homeostasis consensus handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubHomeostasisConsensus);

    // Feed decision confidence — self-governance decision quality
    const unsubHomeostasisDecision = globalEventBus.subscribe(
      'cosmos:decision',
      (event) => {
        try {
          const d = event.payload?.decision || event.payload || {};
          if (typeof d.confidence === 'number') {
            homeostasis.update('decisionConfidence', d.confidence);
            _stats.homeostasisObservations++;
          }
        } catch (err) {
          log.debug('Homeostasis decision handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubHomeostasisDecision);

    const unsubHomeostasisCynicDecision = globalEventBus.subscribe(
      'cynic:decision',
      (event) => {
        try {
          const d = event.payload || event;
          if (typeof d.confidence === 'number') {
            homeostasis.update('decisionConfidence', d.confidence);
            _stats.homeostasisObservations++;
          }
        } catch (err) {
          log.debug('Homeostasis cynic decision handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubHomeostasisCynicDecision);

    log.info('HomeostasisTracker (C6.1) wired to JUDGMENT_CREATED + CYNIC_STATE + PATTERN_DETECTED + CONSCIOUSNESS_CHANGED + CONSENSUS_COMPLETED + decisions');
  }

  // 3g½. cynic:decision → CynicActor: Execute self-governance decisions (C6.3→C6.4)
  if (cynicActor?.processDecision) {
    const unsubCynicDecisionAction = globalEventBus.subscribe(
      'cynic:decision',
      (event) => {
        try {
          const decision = event.payload || event;
          if (!decision?.type || decision.type === 'acknowledge') return;

          const actions = cynicActor.processDecision(decision);
          _stats.cynicSelfHealActions = (_stats.cynicSelfHealActions || 0) + actions.length;

          // Publish cynic:action for chain completeness (same pattern as cosmos:action)
          if (actions.length > 0) {
            globalEventBus.publish('cynic:action', {
              source: 'CynicActor',
              decision: { type: decision.type, urgency: decision.urgency },
              actions: actions.map(a => ({ type: a.type || a.action, reason: a.reason })),
            }, { source: 'event-listeners' });
          }
        } catch (err) {
          log.debug('cynic:decision → CynicActor failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCynicDecisionAction);
  }

  // 3h. SESSION_STARTED/ENDED + USER_FEEDBACK → HumanAccountant (C5.6)
  if (humanAccountant) {
    // Session start tracking
    const unsubHumanAcctStart = globalEventBus.subscribe(
      EventType.SESSION_STARTED,
      (event) => {
        try {
          humanAccountant.startSession({
            sessionId: context.sessionId,
            userId: context.userId,
            ...event.payload,
          });
          _stats.humanSessionsTracked++;
        } catch (err) {
          log.debug('HumanAccountant.startSession failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubHumanAcctStart);

    // Session end tracking + feed summary to HumanEmergence (C5.7)
    const unsubHumanAcctEnd = globalEventBus.subscribe(
      EventType.SESSION_ENDED,
      (event) => {
        try {
          const summary = humanAccountant.endSession();
          _stats.humanSessionsTracked++;

          // Persist session summary to unified_signals (C5.6 persistence)
          if (persistence?.query && summary) {
            const id = `hs_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            persistence.query(`
              INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
              VALUES ($1, $2, $3, $4, $5, $6)
            `, [
              id, 'human_accounting', context.sessionId || null,
              JSON.stringify({ durationMs: summary.durationMs, tasksCompleted: summary.tasksCompleted, tasksAttempted: summary.tasksAttempted }),
              JSON.stringify({ efficiency: summary.efficiency, productivityRating: summary.productivityRating }),
              JSON.stringify({ topActivities: summary.topActivities, toolsUsed: summary.toolsUsed }),
            ]).catch(err => {
              log.debug('Human accounting persistence failed', { error: err.message });
            });
          }

          // Feed session summary to HumanEmergence for pattern analysis (C5.7 boost)
          if (humanEmergence && summary) {
            try {
              humanEmergence.recordDailySnapshot({
                energy: summary.productivityRating === 'excellent' ? 80 : summary.productivityRating === 'good' ? 60 : 40,
                focus: summary.efficiency ? summary.efficiency * 100 : 50,
                tasksCompleted: summary.tasksCompleted || 0,
                tasksAttempted: summary.tasksAttempted || 0,
                sessionDuration: summary.durationMs || 0,
                efficiency: summary.efficiency || 0,
              });
            } catch (emergeErr) {
              log.debug('HumanEmergence session feed failed', { error: emergeErr.message });
            }
          }
        } catch (err) {
          log.debug('HumanAccountant.endSession failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubHumanAcctEnd);

    // Tool use → activity recording (piggyback on JUDGMENT_CREATED as proxy for work)
    const unsubHumanAcctActivity = globalEventBus.subscribe(
      EventType.JUDGMENT_CREATED,
      (event) => {
        try {
          const j = event.payload || {};
          const itemType = j.itemType || j.item_type || '';
          let activityType = 'coding'; // default
          if (itemType.includes('review') || itemType.includes('commit')) activityType = 'review';
          else if (itemType.includes('debug')) activityType = 'debugging';
          else if (itemType.includes('doc')) activityType = 'documentation';
          else if (itemType.includes('plan')) activityType = 'planning';

          humanAccountant.startActivity(activityType, { source: 'judgment', estimatedMs: 60000 });
          humanAccountant.endActivity({ completed: true, source: 'judgment' });
          _stats.humanActivitiesRecorded++;
        } catch (err) {
          log.debug('HumanAccountant.recordActivity failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubHumanAcctActivity);

    // USER_FEEDBACK → completion tracking
    const unsubHumanAcctFeedback = globalEventBus.subscribe(
      EventType.USER_FEEDBACK,
      (event) => {
        try {
          const { type } = event.payload || {};
          if (!type) return;
          const success = type === 'positive';
          humanAccountant.recordTask(success, { feedbackType: type, reason });
        } catch (err) {
          log.debug('HumanAccountant.recordCompletion failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubHumanAcctFeedback);

    log.info('HumanAccountant (C5.6) wired to SESSION_STARTED/ENDED + JUDGMENT_CREATED + USER_FEEDBACK');
  }

  // 3i. CynicEmergence.pattern_detected → CynicDecider: Self-governance (C6.3)
  // Patterns from C6.7 → decisions in C6.3 → actions in C6.4
  if (cynicDecider && cynicEmergence) {
    cynicEmergence.on('pattern_detected', (pattern) => {
      try {
        const decision = cynicDecider.decideOnPattern(pattern);
        if (decision && decision.type !== 'acknowledge') {
          _stats.cynicDecisions = (_stats.cynicDecisions || 0) + 1;

          // CynicActor bridge now handled via cynic:decision event subscriber (below)

          // Persist decision to unified_signals
          if (persistence?.query) {
            const id = `cd_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            persistence.query(`
              INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
              VALUES ($1, $2, $3, $4, $5, $6)
            `, [
              id, 'cynic_decision', context.sessionId || null,
              JSON.stringify({ patternType: pattern.type, significance: pattern.significance }),
              JSON.stringify({ decision: decision.type, urgency: decision.urgency }),
              JSON.stringify(decision.context || {}),
            ]).catch(err => {
              log.debug('CynicDecider persistence failed', { error: err.message });
            });
          }
        }
      } catch (err) {
        log.debug('CynicDecider.decideOnPattern failed', { error: err.message });
      }
    });

    // Also wire consciousness changes → CynicDecider for preemptive governance
    const unsubCynicDecideConsciousness = globalEventBus.subscribe(
      EventType.CONSCIOUSNESS_CHANGED,
      (event) => {
        try {
          const payload = event.payload || event;
          const stats = cynicActor?.getStats?.() || {};
          const decision = cynicDecider.decideOnConsciousness({
            awarenessLevel: payload.awarenessLevel,
            healthState: stats.currentHealthState || 'nominal',
            trend: stats.trend || 0,
          });
          if (decision) {
            _stats.cynicDecisions = (_stats.cynicDecisions || 0) + 1;

            // CynicActor bridge now handled via cynic:decision event subscriber (below)

            // Persist consciousness decision to unified_signals
            if (persistence?.query) {
              const id = `ccd_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
              persistence.query(`
                INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
                VALUES ($1, $2, $3, $4, $5, $6)
              `, [
                id, 'cynic_consciousness_decision', context.sessionId || null,
                JSON.stringify({ awarenessLevel: payload.awarenessLevel, healthState: stats.currentHealthState }),
                JSON.stringify({ decision: decision.type, urgency: decision.urgency }),
                JSON.stringify(decision.context || {}),
              ]).catch(err => {
                log.debug('Consciousness decision persistence failed', { error: err.message });
              });
            }
          }
        } catch (err) {
          log.debug('CynicDecider.decideOnConsciousness failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCynicDecideConsciousness);

    log.info('CynicDecider (C6.3) wired to CynicEmergence patterns + consciousness changes');
  }

  // ═════════════════════════════════════════════════════════════════════════════
  // COSMOS PIPELINE WIRING (C7.2-C7.5)
  // "Le chien juge les étoiles, décide le cap, agit, et apprend"
  // Chain: PATTERN_DETECTED → CosmosJudge → cosmos:judgment → CosmosDecider
  //        → cosmos:decision → CosmosActor → cosmos:action → CosmosLearner
  // ═════════════════════════════════════════════════════════════════════════════

  // 3e. PATTERN_DETECTED → CosmosJudge: Judge ecosystem patterns (C7.2)
  if (cosmosJudge) {
    const unsubCosmosJudge = globalEventBus.subscribe(
      EventType.PATTERN_DETECTED,
      (event) => {
        try {
          const d = event.payload || {};
          // Only judge high-significance or cosmos-category patterns
          const sig = d.significance || 'low';
          if (sig !== 'HIGH' && sig !== 'CRITICAL' && d.category !== 'cosmos') return;

          const judgment = cosmosJudge.judge({
            type: d.category === 'cosmos' ? 'emergence_signal' : 'ecosystem_health',
            data: {
              significance: sig === 'CRITICAL' ? 1.0 : sig === 'HIGH' ? 0.8 : 0.5,
              confidence: d.confidence || 0.5,
              repos: d.domains || [],
              convergence: d.convergence || d.confidence || 0.5,
            },
          });

          if (judgment) {
            _stats.cosmosJudgments++;
            // Publish for downstream decision pipeline
            globalEventBus.publish('cosmos:judgment', {
              source: 'CosmosJudge',
              judgment,
              triggeredBy: event.id,
            }, { source: 'event-listeners' });
          }
        } catch (err) {
          log.debug('CosmosJudge.judge failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCosmosJudge);
  }

  // 3e½. cosmos:judgment → CosmosDecider: Decide ecosystem action (C7.3)
  if (cosmosDecider) {
    const unsubCosmosDecision = globalEventBus.subscribe(
      'cosmos:judgment',
      (event) => {
        try {
          const { judgment } = event.payload || {};
          if (!judgment) return;

          // Only decide on concerning verdicts (GROWL/BARK) or significant patterns
          if (judgment.verdict !== 'GROWL' && judgment.verdict !== 'BARK' && judgment.verdict !== 'HOWL') return;

          const decision = cosmosDecider.decide(judgment, {
            observationCount: cosmosJudge?.getStats?.()?.totalJudgments || 0,
            trend: cosmosJudge?.getHealth?.()?.healthTrend || 'stable',
          });

          if (decision) {
            _stats.cosmosDecisions++;
            globalEventBus.publish('cosmos:decision', {
              source: 'CosmosDecider',
              decision,
              judgment,
            }, { source: 'event-listeners' });
          }
        } catch (err) {
          log.debug('CosmosDecider.decide failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCosmosDecision);
  }

  // 3e¾. cosmos:decision → CosmosActor: Execute advisory action (C7.4)
  if (cosmosActor) {
    const unsubCosmosAction = globalEventBus.subscribe(
      'cosmos:decision',
      (event) => {
        try {
          const { decision } = event.payload || {};
          if (!decision) return;

          const action = cosmosActor.act(decision);

          if (action) {
            _stats.cosmosActions++;

            // Publish cosmos:action for downstream consumers (C7.4 integration)
            globalEventBus.publish('cosmos:action', {
              source: 'CosmosActor',
              action,
              decision,
            }, { source: 'event-listeners' });

            // Feed outcome to learner
            if (cosmosLearner) {
              cosmosLearner.recordOutcome({
                category: 'decision_calibration',
                data: {
                  decision: decision.decision,
                  wasGood: decision.verdict !== 'BARK', // optimistic: assume non-BARK was good
                },
              });
              _stats.cosmosLearnings++;
            }

            // Feed outcome back to CosmosDecider for calibration (C7.3 loop)
            if (cosmosDecider?.recordOutcome) {
              cosmosDecider.recordOutcome({
                decisionType: decision.decision,
                result: action ? 'success' : 'failure',
                reason: action?.message || 'action_executed',
              });
            }

            // Persist cosmos action to unified_signals (C7.4 persistence)
            if (persistence?.query) {
              const id = `ca_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
              persistence.query(`
                INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
                VALUES ($1, $2, $3, $4, $5, $6)
              `, [
                id, 'cosmos_action', context.sessionId || null,
                JSON.stringify({ decision: decision.decision, verdict: decision.verdict }),
                JSON.stringify({ action: action.type, urgency: action.urgency }),
                JSON.stringify({ message: action.message }),
              ]).catch(err => {
                log.debug('Cosmos action persistence failed', { error: err.message });
              });
            }
          }
        } catch (err) {
          log.debug('CosmosActor.act failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCosmosAction);
  }

  // 3e¾½. cosmos:action → CosmosAccountant: Track ecosystem action value (C7.6)
  if (cosmosAccountant?.trackValueFlow && cosmosActor) {
    const unsubCosmosActionAccounting = globalEventBus.subscribe(
      'cosmos:action',
      (event) => {
        try {
          const d = event.payload || {};
          const action = d.action || {};
          const decision = d.decision || {};
          cosmosAccountant.trackValueFlow({
            type: 'ecosystem_event',
            direction: 'out',
            magnitude: action.urgency === 'critical' ? 0.8 : action.urgency === 'high' ? 0.5 : 0.2,
            domain: 'cosmos',
            targetDomain: action.type || 'unknown',
            significance: decision.verdict === 'HOWL' ? 0.8 : decision.verdict === 'WAG' ? 0.5 : 0.3,
          }, {
            source: 'cosmos:action',
            actionType: action.type,
            verdict: decision.verdict,
          });
          _stats.cosmosAccountingOps = (_stats.cosmosAccountingOps || 0) + 1;
        } catch (err) {
          log.debug('cosmos:action → CosmosAccountant failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCosmosActionAccounting);
  }

  // 3e⅞. cosmos:judgment → CosmosLearner: Learn from health observations (C7.5)
  if (cosmosLearner) {
    const unsubCosmosLearn = globalEventBus.subscribe(
      'cosmos:judgment',
      (event) => {
        try {
          const { judgment } = event.payload || {};
          if (!judgment) return;

          // C7.5 prediction validation: compare previous prediction vs actual score
          const prediction = cosmosLearner.predictHealth?.() ?? null;
          if (prediction?.prediction !== null && prediction?.prediction !== undefined) {
            const actualScore = judgment.score;
            const predicted = prediction.prediction;
            const wasCorrect = Math.abs(predicted - actualScore) < 15; // within 15 points = correct
            cosmosLearner.recordPredictionOutcome(wasCorrect);
          }

          // Feed health score to learner
          cosmosLearner.recordOutcome({
            category: 'health_prediction',
            data: {
              score: judgment.score,
              verdict: judgment.verdict,
              health: judgment.score / 100,
            },
          });
          _stats.cosmosLearnings++;

          // C7.5 concentration_risk: derive from sub-score spread
          const scores = judgment.scores || {};
          const subValues = [scores.coherence, scores.utility, scores.sustainability].filter(v => typeof v === 'number');
          if (subValues.length >= 2) {
            const maxSub = Math.max(...subValues);
            const minSub = Math.min(...subValues);
            const spread = maxSub - minSub;
            const concentration = spread / 100; // 0=balanced, 1=maximally concentrated
            cosmosLearner.recordOutcome({
              category: 'concentration_risk',
              data: {
                concentration,
                level: concentration,
                wasRisky: concentration > 0.4, // > 40pt spread = risky
              },
            });
          }

          // Persist learning outcome to unified_signals (C7.5 persistence)
          if (persistence?.query) {
            const id = `cl_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            persistence.query(`
              INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
              VALUES ($1, $2, $3, $4, $5, $6)
            `, [
              id, 'cosmos_learning', context.sessionId || null,
              JSON.stringify({ score: judgment.score, verdict: judgment.verdict }),
              JSON.stringify({ health: judgment.score / 100, prediction }),
              JSON.stringify({ category: 'health_prediction' }),
            ]).catch(err => {
              log.debug('Cosmos learning persistence failed', { error: err.message });
            });
          }
        } catch (err) {
          log.debug('CosmosLearner.recordOutcome failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCosmosLearn);

    // C7.5 cross_repo_convergence: learn from multi-domain patterns
    const unsubCosmosConvergence = globalEventBus.subscribe(
      EventType.PATTERN_DETECTED,
      (event) => {
        try {
          const p = event.payload || {};
          // Only feed cosmos/ecosystem patterns with domain info
          if (p.category !== 'cosmos' && p.category !== 'ecosystem' && p.source !== 'CosmosEmergence') return;
          const domains = p.domains || p.repos || [];
          if (domains.length < 2) return;
          cosmosLearner.recordOutcome({
            category: 'cross_repo_convergence',
            data: {
              domains,
              strength: p.confidence || 0.5,
              convergence: p.confidence || 0.5,
            },
          });
          _stats.cosmosLearnings++;
        } catch (err) {
          log.debug('CosmosLearner convergence handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCosmosConvergence);
  }

  if (codeDecider || codeActor || cynicAccountant || codeAccountant || socialAccountant || cosmosAccountant || humanActor || cosmosJudge) {
    log.info('RIGHT side event listeners wired', {
      codeDecider: !!codeDecider,
      codeActor: !!codeActor,
      cynicAccountant: !!cynicAccountant,
      costLedger: !!costLedger,
      modelIntelligence: !!modelIntelligence,
      codeAccountant: !!codeAccountant,
      socialAccountant: !!socialAccountant,
      cosmosAccountant: !!cosmosAccountant,
      humanActor: !!humanActor,
      cosmosJudge: !!cosmosJudge,
      cosmosDecider: !!cosmosDecider,
      cosmosActor: !!cosmosActor,
      cosmosLearner: !!cosmosLearner,
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

    // 4a½. perception:fs:change → CodeEmergence: Feed raw filesystem changes (C1.1→C1.7 bridge)
    const unsubFsChange = globalEventBus.subscribe(
      'perception:fs:change',
      (event) => {
        try {
          const d = event.payload || event;
          if (!d.path) return;
          codeEmergence.recordChange({
            filePath: d.path,
            linesAdded: 0,
            linesRemoved: 0,
            imports: [],
            complexityDelta: 0,
          });
          _stats.codeEmergenceChanges++;
        } catch (err) {
          log.debug('CodeEmergence fs:change handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubFsChange);

    const unsubFsAdd = globalEventBus.subscribe(
      'perception:fs:add',
      (event) => {
        try {
          const d = event.payload || event;
          if (!d.path) return;
          codeEmergence.recordChange({
            filePath: d.path,
            linesAdded: 1,
            linesRemoved: 0,
            imports: [],
            complexityDelta: 0.05,
          });
          _stats.codeEmergenceChanges++;
        } catch (err) {
          log.debug('CodeEmergence fs:add handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubFsAdd);

    const unsubFsUnlink = globalEventBus.subscribe(
      'perception:fs:unlink',
      (event) => {
        try {
          const d = event.payload || event;
          if (!d.path) return;
          codeEmergence.recordChange({
            filePath: d.path,
            linesAdded: 0,
            linesRemoved: 1,
            imports: [],
            complexityDelta: -0.1, // BURN axiom: deletion simplifies
          });
          _stats.codeEmergenceChanges++;
        } catch (err) {
          log.debug('CodeEmergence fs:unlink handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubFsUnlink);

    // 4a¾. perception:fs:addDir / unlinkDir → CodeEmergence (C1.1→C1.7 structural changes)
    const unsubFsAddDir = globalEventBus.subscribe(
      'perception:fs:addDir',
      (event) => {
        try {
          const d = event.payload || event;
          if (!d.path) return;
          codeEmergence.recordChange({
            filePath: d.path,
            linesAdded: 0,
            linesRemoved: 0,
            imports: [],
            complexityDelta: 0.05, // New directory = structural growth
          });
          _stats.codeEmergenceChanges++;
        } catch (err) {
          log.debug('CodeEmergence fs:addDir handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubFsAddDir);

    const unsubFsUnlinkDir = globalEventBus.subscribe(
      'perception:fs:unlinkDir',
      (event) => {
        try {
          const d = event.payload || event;
          if (!d.path) return;
          codeEmergence.recordChange({
            filePath: d.path,
            linesAdded: 0,
            linesRemoved: 0,
            imports: [],
            complexityDelta: -0.05, // BURN axiom: directory removal simplifies
          });
          _stats.codeEmergenceChanges++;
        } catch (err) {
          log.debug('CodeEmergence fs:unlinkDir handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubFsUnlinkDir);

    // 4a⅞. perception:fs:error → unified_signals (C1.1 error tracking)
    const unsubFsError = globalEventBus.subscribe(
      'perception:fs:error',
      (event) => {
        try {
          const d = event.payload || event;
          if (persistence?.query) {
            const id = `fse_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            persistence.query(`
              INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
              VALUES ($1, $2, $3, $4, $5, $6)
            `, [
              id, 'fs_perception_error', sessionId || null,
              JSON.stringify({ error: d.error }),
              JSON.stringify({ type: 'perception_error' }),
              JSON.stringify({ stack: d.stack }),
            ]).catch(() => {});
          }
        } catch (err) {
          log.debug('fs:error persist handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubFsError);

    // 4a⁹⁄₁₆. perception:fs:ready → log + stats (C1.1 lifecycle)
    const unsubFsReady = globalEventBus.subscribe(
      'perception:fs:ready',
      (event) => {
        try {
          const d = event.payload || event;
          log.info('Filesystem perception ready', {
            paths: d.paths,
            filesWatched: d.filesWatched,
          });
        } catch (err) {
          log.debug('fs:ready handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubFsReady);

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

    // 4b2. CodeEmergence.pattern_detected → persist + feed CodeDecider calibration (C1.3→C1.7 bridge)
    codeEmergence.on('pattern_detected', (pattern) => {
      try {
        // Persist code emergence patterns to unified_signals
        if (persistence?.query) {
          const id = `ce_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          persistence.query(`
            INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
          `, [
            id, 'code_emergence', context.sessionId || null,
            JSON.stringify({ patternType: pattern.type || pattern.key, significance: pattern.significance }),
            JSON.stringify({ confidence: pattern.confidence, message: pattern.message }),
            JSON.stringify(pattern.context || pattern.data || {}),
          ]).catch(err => {
            log.debug('Code emergence pattern persistence failed', { error: err.message });
          });
        }

        // C1.7→C1.3: Feed code patterns as calibration to CodeDecider
        if (codeDecider?.recordOutcome) {
          const pType = pattern.type || pattern.key || '';
          const isNegative = pType.includes('hotspot') || pType.includes('complexity') || pType.includes('coupling');
          codeDecider.recordOutcome({
            decision: isNegative ? 'FLAG_REVIEW' : 'APPROVE',
            outcome: isNegative ? 'pattern_warning' : 'pattern_healthy',
            riskLevel: isNegative ? 'high' : 'low',
            reason: `emergence_pattern: ${pType}`,
          });
        }

        _stats.codeEmergencePatterns++;
      } catch (err) {
        log.debug('CodeEmergence pattern_detected handler error', { error: err.message });
      }
    });

    log.info('CodeEmergence (C1.7) wired to JUDGMENT_CREATED + F7 interval + pattern persistence');
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

    // 4d2. HumanEmergence.pattern_detected → persist + downstream consumers (C5.7)
    humanEmergence.on('pattern_detected', (pattern) => {
      try {
        if (persistence?.query) {
          const id = `hep_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          persistence.query(`
            INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
          `, [
            id, 'human_emergence', context.sessionId || null,
            JSON.stringify({ patternType: pattern.type, significance: pattern.significance }),
            JSON.stringify({ confidence: pattern.confidence, message: pattern.message }),
            JSON.stringify(pattern.data || {}),
          ]).catch(err => {
            log.debug('Human emergence pattern persistence failed', { error: err.message });
          });
        }

        // C5.7→C6.1: Feed burnout/risk patterns to HomeostasisTracker
        if (homeostasis) {
          const pType = pattern.type || '';
          if (pType === 'burnout_risk' || pType === 'overwork_pattern' || pType === 'declining_engagement') {
            const riskLevel = pattern.confidence || 0.5;
            homeostasis.update('humanBurnoutRisk', riskLevel);
            _stats.homeostasisObservations++;
          }
        }

        // C5.7→consciousness: Feed high-significance patterns as observation
        if (consciousnessMonitor) {
          const sigLevel = pattern.significance?.level ?? (pattern.significance === 'High' ? 3 : pattern.significance === 'Medium' ? 2 : 1);
          if (sigLevel >= 2) {
            const score = pattern.confidence || 0.5;
            consciousnessMonitor.observe('HUMAN_EMERGENCE', {
              patternType: pattern.type,
              significance: pattern.significance,
              message: pattern.message,
            }, score, 'human-emergence-bridge');
          }
        }

        _stats.humanEmergencePatterns++;
      } catch (err) {
        log.debug('HumanEmergence pattern_detected handler error', { error: err.message });
      }
    });

    log.info('HumanEmergence (C5.7) wired to CYNIC_STATE + F9 interval + pattern persistence');
  }

  // 4e. DOG_EVENT + CONSENSUS_COMPLETED → CynicEmergence: Feed Dog behavior data (C6.7)
  if (cynicEmergence) {
    const unsubCynicEmergenceDog = globalEventBus.subscribe(
      EventType.DOG_EVENT,
      (event) => {
        try {
          const d = event?.payload || event;
          cynicEmergence.recordDogEvent({
            dog: d.dog || d.dogName,
            eventType: d.eventType || d.type,
          });
          _stats.cynicEmergenceDogEvents++;
        } catch (err) {
          log.debug('CynicEmergence dog handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCynicEmergenceDog);

    const unsubCynicEmergenceConsensus = globalEventBus.subscribe(
      EventType.CONSENSUS_COMPLETED,
      (event) => {
        try {
          const d = event?.payload || event;
          cynicEmergence.recordConsensus({
            approved: d.approved,
            agreement: d.agreement || d.agreementRatio,
            vetoCount: d.vetoCount || d.vetoes,
            dogCount: d.dogCount || d.voterCount,
          });
          _stats.cynicEmergenceConsensus++;
        } catch (err) {
          log.debug('CynicEmergence consensus handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCynicEmergenceConsensus);

    // Also feed CYNIC_STATE as health snapshots (sampled 1:5)
    let cynicStateCounter = 0;
    const unsubCynicEmergenceHealth = globalEventBus.subscribe(
      EventType.CYNIC_STATE,
      (event) => {
        cynicStateCounter++;
        if (cynicStateCounter % 5 !== 0) return;
        try {
          const d = event?.payload || event;
          cynicEmergence.recordHealthSnapshot({
            avgHealth: d.health?.avg || d.avgHealth || 0.5,
            memoryLoad: d.memoryLoad || 0,
            patternCount: d.patternCount || 0,
            dogCount: d.dogCount || d.activeDogs || 0,
          });
        } catch (err) {
          log.debug('CynicEmergence health handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCynicEmergenceHealth);

    // F8=21min: CynicEmergence analysis
    _cynicEmergenceInterval = setInterval(() => {
      try {
        const patterns = cynicEmergence.analyze();
        if (patterns?.length > 0) {
          for (const pattern of patterns) {
            globalEventBus.publish(EventType.PATTERN_DETECTED, {
              source: 'CynicEmergence',
              key: pattern.type,
              significance: pattern.significance || 'medium',
              category: 'cynic',
              ...pattern,
            }, { source: 'cynic-emergence' });
          }
          _stats.cynicEmergencePatterns += patterns.length;
        }
      } catch (err) {
        log.debug('Cynic emergence analysis error', { error: err.message });
      }
    }, 21 * 60 * 1000); // F8 = 21 minutes
    _cynicEmergenceInterval.unref();

    log.info('CynicEmergence (C6.7) wired to DOG_EVENT + CONSENSUS_COMPLETED + F8 interval');
  }

  // 4f. SOCIAL_CAPTURE → SocialEmergence: Feed social interaction data (C4.7)
  if (socialEmergence) {
    const unsubSocialEmergence = globalEventBus.subscribe(
      EventType.SOCIAL_CAPTURE,
      (event) => {
        try {
          const d = event?.payload || event;
          socialEmergence.recordCapture({
            platform: d.platform,
            type: d.type || d.eventType,
            engagement: d.engagement || d.likes || d.interactions,
            sentiment: d.sentiment,
            author: d.author || d.username,
          });
          _stats.socialEmergenceCaptures++;
        } catch (err) {
          log.debug('SocialEmergence capture handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubSocialEmergence);

    // F10=55min: SocialEmergence analysis
    _socialEmergenceInterval = setInterval(() => {
      try {
        const patterns = socialEmergence.analyze();
        if (patterns?.length > 0) {
          for (const pattern of patterns) {
            globalEventBus.publish(EventType.PATTERN_DETECTED, {
              source: 'SocialEmergence',
              key: pattern.type,
              significance: pattern.significance || 'medium',
              category: 'social',
              ...pattern,
            }, { source: 'social-emergence' });
          }
          _stats.socialEmergencePatterns += patterns.length;
        }
      } catch (err) {
        log.debug('Social emergence analysis error', { error: err.message });
      }
    }, 55 * 60 * 1000); // F10 = 55 minutes
    _socialEmergenceInterval.unref();

    log.info('SocialEmergence (C4.7) wired to SOCIAL_CAPTURE + F10 interval');
  }

  // 4g. PATTERN_DETECTED + CYNIC_STATE → CosmosEmergence: Feed ecosystem data (C7.7)
  if (cosmosEmergence) {
    // Feed cross-domain patterns
    const unsubCosmosEmergence = globalEventBus.subscribe(
      EventType.PATTERN_DETECTED,
      (event) => {
        try {
          const d = event?.payload || event;
          // Only record patterns from OTHER emergence detectors (avoid self-feeding loop)
          if (d.source === 'CosmosEmergence') return;
          cosmosEmergence.recordCrossEvent({
            repos: [d.category || d.dimension || 'unknown'],
            eventType: d.key || d.type || 'pattern',
            significance: d.significance || 'low',
          });
          // Also track repo activity per source dimension
          cosmosEmergence.recordRepoActivity({
            repo: d.category || d.dimension || d.source || 'unknown',
            eventType: d.key || d.type || 'pattern',
          });
          _stats.cosmosEmergenceSnapshots++;
        } catch (err) {
          log.debug('CosmosEmergence pattern handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCosmosEmergence);

    // Feed health snapshots from CYNIC_STATE (unblocks health trajectory detector)
    const unsubCosmosEmergenceHealth = globalEventBus.subscribe(
      EventType.CYNIC_STATE,
      (event) => {
        try {
          // Sample 1:5 to avoid flooding
          if (Math.random() > 0.2) return;
          const state = event.payload || {};
          const dogs = state.dogs || {};
          const health = state.health || state.consciousness || {};
          cosmosEmergence.recordHealthSnapshot({
            avgHealth: health.awarenessLevel ?? health.score ?? 0.5,
            repoCount: Object.keys(dogs).length || 0,
            totalIssues: state.goalViolations || 0,
            stalePRs: 0,
          });
        } catch (err) {
          log.debug('CosmosEmergence health snapshot error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCosmosEmergenceHealth);

    // F11=89min: CosmosEmergence analysis
    _cosmosEmergenceInterval = setInterval(() => {
      try {
        const patterns = cosmosEmergence.analyze();
        if (patterns?.length > 0) {
          for (const pattern of patterns) {
            globalEventBus.publish(EventType.PATTERN_DETECTED, {
              source: 'CosmosEmergence',
              key: pattern.type,
              significance: pattern.significance || 'medium',
              category: 'cosmos',
              ...pattern,
            }, { source: 'cosmos-emergence' });
          }
          _stats.cosmosEmergencePatterns += patterns.length;
        }
      } catch (err) {
        log.debug('Cosmos emergence analysis error', { error: err.message });
      }
    }, 89 * 60 * 1000); // F11 = 89 minutes
    _cosmosEmergenceInterval.unref();

    // Persist CosmosEmergence detected patterns to unified_signals (C7.7 persistence)
    cosmosEmergence.on('pattern_detected', (pattern) => {
      try {
        if (persistence?.query) {
          const id = `cep_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          persistence.query(`
            INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
          `, [
            id, 'cosmos_emergence', context.sessionId || null,
            JSON.stringify({ patternType: pattern.type, significance: pattern.significance }),
            JSON.stringify({ confidence: pattern.confidence, message: pattern.message }),
            JSON.stringify(pattern.data || {}),
          ]).catch(err => {
            log.debug('Cosmos emergence pattern persistence failed', { error: err.message });
          });
        }
      } catch (err) {
        log.debug('CosmosEmergence pattern_detected handler error', { error: err.message });
      }
    });

    log.info('CosmosEmergence (C7.7) wired to PATTERN_DETECTED + F11 interval + pattern persistence');
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // 5. HUMAN JUDGE (C5.2) — Periodic human state assessment
  // ═══════════════════════════════════════════════════════════════════════════

  if (humanJudge) {
    // 5a. F8=21min: Periodic human state judgment from psychology file
    _humanJudgeInterval = setInterval(async () => {
      try {
        const { readFileSync, existsSync } = await import('fs');
        const { join } = await import('path');
        const { homedir } = await import('os');
        const psyPath = join(homedir(), '.cynic', 'psychology', 'state.json');
        if (!existsSync(psyPath)) return;

        const psyState = JSON.parse(readFileSync(psyPath, 'utf8'));
        // Only judge if recently updated (within 30 min)
        if (!psyState.updatedAt || (Date.now() - psyState.updatedAt) > 30 * 60 * 1000) return;

        const dims = psyState.dimensions || {};
        const temporal = psyState.temporal || {};

        const perception = {
          energy: dims.energy?.value ?? 0.5,
          focus: dims.focus?.value ?? 0.5,
          frustration: dims.frustration?.value ?? 0.2,
          cognitiveLoad: dims.cognitiveLoad?.value ?? 4,
          sessionMinutes: temporal.sessionDuration ? Math.floor(temporal.sessionDuration / 60000) : 0,
        };

        humanJudge.judge(perception, { source: 'periodic', sessionId: context.sessionId });
        _stats.humanJudgments = (_stats.humanJudgments || 0) + 1;
      } catch (err) {
        log.debug('HumanJudge periodic assessment error', { error: err.message });
      }
    }, 21 * 60 * 1000); // F8 = 21 minutes
    _humanJudgeInterval.unref();

    // 5b. human:judgment → HomeostasisTracker + CosmosAccountant + persist (C5.2 downstream)
    const unsubHumanJudgment = globalEventBus.subscribe(
      'human:judgment',
      (event) => {
        try {
          const j = event.payload || event;

          // Feed human wellbeing to homeostasis
          if (homeostasis) {
            if (typeof j.qScore === 'number') {
              homeostasis.update('humanWellbeing', j.qScore);
              _stats.homeostasisObservations++;
            }
            if (j.scores?.burnoutRisk !== undefined) {
              homeostasis.update('humanBurnoutRisk', j.scores.burnoutRisk);
              _stats.homeostasisObservations++;
            }
          }

          // Feed to CosmosAccountant as cross-domain value flow
          if (cosmosAccountant?.trackValueFlow) {
            cosmosAccountant.trackValueFlow({
              type: 'ecosystem_event',
              direction: 'in',
              magnitude: j.qScore || 0.5,
              domain: 'human',
              significance: j.verdict === 'critical' ? 0.9 : j.verdict === 'strained' ? 0.6 : 0.3,
            });
          }

          // Persist to unified_signals
          if (persistence?.query) {
            const id = `hj_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            persistence.query(`
              INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
              VALUES ($1, $2, $3, $4, $5, $6)
            `, [
              id, 'human_judgment', context.sessionId || null,
              JSON.stringify({ qScore: j.qScore, verdict: j.verdict }),
              JSON.stringify({ scores: j.scores }),
              JSON.stringify({ cell: 'C5.2', recommendations: j.recommendations }),
            ]).catch(err => {
              log.debug('HumanJudge persistence failed', { error: err.message });
            });
          }

          _stats.humanJudgments = (_stats.humanJudgments || 0) + 1;
        } catch (err) {
          log.debug('HumanJudge downstream handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubHumanJudgment);

    log.info('HumanJudge (C5.2) wired with F8 periodic + downstream persistence');
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // 6. COSMOS JUDGE PERIODIC + COSMOS ACCOUNTANT CROSS-DOMAIN (C7.2 + C7.6)
  // ═══════════════════════════════════════════════════════════════════════════

  // 6a. F9=34min: Periodic CosmosJudge ecosystem health snapshot
  if (cosmosJudge) {
    _cosmosJudgeInterval = setInterval(() => {
      try {
        // Build ecosystem health snapshot from accumulated emergence data
        const healthData = {
          avgHealth: consciousnessMonitor?.getHealth?.()?.score ?? 0.5,
          repoCount: 0,
          totalIssues: 0,
          stalePRs: 0,
        };

        // Gather health from all emergence detectors
        const emergenceDetectors = [codeEmergence, humanEmergence, cynicEmergence, socialEmergence, cosmosEmergence];
        let activeDetectors = 0;
        for (const detector of emergenceDetectors) {
          if (detector?.getStats) {
            activeDetectors++;
            const stats = detector.getStats();
            healthData.totalIssues += stats.anomalies || stats.warnings || 0;
          }
        }
        healthData.repoCount = activeDetectors;

        cosmosJudge.judge({
          type: 'ecosystem_health',
          data: healthData,
        });
        _stats.cosmosJudgments = (_stats.cosmosJudgments || 0) + 1;
      } catch (err) {
        log.debug('CosmosJudge periodic health snapshot error', { error: err.message });
      }
    }, 34 * 60 * 1000); // F9 = 34 minutes
    _cosmosJudgeInterval.unref();

    log.info('CosmosJudge (C7.2) periodic health snapshot wired at F9=34min');
  }

  // 6b. PATTERN_DETECTED → CosmosAccountant: Track cross-domain value flows (C7.6)
  if (cosmosAccountant?.trackValueFlow) {
    const unsubCosmosAccounting = globalEventBus.subscribe(
      EventType.PATTERN_DETECTED,
      (event) => {
        try {
          const d = event.payload || event;
          const category = d.category || d.dimension || 'unknown';
          const sig = d.confidence || (d.significance === 'high' ? 0.8 : d.significance === 'medium' ? 0.5 : 0.3);

          // Track as pattern detection value
          cosmosAccountant.trackValueFlow({
            type: 'pattern_detected',
            direction: 'in',
            magnitude: sig,
            domain: category,
            significance: sig,
          });

          // If pattern spans multiple domains, also track as cross-domain sync
          const domains = d.domains || d.repos || [];
          if (domains.length >= 2) {
            cosmosAccountant.trackValueFlow({
              type: 'cross_domain_sync',
              direction: 'in',
              magnitude: sig,
              domain: domains[0],
              targetDomain: domains[1],
              significance: sig * 1.2, // cross-domain patterns are higher value
            });
          }

          _stats.cosmosAccountingOps = (_stats.cosmosAccountingOps || 0) + 1;
        } catch (err) {
          log.debug('CosmosAccountant pattern tracking error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCosmosAccounting);

    log.info('CosmosAccountant (C7.6) wired to PATTERN_DETECTED for cross-domain tracking');
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // 7. BROKEN CHAIN REPAIRS (social:capture→social:judgment, cynic:state→cynic:judgment, user:feedback→pattern:learned)
  // ═══════════════════════════════════════════════════════════════════════════

  // 7a. SOCIAL_CAPTURE → social:judgment: Inline judgment of social interactions (C4.2)
  // No dedicated SocialJudge module exists — inline lightweight scoring.
  // Pattern: social captures → score quality/engagement → publish judgment → downstream (SocialDecider future)
  if (socialAccountant) {
    const unsubSocialJudgment = globalEventBus.subscribe(
      EventType.SOCIAL_CAPTURE,
      (event) => {
        try {
          const d = event.data || event.payload || {};
          const tweets = d.tweets || [];
          const users = d.users || [];
          if (tweets.length === 0 && users.length === 0) return;

          // Score: engagement volume + sentiment + reach
          const totalEngagement = tweets.reduce((sum, t) =>
            sum + (t.likes || 0) + (t.retweets || 0) + (t.replies || 0), 0);
          const avgSentiment = tweets.length > 0
            ? tweets.reduce((sum, t) => sum + (t.sentiment || 0), 0) / tweets.length
            : 0;
          const totalReach = tweets.reduce((sum, t) => sum + (t.impressions || t.likes || 0), 0)
            + users.reduce((sum, u) => sum + (u.followers || 0), 0);

          // φ-bounded score: normalize engagement + sentiment + reach
          const engagementScore = Math.min(totalEngagement / 100, 1) * 0.4;
          const sentimentScore = ((avgSentiment + 1) / 2) * 0.3; // normalize -1..1 → 0..1
          const reachScore = Math.min(totalReach / 10000, 1) * 0.3;
          const rawScore = (engagementScore + sentimentScore + reachScore) * 100;
          const score = Math.min(rawScore, 61.8); // φ⁻¹ cap

          const verdict = score > 50 ? 'HOWL' : score > 35 ? 'WAG' : score > 20 ? 'GROWL' : 'BARK';

          globalEventBus.publish('social:judgment', {
            cell: 'C4.2',
            score,
            verdict,
            tweetCount: tweets.length,
            userCount: users.length,
            totalEngagement,
            avgSentiment,
            totalReach,
            timestamp: Date.now(),
          }, { source: 'event-listeners:SocialJudge' });

          _stats.socialJudgments = (_stats.socialJudgments || 0) + 1;
        } catch (err) {
          log.debug('social:capture → social:judgment failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubSocialJudgment);
    log.info('SocialJudge (C4.2) inline wired: SOCIAL_CAPTURE → social:judgment');
  }

  // 7a½. social:judgment → persist to unified_signals
  if (persistence?.query) {
    const unsubSocialJudgmentPersist = globalEventBus.subscribe(
      'social:judgment',
      (event) => {
        try {
          const d = event.payload || {};
          const id = `soj_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          persistence.query(`
            INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
          `, [
            id, 'social_judgment', context.sessionId || null,
            JSON.stringify({ tweets: d.tweetCount, users: d.userCount, engagement: d.totalEngagement }),
            JSON.stringify({ score: d.score, verdict: d.verdict }),
            JSON.stringify({ cell: 'C4.2', reach: d.totalReach, sentiment: d.avgSentiment }),
          ]).catch(err => {
            log.debug('social:judgment persistence failed', { error: err.message });
          });
        } catch (err) {
          log.debug('social:judgment persist handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubSocialJudgmentPersist);
  }

  // 7a². social:judgment → SocialDecider → social:decision (C4.3)
  // Pattern: social judgment → decide response → publish decision → downstream SocialActor
  if (socialDecider) {
    const unsubSocialDecision = globalEventBus.subscribe(
      'social:judgment',
      (event) => {
        try {
          const judgment = event.payload || {};
          if (!judgment.score && judgment.score !== 0) return;

          const result = socialDecider.decide(judgment, {
            sessionId: context.sessionId || null,
          });

          if (result) {
            _stats.socialDecisions = (_stats.socialDecisions || 0) + 1;
          }
        } catch (err) {
          log.debug('social:judgment → social:decision failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubSocialDecision);
    log.info('SocialDecider (C4.3) wired: social:judgment → social:decision');
  }

  // 7a³. social:decision → SocialActor → social:action (C4.4)
  // Pattern: social decision → execute advisory action → emit social:action
  if (socialActor && socialDecider) {
    const unsubSocialAction = globalEventBus.subscribe(
      'social:decision',
      (event) => {
        try {
          const decision = event.payload || {};
          if (!decision.decision) return;

          const result = socialActor.act(decision, {
            sessionId: context.sessionId || null,
          });

          if (result) {
            _stats.socialActions = (_stats.socialActions || 0) + 1;

            // Feed outcome back to decider for calibration
            socialDecider.recordOutcome({
              decisionType: decision.decision,
              result: 'success',
              reason: 'action_delivered',
            });

            // Persist to unified_signals
            if (persistence?.query) {
              const id = `soa_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
              persistence.query(`
                INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
                VALUES ($1, $2, $3, $4, $5, $6)
              `, [
                id, 'social_action', context.sessionId || null,
                JSON.stringify({ decision: decision.decision, verdict: decision.verdict }),
                JSON.stringify({ action: result.type, urgency: result.urgency }),
                JSON.stringify({ cell: 'C4.4', message: result.message?.slice(0, 200) }),
              ]).catch(err => {
                log.debug('social:action persistence failed', { error: err.message });
              });
            }
          }
        } catch (err) {
          log.debug('social:decision → social:action failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubSocialAction);
    log.info('SocialActor (C4.4) wired: social:decision → social:action');
  }

  // 7b. CYNIC_STATE → cynic:judgment: Self-assessment of collective health (C6.2)
  // No dedicated CynicJudge module — inline scoring of collective state.
  // Pattern: cynic state snapshots → score health/entropy/consensus → publish judgment
  {
    let cynicJudgeCounter = 0;
    const unsubCynicJudgment = globalEventBus.subscribe(
      EventType.CYNIC_STATE,
      (event) => {
        try {
          cynicJudgeCounter++;
          // Sample 1:5 (same rate as CYNIC_STATE persistence)
          if (cynicJudgeCounter % 5 !== 0) return;

          const d = event.payload || {};
          const collective = d.collective || {};
          const psychology = d.psychology || {};

          // Score dimensions: collective health + consensus quality + dog diversity + low burnout
          const health = collective.averageHealth || 0; // 0..1
          const consensus = collective.consensusRate || 0; // 0..1
          const activeDogs = collective.activeDogs || 0;
          const dogDiversity = Math.min(activeDogs / 7, 1); // 7=L(4) ideal
          const burnoutInverse = 1 - (psychology.burnoutRisk || psychology.frustration || 0); // lower burnout = better

          const rawScore = (health * 0.3 + consensus * 0.25 + dogDiversity * 0.2 + burnoutInverse * 0.25) * 100;
          const score = Math.min(rawScore, 61.8); // φ⁻¹ cap

          const verdict = score > 50 ? 'HOWL' : score > 35 ? 'WAG' : score > 20 ? 'GROWL' : 'BARK';

          globalEventBus.publish('cynic:judgment', {
            cell: 'C6.2',
            score,
            verdict,
            health,
            consensus,
            activeDogs,
            burnoutRisk: 1 - burnoutInverse,
            timestamp: Date.now(),
          }, { source: 'event-listeners:CynicJudge' });

          _stats.cynicJudgments = (_stats.cynicJudgments || 0) + 1;
        } catch (err) {
          log.debug('cynic:state → cynic:judgment failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCynicJudgment);
    log.info('CynicJudge (C6.2) inline wired: CYNIC_STATE → cynic:judgment (1:5 sampling)');
  }

  // 7b½. cynic:judgment → CynicDecider + persist (C6.2 downstream)
  if (cynicDecider || persistence?.query) {
    const unsubCynicJudgmentDown = globalEventBus.subscribe(
      'cynic:judgment',
      (event) => {
        try {
          const d = event.payload || {};

          // Feed to CynicDecider if available
          if (cynicDecider?.evaluate) {
            cynicDecider.evaluate({
              type: 'self_assessment',
              score: d.score,
              verdict: d.verdict,
              health: d.health,
              consensus: d.consensus,
            });
          }

          // Persist
          if (persistence?.query) {
            const id = `cj_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            persistence.query(`
              INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
              VALUES ($1, $2, $3, $4, $5, $6)
            `, [
              id, 'cynic_judgment', context.sessionId || null,
              JSON.stringify({ health: d.health, consensus: d.consensus, dogs: d.activeDogs }),
              JSON.stringify({ score: d.score, verdict: d.verdict }),
              JSON.stringify({ cell: 'C6.2', burnoutRisk: d.burnoutRisk }),
            ]).catch(err => {
              log.debug('cynic:judgment persistence failed', { error: err.message });
            });
          }
        } catch (err) {
          log.debug('cynic:judgment downstream handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCynicJudgmentDown);
  }

  // 7c. USER_FEEDBACK → pattern:learned: Bridge feedback processing to learning events (C5.5→C6.5)
  // CodeLearner processes feedback and persists, but never publishes pattern:learned.
  // Bridge: when feedback produces a learning outcome, publish pattern:learned to globalEventBus.
  {
    const unsubFeedbackLearned = globalEventBus.subscribe(
      EventType.USER_FEEDBACK,
      (event) => {
        try {
          const { type, context: feedbackCtx, reason } = event.payload || {};
          if (!type) return;

          // Only emit pattern:learned for actionable feedback types
          const learnableTypes = ['correct', 'incorrect', 'approve', 'reject', 'override'];
          if (!learnableTypes.includes(type)) return;

          globalEventBus.publish(EventType.PATTERN_LEARNED, {
            source: 'user_feedback',
            feedbackType: type,
            reason: reason || null,
            context: feedbackCtx || {},
            confidence: type === 'correct' || type === 'approve' ? 0.618 : 0.382,
            timestamp: Date.now(),
          }, { source: 'event-listeners:FeedbackBridge' });

          _stats.patternsLearned = (_stats.patternsLearned || 0) + 1;
        } catch (err) {
          log.debug('user:feedback → pattern:learned bridge failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubFeedbackLearned);
    log.info('FeedbackBridge wired: USER_FEEDBACK → pattern:learned');
  }

  // 7c½. pattern:learned → unified_signals + homeostasis (downstream consumer)
  // Without this subscriber, pattern:learned is an orphan event.
  {
    const unsubPatternLearned = globalEventBus.subscribe(
      EventType.PATTERN_LEARNED,
      (event) => {
        try {
          const d = event.payload || {};

          // Feed learning confidence to homeostasis
          if (homeostasis) {
            homeostasis.update('patternLearningRate', d.confidence || 0.5);
            _stats.homeostasisObservations++;
          }

          // Persist to unified_signals
          if (persistence?.query) {
            const id = `pl_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            persistence.query(`
              INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
              VALUES ($1, $2, $3, $4, $5, $6)
            `, [
              id, 'pattern_learned', context.sessionId || null,
              JSON.stringify({ source: d.source, feedbackType: d.feedbackType }),
              JSON.stringify({ confidence: d.confidence, reason: d.reason }),
              JSON.stringify({ timestamp: d.timestamp }),
            ]).catch(err => {
              log.debug('pattern:learned persistence failed', { error: err.message });
            });
          }

          _stats.patternsLearnedConsumed = (_stats.patternsLearnedConsumed || 0) + 1;
        } catch (err) {
          log.debug('pattern:learned consumer error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubPatternLearned);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // 8. ORPHAN EVENT CONSUMERS (events published but nobody subscribed)
  // ═══════════════════════════════════════════════════════════════════════════

  // 8a. cynic:action → CynicAccountant: Track self-healing action costs (C6.4→C6.6)
  // Pattern modeled on cosmos:action → CosmosAccountant (line ~2629)
  if (cynicAccountant) {
    const unsubCynicActionAccounting = globalEventBus.subscribe(
      'cynic:action',
      (event) => {
        try {
          const d = event.payload || {};
          const actions = d.actions || [];
          for (const action of actions) {
            cynicAccountant.trackOperation(
              'CynicActor',
              action.type || 'self_heal',
              { wasSuccessful: true, latencyMs: 0 },
              { action: action.type, reason: action.reason }
            );
          }
          _stats.cynicActionAccountingOps = (_stats.cynicActionAccountingOps || 0) + 1;
        } catch (err) {
          log.debug('cynic:action → CynicAccountant failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCynicActionAccounting);
    log.info('CynicAccountant (C6.6) wired to cynic:action');
  }

  // 8b. solana:action → SolanaAccountant: Track on-chain action costs (C2.4→C2.6)
  if (solanaAccountant) {
    const unsubSolanaActionAccounting = globalEventBus.subscribe(
      'solana:action',
      (event) => {
        try {
          const d = event.payload || {};
          solanaAccountant.recordTransaction({
            type: d.decision?.type || 'unknown',
            success: d.decision?.executed ?? true,
            fee: 0, // Fee tracked separately on actual TX
          });
          _stats.solanaActionAccountingOps = (_stats.solanaActionAccountingOps || 0) + 1;
        } catch (err) {
          log.debug('solana:action → SolanaAccountant failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubSolanaActionAccounting);
    log.info('SolanaAccountant (C2.6) wired to solana:action');
  }

  // 8c. solana:learning → unified_signals + homeostasis (C2.5 downstream)
  // Pattern modeled on code:learning handler (line ~1889)
  {
    const unsubSolanaLearning = globalEventBus.subscribe(
      'solana:learning',
      (event) => {
        try {
          const d = event.payload || {};
          const updates = d.updates || [];

          // Feed to homeostasis
          if (homeostasis && updates.length > 0) {
            homeostasis.update('solanaLearningRate', Math.min(updates.length / 5, 1));
            _stats.homeostasisObservations++;
          }

          // Persist to unified_signals
          if (persistence?.query && updates.length > 0) {
            const id = `sl_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            persistence.query(`
              INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
              VALUES ($1, $2, $3, $4, $5, $6)
            `, [
              id, 'solana_learning', context.sessionId || null,
              JSON.stringify({ cell: d.cell, updateCount: updates.length }),
              JSON.stringify({ updates: updates.slice(0, 5) }), // cap payload
              JSON.stringify({ dimension: 'SOLANA', analysis: 'LEARN' }),
            ]).catch(err => {
              log.debug('solana:learning persistence failed', { error: err.message });
            });
          }

          _stats.solanaLearningConsumed = (_stats.solanaLearningConsumed || 0) + 1;
        } catch (err) {
          log.debug('solana:learning consumer error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubSolanaLearning);
  }

  // 8d. solana:emergence → unified_signals + PATTERN_DETECTED bridge (C2.7 downstream)
  // Emergence patterns should also flow into the PATTERN_DETECTED chain for CosmosJudge
  {
    const unsubSolanaEmergence = globalEventBus.subscribe(
      'solana:emergence',
      (event) => {
        try {
          const d = event.payload || {};
          const patterns = d.patterns || [];

          // Bridge significant patterns to PATTERN_DETECTED
          for (const pattern of patterns) {
            if (pattern.significance === 'high' || pattern.significance === 'critical') {
              globalEventBus.publish(EventType.PATTERN_DETECTED, {
                source: 'SolanaEmergence',
                key: pattern.type || pattern.key || 'solana_anomaly',
                significance: pattern.significance,
                category: 'solana',
                ...pattern,
              }, { source: 'solana-emergence-bridge' });
            }
          }

          // Persist to unified_signals
          if (persistence?.query && patterns.length > 0) {
            const id = `se_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            persistence.query(`
              INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
              VALUES ($1, $2, $3, $4, $5, $6)
            `, [
              id, 'solana_emergence', context.sessionId || null,
              JSON.stringify({ cell: d.cell, patternCount: patterns.length }),
              JSON.stringify({ patterns: patterns.slice(0, 5) }),
              JSON.stringify({ dimension: 'SOLANA', analysis: 'EMERGE' }),
            ]).catch(err => {
              log.debug('solana:emergence persistence failed', { error: err.message });
            });
          }

          _stats.solanaEmergenceConsumed = (_stats.solanaEmergenceConsumed || 0) + 1;
        } catch (err) {
          log.debug('solana:emergence consumer error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubSolanaEmergence);
  }

  // 8e. learning:signal → unified_signals (cross-domain learning archive)
  // UnifiedSignalStore publishes complete signal objects — archive for analytics
  if (persistence?.query) {
    const unsubLearningSignal = globalEventBus.subscribe(
      'learning:signal',
      (event) => {
        try {
          const signal = event.payload || event;
          if (!signal.id) return;

          const id = `ls_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          persistence.query(`
            INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
          `, [
            id, 'learning_signal', context.sessionId || null,
            JSON.stringify({ source: signal.source, input: signal.input }),
            JSON.stringify({ outcome: signal.outcome, confidence: signal.learning?.confidence }),
            JSON.stringify({ signalId: signal.id, canPair: signal.learning?.canPair }),
          ]).catch(err => {
            log.debug('learning:signal persistence failed', { error: err.message });
          });

          _stats.learningSignalsConsumed = (_stats.learningSignalsConsumed || 0) + 1;
        } catch (err) {
          log.debug('learning:signal consumer error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubLearningSignal);
  }

  // 8f. learning:dpo_pair → preference_pairs + unified_signals (cross-domain DPO archive)
  // Pattern modeled on codeLearner.on('dpo_pair') handler (line ~1851)
  if (persistence?.query) {
    const unsubLearningDpoPair = globalEventBus.subscribe(
      'learning:dpo_pair',
      (event) => {
        try {
          const pair = event.payload || event;
          if (!pair.pairId) return;

          // PRIMARY: Insert into preference_pairs (DPOOptimizer consumes)
          persistence.query(`
            INSERT INTO preference_pairs (chosen, rejected, context, context_type, confidence)
            VALUES ($1, $2, $3, $4, $5)
          `, [
            JSON.stringify({ id: pair.chosenId }),
            JSON.stringify({ id: pair.rejectedId }),
            JSON.stringify({ source: 'UnifiedSignalStore', pairId: pair.pairId }),
            'unified_signal',
            0.5, // default confidence for auto-pairs
          ]).catch(err => {
            log.debug('learning:dpo_pair → preference_pairs failed', { error: err.message });
          });

          // ARCHIVE: Also insert into unified_signals
          const id = `dp_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          persistence.query(`
            INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
          `, [
            id, 'dpo_pair', context.sessionId || null,
            JSON.stringify({ chosenId: pair.chosenId, rejectedId: pair.rejectedId }),
            JSON.stringify({ paired: true }),
            JSON.stringify({ pairId: pair.pairId }),
          ]).catch(err => {
            log.debug('learning:dpo_pair → unified_signals failed', { error: err.message });
          });

          _stats.dpoPairsConsumed = (_stats.dpoPairsConsumed || 0) + 1;
        } catch (err) {
          log.debug('learning:dpo_pair consumer error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubLearningDpoPair);
  }

  // 8g. accounting:update → unified_signals (consolidated accounting archive)
  // Multiple accountants publish this — archive all accounting updates
  if (persistence?.query) {
    const unsubAccountingUpdate = globalEventBus.subscribe(
      EventType.ACCOUNTING_UPDATE,
      (event) => {
        try {
          const d = event.payload || {};
          const id = `au_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          persistence.query(`
            INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
          `, [
            id, 'accounting_update', context.sessionId || null,
            JSON.stringify({ source: d.source, type: d.type }),
            JSON.stringify({ interactions: d.interactions, value: d.value }),
            JSON.stringify({ timestamp: Date.now() }),
          ]).catch(err => {
            log.debug('accounting:update persistence failed', { error: err.message });
          });

          _stats.accountingUpdatesConsumed = (_stats.accountingUpdatesConsumed || 0) + 1;
        } catch (err) {
          log.debug('accounting:update consumer error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubAccountingUpdate);
  }

  if (socialAccountant || solanaAccountant || persistence?.query) {
    log.info('Orphan event consumers wired: cynic:action, solana:action, solana:learning, solana:emergence, learning:signal, learning:dpo_pair, accounting:update');
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // 9. REMAINING ORPHAN CONSUMERS (second wave)
  // ═══════════════════════════════════════════════════════════════════════════

  // 9a. human:perceived → HumanJudge: Feed perception data to judgment pipeline (C5.1→C5.2)
  // HumanPerceiver publishes on significant energy/frustration deltas, but nobody consumed it.
  if (humanJudge) {
    const unsubHumanPerceived = globalEventBus.subscribe(
      'human:perceived',
      (event) => {
        try {
          const d = event.payload || {};

          // Feed perception directly to HumanJudge (reactive, not just periodic)
          humanJudge.judge({
            energy: d.energy || 0.5,
            focus: d.focus || 0.5,
            frustration: d.frustration || 0,
            burnoutRisk: d.burnoutRisk || 0,
            sessionMinutes: d.sessionDuration ? Math.floor(d.sessionDuration / 60000) : 0,
          }, { source: 'human:perceived', sessionId: context.sessionId });

          _stats.humanPerceptionsFed = (_stats.humanPerceptionsFed || 0) + 1;
        } catch (err) {
          log.debug('human:perceived → HumanJudge failed', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubHumanPerceived);
    log.info('HumanJudge (C5.2) wired to human:perceived (reactive C5.1→C5.2)');
  }

  // 9b. cynic:self_heal → unified_signals + homeostasis (C6.4 self-repair tracking)
  // CynicActor emits self-healing actions — track for learning what repairs worked
  {
    const unsubSelfHeal = globalEventBus.subscribe(
      'cynic:self_heal',
      (event) => {
        try {
          const d = event.payload || {};

          // Feed to homeostasis — self-healing lowers system entropy
          if (homeostasis) {
            homeostasis.update('selfHealingRate', 0.618); // self-healing = positive health signal
            _stats.homeostasisObservations++;
          }

          // Persist to unified_signals
          if (persistence?.query) {
            const id = `sh_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            persistence.query(`
              INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
              VALUES ($1, $2, $3, $4, $5, $6)
            `, [
              id, 'cynic_self_heal', context.sessionId || null,
              JSON.stringify({ action: d.action, context: d.context }),
              JSON.stringify({ healthState: d.healthState }),
              JSON.stringify({ cell: 'C6.4', timestamp: d.ts }),
            ]).catch(err => {
              log.debug('cynic:self_heal persistence failed', { error: err.message });
            });
          }

          _stats.selfHealsConsumed = (_stats.selfHealsConsumed || 0) + 1;
        } catch (err) {
          log.debug('cynic:self_heal consumer error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubSelfHeal);
  }

  // 9c. cynic:threshold_adjustment → homeostasis + unified_signals (C6.4 calibration tracking)
  // CynicActor adjusts routing/judge thresholds — feed to homeostasis for stability tracking
  {
    const unsubThresholdAdj = globalEventBus.subscribe(
      'cynic:threshold_adjustment',
      (event) => {
        try {
          const d = event.payload || {};

          // Feed to homeostasis — threshold drift indicates system adaptation
          if (homeostasis) {
            homeostasis.update('thresholdDrift', d.urgency === 'high' ? 0.3 : 0.618);
            _stats.homeostasisObservations++;
          }

          // Persist to unified_signals
          if (persistence?.query) {
            const id = `ta_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            persistence.query(`
              INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
              VALUES ($1, $2, $3, $4, $5, $6)
            `, [
              id, 'threshold_adjustment', context.sessionId || null,
              JSON.stringify({ reason: d.reason, recommendation: d.recommendation }),
              JSON.stringify({ urgency: d.urgency }),
              JSON.stringify({ cell: 'C6.4', source: d.source }),
            ]).catch(err => {
              log.debug('cynic:threshold_adjustment persistence failed', { error: err.message });
            });
          }

          _stats.thresholdAdjustmentsConsumed = (_stats.thresholdAdjustmentsConsumed || 0) + 1;
        } catch (err) {
          log.debug('cynic:threshold_adjustment consumer error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubThresholdAdj);
  }

  // 9d. solana:accounting → unified_signals (C2.6 archive)
  // SolanaAccountant publishes transaction records — archive for analytics
  if (persistence?.query) {
    const unsubSolanaAccounting = globalEventBus.subscribe(
      'solana:accounting',
      (event) => {
        try {
          const d = event.payload || {};
          const id = `sa_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          persistence.query(`
            INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
          `, [
            id, 'solana_accounting', context.sessionId || null,
            JSON.stringify({ transaction: d.transaction }),
            JSON.stringify({ cell: d.cell, dimension: d.dimension }),
            JSON.stringify({ analysis: d.analysis, timestamp: d.timestamp }),
          ]).catch(err => {
            log.debug('solana:accounting persistence failed', { error: err.message });
          });

          _stats.solanaAccountingConsumed = (_stats.solanaAccountingConsumed || 0) + 1;
        } catch (err) {
          log.debug('solana:accounting consumer error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubSolanaAccounting);
  }

  // 9e. component:error → unified_signals (error monitoring archive)
  // 7 components publish errors — archive for pattern detection
  if (persistence?.query) {
    const unsubComponentError = globalEventBus.subscribe(
      EventType.COMPONENT_ERROR,
      (event) => {
        try {
          const d = event.payload || {};
          const id = `ce_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          persistence.query(`
            INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
          `, [
            id, 'component_error', context.sessionId || null,
            JSON.stringify({ component: d.component, operation: d.operation }),
            JSON.stringify({ error: d.error }),
            JSON.stringify({ eventId: d.eventId, timestamp: Date.now() }),
          ]).catch(err => {
            log.debug('component:error persistence failed', { error: err.message });
          });

          _stats.componentErrorsConsumed = (_stats.componentErrorsConsumed || 0) + 1;
        } catch (err) {
          log.debug('component:error consumer error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubComponentError);
  }

  // 9f. solana:emergence:immediate → PATTERN_DETECTED bridge (urgent Solana anomalies)
  // Same as solana:emergence but for time-critical patterns
  {
    const unsubSolanaEmergenceImmediate = globalEventBus.subscribe(
      'solana:emergence:immediate',
      (event) => {
        try {
          const d = event.payload || {};
          const pattern = d.pattern || d;

          // Always bridge immediate emergence to PATTERN_DETECTED
          globalEventBus.publish(EventType.PATTERN_DETECTED, {
            source: 'SolanaEmergence:immediate',
            key: pattern.type || pattern.key || 'solana_urgent',
            significance: 'critical',
            category: 'solana',
            urgent: true,
            ...pattern,
          }, { source: 'solana-emergence-immediate-bridge' });

          _stats.solanaEmergenceImmediateConsumed = (_stats.solanaEmergenceImmediateConsumed || 0) + 1;
        } catch (err) {
          log.debug('solana:emergence:immediate consumer error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubSolanaEmergenceImmediate);
  }

  if (humanJudge || homeostasis || persistence?.query) {
    log.info('Second-wave orphan consumers wired: human:perceived, cynic:self_heal, cynic:threshold_adjustment, solana:accounting, component:error, solana:emergence:immediate');
  }

  // ═════════════════════════════════════════════════════════════════════════════
  // SECTION 10: BRIDGE-FORWARDED ORPHAN CONSUMERS
  // Events published by EventBusBridge (agent→core) with zero subscribers
  // ═════════════════════════════════════════════════════════════════════════════

  // 10a. cynic:guidance → persist + homeostasis (C6.3 guidance from dogs)
  //      Published when dogs emit guidance signals via agent bus → EventBusBridge
  {
    const unsubCynicGuidance = globalEventBus.subscribe(
      'cynic:guidance',
      (event) => {
        try {
          const d = event.payload || event;
          if (homeostasis) {
            homeostasis.update('guidanceFrequency', 1);
            _stats.homeostasisObservations++;
          }
          if (persistence?.query) {
            const id = `cg_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            persistence.query(`
              INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
              VALUES ($1, $2, $3, $4, $5, $6)
            `, [
              id, 'cynic_guidance', sessionId || null,
              JSON.stringify({ type: d.type || 'guidance', source: d.source }),
              JSON.stringify({ guidance: d.guidance || d.message }),
              JSON.stringify({ dogs: d.dogs, confidence: d.confidence }),
            ]).catch(() => {});
          }
        } catch (err) {
          log.debug('cynic:guidance consumer error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCynicGuidance);
  }

  // 10b. cynic:override → persist + homeostasis (C6.4 manual override)
  //      Published when dogs force an override via agent bus → EventBusBridge
  {
    const unsubCynicOverride = globalEventBus.subscribe(
      'cynic:override',
      (event) => {
        try {
          const d = event.payload || event;
          if (homeostasis) {
            homeostasis.update('overrideFrequency', 1);
            _stats.homeostasisObservations++;
          }
          if (persistence?.query) {
            const id = `co_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            persistence.query(`
              INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
              VALUES ($1, $2, $3, $4, $5, $6)
            `, [
              id, 'cynic_override', sessionId || null,
              JSON.stringify({ type: d.type || 'override', reason: d.reason }),
              JSON.stringify({ action: d.action, target: d.target }),
              JSON.stringify({ dogs: d.dogs, urgency: d.urgency }),
            ]).catch(() => {});
          }
        } catch (err) {
          log.debug('cynic:override consumer error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubCynicOverride);
  }

  // 10c. vulnerability:detected → PATTERN_DETECTED bridge + persist (security)
  //      Published when agent dogs detect security vulnerabilities
  {
    const unsubVulnerability = globalEventBus.subscribe(
      'vulnerability:detected',
      (event) => {
        try {
          const d = event.payload || event;
          // Bridge to pattern pipeline for downstream handling
          globalEventBus.publish(EventType.PATTERN_DETECTED, {
            source: 'SecurityGuardian',
            key: 'vulnerability_detected',
            significance: 'high',
            category: 'security',
            vulnerability: d.type || d.vulnerability,
            severity: d.severity,
            ...d,
          }, { source: 'event-listeners' });
          if (persistence?.query) {
            const id = `vd_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            persistence.query(`
              INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
              VALUES ($1, $2, $3, $4, $5, $6)
            `, [
              id, 'vulnerability_detected', sessionId || null,
              JSON.stringify({ type: d.type, severity: d.severity }),
              JSON.stringify({ vulnerability: d.vulnerability }),
              JSON.stringify({ file: d.file, line: d.line }),
            ]).catch(() => {});
          }
        } catch (err) {
          log.debug('vulnerability:detected consumer error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubVulnerability);
  }

  // 10d. reality:drift → homeostasis + ANOMALY_DETECTED bridge (drift = instability)
  //      Published when agent dogs detect reality divergence from expected state
  {
    const unsubRealityDrift = globalEventBus.subscribe(
      'reality:drift',
      (event) => {
        try {
          const d = event.payload || event;
          if (homeostasis) {
            homeostasis.update('realityDrift', d.magnitude || 0.5);
            _stats.homeostasisObservations++;
          }
          // Bridge to anomaly pipeline
          globalEventBus.publish(EventType.ANOMALY_DETECTED, {
            source: 'RealityDrift',
            type: 'reality_drift',
            severity: d.severity || 'medium',
            dimension: d.dimension,
            magnitude: d.magnitude,
          }, { source: 'event-listeners' });
        } catch (err) {
          log.debug('reality:drift consumer error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubRealityDrift);
  }

  // 10e. deploy:completed / deploy:failed → unified_signals (deployment tracking)
  //      Published when agent dogs detect deployment lifecycle events
  if (persistence?.query) {
    const unsubDeployCompleted = globalEventBus.subscribe(
      'deploy:completed',
      (event) => {
        try {
          const d = event.payload || event;
          const id = `dc_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          persistence.query(`
            INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
          `, [
            id, 'deploy_completed', sessionId || null,
            JSON.stringify({ service: d.service, version: d.version }),
            JSON.stringify({ status: 'completed', duration: d.duration }),
            JSON.stringify({ environment: d.environment }),
          ]).catch(() => {});
        } catch (err) {
          log.debug('deploy:completed persist error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubDeployCompleted);

    const unsubDeployFailed = globalEventBus.subscribe(
      'deploy:failed',
      (event) => {
        try {
          const d = event.payload || event;
          const id = `df_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          persistence.query(`
            INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
          `, [
            id, 'deploy_failed', sessionId || null,
            JSON.stringify({ service: d.service, version: d.version }),
            JSON.stringify({ status: 'failed', error: d.error }),
            JSON.stringify({ environment: d.environment }),
          ]).catch(() => {});
        } catch (err) {
          log.debug('deploy:failed persist error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubDeployFailed);
  }

  // 10f. tool:called → unified_signals (tool usage tracking, complement to tool:completed)
  //      Published by CollectivePack.receiveHookEvent() on PreToolUse
  {
    const unsubToolCalled = globalEventBus.subscribe(
      EventType.TOOL_CALLED,
      (event) => {
        try {
          const d = event.payload || event;
          if (persistence?.query) {
            const id = `tc_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            persistence.query(`
              INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
              VALUES ($1, $2, $3, $4, $5, $6)
            `, [
              id, 'tool_called', sessionId || null,
              JSON.stringify({ tool: d.tool, hookType: d.hookType }),
              JSON.stringify({ blocked: d.blocked || false }),
              JSON.stringify({ userId: d.userId }),
            ]).catch(() => {});
          }
        } catch (err) {
          log.debug('tool:called persist error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubToolCalled);
  }

  // 10g. component:ready → SystemTopology registration (self-awareness)
  //      Published when components finish initialization
  {
    const unsubComponentReady = globalEventBus.subscribe(
      EventType.COMPONENT_READY,
      (event) => {
        try {
          const d = event.payload || event;
          log.debug('Component ready', { component: d.name || d.component || event.source });
        } catch (err) {
          log.debug('component:ready handler error', { error: err.message });
        }
      }
    );
    _unsubscribers.push(unsubComponentReady);
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

// ═══════════════════════════════════════════════════════════════════════════════
// SOLANA PIPELINE LATE-BINDING (C2.2-C2.7)
// "On-chain is truth" — can be called from startEventListeners() OR late from
// collective-singleton after SolanaWatcher starts and C2.2-C2.7 are initialized.
// Guard prevents double-wiring.
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Wire Solana pipeline event subscriptions.
 *
 * Safe to call multiple times — guarded by _solanaWired flag.
 * Called from startEventListeners() (if singletons available at start time)
 * or late from collective-singleton.js (after async SolanaWatcher init).
 *
 * @param {Object} opts
 * @param {Object} [opts.solanaJudge] - SolanaJudge singleton
 * @param {Object} [opts.solanaDecider] - SolanaDecider singleton
 * @param {Object} [opts.solanaActor] - SolanaActor singleton
 * @param {Object} [opts.solanaLearner] - SolanaLearner singleton
 * @param {Object} [opts.solanaAccountant] - SolanaAccountant singleton
 * @param {Object} [opts.solanaEmergence] - SolanaEmergence singleton
 * @param {Object} [opts.persistence] - Persistence layer for unified_signals
 * @param {string} [opts.sessionId] - Session ID for persistence
 */
export function wireSolanaEventListeners({
  solanaJudge,
  solanaDecider,
  solanaActor,
  solanaLearner,
  solanaAccountant,
  solanaEmergence,
  persistence = null,
  sessionId = null,
} = {}) {
  if (_solanaWired) {
    log.debug('Solana event listeners already wired — skipping');
    return;
  }
  if (!solanaJudge) {
    log.debug('Solana event listeners skipped (no solanaJudge)');
    return;
  }

  _solanaWired = true;

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

          // Publish solana:action for chain completeness (same pattern as cynic:action)
          globalEventBus.publish('solana:action', {
            source: 'SolanaActor',
            decision: { type: decision.type || decision.action, executed: isLive },
          }, { source: 'event-listeners' });

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
            id, 'solana_judgment', sessionId || null,
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
            id, 'solana_decision', sessionId || null,
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

  // ═════════════════════════════════════════════════════════════════════════════
  // 2g. perception:solana:connected → SolanaAccountant + SolanaEmergence (C2.1→C2.6/C2.7)
  //     Track uptime windows, initialize baseline metrics
  // ═════════════════════════════════════════════════════════════════════════════
  const unsubSolanaConnected = globalEventBus.subscribe(
    'perception:solana:connected',
    (event) => {
      try {
        const d = event.payload || event;
        if (solanaAccountant) {
          solanaAccountant.recordTransaction({
            type: 'connection_established',
            slot: d.slot,
            timestamp: d.timestamp || Date.now(),
          });
          _stats.solanaAccountingOps++;
        }
        if (solanaEmergence) {
          solanaEmergence.recordActivity({
            type: 'connection',
            slot: d.slot,
            transactionCount: 0,
          });
        }
      } catch (err) {
        log.debug('Solana connected handler error', { error: err.message });
      }
    }
  );
  _unsubscribers.push(unsubSolanaConnected);

  // ═════════════════════════════════════════════════════════════════════════════
  // 2h. perception:solana:disconnected → SolanaAccountant (C2.1→C2.6)
  //     Record downtime, close connection session
  // ═════════════════════════════════════════════════════════════════════════════
  const unsubSolanaDisconnected = globalEventBus.subscribe(
    'perception:solana:disconnected',
    (event) => {
      try {
        if (solanaAccountant) {
          solanaAccountant.recordTransaction({
            type: 'connection_lost',
            timestamp: event.payload?.timestamp || Date.now(),
          });
          _stats.solanaAccountingOps++;
        }
      } catch (err) {
        log.debug('Solana disconnected handler error', { error: err.message });
      }
    }
  );
  _unsubscribers.push(unsubSolanaDisconnected);

  // ═════════════════════════════════════════════════════════════════════════════
  // 2i. perception:solana:error → SolanaEmergence (C2.1→C2.7)
  //     Feed error clusters for anomaly detection
  // ═════════════════════════════════════════════════════════════════════════════
  const unsubSolanaError = globalEventBus.subscribe(
    'perception:solana:error',
    (event) => {
      try {
        const d = event.payload || event;
        if (solanaEmergence) {
          solanaEmergence.recordActivity({
            type: 'error',
            error: d.error,
            transactionCount: 0,
          });
        }
        // Persist error for pattern analysis
        if (persistence?.query) {
          const id = `se_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          persistence.query(`
            INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
          `, [
            id, 'solana_perception_error', sessionId || null,
            JSON.stringify({ error: d.error }),
            JSON.stringify({ type: 'error' }),
            JSON.stringify({ stack: d.stack }),
          ]).catch(() => {});
        }
      } catch (err) {
        log.debug('Solana error handler error', { error: err.message });
      }
    }
  );
  _unsubscribers.push(unsubSolanaError);

  // ═════════════════════════════════════════════════════════════════════════════
  // 2j. perception:solana:log → SolanaJudge + SolanaLearner (C2.1→C2.2/C2.5)
  //     Judge transaction quality from logs, learn from patterns
  // ═════════════════════════════════════════════════════════════════════════════
  const unsubSolanaLog = globalEventBus.subscribe(
    'perception:solana:log',
    (event) => {
      try {
        const d = event.payload || event;
        // Judge transaction if it has a signature (completed tx)
        if (d.signature) {
          const judgment = solanaJudge.judgeTransaction({
            signature: d.signature,
            slot: d.slot,
            error: d.error,
            hasLogs: Array.isArray(d.logs) && d.logs.length > 0,
          });
          _stats.solanaJudgments++;
          // Publish judgment for downstream pipeline
          globalEventBus.publish('solana:judgment', {
            type: 'transaction_log',
            judgment,
            signature: d.signature,
            slot: d.slot,
          }, { source: 'event-listeners' });
        }
        // Feed learner with log patterns
        if (solanaLearner && d.logs?.length > 0) {
          solanaLearner.recordOutcome({
            type: 'log_analysis',
            signature: d.signature,
            hasError: !!d.error,
            logCount: d.logs.length,
            timestamp: Date.now(),
          });
          _stats.solanaLearnings++;
        }
      } catch (err) {
        log.debug('Solana log handler error', { error: err.message });
      }
    }
  );
  _unsubscribers.push(unsubSolanaLog);

  // ═════════════════════════════════════════════════════════════════════════════
  // 2k. perception:solana:program → SolanaJudge + SolanaEmergence (C2.1→C2.2/C2.7)
  //     Judge program activity, detect program surge patterns
  // ═════════════════════════════════════════════════════════════════════════════
  const unsubSolanaProgram = globalEventBus.subscribe(
    'perception:solana:program',
    (event) => {
      try {
        const d = event.payload || event;
        // Judge program state
        const judgment = solanaJudge.judgeProgram({
          programId: d.programId,
          pubkey: d.pubkey,
          slot: d.slot,
          lamports: d.lamports,
          dataLength: d.dataLength,
        });
        _stats.solanaJudgments++;
        // Feed emergence with program activity
        if (solanaEmergence) {
          solanaEmergence.recordActivity({
            type: 'program_change',
            programId: d.programId,
            slot: d.slot,
            transactionCount: 1,
          });
        }
      } catch (err) {
        log.debug('Solana program handler error', { error: err.message });
      }
    }
  );
  _unsubscribers.push(unsubSolanaProgram);

  // ═════════════════════════════════════════════════════════════════════════════
  // 2l. perception:solana:signature → SolanaAccountant + SolanaLearner (C2.1→C2.6/C2.5)
  //     Track confirmation success rate, feed feedback loop
  // ═════════════════════════════════════════════════════════════════════════════
  const unsubSolanaSignature = globalEventBus.subscribe(
    'perception:solana:signature',
    (event) => {
      try {
        const d = event.payload || event;
        if (solanaAccountant) {
          solanaAccountant.recordTransaction({
            type: 'signature_confirmation',
            signature: d.signature,
            confirmed: d.confirmed,
            slot: d.slot,
            timestamp: d.timestamp || Date.now(),
          });
          _stats.solanaAccountingOps++;
        }
        if (solanaLearner) {
          solanaLearner.recordOutcome({
            type: 'confirmation',
            signature: d.signature,
            confirmed: d.confirmed,
            hasError: !!d.error,
            slot: d.slot,
            timestamp: Date.now(),
          });
          _stats.solanaLearnings++;
        }
      } catch (err) {
        log.debug('Solana signature handler error', { error: err.message });
      }
    }
  );
  _unsubscribers.push(unsubSolanaSignature);

  // ═════════════════════════════════════════════════════════════════════════════
  // 2m. perception:solana:state → unified_signals persistence (C2.1 health snapshot)
  //     Archive C2.1 perception health for trend analysis
  // ═════════════════════════════════════════════════════════════════════════════
  const unsubSolanaState = globalEventBus.subscribe(
    'perception:solana:state',
    (event) => {
      try {
        const d = event.payload || event;
        if (persistence?.query) {
          const id = `ss_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
          persistence.query(`
            INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
          `, [
            id, 'solana_perception_state', sessionId || null,
            JSON.stringify({ cell: d.cell, lastSlot: d.lastSlot }),
            JSON.stringify({ status: d.health?.status, score: d.health?.score }),
            JSON.stringify({
              subscriptions: d.subscriptions?.length || 0,
              isRunning: d.health?.isRunning,
              errorRate: d.health?.errorRate,
            }),
          ]).catch(() => {});
        }
      } catch (err) {
        log.debug('Solana state handler error', { error: err.message });
      }
    }
  );
  _unsubscribers.push(unsubSolanaState);

  log.info('Solana pipeline event listeners wired (C2.1-C2.7)', {
    judge: !!solanaJudge,
    decider: !!solanaDecider,
    actor: !!solanaActor,
    learner: !!solanaLearner,
    accountant: !!solanaAccountant,
    emergence: !!solanaEmergence,
    perceptionListeners: 9, // slot, account + 7 new
  });
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
  if (_cynicEmergenceInterval) {
    clearInterval(_cynicEmergenceInterval);
    _cynicEmergenceInterval = null;
  }
  if (_socialEmergenceInterval) {
    clearInterval(_socialEmergenceInterval);
    _socialEmergenceInterval = null;
  }
  if (_cosmosEmergenceInterval) {
    clearInterval(_cosmosEmergenceInterval);
    _cosmosEmergenceInterval = null;
  }
  if (_humanJudgeInterval) {
    clearInterval(_humanJudgeInterval);
    _humanJudgeInterval = null;
  }
  if (_cosmosJudgeInterval) {
    clearInterval(_cosmosJudgeInterval);
    _cosmosJudgeInterval = null;
  }
  if (_socialJudgeInterval) {
    clearInterval(_socialJudgeInterval);
    _socialJudgeInterval = null;
  }
  if (_cynicJudgeInterval) {
    clearInterval(_cynicJudgeInterval);
    _cynicJudgeInterval = null;
  }

  _unsubscribers = [];
  _started = false;
  _solanaWired = false;

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
