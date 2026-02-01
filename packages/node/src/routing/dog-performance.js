/**
 * Dog Performance Tracker
 *
 * Tracks and analyzes performance metrics for each dog (Sefira):
 * - Success/failure rates
 * - Latency statistics
 * - Task type specialization
 * - Learning curves
 *
 * "Le chien mesure ses propres pas" - κυνικός
 *
 * @module @cynic/node/routing/dog-performance
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2 } from '@cynic/core';
import { DogId, DOG_CAPABILITIES } from './dog-capabilities.js';
import { TaskType } from './task-descriptor.js';

/**
 * Performance metrics for a single dog
 */
export class DogMetrics {
  /**
   * @param {string} dogId - Dog identifier
   */
  constructor(dogId) {
    this.dogId = dogId;
    this.taskCount = 0;
    this.successCount = 0;
    this.failureCount = 0;
    this.escalationCount = 0;
    this.totalLatency = 0;
    this.minLatency = Infinity;
    this.maxLatency = 0;

    // Per task-type metrics
    this.byTaskType = new Map();

    // Recent performance window (last N tasks)
    this.recentResults = [];
    this.recentWindowSize = 20;

    // Learning curve tracking
    this.epochStats = [];
    this.epochSize = 10;

    // Created timestamp
    this.createdAt = Date.now();
    this.lastTaskAt = null;
  }

  /**
   * Record a task outcome
   *
   * @param {Object} outcome
   * @param {string} outcome.taskType - Task type
   * @param {boolean} outcome.success - Whether task succeeded
   * @param {number} outcome.latency - Task latency in ms
   * @param {boolean} [outcome.escalated=false] - Whether it was escalated
   */
  record(outcome) {
    const { taskType, success, latency, escalated = false } = outcome;

    this.taskCount++;
    this.lastTaskAt = Date.now();

    if (success) {
      this.successCount++;
    } else {
      this.failureCount++;
    }

    if (escalated) {
      this.escalationCount++;
    }

    // Latency stats
    if (latency > 0) {
      this.totalLatency += latency;
      this.minLatency = Math.min(this.minLatency, latency);
      this.maxLatency = Math.max(this.maxLatency, latency);
    }

    // Per task-type tracking
    if (taskType) {
      if (!this.byTaskType.has(taskType)) {
        this.byTaskType.set(taskType, {
          count: 0,
          success: 0,
          failure: 0,
          totalLatency: 0,
        });
      }
      const typeStats = this.byTaskType.get(taskType);
      typeStats.count++;
      if (success) {
        typeStats.success++;
      } else {
        typeStats.failure++;
      }
      if (latency > 0) {
        typeStats.totalLatency += latency;
      }
    }

    // Recent results window
    this.recentResults.push({
      success,
      latency,
      taskType,
      timestamp: Date.now(),
    });
    if (this.recentResults.length > this.recentWindowSize) {
      this.recentResults.shift();
    }

    // Epoch tracking for learning curve
    if (this.taskCount % this.epochSize === 0) {
      this.epochStats.push({
        epoch: Math.floor(this.taskCount / this.epochSize),
        successRate: this.successCount / this.taskCount,
        avgLatency: this.totalLatency / this.taskCount,
        timestamp: Date.now(),
      });
    }
  }

  /**
   * Get overall success rate
   * @returns {number} 0-1
   */
  getSuccessRate() {
    if (this.taskCount === 0) return 0;
    return this.successCount / this.taskCount;
  }

  /**
   * Get recent success rate (last N tasks)
   * @returns {number} 0-1
   */
  getRecentSuccessRate() {
    if (this.recentResults.length === 0) return 0;
    const successes = this.recentResults.filter(r => r.success).length;
    return successes / this.recentResults.length;
  }

  /**
   * Get average latency
   * @returns {number} ms
   */
  getAvgLatency() {
    if (this.taskCount === 0) return 0;
    return this.totalLatency / this.taskCount;
  }

  /**
   * Get success rate for a specific task type
   * @param {string} taskType
   * @returns {number} 0-1
   */
  getTaskTypeSuccessRate(taskType) {
    const stats = this.byTaskType.get(taskType);
    if (!stats || stats.count === 0) return 0;
    return stats.success / stats.count;
  }

  /**
   * Get best task types (by success rate)
   * @param {number} [topK=3]
   * @returns {Array<{taskType: string, successRate: number, count: number}>}
   */
  getBestTaskTypes(topK = 3) {
    const types = [];
    for (const [taskType, stats] of this.byTaskType) {
      if (stats.count >= 3) { // Minimum sample size
        types.push({
          taskType,
          successRate: stats.success / stats.count,
          count: stats.count,
        });
      }
    }
    return types
      .sort((a, b) => b.successRate - a.successRate)
      .slice(0, topK);
  }

