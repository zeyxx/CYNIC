/**
 * @cynic/node - Autonomous Capabilities for Collective Dogs
 *
 * "Le chien agit sans qu'on lui demande" - κυνικός
 *
 * Provides autonomous capabilities that can be mixed into any Collective Dog:
 * - Goal tracking and progress updates
 * - Task scheduling for background processing
 * - Proactive notification generation
 * - Self-correction from lessons learned
 *
 * Integrates with Total Memory system (Phase 16).
 *
 * @module @cynic/node/agents/collective/autonomous
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

/**
 * φ-aligned constants for autonomous behavior
 */
export const AUTONOMOUS_CONSTANTS = {
  /** Fibonacci timing for checks (minutes) */
  FIBONACCI_TIMING: [1, 2, 3, 5, 8, 13, 21, 34, 55],

  /** Min confidence to act autonomously (φ⁻¹) */
  MIN_CONFIDENCE: PHI_INV,

  /** Min importance to create notification (φ⁻²) */
  MIN_NOTIFICATION_IMPORTANCE: PHI_INV_2,

  /** Max notifications per session (Fib(5)) */
  MAX_NOTIFICATIONS_PER_SESSION: 5,

  /** Goal progress threshold to notify (Fib(8) / 100) */
  PROGRESS_NOTIFY_THRESHOLD: 0.21,
};

/**
 * Autonomous goal types for each Dog
 */
export const DogGoalTypes = {
  GUARDIAN: {
    id: 'guardian',
    name: 'Shadow',
    goals: ['security_audit', 'vulnerability_scan', 'threat_monitoring'],
  },
  ARCHITECT: {
    id: 'architect',
    name: 'Max',
    goals: ['decision_tracking', 'drift_detection', 'pattern_consistency'],
  },
  JANITOR: {
    id: 'janitor',
    name: 'Ralph',
    goals: ['test_coverage', 'code_quality', 'tech_debt_reduction'],
  },
  SCHOLAR: {
    id: 'scholar',
    name: 'Archie',
    goals: ['memory_compaction', 'knowledge_synthesis', 'pattern_extraction'],
  },
  SAGE: {
    id: 'sage',
    name: 'Luna',
    goals: ['user_insights', 'learning_tracking', 'proactive_guidance'],
  },
};

/**
 * Autonomous capabilities mixin
 *
 * Add to any Collective Dog to enable autonomous behavior.
 *
 * @param {Object} repositories - Total Memory repositories
 * @param {Object} repositories.goals - AutonomousGoalsRepository
 * @param {Object} repositories.tasks - AutonomousTasksRepository
 * @param {Object} repositories.notifications - ProactiveNotificationsRepository
 * @param {Object} repositories.memory - MemoryRetriever
 * @returns {Object} Autonomous capability methods
 */
