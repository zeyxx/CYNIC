/**
 * CYNIC Session Manager
 *
 * Coordinates multi-user session isolation.
 * Redis for active sessions, PostgreSQL for persistence.
 *
 * "Each dog knows its own master" - κυνικός
 *
 * @module @cynic/mcp/session-manager
 */

'use strict';

import crypto from 'crypto';
import { createLogger } from '@cynic/core';

const log = createLogger('SessionManager');

/**
 * Generate session ID
 */
function generateSessionId() {
  return 'ses_' + crypto.randomBytes(12).toString('hex');
}

/**
 * Session Manager for multi-user isolation
 *
 * Responsibilities:
 * - Create/manage user sessions
 * - Provide session context to tools
 * - Ensure data isolation per user/session
 * - Coordinate Redis (active) + PostgreSQL (persistent)
 */
export class SessionManager {
  /**
   * @param {Object} persistence - PersistenceManager instance
   */
  constructor(persistence) {
    this.persistence = persistence;
    this.activeSessions = new Map(); // Local cache for quick access
    this._currentSession = null; // Current active session for this server instance
  }

  /**
   * Get or create a session for a user
   * @param {string} userId - User identifier (wallet, email, etc.)
   * @param {Object} [context] - Additional session context (project, etc.)
   * @returns {Promise<Object>} Session object
   */
  async getOrCreateSession(userId, context = {}) {
    // Check local cache first
    const cacheKey = `${userId}:${context.project || 'default'}`;
    if (this.activeSessions.has(cacheKey)) {
      const session = this.activeSessions.get(cacheKey);
      session.lastActiveAt = new Date().toISOString();
      return session;
    }

    // Try Redis session store (if available)
    if (this.persistence?.sessionStore) {
      try {
        const sessionId = generateSessionId();
        const session = await this.persistence.sessionStore.getOrCreate(sessionId, userId);
        session.sessionId = sessionId;
        session.project = context.project || 'default';
        session.toolCalls = session.toolCalls || 0;
        session.errors = session.errors || 0;
        session.dangerBlocked = session.dangerBlocked || 0;

        // Also persist to PostgreSQL for cross-session history
        if (this.persistence?.sessions) {
          try {
            await this.persistence.sessions.create({
              sessionId,
              userId,
              judgmentCount: 0,
              digestCount: 0,
              feedbackCount: 0,
              context: { project: session.project },
            });
          } catch (pgErr) {
            log.warn('PostgreSQL session create error', { error: pgErr.message });
          }
        }

        // Cache locally
        this.activeSessions.set(cacheKey, session);
        this._currentSession = session;

        return session;
      } catch (err) {
        log.warn('Redis error', { error: err.message });
      }
    }

    // Fallback: create local session
    const session = {
      sessionId: generateSessionId(),
      userId,
      project: context.project || 'default',
      createdAt: new Date().toISOString(),
      lastActiveAt: new Date().toISOString(),
      judgmentCount: 0,
      digestCount: 0,
      feedbackCount: 0,
      toolCalls: 0,
      errors: 0,
      dangerBlocked: 0,
      context: context,
    };

    // Persist to PostgreSQL even without Redis
    if (this.persistence?.sessions) {
      try {
        await this.persistence.sessions.create({
          sessionId: session.sessionId,
          userId,
          judgmentCount: 0,
          digestCount: 0,
          feedbackCount: 0,
          context: { project: session.project },
        });
      } catch (pgErr) {
        log.warn('PostgreSQL session create error', { error: pgErr.message });
      }
    }

    this.activeSessions.set(cacheKey, session);
    this._currentSession = session;

    return session;
  }

  /**
   * Start a new session explicitly
   * @param {string} userId - User identifier
   * @param {Object} [options] - Session options
   * @returns {Promise<Object>} New session
   */
  async startSession(userId, options = {}) {
    const { project, metadata = {} } = options;

    // End any existing session for this user/project
    const cacheKey = `${userId}:${project || 'default'}`;
    if (this.activeSessions.has(cacheKey)) {
      await this.endSession(this.activeSessions.get(cacheKey).sessionId);
    }

    // Create new session
    const session = await this.getOrCreateSession(userId, { project, ...metadata });

    log.info('Session started', { sessionId: session.sessionId.slice(0, 12), userId: userId.slice(0, 8) });

    return session;
  }

