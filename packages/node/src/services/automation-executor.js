/**
 * CYNIC Automation Executor
 *
 * Scheduled execution of automated tasks:
 * - Learning cycles (every 5 minutes - Fibonacci)
 * - Trigger evaluation (every 1 minute)
 * - Cleanup tasks (every 13 minutes - Fibonacci)
 *
 * Integrates:
 * - LearningManager for RLHF cycles
 * - TriggerManager for condition evaluation
 * - EventBus for event-driven automation
 *
 * "φ automates, φ learns" - κυνικός
 *
 * @module @cynic/node/services/automation-executor
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';
import { EventBus, EventType, getEventBus } from './event-bus.js';
import { PatternLearning } from '@cynic/persistence';

const log = createLogger('AutomationExecutor');

/**
 * Fibonacci-aligned intervals (in milliseconds)
 */
const INTERVALS = {
  LEARNING: 5 * 60 * 1000,     // 5 minutes (Fibonacci)
  TRIGGERS: 1 * 60 * 1000,     // 1 minute
  CLEANUP: 13 * 60 * 1000,     // 13 minutes (Fibonacci)
  HEARTBEAT: 30 * 1000,        // 30 seconds
  TASKS: 2 * 60 * 1000,        // 2 minutes (Fibonacci) - task queue
  GOALS: 8 * 60 * 1000,        // 8 minutes (Fibonacci) - goal progress
  NOTIFICATIONS: 21 * 60 * 1000, // 21 minutes (Fibonacci) - proactive insights
  GOVERNANCE: 34 * 60 * 1000,  // 34 minutes (F9 Fibonacci) - dimension governance
  DATA_GRAVES: 21 * 60 * 1000, // 21 minutes (F8 Fibonacci) - data grave analysis
  PATTERN_LEARNING: 55 * 60 * 1000, // 55 minutes (F10 Fibonacci) - pattern extraction/decay/clustering
};

/**
 * Minimum feedback samples before auto-learning
 */
const MIN_FEEDBACK_SAMPLES = 5;

/**
 * Minimum anomaly samples before governance review
 */
const MIN_GOVERNANCE_SAMPLES = 5;

/**
 * Automation Executor
 *
 * Background service that runs scheduled automation tasks.
 */
export class AutomationExecutor {
  /**
   * Create the executor
   *
   * @param {Object} options - Options
   * @param {Object} [options.learningManager] - LearningManager instance
   * @param {Object} [options.triggerManager] - TriggerManager instance
   * @param {Object} [options.pool] - PostgreSQL connection pool
   * @param {EventBus} [options.eventBus] - Event bus (defaults to global)
   * @param {Object} [options.intervals] - Custom intervals
   * @param {Object} [options.goalsRepo] - AutonomousGoalsRepository
   * @param {Object} [options.tasksRepo] - AutonomousTasksRepository
   * @param {Object} [options.notificationsRepo] - ProactiveNotificationsRepository
   */
  constructor(options = {}) {
    this.learningManager = options.learningManager;
    this.triggerManager = options.triggerManager;
    this.pool = options.pool;
    this.eventBus = options.eventBus || getEventBus();

    // Dimension governance
    this.residualGovernance = options.residualGovernance || null;
    this.collectivePack = options.collectivePack || null;

    // Pattern learning (auto-create from pool if not injected)
    this.patternLearning = options.patternLearning || null;
    if (!this.patternLearning && this.pool) {
      try {
        this.patternLearning = new PatternLearning({ pool: this.pool });
      } catch (e) {
        log.debug('PatternLearning auto-create failed (non-blocking)', { error: e.message });
      }
    }

    // Autonomy repos
    this.goalsRepo = options.goalsRepo;
    this.tasksRepo = options.tasksRepo;
    this.notificationsRepo = options.notificationsRepo;

    // Custom intervals
    this.intervals = {
      ...INTERVALS,
      ...options.intervals,
    };

    // State
    this.running = false;
    this._intervalHandles = new Map();
    this._pendingFeedback = [];
    this._unsubscribers = [];

    // Statistics
    this.stats = {
      startedAt: null,
      learningCycles: 0,
      triggersEvaluated: 0,
      cleanupRuns: 0,
      tasksProcessed: 0,
      goalsUpdated: 0,
      notificationsGenerated: 0,
      governanceReviews: 0,
      governancePromotions: 0,
      errors: 0,
      lastLearningCycle: null,
      lastTriggerEval: null,
      lastCleanup: null,
      lastTaskProcess: null,
      lastGoalUpdate: null,
      lastNotificationGen: null,
      lastGovernanceReview: null,
      dataGraveRuns: 0,
      lastDataGraveRun: null,
      dataGraveFindings: 0,
      patternLearningRuns: 0,
      lastPatternLearningRun: null,
      patternsExtracted: 0,
      patternsDecayed: 0,
    };

    log.debug('Automation executor created');
  }

  /**
   * Start the automation executor
   */
  async start() {
    if (this.running) {
      log.warn('Executor already running');
      return;
    }

    this.running = true;
    this.stats.startedAt = Date.now();

    log.info('Automation executor starting');

    // Subscribe to events
    this._setupEventSubscriptions();

    // Start scheduled intervals
    this._startIntervals();

    // Emit start event
    this.eventBus.publish(EventType.AUTOMATION_START, {
      intervals: this.intervals,
    }, { source: 'AutomationExecutor' });

    log.info('Automation executor started', {
      learningInterval: `${this.intervals.LEARNING / 60000}min`,
      triggerInterval: `${this.intervals.TRIGGERS / 60000}min`,
      cleanupInterval: `${this.intervals.CLEANUP / 60000}min`,
    });
  }

