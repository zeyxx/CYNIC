/**
 * Local X/Twitter Data Store (SQLite)
 *
 * Privacy-first local storage for X data.
 * All captured data stays here by default.
 * User explicitly chooses what to sync to cloud.
 *
 * "Your data, your device, your choice" - κυνικός
 *
 * @module @cynic/persistence/sqlite/LocalXStore
 */

'use strict';

import Database from 'better-sqlite3';
import { join } from 'path';
import { mkdirSync, existsSync } from 'fs';
import { homedir } from 'os';

// Default location: ~/.cynic/x-local.db
const DEFAULT_DB_PATH = join(homedir(), '.cynic', 'x-local.db');

/**
 * Local SQLite store for X/Twitter data
 *
 * Privacy by design:
 * - All data captured locally first
 * - sync_status tracks what user has chosen to share
 * - synced_at shows when data was last synced
 */
export class LocalXStore {
  /**
   * @param {Object} [options] - Store options
   * @param {string} [options.dbPath] - Path to SQLite database
   * @param {boolean} [options.verbose] - Enable verbose logging
   */
  constructor(options = {}) {
    this.dbPath = options.dbPath || process.env.CYNIC_X_LOCAL_DB || DEFAULT_DB_PATH;
    this.verbose = options.verbose || false;
    this.db = null;
  }

  /**
   * Initialize the database
   */
  async initialize() {
    // Ensure directory exists
    const dir = this.dbPath.substring(0, this.dbPath.lastIndexOf('/') || this.dbPath.lastIndexOf('\\'));
    if (dir && !existsSync(dir)) {
      mkdirSync(dir, { recursive: true });
    }

    // Open database
    this.db = new Database(this.dbPath, {
      verbose: this.verbose ? console.error : null,
    });

    // Enable WAL mode for better concurrency
    this.db.pragma('journal_mode = WAL');

    // Disable FK enforcement — this is a local cache, not a relational DB.
    // better-sqlite3 may be compiled with SQLITE_DEFAULT_FK_ENFORCEMENT=1,
    // which breaks tweet inserts when users arrive in the same batch.
    this.db.pragma('foreign_keys = OFF');

    // Create tables
    this._createTables();

    if (this.verbose) {
      console.error(`[LocalXStore] Initialized at ${this.dbPath}`);
    }

    return this;
  }

  /**
   * Create database tables
   * @private
   */
  _createTables() {
    // Users table
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS x_users (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        x_user_id TEXT UNIQUE NOT NULL,
        username TEXT NOT NULL,
        display_name TEXT,
        bio TEXT,
        profile_image_url TEXT,
        followers_count INTEGER DEFAULT 0,
        following_count INTEGER DEFAULT 0,
        tweet_count INTEGER DEFAULT 0,
        verified INTEGER DEFAULT 0,
        protected INTEGER DEFAULT 0,
        created_at TEXT,
        captured_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        -- Privacy fields
        sync_status TEXT DEFAULT 'local' CHECK (sync_status IN ('local', 'pending', 'synced', 'never')),
        synced_at TEXT,
        is_private INTEGER DEFAULT 0
      );

      CREATE INDEX IF NOT EXISTS idx_x_users_username ON x_users(username);
      CREATE INDEX IF NOT EXISTS idx_x_users_sync_status ON x_users(sync_status);
    `);

    // Tweets table
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS x_tweets (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        tweet_id TEXT UNIQUE NOT NULL,
        x_user_id TEXT NOT NULL,
        text TEXT NOT NULL,
        created_at TEXT,
        language TEXT,
        -- Engagement
        like_count INTEGER DEFAULT 0,
        retweet_count INTEGER DEFAULT 0,
        reply_count INTEGER DEFAULT 0,
        quote_count INTEGER DEFAULT 0,
        view_count INTEGER DEFAULT 0,
        bookmark_count INTEGER DEFAULT 0,
        -- Content
        hashtags TEXT, -- JSON array
        mentions TEXT, -- JSON array
        urls TEXT, -- JSON array
        media TEXT, -- JSON array
        -- Thread/conversation
        conversation_id TEXT,
        in_reply_to_tweet_id TEXT,
        in_reply_to_user_id TEXT,
        is_retweet INTEGER DEFAULT 0,
        is_quote INTEGER DEFAULT 0,
        quoted_tweet_id TEXT,
        -- Analysis (done locally)
        sentiment_score REAL,
        sentiment_label TEXT,
        topics TEXT, -- JSON array
        -- Metadata
        source TEXT,
        captured_at TEXT DEFAULT (datetime('now')),
        -- Privacy fields
        sync_status TEXT DEFAULT 'local' CHECK (sync_status IN ('local', 'pending', 'synced', 'never')),
        synced_at TEXT,
        is_private INTEGER DEFAULT 0,
        is_my_tweet INTEGER DEFAULT 0,

        FOREIGN KEY (x_user_id) REFERENCES x_users(x_user_id)
      );

      CREATE INDEX IF NOT EXISTS idx_x_tweets_user ON x_tweets(x_user_id);
      CREATE INDEX IF NOT EXISTS idx_x_tweets_created ON x_tweets(created_at DESC);
      CREATE INDEX IF NOT EXISTS idx_x_tweets_sync_status ON x_tweets(sync_status);
      CREATE INDEX IF NOT EXISTS idx_x_tweets_conversation ON x_tweets(conversation_id);
    `);

