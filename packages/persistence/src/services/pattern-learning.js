/**
 * Pattern Learning Service
 *
 * v1.1: Auto-extracts patterns from judgments, applies confidence decay,
 * and clusters similar patterns.
 *
 * φ-derived thresholds:
 * - Extraction threshold: 0.618 (φ⁻¹) - minimum judgment score to extract pattern
 * - Confidence decay: 0.0618 (φ⁻¹ / 10) - decay per period without use
 * - Similarity threshold: 0.618 (φ⁻¹) - for clustering
 *
 * "Patterns emerge from chaos through attention" - κυνικός
 *
 * @module @cynic/persistence/services/pattern-learning
 */

'use strict';

import { createLogger } from '@cynic/core';

const log = createLogger('PatternLearning');

// φ constants
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;
const PHI_INV_3 = 0.236067977499790;

/**
 * Default pattern learning configuration
 */
export const DEFAULT_PATTERN_CONFIG = Object.freeze({
  extractionThreshold: PHI_INV,        // 61.8% - min judgment score to extract
  confidenceDecay: PHI_INV / 10,       // 6.18% - decay per period
  minConfidence: PHI_INV_3,            // 23.6% - min confidence to keep
  similarityThreshold: PHI_INV,        // 61.8% - for clustering
  maxPatternsPerCategory: 21,          // Fibonacci
  decayPeriodDays: 13,                 // Fibonacci
  batchSize: 34,                       // Fibonacci
});

/**
 * Pattern Learning Service
 *
 * Learns patterns from judgments and maintains pattern confidence.
 */
