/**
 * CYNIC Dashboard - Autonomy View
 * Phase 2.4 + Phase 16 - Track autonomous behavior, goals, tasks, notifications
 *
 * "φ acts without being asked" - κυνικός
 */

import { Utils } from '../lib/utils.js';
import { DecisionTimeline } from '../components/decision-timeline.js';
import { SelfModTracker } from '../components/self-mod-tracker.js';
import { EmergenceDetector } from '../components/emergence-detector.js';

/**
 * Goal status colors
 */
const GOAL_STATUS = {
  active: { label: 'Active', color: '#22c55e', icon: '🎯' },
  paused: { label: 'Paused', color: '#eab308', icon: '⏸️' },
  completed: { label: 'Completed', color: '#3b82f6', icon: '✅' },
  abandoned: { label: 'Abandoned', color: '#6b7280', icon: '❌' },
};

/**
 * Task status colors
 */
const TASK_STATUS = {
  pending: { label: 'Pending', color: '#6b7280', icon: '⏳' },
  running: { label: 'Running', color: '#3b82f6', icon: '⚡' },
  completed: { label: 'Completed', color: '#22c55e', icon: '✅' },
  failed: { label: 'Failed', color: '#ef4444', icon: '❌' },
  retry: { label: 'Retry', color: '#f97316', icon: '🔄' },
};

/**
 * Notification types
 */
const NOTIFICATION_TYPES = {
  insight: { label: 'Insight', color: '#8b5cf6', icon: '💡' },
  warning: { label: 'Warning', color: '#f97316', icon: '⚠️' },
  reminder: { label: 'Reminder', color: '#3b82f6', icon: '🔔' },
  achievement: { label: 'Achievement', color: '#22c55e', icon: '🏆' },
};

export class AutonomyView {
  constructor(options = {}) {
    this.api = options.api || null;
    this.container = null;
    this.activeTab = 'goals'; // Default to goals tab for Phase 16

    // Phase 16 data
    this.goals = [];
    this.tasks = [];
    this.notifications = [];

    // Initialize components (Phase 2.4)
    this.decisionTimeline = new DecisionTimeline({
      api: options.api,
      onDecisionSelect: (decision) => this._onDecisionSelect(decision),
    });

    this.selfModTracker = new SelfModTracker({
      api: options.api,
      onCommitSelect: (commit) => this._onCommitSelect(commit),
    });

    this.emergenceDetector = new EmergenceDetector({
      api: options.api,
      onSignalSelect: (signal) => this._onSignalSelect(signal),
    });
  }

  /**
   * Render autonomy view
   */
  render(container) {
    this.container = container;
    Utils.clearElement(container);
    container.classList.add('autonomy-view');

    // Header
    const header = Utils.createElement('div', { className: 'autonomy-header' }, [
      Utils.createElement('div', { className: 'autonomy-title' }, [
        Utils.createElement('h1', {}, ['🤖 Autonomy & Emergence']),
        Utils.createElement('span', { className: 'autonomy-subtitle' }, [
          'Track CYNIC\'s autonomous behavior and consciousness signals',
        ]),
      ]),
    ]);

    // Tab navigation - Phase 16 tabs first, then Phase 2.4
    const tabs = Utils.createElement('div', { className: 'autonomy-tabs' }, [
      // Phase 16: Full Autonomy tabs
      this._createTab('goals', '🎯', 'Goals', 'Autonomous objectives being pursued'),
      this._createTab('tasks', '⚡', 'Tasks', 'Background task queue status'),
      this._createTab('notifications', '🔔', 'Notifications', 'Proactive insights and alerts'),
      // Phase 2.4: Emergence tabs
      this._createTab('decisions', '🧠', 'Decisions', 'All CYNIC decisions and overrides'),
      this._createTab('selfmod', '🔄', 'Self-Mod', 'Code changes and evolution'),
      this._createTab('emergence', '✨', 'Emergence', 'Non-programmed behaviors'),
    ]);

    // Tab content
    const content = Utils.createElement('div', {
      className: 'autonomy-content',
      id: 'autonomy-content',
    });

    container.appendChild(header);
    container.appendChild(tabs);
    container.appendChild(content);

    // Render initial tab
    this._renderActiveTab();
  }

