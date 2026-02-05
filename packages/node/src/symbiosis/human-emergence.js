/**
 * Human Emergence - C5.7 (HUMAN × EMERGE)
 *
 * Detects emergent patterns in human behavior over time.
 * Part of the 7×7 Fractal Matrix symbiosis layer.
 *
 * "Le chien voit la trajectoire, pas juste le moment" - κυνικός
 *
 * Detects:
 * - Skill growth trajectories
 * - Burnout warning patterns
 * - Productivity cycles (weekly, monthly)
 * - Learning velocity changes
 * - Interest drift (domains of focus)
 *
 * @module @cynic/node/symbiosis/human-emergence
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

/**
 * Emergent pattern types
 */
export const HumanPatternType = {
  // Growth patterns
  SKILL_GROWTH: 'skill_growth',
  LEARNING_ACCELERATION: 'learning_acceleration',
  EXPERTISE_PLATEAU: 'expertise_plateau',

  // Risk patterns
  BURNOUT_RISK: 'burnout_risk',
  OVERWORK_PATTERN: 'overwork_pattern',
  DECLINING_ENGAGEMENT: 'declining_engagement',

  // Cycle patterns
  PRODUCTIVITY_CYCLE: 'productivity_cycle',
  ENERGY_RHYTHM: 'energy_rhythm',
  WEEKLY_PATTERN: 'weekly_pattern',

  // Drift patterns
  INTEREST_SHIFT: 'interest_shift',
  TOOL_PREFERENCE_CHANGE: 'tool_preference_change',
  STYLE_EVOLUTION: 'style_evolution',
};

/**
 * Pattern significance levels (φ-aligned)
 */
export const SignificanceLevel = {
  HIGH: { level: 3, threshold: PHI_INV, label: 'High' },
  MEDIUM: { level: 2, threshold: PHI_INV_2, label: 'Medium' },
  LOW: { level: 1, threshold: PHI_INV_3, label: 'Low' },
  NOISE: { level: 0, threshold: 0, label: 'Noise' },
};

/**
 * Configuration
 */
const EMERGENCE_CONFIG = {
  // Minimum days of data before detecting patterns
  minDataDays: 7,

  // Window sizes for analysis
  shortWindowDays: 7,
  mediumWindowDays: 30,
  longWindowDays: 90,

  // Trend thresholds
  growthThreshold: 0.1,      // 10% improvement
  declineThreshold: -0.15,   // 15% decline

  // Burnout risk factors
  burnoutOverworkHours: 10,  // Hours per day threshold
  burnoutConsecutiveDays: 5, // Days in a row

  // Max patterns to track
  maxPatterns: 100,
};

/**
 * Human Emergence Detector
 */
export class HumanEmergence extends EventEmitter {
  /**
   * @param {Object} options
   * @param {Object} [options.humanLearning] - HumanLearning instance
   * @param {Object} [options.humanAccountant] - HumanAccountant instance
   */
  constructor(options = {}) {
    super();

    this._humanLearning = options.humanLearning || null;
    this._humanAccountant = options.humanAccountant || null;

    // Detected patterns
    this._patterns = [];

    // Time series data for analysis
    this._dailySnapshots = [];
    this._maxSnapshots = 365; // One year of daily data

    // Statistics
    this._stats = {
      patternsDetected: 0,
      analysesRun: 0,
      lastAnalysis: null,
      activeWarnings: 0,
    };
  }

  /**
   * Set dependencies
   */
  setDependencies({ humanLearning, humanAccountant }) {
    if (humanLearning) this._humanLearning = humanLearning;
    if (humanAccountant) this._humanAccountant = humanAccountant;
  }

  /**
   * Record a daily snapshot for emergence analysis
   *
   * @param {Object} snapshot - Daily metrics
   */
  recordDailySnapshot(snapshot) {
    const entry = {
      date: snapshot.date || new Date().toISOString().split('T')[0],
      timestamp: Date.now(),
      ...snapshot,
    };

    this._dailySnapshots.push(entry);

    // Trim old snapshots
    while (this._dailySnapshots.length > this._maxSnapshots) {
      this._dailySnapshots.shift();
    }

    // Auto-analyze if enough data
    if (this._dailySnapshots.length >= EMERGENCE_CONFIG.minDataDays) {
      this._detectPatterns();
    }
  }