  /**
   * Get learning curve trend
   * @returns {'improving'|'stable'|'declining'|'unknown'}
   */
  getLearningTrend() {
    if (this.epochStats.length < 3) return 'unknown';

    const recent = this.epochStats.slice(-3);
    const diffs = [];
    for (let i = 1; i < recent.length; i++) {
      diffs.push(recent[i].successRate - recent[i - 1].successRate);
    }

    const avgDiff = diffs.reduce((a, b) => a + b, 0) / diffs.length;

    if (avgDiff > 0.05) return 'improving';
    if (avgDiff < -0.05) return 'declining';
    return 'stable';
  }

  /**
   * Calculate specialization score (how specialized vs generalist)
   * @returns {number} 0-1 (1 = highly specialized)
   */
  getSpecializationScore() {
    if (this.byTaskType.size === 0) return 0;

    // Calculate Gini coefficient of task distribution
    const counts = [...this.byTaskType.values()].map(s => s.count);
    const total = counts.reduce((a, b) => a + b, 0);
    if (total === 0) return 0;

    const proportions = counts.map(c => c / total).sort();
    const n = proportions.length;

    if (n === 1) return 1; // Only one task type = fully specialized

    // Gini = 1 - 2 * sum of cumulative proportions / n
    let cumulativeSum = 0;
    for (let i = 0; i < n; i++) {
      cumulativeSum += proportions[i] * (n - i);
    }
    const gini = 1 - (2 * cumulativeSum / (n * total));

    return Math.max(0, Math.min(1, gini));
  }

  /**
   * Serialize metrics
   * @returns {Object}
   */
  toJSON() {
    const cap = DOG_CAPABILITIES[this.dogId];
    return {
      dogId: this.dogId,
      dogName: cap?.name,
      dogEmoji: cap?.emoji,
      taskCount: this.taskCount,
      successCount: this.successCount,
      failureCount: this.failureCount,
      escalationCount: this.escalationCount,
      successRate: Math.round(this.getSuccessRate() * 1000) / 1000,
      recentSuccessRate: Math.round(this.getRecentSuccessRate() * 1000) / 1000,
      escalationRate: this.taskCount > 0 ? Math.round((this.escalationCount / this.taskCount) * 1000) / 1000 : 0,
      avgLatency: Math.round(this.getAvgLatency()),
      totalLatency: this.totalLatency,
      minLatency: this.minLatency === Infinity ? 0 : this.minLatency,
      maxLatency: this.maxLatency,
      learningTrend: this.getLearningTrend(),
      specialization: Math.round(this.getSpecializationScore() * 1000) / 1000,
      bestTaskTypes: this.getBestTaskTypes(),
      epochCount: this.epochStats.length,
      createdAt: this.createdAt,
      lastTaskAt: this.lastTaskAt,
    };
  }

  /**
   * Restore metrics from serialized data
   * @param {Object} data - Serialized metrics from toJSON()
   * @returns {DogMetrics}
   */
  static fromJSON(data) {
    const metrics = new DogMetrics(data.dogId);
    metrics.taskCount = data.taskCount || 0;
    metrics.successCount = data.successCount || 0;
    metrics.failureCount = data.failureCount || 0;
    metrics.escalationCount = data.escalationCount || 0;
    metrics.totalLatency = data.totalLatency || 0;
    metrics.minLatency = data.minLatency || Infinity;
    metrics.maxLatency = data.maxLatency || 0;
    metrics.createdAt = data.createdAt || Date.now();
    metrics.lastTaskAt = data.lastTaskAt || null;
    return metrics;
  }
}

/**
 * Dog Performance Tracker - Tracks all dogs
 */
export class DogPerformanceTracker extends EventEmitter {
  /**
   * @param {Object} [options]
   * @param {boolean} [options.persistWeights=true] - Auto-persist to router
   * @param {number} [options.adaptationRate=0.1] - How fast to adapt routing
   */
  constructor(options = {}) {
    super();

    this.metrics = new Map();
    this.persistWeights = options.persistWeights !== false;
    this.adaptationRate = options.adaptationRate || 0.1;

    // Initialize metrics for all dogs
    for (const dogId of Object.values(DogId)) {
      this.metrics.set(dogId, new DogMetrics(dogId));
    }

    // Global statistics
    this.globalStats = {
      totalTasks: 0,
      successCount: 0,
      escalations: 0,
      startedAt: Date.now(),
    };
  }

  /**
   * Record task outcome for a dog
   *
   * @param {string} dogId
   * @param {Object} outcome
   */
  record(dogId, outcome) {
    if (!this.metrics.has(dogId)) {
      this.metrics.set(dogId, new DogMetrics(dogId));
    }

