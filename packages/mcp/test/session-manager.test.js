/**
 * Session Manager Tests
 *
 * Tests for the SessionManager that handles multi-user isolation.
 *
 * @module @cynic/mcp/test/session-manager
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { SessionManager } from '../src/session-manager.js';

/**
 * Create mock persistence with session store
 */
function createMockPersistence() {
  const sessions = new Map();

  return {
    sessionStore: {
      getOrCreate: async (sessionId, userId) => {
        const existing = sessions.get(userId);
        if (existing) return { ...existing }; // Return copy

        const session = {
          sessionId,
          userId,
          createdAt: new Date().toISOString(),
          lastActiveAt: new Date().toISOString(),
          judgmentCount: 0,
          digestCount: 0,
          feedbackCount: 0,
        };
        sessions.set(userId, session);
        return { ...session }; // Return copy to avoid shared reference
      },
      delete: async (sessionId) => {
        for (const [userId, session] of sessions) {
          if (session.sessionId === sessionId) {
            sessions.delete(userId);
            return true;
          }
        }
        return false;
      },
      increment: async (sessionId, field) => {
        // Just acknowledge - the manager tracks locally
        return;
      },
    },
    _sessions: sessions,
  };
}

describe('SessionManager', () => {
  let manager;
  let mockPersistence;

  beforeEach(() => {
    mockPersistence = createMockPersistence();
    manager = new SessionManager(mockPersistence);
  });

  describe('constructor', () => {
    it('creates with persistence', () => {
      assert.ok(manager.persistence);
      assert.ok(manager.activeSessions instanceof Map);
      assert.equal(manager._currentSession, null);
    });

    it('creates without persistence', () => {
      const mgr = new SessionManager(null);
      assert.equal(mgr.persistence, null);
    });
  });

  describe('getOrCreateSession', () => {
    it('creates new session for user', async () => {
      const session = await manager.getOrCreateSession('user123');

      assert.ok(session.sessionId);
      assert.ok(session.sessionId.startsWith('ses_'));
      assert.equal(session.userId, 'user123');
      assert.ok(session.createdAt);
    });

    it('returns cached session on subsequent calls', async () => {
      const session1 = await manager.getOrCreateSession('user456');
      const session2 = await manager.getOrCreateSession('user456');

      assert.equal(session1.sessionId, session2.sessionId);
    });

    it('creates different sessions for different projects', async () => {
      const session1 = await manager.getOrCreateSession('user789', { project: 'proj1' });
      const session2 = await manager.getOrCreateSession('user789', { project: 'proj2' });

      assert.notEqual(session1.sessionId, session2.sessionId);
    });

    it('uses default project when none specified', async () => {
      const session = await manager.getOrCreateSession('user000');

      assert.equal(session.project, 'default');
    });

    it('sets current session', async () => {
      const session = await manager.getOrCreateSession('userABC');

      assert.equal(manager._currentSession, session);
    });

    it('works without persistence', async () => {
      const mgr = new SessionManager(null);
      const session = await mgr.getOrCreateSession('localUser');

      assert.ok(session.sessionId);
      assert.equal(session.userId, 'localUser');
    });
  });

  describe('startSession', () => {
    it('starts new session', async () => {
      const session = await manager.startSession('startUser');

      assert.ok(session.sessionId);
      assert.equal(session.userId, 'startUser');
    });

    it('creates new session and keeps both active for different projects', async () => {
      // With deterministic session IDs, same user + same project = same ID
      // Different projects = different IDs, and both sessions can be active
      const session1 = await manager.startSession('replaceUser', { project: 'project-a' });
      const session2 = await manager.startSession('replaceUser', { project: 'project-b' });

      // Different projects = different deterministic session IDs
      assert.notEqual(session1.sessionId, session2.sessionId);
      // Both sessions should be active (different projects)
      assert.ok(manager.getSession(session1.sessionId));
      assert.ok(manager.getSession(session2.sessionId));
    });

    it('accepts project option', async () => {
      const session = await manager.startSession('projUser', { project: 'myProject' });

      assert.equal(session.project, 'myProject');
    });

    it('accepts metadata option', async () => {
      const session = await manager.startSession('metaUser', { metadata: { foo: 'bar' } });

      assert.ok(session);
    });
  });

  describe('endSession', () => {
    it('ends existing session', async () => {
      const session = await manager.startSession('endUser');
      const result = await manager.endSession(session.sessionId);

      assert.equal(result.ended, true);
      assert.ok(result.summary);
      assert.equal(result.summary.userId, 'endUser');
    });

    it('returns summary with stats', async () => {
      const session = await manager.startSession('statsUser');
      session.judgmentCount = 5;
      session.digestCount = 3;

      const result = await manager.endSession(session.sessionId);

      assert.equal(result.summary.judgmentCount, 5);
      assert.equal(result.summary.digestCount, 3);
      assert.ok(result.summary.duration >= 0);
    });

    it('clears current session if it was active', async () => {
      const session = await manager.startSession('currentUser');
      assert.equal(manager._currentSession, session);

      await manager.endSession(session.sessionId);

      assert.equal(manager._currentSession, null);
    });

    it('returns not found for unknown session', async () => {
      const result = await manager.endSession('ses_nonexistent');

      assert.equal(result.ended, false);
      assert.equal(result.reason, 'session_not_found');
    });

    it('removes session from active sessions', async () => {
      const session = await manager.startSession('removeUser');
      await manager.endSession(session.sessionId);

      assert.equal(manager.getSession(session.sessionId), null);
    });
  });

  describe('getSessionContext', () => {
    it('returns null values when no session', () => {
      const ctx = manager.getSessionContext();

      assert.equal(ctx.userId, null);
      assert.equal(ctx.sessionId, null);
    });

    it('returns session context when active', async () => {
      await manager.startSession('ctxUser', { project: 'ctxProject' });

      const ctx = manager.getSessionContext();

      assert.equal(ctx.userId, 'ctxUser');
      assert.ok(ctx.sessionId);
      assert.equal(ctx.project, 'ctxProject');
    });
  });

  describe('incrementCounter', () => {
    it('increments judgment count', async () => {
      await manager.startSession('incUser');

      await manager.incrementCounter('judgmentCount');
      await manager.incrementCounter('judgmentCount');

      assert.equal(manager._currentSession.judgmentCount, 2);
    });

    it('increments digest count', async () => {
      await manager.startSession('digestUser');

      await manager.incrementCounter('digestCount');

      assert.equal(manager._currentSession.digestCount, 1);
    });

    it('increments feedback count', async () => {
      await manager.startSession('fbUser');

      await manager.incrementCounter('feedbackCount');

      assert.equal(manager._currentSession.feedbackCount, 1);
    });

    it('does nothing when no current session', async () => {
      // Should not throw
      await manager.incrementCounter('judgmentCount');
    });

    it('ignores invalid field', async () => {
      await manager.startSession('invalidUser');

      await manager.incrementCounter('nonexistentField');

      // Should not throw, field not created as a number
      assert.ok(!('nonexistentField' in manager._currentSession) ||
                typeof manager._currentSession.nonexistentField !== 'number');
    });
  });

  describe('getSession', () => {
    it('returns session by ID', async () => {
      const created = await manager.startSession('getUser');
      const found = manager.getSession(created.sessionId);

      assert.equal(found.sessionId, created.sessionId);
    });

    it('returns null for unknown ID', () => {
      const found = manager.getSession('ses_unknown');

      assert.equal(found, null);
    });
  });

  describe('getSummary', () => {
    it('returns empty summary initially', () => {
      const summary = manager.getSummary();

      assert.equal(summary.activeCount, 0);
      assert.equal(summary.currentSession, null);
      assert.deepEqual(summary.sessions, []);
    });

    it('returns active sessions info', async () => {
      await manager.startSession('sum1');
      await manager.getOrCreateSession('sum2', { project: 'proj2' });

      const summary = manager.getSummary();

      assert.equal(summary.activeCount, 2);
      assert.ok(summary.currentSession);
      assert.equal(summary.sessions.length, 2);
    });

    it('truncates user IDs in summary', async () => {
      await manager.startSession('verylongusername');

      const summary = manager.getSummary();

      assert.ok(summary.sessions[0].userId.includes('...'));
    });

    it('includes session details', async () => {
      const session = await manager.startSession('detailUser', { project: 'detailProj' });
      session.judgmentCount = 10;

      const summary = manager.getSummary();

      assert.equal(summary.sessions[0].project, 'detailProj');
      assert.equal(summary.sessions[0].judgmentCount, 10);
      assert.ok(summary.sessions[0].createdAt);
    });
  });
});

describe('SessionManager with Redis errors', () => {
  it('falls back to local session on Redis error', async () => {
    const brokenPersistence = {
      sessionStore: {
        getOrCreate: async () => {
          throw new Error('Redis connection failed');
        },
      },
    };

    const manager = new SessionManager(brokenPersistence);
    const session = await manager.getOrCreateSession('fallbackUser');

    // Should still create a local session
    assert.ok(session.sessionId);
    assert.equal(session.userId, 'fallbackUser');
  });

  it('handles delete error gracefully', async () => {
    const errorPersistence = {
      sessionStore: {
        getOrCreate: async (id, userId) => ({
          sessionId: id,
          userId,
          createdAt: new Date().toISOString(),
        }),
        delete: async () => {
          throw new Error('Delete failed');
        },
      },
    };

    const manager = new SessionManager(errorPersistence);
    const session = await manager.startSession('deleteErrorUser');

    // Should not throw
    const result = await manager.endSession(session.sessionId);
    assert.equal(result.ended, true);
  });
});
