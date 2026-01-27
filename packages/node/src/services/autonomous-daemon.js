/**
 * Autonomous Daemon Service
 *
 * Background daemon that runs autonomously, processing tasks,
 * tracking goals, performing self-correction, and generating notifications.
 *
 * Uses Fibonacci timing for checks: 1, 2, 3, 5, 8, 13, 21 minutes
 *
 * Part of CYNIC's Full Autonomy system.
 *
 * @module @cynic/node/services/autonomous-daemon
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';

const log = createLogger('AutonomousDaemon');

/**
 * Fibonacci sequence for timing (in minutes)
 */
const FIBONACCI = [1, 2, 3, 5, 8, 13, 21, 34, 55, 89];

/**
 * Task handlers registry
 */
const TASK_HANDLERS = new Map();

/**
 * Register a task handler
 * @param {string} taskType - Task type
 * @param {Function} handler - Handler function (task, context) => Promise<result>
 */
export function registerTaskHandler(taskType, handler) {
  TASK_HANDLERS.set(taskType, handler);
}

/**
 * Autonomous Daemon
 *
 * Runs in the background, processing tasks and maintaining goals.
 */
export class AutonomousDaemon {
  /**
   * Create the daemon
   *
   * @param {Object} options - Options
   * @param {Object} options.pool - PostgreSQL connection pool
   * @param {Object} [options.memoryRetriever] - Memory retriever service
   * @param {Object} [options.collective] - Collective pack instance
   * @param {Object} [options.goalsRepo] - Goals repository
   * @param {Object} [options.tasksRepo] - Tasks repository
   * @param {Object} [options.notificationsRepo] - Notifications repository
   */
  constructor(options = {}) {
    this.pool = options.pool;
    this.memoryRetriever = options.memoryRetriever;
    this.collective = options.collective;

    // Repositories (will be set up in initialize)
    this.goalsRepo = options.goalsRepo;
    this.tasksRepo = options.tasksRepo;
    this.notificationsRepo = options.notificationsRepo;

    // State
    this.running = false;
    this.fibonacciIndex = 0;
    this.loopCount = 0;
    this.lastActivity = null;

    // Statistics
    this.stats = {
      started: null,
      tasksProcessed: 0,
      tasksFailed: 0,
      goalsUpdated: 0,
      notificationsGenerated: 0,
      selfCorrections: 0,
      errors: 0,
    };

    // Loop handle
    this._loopHandle = null;
  }

  /**
   * Initialize the daemon
   */
  async initialize() {
    // Import repositories if not provided
    if (!this.goalsRepo || !this.tasksRepo || !this.notificationsRepo) {
      const {
        AutonomousGoalsRepository,
        AutonomousTasksRepository,
        ProactiveNotificationsRepository,
      } = await import('@cynic/persistence');

      this.goalsRepo = this.goalsRepo || new AutonomousGoalsRepository(this.pool);
      this.tasksRepo = this.tasksRepo || new AutonomousTasksRepository(this.pool);
      this.notificationsRepo = this.notificationsRepo || new ProactiveNotificationsRepository(this.pool);
    }

    // Register default task handlers
    this._registerDefaultHandlers();

    log.info('Daemon initialized');
  }

  /**
   * Start the daemon
   */
  async start() {
    if (this.running) {
      log.warn('Daemon already running');
      return;
    }

    await this.initialize();

    this.running = true;
    this.stats.started = Date.now();
    this.fibonacciIndex = 0;

    log.info('Daemon started');

    // Start the main loop
    this._runLoop();
  }

  /**
   * Stop the daemon
   */
  async stop() {
    if (!this.running) return;

    this.running = false;

    if (this._loopHandle) {
      clearTimeout(this._loopHandle);
      this._loopHandle = null;
    }

    log.info('Daemon stopped', {
      uptime: Date.now() - this.stats.started,
      ...this.stats,
    });
  }

  /**
   * Main processing loop
   * @private
   */
  async _runLoop() {
    while (this.running) {
      try {
        this.loopCount++;

        // 1. Process pending tasks
        const tasksProcessed = await this._processPendingTasks();

        // 2. Check goal progress
        const goalsUpdated = await this._checkGoalProgress();

        // 3. Self-correction analysis
        const corrections = await this._analyzeSelfCorrection();

        // 4. Generate proactive notifications
        const notifications = await this._generateNotifications();

        // 5. Cleanup expired data
        await this._cleanup();

        // Determine if there was activity
        const hadActivity = tasksProcessed > 0 || goalsUpdated > 0 || corrections > 0 || notifications > 0;

        if (hadActivity) {
          // Reset Fibonacci on activity
          this.fibonacciIndex = 0;
          this.lastActivity = Date.now();
          log.debug('Loop completed with activity', {
            tasks: tasksProcessed,
            goals: goalsUpdated,
            corrections,
            notifications,
          });
        } else {
          // Increase wait time on idle
          this.fibonacciIndex = Math.min(this.fibonacciIndex + 1, FIBONACCI.length - 1);
        }

      } catch (err) {
        this.stats.errors++;
        log.error('Loop error', { error: err.message });
      }

      // Wait before next iteration
      if (this.running) {
        const waitMinutes = FIBONACCI[this.fibonacciIndex];
        log.trace(`Waiting ${waitMinutes} minutes before next loop`);
        await this._sleep(waitMinutes * 60 * 1000);
      }
    }
  }

