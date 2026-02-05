/**
 * DPO Processor - Converts feedback to preference pairs for DPO training
 *
 * "Le chien apprend de ses erreurs" - Transform user corrections into training signal
 *
 * Part of the DPO learning pipeline:
 * Feedback → DPOProcessor → preference_pairs → DPOOptimizer → routing_weights
 *
 * @module @cynic/node/judge/dpo-processor
 */

'use strict';

import { getPool, FeedbackRepository } from '@cynic/persistence';

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

const PHI_INV = 0.618033988749895;  // φ⁻¹ - default confidence
const PHI_INV_2 = 0.381966011250105; // φ⁻² - minimum confidence
const PHI_INV_3 = 0.236067977499790; // φ⁻³ - learning threshold

/**
 * DPO Processor
 *
 * Processes raw feedback into DPO preference pairs.
 * Creates chosen/rejected pairs by comparing correct vs incorrect outcomes.
 */
export class DPOProcessor {
  constructor(options = {}) {
    this.pool = options.pool || getPool();
    this.feedbackRepo = options.feedbackRepo || new FeedbackRepository(this.pool);
    this.serviceId = options.serviceId || 'default';

    // Processing config
    this.batchSize = options.batchSize || 100;
    this.minConfidence = options.minConfidence || PHI_INV_2;

    // Stats
    this.stats = {
      processed: 0,
      pairsCreated: 0,
      errors: 0,
      lastProcessedAt: null,
    };
  }

  /**
   * Process unapplied feedback into preference pairs
   *
   * @returns {Promise<Object>} Processing result
   */
  async process() {
    const startTime = Date.now();
    const result = {
      feedbackProcessed: 0,
      pairsCreated: 0,
      errors: [],
    };

    try {
      // Get unapplied feedback with judgment data
      const feedback = await this.feedbackRepo.findUnapplied(this.batchSize);

      if (feedback.length === 0) {
        log.debug('DPOProcessor', 'No unapplied feedback to process');
        return result;
      }

      log.info('DPOProcessor', `Processing ${feedback.length} feedback items`);

      // Group feedback by context type for pairing
      const grouped = this._groupByContext(feedback);

      // Create preference pairs from groups
      for (const [contextKey, items] of Object.entries(grouped)) {
        try {
          const pairs = await this._createPairsFromGroup(contextKey, items);
          result.pairsCreated += pairs.length;

          // Mark feedback as applied
          for (const item of items) {
            await this.feedbackRepo.markApplied(item.id);
            result.feedbackProcessed++;
          }
        } catch (err) {
          log.error('DPOProcessor', `Error processing group ${contextKey}`, { error: err.message });
          result.errors.push({ context: contextKey, error: err.message });
          this.stats.errors++;
        }
      }

      // Update stats
      this.stats.processed += result.feedbackProcessed;
      this.stats.pairsCreated += result.pairsCreated;
      this.stats.lastProcessedAt = new Date().toISOString();

      log.info('DPOProcessor', `Processed ${result.feedbackProcessed} feedback → ${result.pairsCreated} pairs`, {
        duration: Date.now() - startTime,
      });

      return result;

    } catch (err) {
      log.error('DPOProcessor', 'Processing failed', { error: err.message });
      this.stats.errors++;
      throw err;
    }
  }

  /**
   * Group feedback by context for pairing
   *
   * @private
   * @param {Object[]} feedback - Feedback items
   * @returns {Object} Grouped feedback by context key
   */
  _groupByContext(feedback) {
    const groups = {};

    for (const item of feedback) {
      // Create context key from item_type and source_type
      const contextKey = `${item.item_type || 'unknown'}:${item.source_type || 'manual'}`;

      if (!groups[contextKey]) {
        groups[contextKey] = [];
      }
      groups[contextKey].push(item);
    }

    return groups;
  }

