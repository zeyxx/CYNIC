/**
 * CYNIC Social Watcher - C4.1 (SOCIAL × PERCEIVE)
 *
 * Watches Twitter/Discord for $asdfasdfa cult activity:
 * - Twitter: @asdfasdfa_sol mentions, #asdfasdfa hashtags
 * - Discord: Community activity (if webhook configured)
 *
 * Feeds SocialJudge (C4.2) and SocialEmergence (C4.7) via SOCIAL_CAPTURE events.
 *
 * "Le chien écoute les murmures du culte" - κυνικός
 *
 * @module @cynic/node/perception/social-watcher
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, globalEventBus, EventType } from '@cynic/core';
import { createSingleton, phiBound } from '@cynic/core/axioms/phi-utils.js';
import { getEventBus } from '../services/event-bus.js';

const log = createLogger('SocialWatcher');

/**
 * Social event types
 * @readonly
 * @enum {string}
 */
export const SocialEventType = {
  TWITTER_MENTION: 'perception:social:twitter:mention',
  DISCORD_MESSAGE: 'perception:social:discord:message',
  SENTIMENT_DETECTED: 'perception:social:sentiment',
  ENGAGEMENT_SPIKE: 'perception:social:engagement',
};

/**
 * Default poll intervals (φ-aligned)
 */
const POLL_INTERVALS = {
  twitter: 60 * 1000,    // 1 min (Twitter rate limit: 450/15min = 30s min, use 60s for safety)
  discord: 2 * 60 * 1000, // 2 min (slower, webhook-driven)
};

/**
 * Rate limit backoff (exponential)
 */
const RATE_LIMIT_BACKOFF_MS = [1000, 2000, 5000, 10000, 30000]; // Up to 30s

/**
 * Sentiment analysis keywords
 */
const SENTIMENT_KEYWORDS = {
  positive: [
    'bullish', 'moon', 'wagmi', 'gm', 'lfg', 'based',
    'king', 'queen', 'gigabrain', 'alpha', 'pump',
    'hodl', 'diamond', 'hands', 'fren', 'chad',
  ],
  negative: [
    'rug', 'dump', 'scam', 'ponzi', 'ngmi', 'rekt',
    'fud', 'bear', 'sell', 'exit', 'liquidity',
    'dead', 'over', 'cope', 'seethe', 'mald',
  ],
};

/**
 * SocialWatcher - Polls Twitter/Discord for $asdfasdfa cult activity
 *
 * Implements C4.1 (SOCIAL × PERCEIVE) in 7×7 matrix.
 * Feeds SocialJudge (C4.2) and SocialEmergence (C4.7).
 *
 * Phase 1: Mock mode (no API keys required)
 * Phase 2: Real Twitter API v2 integration
 * Phase 3: Discord webhook listener
 */
export class SocialWatcher extends EventEmitter {
  /**
   * Create SocialWatcher
   *
   * @param {Object} [options]
   * @param {string} [options.twitterApiKey] - Twitter API bearer token
   * @param {string} [options.discordWebhookUrl] - Discord webhook URL
   * @param {EventBus} [options.eventBus] - EventBus instance
   * @param {Object} [options.db] - PostgreSQL pool for heartbeats
   * @param {number} [options.pollInterval] - Override default poll interval
   * @param {boolean} [options.mockMode] - Enable mock data mode (default: auto-detect)
   */
  constructor(options = {}) {
    super();

    this.twitterApiKey = options.twitterApiKey || process.env.TWITTER_API_KEY || null;
    this.discordWebhookUrl = options.discordWebhookUrl || process.env.DISCORD_WEBHOOK_URL || null;
    this.eventBus = options.eventBus || getEventBus();
    this.db = options.db || null;
    this.pollInterval = options.pollInterval || POLL_INTERVALS.twitter;

    // Auto-enable mock mode if no Twitter API key
    this.mockMode = options.mockMode !== undefined ? options.mockMode : !this.twitterApiKey;

    this._timers = new Map(); // type -> timer
    this._isRunning = false;
    this._rateLimitBackoffIndex = 0;

    // Social state
    this._lastTwitterPoll = null;
    this._lastDiscordPoll = null;
    this._tweetsSeen = new Set(); // Track tweet IDs to avoid duplicates

    // Stats
    this.stats = {
      polls: 0,
      mentions: 0,
      discordMessages: 0,
      sentimentDistribution: {
        positive: 0,
        neutral: 0,
        negative: 0,
      },
      engagementSpikes: 0,
      rateLimitHits: 0,
      errors: 0,
      lastPollAt: null,
    };

    // Event rate tracking (per-minute window)
    this._eventWindow = [];
    this._windowSize = 60000; // 1 minute
  }

