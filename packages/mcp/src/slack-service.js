/**
 * Slack Notification Service
 *
 * Bridges CYNIC's internal notifications to Slack channels.
 * Uses Slack Web API for direct integration.
 *
 * "The dog barks across channels" - κυνικός
 *
 * @module @cynic/mcp/slack-service
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';

const log = createLogger('SlackService');

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Notification type to Slack emoji mapping
 */
const TYPE_EMOJI = {
  insight: ':bulb:',
  warning: ':warning:',
  reminder: ':bell:',
  achievement: ':trophy:',
  pattern: ':repeat:',
  suggestion: ':thought_balloon:',
  learning: ':brain:',
  question: ':question:',
  danger: ':rotating_light:',
  success: ':white_check_mark:',
};

/**
 * Priority to color mapping (Slack attachment colors)
 */
const PRIORITY_COLORS = {
  critical: '#e74c3c', // Red
  high: '#f39c12',     // Orange
  medium: '#f1c40f',   // Yellow
  low: '#27ae60',      // Green
  info: '#3498db',     // Blue
};

/**
 * Default channel for notifications
 */
const DEFAULT_CHANNEL = process.env.CYNIC_SLACK_CHANNEL || '#cynic-alerts';

// ═══════════════════════════════════════════════════════════════════════════
// SLACK SERVICE CLASS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Slack Notification Service
 *
 * Provides integration with Slack for sending CYNIC notifications,
 * alerts, and insights to configured channels.
 *
 * @example
 * const slack = new SlackService({ token: process.env.SLACK_BOT_TOKEN });
 * await slack.sendNotification({
 *   type: 'warning',
 *   title: 'Guardian Alert',
 *   message: 'Dangerous operation blocked',
 *   priority: 'high',
 * });
 */
export class SlackService {
  /**
   * @param {Object} options - Configuration options
   * @param {string} [options.token] - Slack Bot Token (xoxb-...)
   * @param {string} [options.channel] - Default channel for notifications
   * @param {boolean} [options.enabled] - Enable/disable notifications
   * @param {number} [options.rateLimit] - Max messages per minute (default: 20)
   */
  constructor(options = {}) {
    this.token = options.token || process.env.SLACK_BOT_TOKEN;
    this.defaultChannel = options.channel || DEFAULT_CHANNEL;
    this.enabled = options.enabled !== false && !!this.token;
    this.rateLimit = options.rateLimit || 20;

    // Rate limiting
    this._messageQueue = [];
    this._sentThisMinute = 0;
    this._lastReset = Date.now();

    // Stats
    this._stats = {
      sent: 0,
      failed: 0,
      rateLimited: 0,
      queued: 0,
    };

    if (this.enabled) {
      log.info('Slack service initialized', { channel: this.defaultChannel });
    } else {
      log.warn('Slack service disabled (no token)');
    }
  }

  // =========================================================================
  // CORE API
  // =========================================================================

  /**
   * Send a notification to Slack
   *
   * @param {Object} notification - Notification data
   * @param {string} notification.type - Notification type (warning, insight, etc.)
   * @param {string} notification.title - Notification title
   * @param {string} notification.message - Notification message
   * @param {string} [notification.priority] - Priority (critical, high, medium, low, info)
   * @param {Object} [notification.context] - Additional context data
   * @param {string} [notification.channel] - Override default channel
   * @returns {Promise<Object>} Slack API response
   */
  async sendNotification(notification) {
    if (!this.enabled) {
      return { ok: false, error: 'slack_disabled' };
    }

    // Rate limiting check
    this._checkRateLimit();
    if (this._sentThisMinute >= this.rateLimit) {
      this._stats.rateLimited++;
      this._messageQueue.push(notification);
      this._stats.queued = this._messageQueue.length;
      log.warn('Slack rate limited, message queued');
      return { ok: false, error: 'rate_limited', queued: true };
    }

    const channel = notification.channel || this.defaultChannel;
    const emoji = TYPE_EMOJI[notification.type] || ':dog:';
    const color = PRIORITY_COLORS[notification.priority] || PRIORITY_COLORS.info;

    // Build Slack message with blocks
    const payload = {
      channel,
      text: `${emoji} ${notification.title}`, // Fallback text
      attachments: [
        {
          color,
          blocks: this._buildBlocks(notification, emoji),
        },
      ],
    };

    try {
      const response = await this._postMessage(payload);
      this._sentThisMinute++;
      this._stats.sent++;
      log.debug('Slack notification sent', { channel, type: notification.type });
      return response;
    } catch (error) {
      this._stats.failed++;
      log.error('Slack notification failed', { error: error.message });
      return { ok: false, error: error.message };
    }
  }