  /**
   * Stop the automation executor
   */
  async stop() {
    if (!this.running) return;

    this.running = false;

    // Clear all intervals
    for (const [name, handle] of this._intervalHandles) {
      clearInterval(handle);
      log.debug(`Stopped interval: ${name}`);
    }
    this._intervalHandles.clear();

    // Unsubscribe from events
    for (const unsubscribe of this._unsubscribers) {
      unsubscribe();
    }
    this._unsubscribers = [];

    // Emit stop event
    this.eventBus.publish(EventType.AUTOMATION_STOP, {
      uptime: Date.now() - this.stats.startedAt,
      stats: this.stats,
    }, { source: 'AutomationExecutor' });

    log.info('Automation executor stopped', {
      uptime: Date.now() - this.stats.startedAt,
      ...this.stats,
    });
  }

  /**
   * Set up event subscriptions
   * @private
   */
  _setupEventSubscriptions() {
    // Subscribe to feedback events
    const feedbackUnsub = this.eventBus.subscribe(EventType.FEEDBACK_RECEIVED, async (event) => {
      await this._handleFeedbackReceived(event.data);
    });
    this._unsubscribers.push(feedbackUnsub);

    // Subscribe to judgment events for trigger evaluation
    const judgmentUnsub = this.eventBus.subscribe(EventType.JUDGMENT_CREATED, async (event) => {
      await this._handleJudgmentCreated(event.data);
    });
    this._unsubscribers.push(judgmentUnsub);

    // Subscribe to session events
    const sessionStartUnsub = this.eventBus.subscribe(EventType.SESSION_START, async (event) => {
      log.debug('Session started', { userId: event.data.userId });
    });
    this._unsubscribers.push(sessionStartUnsub);

    const sessionEndUnsub = this.eventBus.subscribe(EventType.SESSION_END, async (event) => {
      // Process any pending feedback when session ends
      if (this._pendingFeedback.length > 0) {
        await this._runLearningCycle('session_end');
      }
    });
    this._unsubscribers.push(sessionEndUnsub);

    log.debug('Event subscriptions set up');
  }

  /**
   * Start scheduled intervals
   * @private
   */
  _startIntervals() {
    // Learning cycle interval
    const learningHandle = setInterval(() => {
      this._runLearningCycle('scheduled').catch((err) => {
        this.stats.errors++;
        log.error('Scheduled learning cycle failed', { error: err.message });
      });
    }, this.intervals.LEARNING);
    this._intervalHandles.set('learning', learningHandle);

    // Trigger evaluation interval
    const triggerHandle = setInterval(() => {
      this._evaluateTriggers().catch((err) => {
        this.stats.errors++;
        log.error('Trigger evaluation failed', { error: err.message });
      });
    }, this.intervals.TRIGGERS);
    this._intervalHandles.set('triggers', triggerHandle);

    // Cleanup interval
    const cleanupHandle = setInterval(() => {
      this._runCleanup().catch((err) => {
        this.stats.errors++;
        log.error('Cleanup failed', { error: err.message });
      });
    }, this.intervals.CLEANUP);
    this._intervalHandles.set('cleanup', cleanupHandle);

    // Heartbeat interval
    const heartbeatHandle = setInterval(() => {
      this.eventBus.publish(EventType.AUTOMATION_TICK, {
        uptime: Date.now() - this.stats.startedAt,
        pendingFeedback: this._pendingFeedback.length,
        stats: this.stats,
      }, { source: 'AutomationExecutor' });
    }, this.intervals.HEARTBEAT);
    this._intervalHandles.set('heartbeat', heartbeatHandle);

    // Task queue processing interval
    const tasksHandle = setInterval(() => {
      this._processTaskQueue().catch((err) => {
        this.stats.errors++;
        log.error('Task queue processing failed', { error: err.message });
      });
    }, this.intervals.TASKS);
    this._intervalHandles.set('tasks', tasksHandle);

    // Goal progress update interval
    const goalsHandle = setInterval(() => {
      this._updateGoalProgress().catch((err) => {
        this.stats.errors++;
        log.error('Goal progress update failed', { error: err.message });
      });
    }, this.intervals.GOALS);
    this._intervalHandles.set('goals', goalsHandle);

    // Notification generation interval
    const notificationsHandle = setInterval(() => {
      this._generateNotifications().catch((err) => {
        this.stats.errors++;
        log.error('Notification generation failed', { error: err.message });
      });
    }, this.intervals.NOTIFICATIONS);
    this._intervalHandles.set('notifications', notificationsHandle);

    // Governance review interval (F9 = 34 min Fibonacci)
    const governanceHandle = setInterval(() => {
      this._runGovernanceReview().catch((err) => {
        this.stats.errors++;
        log.error('Governance review failed', { error: err.message });
      });
    }, this.intervals.GOVERNANCE);
    this._intervalHandles.set('governance', governanceHandle);

    // Data grave analysis interval (F8 = 21 min Fibonacci)
    const dataGraveHandle = setInterval(() => {
      this._analyzeDataGraves().catch((err) => {
        this.stats.errors++;
        log.error('Data grave analysis failed', { error: err.message });
      });
    }, this.intervals.DATA_GRAVES);
    this._intervalHandles.set('dataGraves', dataGraveHandle);

    // Pattern learning interval (F10 = 55 min Fibonacci)
    if (this.patternLearning) {
      const patternHandle = setInterval(() => {
        this._runPatternLearning().catch((err) => {
          this.stats.errors++;
          log.error('Pattern learning failed', { error: err.message });
        });
      }, this.intervals.PATTERN_LEARNING);
      this._intervalHandles.set('patternLearning', patternHandle);
    }

    log.debug('Intervals started');
  }

