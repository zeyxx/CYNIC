/**
 * Metrics Service
 *
 * Prometheus-format metrics for CYNIC monitoring and dashboards.
 * Collects stats from all services and exposes in standard format.
 *
 * "What gets measured gets managed" - kynikos
 *
 * @module @cynic/mcp/metrics-service
 */

'use strict';

import { EventEmitter } from 'events';
import { formatPrometheus } from './metrics/PrometheusFormatter.js';
import { formatHtml } from './metrics/HtmlReporter.js';
import { AlertManager } from './metrics/AlertManager.js';

const PHI_INV = 0.618033988749895;

/**
 * MetricsService
 *
 * Coordinates metrics collection, formatting, and alerting.
 * Delegates to specialized modules for specific concerns.
 */
export class MetricsService extends EventEmitter {
  /**
   * @param {Object} options - Configuration
   * @param {Object} [options.persistence] - PersistenceManager
   * @param {Object} [options.sessionManager] - SessionManager
   * @param {Object} [options.pojChainManager] - PoJChainManager
   * @param {Object} [options.librarian] - LibrarianService
   * @param {Object} [options.ecosystem] - EcosystemService
   * @param {Object} [options.integrator] - IntegratorService
   * @param {Object} [options.judge] - CYNICJudge
   * @param {Object} [options.collective] - CollectivePack (The Eleven Dogs)
   * @param {Object} [options.thresholds] - Alert thresholds
   */
  constructor(options = {}) {
    super();

    // Service references for metric collection
    this.persistence = options.persistence || null;
    this.sessionManager = options.sessionManager || null;
    this.pojChainManager = options.pojChainManager || null;
    this.librarian = options.librarian || null;
    this.ecosystem = options.ecosystem || null;
    this.integrator = options.integrator || null;
    this.judge = options.judge || null;
    this.collective = options.collective || null;

    // Alert manager (handles thresholds and alert events)
    this._alertManager = new AlertManager({
      thresholds: options.thresholds,
      pojChainManager: this.pojChainManager,
    });

    // Forward alert events
    this._alertManager.on('alert', (alert) => this.emit('alert', alert));
    this._alertManager.on('alert_cleared', (alert) => this.emit('alert_cleared', alert));

    // Metrics cache
    this._metricsCache = null;
    this._lastCollect = null;

    // Stats
    this._stats = {
      collectCount: 0,
      lastCollectMs: 0,
    };
  }

