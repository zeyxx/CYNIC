/**
 * DPO Optimizer - Direct Preference Optimization for CYNIC routing
 *
 * "Le chien s'améliore continuellement" - Continuous improvement through preferences
 *
 * Implements Bradley-Terry preference model with φ-aligned learning:
 * - Reads preference pairs from PostgreSQL
 * - Computes DPO gradients
 * - Updates routing weights with EWC regularization
 * - Tracks convergence and calibration
 *
 * Loss function: L = -log(σ(β * (log π(y_w|x) - log π(y_l|x))))
 * Where π is the policy (routing probability), y_w is chosen, y_l is rejected
 *
 * @module @cynic/node/judge/dpo-optimizer
 */

'use strict';

import { getPool } from '@cynic/persistence';

// Simple logger (no external dependency)
const log = {
  debug: (mod, msg, data) => process.env.CYNIC_DEBUG && console.debug(`[${mod}]`, msg, data || ''),
  info: (mod, msg, data) => console.log(`[${mod}]`, msg, data || ''),
  warn: (mod, msg, data) => console.warn(`[${mod}]`, msg, data || ''),
  error: (mod, msg, data) => console.error(`[${mod}]`, msg, data || ''),
};

// ═══════════════════════════════════════════════════════════════════════════
// φ CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;  // φ⁻¹ - max confidence, regularization strength
const PHI_INV_2 = 0.381966011250105; // φ⁻² - convergence threshold
const PHI_INV_3 = 0.236067977499790; // φ⁻³ - learning rate

/**
 * Sigmoid function for Bradley-Terry model
 */
function sigmoid(x) {
  return 1 / (1 + Math.exp(-x));
}

/**
 * DPO Optimizer
 *
 * Runs DPO training on preference pairs to update routing weights.
 */
export class DPOOptimizer {
  constructor(options = {}) {
    this.pool = options.pool || getPool();
    this.serviceId = options.serviceId || 'default';

    // φ-aligned hyperparameters
    this.learningRate = options.learningRate || PHI_INV_3; // 0.236
    this.beta = options.beta || 0.1; // KL divergence penalty
    this.regularization = options.regularization || PHI_INV; // EWC strength
    this.batchSize = options.batchSize || 32;
    this.maxEpochs = options.maxEpochs || 100;
    this.convergenceThreshold = options.convergenceThreshold || PHI_INV_2;
    this.convergenceWindow = options.convergenceWindow || 5;

    // Training state
    this.epoch = 0;
    this.lossHistory = [];
    this.isRunning = false;

    // Stats
    this.stats = {
      runs: 0,
      totalPairsProcessed: 0,
      lastLoss: null,
      bestLoss: null,
      convergenceEpochs: [],
      lastRunAt: null,
      lastRunDuration: null,
    };
  }