  /**
   * Handle feedback received event
   * @private
   */
  async _handleFeedbackReceived(feedback) {
    this._pendingFeedback.push({
      ...feedback,
      receivedAt: Date.now(),
    });

    log.debug('Feedback received', {
      pending: this._pendingFeedback.length,
      minSamples: MIN_FEEDBACK_SAMPLES,
    });

    // Auto-trigger learning if threshold reached
    if (this._pendingFeedback.length >= MIN_FEEDBACK_SAMPLES) {
      await this._runLearningCycle('threshold');
    }
  }

  /**
   * Handle judgment created event
   * @private
   */
  async _handleJudgmentCreated(judgment) {
    // Could trigger immediate evaluation for certain judgment types
    log.debug('Judgment created', { judgmentId: judgment.judgmentId });
  }

  /**
   * Run a learning cycle
   * @private
   */
  async _runLearningCycle(trigger = 'manual') {
    if (!this.learningManager) {
      log.debug('No learning manager configured, skipping cycle');
      return null;
    }

    this.eventBus.publish(EventType.LEARNING_CYCLE_START, {
      trigger,
      pendingFeedback: this._pendingFeedback.length,
    }, { source: 'AutomationExecutor' });

    try {
      const result = await this.learningManager.runLearningCycle();

      this.stats.learningCycles++;
      this.stats.lastLearningCycle = Date.now();

      // Clear processed feedback
      this._pendingFeedback = [];

      this.eventBus.publish(EventType.LEARNING_CYCLE_COMPLETE, {
        trigger,
        result,
        cycleNumber: this.stats.learningCycles,
      }, { source: 'AutomationExecutor' });

      log.info('Learning cycle completed', {
        trigger,
        cycleNumber: this.stats.learningCycles,
        feedbackProcessed: result.feedback?.processed || 0,
        patternsUpdated: result.patterns?.updated || 0,
      });

      return result;
    } catch (err) {
      this.stats.errors++;
      log.error('Learning cycle failed', { trigger, error: err.message });
      throw err;
    }
  }

  /**
   * Evaluate triggers
   * @private
   */
  async _evaluateTriggers() {
    if (!this.triggerManager) {
      log.trace('No trigger manager configured, skipping evaluation');
      return [];
    }

    this.eventBus.publish(EventType.TRIGGER_EVALUATED, {
      timestamp: Date.now(),
    }, { source: 'AutomationExecutor' });

    try {
      // Get enabled triggers
      const triggers = await this.triggerManager.getEnabled();
      const results = [];

      for (const trigger of triggers) {
        try {
          const shouldFire = await this._evaluateTriggerCondition(trigger);

          if (shouldFire) {
            const result = await this._executeTriggerAction(trigger);
            results.push({ triggerId: trigger.triggerId, result });

            this.eventBus.publish(EventType.TRIGGER_FIRED, {
              triggerId: trigger.triggerId,
              name: trigger.name,
              action: trigger.action,
              result,
            }, { source: 'AutomationExecutor' });
          }
        } catch (err) {
          log.warn('Trigger evaluation failed', {
            triggerId: trigger.triggerId,
            error: err.message,
          });
        }
      }

      this.stats.triggersEvaluated++;
      this.stats.lastTriggerEval = Date.now();

      return results;
    } catch (err) {
      this.stats.errors++;
      log.error('Trigger evaluation failed', { error: err.message });
      throw err;
    }
  }

  /**
   * Evaluate a trigger's condition
   * @private
   */
  async _evaluateTriggerCondition(trigger) {
    const condition = trigger.condition || {};

    switch (trigger.triggerType) {
      case 'periodic': {
        const interval = condition.interval || 3600000;
        const lastActivated = trigger.lastActivatedAt
          ? new Date(trigger.lastActivatedAt).getTime()
          : 0;
        return Date.now() - lastActivated >= interval;
      }

      case 'threshold': {
        // Would need to query metrics - simplified for now
        return false;
      }

      case 'event': {
        // Events are handled via event bus subscriptions
        return false;
      }

      case 'pattern': {
        // Pattern matching on recent events
        return false;
      }

      default:
        return false;
    }
  }

