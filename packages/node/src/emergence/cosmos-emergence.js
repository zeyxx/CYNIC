/**
 * Cosmos Emergence - C7.7 (COSMOS × EMERGE)
 *
 * Detects emergent patterns across the entire ecosystem.
 * Part of the 7×7 Fractal Matrix emergence layer.
 *
 * "Le chien voit les étoiles dans le code" - κυνικός
 *
 * Emerges:
 * - Cross-repo convergence (multiple repos evolving toward same pattern)
 * - Ecosystem health trajectories (overall health trend)
 * - Dependency cascade risks (breaking change propagation)
 * - Activity distribution shifts (work concentration across repos)
 * - Collective learning velocity (ecosystem-wide improvement rate)
 *
 * @module @cynic/node/emergence/cosmos-emergence
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, createLogger } from '@cynic/core';

const log = createLogger('CosmosEmergence');

/**
 * Emergent pattern types for ecosystem awareness
 */
export const CosmosPatternType = {
  HEALTH_TRAJECTORY: 'ecosystem_health_trajectory',
  ACTIVITY_CONCENTRATION: 'activity_concentration',
  DEPENDENCY_CASCADE: 'dependency_cascade_risk',
  CROSS_REPO_CONVERGENCE: 'cross_repo_convergence',
  LEARNING_VELOCITY: 'ecosystem_learning_velocity',
  ECOSYSTEM_SILENCE: 'ecosystem_silence',
};

export const SignificanceLevel = {
  LOW: 'low',
  MEDIUM: 'medium',
  HIGH: 'high',
  CRITICAL: 'critical',
};

const THRESHOLDS = {
  healthDeclineRate: -0.15,          // 15% decline = ecosystem health concern
  activityConcentration: PHI_INV,    // 61.8% in one repo = concentration risk
  silenceHours: 6,                   // 6+ hours without cross-repo activity = silence
  minDataPoints: 3,                  // Minimum data before detecting patterns
  maxHistory: 300,                   // Rolling window size
};

/**
 * CosmosEmergence - Ecosystem-wide pattern emergence detector
 */
export class CosmosEmergence extends EventEmitter {
  constructor(options = {}) {
    super();

    // Rolling history windows
    this._healthSnapshots = []; // { avgHealth, repoCount, totalIssues, timestamp }
    this._repoActivity = [];   // { repo, eventType, timestamp }
    this._crossEvents = [];    // { repos, eventType, significance, timestamp }

    // Baselines
    this._baselines = {
      avgHealth: 0.5,
      activeRepos: 0,
      repoDistribution: {},
    };

    // Detected patterns
    this._patterns = [];
    this._maxPatterns = 100;

    // Stats
    this._stats = {
      healthSnapshotsRecorded: 0,
      repoActivitiesRecorded: 0,
      crossEventsRecorded: 0,
      patternsDetected: 0,
      analysesRun: 0,
      lastAnalysis: null,
    };
  }

  /**
   * Record an ecosystem health snapshot
   */
  recordHealthSnapshot(data) {
    const entry = {
      avgHealth: data.avgHealth || data.health || 0.5,
      repoCount: data.repoCount || data.repos || 0,
      totalIssues: data.totalIssues || data.issues || 0,
      stalePRs: data.stalePRs || 0,
      timestamp: Date.now(),
    };

    this._healthSnapshots.push(entry);
    if (this._healthSnapshots.length > THRESHOLDS.maxHistory) {
      this._healthSnapshots.shift();
    }
    this._stats.healthSnapshotsRecorded++;
  }

  /**
   * Record activity in a specific repo
   */
  recordRepoActivity(data) {
    const entry = {
      repo: data.repo || data.repository || 'unknown',
      eventType: data.eventType || data.type || 'commit',
      timestamp: Date.now(),
    };

    this._repoActivity.push(entry);
    if (this._repoActivity.length > THRESHOLDS.maxHistory) {
      this._repoActivity.shift();
    }
    this._stats.repoActivitiesRecorded++;
  }

  /**
   * Record a cross-repo event (pattern detected across repos)
   */
  recordCrossEvent(data) {
    const entry = {
      repos: data.repos || [],
      eventType: data.eventType || data.type || 'convergence',
      significance: data.significance || 'low',
      timestamp: Date.now(),
    };

    this._crossEvents.push(entry);
    if (this._crossEvents.length > THRESHOLDS.maxHistory) {
      this._crossEvents.shift();
    }
    this._stats.crossEventsRecorded++;
  }