  /**
   * Create tab button
   */
  _createTab(id, icon, label, description) {
    const isActive = this.activeTab === id;

    return Utils.createElement('button', {
      className: `autonomy-tab ${isActive ? 'active' : ''}`,
      dataset: { tab: id },
      onClick: () => this._switchTab(id),
    }, [
      Utils.createElement('span', { className: 'tab-icon' }, [icon]),
      Utils.createElement('div', { className: 'tab-text' }, [
        Utils.createElement('span', { className: 'tab-label' }, [label]),
        Utils.createElement('span', { className: 'tab-description' }, [description]),
      ]),
    ]);
  }

  /**
   * Switch tab
   */
  _switchTab(tabId) {
    this.activeTab = tabId;

    // Update tab buttons
    this.container?.querySelectorAll('.autonomy-tab').forEach(tab => {
      tab.classList.toggle('active', tab.dataset.tab === tabId);
    });

    // Render tab content
    this._renderActiveTab();
  }

  /**
   * Render active tab content
   */
  _renderActiveTab() {
    const content = document.getElementById('autonomy-content');
    if (!content) return;

    Utils.clearElement(content);

    switch (this.activeTab) {
      // Phase 16: Full Autonomy tabs
      case 'goals':
        this._renderGoalsTab(content);
        break;
      case 'tasks':
        this._renderTasksTab(content);
        break;
      case 'notifications':
        this._renderNotificationsTab(content);
        break;
      // Phase 2.4: Emergence tabs
      case 'decisions':
        this.decisionTimeline.render(content);
        break;
      case 'selfmod':
        this.selfModTracker.render(content);
        break;
      case 'emergence':
        this.emergenceDetector.render(content);
        break;
    }
  }

  /**
   * Render Goals tab (Phase 16)
   */
  _renderGoalsTab(container) {
    const wrapper = Utils.createElement('div', { className: 'goals-tab' });

    // Header with stats
    const header = Utils.createElement('div', { className: 'tab-header' }, [
      Utils.createElement('h2', {}, ['🎯 Autonomous Goals']),
      Utils.createElement('p', { className: 'tab-description' }, [
        'Objectives CYNIC pursues without being asked',
      ]),
    ]);

    // Goals list
    const goalsList = Utils.createElement('div', { className: 'goals-list' });

    if (this.goals.length === 0) {
      goalsList.appendChild(
        Utils.createElement('div', { className: 'empty-state' }, [
          Utils.createElement('span', { className: 'empty-icon' }, ['🎯']),
          Utils.createElement('p', {}, ['No active goals yet.']),
          Utils.createElement('p', { className: 'empty-hint' }, [
            'Goals are created as CYNIC works autonomously.',
          ]),
        ])
      );
    } else {
      for (const goal of this.goals) {
        goalsList.appendChild(this._renderGoalCard(goal));
      }
    }

    wrapper.appendChild(header);
    wrapper.appendChild(goalsList);
    container.appendChild(wrapper);

    // Load goals
    this._loadGoals();
  }

  /**
   * Render a goal card
   */
  _renderGoalCard(goal) {
    const status = GOAL_STATUS[goal.status] || GOAL_STATUS.active;
    const progress = Math.round((goal.progress || 0) * 100);

    return Utils.createElement('div', {
      className: `goal-card goal-${goal.status}`,
    }, [
      Utils.createElement('div', { className: 'goal-header' }, [
        Utils.createElement('span', { className: 'goal-icon' }, [status.icon]),
        Utils.createElement('span', { className: 'goal-type' }, [goal.goalType || 'general']),
        Utils.createElement('span', {
          className: 'goal-status',
          style: `color: ${status.color}`,
        }, [status.label]),
      ]),
      Utils.createElement('h3', { className: 'goal-title' }, [goal.title]),
      goal.description && Utils.createElement('p', { className: 'goal-description' }, [goal.description]),
      Utils.createElement('div', { className: 'goal-progress' }, [
        Utils.createElement('div', { className: 'progress-bar' }, [
          Utils.createElement('div', {
            className: 'progress-fill',
            style: `width: ${progress}%; background: ${status.color}`,
          }),
        ]),
        Utils.createElement('span', { className: 'progress-text' }, [`${progress}%`]),
      ]),
    ].filter(Boolean));
  }