  /**
   * Run emergence analysis
   *
   * @returns {Object} Analysis results
   */
  analyze() {
    this._stats.analysesRun++;
    this._stats.lastAnalysis = Date.now();

    // Collect current data from dependencies
    this._collectCurrentData();

    // Detect patterns
    const newPatterns = this._detectPatterns();

    // Calculate trajectories
    const trajectories = this._calculateTrajectories();

    // Identify risks
    const risks = this._identifyRisks();

    return {
      timestamp: Date.now(),
      dataPointsAnalyzed: this._dailySnapshots.length,
      newPatterns: newPatterns.length,
      trajectories,
      risks,
      activePatterns: this.getActivePatterns(),
    };
  }

  /**
   * Collect current data from dependencies
   * @private
   */
  _collectCurrentData() {
    // Get daily summary from accountant
    if (this._humanAccountant) {
      const daily = this._humanAccountant.getDailySummary();
      if (daily && daily.sessions > 0) {
        this.recordDailySnapshot({
          date: daily.date,
          sessions: daily.sessions,
          totalMinutes: daily.totalTimeMinutes,
          productiveMinutes: daily.productiveTimeMinutes,
          productivityRatio: daily.productivityRatio,
          tasksCompleted: daily.tasksCompleted,
          taskCompletionRate: daily.taskCompletionRate,
        });
      }
    }
  }

  /**
   * Detect emergent patterns
   * @private
   */
  _detectPatterns() {
    const newPatterns = [];

    if (this._dailySnapshots.length < EMERGENCE_CONFIG.minDataDays) {
      return newPatterns;
    }

    // Detect burnout risk
    const burnout = this._detectBurnoutRisk();
    if (burnout) newPatterns.push(burnout);

    // Detect productivity cycles
    const productivityCycle = this._detectProductivityCycle();
    if (productivityCycle) newPatterns.push(productivityCycle);

    // Detect skill growth
    const skillGrowth = this._detectSkillGrowth();
    if (skillGrowth) newPatterns.push(skillGrowth);

    // Detect declining engagement
    const declining = this._detectDecliningEngagement();
    if (declining) newPatterns.push(declining);

    // Store new patterns
    for (const pattern of newPatterns) {
      this._addPattern(pattern);
    }

    return newPatterns;
  }

  /**
   * Detect burnout risk
   * @private
   */
  _detectBurnoutRisk() {
    const recentDays = this._dailySnapshots.slice(-EMERGENCE_CONFIG.shortWindowDays);
    if (recentDays.length < 5) return null;

    // Check for overwork pattern
    let consecutiveOverwork = 0;
    let maxConsecutive = 0;

    for (const day of recentDays) {
      const hours = (day.totalMinutes || 0) / 60;
      if (hours > EMERGENCE_CONFIG.burnoutOverworkHours) {
        consecutiveOverwork++;
        maxConsecutive = Math.max(maxConsecutive, consecutiveOverwork);
      } else {
        consecutiveOverwork = 0;
      }
    }

    // Check for declining productivity despite high hours
    const avgProductivity = recentDays.reduce((s, d) => s + (d.productivityRatio || 0), 0) / recentDays.length;
    const avgHours = recentDays.reduce((s, d) => s + ((d.totalMinutes || 0) / 60), 0) / recentDays.length;

    const highHoursLowProductivity = avgHours > 6 && avgProductivity < PHI_INV_2;

    if (maxConsecutive >= EMERGENCE_CONFIG.burnoutConsecutiveDays || highHoursLowProductivity) {
      const confidence = Math.min(PHI_INV,
        (maxConsecutive / EMERGENCE_CONFIG.burnoutConsecutiveDays) * 0.5 +
        (highHoursLowProductivity ? 0.3 : 0)
      );

      return {
        type: HumanPatternType.BURNOUT_RISK,
        significance: confidence >= PHI_INV_2 ? SignificanceLevel.HIGH : SignificanceLevel.MEDIUM,
        confidence,
        evidence: {
          consecutiveOverworkDays: maxConsecutive,
          averageHoursPerDay: avgHours.toFixed(1),
          averageProductivity: (avgProductivity * 100).toFixed(1) + '%',
          highHoursLowProductivity,
        },
        recommendation: 'Consider taking a break. Productivity declines with sustained overwork.',
        detectedAt: Date.now(),
      };
    }

    return null;
  }

