/**
 * CYNIC Self-Emergence - C6.7 (CYNIC × EMERGE)
 *
 * Detects emergent patterns in CYNIC's own behavior over time.
 * Part of the 7×7 Fractal Matrix emergence layer.
 *
 * "Le chien s'observe lui-même" - κυνικός
 *
 * Emerges:
 * - Dog dominance shifts (which Dog leads over time)
 * - Consensus quality trends (approval rates, veto frequency)
 * - Collective health trajectories
 * - Learning velocity changes
 * - Memory pressure patterns
 *
 * @module @cynic/node/emergence/cynic-emergence
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, createLogger, globalEventBus, EventType } from '@cynic/core';
import { getUnifiedSignalStore, SignalSource } from '../learning/unified-signal.js';

const log = createLogger('CynicEmergence');

/**
 * Emergent pattern types for CYNIC self-awareness
 */
export const CynicPatternType = {
  DOG_DOMINANCE_SHIFT: 'dog_dominance_shift',
  CONSENSUS_QUALITY_CHANGE: 'consensus_quality_change',
  COLLECTIVE_HEALTH_TREND: 'collective_health_trend',
  LEARNING_VELOCITY_CHANGE: 'learning_velocity_change',
  MEMORY_PRESSURE: 'memory_pressure',
  GUARDIAN_ESCALATION: 'guardian_escalation',
  DOG_SILENCE: 'dog_silence',
  PATTERN_ACCELERATION: 'pattern_acceleration',
};

export const SignificanceLevel = {
  LOW: 'low',
  MEDIUM: 'medium',
  HIGH: 'high',
  CRITICAL: 'critical',
};

const THRESHOLDS = {
  dominanceShiftRatio: PHI_INV,    // 61.8% of events from one Dog = dominance
  consensusMinApproval: PHI_INV_2, // Below 38.2% approval = quality concern
  healthDeclineRate: -0.1,         // 10% decline = trend
  silenceWindow: 10,               // 10+ events without a Dog = silence
  minDataPoints: 5,                // Minimum data before detecting patterns
  maxHistory: 500,                 // Rolling window size
};

/**
 * CynicEmergence - Self-awareness emergence detector
 */
export class CynicEmergence extends EventEmitter {
  constructor(options = {}) {
    super();

    // Rolling history windows
    this._dogEvents = [];       // { dog, eventType, timestamp }
    this._consensusResults = []; // { approved, agreement, vetoCount, timestamp }
    this._healthSnapshots = [];  // { avgHealth, memoryLoad, patternCount, timestamp }

    // Baselines (rolling averages)
    this._baselines = {
      dogDistribution: {},      // dog → frequency ratio
      approvalRate: 0.5,
      avgHealth: 0.5,
      memoryLoad: 0.3,
    };

    // Detected patterns
    this._patterns = [];
    this._maxPatterns = 100;

    // Stats
    this._stats = {
      dogEventsRecorded: 0,
      consensusRecorded: 0,
      healthSnapshotsRecorded: 0,
      patternsDetected: 0,
      analysesRun: 0,
      lastAnalysis: null,
    };
  }

  /**
   * Record a Dog event for emergence analysis
   */
  recordDogEvent(data) {
    const entry = {
      dog: data.dog || data.dogName || 'unknown',
      eventType: data.eventType || data.type || 'invocation',
      timestamp: Date.now(),
    };

    this._dogEvents.push(entry);
    if (this._dogEvents.length > THRESHOLDS.maxHistory) {
      this._dogEvents.shift();
    }
    this._stats.dogEventsRecorded++;
  }

  /**
   * Record a consensus result
   */
  recordConsensus(data) {
    const entry = {
      approved: data.approved !== false,
      agreement: data.agreement || data.agreementRatio || 0,
      vetoCount: data.vetoCount || data.vetoes || 0,
      dogCount: data.dogCount || data.voterCount || 0,
      timestamp: Date.now(),
    };

    this._consensusResults.push(entry);
    if (this._consensusResults.length > THRESHOLDS.maxHistory) {
      this._consensusResults.shift();
    }
    this._stats.consensusRecorded++;
  }

