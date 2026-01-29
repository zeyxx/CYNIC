/**
 * Total Memory Hook Module
 *
 * Provides Total Memory integration for Claude Code hooks.
 * Lazy-loaded to avoid startup cost when not needed.
 *
 * "φ remembers everything" - CYNIC
 *
 * @module scripts/lib/total-memory
 */

'use strict';

const path = require('path');

// Lazy-loaded instances
let _pool = null;
let _memoryRetriever = null;
let _goalsRepo = null;
let _notificationsRepo = null;
let _tasksRepo = null;
let _initialized = false;

/**
 * φ constants
 */
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;

/**
 * Initialize Total Memory system
 * Connects to PostgreSQL and creates repositories
 */
async function init() {
  if (_initialized) return true;

  try {
    // Dynamic import of ESM modules
    const { getPool } = await import('@cynic/persistence');
    const { MemoryRetriever } = await import('@cynic/persistence');
    const {
      AutonomousGoalsRepository,
      AutonomousTasksRepository,
      ProactiveNotificationsRepository,
    } = await import('@cynic/persistence');

    _pool = getPool();

    // Create repositories
    _memoryRetriever = new MemoryRetriever({ pool: _pool });
    _goalsRepo = new AutonomousGoalsRepository(_pool);
    _notificationsRepo = new ProactiveNotificationsRepository(_pool);
    _tasksRepo = new AutonomousTasksRepository(_pool);

    _initialized = true;
    return true;
  } catch (e) {
    console.error('[CYNIC] Total Memory init failed:', e.message);
    return false;
  }
}

/**
 * Get MemoryRetriever instance
 * @returns {Object|null} MemoryRetriever or null if not initialized
 */
function getMemoryRetriever() {
  return _memoryRetriever;
}

/**
 * Get GoalsRepository instance
 * @returns {Object|null} Repository or null if not initialized
 */
function getGoalsRepo() {
  return _goalsRepo;
}

/**
 * Get NotificationsRepository instance
 * @returns {Object|null} Repository or null if not initialized
 */
function getNotificationsRepo() {
  return _notificationsRepo;
}

/**
 * Get TasksRepository instance
 * @returns {Object|null} Repository or null if not initialized
 */
function getTasksRepo() {
  return _tasksRepo;
}

// ═══════════════════════════════════════════════════════════════════════════
// SESSION START HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Load relevant memories for session start
 * @param {string} userId - User ID
 * @param {Object} context - Session context (project, topics, etc)
 * @returns {Promise<Object>} Memories and context
 */
async function loadSessionMemories(userId, context = {}) {
  if (!_initialized) await init();
  if (!_memoryRetriever) return { memories: [], decisions: [], lessons: [] };

  try {
    // Build search query from context
    const queryParts = [];
    if (context.projectPath) queryParts.push(path.basename(context.projectPath));
    if (context.projectName) queryParts.push(context.projectName);
    if (context.recentTopics) queryParts.push(...context.recentTopics.slice(0, 3));

    const query = queryParts.join(' ') || 'recent session';

    const results = await _memoryRetriever.search(userId, query, {
      limit: 5,
      sources: ['memories', 'decisions', 'lessons'],
      minRelevance: PHI_INV_2, // 0.382 threshold
    });

    return {
      memories: results.sources?.memories || [],
      decisions: results.sources?.decisions || [],
      lessons: results.sources?.lessons || [],
      query,
    };
  } catch (e) {
    console.error('[CYNIC] Load session memories failed:', e.message);
    return { memories: [], decisions: [], lessons: [] };
  }
}

/**
 * Get pending notifications for session start
 * @param {string} userId - User ID
 * @param {number} limit - Max notifications
 * @returns {Promise<Array>} Pending notifications
 */
async function getPendingNotifications(userId, limit = 5) {
  if (!_initialized) await init();
  if (!_notificationsRepo) return [];

  try {
    return await _notificationsRepo.getPending(userId, limit);
  } catch (e) {
    console.error('[CYNIC] Get notifications failed:', e.message);
    return [];
  }
}

/**
 * Get active goals for session start
 * @param {string} userId - User ID
 * @returns {Promise<Array>} Active goals
 */