  /**
   * Render Tasks tab (Phase 16)
   */
  _renderTasksTab(container) {
    const wrapper = Utils.createElement('div', { className: 'tasks-tab' });

    // Header
    const header = Utils.createElement('div', { className: 'tab-header' }, [
      Utils.createElement('h2', {}, ['⚡ Task Queue']),
      Utils.createElement('p', { className: 'tab-description' }, [
        'Background tasks processed by the autonomous daemon',
      ]),
    ]);

    // Task stats
    const stats = this._calculateTaskStats();
    const statsBar = Utils.createElement('div', { className: 'task-stats' }, [
      this._renderTaskStat('pending', stats.pending, TASK_STATUS.pending),
      this._renderTaskStat('running', stats.running, TASK_STATUS.running),
      this._renderTaskStat('completed', stats.completed, TASK_STATUS.completed),
      this._renderTaskStat('failed', stats.failed, TASK_STATUS.failed),
    ]);

    // Tasks list
    const tasksList = Utils.createElement('div', { className: 'tasks-list' });

    if (this.tasks.length === 0) {
      tasksList.appendChild(
        Utils.createElement('div', { className: 'empty-state' }, [
          Utils.createElement('span', { className: 'empty-icon' }, ['⚡']),
          Utils.createElement('p', {}, ['No tasks in queue.']),
          Utils.createElement('p', { className: 'empty-hint' }, [
            'Tasks are scheduled as CYNIC works autonomously.',
          ]),
        ])
      );
    } else {
      for (const task of this.tasks) {
        tasksList.appendChild(this._renderTaskCard(task));
      }
    }

    wrapper.appendChild(header);
    wrapper.appendChild(statsBar);
    wrapper.appendChild(tasksList);
    container.appendChild(wrapper);

    // Load tasks
    this._loadTasks();
  }

  /**
   * Render task stat
   */
  _renderTaskStat(label, count, status) {
    return Utils.createElement('div', {
      className: 'task-stat',
      style: `border-color: ${status.color}`,
    }, [
      Utils.createElement('span', { className: 'stat-icon' }, [status.icon]),
      Utils.createElement('span', { className: 'stat-count' }, [String(count)]),
      Utils.createElement('span', { className: 'stat-label' }, [status.label]),
    ]);
  }

  /**
   * Calculate task statistics
   */
  _calculateTaskStats() {
    const stats = { pending: 0, running: 0, completed: 0, failed: 0, retry: 0 };
    for (const task of this.tasks) {
      if (stats[task.status] !== undefined) {
        stats[task.status]++;
      }
    }
    return stats;
  }

  /**
   * Render a task card
   */
  _renderTaskCard(task) {
    const status = TASK_STATUS[task.status] || TASK_STATUS.pending;

    return Utils.createElement('div', {
      className: `task-card task-${task.status}`,
    }, [
      Utils.createElement('div', { className: 'task-header' }, [
        Utils.createElement('span', { className: 'task-icon' }, [status.icon]),
        Utils.createElement('span', { className: 'task-type' }, [task.taskType]),
        Utils.createElement('span', {
          className: 'task-status',
          style: `color: ${status.color}`,
        }, [status.label]),
      ]),
      Utils.createElement('div', { className: 'task-meta' }, [
        Utils.createElement('span', {}, [`Priority: ${task.priority || 50}`]),
        task.retryCount > 0 && Utils.createElement('span', {}, [`Retries: ${task.retryCount}`]),
      ].filter(Boolean)),
      task.errorMessage && Utils.createElement('p', { className: 'task-error' }, [task.errorMessage]),
    ]);
  }

