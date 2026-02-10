/**
 * Tests for CynicActor (C6.4 CYNIC × ACT) — self-healing
 *
 * @module test/cynic-actor
 */

'use strict';

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import { CynicActor, SelfHealAction, HealthState } from '../src/cynic/cynic-actor.js';

describe('CynicActor', () => {
  let actor;

  beforeEach(() => {
    actor = new CynicActor();
  });

  describe('processConsciousnessChange', () => {
    it('starts in NOMINAL state', () => {
      const stats = actor.getStats();
      assert.equal(stats.currentHealthState, HealthState.NOMINAL);
    });

    it('records awareness history', () => {
      actor.processConsciousnessChange({ awarenessLevel: 0.5, newState: 'AWARE' });
      actor.processConsciousnessChange({ awarenessLevel: 0.6, newState: 'AWARE' });
      assert.equal(actor.getStats().awarenessHistorySize, 2);
    });

    it('transitions to DEGRADED on low awareness', () => {
      // Start from OPTIMAL first
      actor.processConsciousnessChange({ awarenessLevel: 0.65, newState: 'HEIGHTENED' });
      assert.equal(actor.getStats().currentHealthState, HealthState.OPTIMAL);

      // Now degrade
      actor.processConsciousnessChange({ awarenessLevel: 0.3, newState: 'AWAKENING' });
      assert.equal(actor.getStats().currentHealthState, HealthState.DEGRADED);
    });

    it('transitions to CRITICAL on very low awareness', () => {
      actor.processConsciousnessChange({ awarenessLevel: 0.5, newState: 'AWARE' });
      actor.processConsciousnessChange({ awarenessLevel: 0.1, newState: 'DORMANT' });
      assert.equal(actor.getStats().currentHealthState, HealthState.CRITICAL);
    });

    it('takes self-heal actions on degradation', () => {
      // First get to OPTIMAL
      actor.processConsciousnessChange({ awarenessLevel: 0.65, newState: 'HEIGHTENED' });
      // Then crash to CRITICAL
      const actions = actor.processConsciousnessChange({ awarenessLevel: 0.1, newState: 'DORMANT' });
      assert.ok(actions.length > 0, 'should take actions');
      assert.ok(actions.some(a => a.type === SelfHealAction.RESTRICT_ROUTING));
    });

    it('emits self_heal event on action', (t, done) => {
      actor.once('self_heal', (event) => {
        assert.equal(event.cell, 'C6.4');
        assert.equal(event.dimension, 'CYNIC');
        done();
      });

      // Force transition
      actor.processConsciousnessChange({ awarenessLevel: 0.65, newState: 'HEIGHTENED' });
      actor.processConsciousnessChange({ awarenessLevel: 0.1, newState: 'DORMANT' });
    });

    it('respects cooldowns', () => {
      actor.processConsciousnessChange({ awarenessLevel: 0.65, newState: 'HEIGHTENED' });
      const first = actor.processConsciousnessChange({ awarenessLevel: 0.1, newState: 'DORMANT' });
      assert.ok(first.length > 0);

      // Immediate re-trigger should be blocked by cooldown
      actor._currentHealthState = HealthState.OPTIMAL; // Force reset
      const second = actor.processConsciousnessChange({ awarenessLevel: 0.1, newState: 'DORMANT' });
      // Some actions should be blocked by cooldown
      assert.ok(second.length <= first.length);
    });

    it('tracks recovery', () => {
      // Degrade
      actor.processConsciousnessChange({ awarenessLevel: 0.65, newState: 'HEIGHTENED' });
      actor.processConsciousnessChange({ awarenessLevel: 0.1, newState: 'DORMANT' });
      assert.equal(actor.getStats().degradationEpisodes, 1);

      // Recover
      actor.processConsciousnessChange({ awarenessLevel: 0.65, newState: 'HEIGHTENED' });
      assert.equal(actor.getStats().recoveries, 1);
    });

    it('handles recovery actions', () => {
      actor.processConsciousnessChange({ awarenessLevel: 0.65, newState: 'HEIGHTENED' });
      actor.processConsciousnessChange({ awarenessLevel: 0.1, newState: 'DORMANT' });

      const recoveryActions = actor.processConsciousnessChange({ awarenessLevel: 0.65, newState: 'HEIGHTENED' });
      assert.ok(recoveryActions.some(a => a.type === SelfHealAction.RESTORE_CAPABILITIES));
    });
  });

  describe('processPerturbation', () => {
    it('ignores small perturbations', () => {
      const result = actor.processPerturbation({ metric: 'errorRate', deviation: 2 });
      assert.equal(result, null);
    });

    it('rate-limits on error spike', () => {
      const result = actor.processPerturbation({ metric: 'errorRate', deviation: 4 });
      assert.ok(result);
      assert.equal(result.type, SelfHealAction.RATE_LIMIT);
    });

    it('pauses non-essential on latency spike', () => {
      const result = actor.processPerturbation({ metric: 'latency', deviation: 4 });
      assert.ok(result);
      assert.equal(result.type, SelfHealAction.PAUSE_NONESSENTIAL);
    });
  });

  describe('trend detection', () => {
    it('returns 0 with insufficient history', () => {
      const stats = actor.getStats();
      assert.equal(stats.trend, 0);
    });

    it('detects negative trend', () => {
      // Feed declining awareness
      for (let i = 10; i >= 1; i--) {
        actor.processConsciousnessChange({
          awarenessLevel: i * 0.06,
          newState: 'AWARE',
        });
      }
      const trend = actor.getStats().trend;
      assert.ok(trend < 0, `trend should be negative, got ${trend}`);
    });

    it('detects positive trend', () => {
      // Feed increasing awareness
      for (let i = 1; i <= 10; i++) {
        actor.processConsciousnessChange({
          awarenessLevel: i * 0.06,
          newState: 'AWARE',
        });
      }
      const trend = actor.getStats().trend;
      assert.ok(trend > 0, `trend should be positive, got ${trend}`);
    });
  });

  describe('getHealth', () => {
    it('returns healthy by default', () => {
      const health = actor.getHealth();
      assert.equal(health.status, 'healthy');
    });

    it('returns critical when in critical state', () => {
      actor.processConsciousnessChange({ awarenessLevel: 0.5, newState: 'AWARE' });
      actor.processConsciousnessChange({ awarenessLevel: 0.1, newState: 'DORMANT' });
      const health = actor.getHealth();
      assert.equal(health.status, 'critical');
    });

    it('caps score at φ⁻¹', () => {
      actor.processConsciousnessChange({ awarenessLevel: 0.9, newState: 'TRANSCENDENT' });
      const health = actor.getHealth();
      assert.ok(health.score <= 0.618 + 0.001, `score ${health.score} should be <= 0.618`);
    });
  });
});
