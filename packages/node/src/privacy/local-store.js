/**
 * @cynic/node - Local Store
 *
 * Device-only storage that NEVER syncs to the collective.
 * Raw signals, session histories, and personal data stay here.
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/privacy/local-store
 */

'use strict';

import { createHash, randomBytes } from 'crypto';
import { PHI_INV } from '@cynic/core';

/**
 * φ-aligned constants for local storage
 */
export const LOCAL_STORE_CONSTANTS = {
  /** Retention period in days (Fib(11) = 89) */
  RETENTION_DAYS: 89,

  /** Cleanup interval in hours (Fib(8) = 21) */
  CLEANUP_INTERVAL_HOURS: 21,

  /** Max entries per category (Fib(12) = 144) */
  MAX_ENTRIES_PER_CATEGORY: 144,

  /** Max total storage entries (Fib(14) = 377) */
  MAX_TOTAL_ENTRIES: 377,

  /** Session history limit (Fib(10) = 55) */
  MAX_SESSION_HISTORY: 55,

  /** Signal history per type (Fib(9) = 34) */
  MAX_SIGNAL_HISTORY: 34,
};

/**
 * Data categories that NEVER leave the device
 */
export const LocalDataCategory = {
  /** Raw user messages */
  RAW_MESSAGES: 'raw_messages',

  /** Full interaction logs */
  INTERACTIONS: 'interactions',

  /** Detailed profile signals */
  PROFILE_SIGNALS: 'profile_signals',

  /** Session histories */
  SESSION_HISTORY: 'session_history',

  /** Tool usage patterns */
  TOOL_PATTERNS: 'tool_patterns',

  /** Error occurrences */
  ERRORS: 'errors',

  /** Code snippets analyzed */
  CODE_SNIPPETS: 'code_snippets',
};

/**
 * Generate a unique local ID (never exposed externally)
 * @returns {string} Local identifier
 */
export function generateLocalId() {
  const timestamp = Date.now().toString(36);
  const random = randomBytes(8).toString('hex');
  return `local_${timestamp}_${random}`;
}

/**
 * Hash content for local indexing (not for external use)
 * @param {string} content - Content to hash
 * @returns {string} Local hash
 */
export function localHash(content) {
  return createHash('sha256')
    .update(content)
    .update('cynic_local_salt')
    .digest('hex')
    .slice(0, 16);
}

/**
 * Local-only data entry
 */
export class LocalEntry {
  /**
   * @param {string} category - Data category
   * @param {*} data - The data to store
   * @param {object} [metadata] - Optional metadata
   */
  constructor(category, data, metadata = {}) {
    this.id = generateLocalId();
    this.category = category;
    this.data = data;
    this.metadata = metadata;
    this.createdAt = new Date();
    this.accessedAt = new Date();
    this.accessCount = 0;
  }

  /**
   * Mark entry as accessed
   */
  touch() {
    this.accessedAt = new Date();
    this.accessCount++;
  }

  /**
   * Check if entry is expired
   * @returns {boolean}
   */
  isExpired() {
    const maxAge = LOCAL_STORE_CONSTANTS.RETENTION_DAYS * 24 * 60 * 60 * 1000;
    return Date.now() - this.createdAt.getTime() > maxAge;
  }

  /**
   * Get entry age in days
   * @returns {number}
   */
  getAgeDays() {
    return (Date.now() - this.createdAt.getTime()) / (24 * 60 * 60 * 1000);
  }
}

/**
 * Local-only storage manager
 *
 * CRITICAL: This data NEVER leaves the device.
 * Used for personalization without exposing to collective.
 */
export class LocalStore {
  constructor() {
    /** @type {Map<string, LocalEntry>} */
    this.entries = new Map();

    /** @type {Map<string, Set<string>>} Category -> entry IDs */
    this.categoryIndex = new Map();

    /** @type {Map<string, string[]>} Hash -> entry IDs for deduplication */
    this.hashIndex = new Map();

    this.lastCleanup = Date.now();
    this.stats = {
      totalAdded: 0,
      totalRemoved: 0,
      totalAccessed: 0,
    };
  }