export function createAutonomousCapabilities(repositories = {}) {
  const { goals, tasks, notifications, memory } = repositories;

  // Track notifications created this session to avoid spam
  let sessionNotificationCount = 0;

  return {
    /**
     * Check if autonomous action is allowed
     * @param {number} confidence - Confidence level (0-1)
     * @returns {boolean}
     */
    canActAutonomously(confidence) {
      return confidence >= AUTONOMOUS_CONSTANTS.MIN_CONFIDENCE;
    },

    /**
     * Create or update a goal for this Dog
     * @param {string} userId - User ID
     * @param {string} dogType - Dog type (from DogGoalTypes)
     * @param {Object} goalData - Goal data
     * @returns {Promise<Object|null>} Created/updated goal
     */
    async trackGoal(userId, dogType, goalData) {
      if (!goals) return null;

      try {
        const dogConfig = DogGoalTypes[dogType.toUpperCase()];
        if (!dogConfig) return null;

        // Check if goal already exists
        const existingGoals = await goals.findByUser(userId, {
          status: 'active',
          goalType: goalData.type || dogConfig.goals[0],
        });

        const existing = existingGoals.find(g =>
          g.title === goalData.title ||
          g.goalType === goalData.type
        );

        if (existing) {
          // Update progress
          if (goalData.progress !== undefined) {
            return await goals.updateProgress(existing.id, goalData.progress);
          }
          return existing;
        }

        // Create new goal
        return await goals.create({
          userId,
          goalType: goalData.type || dogConfig.goals[0],
          title: goalData.title || `${dogConfig.name}: ${goalData.type}`,
          description: goalData.description,
          successCriteria: goalData.criteria || [],
          priority: goalData.priority || 50,
        });
      } catch (e) {
        console.error(`[${dogType}] Goal tracking failed:`, e.message);
        return null;
      }
    },

    /**
     * Schedule a background task
     * @param {string} userId - User ID
     * @param {string} taskType - Task type
     * @param {Object} payload - Task payload
     * @param {Object} options - Task options
     * @returns {Promise<Object|null>} Created task
     */
    async scheduleTask(userId, taskType, payload, options = {}) {
      if (!tasks) return null;

      try {
        return await tasks.create({
          userId,
          taskType,
          payload,
          priority: options.priority || 50,
          scheduledFor: options.scheduledFor || new Date(),
          maxRetries: options.maxRetries || 3,
        });
      } catch (e) {
        console.error(`[Autonomous] Task scheduling failed:`, e.message);
        return null;
      }
    },

    /**
     * Create a proactive notification for next session
     * @param {string} userId - User ID
     * @param {Object} notificationData - Notification data
     * @returns {Promise<Object|null>} Created notification
     */
    async notify(userId, notificationData) {
      if (!notifications) return null;

      // Limit notifications per session
      if (sessionNotificationCount >= AUTONOMOUS_CONSTANTS.MAX_NOTIFICATIONS_PER_SESSION) {
        return null;
      }

      // Check importance threshold
      const importance = notificationData.importance || 0.5;
      if (importance < AUTONOMOUS_CONSTANTS.MIN_NOTIFICATION_IMPORTANCE) {
        return null;
      }

      try {
        sessionNotificationCount++;

        return await notifications.create({
          userId,
          notificationType: notificationData.type || 'insight',
          title: notificationData.title,
          message: notificationData.message,
          priority: Math.round(importance * 100),
          context: notificationData.context,
          expiresAt: notificationData.expiresAt,
        });
      } catch (e) {
        console.error(`[Autonomous] Notification creation failed:`, e.message);
        return null;
      }
    },

    /**
     * Check for relevant lessons before taking action
     * @param {string} userId - User ID
     * @param {string} action - Proposed action description
     * @returns {Promise<Object>} Check result with warning if applicable
     */
    async checkLessons(userId, action) {
      if (!memory) return { warning: false };

      try {
        return await memory.checkForMistakes(userId, action);
      } catch (e) {
        return { warning: false };
      }
    },

    /**
     * Store a lesson learned from autonomous operation
     * @param {string} userId - User ID
     * @param {Object} lesson - Lesson data
     * @returns {Promise<Object|null>} Created lesson
     */
    async learnLesson(userId, lesson) {
      if (!memory) return null;

      try {
        return await memory.rememberLesson(userId, {
          category: lesson.category || 'process',
          mistake: lesson.mistake,
          correction: lesson.correction,
          prevention: lesson.prevention,
          severity: lesson.severity || 'low',
        });
      } catch (e) {
        console.error(`[Autonomous] Lesson storage failed:`, e.message);
        return null;
      }
    },

    /**
     * Store an architectural decision
     * @param {string} userId - User ID
     * @param {Object} decision - Decision data
     * @returns {Promise<Object|null>} Created decision
     */
    async recordDecision(userId, decision) {
      if (!memory) return null;

      try {
        return await memory.rememberDecision(userId, {
          projectPath: decision.projectPath,
          decisionType: decision.type || 'pattern',
          title: decision.title,
          description: decision.description,
          rationale: decision.rationale,
          alternatives: decision.alternatives || [],
          consequences: decision.consequences || {},
        });
      } catch (e) {
        console.error(`[Autonomous] Decision storage failed:`, e.message);
        return null;
      }
    },

    /**
     * Update goal progress based on activity
     * @param {string} userId - User ID
     * @param {string} goalId - Goal ID
     * @param {number} progressDelta - Progress change (0-1)
     * @returns {Promise<Object|null>} Updated goal
     */
    async updateGoalProgress(userId, goalId, progressDelta) {
      if (!goals) return null;

      try {
        const goal = await goals.findById(goalId);
        if (!goal) return null;

        const newProgress = Math.min((goal.progress || 0) + progressDelta, 1.0);
        const updated = await goals.updateProgress(goalId, newProgress);

        // Notify on significant progress
        if (progressDelta >= AUTONOMOUS_CONSTANTS.PROGRESS_NOTIFY_THRESHOLD) {
          await this.notify(userId, {
            type: 'achievement',
            title: `Goal Progress: ${goal.title}`,
            message: `Progress increased to ${Math.round(newProgress * 100)}%`,
            importance: 0.6,
            context: { goalId, progress: newProgress },
          });
        }

        // Check for completion
        if (newProgress >= 1.0 && goal.status === 'active') {
          await goals.complete(goalId);
          await this.notify(userId, {
            type: 'achievement',
            title: `Goal Completed: ${goal.title}`,
            message: `Congratulations! Goal has been achieved.`,
            importance: 0.9,
            context: { goalId },
          });
        }

        return updated;
      } catch (e) {
        console.error(`[Autonomous] Goal progress update failed:`, e.message);
        return null;
      }
    },

    /**
     * Reset session notification count
     */
    resetSessionNotifications() {
      sessionNotificationCount = 0;
    },

    /**
     * Get session notification count
     * @returns {number}
     */
    getSessionNotificationCount() {
      return sessionNotificationCount;
    },
  };
}