  /**
   * Start watching social activity
   */
  async start() {
    if (this._isRunning) {
      log.debug('SocialWatcher already running');
      return;
    }

    try {
      // Test Twitter API connection (or verify mock mode)
      if (this.mockMode) {
        log.info('SocialWatcher starting in MOCK mode (no Twitter API key)', {
          pollInterval: this.pollInterval,
        });
      } else {
        // TODO Phase 2: Test Twitter API connection with bearer token
        log.info('SocialWatcher starting with Twitter API v2', {
          pollInterval: this.pollInterval,
        });
      }

      this._isRunning = true;

      // Start polling loops
      this._startTwitterPolling();
      if (this.discordWebhookUrl) {
        this._startDiscordListener();
      }

      // Emit startup event
      globalEventBus.emit(EventType.WATCHER_STARTED || 'watcher:started', {
        watcher: 'social',
        mockMode: this.mockMode,
        timestamp: Date.now(),
      });

      // Record heartbeat
      await this._recordHeartbeat('active');

      log.info('SocialWatcher started');
    } catch (error) {
      log.error('Failed to start SocialWatcher', { error: error.message });
      this._isRunning = false;
      throw error;
    }
  }

  /**
   * Stop watching
   */
  async stop() {
    if (!this._isRunning) return;

    this._isRunning = false;

    // Clear all timers
    for (const timer of this._timers.values()) {
      clearInterval(timer);
    }
    this._timers.clear();

    // Record final heartbeat
    await this._recordHeartbeat('stopped');

    globalEventBus.emit(EventType.WATCHER_STOPPED || 'watcher:stopped', {
      watcher: 'social',
      timestamp: Date.now(),
    });

    log.info('SocialWatcher stopped');
  }

  /**
   * Get current stats
   */
  getStats() {
    return { ...this.stats };
  }

  /**
   * Get current state (for concurrent polling)
   */
  getState() {
    return {
      isRunning: this._isRunning,
      mockMode: this.mockMode,
      lastTwitterPoll: this._lastTwitterPoll,
      lastDiscordPoll: this._lastDiscordPoll,
      stats: this.getStats(),
      timestamp: Date.now(),
    };
  }

  // =============================================================================
  // PRIVATE POLLING LOOPS
  // =============================================================================

  /**
   * Start Twitter polling loop
   * @private
   */
  _startTwitterPolling() {
    const poll = async () => {
      if (!this._isRunning) return;

      try {
        const data = await this._fetchTwitterMentions();
        if (data && data.tweets && data.tweets.length > 0) {
          // Analyze aggregate sentiment
          const sentiment = this._aggregateSentiment(data.tweets);

          // Emit SOCIAL_CAPTURE event
          globalEventBus.emit(EventType.SOCIAL_CAPTURE, {
            source: 'twitter',
            tweets: data.tweets,
            users: data.users,
            sentiment,
            timestamp: Date.now(),
          });

          // Update stats
          this.stats.mentions += data.tweets.length;
          this.stats.sentimentDistribution[sentiment]++;
          this.stats.polls++;
          this.stats.lastPollAt = Date.now();

          // Reset rate limit backoff on success
          this._rateLimitBackoffIndex = 0;

          await this._recordHeartbeat('active', data.tweets.length);
        } else {
          // No new data, but poll succeeded
          this.stats.polls++;
          this.stats.lastPollAt = Date.now();
          await this._recordHeartbeat('active', 0);
        }

        this._lastTwitterPoll = Date.now();
      } catch (error) {
        this._handlePollError('twitter', error);
      }
    };

    // Initial poll
    poll();

    // Recurring poll
    const timer = setInterval(poll, this.pollInterval);
    this._timers.set('twitter', timer);
  }

