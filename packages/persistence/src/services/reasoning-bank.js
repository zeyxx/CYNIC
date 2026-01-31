/**
 * ReasoningBank Service (M3)
 *
 * Stores and replays successful reasoning trajectories.
 * Inspired by Claude-flow SAFLA's ReasoningBank.
 *
 * Features:
 * - Trajectory storage (State → Action → Outcome → Reward)
 * - Success replay for similar problems
 * - Policy learning from successful trajectories
 * - Bayesian confidence updates
 *
 * @module @cynic/persistence/services/reasoning-bank
 */

'use strict';

import { createLogger } from '@cynic/core';
import {
  TrajectoriesRepository,
  TrajectoryOutcome,
} from '../postgres/repositories/trajectories.js';

const log = createLogger('ReasoningBank');

// φ constants
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;

/**
 * ReasoningBank Service
 *
 * Manages reasoning trajectory storage and replay.
 */
export class ReasoningBank {
  /**
   * Create ReasoningBank
   *
   * @param {Object} options
   * @param {Object} options.pool - PostgreSQL pool
   * @param {Object} [options.vectorStore] - VectorStore for semantic similarity
   */
  constructor(options = {}) {
    if (!options.pool) {
      throw new Error('ReasoningBank requires database pool');
    }

    this._pool = options.pool;
    this._vectorStore = options.vectorStore || null;
    this._trajRepo = new TrajectoriesRepository(options.pool);
    this._initialized = false;

    // Active trajectories (in-progress)
    this._active = new Map();

    // Cache of successful trajectories for quick lookup
    this._successCache = new Map();
    this._cacheMaxSize = 100;

    this._stats = {
      trajectoriesStarted: 0,
      trajectoriesCompleted: 0,
      replaysAttempted: 0,
      replaysSuccessful: 0,
    };
  }