  /**
   * Add entry to local store
   *
   * @param {string} category - Data category
   * @param {*} data - Data to store
   * @param {object} [options] - Storage options
   * @returns {LocalEntry} The stored entry
   */
  add(category, data, options = {}) {
    this._maybeCleanup();

    // Check capacity
    if (this.entries.size >= LOCAL_STORE_CONSTANTS.MAX_TOTAL_ENTRIES) {
      this._evictOldest();
    }

    // Check category capacity
    const categoryIds = this.categoryIndex.get(category) || new Set();
    if (categoryIds.size >= LOCAL_STORE_CONSTANTS.MAX_ENTRIES_PER_CATEGORY) {
      this._evictOldestInCategory(category);
    }

    // Create entry
    const entry = new LocalEntry(category, data, options.metadata);

    // Store
    this.entries.set(entry.id, entry);

    // Update category index
    if (!this.categoryIndex.has(category)) {
      this.categoryIndex.set(category, new Set());
    }
    this.categoryIndex.get(category).add(entry.id);

    // Update hash index for deduplication
    if (options.dedupKey) {
      const hash = localHash(options.dedupKey);
      if (!this.hashIndex.has(hash)) {
        this.hashIndex.set(hash, []);
      }
      this.hashIndex.get(hash).push(entry.id);
    }

    this.stats.totalAdded++;
    return entry;
  }

  /**
   * Get entry by ID
   *
   * @param {string} id - Entry ID
   * @returns {LocalEntry | null}
   */
  get(id) {
    const entry = this.entries.get(id);
    if (entry) {
      entry.touch();
      this.stats.totalAccessed++;
    }
    return entry || null;
  }

  /**
   * Get all entries in a category
   *
   * @param {string} category - Category to query
   * @param {object} [options] - Query options
   * @returns {LocalEntry[]}
   */
  getByCategory(category, options = {}) {
    const ids = this.categoryIndex.get(category);
    if (!ids) return [];

    let entries = Array.from(ids)
      .map(id => this.entries.get(id))
      .filter(e => e && !e.isExpired());

    // Sort by creation time (newest first by default)
    entries.sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime());

    // Apply limit
    if (options.limit) {
      entries = entries.slice(0, options.limit);
    }

    // Touch all accessed entries
    for (const entry of entries) {
      entry.touch();
      this.stats.totalAccessed++;
    }