  /**
   * End a session
   * @param {string} sessionId - Session to end
   * @returns {Promise<Object>} Session summary
   */
  async endSession(sessionId) {
    // Find session in cache
    let session = null;
    let cacheKey = null;

    for (const [key, s] of this.activeSessions) {
      if (s.sessionId === sessionId) {
        session = s;
        cacheKey = key;
        break;
      }
    }

    if (!session) {
      return { ended: false, reason: 'session_not_found' };
    }

    // Calculate session summary
    const summary = {
      sessionId,
      userId: session.userId,
      duration: Date.now() - new Date(session.createdAt).getTime(),
      judgmentCount: session.judgmentCount,
      digestCount: session.digestCount,
      feedbackCount: session.feedbackCount,
      endedAt: new Date().toISOString(),
    };

    // Persist session stats to PostgreSQL BEFORE removing from Redis
    if (this.persistence?.sessions) {
      try {
        await this.persistence.sessions.update(sessionId, {
          judgmentCount: summary.judgmentCount,
          digestCount: summary.digestCount,
          feedbackCount: summary.feedbackCount,
          context: session.context,
        });
        log.info('Session persisted to PostgreSQL', { sessionId: sessionId.slice(0, 12) });
      } catch (err) {
        log.error('Failed to persist session to PostgreSQL', { sessionId: sessionId.slice(0, 12), error: err.message });
      }
    }

    // Aggregate session stats into user_learning_profiles
    if (this.persistence?.userLearningProfiles && session.userId) {
      try {
        await this.persistence.userLearningProfiles.aggregateSessionStats(session.userId, {
          toolCalls: session.toolCalls || 0,
          errors: session.errors || 0,
          dangerBlocked: session.dangerBlocked || 0,
        });
        log.info('User profile aggregated', { userId: session.userId.slice(0, 8) });
      } catch (err) {
        log.warn('Failed to aggregate user profile', { error: err.message });
      }
    }

    log.info('Session ended', { sessionId: sessionId.slice(0, 12), judgments: summary.judgmentCount, digests: summary.digestCount });

    // Remove from Redis (if available) - only AFTER PostgreSQL persist
    if (this.persistence?.sessionStore) {
      try {
        await this.persistence.sessionStore.delete(sessionId);
      } catch (err) {
        log.warn('Error deleting from Redis', { error: err.message });
      }
    }

    // Remove from local cache
    if (cacheKey) {
      this.activeSessions.delete(cacheKey);
    }

    if (this._currentSession?.sessionId === sessionId) {
      this._currentSession = null;
    }

    return { ended: true, summary };
  }

  /**
   * Get current session context for tool calls
   * @returns {Object} Session context with userId and sessionId
   */
  getSessionContext() {
    if (!this._currentSession) {
      return { userId: null, sessionId: null };
    }

    return {
      userId: this._currentSession.userId,
      sessionId: this._currentSession.sessionId,
      project: this._currentSession.project,
    };
  }

  /**
   * Increment session counter
   * @param {string} field - Counter field (judgmentCount, digestCount, feedbackCount, toolCalls, errors, dangerBlocked)
   */
  async incrementCounter(field) {
    if (!this._currentSession) return;

    if (typeof this._currentSession[field] === 'number') {
      this._currentSession[field]++;
    }

    const sessionId = this._currentSession.sessionId;

    // Update Redis if available
    if (this.persistence?.sessionStore && sessionId) {
      try {
        await this.persistence.sessionStore.increment(sessionId, field);
      } catch (err) {
        // Non-critical for Redis
      }
    }

    // Also update PostgreSQL for persistence (only for session-table fields)
    // Map camelCase to snake_case for DB
    const fieldMap = {
      judgmentCount: 'judgment_count',
      digestCount: 'digest_count',
      feedbackCount: 'feedback_count',
    };

    // toolCalls, errors, dangerBlocked are tracked in-memory and aggregated at session end
    if (this.persistence?.sessions && sessionId && fieldMap[field]) {
      try {
        await this.persistence.sessions.increment(sessionId, fieldMap[field]);
      } catch (err) {
        log.warn('Error incrementing counter in PostgreSQL', { field, error: err.message });
      }
    }
  }

  /**
   * Record a tool call (for user profile aggregation)
   */
  recordToolCall() {
    if (this._currentSession) {
      this._currentSession.toolCalls = (this._currentSession.toolCalls || 0) + 1;
    }
  }

  /**
   * Record an error (for user profile aggregation)
   */
  recordError() {
    if (this._currentSession) {
      this._currentSession.errors = (this._currentSession.errors || 0) + 1;
    }
  }

  /**
   * Record a blocked danger (for user profile aggregation)
   */
  recordDangerBlocked() {
    if (this._currentSession) {
      this._currentSession.dangerBlocked = (this._currentSession.dangerBlocked || 0) + 1;
    }
  }

  /**
   * Get session by ID
   * @param {string} sessionId - Session ID
   * @returns {Object|null} Session or null
   */
  getSession(sessionId) {
    for (const session of this.activeSessions.values()) {
      if (session.sessionId === sessionId) {
        return session;
      }
    }
    return null;
  }

  /**
   * Get active sessions summary
   * @returns {Object} Summary of active sessions
   */
  getSummary() {
    const sessions = Array.from(this.activeSessions.values());
    return {
      activeCount: sessions.length,
      currentSession: this._currentSession?.sessionId || null,
      sessions: sessions.map(s => ({
        sessionId: s.sessionId,
        userId: s.userId?.slice(0, 8) + '...',
        project: s.project,
        judgmentCount: s.judgmentCount,
        createdAt: s.createdAt,
      })),
    };
  }
}

export default SessionManager;