async function getActiveGoals(userId) {
  if (!_initialized) await init();
  if (!_goalsRepo) return [];

  try {
    // Use findActive() - the correct method for active goals
    return await _goalsRepo.findActive(userId, 5);
  } catch (e) {
    console.error('[CYNIC] Get goals failed:', e.message);
    return [];
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// STOP/DIGEST HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Store a conversation memory
 * @param {string} userId - User ID
 * @param {string} memoryType - Type of memory
 * @param {string} content - Content to remember
 * @param {Object} options - Additional options
 */
async function rememberConversation(userId, memoryType, content, options = {}) {
  if (!_initialized) await init();
  if (!_memoryRetriever) return null;

  try {
    return await _memoryRetriever.rememberConversation(userId, memoryType, content, options);
  } catch (e) {
    console.error('[CYNIC] Remember conversation failed:', e.message);
    return null;
  }
}

/**
 * Store an architectural decision
 * @param {string} userId - User ID
 * @param {Object} decision - Decision data
 */
async function rememberDecision(userId, decision) {
  if (!_initialized) await init();
  if (!_memoryRetriever) return null;

  try {
    return await _memoryRetriever.rememberDecision(userId, decision);
  } catch (e) {
    console.error('[CYNIC] Remember decision failed:', e.message);
    return null;
  }
}

/**
 * Store a lesson learned
 * @param {string} userId - User ID
 * @param {Object} lesson - Lesson data
 */
async function rememberLesson(userId, lesson) {
  if (!_initialized) await init();
  if (!_memoryRetriever) return null;

  try {
    return await _memoryRetriever.rememberLesson(userId, lesson);
  } catch (e) {
    console.error('[CYNIC] Remember lesson failed:', e.message);
    return null;
  }
}

/**
 * Check for similar past mistakes before action
 * @param {string} userId - User ID
 * @param {string} action - Proposed action description
 */
async function checkForMistakes(userId, action) {
  if (!_initialized) await init();
  if (!_memoryRetriever) return { warning: false };

  try {
    return await _memoryRetriever.checkForMistakes(userId, action);
  } catch (e) {
    console.error('[CYNIC] Check mistakes failed:', e.message);
    return { warning: false };
  }
}

/**
 * Extract and store session summary
 * @param {string} userId - User ID
 * @param {Object} sessionData - Session data to summarize
 */
async function storeSessionSummary(userId, sessionData) {
  if (!_initialized) await init();
  if (!_memoryRetriever) return null;

  try {
    const summary = [
      `Session: ${sessionData.toolsUsed || 0} tools used`,
      sessionData.errorsEncountered > 0 ? `${sessionData.errorsEncountered} errors` : null,
      sessionData.topTools?.length > 0 ? `Top tools: ${sessionData.topTools.join(', ')}` : null,
      sessionData.insights?.length > 0 ? `Insights: ${sessionData.insights.length}` : null,
    ].filter(Boolean).join('. ');

    return await _memoryRetriever.rememberConversation(userId, 'summary', summary, {
      importance: 0.6,
      context: {
        sessionId: sessionData.sessionId,
        duration: sessionData.duration,
        project: sessionData.project,
      },
    });
  } catch (e) {
    console.error('[CYNIC] Store session summary failed:', e.message);
    return null;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SESSION END HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Update goal progress based on session activity
 * @param {string} userId - User ID
 * @param {Object} activity - Session activity summary
 */
async function updateGoalProgress(userId, activity) {
  if (!_initialized) await init();
  if (!_goalsRepo) return;

  try {
    const goals = await _goalsRepo.findActive(userId);

    for (const goal of goals) {
      // Auto-progress based on goal type and activity
      let progressDelta = 0;

      switch (goal.goalType) {
        case 'quality':
          if (activity.testsRun > 0) progressDelta = 0.05;
          if (activity.errorsFixed > 0) progressDelta += 0.03;
          break;

        case 'learning':
          if (activity.lessonsLearned > 0) progressDelta = 0.1;
          if (activity.decisionsRecorded > 0) progressDelta += 0.05;
          break;

        case 'maintenance':
          if (activity.refactoring) progressDelta = 0.08;
          if (activity.techDebtReduced) progressDelta += 0.05;
          break;
      }

      if (progressDelta > 0) {
        const newProgress = Math.min(goal.progress + progressDelta, 1.0);
        await _goalsRepo.updateProgress(goal.id, newProgress);
      }
    }
  } catch (e) {
    console.error('[CYNIC] Update goal progress failed:', e.message);
  }
}

/**
 * Schedule background task for autonomous daemon
 * @param {string} userId - User ID
 * @param {string} taskType - Task type
 * @param {Object} payload - Task payload
 * @param {Object} options - Task options
 */
async function scheduleTask(userId, taskType, payload, options = {}) {
  if (!_initialized) await init();
  if (!_tasksRepo) return null;

  try {
    return await _tasksRepo.create({
      userId,
      taskType,
      payload,
      priority: options.priority || 50,
      scheduledFor: options.scheduledFor || new Date(),
      maxRetries: options.maxRetries || 3,
    });
  } catch (e) {
    console.error('[CYNIC] Schedule task failed:', e.message);
    return null;
  }
}

/**
 * Create proactive notification for next session
 * @param {string} userId - User ID
 * @param {Object} notification - Notification data
 */
async function createNotification(userId, notification) {
  if (!_initialized) await init();
  if (!_notificationsRepo) return null;

  try {
    return await _notificationsRepo.create({
      userId,
      notificationType: notification.type || 'insight',
      title: notification.title,
      message: notification.message,
      priority: notification.priority || 50,
      context: notification.context,
      expiresAt: notification.expiresAt,
    });
  } catch (e) {
    console.error('[CYNIC] Create notification failed:', e.message);
    return null;
  }
}

/**
 * Mark notifications as delivered
 * @param {string[]} notificationIds - Notification IDs to mark
 */
async function markNotificationsDelivered(notificationIds) {
  if (!_initialized) await init();
  if (!_notificationsRepo || !notificationIds?.length) return;

  try {
    for (const id of notificationIds) {
      await _notificationsRepo.markDelivered(id);
    }
  } catch (e) {
    console.error('[CYNIC] Mark notifications delivered failed:', e.message);
  }
}

/**
 * Get memory statistics
 * @param {string} userId - User ID
 */
async function getMemoryStats(userId) {
  if (!_initialized) await init();
  if (!_memoryRetriever) return null;

  try {
    return await _memoryRetriever.getStats(userId);
  } catch (e) {
    console.error('[CYNIC] Get memory stats failed:', e.message);
    return null;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// EXPORTS
// ═══════════════════════════════════════════════════════════════════════════

module.exports = {
  // Initialization
  init,

  // Instance getters
  getMemoryRetriever,
  getGoalsRepo,
  getNotificationsRepo,
  getTasksRepo,

  // Session start
  loadSessionMemories,
  getPendingNotifications,
  getActiveGoals,

  // Stop/Digest
  rememberConversation,
  rememberDecision,
  rememberLesson,
  checkForMistakes,
  storeSessionSummary,

  // Session end
  updateGoalProgress,
  scheduleTask,
  createNotification,
  markNotificationsDelivered,
  getMemoryStats,

  // Constants
  PHI,
  PHI_INV,
  PHI_INV_2,
};
