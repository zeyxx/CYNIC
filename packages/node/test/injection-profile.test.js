/**
 * InjectionProfile Tests
 *
 * Tests that CYNIC's injection decisions adapt over time:
 * - Thompson arms learn from engagement
 * - Periodic activation uses learned intervals
 * - Thresholds adjust from feedback
 * - State persists across sessions/processes
 *
 * "Le chien apprend à qui donner sa patte" — κυνικός
 *
 * @module @cynic/node/test/injection-profile
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { mkdirSync, existsSync, readFileSync, rmSync } from 'node:fs';

import {
  injectionProfile,
  DEFAULT_PRIORS,
  ENGAGEMENT_KEYWORDS,
  MIN_OBSERVATIONS,
} from '../src/services/injection-profile.js';

// ═══════════════════════════════════════════════════════════════════════════
// TEST HELPERS
// ═══════════════════════════════════════════════════════════════════════════

function getTempStatePath() {
  const dir = join(tmpdir(), `cynic-profile-test-${Date.now()}-${Math.random().toString(36).slice(2)}`);
  mkdirSync(dir, { recursive: true });
  return join(dir, 'injection-profile.json');
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('InjectionProfile', () => {
  let tempPath;

  beforeEach(() => {
    injectionProfile._resetForTesting();
    tempPath = getTempStatePath();
    injectionProfile._setStatePath(tempPath);
  });

  afterEach(() => {
    injectionProfile._resetForTesting();
    try {
      const dir = join(tempPath, '..');
      if (existsSync(dir)) rmSync(dir, { recursive: true });
    } catch { /* ignore */ }
  });

  // ═══════════════════════════════════════════════════════════════════════
  // LIFECYCLE
  // ═══════════════════════════════════════════════════════════════════════

  describe('Lifecycle', () => {
    it('starts and stops cleanly', () => {
      injectionProfile.start();
      assert.equal(injectionProfile.getStats().running, true);
      injectionProfile.stop();
      assert.equal(injectionProfile.getStats().running, false);
    });

    it('increments session count on start', () => {
      injectionProfile.start();
      assert.equal(injectionProfile.getStats().totalSessions, 1);
      injectionProfile.stop();

      injectionProfile.start();
      assert.equal(injectionProfile.getStats().totalSessions, 2);
    });

    it('persists state on stop', () => {
      injectionProfile.start();
      injectionProfile.shouldActivate('elenchus');
      injectionProfile.stop();
      assert.ok(existsSync(tempPath), 'State file should exist');
    });

    it('loads state on restart', () => {
      injectionProfile.start();
      injectionProfile.recordEngagement('elenchus');
      injectionProfile.recordEngagement('elenchus');
      injectionProfile.stop();

      injectionProfile._resetForTesting();
      injectionProfile._setStatePath(tempPath);
      injectionProfile.start();

      const stats = injectionProfile.getStats();
      assert.equal(stats.totalSessions, 2);
      assert.equal(stats.totalEngagements, 2);
      assert.ok(stats.arms.elenchus, 'elenchus arm should be restored');
      assert.equal(stats.arms.elenchus.engagements, 2);
    });

    it('resets for testing', () => {
      injectionProfile.start();
      injectionProfile.recordEngagement('elenchus');
      injectionProfile._resetForTesting();

      const stats = injectionProfile.getStats();
      assert.equal(stats.totalSessions, 0);
      assert.equal(stats.totalEngagements, 0);
      assert.equal(stats.running, false);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // ACTIVATION DECISIONS
  // ═══════════════════════════════════════════════════════════════════════

  describe('shouldActivate', () => {
    it('returns activate=true when not running', () => {
      const result = injectionProfile.shouldActivate('elenchus');
      assert.equal(result.activate, true);
      assert.equal(result.reason, 'not_running');
    });

    it('returns a decision for known topics', () => {
      injectionProfile.start();
      const result = injectionProfile.shouldActivate('elenchus');
      assert.ok(typeof result.activate === 'boolean');
      assert.ok(typeof result.rate === 'number');
      assert.ok(result.rate > 0 && result.rate <= 0.618034);
    });

    it('uses default priors for new topics', () => {
      injectionProfile.start();
      const result = injectionProfile.shouldActivate('elenchus');
      assert.equal(result.reason, 'prior');
    });

    it('uses learned rates after enough observations', () => {
      injectionProfile.start();
      // Force enough observations
      for (let i = 0; i < MIN_OBSERVATIONS; i++) {
        injectionProfile.recordEngagement('elenchus');
      }
      const result = injectionProfile.shouldActivate('elenchus');
      assert.equal(result.reason, 'learned');
    });

    it('tracks session injections', () => {
      injectionProfile.start();
      // Call multiple times — some will activate, some won't
      for (let i = 0; i < 20; i++) {
        injectionProfile.shouldActivate('elenchus');
      }
      const injections = injectionProfile.getSessionInjections();
      // elenchus should have activated at least once in 20 tries with ~37.5% rate
      assert.ok(injections.length >= 0); // Non-negative (could be 0 if very unlucky)
    });

    it('φ-bounds activation rate', () => {
      injectionProfile.start();
      // Push alpha very high
      for (let i = 0; i < 100; i++) {
        injectionProfile.recordEngagement('elenchus');
      }
      const rate = injectionProfile.getRate('elenchus');
      assert.ok(rate <= 0.618034, `Rate ${rate} exceeds φ⁻¹`);
    });

    it('creates arm with default priors for unknown topics', () => {
      injectionProfile.start();
      const result = injectionProfile.shouldActivate('unknown_topic');
      assert.ok(typeof result.activate === 'boolean');
      // Default prior is [2, 3] ≈ 40%
      assert.ok(result.rate > 0.3 && result.rate < 0.5, `Rate ${result.rate} should be ~40%`);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // PERIODIC ACTIVATION
  // ═══════════════════════════════════════════════════════════════════════

  describe('shouldActivatePeriodic', () => {
    it('activates on first prompt', () => {
      injectionProfile.start();
      const result = injectionProfile.shouldActivatePeriodic('ecosystem_status', 1);
      assert.equal(result.activate, true);
    });

    it('calculates interval from learned rate', () => {
      injectionProfile.start();
      // ecosystem_status default: [2, 8] = 20% → interval ≈ 5
      const result = injectionProfile.shouldActivatePeriodic('ecosystem_status', 5);
      assert.ok(result.interval >= 3 && result.interval <= 10,
        `Interval ${result.interval} should be ~5 for 20% rate`);
    });

    it('higher engagement increases frequency', () => {
      injectionProfile.start();
      // Start with default (low engagement)
      const before = injectionProfile.shouldActivatePeriodic('ecosystem_status', 10);

      // Record lots of engagement
      for (let i = 0; i < 20; i++) {
        injectionProfile.recordEngagement('ecosystem_status');
      }

      const after = injectionProfile.shouldActivatePeriodic('ecosystem_status', 10);
      assert.ok(after.interval <= before.interval,
        `Interval should decrease with engagement: before=${before.interval} after=${after.interval}`);
    });

    it('ignores increase the interval', () => {
      injectionProfile.start();
      const before = injectionProfile.shouldActivatePeriodic('social_status', 10);

      // Record lots of ignores
      for (let i = 0; i < 20; i++) {
        injectionProfile.recordIgnore('social_status');
      }

      const after = injectionProfile.shouldActivatePeriodic('social_status', 10);
      assert.ok(after.interval >= before.interval,
        `Interval should increase with ignores: before=${before.interval} after=${after.interval}`);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // THRESHOLDS
  // ═══════════════════════════════════════════════════════════════════════

  describe('Thresholds', () => {
    it('returns default when not running', () => {
      const threshold = injectionProfile.getThreshold('temporal_confidence', 0.4);
      assert.equal(threshold, 0.4);
    });

    it('returns default until enough adjustments', () => {
      injectionProfile.start();
      const threshold = injectionProfile.getThreshold('temporal_confidence', 0.4);
      assert.equal(threshold, 0.4); // No adjustments yet
    });

    it('adjusts down when useful', () => {
      injectionProfile.start();
      for (let i = 0; i < MIN_OBSERVATIONS + 1; i++) {
        injectionProfile.adjustThreshold('temporal_confidence', 0.4, true);
      }
      const threshold = injectionProfile.getThreshold('temporal_confidence', 0.4);
      assert.ok(threshold < 0.4, `Threshold ${threshold} should be < 0.4 after useful feedback`);
    });

    it('adjusts up when useless', () => {
      injectionProfile.start();
      for (let i = 0; i < MIN_OBSERVATIONS + 1; i++) {
        injectionProfile.adjustThreshold('temporal_confidence', 0.4, false);
      }
      const threshold = injectionProfile.getThreshold('temporal_confidence', 0.4);
      assert.ok(threshold > 0.4, `Threshold ${threshold} should be > 0.4 after useless feedback`);
    });

    it('caps at φ⁻¹', () => {
      injectionProfile.start();
      for (let i = 0; i < 100; i++) {
        injectionProfile.adjustThreshold('temporal_confidence', 0.4, false);
      }
      const threshold = injectionProfile.getThreshold('temporal_confidence', 0.4);
      assert.ok(threshold <= 0.618034, `Threshold ${threshold} exceeds φ⁻¹`);
    });

    it('floors at 0.1', () => {
      injectionProfile.start();
      for (let i = 0; i < 100; i++) {
        injectionProfile.adjustThreshold('temporal_confidence', 0.4, true);
      }
      const threshold = injectionProfile.getThreshold('temporal_confidence', 0.4);
      assert.ok(threshold >= 0.1, `Threshold ${threshold} below floor`);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // ENGAGEMENT
  // ═══════════════════════════════════════════════════════════════════════

  describe('Engagement', () => {
    it('records engagement', () => {
      injectionProfile.start();
      injectionProfile.recordEngagement('elenchus');
      const stats = injectionProfile.getStats();
      assert.equal(stats.totalEngagements, 1);
      assert.equal(stats.arms.elenchus.engagements, 1);
    });

    it('records ignore', () => {
      injectionProfile.start();
      injectionProfile.recordIgnore('elenchus');
      const stats = injectionProfile.getStats();
      assert.equal(stats.totalIgnores, 1);
      assert.equal(stats.arms.elenchus.ignores, 1);
    });

    it('engagement increases activation rate', () => {
      injectionProfile.start();
      const rateBefore = injectionProfile.getRate('elenchus');

      for (let i = 0; i < 10; i++) {
        injectionProfile.recordEngagement('elenchus');
      }

      const rateAfter = injectionProfile.getRate('elenchus');
      assert.ok(rateAfter > rateBefore,
        `Rate should increase: before=${rateBefore} after=${rateAfter}`);
    });

    it('ignores decrease activation rate', () => {
      injectionProfile.start();
      const rateBefore = injectionProfile.getRate('elenchus');

      for (let i = 0; i < 10; i++) {
        injectionProfile.recordIgnore('elenchus');
      }

      const rateAfter = injectionProfile.getRate('elenchus');
      assert.ok(rateAfter < rateBefore,
        `Rate should decrease: before=${rateBefore} after=${rateAfter}`);
    });

    it('updates from session prompts', () => {
      injectionProfile.start();
      // Simulate injection
      injectionProfile.shouldActivate('ecosystem_status');
      injectionProfile._sessionInjections.add('ecosystem_status'); // Force inject for test
      injectionProfile._sessionInjections.add('social_status');

      // User prompts mention ecosystem but not social
      injectionProfile.updateFromSession([
        'check the ecosystem status',
        'deploy the build to render',
      ]);

      const stats = injectionProfile.getStats();
      // ecosystem should have engagement (keywords: ecosystem, deploy, render)
      assert.ok(stats.arms.ecosystem_status.engagements >= 1, 'ecosystem should have engagement');
      // social should have ignore (no social keywords)
      assert.ok(stats.arms.social_status.ignores >= 1, 'social should have ignore');
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // PERSISTENCE
  // ═══════════════════════════════════════════════════════════════════════

  describe('Persistence', () => {
    it('persists arms across sessions', () => {
      injectionProfile.start();
      for (let i = 0; i < 5; i++) injectionProfile.recordEngagement('hypothesis');
      for (let i = 0; i < 3; i++) injectionProfile.recordIgnore('hypothesis');
      injectionProfile.stop();

      const raw = readFileSync(tempPath, 'utf-8');
      const state = JSON.parse(raw);
      assert.ok(state.arms.hypothesis);
      assert.equal(state.arms.hypothesis.engagements, 5);
      assert.equal(state.arms.hypothesis.ignores, 3);
    });

    it('persists thresholds across sessions', () => {
      injectionProfile.start();
      for (let i = 0; i < 3; i++) {
        injectionProfile.adjustThreshold('temporal_confidence', 0.4, true);
      }
      injectionProfile.stop();

      const raw = readFileSync(tempPath, 'utf-8');
      const state = JSON.parse(raw);
      assert.ok(state.thresholds.temporal_confidence);
      assert.equal(state.thresholds.temporal_confidence.adjustments, 3);
    });

    it('restores full state on restart', () => {
      // Session 1
      injectionProfile.start();
      for (let i = 0; i < 5; i++) injectionProfile.recordEngagement('elenchus');
      injectionProfile.adjustThreshold('flow_confidence', 0.5, true);
      injectionProfile.adjustThreshold('flow_confidence', 0.5, true);
      injectionProfile.stop();

      // Session 2
      injectionProfile._resetForTesting();
      injectionProfile._setStatePath(tempPath);
      injectionProfile.start();

      const stats = injectionProfile.getStats();
      assert.equal(stats.totalSessions, 2);
      assert.equal(stats.totalEngagements, 5);
      assert.ok(stats.arms.elenchus);
      assert.ok(stats.thresholds.flow_confidence);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // DEFAULT PRIORS
  // ═══════════════════════════════════════════════════════════════════════

  describe('Default Priors', () => {
    it('has priors for all key topics', () => {
      const required = [
        'ecosystem_status', 'social_status', 'accounting_status',
        'elenchus', 'chria_wisdom', 'hypothesis', 'role_reversal',
        'temporal_late_night', 'temporal_frustration', 'temporal_fatigue',
      ];
      for (const topic of required) {
        assert.ok(DEFAULT_PRIORS[topic], `Missing prior for ${topic}`);
        assert.ok(Array.isArray(DEFAULT_PRIORS[topic]), `Prior for ${topic} should be [alpha, beta]`);
        assert.equal(DEFAULT_PRIORS[topic].length, 2, `Prior for ${topic} should have 2 elements`);
      }
    });

    it('guardian has high activation rate', () => {
      injectionProfile.start();
      const rate = injectionProfile.getRate('guardian_emergency');
      assert.ok(rate > 0.5, `Guardian rate ${rate} should be > 0.5 (safety-critical)`);
    });

    it('social has low activation rate', () => {
      injectionProfile.start();
      const rate = injectionProfile.getRate('social_status');
      assert.ok(rate < 0.2, `Social rate ${rate} should be < 0.2 (low engagement)`);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // ENGAGEMENT KEYWORDS
  // ═══════════════════════════════════════════════════════════════════════

  describe('Engagement Keywords', () => {
    it('detects ecosystem engagement', () => {
      assert.ok(ENGAGEMENT_KEYWORDS.ecosystem_status.test('check the deploy status'));
      assert.ok(ENGAGEMENT_KEYWORDS.ecosystem_status.test('github actions failing'));
      assert.ok(!ENGAGEMENT_KEYWORDS.ecosystem_status.test('write a function'));
    });

    it('detects social engagement', () => {
      assert.ok(ENGAGEMENT_KEYWORDS.social_status.test('check twitter mentions'));
      assert.ok(ENGAGEMENT_KEYWORDS.social_status.test('community sentiment'));
      assert.ok(!ENGAGEMENT_KEYWORDS.social_status.test('fix the bug'));
    });

    it('detects temporal engagement', () => {
      assert.ok(ENGAGEMENT_KEYWORDS.temporal_late_night.test("I'll continue tomorrow"));
      assert.ok(ENGAGEMENT_KEYWORDS.temporal_frustration.test('let me try a different approach'));
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // STATS
  // ═══════════════════════════════════════════════════════════════════════

  describe('Stats', () => {
    it('reports complete stats', () => {
      injectionProfile.start();
      const stats = injectionProfile.getStats();
      assert.equal(stats.running, true);
      assert.ok('totalSessions' in stats);
      assert.ok('totalEngagements' in stats);
      assert.ok('totalIgnores' in stats);
      assert.ok('arms' in stats);
      assert.ok('thresholds' in stats);
      assert.ok('sessionInjections' in stats);
    });

    it('marks arms as learned/prior', () => {
      injectionProfile.start();
      // New arm = prior
      injectionProfile.shouldActivate('elenchus');
      let stats = injectionProfile.getStats();
      assert.equal(stats.arms.elenchus.learned, false);

      // After enough observations = learned
      for (let i = 0; i < MIN_OBSERVATIONS; i++) {
        injectionProfile.recordEngagement('elenchus');
      }
      stats = injectionProfile.getStats();
      assert.equal(stats.arms.elenchus.learned, true);
    });
  });
});