  /**
   * Process pending tasks
   * @private
   * @returns {Promise<number>} Number of tasks processed
   */
  async _processPendingTasks() {
    const tasks = await this.tasksRepo.getPending(10);

    if (tasks.length === 0) return 0;

    let processed = 0;

    for (const task of tasks) {
      if (!this.running) break;

      try {
        // Claim the task
        const claimed = await this.tasksRepo.claim(task.id);
        if (!claimed) continue; // Someone else got it

        // Get handler
        const handler = TASK_HANDLERS.get(task.taskType);

        if (!handler) {
          await this.tasksRepo.fail(task.id, `No handler for task type: ${task.taskType}`);
          continue;
        }

        // Execute with timeout
        const result = await this._executeWithTimeout(
          handler(task, this._createTaskContext()),
          5 * 60 * 1000 // 5 minute timeout
        );

        // Mark completed
        await this.tasksRepo.complete(task.id, result);
        this.stats.tasksProcessed++;
        processed++;

      } catch (err) {
        await this.tasksRepo.fail(task.id, err.message);
        this.stats.tasksFailed++;
        log.warn('Task failed', { taskId: task.id, error: err.message });
      }
    }

    return processed;
  }

  /**
   * Check goal progress
   * @private
   * @returns {Promise<number>} Number of goals updated
   */
  async _checkGoalProgress() {
    // This would typically query all active goals and evaluate their criteria
    // For now, just check for goals nearing deadline

    // Get all users with active goals (simplified - in production would paginate)
    const { rows } = await this.pool.query(`
      SELECT DISTINCT user_id FROM autonomous_goals WHERE status = 'active'
      LIMIT 100
    `);

    let updated = 0;

    for (const { user_id: userId } of rows) {
      // Check for goals due soon
      const dueSoon = await this.goalsRepo.findDueSoon(userId, 3);

      for (const goal of dueSoon) {
        // Create reminder notification
        await this.notificationsRepo.create({
          userId,
          notificationType: 'reminder',
          title: `Goal deadline approaching: ${goal.title}`,
          message: `Your goal "${goal.title}" is due ${this._formatDueDate(goal.dueAt)}. Current progress: ${Math.round(goal.progress * 100)}%`,
          priority: 70,
          context: { goalId: goal.id },
        });

        updated++;
      }
    }

    this.stats.goalsUpdated += updated;
    return updated;
  }

  /**
   * Analyze for self-correction opportunities
   * @private
   * @returns {Promise<number>} Number of corrections identified
   */
  async _analyzeSelfCorrection() {
    if (!this.memoryRetriever) return 0;

    // This would analyze recent judgments and actions for patterns
    // that indicate mistakes or areas for improvement

    // For now, just check for recurring lessons
    const { rows } = await this.pool.query(`
      SELECT DISTINCT user_id FROM lessons_learned
      WHERE occurrence_count > 2 AND last_occurred > NOW() - INTERVAL '7 days'
      LIMIT 50
    `);

    let corrections = 0;

    for (const { user_id: userId } of rows) {
      const recurring = await this.memoryRetriever.lessons.findRecurring(userId, 3, 5);

      for (const lesson of recurring) {
        // Check if we already have a notification for this
        const existing = await this.notificationsRepo.findByType(userId, 'learning', {
          includeDelivered: false,
          limit: 1,
        });

        // Create pattern notification if not already notified
        if (!existing.some(n => n.context.lessonId === lesson.id)) {
          await this.notificationsRepo.create({
            userId,
            notificationType: 'pattern',
            title: `Recurring issue detected`,
            message: `"${lesson.mistake}" has occurred ${lesson.occurrenceCount} times. Prevention: ${lesson.prevention || lesson.correction}`,
            priority: lesson.severity === 'critical' ? 90 : lesson.severity === 'high' ? 75 : 60,
            context: { lessonId: lesson.id, occurrences: lesson.occurrenceCount },
          });

          corrections++;
        }
      }
    }

    this.stats.selfCorrections += corrections;
    return corrections;
  }