  /**
   * Record a collective health snapshot
   */
  recordHealthSnapshot(data) {
    const entry = {
      avgHealth: data.avgHealth || data.averageHealth || 0.5,
      memoryLoad: data.memoryLoad || 0,
      patternCount: data.patternCount || 0,
      dogCount: data.dogCount || data.activeDogs || 0,
      timestamp: Date.now(),
    };

    this._healthSnapshots.push(entry);
    if (this._healthSnapshots.length > THRESHOLDS.maxHistory) {
      this._healthSnapshots.shift();
    }
    this._stats.healthSnapshotsRecorded++;
  }

  /**
   * Accumulate cycle data for pattern detection
   * This is the main entry point for C6.7 (CYNIC × EMERGE)
   *
   * @param {Object} data - Cycle data
   * @param {string} data.cycleId - Unique cycle identifier
   * @param {Object} data.judgment - Judgment results
   * @param {Object} data.decision - Decision made
   * @param {Object} data.outcome - Outcome (success/failure)
   * @param {Object} [data.metrics] - Optional metrics (heap, latency, etc)
   */
  accumulate({ cycleId, judgment, decision, outcome, metrics = {} }) {
    const timestamp = Date.now();

    // Accumulate data into rolling buffers
    if (judgment) {
      this.recordDogEvent({
        dog: judgment.dog || 'unknown',
        eventType: judgment.verdict || 'judgment',
        timestamp,
      });
    }

    if (decision) {
      this.recordConsensus({
        approved: decision.approved !== false,
        agreement: decision.agreementRatio || decision.agreement || 0,
        vetoCount: decision.vetoCount || 0,
        dogCount: decision.voterCount || decision.dogCount || 0,
        timestamp,
      });
    }

    if (metrics) {
      this.recordHealthSnapshot({
        avgHealth: metrics.health || metrics.avgHealth || 0.5,
        memoryLoad: metrics.heapUsed || metrics.memoryLoad || 0,
        patternCount: this._patterns.length,
        dogCount: metrics.activeDogs || 0,
        timestamp,
      });
    }

    // Run analysis every 10 cycles (configurable)
    const minCyclesBeforeAnalysis = 10;
    if (this._dogEvents.length >= minCyclesBeforeAnalysis) {
      const newPatterns = this.analyze();

      // Emit detected patterns to unified_signals
      if (newPatterns.length > 0) {
        this._emitPatternsToUnifiedSignals(newPatterns, cycleId);
      }
    }
  }

  /**
   * Emit detected patterns to unified_signals table
   * @private
   */
  _emitPatternsToUnifiedSignals(patterns, cycleId) {
    const store = getUnifiedSignalStore();

    for (const pattern of patterns) {
      // Emit to global event bus for immediate consumption
      globalEventBus.publish(EventType.CYNIC_EMERGENCE, {
        pattern,
        cycleId,
        cell: 'C6.7',
        dimension: 'CYNIC',
        analysis: 'EMERGE',
      });

      // Persist to unified_signals for learning
      try {
        store.record({
          source: SignalSource.PATTERN,
          sessionId: cycleId,
          itemType: 'cynic_pattern',
          itemContent: pattern.message,
          metadata: {
            patternType: pattern.type,
            significance: pattern.significance,
            confidence: pattern.confidence,
            data: pattern.data,
            cell: 'C6.7',
          },
          outcome: pattern.significance === SignificanceLevel.CRITICAL ? 'critical' : 'success',
        });
      } catch (error) {
        log.error('Failed to persist pattern to unified_signals', { error: error.message });
      }
    }

    log.info('Patterns emitted to unified_signals', {
      count: patterns.length,
      cycleId,
      types: patterns.map(p => p.type),
    });
  }

