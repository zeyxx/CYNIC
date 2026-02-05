/**
 * Event Listeners Tests (AXE 2: PERSIST)
 *
 * Tests for data loop closure - ensuring judgments and feedback are persisted.
 *
 * @module @cynic/node/test/event-listeners
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert';
import { globalEventBus, EventType } from '@cynic/core';
import {
  startEventListeners,
  stopEventListeners,
  isRunning,
  getListenerStats,
} from '../src/services/event-listeners.js';

describe('EventListeners (AXE 2: PERSIST)', () => {
  // Mock repositories
  let mockJudgments = [];
  let mockFeedback = [];
  let mockSessionIncrements = [];

  const mockRepositories = {
    judgments: {
      create: async (judgment) => {
        mockJudgments.push(judgment);
        return { judgment_id: judgment.id, ...judgment };
      },
    },
    feedback: {
      create: async (feedback) => {
        mockFeedback.push(feedback);
        return { id: mockFeedback.length, ...feedback };
      },
    },
    sessions: {
      increment: async (sessionId, field) => {
        mockSessionIncrements.push({ sessionId, field });
        return { session_id: sessionId };
      },
    },
  };

  beforeEach(() => {
    mockJudgments = [];
    mockFeedback = [];
    mockSessionIncrements = [];
    stopEventListeners(); // Clean slate
  });

  afterEach(() => {
    stopEventListeners();
  });

  describe('startEventListeners', () => {
    it('should start listeners and track running state', () => {
      const control = startEventListeners({
        repositories: mockRepositories,
        sessionId: 'test-session',
        userId: 'test-user',
      });

      assert.ok(isRunning(), 'Listeners should be running');
      assert.ok(control.stop, 'Should return stop function');
      assert.ok(control.wireFeedbackProcessor, 'Should return wireFeedbackProcessor');
      assert.ok(control.getStats, 'Should return getStats');
    });

    it('should not start twice', () => {
      startEventListeners({ repositories: mockRepositories });
      const control2 = startEventListeners({ repositories: mockRepositories });

      // Second call should be a no-op
      assert.ok(isRunning(), 'Should still be running');
      assert.ok(control2.stop, 'Should still return control object');
    });
  });

  describe('JUDGMENT_CREATED persistence', () => {
    it('should persist judgment on JUDGMENT_CREATED event', async () => {
      startEventListeners({
        repositories: mockRepositories,
        sessionId: 'test-session',
        userId: 'test-user',
      });

      // Emit a judgment event
      globalEventBus.publish(EventType.JUDGMENT_CREATED, {
        qScore: 75,
        verdict: 'WAG',
        dimensions: { COHERENCE: 80, ACCURACY: 70 },
        itemType: 'test',
        confidence: 0.6,
      });

      // Wait for async handler
      await new Promise((r) => setTimeout(r, 50));

      assert.strictEqual(mockJudgments.length, 1, 'Should persist one judgment');
      assert.strictEqual(mockJudgments[0].qScore, 75);
      assert.strictEqual(mockJudgments[0].verdict, 'WAG');
      assert.strictEqual(mockJudgments[0].sessionId, 'test-session');
    });

    it('should skip judgment without score or verdict', async () => {
      startEventListeners({
        repositories: mockRepositories,
      });

      globalEventBus.publish(EventType.JUDGMENT_CREATED, {
        dimensions: { COHERENCE: 80 },
        // No qScore or verdict
      });

      await new Promise((r) => setTimeout(r, 50));

      assert.strictEqual(mockJudgments.length, 0, 'Should not persist invalid judgment');
    });

    it('should increment session judgment_count', async () => {
      startEventListeners({
        repositories: mockRepositories,
        sessionId: 'test-session',
      });

      globalEventBus.publish(EventType.JUDGMENT_CREATED, {
        qScore: 50,
        verdict: 'GROWL',
        itemType: 'test',
      });

      await new Promise((r) => setTimeout(r, 50));

      const judgeIncrements = mockSessionIncrements.filter(
        (inc) => inc.field === 'judgment_count'
      );
      assert.strictEqual(judgeIncrements.length, 1, 'Should increment judgment counter');
      assert.strictEqual(judgeIncrements[0].sessionId, 'test-session');
    });
  });

  describe('USER_FEEDBACK persistence', () => {
    it('should persist feedback on USER_FEEDBACK event', async () => {
      startEventListeners({
        repositories: mockRepositories,
        sessionId: 'test-session',
        userId: 'test-user',
      });

      globalEventBus.publish(EventType.USER_FEEDBACK, {
        judgmentId: 'jdg_123',
        outcome: 'correct',
        reason: 'Good judgment',
      });

      await new Promise((r) => setTimeout(r, 50));

      assert.strictEqual(mockFeedback.length, 1, 'Should persist feedback');
      assert.strictEqual(mockFeedback[0].outcome, 'correct');
      assert.strictEqual(mockFeedback[0].judgmentId, 'jdg_123');
    });

    it('should increment session feedback_count', async () => {
      startEventListeners({
        repositories: mockRepositories,
        sessionId: 'test-session',
      });

      globalEventBus.publish(EventType.USER_FEEDBACK, {
        outcome: 'incorrect',
      });

      await new Promise((r) => setTimeout(r, 50));

      const feedbackIncrements = mockSessionIncrements.filter(
        (inc) => inc.field === 'feedback_count'
      );
      assert.strictEqual(feedbackIncrements.length, 1, 'Should increment feedback counter');
    });
  });

  describe('stopEventListeners', () => {
    it('should stop all listeners', () => {
      startEventListeners({ repositories: mockRepositories });
      assert.ok(isRunning(), 'Should be running');

      stopEventListeners();
      assert.ok(!isRunning(), 'Should not be running after stop');
    });

    it('should not persist after stopping', async () => {
      startEventListeners({ repositories: mockRepositories });
      stopEventListeners();

      globalEventBus.publish(EventType.JUDGMENT_CREATED, {
        qScore: 50,
        verdict: 'WAG',
      });

      await new Promise((r) => setTimeout(r, 50));

      assert.strictEqual(mockJudgments.length, 0, 'Should not persist after stop');
    });
  });

  describe('getListenerStats', () => {
    it('should track statistics', async () => {
      startEventListeners({
        repositories: mockRepositories,
        sessionId: 'test-session',
      });

      // Generate some events
      globalEventBus.publish(EventType.JUDGMENT_CREATED, {
        qScore: 50,
        verdict: 'WAG',
      });
      globalEventBus.publish(EventType.USER_FEEDBACK, {
        outcome: 'correct',
      });

      await new Promise((r) => setTimeout(r, 100));

      const stats = getListenerStats();
      assert.ok(stats.running, 'Should show running');
      assert.ok(stats.startedAt, 'Should have startedAt timestamp');
      assert.ok(stats.judgmentsPersisted >= 1, 'Should track judgments persisted');
      assert.ok(stats.feedbackPersisted >= 1, 'Should track feedback persisted');
    });
  });

  describe('wireFeedbackProcessor', () => {
    it('should wire FeedbackProcessor events', async () => {
      const { EventEmitter } = await import('events');
      const mockProcessor = new EventEmitter();

      const control = startEventListeners({
        repositories: mockRepositories,
        sessionId: 'test-session',
      });

      control.wireFeedbackProcessor(mockProcessor, {
        judgmentId: 'jdg_456',
        source: 'test_result',
      });

      // Emit from processor
      mockProcessor.emit('feedback-processed', {
        scoreDelta: -10,
        queueSize: 5,
      });

      await new Promise((r) => setTimeout(r, 50));

      assert.strictEqual(mockFeedback.length, 1, 'Should persist from processor');
      assert.strictEqual(mockFeedback[0].sourceType, 'test_result');
    });
  });

  describe('retry mechanism', () => {
    it('should retry on transient failure', async () => {
      let attempts = 0;
      const failOnceRepo = {
        judgments: {
          create: async (judgment) => {
            attempts++;
            if (attempts === 1) {
              throw new Error('Transient DB error');
            }
            mockJudgments.push(judgment);
            return { judgment_id: judgment.id };
          },
        },
        sessions: mockRepositories.sessions,
      };

      startEventListeners({
        repositories: failOnceRepo,
        sessionId: 'test',
      });

      globalEventBus.publish(EventType.JUDGMENT_CREATED, {
        qScore: 50,
        verdict: 'WAG',
      });

      // Wait for retries
      await new Promise((r) => setTimeout(r, 500));

      assert.ok(attempts >= 2, 'Should have retried');
      assert.strictEqual(mockJudgments.length, 1, 'Should eventually persist');
    });
  });
});
