/**
 * LV-5: Elastic Weight Consolidation (EWC) Manager
 *
 * Prevents catastrophic forgetting in Q-learning by:
 * 1. Tracking Fisher Information (importance of each Q-value)
 * 2. Adding EWC penalty to Q-updates
 * 3. Consolidating knowledge after task completion
 *
 * Formula:
 *   Q(s,a) <- Q(s,a) + alpha[TD-target] - lambda * F(s,a) * [Q(s,a) - Q_old(s,a)]
 *
 * @module @cynic/node/orchestration/ewc-manager
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

export const EWC_CONFIG = {
  lambda: 0.1,
  minUpdatesForFisher: 5,
  fisherUpdateInterval: 50,
  importanceThreshold: PHI_INV,
  moderateThreshold: PHI_INV_2,
  lowThreshold: PHI_INV_3,
  consolidateAfterEpisodes: 100,
};

export class FisherTracker {
  constructor() {
    this.gradients = new Map();
    this.fisher = new Map();
    this.stats = {
      totalUpdates: 0,
      fisherComputations: 0,
      lastFisherUpdate: null,
    };
  }

  recordGradient(stateKey, action, tdError) {
    const key = `${stateKey}:${action}`;
    const grad = this.gradients.get(key) || { sum: 0, sqSum: 0, count: 0 };

    grad.sum += tdError;
    grad.sqSum += tdError * tdError;
    grad.count += 1;

    this.gradients.set(key, grad);
    this.stats.totalUpdates++;
  }

  computeFisher(stateKey, action) {
    const key = `${stateKey}:${action}`;
    const grad = this.gradients.get(key);

    if (!grad || grad.count < EWC_CONFIG.minUpdatesForFisher) {
      return 0;
    }

    const mean = grad.sum / grad.count;
    const meanSq = grad.sqSum / grad.count;
    const variance = meanSq - (mean * mean);

    const fisher = Math.min(1.0, Math.abs(variance) * PHI_INV);

    this.fisher.set(key, fisher);
    this.stats.fisherComputations++;
    this.stats.lastFisherUpdate = new Date();

    return fisher;
  }

  computeAllFisher() {
    const results = new Map();

    for (const [key, grad] of this.gradients.entries()) {
      if (grad.count >= EWC_CONFIG.minUpdatesForFisher) {
        // Split from last colon (state key may contain colons)
        const lastColonIndex = key.lastIndexOf(':');
        const stateKey = key.substring(0, lastColonIndex);
        const action = key.substring(lastColonIndex + 1);
        const fisher = this.computeFisher(stateKey, action);
        results.set(key, fisher);
      }
    }

    return results;
  }

  getFisher(stateKey, action) {
    const key = `${stateKey}:${action}`;
    return this.fisher.get(key) || 0;
  }

  getFisherStats() {
    const values = Array.from(this.fisher.values());
    if (values.length === 0) {
      return {
        count: 0,
        avg: 0,
        max: 0,
        critical: 0,
        important: 0,
      };
    }

    return {
      count: values.length,
      avg: values.reduce((sum, v) => sum + v, 0) / values.length,
      max: Math.max(...values),
      critical: values.filter(v => v >= EWC_CONFIG.importanceThreshold).length,
      important: values.filter(v => v >= EWC_CONFIG.moderateThreshold).length,
    };
  }

  toJSON() {
    return {
      gradients: Object.fromEntries(this.gradients),
      fisher: Object.fromEntries(this.fisher),
      stats: this.stats,
    };
  }

  static fromJSON(json) {
    const tracker = new FisherTracker();
    if (json.gradients) {
      tracker.gradients = new Map(Object.entries(json.gradients));
    }
    if (json.fisher) {
      tracker.fisher = new Map(Object.entries(json.fisher));
    }
    if (json.stats) {
      tracker.stats = { ...tracker.stats, ...json.stats };
    }
    return tracker;
  }
}

export class EWCManager {
  constructor(options = {}) {
    this.lambda = options.lambda || EWC_CONFIG.lambda;
    this.fisherTracker = new FisherTracker();
    this.consolidatedQTable = null;
    this.currentTask = null;
    this.tasks = new Map();
    this.consolidationHistory = [];
    this.stats = {
      consolidations: 0,
      ewcPenaltiesApplied: 0,
      avgPenalty: 0,
    };
  }

  startTask(taskId, taskType) {
    this.currentTask = {
      taskId,
      taskType,
      startedAt: new Date(),
      episodes: 0,
    };

    if (!this.tasks.has(taskId)) {
      this.tasks.set(taskId, {
        taskId,
        taskType,
        episodes: 0,
        consolidated: false,
        firstSeen: new Date(),
      });
    }
  }

  recordUpdate(stateKey, action, tdError) {
    this.fisherTracker.recordGradient(stateKey, action, tdError);

    if (this.fisherTracker.stats.totalUpdates % EWC_CONFIG.fisherUpdateInterval === 0) {
      this.fisherTracker.computeAllFisher();
    }
  }

  calculateEWCPenalty(stateKey, action, currentQ) {
    if (!this.consolidatedQTable) {
      return 0;
    }

    const fisher = this.fisherTracker.getFisher(stateKey, action);
    if (fisher === 0) {
      return 0;
    }

    const qOld = this._getConsolidatedQ(stateKey, action);
    if (qOld === null) {
      return 0;
    }

    const penalty = this.lambda * fisher * (currentQ - qOld);

    this.stats.ewcPenaltiesApplied++;
    this.stats.avgPenalty = (this.stats.avgPenalty * (this.stats.ewcPenaltiesApplied - 1) + Math.abs(penalty)) 
                           / this.stats.ewcPenaltiesApplied;

    return penalty;
  }

  consolidate(qTable, taskId = null) {
    this.consolidatedQTable = JSON.parse(JSON.stringify(qTable.toJSON()));

    const fisherMap = this.fisherTracker.computeAllFisher();

    const consolidationEvent = {
      consolidationId: `ewc_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
      timestamp: new Date(),
      taskId: taskId || this.currentTask?.taskId,
      taskType: this.currentTask?.taskType,
      fisherStats: this.fisherTracker.getFisherStats(),
      qTableStats: qTable.stats,
    };

    this.consolidationHistory.push(consolidationEvent);
    this.stats.consolidations++;

    if (taskId && this.tasks.has(taskId)) {
      const task = this.tasks.get(taskId);
      task.consolidated = true;
      task.consolidatedAt = new Date();
    }

    // Record to learning_events for G1.2 metric
    (async () => {
      try {
        const { getPool } = await import('@cynic/persistence');
        const pool = getPool();
        await pool.query(`
          INSERT INTO learning_events (loop_type, event_type, pattern_id, metadata)
          VALUES ($1, $2, $3, $4)
        `, [
          'ewc-consolidation',
          'consolidation',
          taskId || 'general',
          JSON.stringify({
            consolidationId: consolidationEvent.consolidationId,
            taskType: this.currentTask?.taskType,
            fisherStats: consolidationEvent.fisherStats,
            qTableStates: qTable.stats?.states || 0
          })
        ]);
      } catch { /* non-blocking DB write */ }
    })();

    return consolidationEvent;
  }

  shouldConsolidate(episodesSinceLastConsolidation) {
    return episodesSinceLastConsolidation >= EWC_CONFIG.consolidateAfterEpisodes;
  }

  _getConsolidatedQ(stateKey, action) {
    if (!this.consolidatedQTable?.table) {
      return null;
    }

    const stateQ = this.consolidatedQTable.table[stateKey];
    return stateQ?.[action] ?? null;
  }

  getStatus() {
    return {
      consolidated: this.consolidatedQTable !== null,
      lambda: this.lambda,
      fisherStats: this.fisherTracker.getFisherStats(),
      consolidations: this.stats.consolidations,
      ewcPenaltiesApplied: this.stats.ewcPenaltiesApplied,
      avgPenalty: this.stats.avgPenalty,
      tasks: {
        total: this.tasks.size,
        consolidated: Array.from(this.tasks.values()).filter(t => t.consolidated).length,
      },
    };
  }

  toJSON() {
    return {
      lambda: this.lambda,
      fisherTracker: this.fisherTracker.toJSON(),
      consolidatedQTable: this.consolidatedQTable,
      tasks: Object.fromEntries(this.tasks),
      consolidationHistory: this.consolidationHistory.slice(-10),
      stats: this.stats,
    };
  }

  static fromJSON(json) {
    const manager = new EWCManager({ lambda: json.lambda });

    if (json.fisherTracker) {
      manager.fisherTracker = FisherTracker.fromJSON(json.fisherTracker);
    }
    if (json.consolidatedQTable) {
      manager.consolidatedQTable = json.consolidatedQTable;
    }
    if (json.tasks) {
      manager.tasks = new Map(Object.entries(json.tasks));
    }
    if (json.consolidationHistory) {
      manager.consolidationHistory = json.consolidationHistory;
    }
    if (json.stats) {
      manager.stats = { ...manager.stats, ...json.stats };
    }

    return manager;
  }
}

export default EWCManager;
