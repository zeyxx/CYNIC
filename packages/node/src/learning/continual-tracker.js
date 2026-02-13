/**
 * Continual Learning Tracker - BWT/FWT Metrics
 *
 * Tracks performance across multiple tasks to measure:
 * - BWT (Backward Transfer): How much we forget old tasks
 * - FWT (Forward Transfer): How much old tasks help new tasks
 *
 * Formulas:
 *   BWT = (1/T-1) · Σ(R_T,i - R_i,i)
 *     where R_T,i = performance on task i after learning all T tasks
 *           R_i,i = performance on task i right after learning it
 *
 *   FWT = (1/T-1) · Σ(b_i - b_i*)
 *     where b_i = performance on task i after learning i-1 tasks
 *           b_i* = performance on task i from scratch
 *
 * "Le chien n'oublie pas ses vieux tours" - κυνικός
 *
 * @module @cynic/node/learning/continual-tracker
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';

const log = createLogger('ContinualTracker');

// Acceptance thresholds
export const CONTINUAL_THRESHOLDS = {
  bwtExcellent: -0.05,  // < 5% forgetting
  bwtGood: -0.10,       // < 10% forgetting
  bwtAcceptable: -0.20, // < 20% forgetting
  fwtGood: 0.10,        // 10% improvement from prior learning
};

/**
 * Continual Learning Tracker
 *
 * Tracks multi-task learning performance and computes transfer metrics.
 */
export class ContinualTracker {
  constructor(options = {}) {
    this.serviceId = options.serviceId || 'default';

    // Task performance tracking
    this.tasks = new Map(); // taskId -> { taskType, initialPerf, currentPerf, baselinePerf, episodes, ... }
    this.taskOrder = [];    // Ordered list of task IDs as they were learned

    // Performance snapshots per task
    this.performanceMatrix = new Map(); // "task_i:task_j" -> performance

    // Stats
    this.stats = {
      totalTasks: 0,
      totalEpisodes: 0,
      bwt: null,
      fwt: null,
      lastBwtUpdate: null,
      lastFwtUpdate: null,
    };
  }

  /**
   * Register a new task
   *
   * @param {string} taskId - Unique task identifier
   * @param {string} taskType - Type of task (for grouping)
   * @param {number} [baselinePerf] - Performance from scratch (for FWT)
   */
  registerTask(taskId, taskType, baselinePerf = null) {
    if (this.tasks.has(taskId)) {
      log.debug('Task already registered', { taskId });
      return;
    }

    this.tasks.set(taskId, {
      taskId,
      taskType,
      initialPerf: null,      // R_i,i - performance right after learning task i
      currentPerf: null,      // R_T,i - current performance on task i
      baselinePerf,           // b_i* - performance from scratch
      priorPerf: null,        // b_i - performance with i-1 prior tasks
      episodes: 0,
      firstSeen: Date.now(),
      lastEvaluated: null,
    });

    this.taskOrder.push(taskId);
    this.stats.totalTasks++;

    log.info('Task registered', { taskId, taskType, taskIndex: this.taskOrder.length });
  }

  /**
   * Record performance on a task
   *
   * @param {string} taskId - Task identifier
   * @param {number} performance - Performance metric (0-1, e.g., accuracy)
   * @param {Object} [context] - Additional context
   */
  recordPerformance(taskId, performance, context = {}) {
    const task = this.tasks.get(taskId);

    if (!task) {
      log.warn('Task not registered', { taskId });
      this.registerTask(taskId, context.taskType || 'unknown');
      return this.recordPerformance(taskId, performance, context);
    }

    // First time seeing performance for this task = initial performance
    if (task.initialPerf === null) {
      task.initialPerf = performance;
      log.debug('Initial performance recorded', { taskId, performance });
    }

    // Always update current performance
    task.currentPerf = performance;
    task.lastEvaluated = Date.now();
    task.episodes++;
    this.stats.totalEpisodes++;

    // Store in performance matrix (for cross-task analysis)
    const currentTaskIndex = this.taskOrder.length;
    const taskIndex = this.taskOrder.indexOf(taskId) + 1;
    const matrixKey = `${currentTaskIndex}:${taskIndex}`;
    this.performanceMatrix.set(matrixKey, performance);

    log.debug('Performance recorded', { taskId, performance, matrixKey });
  }

  /**
   * Record "prior performance" - performance on a new task before learning it
   * (used for FWT calculation)
   *
   * @param {string} taskId - Task identifier
   * @param {number} performance - Performance with i-1 prior tasks learned
   */
  recordPriorPerformance(taskId, performance) {
    const task = this.tasks.get(taskId);

    if (!task) {
      log.warn('Task not registered for prior performance', { taskId });
      return;
    }

    task.priorPerf = performance;
    log.debug('Prior performance recorded', { taskId, performance });
  }

