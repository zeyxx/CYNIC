/**
 * MetricsService Tests
 *
 * "What gets measured gets managed" - κυνικός
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert/strict';
import { MetricsService } from '../src/metrics-service.js';

/**
 * Create mock services for testing
 */
function createMockServices() {
  return {
    persistence: {
      judgments: true,
      pojBlocks: {
        getStats: async () => ({ totalBlocks: 10, totalJudgments: 100 }),
      },
      getJudgmentStats: async () => ({
        total: 50,
        avgScore: 65.5,
        avgConfidence: 0.55,
        verdicts: { WAG: 20, HOWL: 5, GROWL: 15, BARK: 10 },
        last24h: 12,
      }),
      sessions: {
        getStats: async () => ({
          total: 2,
          totalJudgments: 8,
          totalDigests: 0,
          totalFeedback: 0,
        }),
      },
    },
    sessionManager: {
      getSummary: () => ({
        activeCount: 2,
        currentSession: 'test-session-id',
        sessions: [
          { sessionId: 'session-1', userId: 'test-us...', project: 'test', judgmentCount: 5, createdAt: Date.now() },
          { sessionId: 'session-2', userId: 'other-u...', project: 'test', judgmentCount: 3, createdAt: Date.now() },
        ],
      }),
    },
    pojChainManager: {
      getStatus: () => ({
        headSlot: 42,
        pendingJudgments: 3,
        initialized: true,
        stats: {
          blocksCreated: 42,
          judgmentsProcessed: 420,
        },
      }),
      verifyIntegrity: async () => ({ valid: true, blocksChecked: 42, errors: [] }),
    },
    librarian: {
      getStats: async () => ({
        cache: {
          totalHits: 100,
          totalMisses: 20,
          activeEntries: 15,
          memorySize: 1024,
        },
        hitRate: 0.833,
      }),
    },
    ecosystem: {
      getStats: async () => ({
        total_docs: 12,
        searchCount: 50,
        hitCount: 45,
      }),
    },
    integrator: {
      getStats: () => ({
        checksPerformed: 5,
        driftsDetected: 2,
        modulesTracked: 3,
        projectsTracked: 5,
      }),
      getDrifts: () => [
        { type: 'hash_mismatch', module: 'harmony.js', critical: true },
      ],
    },
    judge: {
      getStats: () => ({
        totalJudgments: 100,
        avgScore: 60,
        verdicts: { WAG: 40, GROWL: 30, BARK: 20, HOWL: 10 },
      }),
    },
    collective: {
      getSummary: () => ({
        agentCount: 11,
        collectiveStats: { totalProcessed: 50 },
        cynic: { state: 'active', eventsObserved: 100 },
        agents: {
          guardian: { invocations: 20, blocks: 5, warnings: 10 },
          analyst: { invocations: 15, patterns: 15 },
          scholar: { invocations: 10 },
          architect: { invocations: 8 },
          sage: { invocations: 12, wisdom: 8 },
          janitor: { invocations: 5 },
          scout: { invocations: 7 },
          cartographer: { invocations: 6 },
          oracle: { invocations: 4 },
          deployer: { invocations: 3 },
          cynic: { invocations: 100 },
        },
      }),
    },
  };
}