    const dogMetrics = this.metrics.get(dogId);
    dogMetrics.record(outcome);

    // Update global stats
    this.globalStats.totalTasks++;
    if (outcome.success) {
      this.globalStats.successCount++;
    }
    if (outcome.escalated) {
      this.globalStats.escalations++;
    }

    this.emit('recorded', { dogId, outcome, metrics: dogMetrics.toJSON() });

    // Check for performance alerts
    this._checkAlerts(dogId, dogMetrics);
  }

  /**
   * Get metrics for a specific dog
   * @param {string} dogId
   * @returns {DogMetrics|null}
   */
  getMetrics(dogId) {
    return this.metrics.get(dogId) || null;
  }

  /**
   * Get all metrics
   * @returns {Object}
   */
  getAllMetrics() {
    const result = {};
    for (const [dogId, metrics] of this.metrics) {
      result[dogId] = metrics.toJSON();
    }
    return result;
  }

  /**
   * Get top performing dogs
   * @param {number} [topK=5]
   * @returns {Array<{dogId: string, metrics: Object}>}
   */
  getTopDogs(topK = 5) {
    const ranked = [];
    for (const [dogId, metrics] of this.metrics) {
      if (metrics.taskCount >= 5) { // Minimum sample size
        ranked.push({
          dogId,
          score: this._calculatePerformanceScore(metrics),
          metrics: metrics.toJSON(),
        });
      }
    }
    return ranked
      .sort((a, b) => b.score - a.score)
      .slice(0, topK);
  }

  /**
   * Get underperforming dogs
   * @returns {Array<{dogId: string, issue: string, metrics: Object}>}
   */
  getUnderperformers() {
    const issues = [];

    for (const [dogId, metrics] of this.metrics) {
      if (metrics.taskCount < 5) continue;

      // Check for low success rate
      if (metrics.getSuccessRate() < PHI_INV_2) {
        issues.push({
          dogId,
          issue: 'low_success_rate',
          value: metrics.getSuccessRate(),
          threshold: PHI_INV_2,
          metrics: metrics.toJSON(),
        });
      }

      // Check for high escalation rate
      const escalationRate = metrics.escalationCount / metrics.taskCount;
      if (escalationRate > 0.3) {
        issues.push({
          dogId,
          issue: 'high_escalation',
          value: escalationRate,
          threshold: 0.3,
          metrics: metrics.toJSON(),
        });
      }

      // Check for declining trend
      if (metrics.getLearningTrend() === 'declining') {
        issues.push({
          dogId,
          issue: 'declining_performance',
          metrics: metrics.toJSON(),
        });
      }
    }

    return issues;
  }

  /**
   * Get recommended weight adjustments for IntelligentRouter
   * @returns {Object} dogId → { taskType → adjustment }
   */
  getRecommendedWeights() {
    const weights = {};

    for (const [dogId, metrics] of this.metrics) {
      if (metrics.taskCount < 5) continue;

      weights[dogId] = {};

      // For each task type this dog has handled
      for (const [taskType, stats] of metrics.byTaskType) {
        if (stats.count < 3) continue;

        const successRate = stats.success / stats.count;
        const expectedCap = DOG_CAPABILITIES[dogId];
        const expectedAffinity = expectedCap?.taskAffinities?.[taskType] || 0;

        // Calculate adjustment based on actual vs expected performance
        // If performing better than expected, boost weight
        // If performing worse, reduce weight
        const adjustment = (successRate - 0.5) * this.adaptationRate;

        weights[dogId][taskType] = Math.max(-0.2, Math.min(0.2, adjustment));
      }
    }

    return weights;
  }

  /**
   * Get global statistics
   * @returns {Object}
   */
  getGlobalStats() {
    return {
      ...this.globalStats,
      successRate: this.globalStats.totalTasks > 0
        ? this.globalStats.successCount / this.globalStats.totalTasks
        : 0,
      escalationRate: this.globalStats.totalTasks > 0
        ? this.globalStats.escalations / this.globalStats.totalTasks
        : 0,
      uptime: Date.now() - this.globalStats.startedAt,
      activeDogs: [...this.metrics.values()].filter(m => m.taskCount > 0).length,
    };
  }

