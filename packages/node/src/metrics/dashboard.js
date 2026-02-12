/**
 * MetricsDashboard — Data-Driven Organism Measurement
 *
 * Queries metrics tables to compute progress toward weekly goals.
 * Foundation for coordinated parallel development.
 *
 * "Data first, then decisions" - κυνικός
 *
 * @module @cynic/node/metrics/dashboard
 */

'use strict';

import { createLogger, PHI_INV, PHI_INV_2 } from '@cynic/core';
import { PostgresClient } from '@cynic/persistence';

const log = createLogger('MetricsDashboard');

/**
 * MetricsDashboard
 *
 * Measures organism health across 5 dimensions:
 * - Perception (watchers active)
 * - Learning (loops consuming data)
 * - Routing (accuracy, diversity)
 * - Cost (budget awareness)
 * - Multi-domain (cross-domain routing)
 */
export class MetricsDashboard {
  constructor(options = {}) {
    this.db = options.db || new PostgresClient();
  }

  /**
   * Compute Week 1 progress (from data-driven-roadmap.md)
   *
   * Goals:
   * G1.1: Watchers polling ≥3 active
   * G1.2: Learning loops ≥5 consuming data
   * G1.3: Q-weights ≥10 updates/day
   * G1.4: KabbalisticRouter ≥20 calls
   * G1.5: LLMRouter ≥10 non-Anthropic routes
   *
   * @returns {Promise<Object>} Progress report
   */
  async getWeek1Progress() {
    log.info('Computing Week 1 progress');

    try {
      const [
        watchersActive,
        learningLoopsActive,
        qUpdatesToday,
        kabbalisticCalls,
        llmNonAnthropicRoutes,
        budgetStatus,
      ] = await Promise.all([
        this._getActiveWatchers(),
        this._getActiveLearningLoops(),
        this._getQUpdatesToday(),
        this._getKabbalisticRouterCalls(),
        this._getLLMNonAnthropicRoutes(),
        this._getBudgetStatus(),
      ]);

      // Compute pass/fail for each goal
      const goals = {
        'G1.1': { target: 3, actual: watchersActive, pass: watchersActive >= 3 },
        'G1.2': { target: 5, actual: learningLoopsActive, pass: learningLoopsActive >= 5 },
        'G1.3': { target: 10, actual: qUpdatesToday, pass: qUpdatesToday >= 10 },
        'G1.4': { target: 20, actual: kabbalisticCalls, pass: kabbalisticCalls >= 20 },
        'G1.5': { target: 10, actual: llmNonAnthropicRoutes, pass: llmNonAnthropicRoutes >= 10 },
      };

      const passCount = Object.values(goals).filter(g => g.pass).length;
      const totalGoals = Object.keys(goals).length;

      return {
        timestamp: new Date().toISOString(),
        week: 1,
        goals,
        summary: {
          passCount,
          totalGoals,
          passRatio: passCount / totalGoals,
          weekComplete: passCount >= 4, // 4/5 goals = success
        },
        budget: budgetStatus,
      };
    } catch (error) {
      log.error('Failed to compute Week 1 progress', { error: error.message });
      throw error;
    }
  }

  /**
   * Get Functional Autonomy composite score
   *
   * FA = 0.25×Perception + 0.25×Learning + 0.20×Routing + 0.15×Cost + 0.15×MultiDomain
   *
   * @returns {Promise<Object>} Functional autonomy breakdown
   */
  async getFunctionalAutonomy() {
    log.info('Computing Functional Autonomy');

    try {
      const [perception, learning, routing, cost, multiDomain] = await Promise.all([
        this._getPerceptionScore(),
        this._getLearningScore(),
        this._getRoutingScore(),
        this._getCostScore(),
        this._getMultiDomainScore(),
      ]);

      const functionalAutonomy =
        0.25 * perception +
        0.25 * learning +
        0.20 * routing +
        0.15 * cost +
        0.15 * multiDomain;

      return {
        timestamp: new Date().toISOString(),
        functionalAutonomy: Math.min(functionalAutonomy, PHI_INV), // φ-bound
        components: {
          perception,
          learning,
          routing,
          cost,
          multiDomain,
        },
        target: 0.80,
        progress: functionalAutonomy / 0.80,
      };
    } catch (error) {
      log.error('Failed to compute Functional Autonomy', { error: error.message });
      throw error;
    }
  }