describe('MetricsService', () => {
  let service;
  let mocks;

  beforeEach(() => {
    mocks = createMockServices();
    service = new MetricsService(mocks);
  });

  describe('constructor', () => {
    it('creates with services', () => {
      assert.ok(service.persistence);
      assert.ok(service.judge);
      assert.ok(service.getStats().thresholds);
    });

    it('uses default thresholds', () => {
      const thresholds = service.getStats().thresholds;
      assert.equal(thresholds.avgQScoreMin, 30);
      assert.equal(thresholds.cacheHitRateMin, 0.5);
    });

    it('accepts custom thresholds', () => {
      const custom = new MetricsService({
        ...mocks,
        thresholds: { avgQScoreMin: 50 },
      });
      assert.equal(custom.getStats().thresholds.avgQScoreMin, 50);
    });
  });

  describe('collect', () => {
    it('collects metrics from all services', async () => {
      const metrics = await service.collect();

      assert.ok(metrics.timestamp);
      assert.ok(metrics.judgments);
      assert.ok(metrics.sessions);
      assert.ok(metrics.cache);
      assert.ok(metrics.chain);
      assert.ok(metrics.ecosystem);
      assert.ok(metrics.integrator);
      assert.ok(metrics.agents);
      assert.ok(metrics.system);
    });

    it('collects judgment metrics', async () => {
      const metrics = await service.collect();

      assert.equal(metrics.judgments.total, 50);
      assert.equal(metrics.judgments.avgQScore, 65.5);
      assert.ok(metrics.judgments.byVerdict);
    });

    it('collects session metrics', async () => {
      const metrics = await service.collect();

      assert.equal(metrics.sessions.active, 2);
      assert.equal(metrics.sessions.total, 2); // matches sessions array length in mock
    });

    it('collects cache metrics', async () => {
      const metrics = await service.collect();

      assert.equal(metrics.cache.library.hits, 100);
      assert.equal(metrics.cache.library.misses, 20);
      assert.ok(metrics.cache.library.hitRate > 0.8);
    });

    it('collects chain metrics', async () => {
      const metrics = await service.collect();

      assert.equal(metrics.chain.height, 42);
      assert.equal(metrics.chain.pendingJudgments, 3);
    });

    it('collects integrator metrics', async () => {
      const metrics = await service.collect();

      assert.equal(metrics.integrator.currentDrifts, 1);
      assert.equal(metrics.integrator.criticalDrifts, 1);
    });

    it('collects collective agents metrics', async () => {
      const metrics = await service.collect();

      assert.equal(metrics.agents.agentCount, 11);
      assert.equal(metrics.agents.guardian.blocks, 5);
      assert.equal(metrics.agents.analyst.patterns, 15);
    });

    it('collects system metrics', async () => {
      const metrics = await service.collect();

      assert.ok(metrics.system.uptime > 0);
      assert.ok(metrics.system.memoryUsed > 0);
      assert.equal(metrics.system.phi, 0.618033988749895);
    });

    it('updates stats on collect', async () => {
      await service.collect();
      await service.collect();

      const stats = service.getStats();
      assert.equal(stats.collectCount, 2);
      assert.ok(stats.lastCollectMs >= 0);
    });
  });

  describe('toPrometheus', () => {
    it('returns Prometheus format string', async () => {
      const prometheus = await service.toPrometheus();

      assert.ok(typeof prometheus === 'string');
      assert.ok(prometheus.includes('cynic_judgments_total'));
      assert.ok(prometheus.includes('cynic_avg_q_score'));
      assert.ok(prometheus.includes('cynic_active_sessions'));
      assert.ok(prometheus.includes('cynic_poj_chain_height'));
    });

    it('includes HELP comments', async () => {
      const prometheus = await service.toPrometheus();

      assert.ok(prometheus.includes('# HELP'));
      assert.ok(prometheus.includes('# TYPE'));
    });

    it('includes verdict labels', async () => {
      const prometheus = await service.toPrometheus();

      assert.ok(prometheus.includes('verdict="WAG"'));
      assert.ok(prometheus.includes('verdict="HOWL"'));
    });

    it('includes cache metrics', async () => {
      const prometheus = await service.toPrometheus();

      assert.ok(prometheus.includes('cynic_library_cache_hits'));
      assert.ok(prometheus.includes('cynic_library_cache_misses'));
    });

    it('includes integrator metrics', async () => {
      const prometheus = await service.toPrometheus();

      assert.ok(prometheus.includes('cynic_integrator_drifts_current'));
      assert.ok(prometheus.includes('cynic_integrator_drifts_critical'));
    });

    it('includes collective dog metrics', async () => {
      const prometheus = await service.toPrometheus();

      assert.ok(prometheus.includes('cynic_guardian_blocks'));
      assert.ok(prometheus.includes('cynic_analyst_patterns'));
      assert.ok(prometheus.includes('cynic_dog_invocations'));
    });
  });

  describe('toHTML', () => {
    it('returns HTML dashboard', async () => {
      const html = await service.toHTML();

      assert.ok(typeof html === 'string');
      assert.ok(html.includes('<!DOCTYPE html>'));
      assert.ok(html.includes('CYNIC Dashboard'));
    });

    it('includes metric sections', async () => {
      const html = await service.toHTML();

      assert.ok(html.includes('Judgments'));
      assert.ok(html.includes('Sessions'));
      assert.ok(html.includes('PoJ Chain'));
      assert.ok(html.includes('Cache'));
    });

    it('includes phi reference', async () => {
      const html = await service.toHTML();

      assert.ok(html.includes('61.8'));
      assert.ok(html.includes('κυνικός'));
    });
  });

  describe('alerts', () => {
    it('detects low Q-Score alert', async () => {
      // Override to return low score
      mocks.persistence.getJudgmentStats = async () => ({
        total: 50,
        avgScore: 20, // Below threshold
        verdicts: { WAG: 5, BARK: 45 },
      });

      await service.collect();
      const alerts = service.getAlerts();

      const lowScoreAlert = alerts.find(a => a.type === 'low_q_score');
      assert.ok(lowScoreAlert);
      assert.equal(lowScoreAlert.level, 'warning');
    });

    it('detects critical drifts alert', async () => {
      await service.collect();
      const alerts = service.getAlerts();

      const driftAlert = alerts.find(a => a.type === 'critical_drifts');
      assert.ok(driftAlert);
      assert.equal(driftAlert.level, 'critical');
    });

    it('detects chain integrity failure', async () => {
      mocks.pojChainManager.verifyIntegrity = async () => ({
        valid: false,
        blocksChecked: 10,
        errors: [{ blockNumber: 5, expected: 'abc', actual: 'xyz' }],
      });

      await service.collect();
      const alerts = service.getAlerts();

      const chainAlert = alerts.find(a => a.type === 'chain_invalid');
      assert.ok(chainAlert);
      assert.equal(chainAlert.level, 'critical');
    });

    it('emits alert event for new alerts', async () => {
      let emittedAlert = null;
      service.on('alert', (alert) => {
        emittedAlert = alert;
      });

      await service.collect();

      assert.ok(emittedAlert);
    });

    it('clears alert', async () => {
      await service.collect();
      const initialCount = service.getAlerts().length;

      const cleared = service.clearAlert('critical_drifts');

      assert.ok(cleared);
      assert.equal(service.getAlerts().length, initialCount - 1);
    });

    it('returns false for non-existent alert', () => {
      const cleared = service.clearAlert('nonexistent');
      assert.equal(cleared, false);
    });

    it('emits alert_cleared event', async () => {
      await service.collect();

      let clearedAlert = null;
      service.on('alert_cleared', (alert) => {
        clearedAlert = alert;
      });

      service.clearAlert('critical_drifts');

      assert.ok(clearedAlert);
      assert.equal(clearedAlert.type, 'critical_drifts');
    });
  });

  describe('getStats', () => {
    it('returns service stats', async () => {
      await service.collect();

      const stats = service.getStats();

      assert.equal(stats.collectCount, 1);
      assert.ok(stats.lastCollectMs >= 0);
      assert.ok(stats.thresholds);
      assert.ok(stats.alertsActive >= 0);
    });
  });

  describe('getCached', () => {
    it('returns null before first collect', () => {
      assert.equal(service.getCached(), null);
    });

    it('returns cached metrics after collect', async () => {
      await service.collect();

      const cached = service.getCached();

      assert.ok(cached);
      assert.ok(cached.timestamp);
    });
  });

  describe('getAlert', () => {
    it('returns specific alert', async () => {
      await service.collect();

      const alert = service.getAlert('critical_drifts');

      assert.ok(alert);
      assert.equal(alert.type, 'critical_drifts');
    });

    it('returns null for missing alert', () => {
      const alert = service.getAlert('nonexistent');
      assert.equal(alert, null);
    });
  });
});

describe('MetricsService without services', () => {
  it('handles missing services gracefully', async () => {
    const service = new MetricsService({});
    const metrics = await service.collect();

    assert.ok(metrics);
    assert.ok(metrics.system);
    assert.equal(metrics.judgments.total, undefined);
  });

  it('generates prometheus without errors', async () => {
    const service = new MetricsService({});
    const prometheus = await service.toPrometheus();

    assert.ok(prometheus);
    assert.ok(prometheus.includes('cynic_uptime_seconds'));
  });
});
