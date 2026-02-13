/**
 * X/Twitter Post Service
 *
 * Handles OAuth 1.0a signing and POST /2/tweets API calls.
 * CYNIC can now SPEAK, not just observe.
 *
 * "Le chien aboie enfin." - κυνικός
 *
 * @module @cynic/mcp/services/x-post
 */

'use strict';

import crypto from 'crypto';
import { createLogger, PHI_INV } from '@cynic/core';

const log = createLogger('XPost');

const X_API_BASE = 'https://api.x.com';
const TWEET_ENDPOINT = '/2/tweets';

/**
 * X Post Service
 *
 * OAuth 1.0a signed requests to X API v2.
 * Free tier: 1,500 tweets/month (write-only).
 */
export class XPostService {
  /**
   * @param {Object} options
   * @param {string} options.apiKey - X API Key (Consumer Key)
   * @param {string} options.apiSecret - X API Secret (Consumer Secret)
   * @param {string} options.accessToken - User Access Token
   * @param {string} options.accessTokenSecret - User Access Token Secret
   * @param {Object} [options.localXStore] - LocalXStore for logging posts
   * @param {boolean} [options.dryRun=false] - If true, don't actually post
   */
  constructor(options = {}) {
    this.apiKey = options.apiKey;
    this.apiSecret = options.apiSecret;
    this.accessToken = options.accessToken;
    this.accessTokenSecret = options.accessTokenSecret;
    this.localXStore = options.localXStore || null;
    this.dryRun = options.dryRun || false;

    // Rate limiting (free tier: 1,500/month ≈ 50/day)
    this._postCount = 0;
    this._dailyLimit = 50;
    this._monthlyLimit = 1500;
    this._postLog = []; // timestamps of posts

    this._configured = !!(this.apiKey && this.apiSecret && this.accessToken && this.accessTokenSecret);

    if (this._configured) {
      log.info('XPostService configured', { dryRun: this.dryRun });
    } else {
      log.warn('XPostService not configured — missing API credentials');
    }
  }

  /**
   * Check if service is ready to post
   */
  isConfigured() {
    return this._configured;
  }

