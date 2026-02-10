/**
 * Social Emergence - C4.7 (SOCIAL × EMERGE)
 *
 * Detects emergent patterns in social interactions over time.
 * Part of the 7×7 Fractal Matrix emergence layer.
 *
 * "Le chien observe la meute humaine" - κυνικός
 *
 * Emerges:
 * - Engagement velocity changes (acceleration/deceleration)
 * - Sentiment drift (community mood shifts)
 * - Influence concentration (few accounts dominate)
 * - Topic clustering (conversation convergence)
 * - Community health trajectories
 *
 * @module @cynic/node/emergence/social-emergence
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, createLogger } from '@cynic/core';

const log = createLogger('SocialEmergence');

/**
 * Emergent pattern types for social awareness
 */
export const SocialPatternType = {
  ENGAGEMENT_VELOCITY: 'engagement_velocity_change',
  SENTIMENT_DRIFT: 'sentiment_drift',
  INFLUENCE_CONCENTRATION: 'influence_concentration',
  TOPIC_CLUSTERING: 'topic_clustering',
  COMMUNITY_HEALTH: 'community_health_trend',
  ACTIVITY_SURGE: 'activity_surge',
  SILENCE_DETECTED: 'social_silence',
};

export const SignificanceLevel = {
  LOW: 'low',
  MEDIUM: 'medium',
  HIGH: 'high',
  CRITICAL: 'critical',
};

const THRESHOLDS = {
  velocityChangeRatio: 0.3,         // 30% change in engagement rate = trend
  sentimentShiftThreshold: PHI_INV_2, // 38.2% shift in sentiment = drift
  influenceConcentration: PHI_INV,  // 61.8% from top 3 accounts = concentration
  surgeMultiplier: 2.0,             // 2x baseline = surge
  silenceWindow: 20,                // 20+ captures without activity = silence
  minDataPoints: 5,                 // Minimum data before detecting patterns
  maxHistory: 500,                  // Rolling window size
};

/**
 * SocialEmergence - Social pattern emergence detector
 */
export class SocialEmergence extends EventEmitter {
  constructor(options = {}) {
    super();

    // Rolling history windows
    this._captures = [];     // { platform, type, engagement, sentiment, timestamp }
    this._authors = [];      // { author, platform, timestamp }

    // Baselines (rolling averages)
    this._baselines = {
      engagementRate: 0,
      sentimentAvg: 0.5,
      authorDistribution: {},
      capturesPerWindow: 0,
    };

    // Detected patterns
    this._patterns = [];
    this._maxPatterns = 100;

    // Stats
    this._stats = {
      capturesRecorded: 0,
      patternsDetected: 0,
      analysesRun: 0,
      lastAnalysis: null,
    };
  }

  /**
   * Record a social capture for emergence analysis
   */
  recordCapture(data) {
    const entry = {
      platform: data.platform || 'unknown',
      type: data.type || data.eventType || 'post',
      engagement: data.engagement || data.likes || data.interactions || 0,
      sentiment: data.sentiment ?? 0.5,
      author: data.author || data.username || 'unknown',
      timestamp: Date.now(),
    };

    this._captures.push(entry);
    if (this._captures.length > THRESHOLDS.maxHistory) {
      this._captures.shift();
    }

    this._authors.push({ author: entry.author, platform: entry.platform, timestamp: entry.timestamp });
    if (this._authors.length > THRESHOLDS.maxHistory) {
      this._authors.shift();
    }

    this._stats.capturesRecorded++;
  }

