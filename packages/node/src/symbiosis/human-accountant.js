/**
 * Human Accountant - C5.6 (HUMAN × ACCOUNT)
 *
 * Tracks human productivity economics - time invested, output produced, efficiency.
 * Part of the 7×7 Fractal Matrix symbiosis layer.
 *
 * "Le chien compte les heures de son maître" - κυνικός
 *
 * Tracks:
 * - Time invested per session
 * - Tasks completed vs attempted
 * - Tool usage patterns
 * - Energy expenditure (correlates with psychology)
 * - ROI per activity type
 *
 * @module @cynic/node/symbiosis/human-accountant
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

/**
 * Activity types for accounting
 */
export const ActivityType = {
  CODING: 'coding',
  DEBUGGING: 'debugging',
  RESEARCH: 'research',
  PLANNING: 'planning',
  REVIEW: 'review',
  DOCUMENTATION: 'documentation',
  COMMUNICATION: 'communication',
  IDLE: 'idle',
  BREAK: 'break',
};

/**
 * Productivity thresholds (φ-aligned)
 */
const PRODUCTIVITY_THRESHOLDS = {
  excellent: PHI_INV,      // > 61.8%
  good: PHI_INV_2,         // > 38.2%
  acceptable: PHI_INV_3,   // > 23.6%
  poor: 0,                 // <= 23.6%
};

/**
 * Human Accountant
 */
export class HumanAccountant extends EventEmitter {
  /**
   * @param {Object} options
   * @param {string} [options.userId] - User identifier
   */
  constructor(options = {}) {
    super();

    this._userId = options.userId || 'default';

    // Current session tracking
    this._currentSession = null;

    // Activity tracking
    this._activityHistory = [];
    this._maxHistory = 1000;

    // Aggregated metrics
    this._dailyMetrics = new Map(); // date -> metrics
    this._activityMetrics = new Map(); // activity type -> metrics

    for (const activity of Object.values(ActivityType)) {
      this._activityMetrics.set(activity, {
        totalTimeMs: 0,
        sessionCount: 0,
        completedTasks: 0,
        attemptedTasks: 0,
        averageEfficiency: 0,
      });
    }

    // Session totals
    this._sessionTotals = {
      sessionsStarted: 0,
      sessionsCompleted: 0,
      totalTimeMs: 0,
      totalTasksCompleted: 0,
      totalTasksAttempted: 0,
    };
  }