  /**
   * Send a CYNIC judgment result to Slack
   *
   * @param {Object} judgment - Judgment result
   * @param {string} [channel] - Override default channel
   * @returns {Promise<Object>} Slack API response
   */
  async sendJudgment(judgment, channel = null) {
    const verdictEmoji = {
      HOWL: ':wolf:',   // Strong approval
      WAG: ':dog2:',    // Mild approval
      BARK: ':dog:',    // Caution
      GROWL: ':rage:',  // Danger
    };

    const verdictColor = {
      HOWL: '#27ae60',  // Green
      WAG: '#f1c40f',   // Yellow
      BARK: '#f39c12',  // Orange
      GROWL: '#e74c3c', // Red
    };

    const emoji = verdictEmoji[judgment.verdict] || ':question:';
    const color = verdictColor[judgment.verdict] || PRIORITY_COLORS.info;

    return this.sendNotification({
      type: 'judgment',
      title: `${emoji} CYNIC Judgment: ${judgment.verdict}`,
      message: judgment.summary || `Q-Score: ${(judgment.qScore * 100).toFixed(1)}%`,
      priority: judgment.verdict === 'GROWL' ? 'critical' :
                judgment.verdict === 'BARK' ? 'high' :
                judgment.verdict === 'WAG' ? 'medium' : 'low',
      context: {
        qScore: judgment.qScore,
        verdict: judgment.verdict,
        confidence: judgment.confidence,
        dimensions: judgment.dimensions?.length || 0,
      },
      channel: channel || this.defaultChannel,
    });
  }

  /**
   * Send a danger alert (Guardian detection)
   *
   * @param {Object} alert - Alert data
   * @param {string} alert.threat - Threat description
   * @param {string} alert.action - Action taken (blocked, warned)
   * @param {Object} [alert.context] - Additional context
   * @returns {Promise<Object>} Slack API response
   */
  async sendDangerAlert(alert) {
    return this.sendNotification({
      type: 'danger',
      title: ':rotating_light: GUARDIAN ALERT',
      message: alert.threat,
      priority: 'critical',
      context: {
        action: alert.action,
        ...alert.context,
      },
    });
  }

  /**
   * Send a pattern detection notification
   *
   * @param {Object} pattern - Pattern data
   * @param {string} pattern.name - Pattern name
   * @param {number} pattern.occurrences - Number of occurrences
   * @param {string} [pattern.recommendation] - Suggested action
   * @returns {Promise<Object>} Slack API response
   */
  async sendPatternDetected(pattern) {
    return this.sendNotification({
      type: 'pattern',
      title: `:repeat: Pattern Detected: "${pattern.name}"`,
      message: `Seen ${pattern.occurrences}x. ${pattern.recommendation || ''}`,
      priority: pattern.occurrences > 5 ? 'medium' : 'low',
      context: {
        name: pattern.name,
        occurrences: pattern.occurrences,
      },
    });
  }

  /**
   * Send session summary to Slack
   *
   * @param {Object} summary - Session summary
   * @returns {Promise<Object>} Slack API response
   */
  async sendSessionSummary(summary) {
    const blocks = [
      {
        type: 'header',
        text: {
          type: 'plain_text',
          text: ':dog: CYNIC Session Summary',
        },
      },
      {
        type: 'section',
        fields: [
          {
            type: 'mrkdwn',
            text: `*Duration:*\n${summary.duration || 'N/A'}`,
          },
          {
            type: 'mrkdwn',
            text: `*Judgments:*\n${summary.judgmentCount || 0}`,
          },
          {
            type: 'mrkdwn',
            text: `*Patterns:*\n+${summary.newPatterns || 0}`,
          },
          {
            type: 'mrkdwn',
            text: `*Efficiency (η):*\n${((summary.efficiency || 0) * 100).toFixed(1)}%`,
          },
        ],
      },
    ];

    if (summary.activeDogs?.length > 0) {
      blocks.push({
        type: 'section',
        text: {
          type: 'mrkdwn',
          text: `*Most Active Dogs:* ${summary.activeDogs.join(', ')}`,
        },
      });
    }

    return this._postMessage({
      channel: this.defaultChannel,
      text: 'CYNIC Session Summary',
      blocks,
    });
  }

  // =========================================================================
  // INTERNAL METHODS
  // =========================================================================

