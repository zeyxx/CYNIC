/**
 * Learning Manager - Unified Learning System
 *
 * Integrates all learning components:
 * - LearningService (core RLHF feedback loop)
 * - E-Score history tracking
 * - Pattern evolution and fusion
 * - User learning profiles
 *
 * "φ learns from every correction" - κυνικός
 *
 * @module @cynic/node/judge/learning-manager
 */

'use strict';

import crypto from 'crypto';
import { EventEmitter } from 'events';
import { PHI_INV_2, PHI_INV_3 } from '@cynic/core';
import { LearningService } from './learning-service.js';

/**
 * Generate short ID for learning cycles
 */
function generateCycleId() {
  return 'lrn_' + crypto.randomBytes(8).toString('hex');
}

/**
 * Learning Manager - Unified learning system
 *
 * Combines:
 * - LearningService for RLHF weight/threshold adjustments
 * - E-Score history for user engagement trends
 * - Pattern evolution for pattern fusion
 * - User learning profiles for personalized learning
 */
export class LearningManager extends EventEmitter {
  /**
   * @param {Object} options - Manager options
   * @param {Object} [options.persistence] - PersistenceManager with all repositories
   * @param {number} [options.learningRate] - Learning rate (default: φ⁻³ = 23.6%)
   * @param {boolean} [options.autoRecord] - Auto-record E-Score changes (default: true)
   */
  constructor(options = {}) {
    super();

    this.persistence = options.persistence || null;

    // Core RLHF service
    this.learningService = new LearningService({
      persistence: options.persistence,
      learningRate: options.learningRate || PHI_INV_3,
    });

    // Configuration
    this.autoRecord = options.autoRecord !== false;

    // Statistics
    this._stats = {
      cyclesRun: 0,
      totalFeedback: 0,
      escoreSnapshots: 0,
      patternsEvolved: 0,
      profilesUpdated: 0,
    };

    this._initialized = false;
  }

