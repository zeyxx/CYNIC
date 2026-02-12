/**
 * C6.5: CYNIC × LEARN — Catastrophic Forgetting Detection
 * 
 * Tracks BWT/FWT metrics to detect if continual learning destroys past knowledge.
 */

import { EventEmitter } from 'events';

const PHI_INV = 0.618;
const FORGETTING_THRESHOLD = -0.1;
const REEVALUATION_INTERVAL = 100;
const MIN_TASK_SAMPLES = 5;

export class CatastrophicForgettingTracker extends EventEmitter {
  constructor({ db, logger }) {
    super();
    this.db = db;
    this.logger = logger || console;
    this.taskBaselines = new Map();
    this.judgmentCount = 0;
  }

  async recordJudgment(taskType, accuracy, judgmentId) {
    this.judgmentCount++;
    
    if (!this.taskBaselines.has(taskType)) {
      this.taskBaselines.set(taskType, {
        baseline_accuracy: accuracy,
        sample_count: 1,
        last_accuracy: accuracy,
        first_seen: Date.now(),
        judgments: [{ id: judgmentId, accuracy, timestamp: Date.now() }]
      });
      await this._persistBaseline(taskType);
      return;
    }
    
    const baseline = this.taskBaselines.get(taskType);
    baseline.sample_count++;
    baseline.last_accuracy = accuracy;
    baseline.judgments.push({ id: judgmentId, accuracy, timestamp: Date.now() });
    
    if (baseline.judgments.length > 100) {
      baseline.judgments = baseline.judgments.slice(-100);
    }
    
    await this._persistJudgment(taskType, accuracy, judgmentId);
    
    if (this.judgmentCount % REEVALUATION_INTERVAL === 0) {
      await this._triggerReevaluation();
    }
  }

  async calculateBWT(taskType) {
    const baseline = this.taskBaselines.get(taskType);
    if (!baseline || baseline.sample_count < MIN_TASK_SAMPLES) {
      return null;
    }
    
    const recentJudgments = baseline.judgments.slice(-10);
    const recentAccuracy = recentJudgments.reduce((sum, j) => sum + j.accuracy, 0) / recentJudgments.length;
    const bwt = recentAccuracy - baseline.baseline_accuracy;
    
    return {
      taskType,
      bwt,
      baseline: baseline.baseline_accuracy,
      current: recentAccuracy,
      sample_count: baseline.sample_count,
      is_catastrophic: bwt < FORGETTING_THRESHOLD
    };
  }

  async calculateOverallBWT() {
    const bwtScores = [];
    
    for (const taskType of this.taskBaselines.keys()) {
      const bwt = await this.calculateBWT(taskType);
      if (bwt !== null) {
        bwtScores.push(bwt);
      }
    }
    
    if (bwtScores.length === 0) {
      return null;
    }
    
    const avgBWT = bwtScores.reduce((sum, b) => sum + b.bwt, 0) / bwtScores.length;
    const catastrophicTasks = bwtScores.filter(b => b.is_catastrophic);
    
    return {
      average_bwt: avgBWT,
      task_count: bwtScores.length,
      catastrophic_count: catastrophicTasks.length,
      catastrophic_tasks: catastrophicTasks.map(t => t.taskType),
      confidence: Math.min(PHI_INV, 0.5 + (bwtScores.length * 0.05))
    };
  }

  async calculateFWT(taskType, initialAccuracy, randomBaseline = 0.5) {
    const fwt = initialAccuracy - randomBaseline;
    return {
      taskType,
      fwt,
      initial_accuracy: initialAccuracy,
      random_baseline: randomBaseline,
      is_positive_transfer: fwt > 0
    };
  }

  async _triggerReevaluation() {
    this.logger.info('[Forgetting] Re-evaluating...');
    const bwt = await this.calculateOverallBWT();
    
    if (bwt && bwt.catastrophic_count > 0) {
      this.logger.warn(`[Forgetting] GROWL! ${bwt.catastrophic_count} tasks forgotten`);
      this.emit('catastrophic-forgetting', { type: 'catastrophic_forgetting', bwt, timestamp: Date.now() });
      await this._persistAlert(bwt);
    } else if (bwt) {
      this.logger.info(`[Forgetting] BWT: ${bwt.average_bwt.toFixed(3)}`);
    }
    
    if (bwt) {
      await this._persistBWTMetrics(bwt);
    }
  }

