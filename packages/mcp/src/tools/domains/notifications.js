/**
 * Notifications Domain Tools
 *
 * MCP tools for external notifications:
 * - brain_notifications: View and manage internal notifications
 * - brain_slack_send: Send notification to Slack
 * - brain_slack_status: Check Slack integration status
 *
 * Bridges CYNIC's internal awareness to external channels.
 *
 * "The dog barks to those who listen" - κυνικός
 *
 * @module @cynic/mcp/tools/domains/notifications
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';

const log = createLogger('NotificationTools');

// ═══════════════════════════════════════════════════════════════════════════
// INTERNAL NOTIFICATIONS TOOL
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create internal notifications management tool
 *
 * @param {Object} notificationsRepo - ProactiveNotificationsRepository instance
 * @returns {Object} Tool definition
 */
export function createNotificationsTool(notificationsRepo) {
  return {
    name: 'brain_notifications',
    description: `View and manage CYNIC's proactive notifications.
Actions: pending (undelivered), recent (all recent), dismiss (mark as read), create (new notification)
Notifications are delivered at session start.`,
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['pending', 'recent', 'dismiss', 'create', 'stats'],
          description: 'Action: pending, recent, dismiss, create, or stats',
        },
        userId: {
          type: 'string',
          description: 'User ID (defaults to current session user)',
        },
        notificationId: {
          type: 'string',
          description: 'Notification ID (for dismiss action)',
        },
        notification: {
          type: 'object',
          description: 'Notification data (for create action)',
          properties: {
            type: {
              type: 'string',
              enum: ['insight', 'warning', 'reminder', 'achievement', 'pattern', 'suggestion', 'learning', 'question'],
            },
            title: { type: 'string' },
            message: { type: 'string' },
            priority: { type: 'number', minimum: 0, maximum: 100 },
          },
        },
        limit: {
          type: 'number',
          description: 'Maximum notifications to return (default: 10)',
        },
      },
    },
    handler: async (params) => {
      const {
        action = 'pending',
        userId = 'default',
        notificationId,
        notification,
        limit = 10,
      } = params;

      if (!notificationsRepo) {
        return {
          success: false,
          error: 'Notifications repository not available',
        };
      }

      try {
        switch (action) {
          case 'pending': {
            const pending = await notificationsRepo.getPending(userId, limit);
            return {
              success: true,
              action: 'pending',
              notifications: pending.map(formatNotification),
              count: pending.length,
            };
          }

          case 'recent': {
            const recent = await notificationsRepo.findRecent(userId, limit);
            return {
              success: true,
              action: 'recent',
              notifications: recent.map(formatNotification),
              count: recent.length,
            };
          }

          case 'dismiss': {
            if (!notificationId) {
              return { success: false, error: 'notificationId required for dismiss' };
            }
            const dismissed = await notificationsRepo.dismiss(notificationId, 'user_dismissed');
            return {
              success: !!dismissed,
              action: 'dismiss',
              notification: dismissed ? formatNotification(dismissed) : null,
            };
          }

          case 'create': {
            if (!notification || !notification.title || !notification.message) {
              return { success: false, error: 'notification.title and notification.message required' };
            }
            const created = await notificationsRepo.create({
              userId,
              notificationType: notification.type || 'insight',
              title: notification.title,
              message: notification.message,
              priority: notification.priority || 50,
              context: notification.context || {},
            });
            return {
              success: true,
              action: 'create',
              notification: formatNotification(created),
            };
          }

          case 'stats': {
            const stats = await notificationsRepo.getStats(userId);
            return {
              success: true,
              action: 'stats',
              stats,
            };
          }

          default:
            return { success: false, error: `Unknown action: ${action}` };
        }
      } catch (err) {
        log.error('Notifications tool error', { error: err.message });
        return {
          success: false,
          error: err.message,
        };
      }
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// SLACK TOOLS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create Slack send notification tool
 *
 * @param {Object} slackService - SlackService instance
 * @returns {Object} Tool definition
 */
export function createSlackSendTool(slackService) {
  return {
    name: 'brain_slack_send',
    description: `Send a notification to Slack. Requires SLACK_BOT_TOKEN.
Types: insight, warning, danger, achievement, pattern, suggestion
For judgments, use type: judgment with qScore and verdict in context.`,
    inputSchema: {
      type: 'object',
      properties: {
        type: {
          type: 'string',
          enum: ['insight', 'warning', 'danger', 'achievement', 'pattern', 'suggestion', 'judgment', 'session_summary'],
          description: 'Notification type',
        },
        title: {
          type: 'string',
          description: 'Notification title',
        },
        message: {
          type: 'string',
          description: 'Notification message/body',
        },
        priority: {
          type: 'string',
          enum: ['critical', 'high', 'medium', 'low', 'info'],
          description: 'Priority level (affects color)',
        },
        channel: {
          type: 'string',
          description: 'Slack channel (defaults to configured channel)',
        },
        context: {
          type: 'object',
          description: 'Additional context data to include',
        },
      },
      required: ['type', 'title', 'message'],
    },
    handler: async (params) => {
      const { type, title, message, priority = 'medium', channel, context = {} } = params;

      if (!slackService) {
        return {
          success: false,
          error: 'Slack service not available. Set SLACK_BOT_TOKEN.',
          message: '*head tilt* Slack not configured. Need SLACK_BOT_TOKEN environment variable.',
        };
      }

      if (!slackService.enabled) {
        return {
          success: false,
          error: 'Slack service disabled (no token)',
          message: '*sniff* Slack disabled. Check SLACK_BOT_TOKEN.',
        };
      }

      try {
        // Handle special types
        if (type === 'judgment' && context.qScore !== undefined) {
          const result = await slackService.sendJudgment({
            qScore: context.qScore,
            verdict: context.verdict || 'BARK',
            confidence: context.confidence || PHI_INV,
            summary: message,
          }, channel);
          return {
            success: result.ok,
            type: 'judgment',
            slackResult: result,
          };
        }

        if (type === 'danger') {
          const result = await slackService.sendDangerAlert({
            threat: message,
            action: context.action || 'detected',
            context,
          });
          return {
            success: result.ok,
            type: 'danger',
            slackResult: result,
          };
        }

        if (type === 'pattern') {
          const result = await slackService.sendPatternDetected({
            name: title,
            occurrences: context.occurrences || 1,
            recommendation: message,
          });
          return {
            success: result.ok,
            type: 'pattern',
            slackResult: result,
          };
        }

        if (type === 'session_summary') {
          const result = await slackService.sendSessionSummary({
            duration: context.duration,
            judgmentCount: context.judgmentCount,
            newPatterns: context.newPatterns,
            efficiency: context.efficiency,
            activeDogs: context.activeDogs,
          });
          return {
            success: result.ok,
            type: 'session_summary',
            slackResult: result,
          };
        }

        // Standard notification
        const result = await slackService.sendNotification({
          type,
          title,
          message,
          priority,
          channel,
          context,
        });

        return {
          success: result.ok,
          type,
          slackResult: result,
          stats: slackService.getStats(),
        };

      } catch (err) {
        log.error('Slack send error', { error: err.message });
        return {
          success: false,
          error: err.message,
          message: `*GROWL* Slack error: ${err.message}`,
        };
      }
    },
  };
}

/**
 * Create Slack status tool
 *
 * @param {Object} slackService - SlackService instance
 * @returns {Object} Tool definition
 */
export function createSlackStatusTool(slackService) {
  return {
    name: 'brain_slack_status',
    description: 'Check Slack integration status and health.',
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['status', 'health', 'stats'],
          description: 'Action: status (config), health (connection test), stats (usage)',
        },
      },
    },
    handler: async (params) => {
      const { action = 'status' } = params;

      if (!slackService) {
        return {
          success: false,
          enabled: false,
          error: 'Slack service not configured',
          setup: {
            required: ['SLACK_BOT_TOKEN'],
            optional: ['SLACK_TEAM_ID', 'CYNIC_SLACK_CHANNEL'],
            docs: 'Create Slack app at api.slack.com/apps with bot scopes: channels:history, chat:write',
          },
        };
      }

      try {
        switch (action) {
          case 'status': {
            return {
              success: true,
              action: 'status',
              enabled: slackService.enabled,
              channel: slackService.defaultChannel,
              rateLimit: slackService.rateLimit,
              hasToken: !!slackService.token,
            };
          }

          case 'health': {
            const healthy = await slackService.healthCheck();
            return {
              success: true,
              action: 'health',
              healthy,
              enabled: slackService.enabled,
              message: healthy
                ? '*tail wag* Slack connection healthy!'
                : '*sniff* Slack connection failed. Check token.',
            };
          }

          case 'stats': {
            return {
              success: true,
              action: 'stats',
              stats: slackService.getStats(),
            };
          }

          default:
            return { success: false, error: `Unknown action: ${action}` };
        }
      } catch (err) {
        log.error('Slack status error', { error: err.message });
        return {
          success: false,
          error: err.message,
        };
      }
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Format notification for response
 * @param {Object} n - Notification object
 * @returns {Object} Formatted notification
 */
function formatNotification(n) {
  return {
    id: n.id,
    type: n.notificationType,
    title: n.title,
    message: n.message,
    priority: n.priority,
    delivered: n.delivered,
    deliveredAt: n.deliveredAt,
    dismissed: n.dismissed,
    createdAt: n.createdAt,
    context: n.context,
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// FACTORY
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Notifications tools factory
 */
export const notificationsFactory = {
  name: 'notifications',
  domain: 'notifications',
  requires: [], // Optional dependencies

  /**
   * Create all notification tools
   * @param {Object} options
   * @returns {Array} Tool definitions
   */
  create(options) {
    const { notificationsRepo, slackService } = options;
    const tools = [];

    // Internal notifications tool
    if (notificationsRepo) {
      tools.push(createNotificationsTool(notificationsRepo));
    }

    // Slack tools (created even without service for helpful error messages)
    tools.push(createSlackSendTool(slackService));
    tools.push(createSlackStatusTool(slackService));

    log.debug('Notifications tools created', {
      count: tools.length,
      hasRepo: !!notificationsRepo,
      hasSlack: !!slackService?.enabled,
    });

    return tools;
  },
};

export default notificationsFactory;
