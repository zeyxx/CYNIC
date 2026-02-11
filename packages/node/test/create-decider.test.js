/**
 * Decider Factory Tests
 *
 * Tests create-decider.js with CosmosDecider config.
 * Verifies behavioral equivalence with the original hand-written class.
 *
 * @module @cynic/node/test/create-decider
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import { createDecider } from '../src/cycle/create-decider.js';
import { cosmosDeciderConfig, CosmosDecisionType } from '../src/cycle/configs/cosmos-decider.config.js';
import { PHI_INV } from '@cynic/core';

// =============================================================================
// Factory basics
// =============================================================================

describe('createDecider factory', () => {
  it('returns Class, getInstance, resetInstance', () => {
    const result = createDecider(cosmosDeciderConfig);
    assert.equal(typeof result.Class, 'function');
    assert.equal(typeof result.getInstance, 'function');
    assert.equal(typeof result.resetInstance, 'function');
    result.resetInstance();
  });

  it('getInstance returns singleton', () => {
    const { getInstance, resetInstance } = createDecider(cosmosDeciderConfig);
    const a = getInstance();
    const b = getInstance();
    assert.equal(a, b);
    resetInstance();
  });

  it('resetInstance clears singleton', () => {
    const { getInstance, resetInstance } = createDecider(cosmosDeciderConfig);
    const a = getInstance();
    resetInstance();
    const b = getInstance();
    assert.notEqual(a, b);
    resetInstance();
  });
});

// =============================================================================
// CosmosDecider via factory
// =============================================================================

describe('CosmosDecider (factory)', () => {
  let decider;

  beforeEach(() => {
    const factory = createDecider(cosmosDeciderConfig);
    decider = new factory.Class();
  });

  it('has correct stats initialization', () => {
    const stats = decider.getStats();
    assert.equal(stats.decisionsTotal, 0);
    assert.equal(stats.outcomesRecorded, 0);
    assert.equal(stats.stabilityHolds, 0);
    assert.equal(stats.goalViolations, 0);
    for (const type of Object.values(CosmosDecisionType)) {
      assert.equal(stats.byType[type], 0);
    }
  });

  // WAIT decision with insufficient data
  it('returns WAIT with insufficient observations', () => {
    const judgment = { score: 80, verdict: 'HOWL', type: 'ecosystem_coherence' };
    const result = decider.decide(judgment, { observationCount: 2 });
    assert.equal(result.decision, CosmosDecisionType.WAIT);
    assert.ok(result.reason.includes('Insufficient'));
    assert.equal(result.cell, 'C7.3');
    assert.equal(result.dimension, 'COSMOS');
    assert.equal(result.analysis, 'DECIDE');
  });

  // ACCELERATE with high score + enough data
  it('returns ACCELERATE for HOWL verdict with enough data', () => {
    const judgment = {
      score: 85,
      verdict: 'HOWL',
      type: 'ecosystem_coherence',
      scores: { coherence: 85, utility: 80, sustainability: 90 },
    };
    const result = decider.decide(judgment, { observationCount: 10 });
    assert.equal(result.decision, CosmosDecisionType.ACCELERATE);
    assert.ok(result.confidence > 0);
    assert.ok(result.confidence <= PHI_INV);
  });

  // MAINTAIN for WAG
  it('returns MAINTAIN for WAG verdict', () => {
    const judgment = {
      score: 60,
      verdict: 'WAG',
      type: 'ecosystem_utility',
      scores: { coherence: 60, utility: 65, sustainability: 55 },
    };
    const result = decider.decide(judgment, { observationCount: 5 });
    assert.equal(result.decision, CosmosDecisionType.MAINTAIN);
  });

  // DECELERATE for BARK
  it('returns DECELERATE for BARK verdict', () => {
    const judgment = {
      score: 25,
      verdict: 'BARK',
      type: 'ecosystem_coherence',
      scores: { coherence: 25, utility: 30, sustainability: 20 },
    };
    const result = decider.decide(judgment, { observationCount: 10 });
    assert.equal(result.decision, CosmosDecisionType.DECELERATE);
    assert.equal(result.severity, 'high');
  });

  // GROWL → FOCUS or DIVERSIFY based on weakest factor
  it('returns FOCUS when sustainability is weakest (GROWL)', () => {
    const judgment = {
      score: 45,
      verdict: 'GROWL',
      type: 'ecosystem_coherence',
      scores: { coherence: 50, utility: 55, sustainability: 30 },
    };
    const result = decider.decide(judgment, { observationCount: 10 });
    assert.equal(result.decision, CosmosDecisionType.FOCUS);
  });

  it('returns DIVERSIFY when utility is weakest (GROWL)', () => {
    const judgment = {
      score: 45,
      verdict: 'GROWL',
      type: 'ecosystem_coherence',
      scores: { coherence: 50, utility: 30, sustainability: 55 },
    };
    const result = decider.decide(judgment, { observationCount: 10 });
    assert.equal(result.decision, CosmosDecisionType.DIVERSIFY);
  });

  // INTERVENE with falling trend + low score
  it('returns INTERVENE when score < 30 and trend falling', () => {
    const judgment = {
      score: 15,
      verdict: 'BARK',
      type: 'ecosystem_coherence',
      scores: { coherence: 10, utility: 15, sustainability: 20 },
    };
    const result = decider.decide(judgment, { observationCount: 10, trend: 'falling' });
    assert.equal(result.decision, CosmosDecisionType.INTERVENE);
    assert.equal(result.severity, 'critical');
  });

  // Intervention cooldown
  it('respects intervention cooldown', () => {
    const judgment = {
      score: 15,
      verdict: 'BARK',
      type: 'ecosystem_coherence',
      scores: { coherence: 10, utility: 15, sustainability: 20 },
    };
    const ctx = { observationCount: 10, trend: 'falling' };

    // First: INTERVENE
    const first = decider.decide(judgment, ctx);
    assert.equal(first.decision, CosmosDecisionType.INTERVENE);

    // Second (same instant): should decelerate instead
    const second = decider.decide(judgment, ctx);
    assert.equal(second.decision, CosmosDecisionType.DECELERATE);
    assert.ok(second.reason.includes('cooldown'));
  });

  // Confidence is φ-bounded
  it('confidence never exceeds PHI_INV', () => {
    const judgment = {
      score: 95,
      verdict: 'HOWL',
      type: 'ecosystem_coherence',
      scores: { coherence: 95, utility: 95, sustainability: 95 },
    };
    const result = decider.decide(judgment, { observationCount: 100 });
    assert.ok(result.confidence <= PHI_INV);
  });

  // Low consensus reduces confidence
  it('low consensus reduces confidence', () => {
    const judgment = {
      score: 80,
      verdict: 'HOWL',
      type: 'ecosystem_coherence',
      scores: { coherence: 80, utility: 80, sustainability: 80 },
    };

    const highConsensus = decider.decide(judgment, { observationCount: 10, consensus: 0.6 });

    // Reset for clean comparison
    const factory2 = createDecider(cosmosDeciderConfig);
    const decider2 = new factory2.Class();
    const lowConsensus = decider2.decide(judgment, { observationCount: 10, consensus: 0.1 });

    assert.ok(lowConsensus.confidence < highConsensus.confidence);
  });

  // recordOutcome
  it('recordOutcome tracks outcomes', () => {
    decider.recordOutcome({
      decisionType: 'accelerate',
      result: 'success',
      reason: 'worked well',
    });
    assert.equal(decider.getStats().outcomesRecorded, 1);
  });

  // Stats tracking
  it('updates stats on each decision', () => {
    const judgment = {
      score: 80,
      verdict: 'HOWL',
      type: 'ecosystem_coherence',
      scores: { coherence: 80, utility: 80, sustainability: 80 },
    };

    decider.decide(judgment, { observationCount: 10 });
    decider.decide(judgment, { observationCount: 10 });

    const stats = decider.getStats();
    assert.equal(stats.decisionsTotal, 2);
    assert.ok(stats.lastDecision > 0);
  });

  // History
  it('getHistory returns recent decisions', () => {
    const judgment = {
      score: 80,
      verdict: 'HOWL',
      type: 'ecosystem_coherence',
      scores: { coherence: 80, utility: 80, sustainability: 80 },
    };

    decider.decide(judgment, { observationCount: 10 });
    decider.decide(judgment, { observationCount: 10 });

    const history = decider.getHistory();
    assert.equal(history.length, 2);
    assert.equal(history[0].decision, CosmosDecisionType.ACCELERATE);
  });

  it('getHistory respects limit', () => {
    const judgment = {
      score: 80,
      verdict: 'HOWL',
      type: 'ecosystem_coherence',
      scores: { coherence: 80, utility: 80, sustainability: 80 },
    };

    for (let i = 0; i < 10; i++) {
      decider.decide(judgment, { observationCount: 10 });
    }
    assert.equal(decider.getHistory(3).length, 3);
  });

  // Health
  it('getHealth returns healthy status', () => {
    const health = decider.getHealth();
    assert.equal(health.status, 'healthy');
    assert.ok(health.score <= PHI_INV);
  });

  it('getHealth detects stressed (many goal violations)', () => {
    // Manually set goalViolations > 3
    decider._stats.goalViolations = 4;
    const health = decider.getHealth();
    assert.equal(health.status, 'stressed');
  });

  // Event emission
  it('emits decision event', () => {
    let emitted = null;
    decider.on('decision', (d) => { emitted = d; });

    const judgment = {
      score: 80,
      verdict: 'HOWL',
      type: 'ecosystem_coherence',
      scores: { coherence: 80, utility: 80, sustainability: 80 },
    };
    decider.decide(judgment, { observationCount: 10 });

    assert.ok(emitted);
    assert.equal(emitted.decision, CosmosDecisionType.ACCELERATE);
  });

  it('emits outcome_recorded event', () => {
    let emitted = null;
    decider.on('outcome_recorded', (o) => { emitted = o; });

    decider.recordOutcome({
      decisionType: 'accelerate',
      result: 'success',
    });

    assert.ok(emitted);
    assert.equal(emitted.result, 'success');
  });

  // Calibration summary
  it('getCalibrationSummary works', () => {
    for (let i = 0; i < 6; i++) {
      decider.recordOutcome({
        decisionType: 'accelerate',
        result: i < 4 ? 'success' : 'failure',
      });
    }

    const summary = decider.getCalibrationSummary();
    assert.ok(summary.accelerate);
    assert.equal(summary.accelerate.outcomes, 6);
    assert.ok(Math.abs(summary.accelerate.successRate - 4 / 6) < 0.01);
  });

  // Pattern-specific minObservations
  it('ecosystem_security needs only 2 observations', () => {
    const judgment = {
      score: 80,
      verdict: 'HOWL',
      type: 'ecosystem_security',
      scores: { coherence: 80, utility: 80, sustainability: 80 },
    };
    const result = decider.decide(judgment, { observationCount: 2 });
    // Should NOT be WAIT — security only needs 2 observations
    assert.notEqual(result.decision, CosmosDecisionType.WAIT);
  });

  it('cross_project_health needs 8 observations', () => {
    const judgment = {
      score: 80,
      verdict: 'HOWL',
      type: 'cross_project_health',
      scores: { coherence: 80, utility: 80, sustainability: 80 },
    };
    const result = decider.decide(judgment, { observationCount: 5 });
    // Should be WAIT — cross_project_health needs 8
    assert.equal(result.decision, CosmosDecisionType.WAIT);
  });
});