  /**
   * Initialize
   */
  async initialize() {
    if (this._initialized) return;

    try {
      await this._trajRepo.ensureTable();
      this._initialized = true;
      log.info('ReasoningBank initialized');
    } catch (err) {
      log.error('Failed to initialize ReasoningBank', { error: err.message });
      throw err;
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // TRAJECTORY RECORDING
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Start a new trajectory
   *
   * @param {Object} context
   * @param {string} context.userId
   * @param {string} context.sessionId
   * @param {string} context.dogId - Initial dog
   * @param {string} context.taskType - Task type
   * @param {Object} context.initialState - Starting state
   * @returns {Promise<Object>} Trajectory
   */
  async startTrajectory(context) {
    await this.initialize();

    const trajectory = await this._trajRepo.start({
      userId: context.userId,
      sessionId: context.sessionId,
      dogId: context.dogId,
      taskType: context.taskType,
      initialState: context.initialState || {},
      tags: context.tags || [],
    });

    // Track as active
    this._active.set(trajectory.trajectoryId, {
      startTime: Date.now(),
      context,
    });

    this._stats.trajectoriesStarted++;
    log.debug('Trajectory started', { trajectoryId: trajectory.trajectoryId });

    return trajectory;
  }

  /**
   * Record an action in an active trajectory
   *
   * @param {string} trajectoryId
   * @param {Object} action
   */
  async recordAction(trajectoryId, action) {
    await this.initialize();

    if (!this._active.has(trajectoryId)) {
      log.warn('Recording action for unknown trajectory', { trajectoryId });
    }

    return this._trajRepo.recordAction(trajectoryId, action);
  }

  /**
   * Record a dog switch
   *
   * @param {string} trajectoryId
   * @param {string} fromDog
   * @param {string} toDog
   * @param {string} reason
   */
  async recordSwitch(trajectoryId, fromDog, toDog, reason) {
    await this.initialize();
    return this._trajRepo.recordSwitch(trajectoryId, fromDog, toDog, reason);
  }

  /**
   * Complete a trajectory
   *
   * @param {string} trajectoryId
   * @param {Object} result
   * @param {string} result.outcome - 'success' | 'partial' | 'failure' | 'abandoned'
   * @param {Object} [result.details] - Additional details
   */
  async completeTrajectory(trajectoryId, result) {
    await this.initialize();

    const activeInfo = this._active.get(trajectoryId);
    const durationMs = activeInfo ? Date.now() - activeInfo.startTime : 0;

    const trajectory = await this._trajRepo.complete(trajectoryId, {
      ...result,
      durationMs,
      taskType: activeInfo?.context?.taskType,
      dogId: activeInfo?.context?.dogId,
    });

    // Remove from active
    this._active.delete(trajectoryId);
    this._stats.trajectoriesCompleted++;

    // Cache if successful
    if (result.outcome === TrajectoryOutcome.SUCCESS && trajectory.reward > 0.3) {
      this._cacheSuccessful(trajectory);
    }

    // Store in VectorStore for semantic similarity search
    if (this._vectorStore && trajectory) {
      try {
        const text = this._trajectoryToText(trajectory);
        await this._vectorStore.store(trajectory.trajectoryId, text, {
          taskType: trajectory.taskType,
          outcome: trajectory.outcome,
          reward: trajectory.reward,
        });
      } catch (e) {
        // Non-critical
      }
    }

    log.debug('Trajectory completed', {
      trajectoryId,
      outcome: result.outcome,
      reward: trajectory?.reward,
    });

    return trajectory;
  }

  /**
   * Convert trajectory to searchable text
   */
  _trajectoryToText(trajectory) {
    const parts = [
      trajectory.taskType || '',
      trajectory.dogId || '',
      trajectory.outcome || '',
    ];

    // Add action tools
    if (Array.isArray(trajectory.actionSequence)) {
      const tools = trajectory.actionSequence
        .filter(a => a.tool)
        .map(a => a.tool);
      parts.push(...[...new Set(tools)]);
    }

    return parts.join(' ');
  }

  /**
   * Cache a successful trajectory
   */
  _cacheSuccessful(trajectory) {
    const key = `${trajectory.taskType}:${trajectory.dogId}`;

    if (!this._successCache.has(key)) {
      this._successCache.set(key, []);
    }

    const cache = this._successCache.get(key);
    cache.push(trajectory);

    // Keep only top performers
    cache.sort((a, b) => b.reward - a.reward);
    if (cache.length > 10) {
      cache.pop();
    }

    // Limit total cache size
    if (this._successCache.size > this._cacheMaxSize) {
      const firstKey = this._successCache.keys().next().value;
      this._successCache.delete(firstKey);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // SUCCESS REPLAY
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Find similar successful trajectories for replay
   *
   * @param {Object} context
   * @param {string} context.taskType
   * @param {string} [context.dogId]
   * @param {Object} [context.initialState]
   * @returns {Promise<Object[]>} Matching trajectories
   */
  async findSimilar(context) {
    await this.initialize();

    const { taskType, dogId } = context;
    const results = [];

    // 1. Check cache first
    const cacheKey = `${taskType}:${dogId}`;
    if (this._successCache.has(cacheKey)) {
      results.push(...this._successCache.get(cacheKey));
    }

    // 2. Query database for successful trajectories
    const dbResults = await this._trajRepo.findSuccessful(taskType, {
      dogId,
      limit: 5,
      minReward: PHI_INV_2, // 38.2% min reward
    });
    results.push(...dbResults);

    // 3. If VectorStore available, do semantic search
    if (this._vectorStore && results.length < 3) {
      const query = `${taskType} ${dogId || ''} success`;
      try {
        const semantic = await this._vectorStore.search(query, 5, {
          minScore: PHI_INV_2,
        });

        for (const match of semantic) {
          if (!results.find(r => r.trajectoryId === match.id)) {
            const traj = await this._trajRepo.findById(match.id);
            if (traj) results.push(traj);
          }
        }
      } catch (e) {
        // Non-critical
      }
    }

    // Dedupe and sort by reward
    const unique = [];
    const seen = new Set();
    for (const t of results) {
      if (!seen.has(t.trajectoryId)) {
        seen.add(t.trajectoryId);
        unique.push(t);
      }
    }

    return unique
      .sort((a, b) => b.reward - a.reward)
      .slice(0, 5);
  }

  /**
   * Get replay suggestions for a task
   *
   * @param {Object} context
   * @returns {Promise<Object>} Replay suggestions
   */
  async getReplaySuggestions(context) {
    const similar = await this.findSimilar(context);

    if (similar.length === 0) {
      return {
        hasReplay: false,
        confidence: 0,
        message: 'No similar successful trajectories found',
      };
    }

    // Get best trajectory
    const best = similar[0];

    // Calculate confidence based on replay success history
    let confidence = best.reward;
    if (best.replayCount > 0) {
      confidence = best.successAfterReplay
        ? Math.min(1, confidence + 0.1)
        : Math.max(0, confidence - 0.1);
    }

    return {
      hasReplay: true,
      confidence: Math.min(PHI_INV, confidence),
      trajectory: best,
      alternatives: similar.slice(1),
      suggestedDog: best.dogId,
      suggestedActions: this._extractActionPlan(best),
      message: `Found ${similar.length} similar successful trajectories`,
    };
  }

  /**
   * Extract action plan from trajectory
   */
  _extractActionPlan(trajectory) {
    if (!Array.isArray(trajectory.actionSequence)) return [];

    return trajectory.actionSequence
      .filter(a => a.tool && a.success !== false)
      .slice(0, 10)
      .map(a => ({
        tool: a.tool,
        pattern: this._summarizeAction(a),
      }));
  }

  /**
   * Summarize an action for display
   */
  _summarizeAction(action) {
    if (!action.input) return action.tool;

    if (action.tool === 'Read' && action.input.file_path) {
      return `Read ${action.input.file_path}`;
    }
    if (action.tool === 'Edit' && action.input.file_path) {
      return `Edit ${action.input.file_path}`;
    }
    if (action.tool === 'Bash' && action.input.command) {
      return `Run: ${action.input.command.slice(0, 50)}`;
    }
    if (action.tool === 'Glob' && action.input.pattern) {
      return `Find: ${action.input.pattern}`;
    }

    return action.tool;
  }

  /**
   * Record that a replay was attempted
   *
   * @param {string} trajectoryId - Trajectory being replayed
   * @param {boolean} success - Whether replay succeeded
   */
  async recordReplayOutcome(trajectoryId, success) {
    await this.initialize();

    this._stats.replaysAttempted++;
    if (success) {
      this._stats.replaysSuccessful++;
    }

    return this._trajRepo.recordReplay(trajectoryId, success);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // POLICY LEARNING
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get recommended dog for a task type based on trajectory history
   *
   * @param {string} taskType
   * @returns {Promise<Object>} Recommendation
   */
  async getRecommendedDog(taskType) {
    await this.initialize();

    // Get stats per dog for this task type
    const { rows } = await this._pool.query(`
      SELECT
        dog_id,
        COUNT(*) as attempts,
        COUNT(*) FILTER (WHERE outcome = 'success') as successes,
        AVG(reward) as avg_reward
      FROM trajectories
      WHERE task_type = $1
        AND dog_id IS NOT NULL
      GROUP BY dog_id
      ORDER BY avg_reward DESC
      LIMIT 5
    `, [taskType]);

    if (rows.length === 0) {
      return {
        recommended: null,
        confidence: 0,
        message: 'No history for this task type',
      };
    }

    const best = rows[0];
    const successRate = best.attempts > 0 ? best.successes / best.attempts : 0;

    return {
      recommended: best.dog_id,
      confidence: Math.min(PHI_INV, parseFloat(best.avg_reward) || 0),
      successRate,
      attempts: best.attempts,
      alternatives: rows.slice(1).map(r => ({
        dogId: r.dog_id,
        avgReward: parseFloat(r.avg_reward),
        successRate: r.attempts > 0 ? r.successes / r.attempts : 0,
      })),
    };
  }

  /**
   * Learn from completed trajectories (batch processing)
   *
   * @returns {Promise<Object>} Learning results
   */
  async runLearningCycle() {
    await this.initialize();

    const result = {
      processed: 0,
      cached: 0,
      timestamp: Date.now(),
    };

    // Get recent successful trajectories not yet cached
    const topPerformers = await this._trajRepo.getTopPerformers({ limit: 50 });

    for (const trajectory of topPerformers) {
      this._cacheSuccessful(trajectory);
      result.cached++;
      result.processed++;
    }

    log.info('Learning cycle complete', result);
    return result;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // STATISTICS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get statistics
   */
  async getStats(options = {}) {
    await this.initialize();

    const repoStats = await this._trajRepo.getStats(options);
    return {
      ...repoStats,
      processing: { ...this._stats },
      activeTrajectories: this._active.size,
      cacheSize: this._successCache.size,
      replaySuccessRate: this._stats.replaysAttempted > 0
        ? this._stats.replaysSuccessful / this._stats.replaysAttempted
        : 0,
    };
  }

  /**
   * Get active trajectory IDs
   */
  getActiveTrajectories() {
    return [...this._active.keys()];
  }
}

// Re-export outcome enum
export { TrajectoryOutcome };

/**
 * Create ReasoningBank instance
 */
export function createReasoningBank(options) {
  return new ReasoningBank(options);
}

export default ReasoningBank;