  /**
   * Execute a trigger's action
   * @private
   */
  async _executeTriggerAction(trigger) {
    const config = trigger.actionConfig || {};

    switch (trigger.action) {
      case 'log':
        log.info(config.message || 'Trigger fired', { triggerId: trigger.triggerId });
        return { logged: true };

      case 'judge':
        // Emit judge request event for processing by judge service
        if (this.eventBus) {
          this.eventBus.publish({
            type: EventType.JUDGE_REQUEST,
            data: {
              itemType: config.itemType || 'trigger',
              item: config.item || { triggerId: trigger.triggerId },
              source: 'automation',
            },
          });
          return { judged: true, queued: true };
        }
        return { judged: false, reason: 'No event bus' };

      case 'alert':
        log.warn(`ALERT: ${config.message}`, {
          severity: config.severity,
          triggerId: trigger.triggerId,
        });
        return { alerted: true };

      case 'notify':
        // Send notification via repo if available
        if (this.notificationsRepo) {
          try {
            await this.notificationsRepo.create({
              type: config.type || 'trigger',
              title: config.title || 'Trigger Notification',
              message: config.message || `Trigger ${trigger.triggerId} fired`,
              priority: config.priority || 'medium',
              source: 'automation',
            });
            return { notified: true };
          } catch (e) {
            log.error('Failed to create notification', e);
            return { notified: false, reason: e.message };
          }
        }
        // Fallback: emit event
        if (this.eventBus) {
          this.eventBus.publish({
            type: EventType.NOTIFICATION,
            data: {
              title: config.title || 'Trigger Notification',
              message: config.message,
              triggerId: trigger.triggerId,
            },
          });
          return { notified: true, method: 'event' };
        }
        return { notified: false, reason: 'No notification channel' };

      default:
        return { action: trigger.action, executed: false };
    }
  }

  /**
   * Run cleanup tasks
   * @private
   */
  async _runCleanup() {
    if (!this.pool) {
      log.trace('No pool configured, skipping cleanup');
      return;
    }

    try {
      // Cleanup expired trigger events
      const eventResult = await this.pool.query(
        'SELECT cleanup_expired_trigger_events()'
      );
      const eventsDeleted = eventResult.rows[0]?.cleanup_expired_trigger_events || 0;

      // Cleanup total memory (if function exists)
      try {
        await this.pool.query('SELECT cleanup_total_memory()');
      } catch (e) {
        // Function may not exist
      }

      // AXE 2+: Cleanup dog_events and collective_snapshots tables
      // These grow fast and need periodic cleanup (7 days for events, 3 days for snapshots)
      let dogEventsDeleted = 0;
      let snapshotsDeleted = 0;
      try {
        const dogResult = await this.pool.query(
          `DELETE FROM dog_events WHERE created_at < NOW() - INTERVAL '7 days' RETURNING id`
        );
        dogEventsDeleted = dogResult?.rowCount || 0;

        const snapResult = await this.pool.query(
          `DELETE FROM collective_snapshots WHERE created_at < NOW() - INTERVAL '3 days' RETURNING id`
        );
        snapshotsDeleted = snapResult?.rowCount || 0;
      } catch (e) {
        // Tables may not exist yet
        log.trace('Event data cleanup skipped', { error: e.message });
      }

      // Migration 033: Cleanup consciousness metrics tables (30-day retention)
      let consciousnessMetricsCleaned = 0;
      try {
        await this.pool.query('SELECT cleanup_consciousness_metrics()');
        consciousnessMetricsCleaned = 1;
      } catch (e) {
        // Function may not exist yet (pre-migration 033)
      }

      this.stats.cleanupRuns++;
      this.stats.lastCleanup = Date.now();

      if (eventsDeleted > 0 || dogEventsDeleted > 0 || snapshotsDeleted > 0 || consciousnessMetricsCleaned > 0) {
        log.debug('Cleanup completed', { eventsDeleted, dogEventsDeleted, snapshotsDeleted, consciousnessMetricsCleaned });
      }
    } catch (err) {
      this.stats.errors++;
      log.warn('Cleanup failed', { error: err.message });
    }
  }

  /**
   * Process pending tasks from the durable queue
   * @private
   */
  async _processTaskQueue() {
    if (!this.tasksRepo) {
      log.trace('No tasks repo configured, skipping task processing');
      return [];
    }

    try {
      // Get pending tasks that are ready to execute
      const pendingTasks = await this.tasksRepo.getPending(10);
      const results = [];

      for (const task of pendingTasks) {
        try {
          // Claim task atomically (marks as running)
          const claimed = await this.tasksRepo.claim(task.id);
          if (!claimed) {
            // Task was claimed by another process
            continue;
          }

          // Execute task based on type
          const result = await this._executeTask(task);

          // Mark as completed or failed
          if (result.success) {
            await this.tasksRepo.complete(task.id, result);
            this.stats.tasksProcessed++;
          } else {
            await this.tasksRepo.fail(task.id, result.error || 'Unknown error');
          }

          results.push({ taskId: task.id, ...result });
        } catch (err) {
          await this.tasksRepo.fail(task.id, err.message);
          log.warn('Task execution failed', { taskId: task.id, error: err.message });
        }
      }

      this.stats.lastTaskProcess = Date.now();

      if (results.length > 0) {
        log.debug('Tasks processed', { count: results.length });
      }

      return results;
    } catch (err) {
      this.stats.errors++;
      log.error('Task queue processing failed', { error: err.message });
      throw err;
    }
  }

  /**
   * Execute a single task
   * @private
   */
  async _executeTask(task) {
    const payload = task.payload || {};

    switch (task.task_type) {
      case 'learning_cycle':
        const cycleResult = await this._runLearningCycle('task');
        return { success: true, result: cycleResult };

      case 'goal_check':
        await this._updateGoalProgress();
        return { success: true };

      case 'notification':
        await this._generateNotifications();
        return { success: true };

      case 'cleanup':
        await this._runCleanup();
        return { success: true };

      case 'analyze_patterns':
        // Would analyze patterns from recent session
        return { success: true, analyzed: true };

      case 'governance_review':
        const govRes = await this._runGovernanceReview();
        return { success: true, result: govRes };

      default:
        log.warn('Unknown task type', { taskType: task.task_type });
        return { success: false, error: `Unknown task type: ${task.task_type}` };
    }
  }

