/**
 * Tests for CynicDecider (C6.3 CYNIC × DECIDE) — self-governance
 *
 * @module test/cynic-decider
 */

'use strict';

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import { CynicDecider, CynicDecisionType } from '../src/cynic/cynic-decider.js';

describe('CynicDecider', () => {
  let decider;

  beforeEach(() => {
    decider = new CynicDecider();
  });

  describe('decideOnPattern', () => {
    it('decides ROTATE_DOGS on dog dominance exceeding goal', () => {
      const result = decider.decideOnPattern({
        type: 'dog_dominance_shift',
        significance: 'high',
        data: { dominantDog: 'Guardian', ratio: 0.75, previousDominant: 'Scout' },
        confidence: 0.5,
      });

      assert.ok(result);
      assert.equal(result.type, CynicDecisionType.ROTATE_DOGS);
      assert.equal(result.cell, 'C6.3');
      assert.equal(result.dimension, 'CYNIC');
      assert.ok(result.context.reason.includes('dog_dominance'));
    });

    it('acknowledges dominance within bounds', () => {
      const result = decider.decideOnPattern({
        type: 'dog_dominance_shift',
        significance: 'medium',
        data: { dominantDog: 'Guardian', ratio: 0.4, previousDominant: 'Scout' },
        confidence: 0.3,
      });

      assert.ok(result);
      assert.equal(result.type, CynicDecisionType.ACKNOWLEDGE);
    });

    it('decides ROTATE_DOGS on dog silence', () => {
      const result = decider.decideOnPattern({
        type: 'dog_silence',
        significance: 'medium',
        data: { silentDog: 'Oracle', eventsSinceLastSeen: 15 },
        confidence: 0.4,
      });

      assert.ok(result);
      assert.equal(result.type, CynicDecisionType.ROTATE_DOGS);
    });

    it('decides ADJUST_THRESHOLDS on low consensus quality', () => {
      const result = decider.decideOnPattern({
        type: 'consensus_quality_change',
        significance: 'high',
        data: { approvalRate: 0.2, trend: 'declining' },
        confidence: 0.5,
      });

      assert.ok(result);
      assert.equal(result.type, CynicDecisionType.ADJUST_THRESHOLDS);
      assert.ok(result.context.reason.includes('consensus_below_goal'));
    });

    it('acknowledges healthy consensus', () => {
      const result = decider.decideOnPattern({
        type: 'consensus_quality_change',
        significance: 'low',
        data: { approvalRate: 0.6 },
        confidence: 0.3,
      });

      assert.ok(result);
      assert.equal(result.type, CynicDecisionType.ACKNOWLEDGE);
    });

    it('decides COMPACT_MEMORY on high memory pressure', () => {
      const result = decider.decideOnPattern({
        type: 'memory_pressure',
        significance: 'high',
        data: { memoryLoad: 0.75 },
        confidence: 0.5,
      });

      assert.ok(result);
      assert.equal(result.type, CynicDecisionType.COMPACT_MEMORY);
    });

    it('ignores low memory pressure', () => {
      const result = decider.decideOnPattern({
        type: 'memory_pressure',
        significance: 'low',
        data: { memoryLoad: 0.3 },
        confidence: 0.3,
      });

      assert.equal(result, null);
    });

    it('decides PAUSE_LEARNING on stagnation', () => {
      const result = decider.decideOnPattern({
        type: 'learning_velocity_change',
        significance: 'medium',
        data: { velocity: 0.1 },
        confidence: 0.4,
      });

      assert.ok(result);
      assert.equal(result.type, CynicDecisionType.PAUSE_LEARNING);
    });

    it('decides ESCALATE_PATTERN on guardian escalation', () => {
      const result = decider.decideOnPattern({
        type: 'guardian_escalation',
        significance: 'critical',
        data: { count: 5, recentThreats: ['sql_injection'] },
        confidence: 0.6,
      });

      assert.ok(result);
      assert.equal(result.type, CynicDecisionType.ESCALATE_PATTERN);
      assert.equal(result.urgency, 'critical');
    });

    it('decides ESCALATE_PATTERN on declining health', () => {
      const result = decider.decideOnPattern({
        type: 'collective_health_trend',
        significance: 'high',
        data: { avgHealth: 0.15, trend: 'declining' },
        confidence: 0.5,
      });

      assert.ok(result);
      assert.equal(result.type, CynicDecisionType.ESCALATE_PATTERN);
      assert.ok(result.context.reason.includes('health_below_minimum'));
    });

    it('decides ADJUST_THRESHOLDS on moderate declining health', () => {
      const result = decider.decideOnPattern({
        type: 'collective_health_trend',
        significance: 'medium',
        data: { avgHealth: 0.5, trend: 'declining' },
        confidence: 0.4,
      });

      assert.ok(result);
      assert.equal(result.type, CynicDecisionType.ADJUST_THRESHOLDS);
    });

    it('returns null for unknown pattern type', () => {
      const result = decider.decideOnPattern({
        type: 'unknown_pattern',
        significance: 'low',
        data: {},
        confidence: 0.3,
      });

      assert.equal(result, null);
    });
  });

  describe('cooldowns', () => {
    it('respects cooldown for same decision type', () => {
      const first = decider.decideOnPattern({
        type: 'dog_dominance_shift',
        significance: 'high',
        data: { dominantDog: 'Guardian', ratio: 0.75 },
        confidence: 0.5,
      });
      assert.ok(first);
      assert.equal(first.type, CynicDecisionType.ROTATE_DOGS);

      // Same pattern immediately — should be blocked by cooldown
      const second = decider.decideOnPattern({
        type: 'dog_silence',
        significance: 'medium',
        data: { silentDog: 'Oracle', eventsSinceLastSeen: 15 },
        confidence: 0.4,
      });
      assert.equal(second, null); // Same type (ROTATE_DOGS) on cooldown
    });

    it('critical urgency bypasses cooldown', () => {
      // First decision
      decider.decideOnPattern({
        type: 'guardian_escalation',
        significance: 'high',
        data: { count: 3 },
        confidence: 0.5,
      });

      // Same type but critical urgency — should bypass
      const critical = decider.decideOnPattern({
        type: 'guardian_escalation',
        significance: 'critical',
        data: { count: 10 },
        confidence: 0.6,
      });
      assert.ok(critical);
      assert.equal(critical.type, CynicDecisionType.ESCALATE_PATTERN);
    });

    it('ACKNOWLEDGE always passes (no cooldown)', () => {
      const first = decider.decideOnPattern({
        type: 'consensus_quality_change',
        significance: 'low',
        data: { approvalRate: 0.6 },
        confidence: 0.3,
      });
      assert.equal(first.type, CynicDecisionType.ACKNOWLEDGE);

      const second = decider.decideOnPattern({
        type: 'consensus_quality_change',
        significance: 'low',
        data: { approvalRate: 0.5 },
        confidence: 0.3,
      });
      assert.equal(second.type, CynicDecisionType.ACKNOWLEDGE);
    });
  });

  describe('decideOnConsciousness', () => {
    it('returns null for critical state (CynicActor handles)', () => {
      const result = decider.decideOnConsciousness({
        healthState: 'critical',
        trend: -0.3,
      });
      assert.equal(result, null);
    });

    it('adjusts thresholds on negative trend while nominal', () => {
      const result = decider.decideOnConsciousness({
        healthState: 'nominal',
        trend: -0.2,
      });
      assert.ok(result);
      assert.equal(result.type, CynicDecisionType.ADJUST_THRESHOLDS);
    });

    it('returns null when trend is stable', () => {
      const result = decider.decideOnConsciousness({
        healthState: 'nominal',
        trend: 0.0,
      });
      assert.equal(result, null);
    });
  });

  describe('outcome tracking', () => {
    it('records outcomes', () => {
      decider.recordOutcome({
        decisionType: CynicDecisionType.ROTATE_DOGS,
        result: 'success',
        reason: 'dog distribution improved',
      });

      const stats = decider.getStats();
      assert.equal(stats.outcomesRecorded, 1);
    });

    it('emits outcome_recorded event', (t, done) => {
      decider.once('outcome_recorded', (entry) => {
        assert.equal(entry.result, 'success');
        done();
      });

      decider.recordOutcome({
        decisionType: CynicDecisionType.ROTATE_DOGS,
        result: 'success',
      });
    });

    it('calibration adjusts confidence with enough outcomes', () => {
      // Record many successful ROTATE_DOGS outcomes
      for (let i = 0; i < 7; i++) {
        decider.recordOutcome({
          decisionType: CynicDecisionType.ROTATE_DOGS,
          result: 'success',
        });
      }

      const stats = decider.getStats();
      assert.ok(stats.calibration[CynicDecisionType.ROTATE_DOGS]);
      assert.equal(stats.calibration[CynicDecisionType.ROTATE_DOGS].successRate, 1);
    });
  });

  describe('goal violations', () => {
    it('tracks goal violations', () => {
      // Dog dominance over goal
      decider.decideOnPattern({
        type: 'dog_dominance_shift',
        significance: 'high',
        data: { dominantDog: 'Guardian', ratio: 0.75 },
        confidence: 0.5,
      });

      assert.equal(decider.getStats().goalViolations, 1);
    });

    it('multiple violations mark health as stressed', () => {
      // Trigger 4 different goal violations
      decider.decideOnPattern({
        type: 'dog_dominance_shift', significance: 'high',
        data: { dominantDog: 'G', ratio: 0.8 }, confidence: 0.5,
      });
      decider.decideOnPattern({
        type: 'consensus_quality_change', significance: 'high',
        data: { approvalRate: 0.1 }, confidence: 0.5,
      });
      decider.decideOnPattern({
        type: 'memory_pressure', significance: 'high',
        data: { memoryLoad: 0.9 }, confidence: 0.5,
      });
      decider.decideOnPattern({
        type: 'collective_health_trend', significance: 'high',
        data: { avgHealth: 0.1, trend: 'declining' }, confidence: 0.5,
      });

      const health = decider.getHealth();
      assert.equal(health.status, 'stressed');
      assert.ok(health.goalViolations >= 4);
    });
  });

  describe('events', () => {
    it('emits decision event', (t, done) => {
      decider.once('decision', (decision) => {
        assert.equal(decision.cell, 'C6.3');
        assert.equal(decision.type, CynicDecisionType.ROTATE_DOGS);
        done();
      });

      decider.decideOnPattern({
        type: 'dog_dominance_shift',
        significance: 'high',
        data: { dominantDog: 'Guardian', ratio: 0.75 },
        confidence: 0.5,
      });
    });
  });

  describe('getStats', () => {
    it('returns initial stats', () => {
      const stats = decider.getStats();
      assert.equal(stats.decisionsTotal, 0);
      assert.equal(stats.goalViolations, 0);
      assert.ok(stats.goals);
    });

    it('tracks decisions by type', () => {
      decider.decideOnPattern({
        type: 'dog_dominance_shift',
        significance: 'high',
        data: { dominantDog: 'G', ratio: 0.8 },
        confidence: 0.5,
      });

      const stats = decider.getStats();
      assert.equal(stats.decisionsTotal, 1);
      assert.equal(stats.byType[CynicDecisionType.ROTATE_DOGS], 1);
    });
  });

  describe('getHealth', () => {
    it('returns healthy by default', () => {
      const health = decider.getHealth();
      assert.equal(health.status, 'healthy');
    });

    it('caps score at φ⁻¹', () => {
      const health = decider.getHealth();
      assert.ok(health.score <= 0.618 + 0.001);
    });
  });

  describe('getRecentDecisions', () => {
    it('returns empty array initially', () => {
      assert.deepEqual(decider.getRecentDecisions(), []);
    });

    it('returns limited history', () => {
      for (let i = 0; i < 15; i++) {
        decider.decideOnPattern({
          type: 'consensus_quality_change',
          significance: 'low',
          data: { approvalRate: 0.6 },
          confidence: 0.3,
        });
      }

      const recent = decider.getRecentDecisions(5);
      assert.equal(recent.length, 5);
    });
  });
});