/**
 * Dog-specific autonomous behaviors
 */
export const DogAutonomousBehaviors = {
  /**
   * Guardian (Shadow) - Security monitoring
   */
  guardian: {
    async monitorSecurity(capabilities, userId, context) {
      // Check for security-related lessons before action
      const check = await capabilities.checkLessons(userId, 'security configuration');

      if (check.warning) {
        await capabilities.notify(userId, {
          type: 'warning',
          title: 'Security Pattern Detected',
          message: check.message,
          importance: 0.8,
          context: { source: 'guardian', lessons: check.lessons },
        });
      }

      // Track security goal
      await capabilities.trackGoal(userId, 'guardian', {
        type: 'security_audit',
        title: 'Continuous Security Monitoring',
        description: 'Monitor for security vulnerabilities and threats',
        priority: 70,
      });
    },

    async reportThreat(capabilities, userId, threat) {
      await capabilities.notify(userId, {
        type: 'warning',
        title: `Security Threat: ${threat.category}`,
        message: threat.description,
        importance: 0.9,
        context: { threat },
      });

      // Learn from the threat
      await capabilities.learnLesson(userId, {
        category: 'security',
        mistake: `Security issue: ${threat.category}`,
        correction: threat.mitigation || 'Review and fix the vulnerability',
        prevention: 'Regular security audits',
        severity: threat.severity || 'high',
      });
    },
  },

  /**
   * Architect (Max) - Decision tracking
   */
  architect: {
    async trackDecision(capabilities, userId, decision) {
      // Store the architectural decision
      await capabilities.recordDecision(userId, decision);

      // Update architecture goal
      await capabilities.trackGoal(userId, 'architect', {
        type: 'decision_tracking',
        title: 'Architecture Decision Tracking',
        description: 'Track and maintain architectural decisions',
        priority: 60,
      });

      // Notify about significant decisions
      if (decision.significance === 'high') {
        await capabilities.notify(userId, {
          type: 'insight',
          title: `Architecture Decision: ${decision.title}`,
          message: decision.rationale?.substring(0, 100) || 'New decision recorded',
          importance: 0.7,
          context: { decision },
        });
      }
    },

    async detectDrift(capabilities, userId, drift) {
      await capabilities.notify(userId, {
        type: 'warning',
        title: 'Architecture Drift Detected',
        message: drift.description,
        importance: 0.75,
        context: { drift },
      });

      await capabilities.learnLesson(userId, {
        category: 'architecture',
        mistake: `Architecture drift: ${drift.pattern}`,
        correction: drift.recommendation,
        prevention: 'Regular architecture reviews',
        severity: 'medium',
      });
    },
  },

  /**
   * Janitor (Ralph) - Test coverage monitoring
   */
  janitor: {
    async monitorCoverage(capabilities, userId, coverage) {
      // Track test coverage goal
      const goal = await capabilities.trackGoal(userId, 'janitor', {
        type: 'test_coverage',
        title: `Test Coverage: ${coverage.target}%`,
        description: 'Maintain and improve test coverage',
        progress: coverage.current / coverage.target,
        priority: 65,
      });

      // Notify on coverage drop
      if (coverage.delta < -0.05) {
        await capabilities.notify(userId, {
          type: 'warning',
          title: 'Test Coverage Dropped',
          message: `Coverage decreased by ${Math.abs(coverage.delta * 100).toFixed(1)}%`,
          importance: 0.7,
          context: { coverage },
        });
      }

      // Schedule test run if coverage is low
      if (coverage.current < coverage.target * PHI_INV_2) {
        await capabilities.scheduleTask(userId, 'run_tests', {
          reason: 'low_coverage',
          coverage,
        }, { priority: 60 });
      }
    },

    async reportQuality(capabilities, userId, quality) {
      if (quality.score < PHI_INV_2) {
        await capabilities.notify(userId, {
          type: 'warning',
          title: 'Code Quality Below Threshold',
          message: `Quality score: ${Math.round(quality.score * 100)}%`,
          importance: 0.65,
          context: { quality },
        });
      }
    },
  },

  /**
   * Scholar (Archie) - Memory compaction
   */
  scholar: {
    async compactMemory(capabilities, userId, stats) {
      // Track memory compaction goal
      await capabilities.trackGoal(userId, 'scholar', {
        type: 'memory_compaction',
        title: 'Memory Optimization',
        description: 'Compact and synthesize knowledge',
        priority: 40,
      });

      // Schedule compaction task if memory is large
      if (stats.totalMemories > 100) {
        await capabilities.scheduleTask(userId, 'sync', {
          action: 'compact',
          threshold: 100,
        }, { priority: 30 });
      }
    },

    async synthesizeKnowledge(capabilities, userId, synthesis) {
      await capabilities.notify(userId, {
        type: 'insight',
        title: 'Knowledge Synthesis',
        message: synthesis.summary,
        importance: 0.6,
        context: { patterns: synthesis.patterns },
      });
    },
  },

  /**
   * Sage (Luna) - User insights
   */
  sage: {
    async generateInsight(capabilities, userId, insight) {
      await capabilities.notify(userId, {
        type: 'insight',
        title: insight.title,
        message: insight.message,
        importance: insight.importance || 0.5,
        context: insight.context,
      });

      // Track learning goal
      await capabilities.trackGoal(userId, 'sage', {
        type: 'user_insights',
        title: 'Proactive User Guidance',
        description: 'Generate helpful insights and suggestions',
        priority: 50,
      });
    },

    async trackLearning(capabilities, userId, learning) {
      // Store as a positive lesson (what worked)
      if (learning.success) {
        await capabilities.recordDecision(userId, {
          type: 'pattern',
          title: `Successful Pattern: ${learning.pattern}`,
          description: learning.description,
          rationale: 'This approach worked well',
        });
      }
    },
  },
};

export default {
  AUTONOMOUS_CONSTANTS,
  DogGoalTypes,
  createAutonomousCapabilities,
  DogAutonomousBehaviors,
};