  /**
   * Calculate Backward Transfer (BWT)
   *
   * BWT = (1/T-1) · Σ(R_T,i - R_i,i)
   *
   * Measures how much we forget old tasks after learning new ones.
   * - BWT = 0: No forgetting
   * - BWT < 0: Catastrophic forgetting
   * - BWT > 0: Positive transfer (rare)
   *
   * @returns {Object} { bwt, perTask, status }
   */
  calculateBWT() {
    const T = this.taskOrder.length;

    if (T < 2) {
      return {
        bwt: null,
        perTask: [],
        status: 'INSUFFICIENT_TASKS',
        message: 'Need at least 2 tasks to calculate BWT',
      };
    }

    let sum = 0;
    const perTask = [];

    // For each task except the last one (we're still learning the last one)
    for (let i = 0; i < T - 1; i++) {
      const taskId = this.taskOrder[i];
      const task = this.tasks.get(taskId);

      if (!task || task.initialPerf === null || task.currentPerf === null) {
        continue;
      }

      const transfer = task.currentPerf - task.initialPerf;
      sum += transfer;

      perTask.push({
        taskId,
        taskType: task.taskType,
        initialPerf: task.initialPerf,
        currentPerf: task.currentPerf,
        transfer,
      });
    }

    const bwt = perTask.length > 0 ? sum / perTask.length : null;

    this.stats.bwt = bwt;
    this.stats.lastBwtUpdate = Date.now();

    const status = bwt === null ? 'NO_DATA' :
                  bwt >= CONTINUAL_THRESHOLDS.bwtExcellent ? 'EXCELLENT' :
                  bwt >= CONTINUAL_THRESHOLDS.bwtGood ? 'GOOD' :
                  bwt >= CONTINUAL_THRESHOLDS.bwtAcceptable ? 'ACCEPTABLE' : 'POOR';

    log.info('BWT calculated', { bwt, status, tasks: perTask.length });

    return { bwt, perTask, status };
  }

  /**
   * Calculate Forward Transfer (FWT)
   *
   * FWT = (1/T-1) · Σ(b_i - b_i*)
   *
   * Measures how much prior learning helps new tasks.
   * - FWT > 0: Prior learning helps
   * - FWT < 0: Negative transfer
   *
   * @returns {Object} { fwt, perTask, status }
   */
  calculateFWT() {
    const T = this.taskOrder.length;

    if (T < 2) {
      return {
        fwt: null,
        perTask: [],
        status: 'INSUFFICIENT_TASKS',
        message: 'Need at least 2 tasks to calculate FWT',
      };
    }

    let sum = 0;
    const perTask = [];

    // For each task except the first one (first task has no prior learning)
    for (let i = 1; i < T; i++) {
      const taskId = this.taskOrder[i];
      const task = this.tasks.get(taskId);

      if (!task || task.priorPerf === null || task.baselinePerf === null) {
        continue;
      }

      const transfer = task.priorPerf - task.baselinePerf;
      sum += transfer;

      perTask.push({
        taskId,
        taskType: task.taskType,
        baselinePerf: task.baselinePerf,
        priorPerf: task.priorPerf,
        transfer,
      });
    }

    const fwt = perTask.length > 0 ? sum / perTask.length : null;

    this.stats.fwt = fwt;
    this.stats.lastFwtUpdate = Date.now();

    const status = fwt === null ? 'NO_DATA' :
                  fwt >= CONTINUAL_THRESHOLDS.fwtGood ? 'GOOD' :
                  fwt > 0 ? 'POSITIVE' :
                  fwt === 0 ? 'NEUTRAL' : 'NEGATIVE';

    log.info('FWT calculated', { fwt, status, tasks: perTask.length });

    return { fwt, perTask, status };
  }

  /**
   * Get continual learning status
   *
   * @returns {Object} Full status including BWT, FWT, and task details
   */
  getStatus() {
    const bwtResult = this.calculateBWT();
    const fwtResult = this.calculateFWT();

    return {
      totalTasks: this.stats.totalTasks,
      totalEpisodes: this.stats.totalEpisodes,
      bwt: {
        value: bwtResult.bwt,
        status: bwtResult.status,
        perTask: bwtResult.perTask,
      },
      fwt: {
        value: fwtResult.fwt,
        status: fwtResult.status,
        perTask: fwtResult.perTask,
      },
      tasks: Array.from(this.tasks.values()).map(t => ({
        taskId: t.taskId,
        taskType: t.taskType,
        initialPerf: t.initialPerf,
        currentPerf: t.currentPerf,
        episodes: t.episodes,
      })),
    };
  }

  /**
   * Get stats
   */
  getStats() {
    return {
      ...this.stats,
      tasks: this.stats.totalTasks,
      episodes: this.stats.totalEpisodes,
    };
  }

  /**
   * Export state for persistence
   */
  toJSON() {
    return {
      serviceId: this.serviceId,
      tasks: Object.fromEntries(this.tasks),
      taskOrder: this.taskOrder,
      performanceMatrix: Object.fromEntries(this.performanceMatrix),
      stats: this.stats,
    };
  }

  /**
   * Restore from persisted state
   */
  static fromJSON(json) {
    const tracker = new ContinualTracker({ serviceId: json.serviceId });

    if (json.tasks) {
      tracker.tasks = new Map(Object.entries(json.tasks));
    }
    if (json.taskOrder) {
      tracker.taskOrder = json.taskOrder;
    }
    if (json.performanceMatrix) {
      tracker.performanceMatrix = new Map(Object.entries(json.performanceMatrix));
    }
    if (json.stats) {
      tracker.stats = { ...tracker.stats, ...json.stats };
    }

    return tracker;
  }
}

export default ContinualTracker;