  /**
   * Detect productivity cycles
   * @private
   */
  _detectProductivityCycle() {
    if (this._dailySnapshots.length < 14) return null;

    // Group by day of week
    const dayOfWeekStats = {};
    for (let i = 0; i < 7; i++) {
      dayOfWeekStats[i] = { total: 0, count: 0 };
    }

    for (const snapshot of this._dailySnapshots) {
      const date = new Date(snapshot.date);
      const dayOfWeek = date.getDay();
      dayOfWeekStats[dayOfWeek].total += snapshot.productivityRatio || 0;
      dayOfWeekStats[dayOfWeek].count++;
    }

    // Find peak and trough days
    let peakDay = 0, troughDay = 0;
    let peakAvg = 0, troughAvg = 1;

    for (let i = 0; i < 7; i++) {
      if (dayOfWeekStats[i].count > 0) {
        const avg = dayOfWeekStats[i].total / dayOfWeekStats[i].count;
        if (avg > peakAvg) {
          peakAvg = avg;
          peakDay = i;
        }
        if (avg < troughAvg) {
          troughAvg = avg;
          troughDay = i;
        }
      }
    }

    const dayNames = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
    const variance = peakAvg - troughAvg;

    if (variance > 0.15) { // At least 15% difference
      return {
        type: HumanPatternType.WEEKLY_PATTERN,
        significance: variance > 0.3 ? SignificanceLevel.HIGH : SignificanceLevel.MEDIUM,
        confidence: Math.min(PHI_INV, variance),
        evidence: {
          peakDay: dayNames[peakDay],
          peakProductivity: (peakAvg * 100).toFixed(1) + '%',
          troughDay: dayNames[troughDay],
          troughProductivity: (troughAvg * 100).toFixed(1) + '%',
          variance: (variance * 100).toFixed(1) + '%',
        },
        recommendation: `Schedule important work for ${dayNames[peakDay]}. ${dayNames[troughDay]} may be better for routine tasks.`,
        detectedAt: Date.now(),
      };
    }

    return null;
  }

  /**
   * Detect skill growth trend
   * @private
   */
  _detectSkillGrowth() {
    if (this._dailySnapshots.length < EMERGENCE_CONFIG.mediumWindowDays) return null;

    // Compare first half vs second half of recent data
    const half = Math.floor(this._dailySnapshots.length / 2);
    const firstHalf = this._dailySnapshots.slice(0, half);
    const secondHalf = this._dailySnapshots.slice(half);

    const firstAvg = firstHalf.reduce((s, d) => s + (d.taskCompletionRate || 0), 0) / firstHalf.length;
    const secondAvg = secondHalf.reduce((s, d) => s + (d.taskCompletionRate || 0), 0) / secondHalf.length;

    const growth = secondAvg - firstAvg;

    if (growth > EMERGENCE_CONFIG.growthThreshold) {
      return {
        type: HumanPatternType.SKILL_GROWTH,
        significance: growth > 0.2 ? SignificanceLevel.HIGH : SignificanceLevel.MEDIUM,
        confidence: Math.min(PHI_INV, growth * 2),
        evidence: {
          earlierPeriodCompletion: (firstAvg * 100).toFixed(1) + '%',
          recentPeriodCompletion: (secondAvg * 100).toFixed(1) + '%',
          improvement: (growth * 100).toFixed(1) + '%',
          dataPoints: this._dailySnapshots.length,
        },
        recommendation: 'Positive trajectory detected. Consider taking on more challenging tasks.',
        detectedAt: Date.now(),
      };
    }

    return null;
  }

  /**
   * Detect declining engagement
   * @private
   */
  _detectDecliningEngagement() {
    if (this._dailySnapshots.length < EMERGENCE_CONFIG.shortWindowDays * 2) return null;

    const recentWeek = this._dailySnapshots.slice(-7);
    const previousWeek = this._dailySnapshots.slice(-14, -7);

    if (previousWeek.length < 5 || recentWeek.length < 5) return null;

    const recentAvg = recentWeek.reduce((s, d) => s + (d.sessions || 0), 0) / recentWeek.length;
    const previousAvg = previousWeek.reduce((s, d) => s + (d.sessions || 0), 0) / previousWeek.length;

    const change = previousAvg > 0 ? (recentAvg - previousAvg) / previousAvg : 0;

    if (change < EMERGENCE_CONFIG.declineThreshold) {
      return {
        type: HumanPatternType.DECLINING_ENGAGEMENT,
        significance: change < -0.3 ? SignificanceLevel.HIGH : SignificanceLevel.MEDIUM,
        confidence: Math.min(PHI_INV, Math.abs(change)),
        evidence: {
          previousWeekSessions: previousAvg.toFixed(1),
          recentWeekSessions: recentAvg.toFixed(1),
          decline: (Math.abs(change) * 100).toFixed(1) + '%',
        },
        recommendation: 'Engagement declining. Consider taking a break or changing focus area.',
        detectedAt: Date.now(),
      };
    }

    return null;
  }