  /**
   * Start Discord listener
   * @private
   */
  _startDiscordListener() {
    // Phase 3: Implement Discord webhook listener (Express endpoint)
    // For Phase 1: Just mock periodic messages
    const poll = async () => {
      if (!this._isRunning) return;

      try {
        const messages = await this._fetchDiscordMessages();
        if (messages && messages.length > 0) {
          // Emit SOCIAL_CAPTURE event for Discord
          globalEventBus.emit(EventType.SOCIAL_CAPTURE, {
            source: 'discord',
            messages,
            timestamp: Date.now(),
          });

          this.stats.discordMessages += messages.length;
          await this._recordHeartbeat('active', messages.length);
        }

        this._lastDiscordPoll = Date.now();
      } catch (error) {
        this._handlePollError('discord', error);
      }
    };

    // Initial poll (offset by 30s to avoid collision with Twitter)
    setTimeout(poll, 30000);

    // Recurring poll
    const timer = setInterval(poll, POLL_INTERVALS.discord);
    this._timers.set('discord', timer);
  }

  // =============================================================================
  // DATA FETCHING
  // =============================================================================

  /**
   * Fetch Twitter mentions of @asdfasdfa_sol and #asdfasdfa
   * @private
   * @returns {Promise<{ tweets: Array, users: Array }|null>}
   */
  async _fetchTwitterMentions() {
    try {
      if (this.mockMode) {
        // Phase 1: Mock data for testing
        return this._generateMockTwitterData();
      }

      // Phase 2: Real Twitter API v2 implementation
      // TODO: Implement Twitter API v2 search endpoint
      // - Use bearer token authentication
      // - Query: "(@asdfasdfa_sol OR #asdfasdfa) -is:retweet"
      // - Max results: 100
      // - Since last poll timestamp
      // - Handle pagination
      // - Parse response → normalize to { tweets, users }

      log.warn('Twitter API not yet implemented — using mock data');
      return this._generateMockTwitterData();
    } catch (error) {
      throw error;
    }
  }

  /**
   * Generate mock Twitter data (Phase 1)
   * @private
   */
  _generateMockTwitterData() {
    const mockTweets = [
      {
        id: `mock_${Date.now()}_1`,
        text: 'Just aped into $asdfasdfa 🚀 This is the way. WAGMI frens! #asdfasdfa',
        author_id: 'user_1',
        created_at: new Date().toISOString(),
        public_metrics: {
          like_count: Math.floor(Math.random() * 50),
          retweet_count: Math.floor(Math.random() * 20),
          reply_count: Math.floor(Math.random() * 10),
        },
      },
      {
        id: `mock_${Date.now()}_2`,
        text: '@asdfasdfa_sol community is based. Best cult in crypto. LFG! 💎🙌',
        author_id: 'user_2',
        created_at: new Date().toISOString(),
        public_metrics: {
          like_count: Math.floor(Math.random() * 30),
          retweet_count: Math.floor(Math.random() * 10),
          reply_count: Math.floor(Math.random() * 5),
        },
      },
      {
        id: `mock_${Date.now()}_3`,
        text: 'Hmm, $asdfasdfa looking a bit shaky today. Hope this isn\'t a rug... 🤔',
        author_id: 'user_3',
        created_at: new Date().toISOString(),
        public_metrics: {
          like_count: Math.floor(Math.random() * 15),
          retweet_count: Math.floor(Math.random() * 5),
          reply_count: Math.floor(Math.random() * 8),
        },
      },
    ];

    const mockUsers = [
      { id: 'user_1', username: 'crypto_degen_1', name: 'Crypto Degen' },
      { id: 'user_2', username: 'asdfasdfa_maxi', name: 'asdfasdfa Maxi' },
      { id: 'user_3', username: 'cautious_trader', name: 'Cautious Trader' },
    ];

    return { tweets: mockTweets, users: mockUsers };
  }

  /**
   * Fetch Discord messages (Phase 1: mock, Phase 3: real webhook)
   * @private
   * @returns {Promise<Array|null>}
   */
  async _fetchDiscordMessages() {
    if (this.mockMode || !this.discordWebhookUrl) {
      // Phase 1: Mock Discord messages
      return [
        {
          id: `discord_${Date.now()}_1`,
          content: 'GM CYNIC frens! How\'s the $asdfasdfa vibe today?',
          author: { username: 'community_member_1' },
          timestamp: new Date().toISOString(),
        },
      ];
    }

    // Phase 3: Real Discord webhook listener implementation
    // TODO: Set up Express endpoint to receive Discord webhooks
    // TODO: Parse Discord message events
    // TODO: Return normalized message data
    return [];
  }

