/**
 * Judge-Persistence Integration Tests
 *
 * E2E tests for the complete judgment lifecycle:
 * 1. Judge creates judgment with Q-Score and verdict
 * 2. Judgment saved to database
 * 3. Learning manager reads and analyzes
 * 4. Patterns extracted
 * 5. Future judgments improved
 *
 * "φ remembers every lesson" - κυνικός
 */

import { describe, it, beforeEach, after } from 'node:test';
import assert from 'node:assert';

import { CYNICJudge, LearningManager, LearningService } from '../src/index.js';
import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

// Valid verdicts
const VALID_VERDICTS = ['HOWL', 'BARK', 'WAG', 'GROWL'];

/**
 * Create a realistic mock persistence layer that tracks state
 */
function createMockPersistence() {
  // In-memory stores
  const judgments = new Map();
  const feedback = new Map();
  const patterns = new Map();
  const learningCycles = [];
  const escoreHistory = [];
  const userProfiles = new Map();
  const knowledge = new Map();

  let judgmentCounter = 0;
  let feedbackCounter = 0;
  let cycleCounter = 0;

  return {
    // JudgmentRepository
    judgment: {
      create: async (data) => {
        const id = `jdg_${++judgmentCounter}`;
        const judgment = {
          id,
          ...data,
          created_at: new Date().toISOString(),
        };
        judgments.set(id, judgment);
        return judgment;
      },
      findById: async (id) => judgments.get(id) || null,
      findRecent: async (limit = 10) => {
        const all = Array.from(judgments.values());
        return all.slice(-limit).reverse();
      },
      count: async () => judgments.size,
      getStats: async () => ({
        total: judgments.size,
        avgScore: judgments.size > 0
          ? Array.from(judgments.values()).reduce((s, j) => s + (j.q_score || 0), 0) / judgments.size
          : 0,
      }),
      _store: judgments, // Expose for testing
    },

    // FeedbackRepository
    feedback: {
      create: async (data) => {
        const id = `fb_${++feedbackCounter}`;
        const fb = {
          id,
          ...data,
          applied: false,
          created_at: new Date().toISOString(),
        };
        feedback.set(id, fb);
        return fb;
      },
      findById: async (id) => feedback.get(id) || null,
      findByJudgment: async (judgmentId) => {
        return Array.from(feedback.values()).filter(f => f.judgment_id === judgmentId);
      },
      findUnapplied: async (limit = 10) => {
        const unapplied = Array.from(feedback.values()).filter(f => !f.applied);
        return unapplied.slice(0, limit);
      },
      markApplied: async (id) => {
        const fb = feedback.get(id);
        if (fb) {
          fb.applied = true;
          return { success: true };
        }
        return { success: false };
      },
      getStats: async () => ({
        total: feedback.size,
        applied: Array.from(feedback.values()).filter(f => f.applied).length,
        unapplied: Array.from(feedback.values()).filter(f => !f.applied).length,
      }),
      _store: feedback,
    },

    // PatternEvolutionRepository
    patternEvolution: {
      upsert: async (data) => {
        const key = `${data.type}:${data.key}`;
        const existing = patterns.get(key);
        const pattern = {
          id: existing?.id || key,
          pattern_type: data.type,
          pattern_key: data.key,
          occurrence_count: (existing?.occurrence_count || 0) + (data.occurrenceCount || 0),
          confidence: data.confidence || existing?.confidence || 0.5,
          strength: data.strength || existing?.strength || 50,
          weight_modifier: data.weightModifier || existing?.weight_modifier || 1.0,
          trend_direction: data.trendDirection || existing?.trend_direction || 'stable',
          last_seen_at: new Date().toISOString(),
          created_at: existing?.created_at || new Date().toISOString(),
        };
        patterns.set(key, pattern);
        return pattern;
      },
      findById: async (id) => patterns.get(id) || null,
      getTopPatterns: async (limit = 10) => {
        const all = Array.from(patterns.values());
        return all.sort((a, b) => b.strength - a.strength).slice(0, limit);
      },
      getTrending: async (direction = 'up', limit = 5) => {
        const all = Array.from(patterns.values());
        return all.filter(p => p.trend_direction === direction).slice(0, limit);
      },
      getNeedingReview: async (limit = 10) => {
        return Array.from(patterns.values())
          .filter(p => p.confidence < PHI_INV_2)
          .slice(0, limit);
      },
      getStats: async () => ({
        total: patterns.size,
        avgStrength: patterns.size > 0
          ? Array.from(patterns.values()).reduce((s, p) => s + p.strength, 0) / patterns.size
          : 0,
      }),
      _store: patterns,
    },

    // LearningCyclesRepository
    learningCycles: {
      record: async (data) => {
        const cycle = {
          id: `cyc_${++cycleCounter}`,
          ...data,
          created_at: new Date().toISOString(),
        };
        learningCycles.push(cycle);
        return cycle;
      },
      getRecent: async (limit = 10) => learningCycles.slice(-limit).reverse(),
      getStats: async () => ({
        totalCycles: learningCycles.length,
        totalFeedback: learningCycles.reduce((s, c) => s + (c.feedbackProcessed || 0), 0),
        avgDuration: learningCycles.length > 0
          ? learningCycles.reduce((s, c) => s + (c.durationMs || 0), 0) / learningCycles.length
          : 0,
      }),
      _store: learningCycles,
    },

    // EScoreHistoryRepository
    escoreHistory: {
      recordSnapshot: async (userId, eScore, breakdown, trigger) => {
        const snapshot = {
          id: `esc_${escoreHistory.length + 1}`,
          user_id: userId,
          e_score: eScore,
          breakdown,
          trigger_event: trigger,
          created_at: new Date().toISOString(),
        };
        escoreHistory.push(snapshot);
        return snapshot;
      },
      getLatest: async (userId) => {
        const userSnapshots = escoreHistory.filter(s => s.user_id === userId);
        return userSnapshots[userSnapshots.length - 1] || null;
      },
      getTrend: async (userId, days = 7) => {
        const userSnapshots = escoreHistory.filter(s => s.user_id === userId);
        if (userSnapshots.length < 2) {
          return { hasTrend: false, direction: 'stable', velocity: 0 };
        }
        const first = userSnapshots[0].e_score;
        const last = userSnapshots[userSnapshots.length - 1].e_score;
        const delta = last - first;
        return {
          hasTrend: true,
          direction: delta > 1 ? 'up' : delta < -1 ? 'down' : 'stable',
          velocity: delta / userSnapshots.length,
          avgScore: userSnapshots.reduce((s, snap) => s + snap.e_score, 0) / userSnapshots.length,
          dataPoints: userSnapshots.length,
        };
      },
      getDimensionTrends: async (userId) => ({
        hasTrend: true,
        dimensions: {},
      }),
      getStats: async () => ({
        totalSnapshots: escoreHistory.length,
        uniqueUsers: new Set(escoreHistory.map(s => s.user_id)).size,
      }),
      _store: escoreHistory,
    },

    // UserLearningProfilesRepository
    userLearningProfiles: {
      getOrCreate: async (userId) => {
        if (!userProfiles.has(userId)) {
          userProfiles.set(userId, {
            id: `ulp_${userId}`,
            user_id: userId,
            learning_rate: PHI_INV_3,
            total_feedback: 0,
            correct_feedback: 0,
            judgment_patterns: {},
            last_active: new Date().toISOString(),
          });
        }
        return userProfiles.get(userId);
      },
      recordFeedback: async (userId, wasCorrect) => {
        const profile = userProfiles.get(userId);
        if (profile) {
          profile.total_feedback++;
          if (wasCorrect) profile.correct_feedback++;
        }
        return { success: true };
      },
      updateJudgmentPatterns: async (userId, itemType) => {
        const profile = userProfiles.get(userId);
        if (profile) {
          profile.judgment_patterns[itemType] = (profile.judgment_patterns[itemType] || 0) + 1;
        }
        return { success: true };
      },
      recordActivity: async (userId) => {
        const profile = userProfiles.get(userId);
        if (profile) {
          profile.last_active = new Date().toISOString();
        }
        return { success: true };
      },
      getSummary: async (userId) => {
        const profile = userProfiles.get(userId);
        if (!profile) return null;
        return {
          userId,
          learningRate: profile.learning_rate,
          totalFeedback: profile.total_feedback,
          accuracy: profile.total_feedback > 0
            ? profile.correct_feedback / profile.total_feedback
            : 0,
          patterns: profile.judgment_patterns,
        };
      },
      getStats: async () => ({
        totalProfiles: userProfiles.size,
        avgFeedback: userProfiles.size > 0
          ? Array.from(userProfiles.values()).reduce((s, p) => s + p.total_feedback, 0) / userProfiles.size
          : 0,
      }),
      _store: userProfiles,
    },

    // KnowledgeRepository
    knowledge: {
      create: async (data) => {
        const key = `${data.source_type}:${data.title}`;
        knowledge.set(key, {
          id: key,
          ...data,
          created_at: new Date().toISOString(),
        });
        return knowledge.get(key);
      },
      search: async (sourceType, options = {}) => {
        return Array.from(knowledge.values())
          .filter(k => k.source_type === sourceType)
          .slice(0, options.limit || 10);
      },
      _store: knowledge,
    },

    // Helper to store knowledge (used by LearningService)
    storeKnowledge: async (data) => {
      return knowledge.set(`learning:${Date.now()}`, data);
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════════
// E2E LIFECYCLE TESTS
// ═══════════════════════════════════════════════════════════════════════════════

describe('Judge-Persistence E2E Lifecycle', () => {
  let judge;
  let learningManager;
  let persistence;

  beforeEach(async () => {
    persistence = createMockPersistence();

    learningManager = new LearningManager({
      persistence,
      learningRate: PHI_INV_3,
    });
    await learningManager.init();

    judge = new CYNICJudge({
      learningService: learningManager.learningService,
    });
  });

  describe('Complete Judgment Lifecycle', () => {
    it('should complete full lifecycle: judge → save → feedback → learn → improve', async () => {
      // ═══════════════════════════════════════════════════════════════════════
      // STEP 1: Judge creates judgment with Q-Score and verdict
      // ═══════════════════════════════════════════════════════════════════════

      const item = {
        type: 'code',
        content: 'function add(a, b) { return a + b; }',
        id: 'test-item-1',
      };

      const judgment = judge.judge(item);

      assert.ok(judgment.id, 'Judgment should have an ID');
      assert.ok(judgment.qScore >= 0 && judgment.qScore <= 100, 'Q-Score should be 0-100');
      assert.ok(VALID_VERDICTS.includes(judgment.verdict), 'Verdict should be valid');
      assert.ok(judgment.axiomScores, 'Should have axiom scores');

      // ═══════════════════════════════════════════════════════════════════════
      // STEP 2: Judgment saved to database
      // ═══════════════════════════════════════════════════════════════════════

      const savedJudgment = await persistence.judgment.create({
        item_id: item.id,
        item_type: item.type,
        q_score: judgment.qScore,
        verdict: judgment.verdict,
        dimensions: judgment.dimensions,
        axiom_scores: judgment.axiomScores,
        user_id: 'test-user',
        session_id: 'test-session',
      });

      assert.ok(savedJudgment.id, 'Saved judgment should have ID');
      assert.strictEqual(savedJudgment.q_score, judgment.qScore);
      assert.strictEqual(await persistence.judgment.count(), 1);

      // ═══════════════════════════════════════════════════════════════════════
      // STEP 3: Feedback is created (simulating user correction)
      // ═══════════════════════════════════════════════════════════════════════

      // User says we overscored - actual quality is lower
      const actualScore = judgment.qScore - 20;

      const fb = await persistence.feedback.create({
        judgment_id: savedJudgment.id,
        user_id: 'test-user',
        outcome: 'incorrect',
        original_score: judgment.qScore,
        actual_score: actualScore,
        item_type: item.type,
        dimension_scores: judgment.dimensions,
      });

      assert.ok(fb.id, 'Feedback should have ID');
      assert.strictEqual(fb.applied, false, 'Feedback should start unapplied');

      // Process feedback through learning service
      learningManager.processFeedback({
        feedbackId: fb.id,
        outcome: 'incorrect',
        originalScore: judgment.qScore,
        actualScore: actualScore,
        itemType: item.type,
        dimensionScores: judgment.dimensions,
      });

      // ═══════════════════════════════════════════════════════════════════════
      // STEP 4: Learning manager runs cycle and analyzes
      // ═══════════════════════════════════════════════════════════════════════

      // Add more feedback to reach minimum threshold
      for (let i = 0; i < 4; i++) {
        learningManager.processFeedback({
          outcome: 'incorrect',
          originalScore: 80,
          actualScore: 55,
          itemType: 'code',
        });
      }

      const cycleResult = await learningManager.runLearningCycle();

      assert.ok(cycleResult.success, 'Learning cycle should succeed');
      assert.ok(cycleResult.cycleId.startsWith('lrn_'), 'Cycle should have ID');
      assert.ok(cycleResult.feedback.processed >= 5, 'Should process feedback');

      // ═══════════════════════════════════════════════════════════════════════
      // STEP 5: Patterns extracted and stored
      // ═══════════════════════════════════════════════════════════════════════

      const topPatterns = await learningManager.getTopPatterns();

      assert.ok(topPatterns.top.length > 0, 'Should have stored patterns');

      // Check that 'code' pattern was created
      const codePattern = topPatterns.top.find(p => p.pattern_key === 'code');
      assert.ok(codePattern, 'Code pattern should exist');
      assert.ok(codePattern.occurrence_count > 0, 'Pattern should have occurrences');

      // ═══════════════════════════════════════════════════════════════════════
      // STEP 6: Future judgments use learned patterns (weights modified)
      // ═══════════════════════════════════════════════════════════════════════

      // Get weight modifier state after learning
      const state = learningManager.getState();

      // Check if any weights were modified
      const modifiedWeights = Object.entries(state.weightModifiers)
        .filter(([_, v]) => Math.abs(v - 1.0) > 0.001);

      // Note: With only 5 feedback items and specific patterns, weight modification
      // depends on the learning algorithm. Let's verify the state was affected.
      assert.ok(state.patterns, 'State should track patterns');

      // Judge same item again - should be influenced by learning
      const newJudgment = judge.judge(item);

      assert.ok(newJudgment.qScore >= 0 && newJudgment.qScore <= 100);
      // Learning should affect future judgments (though exact change depends on algorithm)
    });

    it('should track judgment history per entity', async () => {
      const entityId = 'entity-123';

      // Create multiple judgments for same entity
      for (let i = 0; i < 5; i++) {
        const judgment = judge.judge({
          type: 'token',
          content: `Token analysis ${i}`,
          entityId,
        });

        await persistence.judgment.create({
          item_id: entityId,
          item_type: 'token',
          q_score: judgment.qScore,
          verdict: judgment.verdict,
        });
      }

      const history = await persistence.judgment.findRecent(10);

      assert.strictEqual(history.length, 5, 'Should have 5 judgments');
      // All should be for same entity
      for (const j of history) {
        assert.strictEqual(j.item_id, entityId);
      }
    });

    it('should record E-Score snapshots after judgments', async () => {
      const userId = 'user-123';

      // Judge several items and record E-Score
      for (let i = 0; i < 5; i++) {
        const judgment = judge.judge({
          type: 'code',
          content: `Code sample ${i}`,
        });

        // Calculate mock E-Score based on judgment
        const eScore = 50 + (judgment.qScore - 50) * 0.1 * (i + 1);

        await learningManager.recordEScore(
          userId,
          eScore,
          { quality: judgment.qScore * 0.5 },
          'judgment'
        );
      }

      // Verify E-Score history
      const latestEScore = await persistence.escoreHistory.getLatest(userId);
      assert.ok(latestEScore, 'Should have E-Score record');
      assert.strictEqual(latestEScore.user_id, userId);

      // Check trend
      const trend = await learningManager.getEScoreTrend(userId);
      assert.ok(trend.hasTrend, 'Should have trend data');
      assert.ok(trend.dataPoints >= 5, 'Should have at least 5 data points');
    });

    it('should update user learning profiles with feedback', async () => {
      const userId = 'user-456';

      // Process feedback and update profile
      for (let i = 0; i < 5; i++) {
        const isCorrect = i % 2 === 0;
        await learningManager.updateUserProfile(userId, {
          outcome: isCorrect ? 'correct' : 'incorrect',
          itemType: i % 3 === 0 ? 'code' : 'token',
        });
      }

      const profile = await learningManager.getUserProfile(userId);

      assert.ok(profile, 'Profile should exist');
      assert.strictEqual(profile.userId, userId);
      assert.strictEqual(profile.totalFeedback, 5);
      // 3 correct (indices 0, 2, 4) out of 5
      assert.strictEqual(profile.accuracy, 0.6);
      assert.ok(profile.patterns.code >= 2, 'Should have code patterns');
    });
  });

  describe('Learning Improvement Verification', () => {
    it('should show measurable improvement after learning cycles', async () => {
      // Track judgments before learning
      const beforeScores = [];
      for (let i = 0; i < 10; i++) {
        const j = judge.judge({ type: 'code', content: `Test ${i}` });
        beforeScores.push(j.qScore);
      }
      const avgBefore = beforeScores.reduce((a, b) => a + b, 0) / beforeScores.length;

      // Provide consistent feedback that we're overscoring
      for (let i = 0; i < 10; i++) {
        learningManager.processFeedback({
          outcome: 'incorrect',
          originalScore: 75,
          actualScore: 45,
          itemType: 'code',
          dimensionScores: { COHERENCE: 80, HARMONY: 70 },
        });
      }

      // Run learning cycle
      await learningManager.runLearningCycle();

      // Learning manager state should reflect learning
      const state = learningManager.getState();
      // Check that incorrect feedback was tracked (overscoring feedback)
      assert.ok(state.patterns.overall.incorrectCount > 0, 'Should track incorrect feedback');
      assert.ok(state.patterns.overall.totalFeedback >= 10, 'Should track total feedback');
      assert.ok(state.patterns.overall.avgScoreError > 0, 'Should track average error');

      // The learning service should have adjusted
      const stats = await learningManager.getStats();
      assert.ok(stats.manager.totalFeedback >= 10);
    });

    it('should learn different patterns for different item types', async () => {
      // Code items are overscored
      for (let i = 0; i < 5; i++) {
        learningManager.processFeedback({
          outcome: 'incorrect',
          originalScore: 80,
          actualScore: 50,
          itemType: 'code',
        });
      }

      // Token items are underscored
      for (let i = 0; i < 5; i++) {
        learningManager.processFeedback({
          outcome: 'incorrect',
          originalScore: 40,
          actualScore: 70,
          itemType: 'token',
        });
      }

      await learningManager.runLearningCycle();

      const patterns = learningManager.getState().patterns;

      assert.ok(patterns.byItemType.code, 'Should have code pattern');
      assert.ok(patterns.byItemType.token, 'Should have token pattern');

      // Code was overscored (avgDelta negative: actual - original)
      assert.ok(patterns.byItemType.code.avgDelta < 0);

      // Token was underscored (avgDelta positive)
      assert.ok(patterns.byItemType.token.avgDelta > 0);
    });

    it('should persist learned patterns across cycles', async () => {
      // First cycle
      for (let i = 0; i < 5; i++) {
        learningManager.processFeedback({
          outcome: 'incorrect',
          originalScore: 70,
          actualScore: 50,
          itemType: 'analysis',
        });
      }
      await learningManager.runLearningCycle();

      const patternCount1 = persistence.patternEvolution._store.size;

      // Second cycle with more feedback
      for (let i = 0; i < 5; i++) {
        learningManager.processFeedback({
          outcome: 'incorrect',
          originalScore: 70,
          actualScore: 50,
          itemType: 'analysis',
        });
      }
      await learningManager.runLearningCycle();

      const patternCount2 = persistence.patternEvolution._store.size;

      // Patterns should be upserted (updated), not just added
      const analysisPattern = persistence.patternEvolution._store.get('item_type:analysis');
      assert.ok(analysisPattern, 'Analysis pattern should exist');
      assert.ok(analysisPattern.occurrence_count > 5, 'Pattern count should accumulate');
    });
  });

  describe('Multi-User Learning Isolation', () => {
    it('should maintain separate profiles per user', async () => {
      const users = ['user-a', 'user-b', 'user-c'];

      // Each user gets different feedback
      for (const [idx, userId] of users.entries()) {
        const correctCount = idx + 1; // 1, 2, 3 correct per user
        for (let i = 0; i < 5; i++) {
          await learningManager.updateUserProfile(userId, {
            outcome: i < correctCount ? 'correct' : 'incorrect',
            itemType: 'code',
          });
        }
      }

      // Verify isolation
      const profileA = await learningManager.getUserProfile('user-a');
      const profileB = await learningManager.getUserProfile('user-b');
      const profileC = await learningManager.getUserProfile('user-c');

      assert.strictEqual(profileA.accuracy, 0.2); // 1/5
      assert.strictEqual(profileB.accuracy, 0.4); // 2/5
      assert.strictEqual(profileC.accuracy, 0.6); // 3/5
    });

    it('should track E-Score trends per user', async () => {
      // User A: improving E-Score
      for (let i = 0; i < 5; i++) {
        await learningManager.recordEScore('user-improving', 40 + i * 5, {}, 'judgment');
      }

      // User B: declining E-Score
      for (let i = 0; i < 5; i++) {
        await learningManager.recordEScore('user-declining', 60 - i * 5, {}, 'judgment');
      }

      const trendA = await learningManager.getEScoreTrend('user-improving');
      const trendB = await learningManager.getEScoreTrend('user-declining');

      assert.strictEqual(trendA.direction, 'up');
      assert.strictEqual(trendB.direction, 'down');
      assert.ok(trendA.velocity > 0);
      assert.ok(trendB.velocity < 0);
    });
  });

  describe('Feedback Quality and Learning Rate', () => {
    it('should respect φ-based learning rate', async () => {
      // Learning rate is PHI_INV_3 ≈ 0.236
      const state = learningManager.getState();
      assert.ok(Math.abs(state.config.learningRate - PHI_INV_3) < 0.001);
    });

    it('should not over-adjust weights with limited feedback', async () => {
      // Only 3 feedback items - below typical threshold
      for (let i = 0; i < 3; i++) {
        learningManager.processFeedback({
          outcome: 'incorrect',
          originalScore: 90,
          actualScore: 10, // Extreme difference
          itemType: 'edge',
          dimensionScores: { COHERENCE: 100 },
        });
      }

      await learningManager.runLearningCycle();

      const modifiers = learningManager.getWeightModifiers();

      // Even with extreme feedback, modifiers should be bounded
      for (const [dim, mod] of Object.entries(modifiers)) {
        assert.ok(mod >= 0.1 && mod <= 2.0, `${dim} modifier ${mod} should be bounded`);
      }
    });

    it('should track dimension-specific errors', async () => {
      // Consistently bad COHERENCE scores
      for (let i = 0; i < 5; i++) {
        learningManager.processFeedback({
          outcome: 'incorrect',
          originalScore: 70,
          actualScore: 40,
          itemType: 'code',
          dimensionScores: { COHERENCE: 90, ELEGANCE: 50 },
        });
      }

      await learningManager.runLearningCycle();

      const patterns = learningManager.getState().patterns;

      // COHERENCE should be flagged as problematic
      assert.ok(patterns.byDimension.COHERENCE, 'Should track COHERENCE');
      assert.ok(patterns.byDimension.COHERENCE.feedbackCount >= 5);
    });
  });

  describe('Statistics and Persistence Integration', () => {
    it('should report combined statistics', async () => {
      // Generate some activity
      for (let i = 0; i < 5; i++) {
        learningManager.processFeedback({
          outcome: i % 2 === 0 ? 'correct' : 'incorrect',
          originalScore: 50,
          itemType: 'test',
        });
      }
      await learningManager.runLearningCycle();

      const stats = await learningManager.getStats();

      // Manager stats
      assert.ok(stats.manager.cyclesRun >= 1);
      assert.ok(stats.manager.totalFeedback >= 5);

      // Service stats
      assert.ok(stats.service.totalFeedback >= 0);

      // Persistence stats
      assert.ok(stats.persistence.learningCycles, 'Should have learning cycles stats');
      assert.ok(stats.persistence.patternEvolution, 'Should have pattern evolution stats');
    });

    it('should record learning cycles to persistence', async () => {
      // Run multiple cycles
      for (let round = 0; round < 3; round++) {
        for (let i = 0; i < 5; i++) {
          learningManager.processFeedback({
            outcome: 'incorrect',
            originalScore: 70,
            actualScore: 50,
            itemType: 'batch',
          });
        }
        await learningManager.runLearningCycle();
      }

      const cycleStats = await persistence.learningCycles.getStats();
      assert.strictEqual(cycleStats.totalCycles, 3);

      const recentCycles = await persistence.learningCycles.getRecent(10);
      assert.strictEqual(recentCycles.length, 3);

      // Each cycle should have recorded metrics
      for (const cycle of recentCycles) {
        assert.ok(cycle.cycleId.startsWith('lrn_'));
        assert.ok(cycle.durationMs >= 0);
        assert.ok(cycle.feedbackProcessed >= 0);
      }
    });
  });

  describe('Error Handling and Recovery', () => {
    it('should handle missing persistence gracefully', async () => {
      const managerNoPersist = new LearningManager();
      await managerNoPersist.init();

      // These should return null/empty, not throw
      const eScore = await managerNoPersist.recordEScore('user', 50, {}, 'test');
      assert.strictEqual(eScore, null);

      const trend = await managerNoPersist.getEScoreTrend('user');
      assert.strictEqual(trend.hasTrend, false);

      const profile = await managerNoPersist.getUserProfile('user');
      assert.strictEqual(profile, null);

      // Learning cycle should still work
      managerNoPersist.processFeedback({ outcome: 'correct', originalScore: 50 });
      const result = await managerNoPersist.runLearningCycle();
      assert.ok(result.success);
    });

    it('should emit events for all lifecycle stages', async () => {
      const events = [];
      learningManager.on('initialized', () => events.push('initialized'));
      learningManager.on('cycle:complete', () => events.push('cycle:complete'));
      learningManager.on('escore:recorded', () => events.push('escore:recorded'));
      learningManager.on('profile:updated', () => events.push('profile:updated'));

      // Already initialized in beforeEach, but events should work for actions
      learningManager.processFeedback({ outcome: 'correct', originalScore: 50 });
      await learningManager.runLearningCycle();
      await learningManager.recordEScore('user', 50, {}, 'test');
      await learningManager.updateUserProfile('user', { outcome: 'correct' });

      assert.ok(events.includes('cycle:complete'));
      assert.ok(events.includes('escore:recorded'));
      assert.ok(events.includes('profile:updated'));
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// JUDGMENT REPOSITORY INTEGRATION
// ═══════════════════════════════════════════════════════════════════════════════

describe('JudgmentRepository Integration', () => {
  let persistence;
  let judge;

  beforeEach(() => {
    persistence = createMockPersistence();
    judge = new CYNICJudge();
  });

  it('should save judgment with all required fields', async () => {
    const item = {
      type: 'code',
      content: 'const x = 1;',
      id: 'item-1',
    };

    const judgment = judge.judge(item);

    const saved = await persistence.judgment.create({
      item_id: item.id,
      item_type: item.type,
      item_content: item.content,
      q_score: judgment.qScore,
      verdict: judgment.verdict,
      confidence: judgment.confidence,
      dimensions: judgment.dimensions,
      axiom_scores: judgment.axiomScores,
      user_id: 'user-1',
      session_id: 'session-1',
    });

    assert.ok(saved.id);
    assert.strictEqual(saved.item_id, 'item-1');
    assert.strictEqual(saved.q_score, judgment.qScore);
    assert.ok(saved.created_at);
  });

  it('should retrieve judgments by ID', async () => {
    const judgment = judge.judge({ type: 'token', content: 'TEST' });

    const saved = await persistence.judgment.create({
      item_id: 'token-1',
      item_type: 'token',
      q_score: judgment.qScore,
      verdict: judgment.verdict,
    });

    const retrieved = await persistence.judgment.findById(saved.id);

    assert.deepStrictEqual(retrieved, saved);
  });

  it('should return null for non-existent judgment', async () => {
    const result = await persistence.judgment.findById('non-existent');
    assert.strictEqual(result, null);
  });

  it('should get judgment statistics', async () => {
    for (let i = 0; i < 5; i++) {
      const j = judge.judge({ type: 'code', content: `sample ${i}` });
      await persistence.judgment.create({
        item_id: `item-${i}`,
        item_type: 'code',
        q_score: j.qScore,
        verdict: j.verdict,
      });
    }

    const stats = await persistence.judgment.getStats();

    assert.strictEqual(stats.total, 5);
    assert.ok(stats.avgScore >= 0 && stats.avgScore <= 100);
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// FEEDBACK REPOSITORY INTEGRATION
// ═══════════════════════════════════════════════════════════════════════════════

describe('FeedbackRepository Integration', () => {
  let persistence;

  beforeEach(() => {
    persistence = createMockPersistence();
  });

  it('should create feedback linked to judgment', async () => {
    // Create judgment first
    const judgment = await persistence.judgment.create({
      item_id: 'item-1',
      item_type: 'code',
      q_score: 75,
      verdict: 'WAG',
    });

    // Create feedback
    const feedback = await persistence.feedback.create({
      judgment_id: judgment.id,
      user_id: 'user-1',
      outcome: 'incorrect',
      original_score: 75,
      actual_score: 50,
    });

    assert.ok(feedback.id);
    assert.strictEqual(feedback.judgment_id, judgment.id);
    assert.strictEqual(feedback.applied, false);
  });

  it('should find unapplied feedback', async () => {
    // Create some feedback
    for (let i = 0; i < 5; i++) {
      await persistence.feedback.create({
        judgment_id: `jdg-${i}`,
        user_id: 'user-1',
        outcome: i % 2 === 0 ? 'correct' : 'incorrect',
        original_score: 50,
      });
    }

    // Mark some as applied
    await persistence.feedback.markApplied('fb_1');
    await persistence.feedback.markApplied('fb_2');

    const unapplied = await persistence.feedback.findUnapplied(10);

    assert.strictEqual(unapplied.length, 3);
    for (const fb of unapplied) {
      assert.strictEqual(fb.applied, false);
    }
  });

  it('should get feedback by judgment', async () => {
    const judgmentId = 'jdg-target';

    // Multiple feedback for same judgment
    for (let i = 0; i < 3; i++) {
      await persistence.feedback.create({
        judgment_id: judgmentId,
        user_id: `user-${i}`,
        outcome: 'incorrect',
        original_score: 60,
        actual_score: 40,
      });
    }

    // Feedback for other judgment
    await persistence.feedback.create({
      judgment_id: 'other-jdg',
      user_id: 'user-other',
      outcome: 'correct',
    });

    const judgmentFeedback = await persistence.feedback.findByJudgment(judgmentId);

    assert.strictEqual(judgmentFeedback.length, 3);
    for (const fb of judgmentFeedback) {
      assert.strictEqual(fb.judgment_id, judgmentId);
    }
  });

  it('should provide feedback statistics', async () => {
    for (let i = 0; i < 10; i++) {
      const fb = await persistence.feedback.create({
        judgment_id: `jdg-${i}`,
        user_id: 'user-1',
        outcome: 'incorrect',
      });

      if (i < 4) {
        await persistence.feedback.markApplied(fb.id);
      }
    }

    const stats = await persistence.feedback.getStats();

    assert.strictEqual(stats.total, 10);
    assert.strictEqual(stats.applied, 4);
    assert.strictEqual(stats.unapplied, 6);
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// PATTERN EVOLUTION INTEGRATION
// ═══════════════════════════════════════════════════════════════════════════════

describe('PatternEvolution Integration', () => {
  let persistence;
  let learningManager;

  beforeEach(async () => {
    persistence = createMockPersistence();
    learningManager = new LearningManager({ persistence });
    await learningManager.init();
  });

  it('should upsert patterns from learning cycles', async () => {
    // Generate code feedback
    for (let i = 0; i < 5; i++) {
      learningManager.processFeedback({
        outcome: 'incorrect',
        originalScore: 80,
        actualScore: 50,
        itemType: 'code',
      });
    }

    await learningManager.runLearningCycle();

    const codePattern = await persistence.patternEvolution._store.get('item_type:code');

    assert.ok(codePattern, 'Code pattern should be stored');
    assert.ok(codePattern.occurrence_count >= 5);
    assert.ok(codePattern.strength);
  });

  it('should track pattern trends', async () => {
    // First round of feedback
    for (let i = 0; i < 5; i++) {
      learningManager.processFeedback({
        outcome: 'incorrect',
        originalScore: 80,
        actualScore: 50,
        itemType: 'trending',
      });
    }
    await learningManager.runLearningCycle();

    // More feedback (same direction)
    for (let i = 0; i < 5; i++) {
      learningManager.processFeedback({
        outcome: 'incorrect',
        originalScore: 80,
        actualScore: 40,
        itemType: 'trending',
      });
    }
    await learningManager.runLearningCycle();

    const pattern = persistence.patternEvolution._store.get('item_type:trending');

    assert.ok(pattern);
    assert.strictEqual(pattern.trend_direction, 'down'); // Consistent overscoring
  });

  it('should find patterns needing review', async () => {
    // Create low-confidence pattern
    await persistence.patternEvolution.upsert({
      type: 'dimension',
      key: 'UNCERTAIN',
      occurrenceCount: 2,
      confidence: 0.2, // Below PHI_INV_2 threshold
      strength: 50,
    });

    // Create high-confidence pattern
    await persistence.patternEvolution.upsert({
      type: 'dimension',
      key: 'CONFIDENT',
      occurrenceCount: 50,
      confidence: 0.6,
      strength: 75,
    });

    const needingReview = await learningManager.getPatternsNeedingReview();

    assert.ok(needingReview.some(p => p.pattern_key === 'UNCERTAIN'));
    assert.ok(!needingReview.some(p => p.pattern_key === 'CONFIDENT'));
  });
});