  /**
   * Run full emergence analysis
   */
  analyze() {
    this._stats.analysesRun++;
    this._stats.lastAnalysis = Date.now();

    const newPatterns = [];

    // 1. Ecosystem health trajectory
    const health = this._detectHealthTrajectory();
    if (health) newPatterns.push(health);

    // 2. Activity concentration
    const concentration = this._detectActivityConcentration();
    if (concentration) newPatterns.push(concentration);

    // 3. Cross-repo convergence
    const convergence = this._detectCrossRepoConvergence();
    if (convergence) newPatterns.push(convergence);

    // 4. Ecosystem silence
    const silence = this._detectEcosystemSilence();
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

  _detectHealthTrajectory() {
    if (this._healthSnapshots.length < THRESHOLDS.minDataPoints * 2) return null;

    const mid = Math.floor(this._healthSnapshots.length / 2);
    const older = this._healthSnapshots.slice(0, mid);
    const recent = this._healthSnapshots.slice(mid);

    const olderAvg = older.reduce((s, h) => s + h.avgHealth, 0) / older.length;
    const recentAvg = recent.reduce((s, h) => s + h.avgHealth, 0) / recent.length;
    const delta = recentAvg - olderAvg;

    if (delta < THRESHOLDS.healthDeclineRate) {
      return {
        type: CosmosPatternType.HEALTH_TRAJECTORY,
        significance: delta < -0.25 ? SignificanceLevel.CRITICAL : SignificanceLevel.HIGH,
        data: { olderAvg, recentAvg, delta, repoCount: recent[recent.length - 1]?.repoCount || 0 },
        confidence: PHI_INV_2,
        message: `Ecosystem health declining: ${(olderAvg * 100).toFixed(0)}% → ${(recentAvg * 100).toFixed(0)}%`,
      };
    }

    this._baselines.avgHealth = recentAvg;
    return null;
  }

  _detectActivityConcentration() {
    if (this._repoActivity.length < THRESHOLDS.minDataPoints) return null;

    const counts = {};
    for (const a of this._repoActivity) {
      counts[a.repo] = (counts[a.repo] || 0) + 1;
    }

    const total = this._repoActivity.length;
    const sorted = Object.entries(counts).sort((a, b) => b[1] - a[1]);
    if (sorted.length === 0) return null;

    const [topRepo, topCount] = sorted[0];
    const ratio = topCount / total;

    if (ratio >= THRESHOLDS.activityConcentration) {
      return {
        type: CosmosPatternType.ACTIVITY_CONCENTRATION,
        significance: ratio > 0.8 ? SignificanceLevel.HIGH : SignificanceLevel.MEDIUM,
        data: {
          dominantRepo: topRepo,
          ratio,
          totalRepos: sorted.length,
          distribution: Object.fromEntries(sorted.slice(0, 5)),
        },
        confidence: Math.min(ratio, PHI_INV),
        message: `${topRepo} dominates ${(ratio * 100).toFixed(0)}% of ecosystem activity`,
      };
    }

    // Update baseline
    for (const [repo, count] of Object.entries(counts)) {
      this._baselines.repoDistribution[repo] = count / total;
    }
    this._baselines.activeRepos = sorted.length;
    return null;
  }

  _detectCrossRepoConvergence() {
    if (this._crossEvents.length < THRESHOLDS.minDataPoints) return null;

    // Count how many cross-repo events are HIGH/CRITICAL
    const significant = this._crossEvents.filter(
      e => e.significance === 'high' || e.significance === 'critical'
    );

    if (significant.length >= 3) {
      // Find common repos
      const repoCounts = {};
      for (const e of significant) {
        for (const repo of e.repos) {
          repoCounts[repo] = (repoCounts[repo] || 0) + 1;
        }
      }

      return {
        type: CosmosPatternType.CROSS_REPO_CONVERGENCE,
        significance: significant.length > 5 ? SignificanceLevel.HIGH : SignificanceLevel.MEDIUM,
        data: {
          significantEvents: significant.length,
          repos: repoCounts,
        },
        confidence: Math.min(significant.length / 10, PHI_INV),
        message: `Cross-repo convergence: ${significant.length} significant events across ecosystem`,
      };
    }

    return null;
  }

  _detectEcosystemSilence() {
    if (this._repoActivity.length === 0) return null;

    const now = Date.now();
    const lastActivity = this._repoActivity[this._repoActivity.length - 1];
    const silenceMs = now - lastActivity.timestamp;
    const silenceHours = silenceMs / (60 * 60 * 1000);

    if (silenceHours > THRESHOLDS.silenceHours && this._stats.repoActivitiesRecorded > THRESHOLDS.minDataPoints) {
      return {
        type: CosmosPatternType.ECOSYSTEM_SILENCE,
        significance: silenceHours > 24 ? SignificanceLevel.HIGH : SignificanceLevel.MEDIUM,
        data: { silenceHours: Math.round(silenceHours), lastActivity: lastActivity.timestamp },
        confidence: PHI_INV_2,
        message: `Ecosystem silence: ${Math.round(silenceHours)}h since last activity`,
      };
    }

    return null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Pattern Management
  // ═══════════════════════════════════════════════════════════════════════════

  _registerPattern(pattern) {
    pattern.timestamp = Date.now();
    pattern.cell = 'C7.7';
    pattern.dimension = 'COSMOS';
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
        healthSnapshots: this._healthSnapshots.length,
        repoActivity: this._repoActivity.length,
        crossEvents: this._crossEvents.length,
      },
      patternsDetected: this._stats.patternsDetected,
      lastAnalysis: this._stats.lastAnalysis,
    };
  }

  clear() {
    this._healthSnapshots = [];
    this._repoActivity = [];
    this._crossEvents = [];
    this._patterns = [];
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Singleton
// ═══════════════════════════════════════════════════════════════════════════

let _instance = null;

export function getCosmosEmergence(options = {}) {
  if (!_instance) {
    _instance = new CosmosEmergence(options);
  }
  return _instance;
}

export function resetCosmosEmergence() {
  if (_instance) {
    _instance.removeAllListeners();
  }
  _instance = null;
}