  /**
   * Get learning velocity (maturity gain per week)
   *
   * Measures how fast the organism is improving across batches.
   *
   * @param {number} batchN - Current batch number
   * @returns {Promise<Object>} Learning velocity
   */
  async getLearningVelocity(batchN = 1) {
    log.info('Computing learning velocity', { batchN });

    try {
      // Get consciousness snapshots for last 2 weeks
      const { rows } = await this.db.query(`
        SELECT
          timestamp,
          q_updates_today,
          patterns_count,
          calibration_ece,
          routing_accuracy_24h
        FROM consciousness_snapshots
        WHERE timestamp > NOW() - INTERVAL '14 days'
        ORDER BY timestamp ASC
      `);

      if (rows.length < 2) {
        return {
          velocity: 0,
          trend: 'insufficient_data',
          snapshots: rows.length,
        };
      }

      // Compute maturity scores for first and last snapshot
      const first = this._computeMaturityScore(rows[0]);
      const last = this._computeMaturityScore(rows[rows.length - 1]);

      const timeDiffDays = (new Date(rows[rows.length - 1].timestamp) - new Date(rows[0].timestamp)) / (1000 * 60 * 60 * 24);
      const velocityPerWeek = ((last - first) / timeDiffDays) * 7;

      return {
        velocity: velocityPerWeek,
        trend: velocityPerWeek > 0 ? 'improving' : 'declining',
        currentMaturity: last,
        weekAgoMaturity: first,
        snapshotsAnalyzed: rows.length,
      };
    } catch (error) {
      log.error('Failed to compute learning velocity', { error: error.message });
      throw error;
    }
  }

  // =============================================================================
  // PRIVATE HELPERS
  // =============================================================================

  /**
   * Get number of active watchers (last 5 minutes)
   */
  async _getActiveWatchers() {
    const { rows } = await this.db.query(`
      SELECT COUNT(DISTINCT watcher_name) as count
      FROM watcher_heartbeats
      WHERE timestamp > NOW() - INTERVAL '5 minutes'
      AND status = 'active'
    `);
    return rows[0]?.count || 0;
  }

  /**
   * Get number of active learning loops (consuming data today)
   */
  async _getActiveLearningLoops() {
    const { rows } = await this.db.query(`
      SELECT COUNT(DISTINCT task_name) as count
      FROM background_tasks
      WHERE task_type = 'learning'
      AND started_at > NOW() - INTERVAL '24 hours'
      AND success_count > 0
    `);
    return rows[0]?.count || 0;
  }

  /**
   * Get Q-Learning weight updates today
   */
  async _getQUpdatesToday() {
    // Q-Learning updates tracked in learning_state table (migration 005)
    // This is a placeholder - actual implementation depends on learning persistence
    const { rows } = await this.db.query(`
      SELECT COALESCE(SUM(success_count), 0) as count
      FROM background_tasks
      WHERE task_type = 'learning'
      AND task_name LIKE '%q-learning%'
      AND started_at > NOW() - INTERVAL '24 hours'
    `);
    return parseInt(rows[0]?.count || 0);
  }

  /**
   * Get KabbalisticRouter calls (last 24h)
   */
  async _getKabbalisticRouterCalls() {
    const { rows } = await this.db.query(`
      SELECT COUNT(*) as count
      FROM routing_accuracy
      WHERE router_type = 'kabbalistic'
      AND timestamp > NOW() - INTERVAL '24 hours'
    `);
    return parseInt(rows[0]?.count || 0);
  }

  /**
   * Get LLM non-Anthropic routes (last 24h)
   */
  async _getLLMNonAnthropicRoutes() {
    const { rows } = await this.db.query(`
      SELECT COUNT(*) as count
      FROM routing_accuracy
      WHERE router_type = 'llm'
      AND timestamp > NOW() - INTERVAL '24 hours'
      AND metadata->>'provider' != 'anthropic'
    `);
    return parseInt(rows[0]?.count || 0);
  }

  /**
   * Get budget status
   */
  async _getBudgetStatus() {
    // Budget tracked in CostLedger (JSON file currently)
    // This is a placeholder - will connect to CostLedger API
    return {
      consumed: 0,
      remaining: 10.00,
      consumedRatio: 0,
      status: 'unknown',
    };
  }