  /**
   * Get leaderboard formatted string
   * @returns {string}
   */
  formatLeaderboard() {
    const top = this.getTopDogs(5);
    if (top.length === 0) return '*sniff* No performance data yet.';

    const lines = ['── DOG PERFORMANCE LEADERBOARD ──────────────────'];
    for (let i = 0; i < top.length; i++) {
      const { dogId, score, metrics } = top[i];
      const cap = DOG_CAPABILITIES[dogId];
      const trend = metrics.learningTrend === 'improving' ? '↑' :
                    metrics.learningTrend === 'declining' ? '↓' : '→';
      lines.push(
        `   ${i + 1}. ${cap?.emoji || '🐕'} ${cap?.name || dogId}: ` +
        `${Math.round(metrics.successRate * 100)}% success, ` +
        `${metrics.taskCount} tasks ${trend}`
      );
    }

    const global = this.getGlobalStats();
    lines.push(`── Total: ${global.totalTasks} tasks, ${Math.round(global.successRate * 100)}% success ──`);

    return lines.join('\n');
  }

  /**
   * Reset all metrics
   */
  reset() {
    for (const [dogId, metrics] of this.metrics) {
      this.metrics.set(dogId, new DogMetrics(dogId));
    }
    this.globalStats = {
      totalTasks: 0,
      successCount: 0,
      escalations: 0,
      startedAt: Date.now(),
    };
    this.emit('reset');
  }

  /**
   * Export metrics for persistence
   * @returns {Object}
   */
  export() {
    return {
      metrics: this.getAllMetrics(),
      global: this.getGlobalStats(),
      weights: this.getRecommendedWeights(),
      exportedAt: Date.now(),
    };
  }

  /**
   * Import metrics from persistence
   * @param {Object} data - Data from export()
   */
  import(data) {
    if (!data || !data.metrics) {
      this.emit('import_failed', { reason: 'Invalid data format' });
      return;
    }

    let imported = 0;
    for (const [dogId, metricsData] of Object.entries(data.metrics)) {
      try {
        const metrics = DogMetrics.fromJSON(metricsData);
        this.metrics.set(dogId, metrics);
        imported++;
      } catch (e) {
        // Skip invalid entries
      }
    }

    this.emit('imported', { count: imported, exportedAt: data.exportedAt });
  }

  /**
   * Calculate composite performance score
   * @private
   */
  _calculatePerformanceScore(metrics) {
    // Weighted composite:
    // - 40% success rate
    // - 30% recent success rate
    // - 20% (inverse) escalation rate
    // - 10% learning trend bonus

    const successWeight = 0.4;
    const recentWeight = 0.3;
    const escalationWeight = 0.2;
    const trendWeight = 0.1;

    let score = 0;
    score += metrics.getSuccessRate() * successWeight;
    score += metrics.getRecentSuccessRate() * recentWeight;

    const escalationRate = metrics.taskCount > 0
      ? metrics.escalationCount / metrics.taskCount
      : 0;
    score += (1 - escalationRate) * escalationWeight;

    const trend = metrics.getLearningTrend();
    const trendBonus = trend === 'improving' ? 1 : trend === 'declining' ? 0 : 0.5;
    score += trendBonus * trendWeight;

    return Math.min(score, PHI_INV); // Cap at φ⁻¹
  }

  /**
   * Check for performance alerts
   * @private
   */
  _checkAlerts(dogId, metrics) {
    const cap = DOG_CAPABILITIES[dogId];

    // Alert on sudden decline
    if (metrics.recentResults.length >= 5) {
      const recentFails = metrics.recentResults.slice(-5).filter(r => !r.success).length;
      if (recentFails >= 4) {
        this.emit('alert', {
          type: 'sudden_decline',
          dogId,
          dogName: cap?.name,
          message: `${cap?.emoji || '🐕'} ${cap?.name || dogId} failing ${recentFails}/5 recent tasks`,
          severity: 'high',
        });
      }
    }

    // Alert on high latency spike
    if (metrics.recentResults.length >= 3) {
      const recent = metrics.recentResults.slice(-3);
      const avgRecent = recent.reduce((a, r) => a + (r.latency || 0), 0) / 3;
      const overall = metrics.getAvgLatency();
      if (avgRecent > overall * 2 && avgRecent > 1000) {
        this.emit('alert', {
          type: 'latency_spike',
          dogId,
          dogName: cap?.name,
          message: `${cap?.emoji || '🐕'} ${cap?.name || dogId} latency spike: ${Math.round(avgRecent)}ms vs ${Math.round(overall)}ms avg`,
          severity: 'medium',
        });
      }
    }
  }
}

/**
 * Create performance tracker
 * @param {Object} [options]
 * @returns {DogPerformanceTracker}
 */
export function createDogPerformanceTracker(options = {}) {
  return new DogPerformanceTracker(options);
}

// Singleton
let _instance = null;

/**
 * Get singleton tracker
 * @returns {DogPerformanceTracker}
 */
export function getDogPerformanceTracker() {
  if (!_instance) {
    _instance = createDogPerformanceTracker();
  }
  return _instance;
}

export default DogPerformanceTracker;