  /**
   * Run a single optimization pass
   *
   * @returns {Promise<Object>} Optimization result
   */
  async optimize() {
    if (this.isRunning) {
      throw new Error('Optimizer already running');
    }

    this.isRunning = true;
    const startTime = Date.now();
    const result = {
      epoch: 0,
      pairsProcessed: 0,
      finalLoss: null,
      converged: false,
      weightsUpdated: 0,
    };

    try {
      // Load optimizer state from database
      await this._loadState();

      // Get unprocessed preference pairs
      const pairs = await this._getUnprocessedPairs();

      if (pairs.length === 0) {
        log.debug('DPOOptimizer', 'No unprocessed pairs to train on');
        return result;
      }

      log.info('DPOOptimizer', `Starting optimization with ${pairs.length} pairs`);

      // Get current routing weights
      const weights = await this._loadWeights();

      // Training loop
      let convergedCount = 0;
      let prevLoss = Infinity;

      for (let epoch = 0; epoch < this.maxEpochs; epoch++) {
        this.epoch = epoch;

        // Shuffle pairs for SGD
        this._shuffleArray(pairs);

        // Process in batches
        let epochLoss = 0;
        let batchCount = 0;

        for (let i = 0; i < pairs.length; i += this.batchSize) {
          const batch = pairs.slice(i, i + this.batchSize);
          const batchResult = await this._processBatch(batch, weights);
          epochLoss += batchResult.loss;
          batchCount++;
        }

        // Average loss
        epochLoss /= batchCount;
        this.lossHistory.push(epochLoss);

        // Check for convergence
        const lossImprovement = prevLoss - epochLoss;
        if (Math.abs(lossImprovement) < this.convergenceThreshold) {
          convergedCount++;
          if (convergedCount >= this.convergenceWindow) {
            log.info('DPOOptimizer', `Converged at epoch ${epoch}`, { loss: epochLoss });
            result.converged = true;
            break;
          }
        } else {
          convergedCount = 0;
        }

        prevLoss = epochLoss;
        result.finalLoss = epochLoss;
        result.epoch = epoch;

        // Update best loss
        if (this.stats.bestLoss === null || epochLoss < this.stats.bestLoss) {
          this.stats.bestLoss = epochLoss;
        }
      }

      // Save updated weights to database
      result.weightsUpdated = await this._saveWeights(weights);

      // Mark pairs as processed
      await this._markPairsProcessed(pairs);
      result.pairsProcessed = pairs.length;

      // Save optimizer state
      this.stats.lastLoss = result.finalLoss;
      await this._saveState();

      // Update stats
      this.stats.runs++;
      this.stats.totalPairsProcessed += pairs.length;
      this.stats.lastRunAt = new Date().toISOString();
      this.stats.lastRunDuration = Date.now() - startTime;

      if (result.converged) {
        this.stats.convergenceEpochs.push(result.epoch);
      }

      log.info('DPOOptimizer', `Optimization complete`, {
        epochs: result.epoch,
        pairs: result.pairsProcessed,
        loss: result.finalLoss,
        converged: result.converged,
        duration: Date.now() - startTime,
      });

      return result;

    } catch (err) {
      log.error('DPOOptimizer', 'Optimization failed', { error: err.message });
      throw err;
    } finally {
      this.isRunning = false;
    }
  }

  /**
   * Process a batch of preference pairs
   *
   * @private
   * @param {Object[]} batch - Batch of pairs
   * @param {Map} weights - Current weights
   * @returns {Promise<Object>} Batch result with loss and gradients
   */
  async _processBatch(batch, weights) {
    let totalLoss = 0;
    const gradients = new Map();

    for (const pair of batch) {
      const context = pair.context || {};
      const contextType = pair.context_type || 'general';

      // Get weight for this context
      const key = `${contextType}`;
      let weight = weights.get(key) || { weight: 0.5, fisher: 0 };

      // Compute log probabilities (simplified: weight as log-prob proxy)
      const logProbChosen = Math.log(weight.weight + 0.001);
      const logProbRejected = Math.log(1 - weight.weight + 0.001);

      // DPO loss: -log(sigmoid(beta * (logProbChosen - logProbRejected)))
      const diff = this.beta * (logProbChosen - logProbRejected);
      const prob = sigmoid(diff);
      const loss = -Math.log(prob + 1e-10);

      totalLoss += loss;

      // Compute gradient: d/dw = -beta * (1 - sigmoid(diff)) / w
      const grad = -this.beta * (1 - prob) / (weight.weight + 0.001);

      // Accumulate gradients
      if (!gradients.has(key)) {
        gradients.set(key, { grad: 0, count: 0 });
      }
      const g = gradients.get(key);
      g.grad += grad;
      g.count++;
    }

    // Apply gradients with EWC regularization
    for (const [key, { grad, count }] of gradients) {
      const avgGrad = grad / count;
      const weight = weights.get(key) || { weight: 0.5, fisher: 0 };

      // EWC penalty: regularization * fisher * (weight - base_weight)^2
      const ewcPenalty = this.regularization * weight.fisher * (weight.weight - 0.5);

      // Update weight: w = w - lr * (grad + ewc_penalty)
      const update = this.learningRate * (avgGrad + ewcPenalty);
      weight.weight = Math.max(0.01, Math.min(0.99, weight.weight - update));
      weight.updated = true;

      weights.set(key, weight);
    }

    return {
      loss: totalLoss / batch.length,
      gradientsApplied: gradients.size,
    };
  }