  /**
   * Post a tweet
   *
   * @param {string} text - Tweet text (max 280 chars)
   * @param {Object} [options]
   * @param {string} [options.replyTo] - Tweet ID to reply to
   * @param {string} [options.quoteTweetId] - Tweet ID to quote
   * @returns {Promise<Object>} - X API response with tweet data
   */
  async postTweet(text, options = {}) {
    if (!this._configured) {
      throw new Error('XPostService not configured. Set X_API_KEY, X_API_SECRET, X_ACCESS_TOKEN, X_ACCESS_TOKEN_SECRET in .env');
    }

    // Validate text
    if (!text || typeof text !== 'string') {
      throw new Error('Tweet text is required');
    }

    if (text.length > 280) {
      throw new Error(`Tweet too long: ${text.length}/280 characters`);
    }

    // Rate limit check
    this._cleanOldPosts();
    const todayCount = this._countToday();
    if (todayCount >= this._dailyLimit) {
      throw new Error(`Daily rate limit reached: ${todayCount}/${this._dailyLimit}. Try again tomorrow.`);
    }

    // Build request body
    const body = { text };

    if (options.replyTo) {
      body.reply = { in_reply_to_tweet_id: options.replyTo };
    }

    if (options.quoteTweetId) {
      body.quote_tweet_id = options.quoteTweetId;
    }

    // Dry run mode
    if (this.dryRun) {
      log.info('DRY RUN — tweet not posted', { text: text.substring(0, 50), chars: text.length });
      return {
        dryRun: true,
        text,
        chars: text.length,
        wouldPost: true,
      };
    }

    // Sign and send
    const url = `${X_API_BASE}${TWEET_ENDPOINT}`;
    const method = 'POST';

    const authHeader = this._buildOAuthHeader(method, url);

    log.info('Posting tweet', { chars: text.length, replyTo: options.replyTo || null });

    const response = await fetch(url, {
      method,
      headers: {
        'Authorization': authHeader,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(body),
    });

    const responseData = await response.json();

    if (!response.ok) {
      const errorMsg = responseData.detail || responseData.title || JSON.stringify(responseData);
      log.error('Tweet failed', { status: response.status, error: errorMsg });
      throw new Error(`X API error (${response.status}): ${errorMsg}`);
    }

    // Log successful post
    this._postLog.push(Date.now());
    this._postCount++;

    const tweetId = responseData.data?.id;
    log.info('Tweet posted', { tweetId, chars: text.length });

    // Store in local X store for records
    if (this.localXStore && tweetId) {
      try {
        this.localXStore.recordAction({
          action_type: 'post',
          tweet_id: tweetId,
          metadata: JSON.stringify({ text, replyTo: options.replyTo, quoteTweetId: options.quoteTweetId }),
        });
      } catch (e) {
        log.warn('Failed to log post to LocalXStore', { error: e.message });
      }
    }

    return {
      success: true,
      tweetId,
      text,
      chars: text.length,
      url: `https://x.com/i/status/${tweetId}`,
      dailyRemaining: this._dailyLimit - this._countToday(),
    };
  }

  /**
   * Delete a tweet
   *
   * @param {string} tweetId - Tweet ID to delete
   * @returns {Promise<Object>}
   */
  async deleteTweet(tweetId) {
    if (!this._configured) {
      throw new Error('XPostService not configured');
    }

    const url = `${X_API_BASE}${TWEET_ENDPOINT}/${tweetId}`;
    const method = 'DELETE';
    const authHeader = this._buildOAuthHeader(method, url);

    const response = await fetch(url, {
      method,
      headers: { 'Authorization': authHeader },
    });

    if (!response.ok) {
      const data = await response.json();
      throw new Error(`Delete failed (${response.status}): ${data.detail || JSON.stringify(data)}`);
    }

    log.info('Tweet deleted', { tweetId });
    return { success: true, tweetId, deleted: true };
  }

  /**
   * Get posting stats
   */
  getStats() {
    this._cleanOldPosts();
    return {
      configured: this._configured,
      dryRun: this.dryRun,
      totalPosts: this._postCount,
      postsToday: this._countToday(),
      dailyLimit: this._dailyLimit,
      dailyRemaining: this._dailyLimit - this._countToday(),
      monthlyLimit: this._monthlyLimit,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════
  // OAuth 1.0a Implementation
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Build OAuth 1.0a Authorization header
   *
   * @param {string} method - HTTP method
   * @param {string} url - Request URL
   * @param {Object} [extraParams] - Additional params for signature
   * @returns {string} Authorization header value
   * @private
   */
  _buildOAuthHeader(method, url, extraParams = {}) {
    const timestamp = Math.floor(Date.now() / 1000).toString();
    const nonce = crypto.randomBytes(16).toString('hex');

    const oauthParams = {
      oauth_consumer_key: this.apiKey,
      oauth_nonce: nonce,
      oauth_signature_method: 'HMAC-SHA1',
      oauth_timestamp: timestamp,
      oauth_token: this.accessToken,
      oauth_version: '1.0',
      ...extraParams,
    };

    // Create signature base string
    const allParams = { ...oauthParams };
    const paramString = Object.keys(allParams)
      .sort()
      .map(k => `${this._percentEncode(k)}=${this._percentEncode(allParams[k])}`)
      .join('&');

    const signatureBase = [
      method.toUpperCase(),
      this._percentEncode(url),
      this._percentEncode(paramString),
    ].join('&');

    // Create signing key
    const signingKey = `${this._percentEncode(this.apiSecret)}&${this._percentEncode(this.accessTokenSecret)}`;

    // Generate signature
    const signature = crypto
      .createHmac('sha1', signingKey)
      .update(signatureBase)
      .digest('base64');

    oauthParams.oauth_signature = signature;

    // Build header
    const headerParts = Object.keys(oauthParams)
      .sort()
      .map(k => `${this._percentEncode(k)}="${this._percentEncode(oauthParams[k])}"`)
      .join(', ');

    return `OAuth ${headerParts}`;
  }

  /**
   * RFC 3986 percent-encode
   * @private
   */
  _percentEncode(str) {
    return encodeURIComponent(String(str))
      .replace(/!/g, '%21')
      .replace(/\*/g, '%2A')
      .replace(/'/g, '%27')
      .replace(/\(/g, '%28')
      .replace(/\)/g, '%29');
  }

  /**
   * Clean post log entries older than 30 days
   * @private
   */
  _cleanOldPosts() {
    const thirtyDaysAgo = Date.now() - (30 * 24 * 60 * 60 * 1000);
    this._postLog = this._postLog.filter(ts => ts > thirtyDaysAgo);
  }

  /**
   * Count posts made today
   * @private
   */
  _countToday() {
    const todayStart = new Date();
    todayStart.setHours(0, 0, 0, 0);
    return this._postLog.filter(ts => ts >= todayStart.getTime()).length;
  }
}

/**
 * Create XPostService from environment variables
 */
export function createXPostService(options = {}) {
  return new XPostService({
    apiKey: process.env.X_API_KEY,
    apiSecret: process.env.X_API_SECRET,
    accessToken: process.env.X_ACCESS_TOKEN,
    accessTokenSecret: process.env.X_ACCESS_TOKEN_SECRET,
    dryRun: process.env.X_DRY_RUN === 'true',
    ...options,
  });
}

export default XPostService;