    return entries;
  }

  /**
   * Get recent entries across all categories
   *
   * @param {number} [limit=21] - Max entries to return (default Fib(8))
   * @returns {LocalEntry[]}
   */
  getRecent(limit = 21) {
    const entries = Array.from(this.entries.values())
      .filter(e => !e.isExpired())
      .sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime())
      .slice(0, limit);

    for (const entry of entries) {
      entry.touch();
      this.stats.totalAccessed++;
    }

    return entries;
  }

  /**
   * Check if similar entry exists (by dedup key)
   *
   * @param {string} dedupKey - Deduplication key
   * @returns {boolean}
   */
  hasSimilar(dedupKey) {
    const hash = localHash(dedupKey);
    const ids = this.hashIndex.get(hash);
    if (!ids || ids.length === 0) return false;

    // Check if any non-expired entry exists
    return ids.some(id => {
      const entry = this.entries.get(id);
      return entry && !entry.isExpired();
    });
  }

  /**
   * Remove entry by ID
   *
   * @param {string} id - Entry ID
   * @returns {boolean} True if removed
   */
  remove(id) {
    const entry = this.entries.get(id);
    if (!entry) return false;

    // Remove from main store
    this.entries.delete(id);

    // Remove from category index
    const categoryIds = this.categoryIndex.get(entry.category);
    if (categoryIds) {
      categoryIds.delete(id);
    }

    this.stats.totalRemoved++;
    return true;
  }

  /**
   * Clear all entries in a category
   *
   * @param {string} category - Category to clear
   * @returns {number} Number of entries removed
   */
  clearCategory(category) {
    const ids = this.categoryIndex.get(category);
    if (!ids) return 0;

    let removed = 0;
    for (const id of ids) {
      if (this.entries.delete(id)) {
        removed++;
      }
    }

    this.categoryIndex.delete(category);
    this.stats.totalRemoved += removed;
    return removed;
  }

  /**
   * Clear ALL local data
   * WARNING: This is irreversible!
   *
   * @returns {{ entries: number, categories: number }}
   */
  clearAll() {
    const entriesCleared = this.entries.size;
    const categoriesCleared = this.categoryIndex.size;

    this.entries.clear();
    this.categoryIndex.clear();
    this.hashIndex.clear();

    this.stats.totalRemoved += entriesCleared;

    return {
      entries: entriesCleared,
      categories: categoriesCleared,
    };
  }

  /**
   * Run cleanup of expired entries
   *
   * @returns {{ removed: number, remaining: number }}
   */
  cleanup() {
    let removed = 0;

    for (const [id, entry] of this.entries) {
      if (entry.isExpired()) {
        this.remove(id);
        removed++;
      }
    }

    // Clean up hash index
    for (const [hash, ids] of this.hashIndex) {
      const validIds = ids.filter(id => this.entries.has(id));
      if (validIds.length === 0) {
        this.hashIndex.delete(hash);
      } else {
        this.hashIndex.set(hash, validIds);
      }
    }

    this.lastCleanup = Date.now();

    return {
      removed,
      remaining: this.entries.size,
    };
  }

  /**
   * Get storage statistics
   *
   * @returns {object}
   */
  getStats() {
    const categoryStats = {};
    for (const [category, ids] of this.categoryIndex) {
      categoryStats[category] = ids.size;
    }

    const oldestEntry = Array.from(this.entries.values())
      .sort((a, b) => a.createdAt.getTime() - b.createdAt.getTime())[0];

    const newestEntry = Array.from(this.entries.values())
      .sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime())[0];

    return {
      totalEntries: this.entries.size,
      maxEntries: LOCAL_STORE_CONSTANTS.MAX_TOTAL_ENTRIES,
      utilizationPercent: (this.entries.size / LOCAL_STORE_CONSTANTS.MAX_TOTAL_ENTRIES) * 100,
      categories: categoryStats,
      retentionDays: LOCAL_STORE_CONSTANTS.RETENTION_DAYS,
      oldestEntryAge: oldestEntry ? oldestEntry.getAgeDays() : 0,
      newestEntryAge: newestEntry ? newestEntry.getAgeDays() : 0,
      ...this.stats,
      lastCleanup: new Date(this.lastCleanup).toISOString(),
    };
  }

  /**
   * Maybe run cleanup if interval has passed
   * @private
   */
  _maybeCleanup() {
    const cleanupMs = LOCAL_STORE_CONSTANTS.CLEANUP_INTERVAL_HOURS * 60 * 60 * 1000;
    if (Date.now() - this.lastCleanup > cleanupMs) {
      this.cleanup();
    }
  }

  /**
   * Evict oldest entry globally
   * @private
   */
  _evictOldest() {
    const oldest = Array.from(this.entries.values())
      .sort((a, b) => a.accessedAt.getTime() - b.accessedAt.getTime())[0];

    if (oldest) {
      this.remove(oldest.id);
    }
  }

  /**
   * Evict oldest entry in category
   * @private
   * @param {string} category
   */
  _evictOldestInCategory(category) {
    const ids = this.categoryIndex.get(category);
    if (!ids || ids.size === 0) return;

    const entries = Array.from(ids)
      .map(id => this.entries.get(id))
      .filter(Boolean)
      .sort((a, b) => a.accessedAt.getTime() - b.accessedAt.getTime());

    if (entries.length > 0) {
      this.remove(entries[0].id);
    }
  }

  /**
   * Export data for backup (still local, not to collective!)
   *
   * @returns {object} Exportable data
   */
  exportForBackup() {
    return {
      exportedAt: new Date().toISOString(),
      version: '1.0.0',
      retentionDays: LOCAL_STORE_CONSTANTS.RETENTION_DAYS,
      entries: Array.from(this.entries.values()).map(e => ({
        id: e.id,
        category: e.category,
        data: e.data,
        metadata: e.metadata,
        createdAt: e.createdAt.toISOString(),
        accessedAt: e.accessedAt.toISOString(),
        accessCount: e.accessCount,
      })),
    };
  }

  /**
   * Import from backup
   *
   * @param {object} backup - Backup data
   * @returns {{ imported: number, skipped: number }}
   */
  importFromBackup(backup) {
    let imported = 0;
    let skipped = 0;

    for (const entryData of backup.entries || []) {
      // Skip if already exists
      if (this.entries.has(entryData.id)) {
        skipped++;
        continue;
      }

      // Reconstruct entry
      const entry = new LocalEntry(
        entryData.category,
        entryData.data,
        entryData.metadata
      );
      entry.id = entryData.id;
      entry.createdAt = new Date(entryData.createdAt);
      entry.accessedAt = new Date(entryData.accessedAt);
      entry.accessCount = entryData.accessCount || 0;

      // Skip expired entries
      if (entry.isExpired()) {
        skipped++;
        continue;
      }

      // Store
      this.entries.set(entry.id, entry);

      // Update category index
      if (!this.categoryIndex.has(entry.category)) {
        this.categoryIndex.set(entry.category, new Set());
      }
      this.categoryIndex.get(entry.category).add(entry.id);

      imported++;
    }

    return { imported, skipped };
  }
}