  /**
   * Build Slack blocks from notification
   * @private
   */
  _buildBlocks(notification, emoji) {
    const blocks = [
      {
        type: 'section',
        text: {
          type: 'mrkdwn',
          text: `${emoji} *${notification.title}*\n${notification.message}`,
        },
      },
    ];

    // Add context fields if present
    if (notification.context && Object.keys(notification.context).length > 0) {
      const fields = Object.entries(notification.context)
        .slice(0, 8) // Max 8 fields
        .map(([key, value]) => ({
          type: 'mrkdwn',
          text: `*${key}:* ${typeof value === 'object' ? JSON.stringify(value) : value}`,
        }));

      if (fields.length > 0) {
        blocks.push({
          type: 'section',
          fields,
        });
      }
    }

    // Add timestamp footer
    blocks.push({
      type: 'context',
      elements: [
        {
          type: 'mrkdwn',
          text: `:clock1: ${new Date().toISOString()} | φ confidence: ${(PHI_INV * 100).toFixed(1)}% max`,
        },
      ],
    });

    return blocks;
  }

  /**
   * Post message to Slack API
   * @private
   */
  async _postMessage(payload) {
    const response = await fetch('https://slack.com/api/chat.postMessage', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.token}`,
      },
      body: JSON.stringify(payload),
    });

    const data = await response.json();

    if (!data.ok) {
      throw new Error(data.error || 'Slack API error');
    }

    return data;
  }

  /**
   * Check and reset rate limit counter
   * @private
   */
  _checkRateLimit() {
    const now = Date.now();
    if (now - this._lastReset >= 60000) {
      this._sentThisMinute = 0;
      this._lastReset = now;

      // Process queued messages
      this._processQueue();
    }
  }

  /**
   * Process queued messages after rate limit reset
   * @private
   */
  async _processQueue() {
    const toProcess = this._messageQueue.splice(0, this.rateLimit);
    for (const notification of toProcess) {
      await this.sendNotification(notification);
    }
    this._stats.queued = this._messageQueue.length;
  }

  // =========================================================================
  // STATS & STATUS
  // =========================================================================

  /**
   * Get service statistics
   * @returns {Object} Stats
   */
  getStats() {
    return {
      ...this._stats,
      enabled: this.enabled,
      channel: this.defaultChannel,
      rateLimit: this.rateLimit,
      sentThisMinute: this._sentThisMinute,
    };
  }

  /**
   * Check if service is healthy
   * @returns {Promise<boolean>}
   */
  async healthCheck() {
    if (!this.enabled) return false;

    try {
      const response = await fetch('https://slack.com/api/auth.test', {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${this.token}`,
        },
      });
      const data = await response.json();
      return data.ok === true;
    } catch {
      return false;
    }
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// FACTORY
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create a Slack service instance
 *
 * @param {Object} [options] - Configuration options
 * @returns {SlackService} Service instance
 */
export function createSlackService(options = {}) {
  return new SlackService(options);
}

// ═══════════════════════════════════════════════════════════════════════════
// NOTIFICATION BRIDGE
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Bridge CYNIC ProactiveNotifications to Slack
 *
 * @param {SlackService} slackService - Slack service instance
 * @param {Object} notificationsRepo - ProactiveNotificationsRepository
 * @param {Object} [options] - Bridge options
 * @returns {Object} Bridge controller
 */
export function createNotificationBridge(slackService, notificationsRepo, options = {}) {
  const {
    typesToBridge = ['warning', 'insight', 'achievement', 'pattern'],
    minPriority = 50,
  } = options;

  return {
    /**
     * Bridge a notification to Slack
     */
    async bridge(notification) {
      // Filter by type and priority
      if (!typesToBridge.includes(notification.notificationType)) {
        return { bridged: false, reason: 'type_filtered' };
      }
      if (notification.priority < minPriority) {
        return { bridged: false, reason: 'priority_filtered' };
      }

      // Send to Slack
      const result = await slackService.sendNotification({
        type: notification.notificationType,
        title: notification.title,
        message: notification.message,
        priority: notification.priority >= 80 ? 'critical' :
                  notification.priority >= 60 ? 'high' :
                  notification.priority >= 40 ? 'medium' : 'low',
        context: notification.context,
      });

      return { bridged: result.ok, slackResult: result };
    },

    /**
     * Bridge all pending notifications
     */
    async bridgePending(userId, limit = 5) {
      const pending = await notificationsRepo.getPending(userId, limit);
      const results = [];

      for (const notification of pending) {
        const result = await this.bridge(notification);
        results.push({ id: notification.id, ...result });
      }

      return results;
    },
  };
}

export default SlackService;
