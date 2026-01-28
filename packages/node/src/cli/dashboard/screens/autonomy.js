/**
 * CYNIC TUI Dashboard - Autonomy Screen
 *
 * Active goals, task queue, and notifications.
 * Phase 18: "CYNIC acts autonomously"
 *
 * @module @cynic/node/cli/dashboard/screens/autonomy
 */

'use strict';

import blessed from 'blessed';
import { COLORS, progressBar } from '../theme.js';

/**
 * Goal types with colors
 */
const GOAL_TYPES = {
  quality: { label: 'Quality', color: 'green', icon: '✓' },
  learning: { label: 'Learning', color: 'cyan', icon: '📚' },
  security: { label: 'Security', color: 'red', icon: '🔒' },
  maintenance: { label: 'Maintenance', color: 'yellow', icon: '🔧' },
};

/**
 * Task statuses with colors
 */
const TASK_STATUSES = {
  pending: { label: 'Pending', color: 'gray', icon: '○' },
  running: { label: 'Running', color: 'yellow', icon: '▶' },
  completed: { label: 'Completed', color: 'green', icon: '✓' },
  failed: { label: 'Failed', color: 'red', icon: '✗' },
  retry: { label: 'Retry', color: 'cyan', icon: '↻' },
};

/**
 * Create Autonomy Screen
 */