  /**
   * Initialize the learning manager
   */
  async init() {
    if (this._initialized) return;

    await this.learningService.init();

    this._initialized = true;
    this.emit('initialized');
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // UNIFIED LEARNING CYCLE
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Run a complete learning cycle
   *
   * 1. Pull unapplied feedback
   * 2. Execute RLHF learning
   * 3. Update pattern evolution
   * 4. Record learning cycle
   *
   * @returns {Object} Cycle results
   */
  async runLearningCycle() {
    await this.init();

    const startTime = Date.now();
    const cycleId = generateCycleId();

    const results = {
      cycleId,
      feedback: { pulled: 0, processed: 0 },
      patterns: { updated: 0, merged: 0 },
      weights: { adjusted: 0 },
      thresholds: { adjusted: 0 },
    };

    try {
      // 1. Run core RLHF learning
      const learnResult = await this.learningService.runLearningCycle();

      results.feedback.pulled = learnResult.pull?.pulled || 0;
      results.feedback.processed = learnResult.learn?.feedbackProcessed || 0;
      this._stats.totalFeedback += results.feedback.processed;

      // 2. Update pattern evolution (if persistence available)
      if (this.persistence?.patternEvolution) {
        const patterns = this.learningService.getPatterns();

        // Update patterns by item type
        for (const [itemType, pattern] of Object.entries(patterns.byItemType)) {
          await this.persistence.patternEvolution.upsert({
            type: 'item_type',
            key: itemType,
            occurrenceCount: pattern.feedbackCount,
            confidence: Math.min(0.618, pattern.feedbackCount / 20),
            strength: 50 + (pattern.avgDelta || 0),
            trendDirection: pattern.avgDelta > 1 ? 'up' : pattern.avgDelta < -1 ? 'down' : 'stable',
          });
          results.patterns.updated++;
        }

        // Update patterns by dimension
        for (const [dimension, pattern] of Object.entries(patterns.byDimension)) {
          const modifier = this.learningService.getWeightModifier(dimension);
          await this.persistence.patternEvolution.upsert({
            type: 'dimension',
            key: dimension,
            occurrenceCount: pattern.feedbackCount,
            confidence: Math.min(0.618, pattern.feedbackCount / 30),
            weightModifier: modifier,
            trendDirection: pattern.avgError > 5 ? 'down' : pattern.avgError < -5 ? 'up' : 'stable',
          });
          results.patterns.updated++;
        }

        this._stats.patternsEvolved += results.patterns.updated;
      }

      // 3. Count weight/threshold adjustments
      const state = this.learningService.getState();
      results.weights.adjusted = Object.values(state.weightModifiers)
        .filter((v) => Math.abs(v - 1.0) > 0.01).length;
      results.thresholds.adjusted = Object.values(state.thresholdAdjustments)
        .reduce((sum, dims) => sum + Object.keys(dims).length, 0);

      // 4. Record learning cycle (if persistence available)
      if (this.persistence?.learningCycles) {
        const avgWeightDelta = results.weights.adjusted > 0
          ? Object.values(state.weightModifiers)
              .map((v) => Math.abs(v - 1.0))
              .reduce((a, b) => a + b, 0) / results.weights.adjusted
          : 0;

        await this.persistence.learningCycles.record({
          cycleId,
          feedbackProcessed: results.feedback.processed,
          patternsUpdated: results.patterns.updated,
          patternsMerged: results.patterns.merged,
          weightsAdjusted: results.weights.adjusted,
          thresholdsAdjusted: results.thresholds.adjusted,
          avgWeightDelta,
          durationMs: Date.now() - startTime,
        });
      }

      this._stats.cyclesRun++;

      results.success = true;
      results.durationMs = Date.now() - startTime;

      this.emit('cycle:complete', results);
    } catch (error) {
      results.success = false;
      results.error = error.message;
      this.emit('cycle:error', error);
    }

    return results;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // E-SCORE TRACKING
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Record E-Score snapshot for a user
   *
   * @param {string} userId - User ID
   * @param {number} eScore - Current E-Score
   * @param {Object} breakdown - Dimension breakdown
   * @param {string} [trigger='manual'] - What triggered the snapshot
   */
  async recordEScore(userId, eScore, breakdown, trigger = 'manual') {
    if (!this.persistence?.escoreHistory) {
      return null;
    }

    const snapshot = await this.persistence.escoreHistory.recordSnapshot(
      userId,
      eScore,
      breakdown,
      trigger
    );

    this._stats.escoreSnapshots++;
    this.emit('escore:recorded', { userId, eScore, trigger });

    return snapshot;
  }

  /**
   * Get E-Score trend for a user
   *
   * @param {string} userId - User ID
   * @param {number} [days=7] - Number of days to analyze
   */
  async getEScoreTrend(userId, days = 7) {
    if (!this.persistence?.escoreHistory) {
      return { hasTrend: false, reason: 'No E-Score history repository' };
    }

    const trend = await this.persistence.escoreHistory.getTrend(userId, days);
    const dimensionTrends = await this.persistence.escoreHistory.getDimensionTrends(userId, days);

    return {
      ...trend,
      dimensions: dimensionTrends.dimensions || {},
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // USER LEARNING PROFILES
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Update user learning profile based on feedback
   *
   * @param {string} userId - User ID
   * @param {Object} feedback - Feedback data
   */
  async updateUserProfile(userId, feedback) {
    if (!this.persistence?.userLearningProfiles) {
      return null;
    }

    // Ensure profile exists
    const profile = await this.persistence.userLearningProfiles.getOrCreate(userId);

    // Record feedback
    const wasCorrect = feedback.outcome === 'correct';
    await this.persistence.userLearningProfiles.recordFeedback(userId, wasCorrect);

    // Update judgment patterns
    if (feedback.itemType) {
      await this.persistence.userLearningProfiles.updateJudgmentPatterns(userId, feedback.itemType);
    }

    // Record activity
    await this.persistence.userLearningProfiles.recordActivity(userId);

    this._stats.profilesUpdated++;
    this.emit('profile:updated', { userId });

    return profile;
  }

  /**
   * Get user learning profile summary
   *
   * @param {string} userId - User ID
   */
  async getUserProfile(userId) {
    if (!this.persistence?.userLearningProfiles) {
      return null;
    }

    return await this.persistence.userLearningProfiles.getSummary(userId);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PATTERN EVOLUTION
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get top patterns from evolution
   *
   * @param {number} [limit=20] - Max patterns to return
   */
  async getTopPatterns(limit = 20) {
    if (!this.persistence?.patternEvolution) {
      // Fall back to in-memory patterns
      return this.learningService.getPatterns();
    }

    const patterns = await this.persistence.patternEvolution.getTopPatterns(limit);
    const trending = await this.persistence.patternEvolution.getTrending('up', 5);

    return {
      top: patterns,
      trending,
      inMemory: this.learningService.getPatterns(),
    };
  }

  /**
   * Find patterns needing review
   */
  async getPatternsNeedingReview() {
    if (!this.persistence?.patternEvolution) {
      return [];
    }

    return await this.persistence.patternEvolution.getNeedingReview(10);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // DELEGATION TO LEARNING SERVICE
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Process feedback (delegates to LearningService)
   */
  processFeedback(feedback) {
    return this.learningService.processFeedback(feedback);
  }

  /**
   * Get weight modifier (delegates to LearningService)
   */
  getWeightModifier(dimension) {
    return this.learningService.getWeightModifier(dimension);
  }

  /**
   * Get all weight modifiers (delegates to LearningService)
   */
  getWeightModifiers() {
    return this.learningService.getAllWeightModifiers();
  }

  /**
   * Get threshold adjustment (delegates to LearningService)
   */
  getThresholdAdjustment(itemType, dimension) {
    return this.learningService.getThresholdAdjustment(itemType, dimension);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // STATISTICS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get combined statistics
   */
  async getStats() {
    const serviceStats = this.learningService.getStats();

    // Get persistence stats if available
    let persistenceStats = {};
    if (this.persistence) {
      if (this.persistence.escoreHistory) {
        persistenceStats.escoreHistory = await this.persistence.escoreHistory.getStats();
      }
      if (this.persistence.learningCycles) {
        persistenceStats.learningCycles = await this.persistence.learningCycles.getStats();
      }
      if (this.persistence.patternEvolution) {
        persistenceStats.patternEvolution = await this.persistence.patternEvolution.getStats();
      }
      if (this.persistence.userLearningProfiles) {
        persistenceStats.userProfiles = await this.persistence.userLearningProfiles.getStats();
      }
    }

    return {
      manager: this._stats,
      service: serviceStats,
      persistence: persistenceStats,
      initialized: this._initialized,
    };
  }

  /**
   * Get learning state summary
   */
  getState() {
    return {
      ...this.learningService.getState(),
      managerStats: this._stats,
    };
  }
}

export default LearningManager;