  /**
   * Calculate trajectories
   * @private
   */
  _calculateTrajectories() {
    if (this._dailySnapshots.length < 7) {
      return { available: false, reason: 'Insufficient data' };
    }

    const recent = this._dailySnapshots.slice(-7);
    const avgProductivity = recent.reduce((s, d) => s + (d.productivityRatio || 0), 0) / recent.length;
    const avgTasks = recent.reduce((s, d) => s + (d.tasksCompleted || 0), 0) / recent.length;

    // Calculate trend
    const older = this._dailySnapshots.slice(-14, -7);
    let productivityTrend = 'stable';
    if (older.length >= 5) {
      const olderAvg = older.reduce((s, d) => s + (d.productivityRatio || 0), 0) / older.length;
      const diff = avgProductivity - olderAvg;
      if (diff > 0.05) productivityTrend = 'improving';
      else if (diff < -0.05) productivityTrend = 'declining';
    }

    return {
      available: true,
      productivity: {
        current: (avgProductivity * 100).toFixed(1) + '%',
        trend: productivityTrend,
      },
      output: {
        avgTasksPerDay: avgTasks.toFixed(1),
      },
      dataPoints: this._dailySnapshots.length,
    };
  }

  /**
   * Identify current risks
   * @private
   */
  _identifyRisks() {
    const risks = [];

    // Check for active burnout risk pattern
    const burnoutPattern = this._patterns.find(p =>
      p.type === HumanPatternType.BURNOUT_RISK &&
      Date.now() - p.detectedAt < 7 * 24 * 60 * 60 * 1000 // Within last week
    );

    if (burnoutPattern) {
      risks.push({
        type: 'burnout',
        severity: burnoutPattern.significance.label,
        pattern: burnoutPattern,
      });
    }

    // Check for declining engagement
    const decliningPattern = this._patterns.find(p =>
      p.type === HumanPatternType.DECLINING_ENGAGEMENT &&
      Date.now() - p.detectedAt < 7 * 24 * 60 * 60 * 1000
    );

    if (decliningPattern) {
      risks.push({
        type: 'disengagement',
        severity: decliningPattern.significance.label,
        pattern: decliningPattern,
      });
    }

    this._stats.activeWarnings = risks.filter(r => r.severity === 'High').length;

    return risks;
  }

  /**
   * Add pattern to tracked list
   * @private
   */
  _addPattern(pattern) {
    // Check for duplicate recent patterns
    const isDuplicate = this._patterns.some(p =>
      p.type === pattern.type &&
      Date.now() - p.detectedAt < 24 * 60 * 60 * 1000 // Within 24 hours
    );

    if (!isDuplicate) {
      this._patterns.push(pattern);
      this._stats.patternsDetected++;

      // Trim old patterns
      while (this._patterns.length > EMERGENCE_CONFIG.maxPatterns) {
        this._patterns.shift();
      }

      this.emit('pattern_detected', pattern);
    }
  }

  /**
   * Get active patterns (recent and significant)
   */
  getActivePatterns() {
    const cutoff = Date.now() - 30 * 24 * 60 * 60 * 1000; // Last 30 days

    return this._patterns
      .filter(p => p.detectedAt >= cutoff)
      .sort((a, b) => b.detectedAt - a.detectedAt);
  }

  /**
   * Get statistics
   */
  getStats() {
    return {
      ...this._stats,
      totalPatterns: this._patterns.length,
      dataPointsStored: this._dailySnapshots.length,
    };
  }

  /**
   * Get summary
   */
  getSummary() {
    const analysis = this.analyze();

    return {
      trajectories: analysis.trajectories,
      risks: analysis.risks,
      activePatterns: analysis.activePatterns.slice(0, 5),
      stats: this.getStats(),
    };
  }

  /**
   * Clear all data
   */
  clear() {
    this._patterns = [];
    this._dailySnapshots = [];
    this._stats = {
      patternsDetected: 0,
      analysesRun: 0,
      lastAnalysis: null,
      activeWarnings: 0,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create HumanEmergence singleton
 */
export function getHumanEmergence(options = {}) {
  if (!_instance) {
    _instance = new HumanEmergence(options);
  } else if (options.humanLearning || options.humanAccountant) {
    _instance.setDependencies(options);
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
export function resetHumanEmergence() {
  if (_instance) {
    _instance.removeAllListeners();
  }
  _instance = null;
}

export default {
  HumanEmergence,
  HumanPatternType,
  SignificanceLevel,
  getHumanEmergence,
  resetHumanEmergence,
};