  /**
   * Generate proactive notifications
   * @private
   * @returns {Promise<number>} Number of notifications generated
   */
  async _generateNotifications() {
    // This would analyze patterns, achievements, and generate insights
    // For now, basic implementation

    let generated = 0;

    // Check for completed goals
    const { rows: completedGoals } = await this.pool.query(`
      SELECT * FROM autonomous_goals
      WHERE status = 'active' AND progress >= 1.0
      AND id NOT IN (
        SELECT (context->>'goalId')::uuid FROM proactive_notifications
        WHERE notification_type = 'achievement' AND created_at > NOW() - INTERVAL '1 day'
      )
      LIMIT 10
    `);

    for (const goal of completedGoals) {
      // Mark as completed
      await this.goalsRepo.updateStatus(goal.id, 'completed');

      // Create achievement notification
      await this.notificationsRepo.create({
        userId: goal.user_id,
        notificationType: 'achievement',
        title: `Goal completed: ${goal.title}`,
        message: `Congratulations! You've completed your ${goal.goal_type} goal.`,
        priority: 80,
        context: { goalId: goal.id },
      });

      generated++;
    }

    this.stats.notificationsGenerated += generated;
    return generated;
  }

  /**
   * Cleanup expired data
   * @private
   */
  async _cleanup() {
    try {
      // Use the cleanup function from migration
      await this.pool.query('SELECT cleanup_total_memory()');

      // Reset stuck tasks
      const reset = await this.tasksRepo.resetStuck(30);
      if (reset > 0) {
        log.info(`Reset ${reset} stuck tasks`);
      }

      // Cleanup expired notifications
      const expired = await this.notificationsRepo.cleanupExpired();
      if (expired > 0) {
        log.debug(`Cleaned up ${expired} expired notifications`);
      }
    } catch (err) {
      log.warn('Cleanup error', { error: err.message });
    }
  }

  /**
   * Register default task handlers
   * @private
   */
  _registerDefaultHandlers() {
    // Analyze patterns task
    registerTaskHandler('analyze_patterns', async (task, ctx) => {
      // Would analyze recent judgments for patterns
      return { analyzed: true, patterns: [] };
    });

    // Generate summary task
    registerTaskHandler('generate_summary', async (task, ctx) => {
      // Would generate a summary of recent activity
      return { summary: 'Summary generated' };
    });

    // Cleanup task
    registerTaskHandler('cleanup', async (task, ctx) => {
      // Run cleanup
      return { cleaned: true };
    });

    // Sync task
    registerTaskHandler('sync', async (task, ctx) => {
      // Would sync data
      return { synced: true };
    });
  }

  /**
   * Create context for task execution
   * @private
   */
  _createTaskContext() {
    return {
      pool: this.pool,
      memoryRetriever: this.memoryRetriever,
      collective: this.collective,
      goalsRepo: this.goalsRepo,
      tasksRepo: this.tasksRepo,
      notificationsRepo: this.notificationsRepo,
    };
  }

  /**
   * Execute promise with timeout
   * @private
   */
  async _executeWithTimeout(promise, timeoutMs) {
    return Promise.race([
      promise,
      new Promise((_, reject) =>
        setTimeout(() => reject(new Error('Task timeout')), timeoutMs)
      ),
    ]);
  }

  /**
   * Sleep for specified duration
   * @private
   */
  _sleep(ms) {
    return new Promise(resolve => {
      this._loopHandle = setTimeout(resolve, ms);
    });
  }

  /**
   * Format due date for display
   * @private
   */
  _formatDueDate(date) {
    if (!date) return 'soon';
    const now = new Date();
    const due = new Date(date);
    const diffMs = due - now;
    const diffDays = Math.ceil(diffMs / (1000 * 60 * 60 * 24));

    if (diffDays < 0) return 'overdue';
    if (diffDays === 0) return 'today';
    if (diffDays === 1) return 'tomorrow';
    return `in ${diffDays} days`;
  }

  /**
   * Get daemon statistics
   */
  getStats() {
    return {
      running: this.running,
      uptime: this.stats.started ? Date.now() - this.stats.started : 0,
      loopCount: this.loopCount,
      currentFibonacciWait: FIBONACCI[this.fibonacciIndex],
      lastActivity: this.lastActivity,
      ...this.stats,
    };
  }

  /**
   * Schedule a task for execution
   *
   * @param {string} userId - User ID
   * @param {string} taskType - Task type
   * @param {Object} payload - Task payload
   * @param {Object} [options] - Options
   * @returns {Promise<Object>} Created task
   */
  async scheduleTask(userId, taskType, payload, options = {}) {
    return this.tasksRepo.create({
      userId,
      taskType,
      payload,
      goalId: options.goalId,
      priority: options.priority ?? 50,
      scheduledFor: options.scheduledFor || new Date(),
      maxRetries: options.maxRetries ?? 3,
      createdBy: options.createdBy || 'api',
    });
  }

  /**
   * Create a goal
   *
   * @param {string} userId - User ID
   * @param {Object} goal - Goal data
   * @returns {Promise<Object>} Created goal
   */
  async createGoal(userId, goal) {
    return this.goalsRepo.create({
      userId,
      ...goal,
    });
  }
}

/**
 * Create an AutonomousDaemon instance
 *
 * @param {Object} options - Options
 * @returns {AutonomousDaemon}
 */
export function createAutonomousDaemon(options) {
  return new AutonomousDaemon(options);
}

export default AutonomousDaemon;