  /**
   * Update progress for all active goals
   * @private
   */
  async _updateGoalProgress() {
    if (!this.goalsRepo || !this.pool) {
      log.trace('No goals repo/pool configured, skipping goal updates');
      return [];
    }

    try {
      // Get all active goals across all users (system-level processing)
      const { rows: activeGoals } = await this.pool.query(`
        SELECT * FROM autonomous_goals
        WHERE status = 'active'
        ORDER BY priority DESC
        LIMIT 50
      `);

      const updates = [];

      for (const goal of activeGoals) {
        try {
          // Evaluate progress based on goal type
          const progress = await this._evaluateGoalProgress(goal);

          if (progress !== null && Math.abs(progress - (goal.progress || 0)) > 0.001) {
            await this.goalsRepo.updateProgress(goal.id, progress);
            this.stats.goalsUpdated++;
            updates.push({ goalId: goal.id, progress });

            // Check if goal completed
            if (progress >= 1.0) {
              await this.goalsRepo.updateStatus(goal.id, 'completed');
              log.info('Goal completed!', { goalId: goal.id, title: goal.title });

              // Generate achievement notification
              if (this.notificationsRepo) {
                await this.notificationsRepo.create({
                  user_id: goal.user_id || 'system',
                  notification_type: 'achievement',
                  title: `Goal Completed: ${goal.title}`,
                  message: `Congratulations! You've achieved your goal.`,
                  priority: 80,
                  context: { goalId: goal.id, goalType: goal.goal_type },
                });
                this.stats.notificationsGenerated++;
              }
            }
          }
        } catch (err) {
          log.warn('Goal progress update failed', { goalId: goal.id, error: err.message });
        }
      }

      this.stats.lastGoalUpdate = Date.now();

      if (updates.length > 0) {
        log.debug('Goals updated', { count: updates.length });
      }

      return updates;
    } catch (err) {
      this.stats.errors++;
      log.error('Goal progress update failed', { error: err.message });
      throw err;
    }
  }

  /**
   * Evaluate progress for a specific goal
   * @private
   */
  async _evaluateGoalProgress(goal) {
    const criteria = goal.success_criteria || [];

    switch (goal.goal_type) {
      case 'quality': {
        // Quality goals: based on test coverage, lint scores, etc.
        // For now, simulate small incremental progress
        const currentProgress = goal.progress || 0;
        return Math.min(1.0, currentProgress + 0.01 * PHI_INV);
      }

      case 'learning': {
        // Learning goals: based on lessons applied, mistakes prevented
        const currentProgress = goal.progress || 0;
        const learningBoost = this.stats.learningCycles > 0 ? 0.05 : 0;
        return Math.min(1.0, currentProgress + learningBoost * PHI_INV);
      }

      case 'security': {
        // Security goals: based on vulnerabilities fixed, audit scores
        return goal.progress; // No auto-progress for security
      }

      case 'maintenance': {
        // Maintenance goals: based on tech debt reduced, deps updated
        const currentProgress = goal.progress || 0;
        return Math.min(1.0, currentProgress + 0.005 * PHI_INV);
      }

      default:
        return null;
    }
  }

  /**
   * Generate proactive notifications based on system state
   * @private
   */
  async _generateNotifications() {
    if (!this.notificationsRepo) {
      log.trace('No notifications repo configured, skipping generation');
      return [];
    }

    try {
      const notifications = [];

      // Insight: Learning progress
      if (this.stats.learningCycles > 0 && this.stats.learningCycles % 5 === 0) {
        const notification = await this.notificationsRepo.create({
          user_id: 'system',
          notification_type: 'insight',
          title: 'Learning Milestone',
          message: `CYNIC has completed ${this.stats.learningCycles} learning cycles. φ improves.`,
          priority: 50,
          context: { learningCycles: this.stats.learningCycles },
        });
        notifications.push(notification);
        this.stats.notificationsGenerated++;
      }

      // Warning: High error rate
      const errorRate = this.stats.errors / Math.max(1, this.stats.learningCycles + this.stats.triggersEvaluated);
      if (errorRate > PHI_INV && this.stats.errors > 5) {
        const notification = await this.notificationsRepo.create({
          user_id: 'system',
          notification_type: 'warning',
          title: 'High Error Rate Detected',
          message: `Error rate is ${Math.round(errorRate * 100)}%. Investigation recommended.`,
          priority: 75,
          context: { errorRate, totalErrors: this.stats.errors },
        });
        notifications.push(notification);
        this.stats.notificationsGenerated++;
      }

      // Insight: Tasks processed milestone
      if (this.stats.tasksProcessed > 0 && this.stats.tasksProcessed % 10 === 0) {
        const notification = await this.notificationsRepo.create({
          user_id: 'system',
          notification_type: 'insight',
          title: 'Automation Progress',
          message: `${this.stats.tasksProcessed} autonomous tasks have been processed.`,
          priority: 40,
          context: { tasksProcessed: this.stats.tasksProcessed },
        });
        notifications.push(notification);
        this.stats.notificationsGenerated++;
      }

      this.stats.lastNotificationGen = Date.now();

      if (notifications.length > 0) {
        log.debug('Notifications generated', { count: notifications.length });
      }

      return notifications;
    } catch (err) {
      this.stats.errors++;
      log.error('Notification generation failed', { error: err.message });
      throw err;
    }
  }

