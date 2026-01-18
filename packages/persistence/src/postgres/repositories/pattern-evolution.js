/**
 * Pattern Evolution Repository
 *
 * Tracks how patterns evolve and merge over time.
 * φ-bounded learning with trend analysis.
 *
 * @module @cynic/persistence/repositories/pattern-evolution
 */

'use strict';

import { getPool } from '../client.js';

// φ constants
const PHI_INV = 0.618033988749895;
const DEFAULT_CONFIDENCE = PHI_INV * PHI_INV; // φ⁻² ≈ 0.382

export class PatternEvolutionRepository {
  constructor(db = null) {
    this.db = db || getPool();
  }

  /**
   * Upsert a pattern evolution record
   */
  async upsert(pattern) {
    const { rows } = await this.db.query(`
      INSERT INTO pattern_evolution (
        pattern_type, pattern_key,
        occurrence_count, confidence, strength,
        weight_modifier, threshold_delta,
        trend_direction, trend_velocity
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
      ON CONFLICT (pattern_type, pattern_key) DO UPDATE SET
        occurrence_count = pattern_evolution.occurrence_count + 1,
        confidence = LEAST(
          GREATEST(EXCLUDED.confidence, pattern_evolution.confidence),
          0.618
        ),
        strength = EXCLUDED.strength,
        weight_modifier = EXCLUDED.weight_modifier,
        threshold_delta = EXCLUDED.threshold_delta,
        trend_direction = EXCLUDED.trend_direction,
        trend_velocity = EXCLUDED.trend_velocity,
        last_seen = NOW()
      RETURNING *
    `, [
      pattern.type,
      pattern.key,
      pattern.occurrenceCount || 1,
      pattern.confidence || DEFAULT_CONFIDENCE,
      pattern.strength || 50,
      pattern.weightModifier || 1.0,
      pattern.thresholdDelta || 0,
      pattern.trendDirection || 'stable',
      pattern.trendVelocity || 0,
    ]);

    return rows[0];
  }

  /**
   * Find pattern by type and key
   */
  async find(type, key) {
    const { rows } = await this.db.query(`
      SELECT * FROM pattern_evolution
      WHERE pattern_type = $1 AND pattern_key = $2
    `, [type, key]);

    return rows[0] || null;
  }

  /**
   * Find patterns by type
   */
  async findByType(type, limit = 50) {
    const { rows } = await this.db.query(`
      SELECT * FROM pattern_evolution
      WHERE pattern_type = $1
        AND merged_at IS NULL
      ORDER BY strength DESC, occurrence_count DESC
      LIMIT $2
    `, [type, limit]);

    return rows;
  }

  /**
   * Get top patterns by strength
   */
  async getTopPatterns(limit = 20) {
    const { rows } = await this.db.query(`
      SELECT * FROM pattern_evolution
      WHERE merged_at IS NULL
      ORDER BY strength DESC, confidence DESC
      LIMIT $1
    `, [limit]);

    return rows;
  }

  /**
   * Update pattern metrics
   */
  async updateMetrics(type, key, metrics) {
    const { rows } = await this.db.query(`
      UPDATE pattern_evolution SET
        occurrence_count = occurrence_count + COALESCE($3, 0),
        confidence = LEAST(COALESCE($4, confidence), 0.618),
        strength = COALESCE($5, strength),
        weight_modifier = COALESCE($6, weight_modifier),
        threshold_delta = COALESCE($7, threshold_delta),
        trend_direction = COALESCE($8, trend_direction),
        trend_velocity = COALESCE($9, trend_velocity),
        last_seen = NOW()
      WHERE pattern_type = $1 AND pattern_key = $2
      RETURNING *
    `, [
      type,
      key,
      metrics.occurrenceIncrement || null,
      metrics.confidence || null,
      metrics.strength || null,
      metrics.weightModifier || null,
      metrics.thresholdDelta || null,
      metrics.trendDirection || null,
      metrics.trendVelocity || null,
    ]);

    return rows[0] || null;
  }

  /**
   * Merge similar patterns
   */
  async merge(sourceIds, targetPattern) {
    // Create/update target pattern
    const target = await this.upsert(targetPattern);

    // Mark sources as merged
    await this.db.query(`
      UPDATE pattern_evolution SET
        merged_at = NOW(),
        parent_ids = array_append(parent_ids, $1)
      WHERE id = ANY($2)
    `, [target.id, sourceIds]);

    // Update target with merged IDs
    await this.db.query(`
      UPDATE pattern_evolution SET
        parent_ids = $2
      WHERE id = $1
    `, [target.id, sourceIds]);

    return target;
  }

  /**
   * Get similar patterns (candidates for merging)
   */
  async findSimilar(type, key, threshold = 0.8) {
    const { rows } = await this.db.query(`
      SELECT *,
        similarity(pattern_key, $2) as sim
      FROM pattern_evolution
      WHERE pattern_type = $1
        AND pattern_key != $2
        AND merged_at IS NULL
        AND similarity(pattern_key, $2) >= $3
      ORDER BY sim DESC
      LIMIT 5
    `, [type, key, threshold]);

    return rows;
  }

  /**
   * Get trending patterns
   */
  async getTrending(direction = 'up', limit = 10) {
    const { rows } = await this.db.query(`
      SELECT * FROM pattern_evolution
      WHERE trend_direction = $1
        AND merged_at IS NULL
        AND ABS(trend_velocity) > 0.01
      ORDER BY ABS(trend_velocity) DESC
      LIMIT $2
    `, [direction, limit]);

    return rows;
  }

  /**
   * Get statistics
   */
  async getStats() {
    const { rows } = await this.db.query(`
      SELECT
        COUNT(*) as total,
        COUNT(*) FILTER (WHERE merged_at IS NULL) as active,
        COUNT(*) FILTER (WHERE merged_at IS NOT NULL) as merged,
        AVG(occurrence_count) as avg_occurrences,
        AVG(confidence) as avg_confidence,
        AVG(strength) as avg_strength,
        COUNT(DISTINCT pattern_type) as type_count
      FROM pattern_evolution
    `);

    const stats = rows[0];
    return {
      total: parseInt(stats.total),
      active: parseInt(stats.active),
      merged: parseInt(stats.merged),
      avgOccurrences: parseFloat(stats.avg_occurrences) || 0,
      avgConfidence: parseFloat(stats.avg_confidence) || DEFAULT_CONFIDENCE,
      avgStrength: parseFloat(stats.avg_strength) || 50,
      typeCount: parseInt(stats.type_count),
    };
  }

  /**
   * Get patterns needing review (low confidence, high occurrence)
   */
  async getNeedingReview(limit = 10) {
    const { rows } = await this.db.query(`
      SELECT * FROM pattern_evolution
      WHERE merged_at IS NULL
        AND confidence < 0.382
        AND occurrence_count > 5
      ORDER BY occurrence_count DESC
      LIMIT $1
    `, [limit]);

    return rows;
  }
}

export default PatternEvolutionRepository;