export function createAutonomyScreen(screen, dataFetcher, options = {}) {
  const container = blessed.box({
    parent: screen,
    top: 0,
    left: 0,
    width: '100%',
    height: '100%',
    tags: true,
    hidden: true,
  });

  // Header
  const header = blessed.box({
    parent: container,
    top: 0,
    left: 0,
    width: '100%',
    height: 1,
    style: { bg: 'green', fg: 'black' },
    content: ' {bold}🤖 AUTONOMY - Goals & Tasks{/}',
    tags: true,
  });

  // Goals panel
  const goalsPanel = blessed.box({
    parent: container,
    label: ' Active Goals ',
    top: 1,
    left: 0,
    width: '50%',
    height: '45%',
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.success },
      label: { fg: COLORS.success, bold: true },
    },
    tags: true,
  });

  const goalsList = blessed.list({
    parent: goalsPanel,
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
    style: {
      selected: { bg: 'green', fg: 'black' },
    },
    tags: true,
    scrollable: true,
    keys: true,
    vi: true,
    mouse: true,
  });

  // Tasks panel
  const tasksPanel = blessed.box({
    parent: container,
    label: ' Task Queue ',
    top: 1,
    left: '50%',
    width: '50%',
    height: '45%',
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.warning },
      label: { fg: COLORS.warning, bold: true },
    },
    tags: true,
  });

  const tasksList = blessed.list({
    parent: tasksPanel,
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
    style: {
      selected: { bg: 'yellow', fg: 'black' },
    },
    tags: true,
    scrollable: true,
    keys: true,
    vi: true,
    mouse: true,
  });

  // Notifications panel
  const notificationsPanel = blessed.box({
    parent: container,
    label: ' Notifications ',
    top: '46%',
    left: 0,
    width: '100%',
    height: '35%',
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.primary },
      label: { fg: COLORS.primary, bold: true },
    },
    tags: true,
  });

  const notificationsList = blessed.list({
    parent: notificationsPanel,
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
    style: {
      selected: { bg: 'blue', fg: 'white' },
    },
    tags: true,
    scrollable: true,
    keys: true,
    vi: true,
    mouse: true,
  });

  // Stats bar
  const statsBar = blessed.box({
    parent: container,
    label: ' Daemon Status ',
    top: '81%',
    left: 0,
    width: '100%',
    height: 4,
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.primary },
      label: { fg: COLORS.primary, bold: true },
    },
    tags: true,
  });

  // Footer
  const footer = blessed.box({
    parent: container,
    bottom: 0,
    left: 0,
    width: '100%',
    height: 1,
    style: { bg: 'black', fg: 'white' },
    content: ' {bold}[G]{/} Goals  {bold}[T]{/} Tasks  {bold}[N]{/} Notifications  {bold}[R]{/}efresh  {bold}[B]{/}ack  {bold}[Q]{/}uit',
    tags: true,
  });

  // Cache data
  let goals = [];
  let tasks = [];
  let notifications = [];
  let daemonStats = {};
  let activePanel = 'goals'; // 'goals', 'tasks', 'notifications'

  /**
   * Fetch autonomy data
   */
  async function fetchData() {
    // Fetch goals
    const goalsResult = await dataFetcher.callTool('brain_goals', {
      action: 'list',
      status: 'active',
    });
    if (goalsResult.success) {
      goals = goalsResult.result.goals || [];
      updateGoalsList();
    }

    // Fetch tasks
    const tasksResult = await dataFetcher.callTool('brain_tasks', {
      action: 'list',
      limit: 20,
    });
    if (tasksResult.success) {
      tasks = tasksResult.result.tasks || [];
      updateTasksList();
    }

    // Fetch notifications
    const notifsResult = await dataFetcher.callTool('brain_notifications', {
      action: 'list',
      delivered: false,
    });
    if (notifsResult.success) {
      notifications = notifsResult.result.notifications || [];
      updateNotificationsList();
    }

    // Fetch daemon stats
    const statsResult = await dataFetcher.callTool('brain_health', {});
    if (statsResult.success && statsResult.result.daemon) {
      daemonStats = statsResult.result.daemon;
      updateStatsBar();
    }
  }

  /**
   * Update goals list
   */
  function updateGoalsList() {
    const items = goals.map(g => {
      const type = GOAL_TYPES[g.goal_type] || GOAL_TYPES.quality;
      const progress = g.progress || 0;
      const bar = progressBar(progress * 100, 10);

      return `${type.icon} {${type.color}-fg}${g.title?.slice(0, 25) || 'Untitled'}{/} ${bar} ${Math.round(progress * 100)}%`;
    });

    if (items.length === 0) {
      items.push('{gray-fg}No active goals{/}');
    }

    goalsList.setItems(items);
  }

  /**
   * Update tasks list
   */
  function updateTasksList() {
    const items = tasks.map(t => {
      const status = TASK_STATUSES[t.status] || TASK_STATUSES.pending;
      const scheduled = t.scheduled_for
        ? new Date(t.scheduled_for).toLocaleTimeString()
        : '';

      return `${status.icon} {${status.color}-fg}${t.task_type?.slice(0, 20) || 'task'}{/} ${scheduled}`;
    });

    if (items.length === 0) {
      items.push('{gray-fg}No pending tasks{/}');
    }

    tasksList.setItems(items);
  }

  /**
   * Update notifications list
   */
  function updateNotificationsList() {
    const items = notifications.map(n => {
      const icon = n.notification_type === 'warning' ? '⚠️' :
                   n.notification_type === 'insight' ? '💡' :
                   n.notification_type === 'achievement' ? '🏆' : '📬';
      const delivered = n.delivered ? '{gray-fg}(read){/}' : '';

      return `${icon} ${n.title?.slice(0, 40) || 'Notification'} ${delivered}`;
    });

    if (items.length === 0) {
      items.push('{gray-fg}No notifications{/}');
    }

    notificationsList.setItems(items);
  }

  /**
   * Update stats bar
   */
  function updateStatsBar() {
    const s = daemonStats;
    const running = s.running ? '{green-fg}RUNNING{/}' : '{red-fg}STOPPED{/}';
    const uptime = s.uptime ? formatUptime(s.uptime) : 'N/A';

    const line = ` Status: ${running}  |  ` +
      `Uptime: {cyan-fg}${uptime}{/}  |  ` +
      `Tasks Processed: {yellow-fg}${s.tasksProcessed || 0}{/}  |  ` +
      `Goals Updated: {green-fg}${s.goalsUpdated || 0}{/}  |  ` +
      `Learning Cycles: {magenta-fg}${s.learningCycles || 0}{/}`;

    statsBar.setContent(line);
  }

  /**
   * Format uptime
   */
  function formatUptime(ms) {
    const hours = Math.floor(ms / 3600000);
    const mins = Math.floor((ms % 3600000) / 60000);
    return `${hours}h ${mins}m`;
  }

  /**
   * Focus goals panel
   */
  function focusGoals() {
    activePanel = 'goals';
    goalsList.focus();
    screen.render();
  }

  /**
   * Focus tasks panel
   */
  function focusTasks() {
    activePanel = 'tasks';
    tasksList.focus();
    screen.render();
  }

  /**
   * Focus notifications panel
   */
  function focusNotifications() {
    activePanel = 'notifications';
    notificationsList.focus();
    screen.render();
  }

  /**
   * Get selected goal
   */
  function getSelectedGoal() {
    return goals[goalsList.selected || 0];
  }

  /**
   * Get selected task
   */
  function getSelectedTask() {
    return tasks[tasksList.selected || 0];
  }

  /**
   * Update with data
   */
  function update(data) {
    if (data?.goals) {
      goals = data.goals;
      updateGoalsList();
    }
    if (data?.tasks) {
      tasks = data.tasks;
      updateTasksList();
    }
    if (data?.notifications) {
      notifications = data.notifications;
      updateNotificationsList();
    }
    if (data?.daemonStats) {
      daemonStats = data.daemonStats;
      updateStatsBar();
    }
  }

  /**
   * Show the screen
   */
  async function show() {
    container.show();
    goalsList.focus();
    await fetchData();
    screen.render();
  }

  /**
   * Hide the screen
   */
  function hide() {
    container.hide();
  }

  return {
    container,
    update,
    show,
    hide,
    focusGoals,
    focusTasks,
    focusNotifications,
    getSelectedGoal,
    getSelectedTask,
    fetchData,
  };
}

export default createAutonomyScreen;