  /**
   * Create preference pairs from a group of feedback
   *
   * @private
   * @param {string} contextKey - Context key
   * @param {Object[]} items - Feedback items in group
   * @returns {Promise<Object[]>} Created pairs
   */
  async _createPairsFromGroup(contextKey, items) {
    const pairs = [];
    const [contextType, sourceType] = contextKey.split(':');

    // Separate correct and incorrect feedback
    const correct = items.filter(i => i.outcome === 'correct');
    const incorrect = items.filter(i => i.outcome === 'incorrect');
    const partial = items.filter(i => i.outcome === 'partial');

    // Create pairs: correct > incorrect
    for (const c of correct) {
      for (const i of incorrect) {
        const pair = await this._createPair(c, i, contextType, sourceType);
        if (pair) pairs.push(pair);
      }
    }

    // Create pairs: correct > partial
    for (const c of correct) {
      for (const p of partial) {
        const pair = await this._createPair(c, p, contextType, sourceType, 0.8);
        if (pair) pairs.push(pair);
      }
    }

    // Create pairs: partial > incorrect (lower confidence)
    for (const p of partial) {
      for (const i of incorrect) {
        const pair = await this._createPair(p, i, contextType, sourceType, 0.5);
        if (pair) pairs.push(pair);
      }
    }

    return pairs;
  }

  /**
   * Create a single preference pair
   *
   * @private
   * @param {Object} chosen - Chosen (better) item
   * @param {Object} rejected - Rejected (worse) item
   * @param {string} contextType - Context type
   * @param {string} taskType - Task type
   * @param {number} [confidenceMultiplier=1.0] - Confidence multiplier
   * @returns {Promise<Object|null>} Created pair or null
   */
  async _createPair(chosen, rejected, contextType, taskType, confidenceMultiplier = 1.0) {
    // Calculate confidence based on score difference
    let confidence = PHI_INV;
    if (chosen.q_score && rejected.q_score) {
      const scoreDiff = (chosen.q_score - rejected.q_score) / 100;
      confidence = Math.min(PHI_INV, Math.max(PHI_INV_3, scoreDiff * 2));
    }
    confidence *= confidenceMultiplier;

    // Skip if confidence too low
    if (confidence < this.minConfidence) {
      return null;
    }

    // Build chosen/rejected data
    const chosenData = this._buildResponseData(chosen);
    const rejectedData = this._buildResponseData(rejected);

    // Build context
    const context = {
      contextType,
      taskType,
      scoreDiff: (chosen.q_score || 50) - (rejected.q_score || 50),
      chosenScore: chosen.q_score,
      rejectedScore: rejected.q_score,
    };

    // Insert into preference_pairs table
    const { rows } = await this.pool.query(`
      INSERT INTO preference_pairs (
        chosen, rejected, context, context_type, task_type,
        feedback_ids, confidence, service_id
      )
      VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
      RETURNING id
    `, [
      JSON.stringify(chosenData),
      JSON.stringify(rejectedData),
      JSON.stringify(context),
      contextType,
      taskType,
      [chosen.id, rejected.id],
      confidence,
      this.serviceId,
    ]);

    return {
      id: rows[0]?.id,
      chosenId: chosen.id,
      rejectedId: rejected.id,
      confidence,
    };
  }

  /**
   * Build response data from feedback item
   *
   * @private
   * @param {Object} item - Feedback item
   * @returns {Object} Response data for preference pair
   */
  _buildResponseData(item) {
    return {
      judgmentId: item.judgment_id,
      qScore: item.q_score,
      verdict: item.verdict,
      itemType: item.item_type,
      outcome: item.outcome,
      actualScore: item.actual_score,
      reason: item.reason,
      sourceContext: item.source_context,
    };
  }

  /**
   * Get processor statistics
   *
   * @returns {Object} Stats
   */
  getStats() {
    return { ...this.stats };
  }

  /**
   * Get unprocessed feedback count
   *
   * @returns {Promise<number>} Count
   */
  async getUnprocessedCount() {
    const { rows } = await this.pool.query(`
      SELECT COUNT(*) as count FROM feedback WHERE applied = FALSE
    `);
    return parseInt(rows[0]?.count || 0);
  }

  /**
   * Get preference pair count
   *
   * @returns {Promise<number>} Count
   */
  async getPairCount() {
    const { rows } = await this.pool.query(`
      SELECT COUNT(*) as count FROM preference_pairs WHERE service_id = $1
    `, [this.serviceId]);
    return parseInt(rows[0]?.count || 0);
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create the DPOProcessor singleton
 *
 * @param {Object} options - Processor options
 * @returns {DPOProcessor} Singleton instance
 */
export function getDPOProcessor(options = {}) {
  if (!_instance) {
    _instance = new DPOProcessor(options);
  }
  return _instance;
}

/**
 * Reset the singleton (for testing)
 */
export function resetDPOProcessor() {
  _instance = null;
}

export default {
  DPOProcessor,
  getDPOProcessor,
  resetDPOProcessor,
};