  // =============================================================================
  // SENTIMENT ANALYSIS
  // =============================================================================

  /**
   * Analyze sentiment of a single tweet
   * @private
   * @param {string} text - Tweet text
   * @returns {'positive' | 'neutral' | 'negative'}
   */
  _analyzeSentiment(text) {
    if (!text) return 'neutral';

    const lowerText = text.toLowerCase();
    let positiveScore = 0;
    let negativeScore = 0;

    // Count positive keywords
    for (const keyword of SENTIMENT_KEYWORDS.positive) {
      if (lowerText.includes(keyword)) {
        positiveScore++;
      }
    }

    // Count negative keywords
    for (const keyword of SENTIMENT_KEYWORDS.negative) {
      if (lowerText.includes(keyword)) {
        negativeScore++;
      }
    }

    // Determine sentiment
    if (positiveScore > negativeScore) return 'positive';
    if (negativeScore > positiveScore) return 'negative';
    return 'neutral';
  }

  /**
   * Aggregate sentiment from multiple tweets
   * @private
   * @param {Array} tweets - Array of tweet objects
   * @returns {'positive' | 'neutral' | 'negative'}
   */
  _aggregateSentiment(tweets) {
    if (!tweets || tweets.length === 0) return 'neutral';

    const sentiments = tweets.map(t => this._analyzeSentiment(t.text));
    const counts = {
      positive: sentiments.filter(s => s === 'positive').length,
      neutral: sentiments.filter(s => s === 'neutral').length,
      negative: sentiments.filter(s => s === 'negative').length,
    };

    // Return dominant sentiment
    if (counts.positive >= counts.neutral && counts.positive >= counts.negative) {
      return 'positive';
    }
    if (counts.negative >= counts.neutral && counts.negative >= counts.positive) {
      return 'negative';
    }
    return 'neutral';
  }

  // =============================================================================
  // ERROR HANDLING & HEARTBEATS
  // =============================================================================

  /**
   * Handle poll error with rate limit backoff
   * @private
   */
  _handlePollError(pollType, error) {
    this.stats.errors++;

    // Check for 429 rate limit
    if (error.message?.includes('429') || error.message?.includes('rate limit')) {
      this.stats.rateLimitHits++;

      // Exponential backoff
      const backoffMs = RATE_LIMIT_BACKOFF_MS[this._rateLimitBackoffIndex] || RATE_LIMIT_BACKOFF_MS[RATE_LIMIT_BACKOFF_MS.length - 1];
      this._rateLimitBackoffIndex = Math.min(this._rateLimitBackoffIndex + 1, RATE_LIMIT_BACKOFF_MS.length - 1);

      log.warn('SocialWatcher rate limited', {
        pollType,
        backoffMs,
        backoffIndex: this._rateLimitBackoffIndex,
      });

      // Pause this poll type temporarily
      const timer = this._timers.get(pollType);
      if (timer) {
        clearInterval(timer);
        setTimeout(() => {
          if (this._isRunning) {
            if (pollType === 'twitter') this._startTwitterPolling();
            else if (pollType === 'discord') this._startDiscordListener();
          }
        }, backoffMs);
      }

      this._recordHeartbeat('idle', 0, 'rate_limited');
    } else {
      log.debug('SocialWatcher poll error', {
        pollType,
        error: error.message,
      });

      this._recordHeartbeat('error', 0, error.message);
    }
  }

  /**
   * Record heartbeat to database
   * @private
   */
  async _recordHeartbeat(status, eventsPolled = 0, errorMessage = null) {
    if (!this.db) return;

    try {
      await this.db.query(`
        INSERT INTO watcher_heartbeats (watcher_name, status, events_polled, error_message)
        VALUES ($1, $2, $3, $4)
      `, ['social', status, eventsPolled, errorMessage]);
    } catch (err) {
      // Non-blocking
      log.debug('Failed to record heartbeat', { error: err.message });
    }
  }
}

// Singleton
const { getInstance, resetInstance } = createSingleton(SocialWatcher);

/**
 * Get singleton instance
 */
export const getSocialWatcher = getInstance;

/**
 * Reset singleton (for testing)
 */
export const resetSocialWatcher = resetInstance;

export default SocialWatcher;