  /**
   * Compute Perception score (0-1)
   * Based on: watchers active, event polling rate
   */
  async _getPerceptionScore() {
    const activeWatchers = await this._getActiveWatchers();
    const targetWatchers = 7; // One per reality dimension

    const { rows } = await this.db.query(`
      SELECT COALESCE(AVG(events_polled), 0) as avg_events
      FROM watcher_heartbeats
      WHERE timestamp > NOW() - INTERVAL '1 hour'
    `);
    const avgEventsPerHour = parseFloat(rows[0]?.avg_events || 0);

    // Perception score = 60% watchers + 40% event rate
    const watcherRatio = Math.min(activeWatchers / targetWatchers, 1);
    const eventRatio = Math.min(avgEventsPerHour / 10, 1); // Target: 10 events/hour

    return 0.6 * watcherRatio + 0.4 * eventRatio;
  }

  /**
   * Compute Learning score (0-1)
   * Based on: loops active, Q-updates, calibration quality
   */
  async _getLearningScore() {
    const loopsActive = await this._getActiveLearningLoops();
    const targetLoops = 11; // All 11 learning loops

    const qUpdates = await this._getQUpdatesToday();
    const targetQUpdates = 50; // Target: 50 updates/day

    // Get latest calibration ECE from consciousness snapshots
    const { rows } = await this.db.query(`
      SELECT calibration_ece
      FROM consciousness_snapshots
      ORDER BY timestamp DESC
      LIMIT 1
    `);
    const ece = parseFloat(rows[0]?.calibration_ece || 0.5);
    const calibrationScore = Math.max(0, 1 - ece); // Lower ECE = better

    // Learning score = 40% loops + 30% Q-updates + 30% calibration
    const loopRatio = Math.min(loopsActive / targetLoops, 1);
    const qRatio = Math.min(qUpdates / targetQUpdates, 1);

    return 0.4 * loopRatio + 0.3 * qRatio + 0.3 * calibrationScore;
  }

  /**
   * Compute Routing score (0-1)
   * Based on: accuracy, Dog diversity, latency
   */
  async _getRoutingScore() {
    const accuracy = await this.db.query(`
      SELECT get_routing_accuracy_24h() as accuracy
    `);
    const accuracyScore = parseFloat(accuracy.rows[0]?.accuracy || 0);

    // Get Dog diversity (how many different Dogs used in last 24h)
    const { rows: diversityRows } = await this.db.query(`
      SELECT COUNT(DISTINCT dog) as unique_dogs
      FROM (
        SELECT unnest(dogs_selected) as dog
        FROM routing_accuracy
        WHERE timestamp > NOW() - INTERVAL '24 hours'
      ) subquery
    `);
    const uniqueDogs = parseInt(diversityRows[0]?.unique_dogs || 0);
    const diversityScore = Math.min(uniqueDogs / 11, 1); // Target: all 11 Dogs

    // Routing score = 70% accuracy + 30% diversity
    return 0.7 * accuracyScore + 0.3 * diversityScore;
  }

  /**
   * Compute Cost score (0-1)
   * Based on: budget awareness, optimization actions
   */
  async _getCostScore() {
    // Placeholder - connect to CostLedger
    return 0.1; // Very low until CostLedger is wired
  }

  /**
   * Compute Multi-Domain score (0-1)
   * Based on: cross-domain routing, 7×7 matrix coverage
   */
  async _getMultiDomainScore() {
    // Count unique domains used in routing
    const { rows } = await this.db.query(`
      SELECT COUNT(DISTINCT metadata->>'domain') as unique_domains
      FROM routing_accuracy
      WHERE timestamp > NOW() - INTERVAL '24 hours'
      AND metadata->>'domain' IS NOT NULL
    `);
    const uniqueDomains = parseInt(rows[0]?.unique_domains || 0);
    return Math.min(uniqueDomains / 7, 1); // Target: all 7 domains
  }

  /**
   * Compute maturity score from snapshot data
   */
  _computeMaturityScore(snapshot) {
    const qScore = Math.min((snapshot.q_updates_today || 0) / 50, 1);
    const patternsScore = Math.min((snapshot.patterns_count || 0) / 100, 1);
    const calibrationScore = Math.max(0, 1 - (snapshot.calibration_ece || 0.5));
    const routingScore = snapshot.routing_accuracy_24h || 0;

    return (qScore + patternsScore + calibrationScore + routingScore) / 4;
  }

  /**
   * Close database connection
   */
  async close() {
    if (this.db) {
      await this.db.close();
    }
  }
}

export default MetricsDashboard;
