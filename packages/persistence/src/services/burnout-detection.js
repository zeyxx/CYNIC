/**
 * Burnout Detection Service
 *
 * v1.1: Tracks psychology over time, detects trends, generates proactive warnings.
 *
 * φ-derived thresholds:
 * - Risk threshold: 0.618 (φ⁻¹) - burnout risk warning
 * - Critical threshold: 0.382 (φ⁻²) - energy critical
 * - Decay window: 13 snapshots (Fibonacci) for trend analysis
 *
 * "Prévenir vaut mieux que guérir" - κυνικός
 *
 * @module @cynic/persistence/services/burnout-detection
 */

'use strict';

import { createLogger } from '@cynic/core';

const log = createLogger('BurnoutDetection');

// φ constants
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;
const PHI_INV_3 = 0.236067977499790;

/**
 * Warning severity levels
 */
export const WarningSeverity = Object.freeze({
  INFO: 'info',
  CAUTION: 'caution',
  WARNING: 'warning',
  CRITICAL: 'critical',
});

/**
 * Warning types
 */
export const WarningType = Object.freeze({
  ENERGY_DECLINING: 'energy_declining',
  FRUSTRATION_RISING: 'frustration_rising',
  BURNOUT_THRESHOLD_NEAR: 'burnout_threshold_near',
  BURNOUT_ACTIVE: 'burnout_active',
  RECOVERY_DETECTED: 'recovery_detected',
  SESSION_TOO_LONG: 'session_too_long',
  ERROR_RATE_HIGH: 'error_rate_high',
});

/**
 * Default burnout detection configuration
 */
export const DEFAULT_BURNOUT_CONFIG = Object.freeze({
  // Thresholds (φ-derived)
  riskThreshold: PHI_INV,              // 61.8% burnout score = warning
  criticalThreshold: 1 - PHI_INV_3,    // 76.4% burnout score = critical
  energyCritical: PHI_INV_2,           // 38.2% energy = low energy warning
  frustrationHigh: PHI_INV,            // 61.8% frustration = warning

  // Trend detection
  trendWindowSize: 13,                 // Fibonacci - snapshots for trend
  trendDeclineThreshold: 0.1,          // 10% decline = trend warning
  snapshotIntervalMinutes: 5,          // Snapshot every 5 minutes

  // Episode tracking
  episodeMinDurationMinutes: 15,       // Min burnout duration for episode
  recoveryConfirmationCount: 3,        // Snapshots to confirm recovery

  // Limits
  maxActiveEpisodes: 1,                // Only 1 active episode per user
  warningCooldownMinutes: 30,          // Don't spam same warning
});

/**
 * Burnout Detection Service
 *
 * Monitors psychology state, detects burnout trends, generates warnings.
 */