  /**
   * Start tracking a session
   *
   * @param {Object} [metadata] - Session metadata
   * @returns {Object} Session info
   */
  startSession(metadata = {}) {
    const sessionId = `session_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

    this._currentSession = {
      id: sessionId,
      startTime: Date.now(),
      endTime: null,
      activities: [],
      currentActivity: null,
      tasksCompleted: 0,
      tasksAttempted: 0,
      toolsUsed: new Map(),
      metadata,
    };

    this._sessionTotals.sessionsStarted++;

    this.emit('session_started', { sessionId, startTime: this._currentSession.startTime });

    return {
      sessionId,
      startTime: this._currentSession.startTime,
    };
  }

  /**
   * End current session
   *
   * @returns {Object} Session summary
   */
  endSession() {
    if (!this._currentSession) {
      return { success: false, error: 'No active session' };
    }

    // End any current activity
    if (this._currentSession.currentActivity) {
      this.endActivity();
    }

    this._currentSession.endTime = Date.now();
    const duration = this._currentSession.endTime - this._currentSession.startTime;

    // Calculate session metrics
    const metrics = this._calculateSessionMetrics();

    // Update daily metrics
    this._updateDailyMetrics(metrics);

    // Update session totals
    this._sessionTotals.sessionsCompleted++;
    this._sessionTotals.totalTimeMs += duration;
    this._sessionTotals.totalTasksCompleted += this._currentSession.tasksCompleted;
    this._sessionTotals.totalTasksAttempted += this._currentSession.tasksAttempted;

    const summary = {
      sessionId: this._currentSession.id,
      duration,
      durationMinutes: Math.round(duration / 60000),
      ...metrics,
    };

    this.emit('session_ended', summary);

    this._currentSession = null;

    return summary;
  }

  /**
   * Start tracking an activity
   *
   * @param {string} activityType - ActivityType
   * @param {Object} [metadata] - Activity metadata
   */
  startActivity(activityType, metadata = {}) {
    if (!this._currentSession) {
      this.startSession();
    }

    // End previous activity
    if (this._currentSession.currentActivity) {
      this.endActivity();
    }

    this._currentSession.currentActivity = {
      type: activityType,
      startTime: Date.now(),
      endTime: null,
      metadata,
    };

    this.emit('activity_started', {
      sessionId: this._currentSession.id,
      activityType,
      startTime: this._currentSession.currentActivity.startTime,
    });
  }

  /**
   * End current activity
   *
   * @param {Object} [outcome] - Activity outcome
   * @returns {Object} Activity summary
   */
  endActivity(outcome = {}) {
    if (!this._currentSession?.currentActivity) {
      return null;
    }

    const activity = this._currentSession.currentActivity;
    activity.endTime = Date.now();
    activity.duration = activity.endTime - activity.startTime;
    activity.outcome = outcome;

    // Store in session
    this._currentSession.activities.push(activity);

    // Store in history
    this._activityHistory.push({
      ...activity,
      sessionId: this._currentSession.id,
    });

    // Trim history
    while (this._activityHistory.length > this._maxHistory) {
      this._activityHistory.shift();
    }

    // Update activity metrics
    const activityMetrics = this._activityMetrics.get(activity.type);
    if (activityMetrics) {
      activityMetrics.totalTimeMs += activity.duration;
      activityMetrics.sessionCount++;
      if (outcome.completed) activityMetrics.completedTasks++;
      if (outcome.attempted) activityMetrics.attemptedTasks++;
    }

    this._currentSession.currentActivity = null;

    const summary = {
      type: activity.type,
      duration: activity.duration,
      durationMinutes: Math.round(activity.duration / 60000),
      outcome,
    };

    this.emit('activity_ended', summary);

    return summary;
  }

  /**
   * Record a task attempt
   *
   * @param {boolean} completed - Was task completed?
   * @param {Object} [details] - Task details
   */
  recordTask(completed, details = {}) {
    if (!this._currentSession) {
      this.startSession();
    }

    this._currentSession.tasksAttempted++;
    if (completed) {
      this._currentSession.tasksCompleted++;
    }

    this.emit('task_recorded', {
      sessionId: this._currentSession.id,
      completed,
      details,
    });
  }

  /**
   * Record tool usage
   *
   * @param {string} toolName - Name of tool used
   * @param {number} [duration] - Duration in ms
   */
  recordToolUsage(toolName, duration = 0) {
    if (!this._currentSession) {
      this.startSession();
    }

    const current = this._currentSession.toolsUsed.get(toolName) || { count: 0, totalDuration: 0 };
    current.count++;
    current.totalDuration += duration;
    this._currentSession.toolsUsed.set(toolName, current);
  }

  /**
   * Calculate session metrics
   * @private
   */
  _calculateSessionMetrics() {
    const session = this._currentSession;
    const duration = (session.endTime || Date.now()) - session.startTime;

    // Calculate time per activity type
    const timeByActivity = {};
    for (const activity of session.activities) {
      timeByActivity[activity.type] = (timeByActivity[activity.type] || 0) + activity.duration;
    }

    // Calculate task completion rate
    const taskCompletionRate = session.tasksAttempted > 0
      ? session.tasksCompleted / session.tasksAttempted
      : 0;

    // Calculate productive time (exclude idle and break)
    const productiveTime = Object.entries(timeByActivity)
      .filter(([type]) => type !== ActivityType.IDLE && type !== ActivityType.BREAK)
      .reduce((sum, [, time]) => sum + time, 0);

    const productivityRatio = duration > 0 ? productiveTime / duration : 0;

    // Rate productivity
    let productivityRating;
    if (productivityRatio >= PRODUCTIVITY_THRESHOLDS.excellent) {
      productivityRating = 'excellent';
    } else if (productivityRatio >= PRODUCTIVITY_THRESHOLDS.good) {
      productivityRating = 'good';
    } else if (productivityRatio >= PRODUCTIVITY_THRESHOLDS.acceptable) {
      productivityRating = 'acceptable';
    } else {
      productivityRating = 'poor';
    }

    // Top tools
    const topTools = Array.from(session.toolsUsed.entries())
      .map(([name, data]) => ({ name, ...data }))
      .sort((a, b) => b.count - a.count)
      .slice(0, 5);

    return {
      tasksCompleted: session.tasksCompleted,
      tasksAttempted: session.tasksAttempted,
      taskCompletionRate,
      productiveTime,
      productivityRatio,
      productivityRating,
      timeByActivity,
      topTools,
      activitiesCount: session.activities.length,
    };
  }

  /**
   * Update daily metrics
   * @private
   */
  _updateDailyMetrics(sessionMetrics) {
    const dateKey = new Date().toISOString().split('T')[0];

    const daily = this._dailyMetrics.get(dateKey) || {
      date: dateKey,
      sessions: 0,
      totalTimeMs: 0,
      productiveTimeMs: 0,
      tasksCompleted: 0,
      tasksAttempted: 0,
    };

    daily.sessions++;
    daily.totalTimeMs += this._currentSession.endTime - this._currentSession.startTime;
    daily.productiveTimeMs += sessionMetrics.productiveTime;
    daily.tasksCompleted += sessionMetrics.tasksCompleted;
    daily.tasksAttempted += sessionMetrics.tasksAttempted;

    this._dailyMetrics.set(dateKey, daily);
  }

  /**
   * Get daily summary
   *
   * @param {string} [date] - Date string (YYYY-MM-DD), defaults to today
   * @returns {Object} Daily metrics
   */
  getDailySummary(date = null) {
    const dateKey = date || new Date().toISOString().split('T')[0];
    const daily = this._dailyMetrics.get(dateKey);

    if (!daily) {
      return {
        date: dateKey,
        sessions: 0,
        totalTimeMinutes: 0,
        productiveTimeMinutes: 0,
        productivityRatio: 0,
        tasksCompleted: 0,
        taskCompletionRate: 0,
      };
    }

    return {
      date: dateKey,
      sessions: daily.sessions,
      totalTimeMinutes: Math.round(daily.totalTimeMs / 60000),
      productiveTimeMinutes: Math.round(daily.productiveTimeMs / 60000),
      productivityRatio: daily.totalTimeMs > 0
        ? daily.productiveTimeMs / daily.totalTimeMs
        : 0,
      tasksCompleted: daily.tasksCompleted,
      taskCompletionRate: daily.tasksAttempted > 0
        ? daily.tasksCompleted / daily.tasksAttempted
        : 0,
    };
  }

  /**
   * Get activity breakdown
   */
  getActivityBreakdown() {
    const breakdown = [];

    for (const [type, metrics] of this._activityMetrics) {
      if (metrics.totalTimeMs > 0) {
        breakdown.push({
          type,
          totalTimeMinutes: Math.round(metrics.totalTimeMs / 60000),
          sessionCount: metrics.sessionCount,
          completedTasks: metrics.completedTasks,
          attemptedTasks: metrics.attemptedTasks,
          completionRate: metrics.attemptedTasks > 0
            ? metrics.completedTasks / metrics.attemptedTasks
            : 0,
        });
      }
    }

    return breakdown.sort((a, b) => b.totalTimeMinutes - a.totalTimeMinutes);
  }

  /**
   * Get overall statistics
   */
  getStats() {
    const avgSessionDuration = this._sessionTotals.sessionsCompleted > 0
      ? this._sessionTotals.totalTimeMs / this._sessionTotals.sessionsCompleted
      : 0;

    return {
      ...this._sessionTotals,
      totalTimeHours: this._sessionTotals.totalTimeMs / (60 * 60 * 1000),
      avgSessionMinutes: Math.round(avgSessionDuration / 60000),
      overallCompletionRate: this._sessionTotals.totalTasksAttempted > 0
        ? this._sessionTotals.totalTasksCompleted / this._sessionTotals.totalTasksAttempted
        : 0,
      daysTracked: this._dailyMetrics.size,
      activitiesTracked: this._activityHistory.length,
    };
  }

  /**
   * Get current session info
   */
  getCurrentSession() {
    if (!this._currentSession) {
      return null;
    }

    const elapsed = Date.now() - this._currentSession.startTime;

    return {
      sessionId: this._currentSession.id,
      startTime: this._currentSession.startTime,
      elapsedMinutes: Math.round(elapsed / 60000),
      currentActivity: this._currentSession.currentActivity?.type || null,
      tasksCompleted: this._currentSession.tasksCompleted,
      tasksAttempted: this._currentSession.tasksAttempted,
      activitiesCount: this._currentSession.activities.length,
    };
  }

  /**
   * Clear all data
   */
  clear() {
    this._currentSession = null;
    this._activityHistory = [];
    this._dailyMetrics.clear();

    for (const [, metrics] of this._activityMetrics) {
      metrics.totalTimeMs = 0;
      metrics.sessionCount = 0;
      metrics.completedTasks = 0;
      metrics.attemptedTasks = 0;
      metrics.averageEfficiency = 0;
    }

    this._sessionTotals = {
      sessionsStarted: 0,
      sessionsCompleted: 0,
      totalTimeMs: 0,
      totalTasksCompleted: 0,
      totalTasksAttempted: 0,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create HumanAccountant singleton
 */
export function getHumanAccountant(options = {}) {
  if (!_instance) {
    _instance = new HumanAccountant(options);
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
export function resetHumanAccountant() {
  if (_instance) {
    _instance.removeAllListeners();
  }
  _instance = null;
}

export default {
  HumanAccountant,
  ActivityType,
  getHumanAccountant,
  resetHumanAccountant,
};