export class PatternLearning {
  /**
   * Create a PatternLearning service
   *
   * @param {Object} options - Configuration
   * @param {Object} options.pool - PostgreSQL connection pool
   * @param {Object} [options.embedder] - Embedding service for similarity
   * @param {Object} [options.config] - Override default config
   */
  constructor(options = {}) {
    if (!options.pool) {
      throw new Error('PatternLearning requires a database pool');
    }

    this._pool = options.pool;
    this._embedder = options.embedder || null;
    this._config = {
      ...DEFAULT_PATTERN_CONFIG,
      ...options.config,
    };

    this._stats = {
      totalExtracted: 0,
      totalDecayed: 0,
      totalPruned: 0,
      totalClustered: 0,
      lastRun: null,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PATTERN EXTRACTION
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Extract patterns from recent judgments
   *
   * @param {Object} [options] - Options
   * @param {number} [options.limit=34] - Max judgments to process
   * @param {boolean} [options.dryRun=false] - Don't save patterns
   * @returns {Promise<Object>} Extraction results
   */
  async extractFromJudgments(options = {}) {
    const { limit = this._config.batchSize, dryRun = false } = options;

    const result = {
      processed: 0,
      extracted: [],
      skipped: 0,
      timestamp: Date.now(),
    };

    // Get high-quality judgments not yet processed for patterns
    const { rows: judgments } = await this._pool.query(`
      SELECT j.*
      FROM judgments j
      LEFT JOIN patterns p ON p.source_judgments ? j.judgment_id
      WHERE j.q_score >= $1
        AND p.pattern_id IS NULL
      ORDER BY j.created_at DESC
      LIMIT $2
    `, [this._config.extractionThreshold * 100, limit]);

    for (const judgment of judgments) {
      result.processed++;

      const pattern = this._extractPatternFromJudgment(judgment);
      if (!pattern) {
        result.skipped++;
        continue;
      }

      if (!dryRun) {
        await this._savePattern(pattern);
      }

      result.extracted.push({
        judgmentId: judgment.judgment_id,
        patternId: pattern.patternId,
        category: pattern.category,
        name: pattern.name,
      });
    }

    this._stats.totalExtracted += result.extracted.length;
    return result;
  }

  /**
   * Extract pattern data from a judgment
   *
   * @param {Object} judgment - Judgment object
   * @returns {Object|null} Pattern or null
   * @private
   */
  _extractPatternFromJudgment(judgment) {
    // Extract category from judgment type or verdict
    const category = this._inferCategory(judgment);
    if (!category) return null;

    // Extract pattern name from judgment content
    const name = this._inferPatternName(judgment);
    if (!name) return null;

    // Calculate initial confidence from judgment score
    const confidence = Math.min(1, (judgment.q_score || 0) / 100);

    return {
      patternId: `pat_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`,
      category,
      name,
      description: judgment.reasoning || null,
      confidence,
      frequency: 1,
      sourceJudgments: [judgment.judgment_id],
      sourceCount: 1,
      tags: this._inferTags(judgment),
      data: {
        verdict: judgment.verdict,
        originalScore: judgment.q_score,
        extractedAt: Date.now(),
      },
    };
  }

  /**
   * Infer category from judgment
   * @private
   */
  _inferCategory(judgment) {
    // From explicit category if present
    if (judgment.category) return judgment.category;

    // From subject type
    if (judgment.subject_type) {
      const typeMap = {
        code: 'code_quality',
        token: 'token_analysis',
        decision: 'decision_making',
        error: 'error_handling',
      };
      return typeMap[judgment.subject_type] || judgment.subject_type;
    }

    // From verdict
    if (judgment.verdict) {
      const verdictMap = {
        HOWL: 'excellence',
        WAG: 'quality',
        BARK: 'warning',
        GROWL: 'danger',
      };
      return verdictMap[judgment.verdict] || 'general';
    }

    return 'general';
  }

  /**
   * Infer pattern name from judgment
   * @private
   */
  _inferPatternName(judgment) {
    // Use subject if available
    if (judgment.subject_name) {
      return `Pattern: ${judgment.subject_name}`;
    }

    // Use first line of reasoning
    if (judgment.reasoning) {
      const firstLine = judgment.reasoning.split('\n')[0].slice(0, 100);
      return firstLine || null;
    }

    return null;
  }

  /**
   * Infer tags from judgment
   * @private
   */
  _inferTags(judgment) {
    const tags = [];

    if (judgment.verdict) tags.push(judgment.verdict.toLowerCase());
    if (judgment.subject_type) tags.push(judgment.subject_type);
    if (judgment.category) tags.push(judgment.category);

    // Extract tags from data if present
    if (judgment.data?.tags) {
      tags.push(...judgment.data.tags);
    }

    return [...new Set(tags)]; // Dedupe
  }

  /**
   * Save pattern to database
   * @private
   */
  async _savePattern(pattern) {
    await this._pool.query(`
      INSERT INTO patterns (
        pattern_id, category, name, description,
        confidence, frequency, source_judgments, source_count,
        tags, data
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
      ON CONFLICT (pattern_id) DO UPDATE SET
        frequency = patterns.frequency + 1,
        source_judgments = patterns.source_judgments || $7,
        source_count = patterns.source_count + 1,
        updated_at = NOW()
    `, [
      pattern.patternId,
      pattern.category,
      pattern.name,
      pattern.description,
      pattern.confidence,
      pattern.frequency,
      JSON.stringify(pattern.sourceJudgments),
      pattern.sourceCount,
      pattern.tags,
      JSON.stringify(pattern.data),
    ]);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // CONFIDENCE DECAY
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Apply confidence decay to stale patterns
   *
   * @param {Object} [options] - Options
   * @param {boolean} [options.dryRun=false] - Don't modify
   * @returns {Promise<Object>} Decay results
   */
  async applyConfidenceDecay(options = {}) {
    const { dryRun = false } = options;

    const result = {
      decayed: 0,
      pruned: 0,
      patterns: [],
      timestamp: Date.now(),
    };

    // Find patterns not updated within decay period
    const decayDate = new Date();
    decayDate.setDate(decayDate.getDate() - this._config.decayPeriodDays);

    const { rows: stalePatterns } = await this._pool.query(`
      SELECT pattern_id, name, confidence, frequency
      FROM patterns
      WHERE updated_at < $1
        AND confidence > $2
      ORDER BY confidence DESC
      LIMIT $3
    `, [decayDate, this._config.minConfidence, this._config.batchSize]);

    for (const pattern of stalePatterns) {
      const newConfidence = Math.max(
        this._config.minConfidence,
        pattern.confidence * (1 - this._config.confidenceDecay),
      );

      result.patterns.push({
        patternId: pattern.pattern_id,
        oldConfidence: pattern.confidence,
        newConfidence,
      });

      if (!dryRun) {
        await this._pool.query(
          'UPDATE patterns SET confidence = $1, updated_at = NOW() WHERE pattern_id = $2',
          [newConfidence, pattern.pattern_id],
        );
      }

      result.decayed++;
    }

    // Prune patterns below minimum confidence
    if (!dryRun) {
      const { rowCount } = await this._pool.query(
        'DELETE FROM patterns WHERE confidence <= $1',
        [this._config.minConfidence],
      );
      result.pruned = rowCount;
    }

    this._stats.totalDecayed += result.decayed;
    this._stats.totalPruned += result.pruned;
    return result;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PATTERN CLUSTERING
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Cluster similar patterns within a category
   *
   * @param {string} category - Category to cluster
   * @param {Object} [options] - Options
   * @param {boolean} [options.dryRun=false] - Don't modify
   * @returns {Promise<Object>} Clustering results
   */
  async clusterPatterns(category, options = {}) {
    const { dryRun = false } = options;

    const result = {
      category,
      clusters: [],
      merged: 0,
      timestamp: Date.now(),
    };

    if (!this._embedder) {
      return result; // Can't cluster without embedder
    }

    // Get all patterns in category
    const { rows: patterns } = await this._pool.query(`
      SELECT pattern_id, name, description, confidence, frequency
      FROM patterns
      WHERE category = $1
      ORDER BY confidence DESC
    `, [category]);

    if (patterns.length < 2) return result;

    // Generate embeddings for pattern names
    const embeddings = new Map();
    for (const p of patterns) {
      try {
        const text = `${p.name} ${p.description || ''}`.trim();
        embeddings.set(p.pattern_id, await this._embedder.embed(text));
      } catch {
        // Skip patterns we can't embed
      }
    }

    // Find similar patterns
    const merged = new Set();

    for (let i = 0; i < patterns.length; i++) {
      if (merged.has(patterns[i].pattern_id)) continue;

      const cluster = {
        primary: patterns[i].pattern_id,
        members: [],
      };

      const embA = embeddings.get(patterns[i].pattern_id);
      if (!embA) continue;

      for (let j = i + 1; j < patterns.length; j++) {
        if (merged.has(patterns[j].pattern_id)) continue;

        const embB = embeddings.get(patterns[j].pattern_id);
        if (!embB) continue;

        const similarity = this._cosineSimilarity(embA, embB);

        if (similarity >= this._config.similarityThreshold) {
          cluster.members.push({
            patternId: patterns[j].pattern_id,
            similarity,
          });
          merged.add(patterns[j].pattern_id);
        }
      }

      if (cluster.members.length > 0) {
        result.clusters.push(cluster);

        if (!dryRun) {
          // Merge secondary patterns into primary
          for (const member of cluster.members) {
            await this._mergePatterns(cluster.primary, member.patternId);
            result.merged++;
          }
        }
      }
    }

    this._stats.totalClustered += result.merged;
    return result;
  }

  /**
   * Merge two patterns
   * @private
   */
  async _mergePatterns(primaryId, secondaryId) {
    // Get both patterns
    const { rows: [primary] } = await this._pool.query(
      'SELECT * FROM patterns WHERE pattern_id = $1',
      [primaryId],
    );
    const { rows: [secondary] } = await this._pool.query(
      'SELECT * FROM patterns WHERE pattern_id = $1',
      [secondaryId],
    );

    if (!primary || !secondary) return;

    // Combine data
    const combinedConfidence = Math.min(1, primary.confidence + secondary.confidence * PHI_INV_2);
    const combinedFrequency = primary.frequency + secondary.frequency;
    const combinedSources = [
      ...JSON.parse(primary.source_judgments || '[]'),
      ...JSON.parse(secondary.source_judgments || '[]'),
    ];

    // Update primary
    await this._pool.query(`
      UPDATE patterns SET
        confidence = $1,
        frequency = $2,
        source_judgments = $3,
        source_count = $4,
        updated_at = NOW()
      WHERE pattern_id = $5
    `, [
      combinedConfidence,
      combinedFrequency,
      JSON.stringify([...new Set(combinedSources)]),
      combinedSources.length,
      primaryId,
    ]);

    // Delete secondary
    await this._pool.query(
      'DELETE FROM patterns WHERE pattern_id = $1',
      [secondaryId],
    );

    log.debug('Merged patterns', { primary: primaryId, secondary: secondaryId });
  }

  /**
   * Calculate cosine similarity
   * @private
   */
  _cosineSimilarity(a, b) {
    if (!a || !b || a.length !== b.length) return 0;

    let dotProduct = 0;
    let normA = 0;
    let normB = 0;

    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }

    const denominator = Math.sqrt(normA) * Math.sqrt(normB);
    return denominator === 0 ? 0 : dotProduct / denominator;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PATTERN REINFORCEMENT
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Reinforce a pattern when it's useful
   *
   * @param {string} patternId - Pattern ID
   * @param {number} [boost=0.05] - Confidence boost
   * @returns {Promise<Object|null>} Updated pattern
   */
  async reinforcePattern(patternId, boost = 0.05) {
    const { rows } = await this._pool.query(`
      UPDATE patterns SET
        confidence = LEAST(1.0, confidence + $1),
        frequency = frequency + 1,
        updated_at = NOW()
      WHERE pattern_id = $2
      RETURNING *
    `, [boost, patternId]);

    return rows[0] || null;
  }

  /**
   * Weaken a pattern when it fails
   *
   * @param {string} patternId - Pattern ID
   * @param {number} [penalty=0.1] - Confidence penalty
   * @returns {Promise<Object|null>} Updated pattern
   */
  async weakenPattern(patternId, penalty = 0.1) {
    const { rows } = await this._pool.query(`
      UPDATE patterns SET
        confidence = GREATEST($2, confidence - $1),
        updated_at = NOW()
      WHERE pattern_id = $3
      RETURNING *
    `, [penalty, this._config.minConfidence, patternId]);

    return rows[0] || null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // RUN FULL CYCLE
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Run full pattern learning cycle
   *
   * @param {Object} [options] - Options
   * @param {boolean} [options.dryRun=false] - Don't modify
   * @returns {Promise<Object>} Cycle results
   */
  async runCycle(options = {}) {
    const { dryRun = false } = options;
    const startTime = Date.now();

    log.info('Starting pattern learning cycle', { dryRun });

    const results = {
      timestamp: startTime,
      dryRun,
      extraction: null,
      decay: null,
      clustering: null,
      duration: 0,
    };

    try {
      // 1. Extract patterns from new judgments
      results.extraction = await this.extractFromJudgments({ dryRun });

      // 2. Apply confidence decay
      results.decay = await this.applyConfidenceDecay({ dryRun });

      // 3. Cluster patterns by category (only if embedder available)
      if (this._embedder) {
        const { rows: categories } = await this._pool.query(
          'SELECT DISTINCT category FROM patterns',
        );

        results.clustering = {
          categories: [],
          totalMerged: 0,
        };

        for (const { category } of categories) {
          const clusterResult = await this.clusterPatterns(category, { dryRun });
          results.clustering.categories.push({
            category,
            clusters: clusterResult.clusters.length,
            merged: clusterResult.merged,
          });
          results.clustering.totalMerged += clusterResult.merged;
        }
      }

    } catch (err) {
      log.error('Pattern learning cycle error', { error: err.message });
      results.error = err.message;
    }

    results.duration = Date.now() - startTime;
    this._stats.lastRun = startTime;

    log.info('Pattern learning cycle complete', {
      duration: results.duration,
      extracted: results.extraction?.extracted?.length || 0,
      decayed: results.decay?.decayed || 0,
      merged: results.clustering?.totalMerged || 0,
    });

    return results;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // STATISTICS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get pattern learning statistics
   * @returns {Object}
   */
  getStats() {
    return {
      ...this._stats,
      config: { ...this._config },
      hasEmbedder: !!this._embedder,
    };
  }
}

/**
 * Create a PatternLearning instance
 *
 * @param {Object} options - Options
 * @returns {PatternLearning}
 */
export function createPatternLearning(options) {
  return new PatternLearning(options);
}

export default PatternLearning;