  /**
   * Run full emergence analysis
   */
  analyze() {
    this._stats.analysesRun++;
    this._stats.lastAnalysis = Date.now();

    const newPatterns = [];

    // 1. Engagement velocity change
    const velocity = this._detectEngagementVelocity();
    if (velocity) newPatterns.push(velocity);

    // 2. Sentiment drift
    const sentiment = this._detectSentimentDrift();
    if (sentiment) newPatterns.push(sentiment);

    // 3. Influence concentration
    const influence = this._detectInfluenceConcentration();
    if (influence) newPatterns.push(influence);

    // 4. Activity surge
    const surge = this._detectActivitySurge();
    if (surge) newPatterns.push(surge);

    // 5. Social silence
    const silence = this._detectSocialSilence();
    if (silence) newPatterns.push(silence);

    // Store and emit new patterns
    for (const pattern of newPatterns) {
      this._registerPattern(pattern);
    }

    return newPatterns;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Pattern Detection
  // ═══════════════════════════════════════════════════════════════════════════

  _detectEngagementVelocity() {
    if (this._captures.length < THRESHOLDS.minDataPoints * 2) return null;

    const mid = Math.floor(this._captures.length / 2);
    const older = this._captures.slice(0, mid);
    const recent = this._captures.slice(mid);

    const olderAvg = older.reduce((s, c) => s + c.engagement, 0) / older.length;
    const recentAvg = recent.reduce((s, c) => s + c.engagement, 0) / recent.length;

    if (olderAvg === 0) return null;
    const changeRatio = (recentAvg - olderAvg) / olderAvg;

    if (Math.abs(changeRatio) >= THRESHOLDS.velocityChangeRatio) {
      const direction = changeRatio > 0 ? 'accelerating' : 'decelerating';
      return {
        type: SocialPatternType.ENGAGEMENT_VELOCITY,
        significance: Math.abs(changeRatio) > 0.5 ? SignificanceLevel.HIGH : SignificanceLevel.MEDIUM,
        data: { olderAvg, recentAvg, changeRatio, direction },
        confidence: Math.min(Math.abs(changeRatio), PHI_INV),
        message: `Engagement ${direction}: ${(changeRatio * 100).toFixed(0)}% change`,
      };
    }

    this._baselines.engagementRate = recentAvg;
    return null;
  }

  _detectSentimentDrift() {
    if (this._captures.length < THRESHOLDS.minDataPoints * 2) return null;

    const mid = Math.floor(this._captures.length / 2);
    const older = this._captures.slice(0, mid);
    const recent = this._captures.slice(mid);

    const olderSentiment = older.reduce((s, c) => s + c.sentiment, 0) / older.length;
    const recentSentiment = recent.reduce((s, c) => s + c.sentiment, 0) / recent.length;
    const drift = recentSentiment - olderSentiment;

    if (Math.abs(drift) >= THRESHOLDS.sentimentShiftThreshold) {
      const direction = drift > 0 ? 'positive' : 'negative';
      return {
        type: SocialPatternType.SENTIMENT_DRIFT,
        significance: Math.abs(drift) > 0.5 ? SignificanceLevel.CRITICAL : SignificanceLevel.HIGH,
        data: { olderSentiment, recentSentiment, drift, direction },
        confidence: Math.min(Math.abs(drift), PHI_INV),
        message: `Sentiment drifting ${direction}: ${(olderSentiment * 100).toFixed(0)}% → ${(recentSentiment * 100).toFixed(0)}%`,
      };
    }

    this._baselines.sentimentAvg = recentSentiment;
    return null;
  }

  _detectInfluenceConcentration() {
    if (this._authors.length < THRESHOLDS.minDataPoints) return null;

    const counts = {};
    for (const a of this._authors) {
      counts[a.author] = (counts[a.author] || 0) + 1;
    }

    const total = this._authors.length;
    const sorted = Object.entries(counts).sort((a, b) => b[1] - a[1]);
    if (sorted.length < 3) return null;

    // Top 3 authors' share
    const top3Share = (sorted[0][1] + sorted[1][1] + sorted[2][1]) / total;

    if (top3Share >= THRESHOLDS.influenceConcentration) {
      return {
        type: SocialPatternType.INFLUENCE_CONCENTRATION,
        significance: top3Share > 0.8 ? SignificanceLevel.HIGH : SignificanceLevel.MEDIUM,
        data: {
          topAuthors: sorted.slice(0, 3).map(([author, count]) => ({ author, share: count / total })),
          concentration: top3Share,
        },
        confidence: Math.min(top3Share, PHI_INV),
        message: `Top 3 authors control ${(top3Share * 100).toFixed(0)}% of activity`,
      };
    }

    return null;
  }

  _detectActivitySurge() {
    if (this._captures.length < THRESHOLDS.minDataPoints * 3) return null;

    // Compare last 1/4 vs middle 2/4
    const quarter = Math.floor(this._captures.length / 4);
    const middle = this._captures.slice(quarter, quarter * 3);
    const recent = this._captures.slice(quarter * 3);

    const middleRate = middle.length;
    const recentRate = recent.length * 2; // Normalize (recent is half the window of middle)

    if (middleRate === 0) return null;
    const ratio = recentRate / middleRate;

    if (ratio >= THRESHOLDS.surgeMultiplier) {
      return {
        type: SocialPatternType.ACTIVITY_SURGE,
        significance: ratio > 3 ? SignificanceLevel.CRITICAL : SignificanceLevel.HIGH,
        data: { middleRate, recentRate, ratio },
        confidence: Math.min(ratio / 4, PHI_INV),
        message: `Activity surge: ${ratio.toFixed(1)}x baseline rate`,
      };
    }

    this._baselines.capturesPerWindow = middleRate;
    return null;
  }

  _detectSocialSilence() {
    if (this._captures.length === 0) return null;

    const now = Date.now();
    const lastCapture = this._captures[this._captures.length - 1];
    const silenceMs = now - lastCapture.timestamp;
    const silenceMinutes = silenceMs / (60 * 1000);

    // If no captures in 2+ hours and we had activity before
    if (silenceMinutes > 120 && this._stats.capturesRecorded > THRESHOLDS.minDataPoints) {
      return {
        type: SocialPatternType.SILENCE_DETECTED,
        significance: silenceMinutes > 360 ? SignificanceLevel.HIGH : SignificanceLevel.MEDIUM,
        data: { silenceMinutes: Math.round(silenceMinutes), lastCapture: lastCapture.timestamp },
        confidence: PHI_INV_2,
        message: `Social silence: ${Math.round(silenceMinutes)}min since last capture`,
      };
    }

    return null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Pattern Management
  // ═══════════════════════════════════════════════════════════════════════════

  _registerPattern(pattern) {
    pattern.timestamp = Date.now();
    pattern.cell = 'C4.7';
    pattern.dimension = 'SOCIAL';
    pattern.analysis = 'EMERGE';

    this._patterns.push(pattern);
    if (this._patterns.length > this._maxPatterns) {
      this._patterns.shift();
    }
    this._stats.patternsDetected++;

    this.emit('pattern_detected', pattern);
    log.info('Pattern detected', { type: pattern.type, significance: pattern.significance });
  }

  getPatterns(limit = 20) {
    return this._patterns.slice(-limit);
  }

  getStats() {
    return { ...this._stats, baselines: { ...this._baselines } };
  }

  getHealth() {
    return {
      healthy: true,
      dataPoints: {
        captures: this._captures.length,
        authors: this._authors.length,
      },
      patternsDetected: this._stats.patternsDetected,
      lastAnalysis: this._stats.lastAnalysis,
    };
  }

  clear() {
    this._captures = [];
    this._authors = [];
    this._patterns = [];
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Singleton
// ═══════════════════════════════════════════════════════════════════════════

let _instance = null;

export function getSocialEmergence(options = {}) {
  if (!_instance) {
    _instance = new SocialEmergence(options);
  }
  return _instance;
}

export function resetSocialEmergence() {
  if (_instance) {
    _instance.removeAllListeners();
  }
  _instance = null;
}