  /**
   * Run dimension governance review
   * Reviews ResidualDetector candidates via CollectivePack voting
   * @private
   */
  async _runGovernanceReview() {
    if (!this.residualGovernance) {
      log.trace("No residual governance configured, skipping review");
      return null;
    }

    try {
      const result = await this.residualGovernance.reviewCandidates();

      this.stats.governanceReviews++;
      this.stats.lastGovernanceReview = Date.now();

      if (result.promoted > 0) {
        this.stats.governancePromotions += result.promoted;
      }

      if (result.reviewed > 0 || result.promoted > 0) {
        log.info("Governance review completed", {
          reviewed: result.reviewed,
          promoted: result.promoted,
          rejected: result.rejected,
          skipped: result.skipped,
          cycleNumber: this.stats.governanceReviews,
        });
      }

      return result;
    } catch (err) {
      this.stats.errors++;
      log.error("Governance review failed", { error: err.message });
      throw err;
    }
  }

  /**
   * Analyze data grave tables for significant patterns.
   * Runs every 21 minutes (Fibonacci F8).
   * Logs findings above phi-2 threshold (0.382).
   * @private
   */
  async _analyzeDataGraves() {
    if (!this.pool) {
      log.trace('No pool configured, skipping data grave analysis');
      return;
    }

    const findings = [];

    try {
      // 1. Dog behavioral summary (last 24h)
      try {
        const { rows } = await this.pool.query(
          `SELECT dog_name, COUNT(*) AS events,
                  COUNT(DISTINCT event_type) AS unique_types,
                  MODE() WITHIN GROUP (ORDER BY health) AS dominant_health
           FROM dog_events
           WHERE created_at > NOW() - INTERVAL '24 hours'
           GROUP BY dog_name
           ORDER BY events DESC LIMIT 11`
        );
        if (rows.length > 0) {
          const unhealthy = rows.filter(r => r.dominant_health !== 'healthy' && r.dominant_health !== 'good');
          if (unhealthy.length > 0) {
            findings.push({ type: 'dog_health', message: `${unhealthy.length} dogs in non-healthy state: ${unhealthy.map(r => r.dog_name).join(', ')}`, severity: 'medium' });
          }
        }
      } catch (e) { /* table may not exist */ }

      // 2. Consensus quality (last 24h)
      try {
        const { rows: [cq] } = await this.pool.query(
          `SELECT COUNT(*) AS total,
                  AVG(agreement) AS avg_agreement,
                  COUNT(*) FILTER (WHERE guardian_veto = true) AS vetoes
           FROM consensus_votes
           WHERE created_at > NOW() - INTERVAL '24 hours'`
        );
        if (cq && parseInt(cq.total) > 0) {
          const avg = parseFloat(cq.avg_agreement) || 0;
          const vetoes = parseInt(cq.vetoes);
          if (avg < PHI_INV * 0.618) {
            findings.push({ type: 'consensus', message: `Low consensus agreement: ${Math.round(avg * 100)}% avg across ${cq.total} votes`, severity: 'high' });
          }
          if (vetoes > 3) {
            findings.push({ type: 'vetoes', message: `${vetoes} guardian vetoes in 24h`, severity: 'medium' });
          }
        }
      } catch (e) { /* table may not exist */ }

      // 3. Tool failure hotspots (last 24h)
      try {
        const { rows } = await this.pool.query(
          `SELECT tool_name,
                  COUNT(*) AS total,
                  COUNT(*) FILTER (WHERE success = false) AS fails,
                  AVG(latency_ms) AS avg_latency
           FROM tool_usage
           WHERE created_at > NOW() - INTERVAL '24 hours'
           GROUP BY tool_name
           HAVING COUNT(*) >= 5
           ORDER BY COUNT(*) FILTER (WHERE success = false) DESC
           LIMIT 5`
        );
        for (const row of rows) {
          const failRate = parseInt(row.fails) / parseInt(row.total);
          if (failRate > 0.382) {
            findings.push({ type: 'tool_failure', message: `${row.tool_name}: ${Math.round(failRate * 100)}% fail rate (${row.total} uses, avg ${Math.round(parseFloat(row.avg_latency))}ms)`, severity: failRate > 0.618 ? 'high' : 'medium' });
          }
        }
      } catch (e) { /* table may not exist */ }

      // 4. Signal volume anomaly (last 6h vs 24h)
      try {
        const { rows: [sig] } = await this.pool.query(
          `SELECT
             COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '6 hours') AS recent,
             COUNT(*) AS total
           FROM dog_signals
           WHERE created_at > NOW() - INTERVAL '24 hours'`
        );
        if (sig && parseInt(sig.total) > 10) {
          const recentRatio = parseInt(sig.recent) / parseInt(sig.total);
          if (recentRatio > 0.618) {
            findings.push({ type: 'signal_spike', message: `Signal spike: ${Math.round(recentRatio * 100)}% of 24h signals in last 6h (${sig.recent}/${sig.total})`, severity: 'medium' });
          }
        }
      } catch (e) { /* table may not exist */ }

      // 5. Friction category hotspots (last 6h, from recent_frictions view)
      try {
        const { rows } = await this.pool.query(
          `SELECT severity, category, SUM(count) as total
           FROM recent_frictions
           WHERE hour > NOW() - INTERVAL '6 hours'
           GROUP BY severity, category
           ORDER BY total DESC
           LIMIT 10`
        );
        for (const row of rows) {
          const total = parseInt(row.total);
          if (row.severity === 'critical' && total >= 3) {
            findings.push({ type: 'friction_hotspot', message: `Critical frictions in ${row.category}: ${total} in 6h`, severity: 'high' });
          } else if (row.severity === 'high' && total >= 5) {
            findings.push({ type: 'friction_hotspot', message: `High frictions in ${row.category}: ${total} in 6h`, severity: 'medium' });
          }
        }
      } catch (e) { /* view may not exist */ }

      // 6. Orchestration domain performance (last 7d, from orchestration_learning_view)
      try {
        const { rows } = await this.pool.query(
          `SELECT domain, outcome, count, skill_success_rate, avg_qscore
           FROM orchestration_learning_view
           WHERE count >= 5
           ORDER BY count DESC
           LIMIT 10`
        );
        for (const row of rows) {
          const successRate = parseFloat(row.skill_success_rate) || 0;
          if (successRate < PHI_INV * 0.618 && parseInt(row.count) >= 10) {
            findings.push({
              type: 'orchestration_quality',
              message: `Domain "${row.domain}" skill success: ${Math.round(successRate * 100)}% (${row.count} decisions, avg Q: ${Math.round(parseFloat(row.avg_qscore) || 0)})`,
              severity: successRate < 0.236 ? 'high' : 'medium',
            });
          }
        }
      } catch (e) { /* view may not exist */ }

      // 7. Burnout trend analysis (last 6h, from psychology_snapshots)
      try {
        const { rows: [psy] } = await this.pool.query(
          `SELECT COUNT(*) AS total,
                  AVG(burnout_score) AS avg_burnout,
                  MAX(burnout_score) AS peak_burnout,
                  AVG(energy) AS avg_energy,
                  AVG(frustration) AS avg_frustration
           FROM psychology_snapshots
           WHERE created_at > NOW() - INTERVAL '6 hours'`
        );
        if (psy && parseInt(psy.total) >= 3) {
          const avgBurnout = parseFloat(psy.avg_burnout) || 0;
          const peakBurnout = parseFloat(psy.peak_burnout) || 0;
          const avgEnergy = parseFloat(psy.avg_energy) || 0.5;
          if (peakBurnout > PHI_INV) {
            findings.push({ type: 'burnout_active', message: `Burnout peak ${Math.round(peakBurnout * 100)}% in 6h (avg ${Math.round(avgBurnout * 100)}%, energy ${Math.round(avgEnergy * 100)}%)`, severity: 'high' });
          } else if (avgBurnout > 0.382 && avgEnergy < 0.382) {
            findings.push({ type: 'burnout_risk', message: `Elevated burnout risk: avg ${Math.round(avgBurnout * 100)}%, low energy ${Math.round(avgEnergy * 100)}%`, severity: 'medium' });
          }
        }
      } catch (e) { /* table may not exist */ }

      // 9. Judgment quality trends (last 24h, from judgments table)
      try {
        const { rows: [jq] } = await this.pool.query(
          `SELECT COUNT(*) AS total,
                  AVG(q_score) AS avg_qscore,
                  COUNT(*) FILTER (WHERE verdict = 'BARK') AS bark_count,
                  COUNT(*) FILTER (WHERE verdict = 'HOWL') AS howl_count,
                  MIN(q_score) AS min_qscore,
                  AVG(confidence) AS avg_confidence
           FROM judgments
           WHERE created_at > NOW() - INTERVAL '24 hours'`
        );
        if (jq && parseInt(jq.total) >= 5) {
          const total = parseInt(jq.total);
          const barkRate = parseInt(jq.bark_count) / total;
          const avgQ = parseFloat(jq.avg_qscore) || 0;
          // BARK rate > φ⁻² (38.2%) = quality degradation
          if (barkRate > 0.382) {
            findings.push({ type: 'judgment_quality', message: `High BARK rate: ${Math.round(barkRate * 100)}% of ${total} judgments (avg Q: ${Math.round(avgQ)})`, severity: 'high' });
          }
          // Average Q-Score below 50 = systemic quality issue
          if (avgQ < 50 && total >= 10) {
            findings.push({ type: 'judgment_drift', message: `Low avg Q-Score: ${Math.round(avgQ)} across ${total} judgments (${jq.howl_count} HOWLs, ${jq.bark_count} BARKs)`, severity: 'medium' });
          }
        }
      } catch (e) { /* table may not exist */ }

      // 10. Reasoning trajectory quality (last 24h, from reasoning_trajectories)
      try {
        const { rows: [trj] } = await this.pool.query(
          `SELECT COUNT(*) AS total,
                  AVG(coherence_score) AS avg_coherence,
                  AVG(efficiency_score) AS avg_efficiency,
                  AVG(step_count) AS avg_steps,
                  COUNT(*) FILTER (WHERE outcome_feedback != 'pending' AND outcome_feedback IS NOT NULL) AS feedback_count,
                  COUNT(*) FILTER (WHERE outcome_feedback = 'incorrect') AS incorrect_count
           FROM reasoning_trajectories
           WHERE created_at > NOW() - INTERVAL '24 hours'`
        );
        if (trj && parseInt(trj.total) >= 3) {
          const total = parseInt(trj.total);
          const avgCoherence = parseFloat(trj.avg_coherence) || 0;
          const avgEfficiency = parseFloat(trj.avg_efficiency) || 0;
          const feedbackCount = parseInt(trj.feedback_count) || 0;
          const incorrectCount = parseInt(trj.incorrect_count) || 0;
          // Low coherence = reasoning paths are disconnected
          if (avgCoherence < 0.382 && total >= 5) {
            findings.push({ type: 'trajectory_coherence', message: `Low trajectory coherence: ${Math.round(avgCoherence * 100)}% avg across ${total} trajectories`, severity: 'medium' });
          }
          // High incorrect rate among feedback trajectories
          if (feedbackCount >= 3 && incorrectCount / feedbackCount > 0.382) {
            findings.push({ type: 'trajectory_accuracy', message: `Trajectory accuracy issue: ${incorrectCount}/${feedbackCount} marked incorrect (${Math.round(incorrectCount / feedbackCount * 100)}%)`, severity: 'high' });
          }
        }
      } catch (e) { /* table may not exist */ }

      // 8. Calibration drift (last 24h)
      try {
        const { rows: [cal] } = await this.pool.query(
          `SELECT COUNT(*) AS total,
                  COUNT(*) FILTER (WHERE predicted_outcome = actual_outcome) AS correct,
                  AVG(predicted_confidence) AS avg_confidence
           FROM calibration_tracking
           WHERE created_at > NOW() - INTERVAL '24 hours'`
        );
        if (cal && parseInt(cal.total) >= 5) {
          const accuracy = parseInt(cal.correct) / parseInt(cal.total);
          const avgConf = parseFloat(cal.avg_confidence) || 0;
          const drift = Math.abs(accuracy - avgConf);
          if (drift > 0.236) {
            findings.push({ type: 'calibration_drift', message: `Calibration drift: accuracy ${Math.round(accuracy * 100)}% vs predicted ${Math.round(avgConf * 100)}% (drift: ${Math.round(drift * 100)}%)`, severity: drift > 0.382 ? 'high' : 'medium' });
          }
        }
      } catch (e) { /* table may not exist */ }

      this.stats.dataGraveRuns++;
      this.stats.lastDataGraveRun = Date.now();
      this.stats.dataGraveFindings += findings.length;

      if (findings.length > 0) {
        log.info('Data grave analysis findings', {
          count: findings.length,
          findings: findings.map(f => `[${f.severity}] ${f.type}: ${f.message}`),
          runNumber: this.stats.dataGraveRuns,
        });

        this.eventBus.publish(EventType.AUTOMATION_TICK, {
          subType: 'data_grave_analysis',
          findings,
          timestamp: Date.now(),
        }, { source: 'AutomationExecutor' });
      }
    } catch (err) {
      this.stats.errors++;
      log.warn('Data grave analysis failed', { error: err.message });
    }
  }

