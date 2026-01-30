/**
 * @cynic/persistence - Burnout Detection Tests
 *
 * v1.1: Tests for burnout detection service
 *
 * @module @cynic/persistence/test/burnout-detection
 */

import { describe, it, mock } from 'node:test';
import assert from 'node:assert/strict';

import {
  BurnoutDetection,
  createBurnoutDetection,
  DEFAULT_BURNOUT_CONFIG,
  WarningSeverity,
  WarningType,
} from '../src/services/burnout-detection.js';

// =============================================================================
// MOCK HELPERS
// =============================================================================

function createMockPool(queryResults = {}) {
  return {
    query: mock.fn(async (sql) => queryResults[sql] || { rows: [], rowCount: 0 }),
  };
}

// =============================================================================
// DEFAULT CONFIG TESTS
// =============================================================================

describe('DEFAULT_BURNOUT_CONFIG', () => {
  it('should have φ-derived thresholds', () => {
    assert.ok(DEFAULT_BURNOUT_CONFIG.riskThreshold > 0.6);
    assert.ok(DEFAULT_BURNOUT_CONFIG.criticalThreshold > 0.7);
    assert.ok(DEFAULT_BURNOUT_CONFIG.energyCritical > 0.3);
    assert.ok(DEFAULT_BURNOUT_CONFIG.frustrationHigh > 0.6);
  });

  it('should have Fibonacci trend window', () => {
    assert.strictEqual(DEFAULT_BURNOUT_CONFIG.trendWindowSize, 13);
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(DEFAULT_BURNOUT_CONFIG));
  });
});

describe('Warning constants', () => {
  it('should have severity levels', () => {
    assert.strictEqual(WarningSeverity.INFO, 'info');
    assert.strictEqual(WarningSeverity.CAUTION, 'caution');
    assert.strictEqual(WarningSeverity.WARNING, 'warning');
    assert.strictEqual(WarningSeverity.CRITICAL, 'critical');
  });

  it('should have warning types', () => {
    assert.strictEqual(WarningType.ENERGY_DECLINING, 'energy_declining');
    assert.strictEqual(WarningType.BURNOUT_ACTIVE, 'burnout_active');
    assert.strictEqual(WarningType.RECOVERY_DETECTED, 'recovery_detected');
  });
});

// =============================================================================
// BURNOUT DETECTION TESTS
// =============================================================================