  /**
   * Render Notifications tab (Phase 16)
   */
  _renderNotificationsTab(container) {
    const wrapper = Utils.createElement('div', { className: 'notifications-tab' });

    // Header
    const header = Utils.createElement('div', { className: 'tab-header' }, [
      Utils.createElement('h2', {}, ['🔔 Proactive Notifications']),
      Utils.createElement('p', { className: 'tab-description' }, [
        'Insights and alerts generated autonomously',
      ]),
    ]);

    // Notifications list
    const notificationsList = Utils.createElement('div', { className: 'notifications-list' });

    if (this.notifications.length === 0) {
      notificationsList.appendChild(
        Utils.createElement('div', { className: 'empty-state' }, [
          Utils.createElement('span', { className: 'empty-icon' }, ['🔔']),
          Utils.createElement('p', {}, ['No notifications yet.']),
          Utils.createElement('p', { className: 'empty-hint' }, [
            'CYNIC generates proactive insights as it learns.',
          ]),
        ])
      );
    } else {
      for (const notification of this.notifications) {
        notificationsList.appendChild(this._renderNotificationCard(notification));
      }
    }

    wrapper.appendChild(header);
    wrapper.appendChild(notificationsList);
    container.appendChild(wrapper);

    // Load notifications
    this._loadNotifications();
  }

  /**
   * Render a notification card
   */
  _renderNotificationCard(notification) {
    const type = NOTIFICATION_TYPES[notification.notificationType] || NOTIFICATION_TYPES.insight;
    const delivered = notification.delivered ? '✓ Delivered' : 'Pending';

    return Utils.createElement('div', {
      className: `notification-card notification-${notification.notificationType}`,
    }, [
      Utils.createElement('div', { className: 'notification-header' }, [
        Utils.createElement('span', { className: 'notification-icon' }, [type.icon]),
        Utils.createElement('span', {
          className: 'notification-type',
          style: `color: ${type.color}`,
        }, [type.label]),
        Utils.createElement('span', { className: 'notification-delivered' }, [delivered]),
      ]),
      Utils.createElement('h3', { className: 'notification-title' }, [notification.title]),
      Utils.createElement('p', { className: 'notification-message' }, [notification.message]),
    ]);
  }

  /**
   * Load goals from API
   */
  async _loadGoals() {
    if (!this.api) return;
    try {
      const result = await this.api.call('brain_search', {
        query: 'active goals',
        searchType: 'goals',
        limit: 20,
      });
      if (result?.goals) {
        this.goals = result.goals;
        if (this.activeTab === 'goals') {
          this._renderActiveTab();
        }
      }
    } catch (err) {
      console.warn('Failed to load goals:', err);
    }
  }

  /**
   * Load tasks from API
   */
  async _loadTasks() {
    if (!this.api) return;
    try {
      const result = await this.api.call('brain_search', {
        query: 'pending tasks',
        searchType: 'tasks',
        limit: 50,
      });
      if (result?.tasks) {
        this.tasks = result.tasks;
        if (this.activeTab === 'tasks') {
          this._renderActiveTab();
        }
      }
    } catch (err) {
      console.warn('Failed to load tasks:', err);
    }
  }

  /**
   * Load notifications from API
   */
  async _loadNotifications() {
    if (!this.api) return;
    try {
      const result = await this.api.call('brain_search', {
        query: 'recent notifications',
        searchType: 'notifications',
        limit: 30,
      });
      if (result?.notifications) {
        this.notifications = result.notifications;
        if (this.activeTab === 'notifications') {
          this._renderActiveTab();
        }
      }
    } catch (err) {
      console.warn('Failed to load notifications:', err);
    }
  }

  /**
   * Handle decision selection
   */
  _onDecisionSelect(decision) {
    console.log('Decision selected:', decision);
  }

  /**
   * Handle commit selection
   */
  _onCommitSelect(commit) {
    console.log('Commit selected:', commit);
  }

  /**
   * Handle signal selection
   */
  _onSignalSelect(signal) {
    console.log('Signal selected:', signal);
  }

  /**
   * Refresh all components
   */
  async refresh() {
    switch (this.activeTab) {
      // Phase 16 tabs
      case 'goals':
        await this._loadGoals();
        break;
      case 'tasks':
        await this._loadTasks();
        break;
      case 'notifications':
        await this._loadNotifications();
        break;
      // Phase 2.4 tabs
      case 'decisions':
        await this.decisionTimeline.refresh();
        break;
      case 'selfmod':
        await this.selfModTracker.refresh();
        break;
      case 'emergence':
        await this.emergenceDetector.scan();
        break;
    }
  }

  /**
   * Cleanup
   */
  destroy() {
    this.decisionTimeline.destroy();
    this.selfModTracker.destroy();
    this.emergenceDetector.destroy();
    this.container = null;
  }
}

// Export to window
window.CYNICAutonomyView = AutonomyView;