  /**
   * Collect all metrics from services
   * @returns {Promise<Object>} Raw metrics object
   */
  async collect() {
    const startTime = Date.now();
    const metrics = {
      timestamp: startTime,
      judgments: {},
      sessions: {},
      cache: {},
      chain: {},
      ecosystem: {},
      integrator: {},
      agents: {},
      system: {},
    };

    // Collect from all sources in parallel where possible
    await Promise.all([
      this._collectJudgments(metrics),
      this._collectSessions(metrics),
      this._collectCache(metrics),
      this._collectChain(metrics),
      this._collectEcosystem(metrics),
      this._collectIntegrator(metrics),
      this._collectAgents(metrics),
    ]);

    // System metrics (synchronous)
    metrics.system = {
      uptime: process.uptime(),
      memoryUsed: process.memoryUsage().heapUsed,
      memoryTotal: process.memoryUsage().heapTotal,
      phi: PHI_INV,
    };

    // Update stats
    this._stats.collectCount++;
    this._stats.lastCollectMs = Date.now() - startTime;
    this._metricsCache = metrics;
    this._lastCollect = Date.now();

    // Check alerts
    await this._alertManager.checkAlerts(metrics);

    return metrics;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Collection Methods (Private)
  // ═══════════════════════════════════════════════════════════════════════════

  async _collectJudgments(metrics) {
    // From persistence
    if (this.persistence?.judgments) {
      try {
        const stats = await this.persistence.getJudgmentStats();
        metrics.judgments = {
          total: stats.total || 0,
          avgQScore: stats.avgScore || 0,
          avgConfidence: stats.avgConfidence || 0,
          byVerdict: stats.verdicts || {},
          last24h: stats.last24h || 0,
        };
      } catch (e) {
        metrics.judgments.error = e.message;
      }
    }

    // From judge in-memory
    if (this.judge) {
      const judgeStats = this.judge.getStats();
      metrics.judgments.memory = {
        total: judgeStats.totalJudgments || 0,
        avgScore: judgeStats.avgScore || 0,
        byVerdict: judgeStats.verdicts || {},
      };
    }
  }

  async _collectSessions(metrics) {
    if (this.sessionManager) {
      const sessionStats = this.sessionManager.getStats();
      metrics.sessions = {
        active: sessionStats.activeSessions || 0,
        total: sessionStats.totalSessions || 0,
        current: sessionStats.currentSession ? {
          userId: sessionStats.currentSession.userId,
          judgmentCount: sessionStats.currentSession.counters?.judgmentCount || 0,
        } : null,
      };
    }
  }

  async _collectCache(metrics) {
    if (this.librarian) {
      try {
        const libStats = await this.librarian.getStats();
        metrics.cache.library = {
          hits: libStats.cache?.totalHits || 0,
          misses: libStats.cache?.totalMisses || 0,
          hitRate: libStats.hitRate || 0,
          activeEntries: libStats.cache?.activeEntries || 0,
          memorySize: libStats.cache?.memorySize || 0,
        };
      } catch (e) {
        metrics.cache.libraryError = e.message;
      }
    }
  }

  async _collectChain(metrics) {
    if (this.pojChainManager) {
      const chainStatus = this.pojChainManager.getStatus();
      metrics.chain = {
        height: chainStatus.headSlot || 0,
        pendingJudgments: chainStatus.pendingJudgments || 0,
        blocksCreated: chainStatus.stats?.blocksCreated || 0,
        judgmentsProcessed: chainStatus.stats?.judgmentsProcessed || 0,
        initialized: chainStatus.initialized,
      };

      // Get full chain stats from persistence
      if (this.persistence?.pojBlocks) {
        try {
          const dbStats = await this.persistence.pojBlocks.getStats();
          metrics.chain.totalBlocks = dbStats.totalBlocks || 0;
          metrics.chain.totalJudgments = dbStats.totalJudgments || 0;
        } catch (e) {
          // Ignore
        }
      }
    }
  }

  async _collectEcosystem(metrics) {
    if (this.ecosystem) {
      try {
        const ecoStats = await this.ecosystem.getStats();
        metrics.ecosystem = {
          docsLoaded: ecoStats.total_docs || ecoStats.loadCount || 0,
          searchCount: ecoStats.searchCount || 0,
          hitCount: ecoStats.hitCount || 0,
        };
      } catch (e) {
        metrics.ecosystem.error = e.message;
      }
    }
  }

  async _collectIntegrator(metrics) {
    if (this.integrator) {
      const intStats = this.integrator.getStats();
      const drifts = this.integrator.getDrifts();
      metrics.integrator = {
        checksPerformed: intStats.checksPerformed || 0,
        driftsDetected: intStats.driftsDetected || 0,
        currentDrifts: drifts.length,
        criticalDrifts: drifts.filter(d => d.critical).length,
        modulesTracked: intStats.modulesTracked || 0,
        projectsTracked: intStats.projectsTracked || 0,
      };
    }
  }

  async _collectAgents(metrics) {
    if (this.collective) {
      const summary = this.collective.getSummary();
      metrics.agents = {
        enabled: true,
        agentCount: summary.agentCount || 11,
        totalDecisions: summary.collectiveStats?.totalProcessed || 0,
        cynicState: summary.cynic?.state || 'unknown',
        guardian: {
          invocations: summary.agents?.guardian?.invocations || 0,
          blocks: summary.agents?.guardian?.blocks || 0,
          warnings: summary.agents?.guardian?.warnings || 0,
        },
        analyst: {
          invocations: summary.agents?.analyst?.invocations || 0,
          patterns: summary.agents?.analyst?.patterns || 0,
        },
        scholar: { invocations: summary.agents?.scholar?.invocations || 0 },
        architect: { invocations: summary.agents?.architect?.invocations || 0 },
        sage: {
          invocations: summary.agents?.sage?.invocations || 0,
          wisdomShared: summary.agents?.sage?.wisdom || 0,
        },
        janitor: { invocations: summary.agents?.janitor?.invocations || 0 },
        scout: { invocations: summary.agents?.scout?.invocations || 0 },
        cartographer: { invocations: summary.agents?.cartographer?.invocations || 0 },
        oracle: { invocations: summary.agents?.oracle?.invocations || 0 },
        deployer: { invocations: summary.agents?.deployer?.invocations || 0 },
        cynic: {
          invocations: summary.agents?.cynic?.invocations || 0,
          eventsObserved: summary.cynic?.eventsObserved || 0,
        },
      };
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Export Methods
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Export metrics in Prometheus format
   * @param {Object} [metrics] - Pre-collected metrics (or collect fresh)
   * @returns {Promise<string>} Prometheus format string
   */
  async toPrometheus(metrics = null) {
    if (!metrics) {
      metrics = await this.collect();
    }
    return formatPrometheus(metrics, { alertCount: this._alertManager.count });
  }

  /**
   * Generate a simple HTML dashboard
   * @param {Object} [metrics] - Pre-collected metrics
   * @returns {Promise<string>} HTML string
   */
  async toHTML(metrics = null) {
    if (!metrics) {
      metrics = await this.collect();
    }
    return formatHtml(metrics, {
      alerts: this._alertManager.getAlerts(),
      lastCollectMs: this._stats.lastCollectMs,
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Alert Methods (Delegate to AlertManager)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get current alerts
   * @returns {Object[]} Active alerts
   */
  getAlerts() {
    return this._alertManager.getAlerts();
  }

  /**
   * Get specific alert by type
   * @param {string} type - Alert type
   * @returns {Object|null} Alert or null
   */
  getAlert(type) {
    return this._alertManager.getAlert(type);
  }

  /**
   * Clear an alert (acknowledge)
   * @param {string} type - Alert type to clear
   * @returns {boolean} Whether alert was found and cleared
   */
  clearAlert(type) {
    return this._alertManager.clearAlert(type);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Cache & Stats
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get cached metrics (without fresh collect)
   * @returns {Object|null} Cached metrics or null
   */
  getCached() {
    return this._metricsCache;
  }

  /**
   * Get service stats
   * @returns {Object} Stats
   */
  getStats() {
    return {
      ...this._stats,
      alertsActive: this._alertManager.count,
      alertsTriggered: this._alertManager.getStats().alertsTriggered,
      lastCollect: this._lastCollect,
      thresholds: this._alertManager.thresholds,
    };
  }

  /**
   * Update thresholds
   * @param {Object} thresholds - New thresholds to merge
   */
  setThresholds(thresholds) {
    this._alertManager.setThresholds(thresholds);
  }
}

export default MetricsService;