    // Feeds table (user-defined collections)
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS x_feeds (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        description TEXT,
        filters TEXT, -- JSON: { users: [], hashtags: [], keywords: [] }
        created_at TEXT DEFAULT (datetime('now')),
        updated_at TEXT DEFAULT (datetime('now')),
        -- Privacy
        sync_status TEXT DEFAULT 'local' CHECK (sync_status IN ('local', 'pending', 'synced', 'never')),
        synced_at TEXT
      );
    `);

    // Feed-tweet associations
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS x_feed_tweets (
        feed_id INTEGER NOT NULL,
        tweet_id TEXT NOT NULL,
        added_at TEXT DEFAULT (datetime('now')),
        PRIMARY KEY (feed_id, tweet_id),
        FOREIGN KEY (feed_id) REFERENCES x_feeds(id) ON DELETE CASCADE,
        FOREIGN KEY (tweet_id) REFERENCES x_tweets(tweet_id) ON DELETE CASCADE
      );
    `);

    // Trends (local copy)
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS x_trends (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        url TEXT,
        tweet_count INTEGER,
        category TEXT,
        captured_at TEXT DEFAULT (datetime('now')),
        -- Privacy
        sync_status TEXT DEFAULT 'local' CHECK (sync_status IN ('local', 'pending', 'synced', 'never')),
        synced_at TEXT
      );

      CREATE INDEX IF NOT EXISTS idx_x_trends_captured ON x_trends(captured_at DESC);
    `);

    // Sync log (track what was synced when)
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS x_sync_log (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        entity_type TEXT NOT NULL, -- 'user', 'tweet', 'feed', 'trend'
        entity_id TEXT NOT NULL,
        action TEXT NOT NULL, -- 'sync', 'unsync', 'delete'
        synced_at TEXT DEFAULT (datetime('now')),
        details TEXT -- JSON
      );

      CREATE INDEX IF NOT EXISTS idx_x_sync_log_entity ON x_sync_log(entity_type, entity_id);
    `);

    // My actions table (likes, RTs, bookmarks, follows)
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS x_my_actions (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        action_type TEXT NOT NULL,
        tweet_id TEXT,
        target_user_id TEXT,
        created_at TEXT DEFAULT (datetime('now')),
        raw_variables TEXT
      );

      CREATE INDEX IF NOT EXISTS idx_x_my_actions_type ON x_my_actions(action_type);
      CREATE INDEX IF NOT EXISTS idx_x_my_actions_tweet ON x_my_actions(tweet_id);
      CREATE INDEX IF NOT EXISTS idx_x_my_actions_time ON x_my_actions(created_at DESC);
    `);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // USERS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Upsert a user
   * @param {Object} user - User data
   * @returns {Object} Stored user
   */
  upsertUser(user) {
    const stmt = this.db.prepare(`
      INSERT INTO x_users (
        x_user_id, username, display_name, bio, profile_image_url,
        followers_count, following_count, tweet_count,
        verified, protected, created_at, is_private
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      ON CONFLICT(x_user_id) DO UPDATE SET
        username = excluded.username,
        display_name = excluded.display_name,
        bio = excluded.bio,
        profile_image_url = excluded.profile_image_url,
        followers_count = excluded.followers_count,
        following_count = excluded.following_count,
        tweet_count = excluded.tweet_count,
        verified = excluded.verified,
        protected = excluded.protected,
        updated_at = datetime('now')
      RETURNING *
    `);

    return stmt.get(
      user.x_user_id || user.xUserId || user.id_str || user.rest_id,
      user.username || user.screen_name,
      user.display_name || user.displayName || user.name,
      user.bio || user.description,
      user.profile_image_url || user.profileImageUrl || user.profile_image_url_https,
      user.followers_count || user.followersCount || 0,
      user.following_count || user.followingCount || user.friends_count || 0,
      user.tweet_count || user.tweetCount || user.statuses_count || 0,
      user.verified ? 1 : 0,
      user.protected ? 1 : 0,
      user.created_at || (user.createdOnX instanceof Date ? user.createdOnX.toISOString() : user.createdOnX),
      user.protected ? 1 : 0,
    );
  }

  /**
   * Find user by username
   * @param {string} username
   * @returns {Object|null}
   */
  findUserByUsername(username) {
    const stmt = this.db.prepare('SELECT * FROM x_users WHERE username = ?');
    return stmt.get(username) || null;
  }

  /**
   * Get users by sync status
   * @param {string} status - 'local', 'pending', 'synced', 'never'
   * @returns {Array}
   */
  getUsersBySyncStatus(status) {
    const stmt = this.db.prepare('SELECT * FROM x_users WHERE sync_status = ?');
    return stmt.all(status);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // TWEETS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Create a tweet (no duplicates)
   * @param {Object} tweet - Tweet data
   * @returns {Object|null} Stored tweet or null if duplicate
   */
  createTweet(tweet) {
    const stmt = this.db.prepare(`
      INSERT OR IGNORE INTO x_tweets (
        tweet_id, x_user_id, text, created_at, language,
        like_count, retweet_count, reply_count, quote_count, view_count, bookmark_count,
        hashtags, mentions, urls, media,
        conversation_id, in_reply_to_tweet_id, in_reply_to_user_id,
        is_retweet, is_quote, quoted_tweet_id,
        sentiment_score, sentiment_label, topics,
        source, is_private, is_my_tweet
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      RETURNING *
    `);

    const createdAt = tweet.created_at
      || (tweet.postedAt instanceof Date ? tweet.postedAt.toISOString() : tweet.postedAt)
      || new Date().toISOString();

    const result = stmt.get(
      tweet.tweet_id || tweet.tweetId || tweet.id_str || tweet.rest_id,
      tweet.x_user_id || tweet.xUserId || tweet.user_id_str,
      tweet.text || tweet.full_text,
      createdAt,
      tweet.language || tweet.lang,
      tweet.like_count || tweet.likesCount || tweet.favorite_count || 0,
      tweet.retweet_count || tweet.retweetsCount || 0,
      tweet.reply_count || tweet.repliesCount || 0,
      tweet.quote_count || tweet.quotesCount || 0,
      tweet.view_count || tweet.viewsCount || 0,
      tweet.bookmark_count || tweet.bookmarksCount || 0,
      JSON.stringify(tweet.hashtags || []),
      JSON.stringify(tweet.mentions || []),
      JSON.stringify(tweet.urls || []),
      JSON.stringify(tweet.media || []),
      tweet.conversation_id || tweet.threadId,
      tweet.in_reply_to_tweet_id || tweet.replyToTweetId || tweet.in_reply_to_status_id_str,
      tweet.in_reply_to_user_id || tweet.replyToUserId || tweet.in_reply_to_user_id_str,
      (tweet.is_retweet || tweet.isRetweet) ? 1 : 0,
      (tweet.is_quote || tweet.isQuote) ? 1 : 0,
      tweet.quoted_tweet_id || tweet.quoteTweetId || tweet.quoted_status_id_str,
      tweet.sentiment_score,
      tweet.sentiment_label,
      JSON.stringify(tweet.topics || []),
      tweet.source,
      tweet.is_private ? 1 : 0,
      tweet.is_my_tweet ? 1 : 0,
    );

    return result || null;
  }

  /**
   * Get recent tweets
   * @param {Object} options
   * @returns {Array}
   */
  getRecentTweets(options = {}) {
    const limit = options.limit || 20;
    const includePrivate = options.includePrivate !== false;

    let sql = `
      SELECT t.*, u.username, u.display_name, u.profile_image_url
      FROM x_tweets t
      LEFT JOIN x_users u ON t.x_user_id = u.x_user_id
    `;

    if (!includePrivate) {
      sql += ' WHERE t.is_private = 0';
    }

    sql += ' ORDER BY t.captured_at DESC LIMIT ?';

    const stmt = this.db.prepare(sql);
    return stmt.all(limit).map(this._parseTweet);
  }

  /**
   * Search tweets (full-text search on text)
   * @param {string} query
   * @param {Object} options
   * @returns {Array}
   */
  searchTweets(query, options = {}) {
    const limit = options.limit || 20;
    const includePrivate = options.includePrivate !== false;

    let sql = `
      SELECT t.*, u.username, u.display_name, u.profile_image_url
      FROM x_tweets t
      LEFT JOIN x_users u ON t.x_user_id = u.x_user_id
      WHERE t.text LIKE ?
    `;

    if (!includePrivate) {
      sql += ' AND t.is_private = 0';
    }

    if (options.username) {
      sql += ` AND u.username = '${options.username}'`;
    }

    sql += ' ORDER BY t.captured_at DESC LIMIT ?';

    const stmt = this.db.prepare(sql);
    return stmt.all(`%${query}%`, limit).map(this._parseTweet);
  }

  /**
   * Get tweets by user
   * @param {string} xUserId
   * @param {Object} options
   * @returns {Array}
   */
  getTweetsByUser(xUserId, options = {}) {
    const limit = options.limit || 20;
    const stmt = this.db.prepare(`
      SELECT t.*, u.username, u.display_name, u.profile_image_url
      FROM x_tweets t
      LEFT JOIN x_users u ON t.x_user_id = u.x_user_id
      WHERE t.x_user_id = ?
      ORDER BY t.created_at DESC
      LIMIT ?
    `);
    return stmt.all(xUserId, limit).map(this._parseTweet);
  }

  /**
   * Get tweet thread (conversation)
   * @param {string} conversationId
   * @returns {Array}
   */
  getTweetThread(conversationId) {
    const stmt = this.db.prepare(`
      SELECT t.*, u.username, u.display_name, u.profile_image_url
      FROM x_tweets t
      LEFT JOIN x_users u ON t.x_user_id = u.x_user_id
      WHERE t.conversation_id = ?
      ORDER BY t.created_at ASC
    `);
    return stmt.all(conversationId).map(this._parseTweet);
  }

  /**
   * Parse tweet JSON fields
   * @private
   */
  _parseTweet(tweet) {
    if (!tweet) return tweet;
    return {
      ...tweet,
      hashtags: JSON.parse(tweet.hashtags || '[]'),
      mentions: JSON.parse(tweet.mentions || '[]'),
      urls: JSON.parse(tweet.urls || '[]'),
      media: JSON.parse(tweet.media || '[]'),
      topics: JSON.parse(tweet.topics || '[]'),
      is_private: !!tweet.is_private,
      is_my_tweet: !!tweet.is_my_tweet,
      verified: !!tweet.verified,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // TRENDS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Upsert a trend
   * @param {Object} trend
   * @returns {Object}
   */
  upsertTrend(trend) {
    const stmt = this.db.prepare(`
      INSERT INTO x_trends (name, url, tweet_count, category)
      VALUES (?, ?, ?, ?)
      RETURNING *
    `);
    return stmt.get(
      trend.name,
      trend.url,
      trend.tweet_count || trend.tweetCount,
      trend.category,
    );
  }

  /**
   * Get recent trends
   * @param {number} limit
   * @returns {Array}
   */
  getRecentTrends(limit = 10) {
    const stmt = this.db.prepare(`
      SELECT * FROM x_trends
      ORDER BY captured_at DESC
      LIMIT ?
    `);
    return stmt.all(limit);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // FEEDS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Create a feed
   * @param {Object} feed
   * @returns {Object}
   */
  createFeed(feed) {
    const stmt = this.db.prepare(`
      INSERT INTO x_feeds (name, description, filters)
      VALUES (?, ?, ?)
      RETURNING *
    `);
    return stmt.get(
      feed.name,
      feed.description,
      JSON.stringify(feed.filters || {}),
    );
  }

  /**
   * Get all feeds
   * @returns {Array}
   */
  getFeeds() {
    const stmt = this.db.prepare('SELECT * FROM x_feeds ORDER BY created_at DESC');
    return stmt.all().map(f => ({
      ...f,
      filters: JSON.parse(f.filters || '{}'),
    }));
  }

  /**
   * Add tweet to feed
   * @param {number} feedId
   * @param {string} tweetId
   */
  addTweetToFeed(feedId, tweetId) {
    const stmt = this.db.prepare(`
      INSERT OR IGNORE INTO x_feed_tweets (feed_id, tweet_id)
      VALUES (?, ?)
    `);
    return stmt.run(feedId, tweetId);
  }

  /**
   * Get tweets in a feed
   * @param {number} feedId
   * @param {Object} options
   * @returns {Array}
   */
  getFeedTweets(feedId, options = {}) {
    const limit = options.limit || 20;
    const stmt = this.db.prepare(`
      SELECT t.*, u.username, u.display_name, u.profile_image_url
      FROM x_tweets t
      JOIN x_feed_tweets ft ON t.tweet_id = ft.tweet_id
      LEFT JOIN x_users u ON t.x_user_id = u.x_user_id
      WHERE ft.feed_id = ?
      ORDER BY ft.added_at DESC
      LIMIT ?
    `);
    return stmt.all(feedId, limit).map(this._parseTweet);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // SYNC MANAGEMENT
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Mark items for sync
   * @param {string} entityType - 'user', 'tweet', 'feed', 'trend'
   * @param {Array<string>} entityIds
   * @param {string} status - 'pending', 'synced', 'never'
   */
  markForSync(entityType, entityIds, status = 'pending') {
    const table = this._getTable(entityType);
    const idColumn = this._getIdColumn(entityType);

    const stmt = this.db.prepare(`
      UPDATE ${table}
      SET sync_status = ?, synced_at = CASE WHEN ? = 'synced' THEN datetime('now') ELSE synced_at END
      WHERE ${idColumn} IN (${entityIds.map(() => '?').join(',')})
    `);

    return stmt.run(status, status, ...entityIds);
  }

  /**
   * Get items pending sync
   * @param {string} entityType
   * @returns {Array}
   */
  getPendingSync(entityType) {
    const table = this._getTable(entityType);
    const stmt = this.db.prepare(`SELECT * FROM ${table} WHERE sync_status = 'pending'`);
    return stmt.all();
  }

  /**
   * Log a sync action
   * @param {string} entityType
   * @param {string} entityId
   * @param {string} action
   * @param {Object} details
   */
  logSync(entityType, entityId, action, details = {}) {
    const stmt = this.db.prepare(`
      INSERT INTO x_sync_log (entity_type, entity_id, action, details)
      VALUES (?, ?, ?, ?)
    `);
    return stmt.run(entityType, entityId, action, JSON.stringify(details));
  }

  /**
   * Get sync history for an entity
   * @param {string} entityType
   * @param {string} entityId
   * @returns {Array}
   */
  getSyncHistory(entityType, entityId) {
    const stmt = this.db.prepare(`
      SELECT * FROM x_sync_log
      WHERE entity_type = ? AND entity_id = ?
      ORDER BY synced_at DESC
    `);
    return stmt.all(entityType, entityId);
  }

  /**
   * Get table name for entity type
   * @private
   */
  _getTable(entityType) {
    const tables = {
      user: 'x_users',
      tweet: 'x_tweets',
      feed: 'x_feeds',
      trend: 'x_trends',
    };
    return tables[entityType] || 'x_tweets';
  }

  /**
   * Get ID column for entity type
   * @private
   */
  _getIdColumn(entityType) {
    const columns = {
      user: 'x_user_id',
      tweet: 'tweet_id',
      feed: 'id',
      trend: 'id',
    };
    return columns[entityType] || 'id';
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PRIVACY CONTROLS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Mark items as private (never sync)
   * @param {string} entityType
   * @param {Array<string>} entityIds
   */
  markAsPrivate(entityType, entityIds) {
    const table = this._getTable(entityType);
    const idColumn = this._getIdColumn(entityType);

    const stmt = this.db.prepare(`
      UPDATE ${table}
      SET is_private = 1, sync_status = 'never'
      WHERE ${idColumn} IN (${entityIds.map(() => '?').join(',')})
    `);

    return stmt.run(...entityIds);
  }

  /**
   * Mark my own tweets (for filtering)
   * @param {string} myUserId
   */
  markMyTweets(myUserId) {
    const stmt = this.db.prepare(`
      UPDATE x_tweets
      SET is_my_tweet = 1, is_private = 1, sync_status = 'never'
      WHERE x_user_id = ?
    `);
    return stmt.run(myUserId);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // MY ACTIONS (likes, RTs, bookmarks, follows)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Record a user action (like, retweet, bookmark, follow, etc.)
   * @param {Object} action
   * @param {string} action.actionType - 'like','unlike','retweet','unretweet','bookmark','unbookmark','follow','unfollow','tweet'
   * @param {string} [action.tweetId] - Target tweet ID
   * @param {string} [action.targetUserId] - Target user ID (for follow/unfollow)
   * @param {Object} [action.rawVariables] - Full mutation variables
   */
  recordAction(action) {
    const stmt = this.db.prepare(`
      INSERT INTO x_my_actions (action_type, tweet_id, target_user_id, raw_variables)
      VALUES (?, ?, ?, ?)
    `);
    return stmt.run(
      action.actionType,
      action.tweetId || null,
      action.targetUserId || null,
      action.rawVariables ? JSON.stringify(action.rawVariables) : null,
    );
  }

  /**
   * Get my recent actions
   * @param {Object} [options]
   * @param {string} [options.type] - Filter by action type
   * @param {number} [options.limit=50]
   * @returns {Array}
   */
  getMyActions(options = {}) {
    const limit = options.limit || 50;
    if (options.type) {
      return this.db.prepare(
        `SELECT a.*, u.username, u.display_name, substr(t.text, 1, 120) as tweet_preview
         FROM x_my_actions a
         LEFT JOIN x_tweets t ON a.tweet_id = t.tweet_id
         LEFT JOIN x_users u ON t.x_user_id = u.x_user_id
         WHERE a.action_type = ?
         ORDER BY a.created_at DESC LIMIT ?`,
      ).all(options.type, limit);
    }
    return this.db.prepare(
      `SELECT a.*, u.username, u.display_name, substr(t.text, 1, 120) as tweet_preview
       FROM x_my_actions a
       LEFT JOIN x_tweets t ON a.tweet_id = t.tweet_id
       LEFT JOIN x_users u ON t.x_user_id = u.x_user_id
       ORDER BY a.created_at DESC LIMIT ?`,
    ).all(limit);
  }

  /**
   * Get action stats summary
   * @returns {Object}
   */
  getActionStats() {
    return this.db.prepare(
      `SELECT action_type, COUNT(*) as count FROM x_my_actions GROUP BY action_type ORDER BY count DESC`,
    ).all();
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // STATS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get store statistics
   * @returns {Object}
   */
  getStats() {
    const tweets = this.db.prepare('SELECT COUNT(*) as count FROM x_tweets').get();
    const users = this.db.prepare('SELECT COUNT(*) as count FROM x_users').get();
    const trends = this.db.prepare('SELECT COUNT(*) as count FROM x_trends').get();
    const feeds = this.db.prepare('SELECT COUNT(*) as count FROM x_feeds').get();

    const syncStats = this.db.prepare(`
      SELECT sync_status, COUNT(*) as count
      FROM x_tweets
      GROUP BY sync_status
    `).all();

    const privateCount = this.db.prepare('SELECT COUNT(*) as count FROM x_tweets WHERE is_private = 1').get();

    const actions = this.db.prepare('SELECT COUNT(*) as count FROM x_my_actions').get();

    return {
      tweets: tweets.count,
      users: users.count,
      trends: trends.count,
      feeds: feeds.count,
      actions: actions.count,
      syncStats: Object.fromEntries(syncStats.map(s => [s.sync_status, s.count])),
      privateTweets: privateCount.count,
      dbPath: this.dbPath,
    };
  }

  /**
   * Close the database
   */
  close() {
    if (this.db) {
      this.db.close();
      this.db = null;
    }
  }
}

export default LocalXStore;