export class BurnoutDetection {
  /**
   * Create a BurnoutDetection service
   *
   * @param {Object} options - Configuration
   * @param {Object} options.pool - PostgreSQL connection pool
   * @param {Object} [options.config] - Override default config
   */
  constructor(options = {}) {
    if (!options.pool) {
      throw new Error('BurnoutDetection requires a database pool');
    }

    this._pool = options.pool;
    this._config = {
      ...DEFAULT_BURNOUT_CONFIG,
      ...options.config,
    };

    this._stats = {
      snapshotsRecorded: 0,
      warningsGenerated: 0,
      episodesStarted: 0,
      episodesEnded: 0,
      lastCheck: null,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // SNAPSHOT RECORDING
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Record a psychology snapshot for trend analysis
   *
   * @param {string} userId - User ID
   * @param {Object} dimensions - Current psychology dimensions
   * @param {Object} [context={}] - Additional context (session, work, errors)
   * @returns {Promise<Object>} Recorded snapshot with burnout analysis
   */
  async recordSnapshot(userId, dimensions, context = {}) {
    const {
      energy = PHI_INV,
      focus = PHI_INV,
      creativity = 0.5,
      frustration = PHI_INV_2,
    } = dimensions;

    // Calculate burnout and flow scores
    const burnoutScore = this._calculateBurnoutScore(energy, frustration, creativity);
    const flowScore = this._calculateFlowScore(energy, focus, creativity, frustration);

    // Insert snapshot
    const { rows } = await this._pool.query(`
      INSERT INTO psychology_snapshots (
        user_id,
        session_id,
        energy,
        focus,
        creativity,
        frustration,
        burnout_score,
        flow_score,
        work_done,
        heat_generated,
        error_count
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
      RETURNING *
    `, [
      userId,
      context.sessionId || null,
      energy,
      focus,
      creativity,
      frustration,
      burnoutScore,
      flowScore,
      context.workDone || 0,
      context.heatGenerated || 0,
      context.errorCount || 0,
    ]);

    this._stats.snapshotsRecorded++;

    // Analyze for warnings and episodes
    const analysis = await this._analyzeSnapshot(userId, rows[0]);

    return {
      snapshot: rows[0],
      burnoutScore,
      flowScore,
      ...analysis,
    };
  }

  /**
   * Calculate burnout score from dimensions
   * Formula: burnout = (frustration × (1 - energy)) × creativity_factor
   * @private
   */
  _calculateBurnoutScore(energy, frustration, creativity) {
    const baseScore = (1 - energy) * frustration;
    const creativityFactor = 1 + ((1 - creativity) * PHI_INV * 0.5);
    return Math.min(1, Math.max(0, baseScore * creativityFactor));
  }

  /**
   * Calculate flow score from dimensions
   * @private
   */
  _calculateFlowScore(energy, focus, creativity, frustration) {
    // Flow = high energy × focus × creativity × low frustration
    return Math.min(1, Math.max(0,
      energy * PHI_INV +
      focus * PHI_INV +
      creativity * (1 - PHI_INV) -
      frustration * PHI_INV,
    ));
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // TREND ANALYSIS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get psychology trends for a user
   *
   * @param {string} userId - User ID
   * @param {Object} [options] - Options
   * @param {number} [options.windowSize] - Snapshots to analyze
   * @returns {Promise<Object>} Trend analysis
   */
  async getTrends(userId, options = {}) {
    const windowSize = options.windowSize || this._config.trendWindowSize;

    const { rows: snapshots } = await this._pool.query(`
      SELECT
        energy,
        focus,
        frustration,
        burnout_score,
        flow_score,
        created_at
      FROM psychology_snapshots
      WHERE user_id = $1
      ORDER BY created_at DESC
      LIMIT $2
    `, [userId, windowSize]);

    if (snapshots.length < 3) {
      return {
        hasSufficientData: false,
        snapshotCount: snapshots.length,
        trends: null,
      };
    }

    // Calculate trends (oldest to newest for regression)
    const orderedSnapshots = [...snapshots].reverse();

    const trends = {
      energy: this._calculateTrend(orderedSnapshots.map(s => s.energy)),
      focus: this._calculateTrend(orderedSnapshots.map(s => s.focus)),
      frustration: this._calculateTrend(orderedSnapshots.map(s => s.frustration)),
      burnout: this._calculateTrend(orderedSnapshots.map(s => s.burnout_score)),
      flow: this._calculateTrend(orderedSnapshots.map(s => s.flow_score)),
    };

    // Determine overall trajectory
    const trajectory = this._determineTrajectory(trends);

    return {
      hasSufficientData: true,
      snapshotCount: snapshots.length,
      timespan: {
        from: snapshots[snapshots.length - 1].created_at,
        to: snapshots[0].created_at,
      },
      current: {
        energy: snapshots[0].energy,
        frustration: snapshots[0].frustration,
        burnout: snapshots[0].burnout_score,
        flow: snapshots[0].flow_score,
      },
      trends,
      trajectory,
    };
  }

  /**
   * Calculate linear trend from values
   * @private
   */
  _calculateTrend(values) {
    if (values.length < 2) return { slope: 0, direction: 'stable' };

    // Simple linear regression
    const n = values.length;
    let sumX = 0, sumY = 0, sumXY = 0, sumX2 = 0;

    for (let i = 0; i < n; i++) {
      sumX += i;
      sumY += values[i];
      sumXY += i * values[i];
      sumX2 += i * i;
    }

    const slope = (n * sumXY - sumX * sumY) / (n * sumX2 - sumX * sumX);
    const avgValue = sumY / n;

    // Normalize slope to percentage change per snapshot
    const normalizedSlope = avgValue > 0 ? slope / avgValue : 0;

    // Direction based on slope magnitude
    let direction;
    if (Math.abs(normalizedSlope) < 0.01) {
      direction = 'stable';
    } else if (normalizedSlope > 0.05) {
      direction = 'rising_fast';
    } else if (normalizedSlope > 0) {
      direction = 'rising';
    } else if (normalizedSlope < -0.05) {
      direction = 'declining_fast';
    } else {
      direction = 'declining';
    }

    return {
      slope: normalizedSlope,
      direction,
      start: values[0],
      end: values[n - 1],
      average: avgValue,
    };
  }

  /**
   * Determine overall trajectory from individual trends
   * @private
   */
  _determineTrajectory(trends) {
    const warnings = [];
    let risk = 'stable';

    // Check for concerning patterns
    if (trends.energy.direction.includes('declining')) {
      warnings.push('energy_declining');
      risk = 'elevated';
    }

    if (trends.frustration.direction.includes('rising')) {
      warnings.push('frustration_rising');
      risk = 'elevated';
    }

    if (trends.burnout.direction.includes('rising')) {
      warnings.push('burnout_trending_up');
      if (trends.burnout.end > this._config.riskThreshold) {
        risk = 'high';
      }
    }

    // Check for recovery
    if (trends.energy.direction.includes('rising') &&
        trends.frustration.direction.includes('declining')) {
      warnings.push('recovery_trend');
      risk = 'recovering';
    }

    return {
      risk,
      warnings,
      isStable: risk === 'stable',
      isRecovering: risk === 'recovering',
      needsIntervention: risk === 'high',
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // BURNOUT RISK CALCULATION
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get comprehensive burnout risk assessment
   *
   * @param {string} userId - User ID
   * @returns {Promise<Object>} Risk assessment
   */
  async getBurnoutRisk(userId) {
    // Get latest snapshot
    const { rows: [latest] } = await this._pool.query(`
      SELECT *
      FROM psychology_snapshots
      WHERE user_id = $1
      ORDER BY created_at DESC
      LIMIT 1
    `, [userId]);

    // Get trends
    const trends = await this.getTrends(userId);

    // Check for active episode
    const { rows: [activeEpisode] } = await this._pool.query(`
      SELECT *
      FROM burnout_episodes
      WHERE user_id = $1 AND ended_at IS NULL
      ORDER BY started_at DESC
      LIMIT 1
    `, [userId]);

    // Calculate comprehensive risk
    const risk = this._calculateComprehensiveRisk(latest, trends, activeEpisode);

    return {
      timestamp: Date.now(),
      userId,
      current: latest ? {
        energy: latest.energy,
        frustration: latest.frustration,
        burnoutScore: latest.burnout_score,
        flowScore: latest.flow_score,
      } : null,
      trends: trends.hasSufficientData ? trends : null,
      activeEpisode: activeEpisode || null,
      risk,
    };
  }

  /**
   * Calculate comprehensive risk from all factors
   * @private
   */
  _calculateComprehensiveRisk(latest, trends, activeEpisode) {
    // No data
    if (!latest) {
      return {
        level: 'unknown',
        score: 0,
        factors: [],
        recommendation: null,
      };
    }

    const factors = [];
    let riskScore = latest.burnout_score;

    // Factor in trends
    if (trends?.hasSufficientData) {
      if (trends.trends.burnout.direction.includes('rising')) {
        riskScore += 0.1;
        factors.push({
          factor: 'burnout_trend',
          impact: 0.1,
          description: 'Burnout trending upward',
        });
      }

      if (trends.trends.energy.direction.includes('declining_fast')) {
        riskScore += 0.15;
        factors.push({
          factor: 'energy_crash',
          impact: 0.15,
          description: 'Energy declining rapidly',
        });
      }
    }

    // Factor in active episode
    if (activeEpisode) {
      riskScore += 0.2;
      factors.push({
        factor: 'active_episode',
        impact: 0.2,
        description: 'Currently in burnout episode',
      });
    }

    // Clamp to [0, 1]
    riskScore = Math.min(1, Math.max(0, riskScore));

    // Determine level
    let level;
    if (riskScore >= this._config.criticalThreshold) {
      level = 'critical';
    } else if (riskScore >= this._config.riskThreshold) {
      level = 'high';
    } else if (riskScore >= PHI_INV_2) {
      level = 'moderate';
    } else {
      level = 'low';
    }

    // Generate recommendation
    const recommendation = this._generateRecommendation(level, factors, latest);

    return {
      level,
      score: riskScore,
      factors,
      recommendation,
    };
  }

  /**
   * Generate recommendation based on risk
   * @private
   */
  _generateRecommendation(level, factors, latest) {
    switch (level) {
      case 'critical':
        return {
          action: 'stop',
          message: '*GROWL* Critical burnout detected. Take a break now.',
          urgency: 'immediate',
        };

      case 'high':
        return {
          action: 'pause',
          message: '*concerned sniff* High burnout risk. Consider wrapping up soon.',
          urgency: 'soon',
        };

      case 'moderate':
        if (latest?.energy < this._config.energyCritical) {
          return {
            action: 'monitor',
            message: '*ears perk* Energy low. Watch your pace.',
            urgency: 'watch',
          };
        }
        return {
          action: 'continue',
          message: 'Moderate stress. Normal working state.',
          urgency: 'none',
        };

      default:
        return {
          action: 'continue',
          message: '*tail wag* Looking good. Keep it up.',
          urgency: 'none',
        };
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PROACTIVE WARNINGS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Generate proactive warnings based on current state
   *
   * @param {string} userId - User ID
   * @returns {Promise<Object[]>} Warnings to show
   */
  async getProactiveWarnings(userId) {
    const risk = await this.getBurnoutRisk(userId);
    const warnings = [];

    if (!risk.current) return warnings;

    // Check recent warnings to avoid spam
    const { rows: recentWarnings } = await this._pool.query(`
      SELECT warning_type
      FROM burnout_warnings
      WHERE user_id = $1
        AND created_at > NOW() - INTERVAL '${this._config.warningCooldownMinutes} minutes'
    `, [userId]);

    const recentTypes = new Set(recentWarnings.map(w => w.warning_type));

    // Generate warnings based on risk factors
    if (risk.risk.level === 'critical' && !recentTypes.has(WarningType.BURNOUT_ACTIVE)) {
      warnings.push({
        type: WarningType.BURNOUT_ACTIVE,
        severity: WarningSeverity.CRITICAL,
        message: risk.risk.recommendation.message,
        burnoutScore: risk.current.burnoutScore,
        energy: risk.current.energy,
        frustration: risk.current.frustration,
      });
    }

    if (risk.risk.level === 'high' && !recentTypes.has(WarningType.BURNOUT_THRESHOLD_NEAR)) {
      warnings.push({
        type: WarningType.BURNOUT_THRESHOLD_NEAR,
        severity: WarningSeverity.WARNING,
        message: risk.risk.recommendation.message,
        burnoutScore: risk.current.burnoutScore,
        energy: risk.current.energy,
        frustration: risk.current.frustration,
      });
    }

    // Trend-based warnings
    if (risk.trends?.trajectory?.warnings?.includes('energy_declining')) {
      if (!recentTypes.has(WarningType.ENERGY_DECLINING)) {
        warnings.push({
          type: WarningType.ENERGY_DECLINING,
          severity: WarningSeverity.CAUTION,
          message: '*sniff* Energy declining. Consider taking a break.',
          burnoutScore: risk.current.burnoutScore,
          energy: risk.current.energy,
          frustration: risk.current.frustration,
        });
      }
    }

    if (risk.trends?.trajectory?.warnings?.includes('frustration_rising')) {
      if (!recentTypes.has(WarningType.FRUSTRATION_RISING)) {
        warnings.push({
          type: WarningType.FRUSTRATION_RISING,
          severity: WarningSeverity.CAUTION,
          message: '*ears flat* Frustration rising. Step back and breathe.',
          burnoutScore: risk.current.burnoutScore,
          energy: risk.current.energy,
          frustration: risk.current.frustration,
        });
      }
    }

    // Record warnings
    for (const warning of warnings) {
      await this._recordWarning(userId, warning);
    }

    return warnings;
  }

  /**
   * Record a warning
   * @private
   */
  async _recordWarning(userId, warning) {
    await this._pool.query(`
      INSERT INTO burnout_warnings (
        user_id,
        warning_type,
        severity,
        message,
        burnout_score,
        energy,
        frustration
      ) VALUES ($1, $2, $3, $4, $5, $6, $7)
    `, [
      userId,
      warning.type,
      warning.severity,
      warning.message,
      warning.burnoutScore,
      warning.energy,
      warning.frustration,
    ]);

    this._stats.warningsGenerated++;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // EPISODE TRACKING
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Analyze snapshot and manage burnout episodes
   * @private
   */
  async _analyzeSnapshot(userId, snapshot) {
    const result = {
      isInBurnout: false,
      episodeStarted: false,
      episodeEnded: false,
      warnings: [],
    };

    const burnoutScore = snapshot.burnout_score;
    const isHighBurnout = burnoutScore >= this._config.riskThreshold;

    // Check for active episode
    const { rows: [activeEpisode] } = await this._pool.query(`
      SELECT *
      FROM burnout_episodes
      WHERE user_id = $1 AND ended_at IS NULL
      LIMIT 1
    `, [userId]);

    if (isHighBurnout) {
      result.isInBurnout = true;

      if (!activeEpisode) {
        // Start new episode
        await this._startEpisode(userId, snapshot);
        result.episodeStarted = true;
        this._stats.episodesStarted++;
      } else {
        // Update peak
        await this._updateEpisodePeak(activeEpisode.id, burnoutScore);
      }
    } else if (activeEpisode) {
      // Check for recovery
      const recovery = await this._checkRecovery(userId, activeEpisode.id);
      if (recovery.confirmed) {
        await this._endEpisode(activeEpisode.id);
        result.episodeEnded = true;
        this._stats.episodesEnded++;
      }
    }

    return result;
  }

  /**
   * Start a new burnout episode
   * @private
   */
  async _startEpisode(userId, snapshot) {
    await this._pool.query(`
      INSERT INTO burnout_episodes (
        user_id,
        started_at,
        peak_burnout_score,
        trigger_context
      ) VALUES ($1, NOW(), $2, $3)
    `, [
      userId,
      snapshot.burnout_score,
      JSON.stringify({
        energy: snapshot.energy,
        frustration: snapshot.frustration,
        workDone: snapshot.work_done,
        errorCount: snapshot.error_count,
      }),
    ]);

    log.warn('Burnout episode started', { userId, burnoutScore: snapshot.burnout_score });
  }

  /**
   * Update episode peak burnout
   * @private
   */
  async _updateEpisodePeak(episodeId, burnoutScore) {
    await this._pool.query(`
      UPDATE burnout_episodes
      SET peak_burnout_score = GREATEST(peak_burnout_score, $1),
          session_count = session_count + 1
      WHERE id = $2
    `, [burnoutScore, episodeId]);
  }

  /**
   * Check if recovery is confirmed
   * @private
   */
  async _checkRecovery(userId, episodeId) {
    const { rows: recentSnapshots } = await this._pool.query(`
      SELECT burnout_score
      FROM psychology_snapshots
      WHERE user_id = $1
      ORDER BY created_at DESC
      LIMIT $2
    `, [userId, this._config.recoveryConfirmationCount]);

    const allBelowThreshold = recentSnapshots.every(
      s => s.burnout_score < this._config.riskThreshold,
    );

    return {
      confirmed: allBelowThreshold && recentSnapshots.length >= this._config.recoveryConfirmationCount,
      snapshotsChecked: recentSnapshots.length,
    };
  }

  /**
   * End a burnout episode
   * @private
   */
  async _endEpisode(episodeId) {
    await this._pool.query(`
      UPDATE burnout_episodes
      SET ended_at = NOW(),
          duration_minutes = EXTRACT(EPOCH FROM (NOW() - started_at)) / 60
      WHERE id = $1
    `, [episodeId]);

    log.info('Burnout episode ended', { episodeId });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // HISTORICAL ANALYSIS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get burnout history for a user
   *
   * @param {string} userId - User ID
   * @param {Object} [options] - Options
   * @param {number} [options.limit=10] - Max episodes
   * @returns {Promise<Object>} History
   */
  async getBurnoutHistory(userId, options = {}) {
    const { limit = 10 } = options;

    const { rows: episodes } = await this._pool.query(`
      SELECT *
      FROM burnout_episodes
      WHERE user_id = $1
      ORDER BY started_at DESC
      LIMIT $2
    `, [userId, limit]);

    // Calculate aggregates
    const { rows: [stats] } = await this._pool.query(`
      SELECT
        COUNT(*) as total_episodes,
        AVG(duration_minutes) as avg_duration,
        AVG(peak_burnout_score) as avg_peak_score,
        MAX(peak_burnout_score) as max_peak_score,
        SUM(duration_minutes) as total_burnout_minutes
      FROM burnout_episodes
      WHERE user_id = $1 AND ended_at IS NOT NULL
    `, [userId]);

    return {
      userId,
      episodes,
      stats: {
        totalEpisodes: parseInt(stats.total_episodes) || 0,
        avgDurationMinutes: parseFloat(stats.avg_duration) || 0,
        avgPeakScore: parseFloat(stats.avg_peak_score) || 0,
        maxPeakScore: parseFloat(stats.max_peak_score) || 0,
        totalBurnoutMinutes: parseFloat(stats.total_burnout_minutes) || 0,
      },
    };
  }

  /**
   * Get warning effectiveness stats
   *
   * @param {string} userId - User ID (optional, for global stats)
   * @returns {Promise<Object>} Warning stats
   */
  async getWarningEffectiveness(userId = null) {
    const params = userId ? [userId] : [];
    const whereClause = userId ? 'WHERE user_id = $1' : '';

    const { rows } = await this._pool.query(`
      SELECT
        warning_type,
        severity,
        COUNT(*) as total,
        SUM(CASE WHEN was_effective THEN 1 ELSE 0 END) as effective,
        AVG(CASE WHEN was_effective THEN 1.0 ELSE 0.0 END) as effectiveness_rate
      FROM burnout_warnings
      ${whereClause}
      GROUP BY warning_type, severity
      ORDER BY total DESC
    `, params);

    return {
      userId,
      byType: rows.reduce((acc, row) => {
        acc[row.warning_type] = {
          severity: row.severity,
          total: parseInt(row.total),
          effective: parseInt(row.effective) || 0,
          rate: parseFloat(row.effectiveness_rate) || 0,
        };
        return acc;
      }, {}),
      overall: {
        total: rows.reduce((sum, r) => sum + parseInt(r.total), 0),
        effective: rows.reduce((sum, r) => sum + (parseInt(r.effective) || 0), 0),
      },
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // FEEDBACK
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Record warning feedback
   *
   * @param {string} warningId - Warning ID
   * @param {boolean} wasAcknowledged - User acknowledged
   * @param {boolean} wasEffective - Warning helped
   */
  async recordWarningFeedback(warningId, wasAcknowledged, wasEffective) {
    await this._pool.query(`
      UPDATE burnout_warnings
      SET was_acknowledged = $1,
          was_effective = $2
      WHERE id = $3
    `, [wasAcknowledged, wasEffective, warningId]);
  }

  /**
   * Record intervention that helped recovery
   *
   * @param {string} episodeId - Episode ID
   * @param {string} intervention - What helped (e.g., 'break', 'walk', 'coffee')
   */
  async recordRecoveryIntervention(episodeId, intervention) {
    await this._pool.query(`
      UPDATE burnout_episodes
      SET recovery_intervention = $1
      WHERE id = $2
    `, [intervention, episodeId]);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // STATISTICS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get burnout detection statistics
   * @returns {Object}
   */
  getStats() {
    return {
      ...this._stats,
      config: { ...this._config },
    };
  }
}

/**
 * Create a BurnoutDetection instance
 *
 * @param {Object} options - Options
 * @returns {BurnoutDetection}
 */
export function createBurnoutDetection(options) {
  return new BurnoutDetection(options);
}

export default BurnoutDetection;