  /**
   * Get unprocessed preference pairs
   *
   * @private
   * @returns {Promise<Object[]>} Pairs
   */
  async _getUnprocessedPairs() {
    const { rows } = await this.pool.query(`
      SELECT id, chosen, rejected, context, context_type, task_type, confidence
      FROM preference_pairs
      WHERE service_id = $1 AND processed = FALSE
      ORDER BY created_at ASC
      LIMIT 1000
    `, [this.serviceId]);

    return rows;
  }

  /**
   * Load current routing weights
   *
   * @private
   * @returns {Promise<Map>} Weights map
   */
  async _loadWeights() {
    const { rows } = await this.pool.query(`
      SELECT dog_name, context_type, weight, fisher_score
      FROM routing_weights
      WHERE service_id = $1
    `, [this.serviceId]);

    const weights = new Map();
    for (const row of rows) {
      const key = `${row.context_type}`;
      weights.set(key, {
        dogName: row.dog_name,
        contextType: row.context_type,
        weight: parseFloat(row.weight),
        fisher: parseFloat(row.fisher_score || 0),
        updated: false,
      });
    }

    return weights;
  }

  /**
   * Save updated weights to database
   *
   * @private
   * @param {Map} weights - Updated weights
   * @returns {Promise<number>} Number of weights updated
   */
  async _saveWeights(weights) {
    let updated = 0;

    for (const [key, data] of weights) {
      if (!data.updated) continue;

      await this.pool.query(`
        UPDATE routing_weights
        SET weight = $1, last_update = NOW(), updated_at = NOW()
        WHERE service_id = $2 AND context_type = $3
      `, [data.weight, this.serviceId, data.contextType]);

      updated++;
    }

    return updated;
  }

  /**
   * Mark preference pairs as processed
   *
   * @private
   * @param {Object[]} pairs - Pairs to mark
   */
  async _markPairsProcessed(pairs) {
    const ids = pairs.map(p => p.id);

    await this.pool.query(`
      UPDATE preference_pairs
      SET processed = TRUE, processed_at = NOW(), updated_at = NOW()
      WHERE id = ANY($1)
    `, [ids]);
  }

  /**
   * Load optimizer state from database
   *
   * @private
   */
  async _loadState() {
    const { rows } = await this.pool.query(`
      SELECT * FROM dpo_optimizer_state WHERE service_id = $1
    `, [this.serviceId]);

    if (rows[0]) {
      const state = rows[0];
      this.learningRate = parseFloat(state.learning_rate) || PHI_INV_3;
      this.beta = parseFloat(state.beta) || 0.1;
      this.regularization = parseFloat(state.regularization) || PHI_INV;
      this.epoch = state.epoch || 0;
      this.stats.lastLoss = state.last_loss ? parseFloat(state.last_loss) : null;
      this.stats.bestLoss = state.best_loss ? parseFloat(state.best_loss) : null;
      this.stats.runs = state.stats?.runs || 0;
    }
  }

  /**
   * Save optimizer state to database
   *
   * @private
   */
  async _saveState() {
    await this.pool.query(`
      UPDATE dpo_optimizer_state
      SET
        epoch = $1,
        last_loss = $2,
        best_loss = $3,
        last_run = NOW(),
        stats = $4,
        updated_at = NOW()
      WHERE service_id = $5
    `, [
      this.epoch,
      this.stats.lastLoss,
      this.stats.bestLoss,
      JSON.stringify({
        runs: this.stats.runs,
        totalPairsProcessed: this.stats.totalPairsProcessed,
        convergenceEpochs: this.stats.convergenceEpochs.slice(-10),
        lossHistory: this.lossHistory.slice(-100),
      }),
      this.serviceId,
    ]);
  }

  /**
   * Shuffle array in place (Fisher-Yates)
   *
   * @private
   * @param {Array} array
   */
  _shuffleArray(array) {
    for (let i = array.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [array[i], array[j]] = [array[j], array[i]];
    }
  }