  /**
   * Run pattern learning cycle (F10 = 55 min Fibonacci).
   * Extracts patterns from judgments, applies confidence decay, clusters similar patterns.
   * Activates the PatternLearning service that was built but never wired.
   * @private
   */
  async _runPatternLearning() {
    if (!this.patternLearning) return;

    try {
      const result = await this.patternLearning.runCycle();
      this.stats.patternLearningRuns++;
      this.stats.lastPatternLearningRun = Date.now();
      this.stats.patternsExtracted += result.extraction?.extracted?.length || 0;
      this.stats.patternsDecayed += result.decay?.decayed || 0;

      if ((result.extraction?.extracted?.length || 0) > 0 || (result.decay?.decayed || 0) > 0) {
        log.info('Pattern learning cycle results', {
          extracted: result.extraction?.extracted?.length || 0,
          decayed: result.decay?.decayed || 0,
          merged: result.clustering?.totalMerged || 0,
          duration: result.duration,
        });

        this.eventBus.publish(EventType.AUTOMATION_TICK, {
          subType: 'pattern_learning',
          extracted: result.extraction?.extracted?.length || 0,
          decayed: result.decay?.decayed || 0,
          merged: result.clustering?.totalMerged || 0,
          timestamp: Date.now(),
        }, { source: 'AutomationExecutor' });
      }
    } catch (err) {
      this.stats.errors++;
      log.warn('Pattern learning cycle failed', { error: err.message });
    }
  }

  /**
   * Manually trigger a governance review
   * @returns {Promise<Object>} Review result
   */
  async triggerGovernanceReview() {
    return this._runGovernanceReview();
  }

  /**
   * Manually trigger a learning cycle
   * @returns {Promise<Object>} Cycle result
   */
  async triggerLearningCycle() {
    return this._runLearningCycle('manual');
  }

  /**
   * Get executor statistics
   * @returns {Object} Stats
   */
  getStats() {
    return {
      running: this.running,
      uptime: this.stats.startedAt ? Date.now() - this.stats.startedAt : 0,
      pendingFeedback: this._pendingFeedback.length,
      intervals: this.intervals,
      ...this.stats,
    };
  }

  /**
   * Add feedback directly (for testing or manual submission)
   * @param {Object} feedback - Feedback data
   */
  addFeedback(feedback) {
    this._handleFeedbackReceived(feedback);
  }
}

/**
 * Create an AutomationExecutor instance
 *
 * @param {Object} options - Options
 * @returns {AutomationExecutor}
 */
export function createAutomationExecutor(options) {
  return new AutomationExecutor(options);
}

export default AutomationExecutor;