/**
 * Specialized store for profile signals
 * Keeps rolling history per signal type
 */
export class ProfileSignalStore extends LocalStore {
  constructor() {
    super();
    this.signalHistory = new Map();
  }

  /**
   * Record a profile signal
   *
   * @param {string} signalType - Type of signal (linguistic, behavioral, code, temporal)
   * @param {number} value - Signal value (0-100)
   * @param {object} [context] - Optional context
   */
  recordSignal(signalType, value, context = {}) {
    // Store in main store
    this.add(LocalDataCategory.PROFILE_SIGNALS, {
      type: signalType,
      value,
      context,
    });

    // Update rolling history
    if (!this.signalHistory.has(signalType)) {
      this.signalHistory.set(signalType, []);
    }

    const history = this.signalHistory.get(signalType);
    history.push({
      value,
      timestamp: Date.now(),
    });

    // Keep only recent history (Fib(9) = 34 entries)
    while (history.length > LOCAL_STORE_CONSTANTS.MAX_SIGNAL_HISTORY) {
      history.shift();
    }
  }

  /**
   * Get signal history for a type
   *
   * @param {string} signalType - Signal type
   * @returns {Array<{ value: number, timestamp: number }>}
   */
  getSignalHistory(signalType) {
    return this.signalHistory.get(signalType) || [];
  }

  /**
   * Get average signal value over history
   *
   * @param {string} signalType - Signal type
   * @returns {number} Average value (0-100)
   */
  getAverageSignal(signalType) {
    const history = this.getSignalHistory(signalType);
    if (history.length === 0) return 50; // Default middle value

    const sum = history.reduce((acc, h) => acc + h.value, 0);
    return sum / history.length;
  }

  /**
   * Get signal trend (positive = improving, negative = declining)
   *
   * @param {string} signalType - Signal type
   * @returns {number} Trend (-1 to 1)
   */
  getSignalTrend(signalType) {
    const history = this.getSignalHistory(signalType);
    if (history.length < 2) return 0;

    // Simple linear regression slope
    const n = history.length;
    let sumX = 0, sumY = 0, sumXY = 0, sumX2 = 0;

    for (let i = 0; i < n; i++) {
      sumX += i;
      sumY += history[i].value;
      sumXY += i * history[i].value;
      sumX2 += i * i;
    }

    const slope = (n * sumXY - sumX * sumY) / (n * sumX2 - sumX * sumX);

    // Normalize to -1 to 1 range
    return Math.max(-1, Math.min(1, slope / 10));
  }

  /**
   * Get all signal statistics
   *
   * @returns {object}
   */
  getSignalStats() {
    const stats = {};

    for (const [type, history] of this.signalHistory) {
      stats[type] = {
        current: history.length > 0 ? history[history.length - 1].value : null,
        average: this.getAverageSignal(type),
        trend: this.getSignalTrend(type),
        samples: history.length,
        maxConfidence: Math.min(PHI_INV, history.length / LOCAL_STORE_CONSTANTS.MAX_SIGNAL_HISTORY),
      };
    }

    return stats;
  }
}

/**
 * Specialized store for session history
 */
export class SessionHistoryStore extends LocalStore {
  constructor() {
    super();
    this.currentSession = null;
  }