  /**
   * Run full emergence analysis
   */
  analyze() {
    this._stats.analysesRun++;
    this._stats.lastAnalysis = Date.now();

    const newPatterns = [];

    // 1. Dog dominance shift detection
    const dominance = this._detectDogDominance();
    if (dominance) newPatterns.push(dominance);

    // 2. Dog silence detection
    const silence = this._detectDogSilence();
    if (silence) newPatterns.push(silence);

    // 3. Consensus quality trend
    const consensus = this._detectConsensusQuality();
    if (consensus) newPatterns.push(consensus);

    // 4. Guardian escalation pattern
    const escalation = this._detectGuardianEscalation();
    if (escalation) newPatterns.push(escalation);

    // 5. Collective health trajectory
    const health = this._detectHealthTrajectory();
    if (health) newPatterns.push(health);

    // 6. Memory pressure
    const memory = this._detectMemoryPressure();
    if (memory) newPatterns.push(memory);

    // Store and emit new patterns
    for (const pattern of newPatterns) {
      this._registerPattern(pattern);
    }

    return newPatterns;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Pattern Detection
  // ═══════════════════════════════════════════════════════════════════════════

  _detectDogDominance() {
    if (this._dogEvents.length < THRESHOLDS.minDataPoints) return null;

    const counts = {};
    for (const e of this._dogEvents) {
      counts[e.dog] = (counts[e.dog] || 0) + 1;
    }

    const total = this._dogEvents.length;
    const sorted = Object.entries(counts).sort((a, b) => b[1] - a[1]);
    if (sorted.length === 0) return null;

    const [topDog, topCount] = sorted[0];
    const ratio = topCount / total;

    if (ratio >= THRESHOLDS.dominanceShiftRatio) {
      // Check if this is a NEW dominance (different from baseline)
      const prevTop = Object.entries(this._baselines.dogDistribution)
        .sort((a, b) => b[1] - a[1])[0];

      if (!prevTop || prevTop[0] !== topDog) {
        // φ-bound confidence: scale ratio to range [0, φ⁻¹]
        const phiBoundedConfidence = Math.floor(ratio * PHI_INV * 1000) / 1000; // Round down to avoid float issues
        return {
          type: CynicPatternType.DOG_DOMINANCE_SHIFT,
          significance: ratio > 0.8 ? SignificanceLevel.HIGH : SignificanceLevel.MEDIUM,
          data: { dominantDog: topDog, ratio, previousDominant: prevTop?.[0] || 'none' },
          confidence: Math.min(PHI_INV, phiBoundedConfidence),
          message: `${topDog} dominates ${(ratio * 100).toFixed(0)}% of events`,
        };
      }
    }

    // Update baseline
    for (const [dog, count] of Object.entries(counts)) {
      this._baselines.dogDistribution[dog] = count / total;
    }
    return null;
  }

  _detectDogSilence() {
    if (this._dogEvents.length < THRESHOLDS.minDataPoints) return null;

    // Check which Dogs haven't fired recently
    const recentDogs = new Set(
      this._dogEvents.slice(-THRESHOLDS.silenceWindow).map(e => e.dog)
    );
    const allDogs = new Set(this._dogEvents.map(e => e.dog));
    const silentDogs = [...allDogs].filter(d => !recentDogs.has(d));

    if (silentDogs.length > 0 && silentDogs.length >= allDogs.size * PHI_INV_2) {
      return {
        type: CynicPatternType.DOG_SILENCE,
        significance: SignificanceLevel.MEDIUM,
        data: { silentDogs, activeDogs: [...recentDogs], windowSize: THRESHOLDS.silenceWindow },
        confidence: PHI_INV_2,
        message: `${silentDogs.length} Dogs silent: ${silentDogs.join(', ')}`,
      };
    }
    return null;
  }

  _detectConsensusQuality() {
    if (this._consensusResults.length < THRESHOLDS.minDataPoints) return null;

    const recent = this._consensusResults.slice(-20);
    const approvalRate = recent.filter(c => c.approved).length / recent.length;
    const avgAgreement = recent.reduce((s, c) => s + c.agreement, 0) / recent.length;

    if (approvalRate < THRESHOLDS.consensusMinApproval) {
      return {
        type: CynicPatternType.CONSENSUS_QUALITY_CHANGE,
        significance: approvalRate < 0.2 ? SignificanceLevel.CRITICAL : SignificanceLevel.HIGH,
        data: { approvalRate, avgAgreement, samples: recent.length },
        confidence: Math.min(recent.length / 20, PHI_INV),
        message: `Consensus approval at ${(approvalRate * 100).toFixed(0)}% (threshold: ${(THRESHOLDS.consensusMinApproval * 100).toFixed(0)}%)`,
      };
    }

    this._baselines.approvalRate = approvalRate;
    return null;
  }

  _detectGuardianEscalation() {
    if (this._consensusResults.length < THRESHOLDS.minDataPoints) return null;

    const recent = this._consensusResults.slice(-20);
    const vetoRate = recent.filter(c => c.vetoCount > 0).length / recent.length;

    if (vetoRate > PHI_INV_2) {
      return {
        type: CynicPatternType.GUARDIAN_ESCALATION,
        significance: vetoRate > PHI_INV ? SignificanceLevel.HIGH : SignificanceLevel.MEDIUM,
        data: { vetoRate, totalVetoes: recent.reduce((s, c) => s + c.vetoCount, 0) },
        confidence: Math.min(PHI_INV, vetoRate * PHI_INV), // φ-bound: scale to max φ⁻¹
        message: `Guardian veto rate ${(vetoRate * 100).toFixed(0)}% — high vigilance`,
      };
    }
    return null;
  }

  _detectHealthTrajectory() {
    if (this._healthSnapshots.length < THRESHOLDS.minDataPoints) return null;

    const recent = this._healthSnapshots.slice(-10);
    const older = this._healthSnapshots.slice(-20, -10);
    if (older.length === 0) return null;

    const recentAvg = recent.reduce((s, h) => s + h.avgHealth, 0) / recent.length;
    const olderAvg = older.reduce((s, h) => s + h.avgHealth, 0) / older.length;
    const delta = recentAvg - olderAvg;

    if (delta < THRESHOLDS.healthDeclineRate) {
      return {
        type: CynicPatternType.COLLECTIVE_HEALTH_TREND,
        significance: delta < -0.2 ? SignificanceLevel.CRITICAL : SignificanceLevel.HIGH,
        data: { currentAvg: recentAvg, previousAvg: olderAvg, delta },
        confidence: PHI_INV_2,
        message: `Health declining: ${(olderAvg * 100).toFixed(0)}% → ${(recentAvg * 100).toFixed(0)}%`,
      };
    }

    this._baselines.avgHealth = recentAvg;
    return null;
  }

  _detectMemoryPressure() {
    if (this._healthSnapshots.length < THRESHOLDS.minDataPoints) return null;

    const recent = this._healthSnapshots.slice(-5);
    const avgMemLoad = recent.reduce((s, h) => s + h.memoryLoad, 0) / recent.length;

    if (avgMemLoad > PHI_INV) {
      return {
        type: CynicPatternType.MEMORY_PRESSURE,
        significance: avgMemLoad > 0.8 ? SignificanceLevel.CRITICAL : SignificanceLevel.HIGH,
        data: { avgMemoryLoad: avgMemLoad, snapshots: recent.length },
        confidence: Math.min(PHI_INV, avgMemLoad * PHI_INV_2), // φ-bound: scale to max φ⁻¹
        message: `Memory load at ${(avgMemLoad * 100).toFixed(0)}% — pressure detected`,
      };
    }

    this._baselines.memoryLoad = avgMemLoad;
    return null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Pattern Management
  // ═══════════════════════════════════════════════════════════════════════════

  _registerPattern(pattern) {
    pattern.timestamp = Date.now();
    pattern.cell = 'C6.7';
    pattern.dimension = 'CYNIC';
    pattern.analysis = 'EMERGE';

    this._patterns.push(pattern);
    if (this._patterns.length > this._maxPatterns) {
      this._patterns.shift();
    }
    this._stats.patternsDetected++;

    this.emit('pattern_detected', pattern);
    log.info('Pattern detected', { type: pattern.type, significance: pattern.significance });
  }

  getPatterns(limit = 20) {
    return this._patterns.slice(-limit);
  }

  getStats() {
    return { ...this._stats, baselines: { ...this._baselines } };
  }

  getHealth() {
    return {
      healthy: true,
      dataPoints: {
        dogEvents: this._dogEvents.length,
        consensusResults: this._consensusResults.length,
        healthSnapshots: this._healthSnapshots.length,
      },
      patternsDetected: this._stats.patternsDetected,
      lastAnalysis: this._stats.lastAnalysis,
    };
  }

  clear() {
    this._dogEvents = [];
    this._consensusResults = [];
    this._healthSnapshots = [];
    this._patterns = [];
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Singleton
// ═══════════════════════════════════════════════════════════════════════════

let _instance = null;

export function getCynicEmergence(options = {}) {
  if (!_instance) {
    _instance = new CynicEmergence(options);
  }
  return _instance;
}

export function resetCynicEmergence() {
  if (_instance) {
    _instance.removeAllListeners();
  }
  _instance = null;
}