  async getReport() {
    const bwt = await this.calculateOverallBWT();
    const taskReports = [];
    
    for (const taskType of this.taskBaselines.keys()) {
      const taskBWT = await this.calculateBWT(taskType);
      if (taskBWT) {
        taskReports.push(taskBWT);
      }
    }
    
    return {
      overall: bwt,
      tasks: taskReports.sort((a, b) => a.bwt - b.bwt),
      timestamp: Date.now(),
      confidence: bwt ? bwt.confidence : 0
    };
  }

  async resetBaseline(taskType) {
    const baseline = this.taskBaselines.get(taskType);
    if (!baseline) return;
    
    const recentJudgments = baseline.judgments.slice(-10);
    const newBaseline = recentJudgments.reduce((sum, j) => sum + j.accuracy, 0) / recentJudgments.length;
    
    baseline.baseline_accuracy = newBaseline;
    baseline.sample_count = recentJudgments.length;
    baseline.judgments = recentJudgments;
    baseline.first_seen = Date.now();
    
    await this._persistBaseline(taskType);
    this.logger.info(`[Forgetting] Reset baseline for ${taskType}`);
  }

  async initialize() {
    try {
      const result = await this.db.query(`
        SELECT task_type, baseline_accuracy, sample_count, created_at
        FROM forgetting_baselines
        ORDER BY created_at DESC
      `);
      
      for (const row of result.rows) {
        this.taskBaselines.set(row.task_type, {
          baseline_accuracy: parseFloat(row.baseline_accuracy),
          sample_count: row.sample_count,
          last_accuracy: parseFloat(row.baseline_accuracy),
          first_seen: row.created_at,
          judgments: []
        });
      }
      
      this.logger.info(`[Forgetting] Loaded ${result.rows.length} baselines`);
    } catch (err) {
      this.logger.error('[Forgetting] Load failed:', err.message);
    }
  }

  async _persistBaseline(taskType) {
    const baseline = this.taskBaselines.get(taskType);
    try {
      await this.db.query(`
        INSERT INTO forgetting_baselines (task_type, baseline_accuracy, sample_count)
        VALUES ($1, $2, $3)
        ON CONFLICT (task_type) DO UPDATE SET baseline_accuracy = $2, sample_count = $3, updated_at = NOW()
      `, [taskType, baseline.baseline_accuracy, baseline.sample_count]);
    } catch (err) {
      this.logger.error('[Forgetting] Baseline persist failed:', err.message);
    }
  }

  async _persistJudgment(taskType, accuracy, judgmentId) {
    try {
      await this.db.query(`INSERT INTO forgetting_judgments (task_type, judgment_id, accuracy) VALUES ($1, $2, $3)`, 
        [taskType, judgmentId, accuracy]);
    } catch (err) {
      this.logger.error('[Forgetting] Judgment persist failed:', err.message);
    }
  }

  async _persistBWTMetrics(bwt) {
    try {
      await this.db.query(`
        INSERT INTO forgetting_metrics (average_bwt, task_count, catastrophic_count, catastrophic_tasks, confidence)
        VALUES ($1, $2, $3, $4, $5)
      `, [bwt.average_bwt, bwt.task_count, bwt.catastrophic_count, JSON.stringify(bwt.catastrophic_tasks), bwt.confidence]);
    } catch (err) {
      this.logger.error('[Forgetting] Metrics persist failed:', err.message);
    }
  }

  async _persistAlert(bwt) {
    try {
      await this.db.query(`
        INSERT INTO forgetting_alerts (alert_type, catastrophic_count, catastrophic_tasks, average_bwt)
        VALUES ($1, $2, $3, $4)
      `, ['catastrophic_forgetting', bwt.catastrophic_count, JSON.stringify(bwt.catastrophic_tasks), bwt.average_bwt]);
    } catch (err) {
      this.logger.error('[Forgetting] Alert persist failed:', err.message);
    }
  }
}

export { FORGETTING_THRESHOLD, PHI_INV };