  /**
   * Start a new session
   *
   * @param {object} [metadata] - Session metadata
   * @returns {string} Session ID
   */
  startSession(metadata = {}) {
    const sessionId = generateLocalId();

    this.currentSession = {
      id: sessionId,
      startedAt: new Date(),
      metadata,
      events: [],
      toolCalls: [],
      errors: [],
    };

    return sessionId;
  }

  /**
   * Record event in current session
   *
   * @param {string} eventType - Event type
   * @param {*} data - Event data
   */
  recordEvent(eventType, data) {
    if (!this.currentSession) return;

    this.currentSession.events.push({
      type: eventType,
      data,
      timestamp: new Date(),
    });
  }

  /**
   * Record tool call in current session
   *
   * @param {string} toolName - Tool name
   * @param {*} args - Tool arguments (sanitized)
   * @param {*} result - Tool result summary
   */
  recordToolCall(toolName, args, result) {
    if (!this.currentSession) return;

    this.currentSession.toolCalls.push({
      tool: toolName,
      args: this._sanitizeArgs(args),
      success: !result?.error,
      timestamp: new Date(),
    });
  }

  /**
   * Record error in current session
   *
   * @param {string} errorType - Error category
   * @param {string} message - Error message
   */
  recordError(errorType, message) {
    if (!this.currentSession) return;

    this.currentSession.errors.push({
      type: errorType,
      message,
      timestamp: new Date(),
    });
  }

  /**
   * End current session and store it
   *
   * @returns {LocalEntry | null}
   */
  endSession() {
    if (!this.currentSession) return null;

    this.currentSession.endedAt = new Date();
    this.currentSession.durationMs =
      this.currentSession.endedAt.getTime() - this.currentSession.startedAt.getTime();

    const entry = this.add(
      LocalDataCategory.SESSION_HISTORY,
      this.currentSession,
      { metadata: { sessionId: this.currentSession.id } }
    );

    this.currentSession = null;
    return entry;
  }

  /**
   * Get recent sessions
   *
   * @param {number} [limit=13] - Max sessions (Fib(7))
   * @returns {LocalEntry[]}
   */
  getRecentSessions(limit = 13) {
    return this.getByCategory(LocalDataCategory.SESSION_HISTORY, { limit });
  }

  /**
   * Get session statistics
   *
   * @returns {object}
   */
  getSessionStats() {
    const sessions = this.getByCategory(LocalDataCategory.SESSION_HISTORY, {
      limit: LOCAL_STORE_CONSTANTS.MAX_SESSION_HISTORY,
    });

    if (sessions.length === 0) {
      return {
        totalSessions: 0,
        avgDurationMs: 0,
        avgToolCalls: 0,
        avgErrors: 0,
      };
    }

    const totalDuration = sessions.reduce((acc, s) => acc + (s.data.durationMs || 0), 0);
    const totalToolCalls = sessions.reduce((acc, s) => acc + (s.data.toolCalls?.length || 0), 0);
    const totalErrors = sessions.reduce((acc, s) => acc + (s.data.errors?.length || 0), 0);

    return {
      totalSessions: sessions.length,
      avgDurationMs: totalDuration / sessions.length,
      avgToolCalls: totalToolCalls / sessions.length,
      avgErrors: totalErrors / sessions.length,
      errorRate: totalErrors / Math.max(1, totalToolCalls),
    };
  }

  /**
   * Sanitize tool arguments (remove sensitive data)
   * @private
   */
  _sanitizeArgs(args) {
    if (!args || typeof args !== 'object') return args;

    const sanitized = { ...args };

    // Remove potentially sensitive fields
    const sensitiveKeys = ['password', 'secret', 'token', 'key', 'auth', 'credential'];
    for (const key of Object.keys(sanitized)) {
      if (sensitiveKeys.some(s => key.toLowerCase().includes(s))) {
        sanitized[key] = '[REDACTED]';
      }
    }

    return sanitized;
  }
}

export default {
  LOCAL_STORE_CONSTANTS,
  LocalDataCategory,
  generateLocalId,
  localHash,
  LocalEntry,
  LocalStore,
  ProfileSignalStore,
  SessionHistoryStore,
};
