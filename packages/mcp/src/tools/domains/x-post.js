/**
 * X/Twitter Post Tool
 *
 * MCP tool for posting tweets via CYNIC.
 * Guardian-protected: requires confirmation for every post.
 *
 * "Le chien parle enfin. Avec prudence." - κυνικός
 *
 * @module @cynic/mcp/tools/domains/x-post
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';

const log = createLogger('XPostTool');

/**
 * Create brain_x_post tool
 *
 * @param {import('../../services/x-post.js').XPostService} xPostService
 * @param {Object} [judge] - CYNICJudge for content evaluation
 */
export function createXPostTool(xPostService, judge) {
  return {
    name: 'brain_x_post',
    description: `Post a tweet to X/Twitter from your account.
Guardian-protected: CYNIC evaluates content quality before posting.
Supports: new tweets, replies, and quote tweets.
Free tier: ~50 tweets/day (1,500/month).
Use brain_x_coach first to refine your draft.`,
    inputSchema: {
      type: 'object',
      properties: {
        text: {
          type: 'string',
          description: 'Tweet text (max 280 characters)',
          maxLength: 280,
        },
        reply_to: {
          type: 'string',
          description: 'Tweet ID to reply to (optional)',
        },
        quote_tweet_id: {
          type: 'string',
          description: 'Tweet ID to quote (optional)',
        },
        confirm: {
          type: 'boolean',
          description: 'Set to true to confirm posting. First call without confirm to preview.',
          default: false,
        },
      },
      required: ['text'],
    },
    handler: async (params) => {
      const { text, reply_to, quote_tweet_id, confirm } = params;

      // Check service is configured
      if (!xPostService || !xPostService.isConfigured()) {
        return {
          content: [{
            type: 'text',
            text: `*GROWL* XPostService not configured.\n\nAdd to .env:\n  X_API_KEY=your_api_key\n  X_API_SECRET=your_api_secret\n  X_ACCESS_TOKEN=your_access_token\n  X_ACCESS_TOKEN_SECRET=your_access_token_secret\n\nGet keys from developer.x.com`,
          }],
        };
      }

      // Validate length
      if (text.length > 280) {
        return {
          content: [{
            type: 'text',
            text: `*GROWL* Tweet too long: ${text.length}/280 characters. Trim ${text.length - 280} chars.`,
          }],
        };
      }

      // Judge content quality (optional)
      let judgment = null;
      if (judge) {
        try {
          judgment = judge.judge(
            { type: 'tweet', content: text },
            { type: 'social', queryType: 'tweet_post' }
          );
        } catch (e) {
          log.warn('Judge failed on tweet content', { error: e.message });
        }
      }

      // Preview mode (no confirm)
      if (!confirm) {
        const stats = xPostService.getStats();
        const preview = {
          text,
          chars: `${text.length}/280`,
          type: reply_to ? 'reply' : quote_tweet_id ? 'quote' : 'tweet',
          dailyRemaining: `${stats.dailyRemaining}/${stats.dailyLimit}`,
        };

        if (judgment) {
          preview.quality = {
            qScore: judgment.qScore,
            verdict: judgment.qVerdict?.verdict,
            confidence: Math.min(judgment.confidence || 0, PHI_INV),
          };
        }

        let previewText = `*sniff* Tweet preview:\n\n`;
        previewText += `"${text}"\n\n`;
        previewText += `Characters: ${text.length}/280\n`;
        previewText += `Type: ${preview.type}\n`;
        previewText += `Daily remaining: ${preview.dailyRemaining}\n`;

        if (judgment) {
          previewText += `\nQuality: Q-Score ${judgment.qScore}/100 (${judgment.qVerdict?.verdict})\n`;
          if (judgment.qScore < 30) {
            previewText += `*GROWL* Low quality score. Consider using brain_x_coach to improve.\n`;
          }
        }

        previewText += `\nTo post: call again with confirm: true`;

        return {
          content: [{
            type: 'text',
            text: previewText,
          }],
        };
      }

      // POSTING MODE — confirm=true
      try {
        const result = await xPostService.postTweet(text, {
          replyTo: reply_to,
          quoteTweetId: quote_tweet_id,
        });

        if (result.dryRun) {
          return {
            content: [{
              type: 'text',
              text: `*sniff* DRY RUN — tweet NOT posted (X_DRY_RUN=true in .env)\n\nWould have posted:\n"${text}"`,
            }],
          };
        }

        let successText = `*tail wag* Tweet posted!\n\n`;
        successText += `"${text}"\n\n`;
        successText += `Tweet ID: ${result.tweetId}\n`;
        successText += `URL: ${result.url}\n`;
        successText += `Daily remaining: ${result.dailyRemaining}/${xPostService._dailyLimit}`;

        return {
          content: [{
            type: 'text',
            text: successText,
          }],
        };
      } catch (error) {
        log.error('Post failed', { error: error.message });
        return {
          content: [{
            type: 'text',
            text: `*GROWL* Post failed: ${error.message}`,
          }],
        };
      }
    },
  };
}

/**
 * Create brain_x_delete tool
 */
export function createXDeleteTool(xPostService) {
  return {
    name: 'brain_x_delete',
    description: 'Delete one of your tweets by ID.',
    inputSchema: {
      type: 'object',
      properties: {
        tweet_id: {
          type: 'string',
          description: 'ID of the tweet to delete',
        },
      },
      required: ['tweet_id'],
    },
    handler: async (params) => {
      if (!xPostService || !xPostService.isConfigured()) {
        return {
          content: [{ type: 'text', text: '*GROWL* XPostService not configured.' }],
        };
      }

      try {
        const result = await xPostService.deleteTweet(params.tweet_id);
        return {
          content: [{
            type: 'text',
            text: `*sniff* Tweet ${params.tweet_id} deleted.`,
          }],
        };
      } catch (error) {
        return {
          content: [{
            type: 'text',
            text: `*GROWL* Delete failed: ${error.message}`,
          }],
        };
      }
    },
  };
}

/**
 * X Post tools factory
 */
export const xPostFactory = {
  name: 'x-post',
  domain: 'social',
  requires: ['xPostService'],

  create(options) {
    const { xPostService, judge } = options;

    if (!xPostService) {
      log.debug('X Post tools skipped: xPostService not available');
      return [];
    }

    return [
      createXPostTool(xPostService, judge),
      createXDeleteTool(xPostService),
    ];
  },
};

export default xPostFactory;