describe('BurnoutDetection', () => {
  describe('Construction', () => {
    it('should require a pool', () => {
      assert.throws(() => new BurnoutDetection(), /requires.*pool/i);
    });

    it('should create with pool', () => {
      const pool = createMockPool();
      const service = new BurnoutDetection({ pool });

      assert.ok(service);
      const stats = service.getStats();
      assert.strictEqual(stats.snapshotsRecorded, 0);
    });

    it('should accept custom config', () => {
      const pool = createMockPool();
      const service = new BurnoutDetection({
        pool,
        config: { trendWindowSize: 21 },
      });

      const stats = service.getStats();
      assert.strictEqual(stats.config.trendWindowSize, 21);
    });
  });

  describe('Burnout Score Calculation', () => {
    it('should calculate burnout score correctly', () => {
      const pool = createMockPool();
      const service = new BurnoutDetection({ pool });

      // Access private method via prototype
      const calculateBurnout = service._calculateBurnoutScore.bind(service);

      // High energy, low frustration = low burnout
      const low = calculateBurnout(0.9, 0.1, 0.8);
      assert.ok(low < 0.2, `Expected low burnout, got ${low}`);

      // Low energy, high frustration = high burnout
      const high = calculateBurnout(0.2, 0.9, 0.2);
      assert.ok(high > 0.5, `Expected high burnout, got ${high}`);

      // Moderate case
      const moderate = calculateBurnout(0.5, 0.5, 0.5);
      assert.ok(moderate > 0.2 && moderate < 0.5, `Expected moderate burnout, got ${moderate}`);
    });

    it('should factor in creativity', () => {
      const pool = createMockPool();
      const service = new BurnoutDetection({ pool });
      const calculateBurnout = service._calculateBurnoutScore.bind(service);

      // Same energy/frustration, different creativity
      const withHighCreativity = calculateBurnout(0.5, 0.5, 0.9);
      const withLowCreativity = calculateBurnout(0.5, 0.5, 0.2);

      // Low creativity should amplify burnout
      assert.ok(withLowCreativity > withHighCreativity,
        `Low creativity should increase burnout: ${withLowCreativity} > ${withHighCreativity}`);
    });
  });

  describe('Flow Score Calculation', () => {
    it('should calculate flow score correctly', () => {
      const pool = createMockPool();
      const service = new BurnoutDetection({ pool });
      const calculateFlow = service._calculateFlowScore.bind(service);

      // High energy, focus, creativity, low frustration = high flow
      const high = calculateFlow(0.9, 0.9, 0.9, 0.1);
      assert.ok(high > 0.7, `Expected high flow, got ${high}`);

      // Low energy, focus = low flow
      const low = calculateFlow(0.2, 0.2, 0.2, 0.8);
      assert.ok(low < 0.2, `Expected low flow, got ${low}`);
    });
  });

  describe('Snapshot Recording', () => {
    it('should record snapshot with calculated scores', async () => {
      const pool = createMockPool();
      pool.query = mock.fn(async (sql) => {
        if (sql.includes('INSERT INTO psychology_snapshots')) {
          return {
            rows: [{
              id: 'snap_123',
              user_id: 'user_123',
              energy: 0.8,
              focus: 0.7,
              creativity: 0.6,
              frustration: 0.3,
              burnout_score: 0.1,
              flow_score: 0.7,
            }],
          };
        }
        if (sql.includes('burnout_episodes')) {
          return { rows: [] };
        }
        return { rows: [] };
      });

      const service = new BurnoutDetection({ pool });
      const result = await service.recordSnapshot('user_123', {
        energy: 0.8,
        focus: 0.7,
        creativity: 0.6,
        frustration: 0.3,
      }, { sessionId: 'sess_123' });

      assert.ok(result.snapshot);
      assert.ok(result.burnoutScore < 0.3);
      assert.ok(result.flowScore > 0.5);
    });
  });

  describe('Trend Analysis', () => {
    it('should detect declining energy trend', async () => {
      const pool = createMockPool();
      pool.query = mock.fn(async () => ({
        rows: [
          { energy: 0.4, focus: 0.5, frustration: 0.6, burnout_score: 0.5, flow_score: 0.3, created_at: new Date() },
          { energy: 0.5, focus: 0.5, frustration: 0.5, burnout_score: 0.4, flow_score: 0.4, created_at: new Date(Date.now() - 60000) },
          { energy: 0.6, focus: 0.5, frustration: 0.4, burnout_score: 0.3, flow_score: 0.5, created_at: new Date(Date.now() - 120000) },
          { energy: 0.7, focus: 0.5, frustration: 0.3, burnout_score: 0.2, flow_score: 0.6, created_at: new Date(Date.now() - 180000) },
          { energy: 0.8, focus: 0.5, frustration: 0.2, burnout_score: 0.1, flow_score: 0.7, created_at: new Date(Date.now() - 240000) },
        ],
      }));

      const service = new BurnoutDetection({ pool });
      const trends = await service.getTrends('user_123');

      assert.ok(trends.hasSufficientData);
      assert.ok(trends.trends.energy.direction.includes('declining'),
        `Expected declining energy, got ${trends.trends.energy.direction}`);
      assert.ok(trends.trends.burnout.direction.includes('rising'),
        `Expected rising burnout, got ${trends.trends.burnout.direction}`);
    });

    it('should detect stable state', async () => {
      const pool = createMockPool();
      pool.query = mock.fn(async () => ({
        rows: [
          { energy: 0.6, focus: 0.6, frustration: 0.3, burnout_score: 0.2, flow_score: 0.6, created_at: new Date() },
          { energy: 0.6, focus: 0.6, frustration: 0.3, burnout_score: 0.2, flow_score: 0.6, created_at: new Date(Date.now() - 60000) },
          { energy: 0.6, focus: 0.6, frustration: 0.3, burnout_score: 0.2, flow_score: 0.6, created_at: new Date(Date.now() - 120000) },
        ],
      }));

      const service = new BurnoutDetection({ pool });
      const trends = await service.getTrends('user_123');

      assert.ok(trends.hasSufficientData);
      assert.strictEqual(trends.trends.energy.direction, 'stable');
      assert.strictEqual(trends.trends.burnout.direction, 'stable');
    });

    it('should handle insufficient data', async () => {
      const pool = createMockPool();
      pool.query = mock.fn(async () => ({
        rows: [
          { energy: 0.6, burnout_score: 0.2 },
        ],
      }));

      const service = new BurnoutDetection({ pool });
      const trends = await service.getTrends('user_123');

      assert.strictEqual(trends.hasSufficientData, false);
      assert.strictEqual(trends.snapshotCount, 1);
    });
  });

  describe('Burnout Risk Assessment', () => {
    it('should return low risk for healthy state', async () => {
      const pool = createMockPool();
      pool.query = mock.fn(async (sql) => {
        if (sql.includes('ORDER BY created_at DESC')) {
          return {
            rows: [{
              energy: 0.8,
              frustration: 0.2,
              burnout_score: 0.1,
              flow_score: 0.8,
            }],
          };
        }
        return { rows: [] };
      });

      const service = new BurnoutDetection({ pool });
      const risk = await service.getBurnoutRisk('user_123');

      assert.strictEqual(risk.risk.level, 'low');
      assert.ok(risk.risk.score < 0.3);
      assert.strictEqual(risk.risk.recommendation.action, 'continue');
    });

    it('should return high risk for burnout state', async () => {
      const pool = createMockPool();
      pool.query = mock.fn(async (sql) => {
        if (sql.includes('ORDER BY created_at DESC') && sql.includes('LIMIT 1')) {
          return {
            rows: [{
              energy: 0.2,
              frustration: 0.9,
              burnout_score: 0.7,
              flow_score: 0.1,
            }],
          };
        }
        return { rows: [] };
      });

      const service = new BurnoutDetection({ pool });
      const risk = await service.getBurnoutRisk('user_123');

      assert.ok(risk.risk.level === 'high' || risk.risk.level === 'critical',
        `Expected high/critical risk, got ${risk.risk.level}`);
      assert.ok(risk.risk.score > 0.5);
    });
  });

  describe('Proactive Warnings', () => {
    it('should generate warning for high burnout', async () => {
      const pool = createMockPool();
      pool.query = mock.fn(async (sql) => {
        if (sql.includes('FROM burnout_warnings')) {
          return { rows: [] }; // No recent warnings
        }
        if (sql.includes('FROM psychology_snapshots') && sql.includes('LIMIT 1')) {
          return {
            rows: [{
              energy: 0.2,
              frustration: 0.8,
              burnout_score: 0.7,
              flow_score: 0.2,
            }],
          };
        }
        if (sql.includes('FROM psychology_snapshots') && sql.includes('LIMIT $2')) {
          return { rows: [] }; // No trend data
        }
        if (sql.includes('FROM burnout_episodes')) {
          return { rows: [] };
        }
        if (sql.includes('INSERT INTO burnout_warnings')) {
          return { rows: [] };
        }
        return { rows: [] };
      });

      const service = new BurnoutDetection({ pool });
      const warnings = await service.getProactiveWarnings('user_123');

      assert.ok(warnings.length > 0, 'Expected at least one warning');
      assert.ok(warnings.some(w =>
        w.type === WarningType.BURNOUT_THRESHOLD_NEAR ||
        w.type === WarningType.BURNOUT_ACTIVE
      ));
    });

    it('should respect warning cooldown', async () => {
      const pool = createMockPool();
      pool.query = mock.fn(async (sql) => {
        if (sql.includes('FROM burnout_warnings')) {
          return { rows: [{ warning_type: WarningType.BURNOUT_THRESHOLD_NEAR }] }; // Recent warning exists
        }
        if (sql.includes('FROM psychology_snapshots') && sql.includes('LIMIT 1')) {
          return {
            rows: [{
              energy: 0.2,
              frustration: 0.8,
              burnout_score: 0.7,
              flow_score: 0.2,
            }],
          };
        }
        if (sql.includes('FROM psychology_snapshots') && sql.includes('LIMIT $2')) {
          return { rows: [] };
        }
        if (sql.includes('FROM burnout_episodes')) {
          return { rows: [] };
        }
        return { rows: [] };
      });

      const service = new BurnoutDetection({ pool });
      const warnings = await service.getProactiveWarnings('user_123');

      // Should not duplicate the recent warning
      assert.ok(!warnings.some(w => w.type === WarningType.BURNOUT_THRESHOLD_NEAR),
        'Should not repeat recent warning');
    });
  });

  describe('Episode Tracking', () => {
    it('should get burnout history', async () => {
      const pool = createMockPool();
      let callCount = 0;
      pool.query = mock.fn(async (sql) => {
        callCount++;
        // First call is episode list, second is stats
        if (callCount === 1) {
          return {
            rows: [
              {
                id: 'ep_1',
                started_at: new Date(Date.now() - 86400000),
                ended_at: new Date(Date.now() - 84600000),
                peak_burnout_score: 0.8,
                duration_minutes: 30,
              },
            ],
          };
        }
        // Second call is stats
        return {
          rows: [{
            total_episodes: '1',
            avg_duration: '30',
            avg_peak_score: '0.8',
            max_peak_score: '0.8',
            total_burnout_minutes: '30',
          }],
        };
      });

      const service = new BurnoutDetection({ pool });
      const history = await service.getBurnoutHistory('user_123');

      assert.strictEqual(history.episodes.length, 1);
      assert.strictEqual(history.stats.totalEpisodes, 1);
      assert.strictEqual(history.stats.avgDurationMinutes, 30);
    });
  });

  describe('Warning Feedback', () => {
    it('should record warning feedback', async () => {
      const pool = createMockPool();
      pool.query = mock.fn(async () => ({ rows: [], rowCount: 1 }));

      const service = new BurnoutDetection({ pool });
      await service.recordWarningFeedback('warn_123', true, true);

      assert.strictEqual(pool.query.mock.calls.length, 1);
      const call = pool.query.mock.calls[0];
      assert.ok(call.arguments[0].includes('UPDATE burnout_warnings'));
    });
  });

  describe('Warning Effectiveness', () => {
    it('should calculate warning effectiveness', async () => {
      const pool = createMockPool();
      pool.query = mock.fn(async () => ({
        rows: [
          { warning_type: 'energy_declining', severity: 'caution', total: '10', effective: '7', effectiveness_rate: '0.7' },
          { warning_type: 'burnout_threshold_near', severity: 'warning', total: '5', effective: '3', effectiveness_rate: '0.6' },
        ],
      }));

      const service = new BurnoutDetection({ pool });
      const effectiveness = await service.getWarningEffectiveness();

      assert.ok(effectiveness.byType.energy_declining);
      assert.strictEqual(effectiveness.byType.energy_declining.total, 10);
      assert.strictEqual(effectiveness.byType.energy_declining.rate, 0.7);
      assert.strictEqual(effectiveness.overall.total, 15);
    });
  });
});

// =============================================================================
// FACTORY FUNCTION TESTS
// =============================================================================

describe('createBurnoutDetection', () => {
  it('should create instance', () => {
    const pool = createMockPool();
    const service = createBurnoutDetection({ pool });

    assert.ok(service instanceof BurnoutDetection);
  });
});