  /**
   * Get optimizer statistics
   *
   * @returns {Object} Stats
   */
  getStats() {
    return {
      ...this.stats,
      isRunning: this.isRunning,
      epoch: this.epoch,
      lossHistoryLength: this.lossHistory.length,
      hyperparameters: {
        learningRate: this.learningRate,
        beta: this.beta,
        regularization: this.regularization,
        batchSize: this.batchSize,
        maxEpochs: this.maxEpochs,
      },
    };
  }

  /**
   * Get DPO training summary from database
   *
   * @returns {Promise<Object>} Summary
   */
  async getSummary() {
    const { rows } = await this.pool.query(`
      SELECT * FROM get_dpo_stats($1)
    `, [this.serviceId]);

    return rows[0] || {};
  }

  /**
   * Update Fisher scores from SharedMemory patterns
   * This syncs EWC++ importance from pattern usage to routing weights
   *
   * @param {Object} sharedMemory - SharedMemory instance
   * @returns {Promise<number>} Number of weights updated
   */
  async updateFisherFromPatterns(sharedMemory) {
    if (!sharedMemory) {
      log.warn('DPOOptimizer', 'No SharedMemory provided for Fisher sync');
      return 0;
    }

    try {
      // Get EWC stats from SharedMemory
      const ewcStats = sharedMemory.getEWCStats?.();
      if (!ewcStats) {
        return 0;
      }

      // Get patterns with their Fisher importance
      const patterns = sharedMemory.getPatterns?.() || [];
      const fisherByContext = new Map();

      // Aggregate Fisher importance by context type
      for (const pattern of patterns) {
        const contextType = pattern.type || 'general';
        const fisher = pattern.fisherImportance || 0;

        if (!fisherByContext.has(contextType)) {
          fisherByContext.set(contextType, { sum: 0, count: 0, max: 0 });
        }

        const ctx = fisherByContext.get(contextType);
        ctx.sum += fisher;
        ctx.count++;
        ctx.max = Math.max(ctx.max, fisher);
      }

      // Update routing_weights Fisher scores
      let updated = 0;
      for (const [contextType, { sum, count, max }] of fisherByContext) {
        // Use max Fisher as the weight's Fisher score (most important pattern wins)
        const avgFisher = count > 0 ? sum / count : 0;
        const fisherScore = Math.max(avgFisher, max * 0.8); // Blend avg and max

        const result = await this.pool.query(`
          UPDATE routing_weights
          SET fisher_score = $1, fisher_updated = NOW(), updated_at = NOW()
          WHERE service_id = $2 AND context_type = $3
          RETURNING id
        `, [fisherScore, this.serviceId, contextType]);

        if (result.rowCount > 0) {
          updated += result.rowCount;
        }
      }

      log.debug('DPOOptimizer', `Synced Fisher scores from ${patterns.length} patterns`, {
        updated,
        contextsProcessed: fisherByContext.size,
      });

      return updated;

    } catch (err) {
      log.error('DPOOptimizer', 'Failed to sync Fisher scores', { error: err.message });
      return 0;
    }
  }

  /**
   * Update Fisher scores after successful routing decisions
   * Call this when a routing decision leads to positive feedback
   *
   * @param {string} contextType - Context type of the decision
   * @param {number} reward - Reward signal (0-1)
   */
  async boostFisher(contextType, reward) {
    if (!contextType || reward <= 0) return;

    const boostAmount = reward * PHI_INV_3; // φ⁻³ * reward

    await this.pool.query(`
      UPDATE routing_weights
      SET fisher_score = LEAST(1.0, fisher_score + $1),
          fisher_updated = NOW(),
          updated_at = NOW()
      WHERE service_id = $2 AND context_type = $3
    `, [boostAmount, this.serviceId, contextType]);
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create the DPOOptimizer singleton
 *
 * @param {Object} options - Optimizer options
 * @returns {DPOOptimizer} Singleton instance
 */
export function getDPOOptimizer(options = {}) {
  if (!_instance) {
    _instance = new DPOOptimizer(options);
  }
  return _instance;
}

/**
 * Reset the singleton (for testing)
 */
export function resetDPOOptimizer() {
  _instance = null;
}

export default {
  DPOOptimizer,
  getDPOOptimizer,
  resetDPOOptimizer,
};
