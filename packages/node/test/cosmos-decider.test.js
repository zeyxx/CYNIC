/**
 * Tests for CosmosDecider (C7.3 COSMOS × DECIDE) — ecosystem governance
 *
 * @module test/cosmos-decider
 */

'use strict';

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import { CosmosDecider, CosmosDecisionType } from '../src/cosmos/cosmos-decider.js';

describe('CosmosDecider', () => {
  let decider;

  beforeEach(() => {
    decider = new CosmosDecider();
  });

  describe('decide — basic verdicts', () => {
    it('returns WAIT with insufficient observations', () => {
      const result = decider.decide({ score: 80, verdict: 'HOWL', type: 'ecosystem_coherence' }, { observationCount: 2 });
      assert.equal(result.decision, CosmosDecisionType.WAIT);
      assert.ok(result.reason.includes('Insufficient'));
    });

    it('returns ACCELERATE for HOWL verdict', () => {
      const result = decider.decide({ score: 80, verdict: 'HOWL', type: 'unknown' }, { observationCount: 5 });
      assert.equal(result.decision, CosmosDecisionType.ACCELERATE);
    });

    it('returns MAINTAIN for WAG verdict', () => {
      const result = decider.decide({ score: 60, verdict: 'WAG', type: 'unknown' }, { observationCount: 5 });
      assert.equal(result.decision, CosmosDecisionType.MAINTAIN);
    });

    it('returns DECELERATE for BARK verdict', () => {
      const result = decider.decide({ score: 15, verdict: 'BARK', type: 'unknown' }, { observationCount: 5 });
      assert.equal(result.decision, CosmosDecisionType.DECELERATE);
      assert.equal(result.severity, 'high');
    });

    it('returns INTERVENE for low score with falling trend', () => {
      const result = decider.decide(
        { score: 20, verdict: 'BARK', scores: { coherence: 20, utility: 20, sustainability: 20 } },
        { observationCount: 5, trend: 'falling' }
      );
      assert.equal(result.decision, CosmosDecisionType.INTERVENE);
      assert.equal(result.severity, 'critical');
    });
  });

  describe('decide — GROWL routing', () => {
    it('returns FOCUS when sustainability is weakest', () => {
      const result = decider.decide(
        { score: 40, verdict: 'GROWL', scores: { coherence: 50, utility: 60, sustainability: 30 } },
        { observationCount: 5 }
      );
      assert.equal(result.decision, CosmosDecisionType.FOCUS);
    });

    it('returns DIVERSIFY when utility is weakest', () => {
      const result = decider.decide(
        { score: 40, verdict: 'GROWL', scores: { coherence: 60, utility: 30, sustainability: 50 } },
        { observationCount: 5 }
      );
      assert.equal(result.decision, CosmosDecisionType.DIVERSIFY);
    });
  });

  describe('pattern-aware strategies', () => {
    it('ecosystem_security needs only 2 observations', () => {
      const result = decider.decide(
        { score: 80, verdict: 'HOWL', type: 'ecosystem_security' },
        { observationCount: 2 }
      );
      assert.equal(result.decision, CosmosDecisionType.ACCELERATE);
    });

    it('cross_project_health needs 8 observations', () => {
      const result = decider.decide(
        { score: 80, verdict: 'HOWL', type: 'cross_project_health' },
        { observationCount: 5 }
      );
      assert.equal(result.decision, CosmosDecisionType.WAIT);
    });

    it('uses weighted scores per judgment type', () => {
      // ecosystem_security weights sustainability at 0.7
      const result = decider.decide(
        { score: 40, verdict: 'GROWL', type: 'ecosystem_security',
          scores: { coherence: 80, utility: 80, sustainability: 20 } },
        { observationCount: 3 }
      );
      // Sustainability is weakest → FOCUS
      assert.equal(result.decision, CosmosDecisionType.FOCUS);
    });

    it('includes severity in output', () => {
      const result = decider.decide(
        { score: 80, verdict: 'HOWL', type: 'unknown' },
        { observationCount: 5 }
      );
      assert.ok(result.severity);
    });
  });

  describe('stability checks', () => {
    it('holds previous decision without enough new observations', () => {
      // First decision: MAINTAIN
      decider.decide({ score: 60, verdict: 'WAG', type: 'test' }, { observationCount: 5 });

      // Immediate reversal attempt: should hold MAINTAIN
      const result = decider.decide({ score: 80, verdict: 'HOWL', type: 'test' }, { observationCount: 5 });
      assert.equal(result.decision, CosmosDecisionType.MAINTAIN);
      assert.ok(result.reason.includes('Holding'));
    });

    it('allows reversal after stability threshold exceeded', () => {
      // First decision
      decider.decide({ score: 60, verdict: 'WAG', type: 'test' }, { observationCount: 5 });

      // Pad history to exceed stability threshold
      decider.decide({ score: 55, verdict: 'WAG', type: 'other1' }, { observationCount: 5 });
      decider.decide({ score: 55, verdict: 'WAG', type: 'other2' }, { observationCount: 5 });
      decider.decide({ score: 55, verdict: 'WAG', type: 'other3' }, { observationCount: 5 });

      // Now reversal should be allowed
      const result = decider.decide({ score: 80, verdict: 'HOWL', type: 'test' }, { observationCount: 5 });
      assert.equal(result.decision, CosmosDecisionType.ACCELERATE);
    });

    it('tracks stabilityHolds in stats', () => {
      decider.decide({ score: 60, verdict: 'WAG', type: 'test' }, { observationCount: 5 });
      decider.decide({ score: 80, verdict: 'HOWL', type: 'test' }, { observationCount: 5 });
      assert.equal(decider.getStats().stabilityHolds, 1);
    });
  });

  describe('intervene cooldown', () => {
    it('falls back to DECELERATE when on cooldown', () => {
      // First intervene
      const first = decider.decide(
        { score: 10, verdict: 'BARK', scores: { coherence: 10, utility: 10, sustainability: 10 } },
        { observationCount: 5, trend: 'falling' }
      );
      assert.equal(first.decision, CosmosDecisionType.INTERVENE);

      // Second attempt — on cooldown
      const second = decider.decide(
        { score: 10, verdict: 'BARK', scores: { coherence: 10, utility: 10, sustainability: 10 } },
        { observationCount: 5, trend: 'falling' }
      );
      assert.equal(second.decision, CosmosDecisionType.DECELERATE);
      assert.ok(second.reason.includes('cooldown'));
    });
  });

  describe('confidence calculation', () => {
    it('caps confidence at φ⁻¹', () => {
      const result = decider.decide(
        { score: 80, verdict: 'HOWL', type: 'unknown' },
        { observationCount: 100 }
      );
      assert.ok(result.confidence <= 0.618 + 0.001);
    });

    it('reduces confidence with low observations', () => {
      const result = decider.decide(
        { score: 80, verdict: 'HOWL', type: 'unknown' },
        { observationCount: 1 }
      );
      // WAIT decision (obs < 3), but confidence should be low
      assert.ok(result.confidence < 0.5);
    });

    it('reduces confidence with low consensus', () => {
      const high = decider.decide(
        { score: 60, verdict: 'WAG', type: 'unknown' },
        { observationCount: 10, consensus: 0.6 }
      );
      const low = decider.decide(
        { score: 60, verdict: 'WAG', type: 'unknown' },
        { observationCount: 10, consensus: 0.1 }
      );
      assert.ok(low.confidence < high.confidence);
    });
  });

  describe('outcome calibration', () => {
    it('records outcomes', () => {
      decider.recordOutcome({ decisionType: CosmosDecisionType.MAINTAIN, result: 'success' });
      assert.equal(decider.getStats().outcomesRecorded, 1);
    });

    it('emits outcome_recorded event', (t, done) => {
      decider.once('outcome_recorded', (entry) => {
        assert.equal(entry.result, 'success');
        done();
      });
      decider.recordOutcome({ decisionType: CosmosDecisionType.MAINTAIN, result: 'success' });
    });

    it('calibration summary tracks success rates', () => {
      for (let i = 0; i < 5; i++) {
        decider.recordOutcome({ decisionType: CosmosDecisionType.MAINTAIN, result: 'success' });
      }
      const stats = decider.getStats();
      assert.ok(stats.calibration[CosmosDecisionType.MAINTAIN]);
      assert.equal(stats.calibration[CosmosDecisionType.MAINTAIN].successRate, 1);
    });
  });

  describe('goal violations', () => {
    it('tracks BARK as goal violation', () => {
      decider.decide({ score: 15, verdict: 'BARK', type: 'unknown' }, { observationCount: 5 });
      assert.equal(decider.getStats().goalViolations, 1);
    });

    it('tracks INTERVENE as goal violation', () => {
      decider.decide(
        { score: 10, verdict: 'BARK', scores: { coherence: 10, utility: 10, sustainability: 10 } },
        { observationCount: 5, trend: 'falling' }
      );
      assert.equal(decider.getStats().goalViolations, 1);
    });
  });

  describe('events', () => {
    it('emits decision event', (t, done) => {
      decider.once('decision', (result) => {
        assert.equal(result.cell, 'C7.3');
        assert.equal(result.dimension, 'COSMOS');
        done();
      });
      decider.decide({ score: 60, verdict: 'WAG', type: 'unknown' }, { observationCount: 5 });
    });
  });

  describe('getStats', () => {
    it('returns initial stats', () => {
      const stats = decider.getStats();
      assert.equal(stats.totalDecisions, 0);
      assert.equal(stats.goalViolations, 0);
      assert.equal(stats.stabilityHolds, 0);
    });

    it('tracks decisions by type', () => {
      decider.decide({ score: 60, verdict: 'WAG', type: 'unknown' }, { observationCount: 5 });
      const stats = decider.getStats();
      assert.equal(stats.totalDecisions, 1);
    });
  });

  describe('getHealth', () => {
    it('returns healthy by default', () => {
      const health = decider.getHealth();
      assert.equal(health.status, 'healthy');
    });

    it('returns stressed after many violations', () => {
      for (let i = 0; i < 5; i++) {
        decider.decide({ score: 10, verdict: 'BARK', type: `t${i}` }, { observationCount: 5 });
      }
      const health = decider.getHealth();
      assert.equal(health.status, 'stressed');
    });

    it('caps score at φ⁻¹', () => {
      const health = decider.getHealth();
      assert.ok(health.score <= 0.618 + 0.001);
    });
  });

  describe('getHistory', () => {
    it('returns empty initially', () => {
      assert.deepEqual(decider.getHistory(), []);
    });

    it('limits history', () => {
      for (let i = 0; i < 10; i++) {
        decider.decide({ score: 60, verdict: 'WAG', type: `t${i}` }, { observationCount: 5 });
      }
      assert.equal(decider.getHistory(3).length, 3);
    });
  });
});
