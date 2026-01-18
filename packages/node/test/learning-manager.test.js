/**
 * Learning Manager Tests
 *
 * Tests for the unified learning system
 *
 * "φ learns from every correction" - κυνικός
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

import { LearningManager, LearningService } from '../src/index.js';
import { PHI_INV_3 } from '@cynic/core';

describe('LearningManager', () => {
  let manager;

  beforeEach(async () => {
    manager = new LearningManager();
    await manager.init();
  });

  describe('Initialization', () => {
    it('should initialize with default config', () => {
      assert.ok(manager._initialized);
      assert.ok(manager.learningService instanceof LearningService);
    });

    it('should accept custom learning rate', async () => {
      const custom = new LearningManager({ learningRate: 0.1 });
      await custom.init();
      assert.strictEqual(custom.learningService.learningRate, 0.1);
    });

    it('should only initialize once', async () => {
      await manager.init();
      await manager.init();
      // No error means success
      assert.ok(manager._initialized);
    });

    it('should emit initialized event', async () => {
      const fresh = new LearningManager();
      let emitted = false;
      fresh.on('initialized', () => {
        emitted = true;
      });

      await fresh.init();
      assert.strictEqual(emitted, true);
    });
  });

  describe('Feedback Processing Delegation', () => {
    it('should delegate processFeedback to LearningService', () => {
      const result = manager.processFeedback({
        outcome: 'correct',
        originalScore: 75,
        itemType: 'code',
      });

      assert.strictEqual(result.scoreDelta, 0);
      assert.strictEqual(result.queueSize, 1);
    });

    it('should delegate getWeightModifier to LearningService', () => {
      const modifier = manager.getWeightModifier('COHERENCE');
      assert.strictEqual(modifier, 1.0);
    });

    it('should delegate getWeightModifiers to LearningService', () => {
      const modifiers = manager.getWeightModifiers();
      assert.ok(typeof modifiers === 'object');
      assert.ok('COHERENCE' in modifiers);
    });

    it('should delegate getThresholdAdjustment to LearningService', () => {
      const adjustment = manager.getThresholdAdjustment('code', 'COHERENCE');
      assert.strictEqual(adjustment, 0);
    });
  });

  describe('Learning Cycle (without persistence)', () => {
    it('should run learning cycle without persistence', async () => {
      // Add some feedback
      for (let i = 0; i < 3; i++) {
        manager.processFeedback({
          outcome: 'incorrect',
          actualScore: 30,
          originalScore: 80,
          itemType: 'code',
        });
      }

      const result = await manager.runLearningCycle();

      assert.ok(result.cycleId.startsWith('lrn_'));
      assert.strictEqual(result.feedback.processed, 3);
      assert.strictEqual(result.success, true);
      assert.ok(result.durationMs >= 0);
    });

    it('should emit cycle:complete event', async () => {
      let eventData = null;
      manager.on('cycle:complete', (data) => {
        eventData = data;
      });

      for (let i = 0; i < 3; i++) {
        manager.processFeedback({ outcome: 'correct', originalScore: 50 });
      }

      await manager.runLearningCycle();

      assert.ok(eventData);
      assert.strictEqual(eventData.success, true);
    });

    it('should update stats after cycle', async () => {
      const initialStats = await manager.getStats();
      const initialCycles = initialStats.manager.cyclesRun;

      for (let i = 0; i < 3; i++) {
        manager.processFeedback({ outcome: 'correct', originalScore: 50 });
      }

      await manager.runLearningCycle();

      const newStats = await manager.getStats();
      assert.strictEqual(newStats.manager.cyclesRun, initialCycles + 1);
    });
  });

  describe('State Management', () => {
    it('should get current state', () => {
      manager.processFeedback({ outcome: 'correct', originalScore: 50 });

      const state = manager.getState();

      assert.ok(state.weightModifiers);
      assert.ok(state.patterns);
      assert.ok(state.config);
      assert.ok(state.managerStats);
    });

    it('should get statistics', async () => {
      manager.processFeedback({ outcome: 'correct', originalScore: 50 });

      const stats = await manager.getStats();

      assert.ok(stats.manager);
      assert.ok(stats.service);
      assert.strictEqual(stats.initialized, true);
    });
  });

  describe('Top Patterns (without persistence)', () => {
    it('should get patterns from LearningService when no persistence', async () => {
      // Add some feedback to generate patterns
      for (let i = 0; i < 5; i++) {
        manager.processFeedback({
          outcome: 'incorrect',
          actualScore: 30,
          originalScore: 80,
          itemType: 'code',
        });
      }

      const patterns = await manager.getTopPatterns();

      // Without persistence, returns directly from LearningService.getPatterns()
      assert.ok(patterns.byItemType);
      assert.ok(patterns.overall);
    });
  });

  describe('E-Score Tracking (without persistence)', () => {
    it('should return null when recording E-Score without persistence', async () => {
      const result = await manager.recordEScore('user-1', 50, { hold: 10 }, 'test');
      assert.strictEqual(result, null);
    });

    it('should return no trend without persistence', async () => {
      const trend = await manager.getEScoreTrend('user-1', 7);
      assert.strictEqual(trend.hasTrend, false);
      assert.ok(trend.reason.includes('No E-Score history'));
    });
  });

  describe('User Profiles (without persistence)', () => {
    it('should return null when updating profile without persistence', async () => {
      const result = await manager.updateUserProfile('user-1', {
        outcome: 'correct',
        itemType: 'code',
      });
      assert.strictEqual(result, null);
    });

    it('should return null when getting profile without persistence', async () => {
      const profile = await manager.getUserProfile('user-1');
      assert.strictEqual(profile, null);
    });
  });

  describe('Patterns Needing Review (without persistence)', () => {
    it('should return empty array without persistence', async () => {
      const patterns = await manager.getPatternsNeedingReview();
      assert.deepStrictEqual(patterns, []);
    });
  });
});

describe('LearningManager with Mock Persistence', () => {
  let manager;
  let mockPersistence;

  beforeEach(async () => {
    // Create mock repositories
    mockPersistence = {
      feedback: {
        findUnapplied: async () => [],
        markApplied: async () => {},
      },
      escoreHistory: {
        recordSnapshot: async (userId, eScore, breakdown, trigger) => ({
          id: 'snapshot-1',
          user_id: userId,
          e_score: eScore,
          trigger_event: trigger,
        }),
        getLatest: async () => null,
        getTrend: async () => ({
          direction: 'up',
          velocity: 0.5,
          avgScore: 50,
          minScore: 45,
          maxScore: 55,
          dataPoints: 10,
        }),
        getDimensionTrends: async () => ({
          hasTrend: true,
          dimensions: {
            hold: { direction: 'up', velocity: 0.1, current: 10, previous: 8 },
          },
        }),
        getStats: async () => ({
          totalSnapshots: 100,
          uniqueUsers: 10,
          avgScore: 50,
        }),
      },
      learningCycles: {
        record: async (cycle) => ({ ...cycle, id: 'cycle-1' }),
        getStats: async () => ({
          totalCycles: 5,
          totalFeedback: 50,
        }),
      },
      patternEvolution: {
        upsert: async (pattern) => ({ ...pattern, id: 'pattern-1' }),
        getTopPatterns: async () => [
          { pattern_type: 'item_type', pattern_key: 'code', strength: 75 },
        ],
        getTrending: async () => [
          { pattern_type: 'dimension', pattern_key: 'COHERENCE', trend_direction: 'up' },
        ],
        getNeedingReview: async () => [],
        getStats: async () => ({
          total: 20,
          active: 18,
          merged: 2,
        }),
      },
      userLearningProfiles: {
        getOrCreate: async (userId) => ({
          id: 'profile-1',
          user_id: userId,
          learning_rate: PHI_INV_3,
        }),
        recordFeedback: async () => ({ total_feedback: 1 }),
        updateJudgmentPatterns: async () => ({}),
        recordActivity: async () => ({}),
        getSummary: async (userId) => ({
          userId,
          learningRate: PHI_INV_3,
          totalFeedback: 5,
          feedbackAccuracy: 0.8,
        }),
        getStats: async () => ({
          totalProfiles: 10,
          avgFeedback: 5,
        }),
      },
    };

    manager = new LearningManager({ persistence: mockPersistence });
    await manager.init();
  });

  describe('E-Score Tracking with Persistence', () => {
    it('should record E-Score snapshot', async () => {
      const result = await manager.recordEScore('user-1', 55, { hold: 15, burn: 10 }, 'judgment');

      assert.ok(result);
      assert.strictEqual(result.e_score, 55);
      assert.strictEqual(result.trigger_event, 'judgment');
    });

    it('should emit escore:recorded event', async () => {
      let eventData = null;
      manager.on('escore:recorded', (data) => {
        eventData = data;
      });

      await manager.recordEScore('user-1', 55, {}, 'test');

      assert.ok(eventData);
      assert.strictEqual(eventData.userId, 'user-1');
      assert.strictEqual(eventData.eScore, 55);
    });

    it('should get E-Score trend', async () => {
      const trend = await manager.getEScoreTrend('user-1', 7);

      assert.strictEqual(trend.direction, 'up');
      assert.strictEqual(trend.velocity, 0.5);
      assert.ok(trend.dimensions);
    });
  });

  describe('User Profiles with Persistence', () => {
    it('should update user profile', async () => {
      const result = await manager.updateUserProfile('user-1', {
        outcome: 'correct',
        itemType: 'code',
      });

      assert.ok(result);
      assert.strictEqual(result.user_id, 'user-1');
    });

    it('should emit profile:updated event', async () => {
      let eventData = null;
      manager.on('profile:updated', (data) => {
        eventData = data;
      });

      await manager.updateUserProfile('user-1', { outcome: 'correct' });

      assert.ok(eventData);
      assert.strictEqual(eventData.userId, 'user-1');
    });

    it('should get user profile summary', async () => {
      const profile = await manager.getUserProfile('user-1');

      assert.ok(profile);
      assert.strictEqual(profile.userId, 'user-1');
      assert.ok(profile.learningRate > 0);
    });
  });

  describe('Pattern Evolution with Persistence', () => {
    it('should get top patterns', async () => {
      const patterns = await manager.getTopPatterns();

      assert.ok(patterns.top);
      assert.ok(patterns.trending);
      assert.ok(patterns.inMemory);
    });
  });

  describe('Full Learning Cycle with Persistence', () => {
    it('should run cycle and update persistence', async () => {
      for (let i = 0; i < 3; i++) {
        manager.processFeedback({
          outcome: 'incorrect',
          actualScore: 30,
          originalScore: 80,
          itemType: 'code',
        });
      }

      const result = await manager.runLearningCycle();

      assert.ok(result.success);
      assert.ok(result.patterns.updated >= 0);
    });
  });

  describe('Statistics with Persistence', () => {
    it('should include persistence stats', async () => {
      const stats = await manager.getStats();

      assert.ok(stats.persistence);
      assert.ok(stats.persistence.escoreHistory);
      assert.ok(stats.persistence.learningCycles);
      assert.ok(stats.persistence.patternEvolution);
      assert.ok(stats.persistence.userProfiles);
    });
  });
});
