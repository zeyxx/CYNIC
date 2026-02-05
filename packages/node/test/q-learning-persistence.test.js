/**
 * @cynic/node - Q-Learning Router Persistence E2E Tests
 *
 * Verifies that Q-Learning state survives save→load cycles.
 * Gap 1 Fix: Prove Session A state === Session B restored state
 *
 * @module @cynic/node/test/q-learning-persistence
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert/strict';

import { QLearningRouter, QTable, Actions, StateFeatures } from '../src/orchestration/q-learning-router.js';

// =============================================================================
// MOCK HELPERS
// =============================================================================

function createMockPersistence() {
  const storage = {};
  return {
    query: mock.fn(async (sql, params) => {
      if (sql.includes('INSERT INTO')) {
        storage['default'] = JSON.parse(params[0]);
        return { rowCount: 1 };
      }
      if (sql.includes('SELECT')) {
        if (storage['default']) {
          return { rows: [{ data: storage['default'] }] };
        }
        return { rows: [] };
      }
      return { rows: [] };
    }),
    _storage: storage,
  };
}

// =============================================================================
// Q-TABLE PERSISTENCE TESTS
// =============================================================================

describe('QTable Persistence', () => {
  it('should serialize and deserialize Q-table via toJSON/fromJSON', () => {
    const qt = new QTable();

    // Add some Q-values
    qt.set([StateFeatures.TASK_SECURITY], Actions.GUARDIAN, 0.8);
    qt.visit([StateFeatures.TASK_SECURITY], Actions.GUARDIAN);
    qt.set([StateFeatures.TASK_ANALYSIS], Actions.ANALYST, 0.9);
    qt.visit([StateFeatures.TASK_ANALYSIS], Actions.ANALYST);
    qt.set([StateFeatures.TASK_EXPLORATION], Actions.SCOUT, 0.7);
    qt.set([StateFeatures.TASK_DOCUMENTATION], Actions.SCHOLAR, 0.6);

    // More visits to TASK_SECURITY
    qt.set([StateFeatures.TASK_SECURITY], Actions.GUARDIAN, 0.85);
    qt.visit([StateFeatures.TASK_SECURITY], Actions.GUARDIAN);
    qt.set([StateFeatures.TASK_SECURITY], Actions.GUARDIAN, 0.9);
    qt.visit([StateFeatures.TASK_SECURITY], Actions.GUARDIAN);

    // Serialize
    const json = qt.toJSON();

    // Deserialize
    const restored = QTable.fromJSON(json);

    // Verify Q-values match
    const origQ1 = qt.get([StateFeatures.TASK_SECURITY], Actions.GUARDIAN);
    const restoredQ1 = restored.get([StateFeatures.TASK_SECURITY], Actions.GUARDIAN);
    assert.ok(Math.abs(origQ1 - restoredQ1) < 0.001, `Q-value should match: ${origQ1} vs ${restoredQ1}`);

    const origQ2 = qt.get([StateFeatures.TASK_ANALYSIS], Actions.ANALYST);
    const restoredQ2 = restored.get([StateFeatures.TASK_ANALYSIS], Actions.ANALYST);
    assert.ok(Math.abs(origQ2 - restoredQ2) < 0.001, `Q-value should match: ${origQ2} vs ${restoredQ2}`);

    // Verify visits match
    const origVisits = qt.getVisits([StateFeatures.TASK_SECURITY], Actions.GUARDIAN);
    const restoredVisits = restored.getVisits([StateFeatures.TASK_SECURITY], Actions.GUARDIAN);
    assert.strictEqual(origVisits, restoredVisits, 'Visits should match');

    // Verify stats match
    assert.strictEqual(qt.stats.updates, restored.stats.updates, 'Update count should match');
  });

  it('should handle empty Q-table serialization', () => {
    const qt = new QTable();
    const json = qt.toJSON();
    const restored = QTable.fromJSON(json);

    assert.strictEqual(restored.table.size, 0, 'Empty table should serialize correctly');
    assert.strictEqual(restored.visits.size, 0, 'Empty visits should serialize correctly');
  });
});

// =============================================================================
// Q-LEARNING ROUTER PERSISTENCE TESTS
// =============================================================================

describe('QLearningRouter Persistence', () => {
  let router;
  let persistence;

  beforeEach(() => {
    persistence = createMockPersistence();
    router = new QLearningRouter({ persistence });
  });

  describe('save→load cycle', () => {
    it('should preserve Q-table through _persist→load cycle', async () => {
      // 1. Build diverse Q-learning state

      // Simulate learning episodes
      const episodes = [
        { features: [StateFeatures.TASK_SECURITY], action: Actions.GUARDIAN, reward: 0.9 },
        { features: [StateFeatures.TASK_SECURITY], action: Actions.GUARDIAN, reward: 0.85 },
        { features: [StateFeatures.TASK_ANALYSIS], action: Actions.ANALYST, reward: 0.8 },
        { features: [StateFeatures.TASK_EXPLORATION], action: Actions.SCOUT, reward: 0.75 },
        { features: [StateFeatures.TASK_CODE_CHANGE, StateFeatures.CONTEXT_COMPLEX], action: Actions.ARCHITECT, reward: 0.7 },
        { features: [StateFeatures.TASK_CLEANUP], action: Actions.JANITOR, reward: 0.6 },
        { features: [StateFeatures.TASK_DEPLOYMENT], action: Actions.DEPLOYER, reward: 0.95 },
      ];

      // Update Q-table with rewards
      for (const ep of episodes) {
        router.qTable.set(ep.features, ep.action, ep.reward);
      }

      // Decay exploration rate
      router.explorationRate = 0.15;

      // Update stats
      router.stats.routingDecisions = 100;
      router.stats.explorations = 25;
      router.stats.totalFeedback = 50;
      router.stats.correctPredictions = 40;

      // 2. Persist
      await router._persist();

      // 3. Create new router and load
      const newRouter = new QLearningRouter({ persistence });
      const loaded = await newRouter.load();

      assert.strictEqual(loaded, true, 'Load should succeed');

      // ═══════════════════════════════════════════════════════════════
      // VERIFY COMPLETE STATE EQUALITY
      // ═══════════════════════════════════════════════════════════════

      // A. Verify Q-values match for each episode
      for (const ep of episodes) {
        const origQ = router.qTable.get(ep.features, ep.action);
        const loadedQ = newRouter.qTable.get(ep.features, ep.action);
        assert.ok(
          Math.abs(origQ - loadedQ) < 0.001,
          `Q-value for ${ep.action} should match: ${origQ} vs ${loadedQ}`
        );
      }

      // B. Verify exploration rate
      assert.strictEqual(
        newRouter.explorationRate,
        router.explorationRate,
        `Exploration rate should match: ${router.explorationRate} vs ${newRouter.explorationRate}`
      );

      // C. Verify stats
      assert.strictEqual(newRouter.stats.routingDecisions, router.stats.routingDecisions, 'routingDecisions should match');
      assert.strictEqual(newRouter.stats.explorations, router.stats.explorations, 'explorations should match');
      assert.strictEqual(newRouter.stats.totalFeedback, router.stats.totalFeedback, 'totalFeedback should match');
      assert.strictEqual(newRouter.stats.correctPredictions, router.stats.correctPredictions, 'correctPredictions should match');

      // D. Verify Q-table stats
      assert.strictEqual(
        newRouter.qTable.stats.updates,
        router.qTable.stats.updates,
        'Q-table update count should match'
      );
    });

    it('should handle load with no persisted data', async () => {
      const emptyPersistence = {
        query: mock.fn(async () => ({ rows: [] })),
      };
      const newRouter = new QLearningRouter({ persistence: emptyPersistence });
      const loaded = await newRouter.load();

      assert.strictEqual(loaded, false, 'Load should return false when no data');
      assert.strictEqual(newRouter.qTable.table.size, 0, 'Q-table should be empty');
    });

    it('should preserve learned policy across sessions', async () => {
      // 1. Train the router: Guardian is best for security tasks
      // Set Q-values with large gap so softmax clearly favors Guardian
      router.qTable.set([StateFeatures.TASK_SECURITY], Actions.GUARDIAN, 0.9);
      router.qTable.set([StateFeatures.TASK_SECURITY], Actions.ANALYST, 0.1);
      router.qTable.set([StateFeatures.TASK_SECURITY], Actions.SCOUT, 0.1);
      // Set negative Q for other actions to make Guardian dominant
      router.qTable.set([StateFeatures.TASK_SECURITY], Actions.ARCHITECT, -0.5);
      router.qTable.set([StateFeatures.TASK_SECURITY], Actions.JANITOR, -0.5);
      router.qTable.set([StateFeatures.TASK_SECURITY], Actions.DEPLOYER, -0.5);
      router.qTable.set([StateFeatures.TASK_SECURITY], Actions.SCHOLAR, -0.5);
      router.qTable.set([StateFeatures.TASK_SECURITY], Actions.SAGE, -0.5);
      router.qTable.set([StateFeatures.TASK_SECURITY], Actions.ORACLE, -0.5);
      router.qTable.set([StateFeatures.TASK_SECURITY], Actions.CARTOGRAPHER, -0.5);

      // Set low exploration to force exploitation
      router.explorationRate = 0.01;

      // 2. Persist
      await router._persist();

      // 3. Load in new router with low temperature to make policy more deterministic
      const newRouter = new QLearningRouter({
        persistence,
        config: { temperature: 0.1 }, // Low temperature = more deterministic
      });
      await newRouter.load();

      // 4. Verify policy preserved: Guardian should be selected for security
      // selectAction returns { action, method, ... }
      let guardianCount = 0;
      for (let i = 0; i < 20; i++) {
        const result = newRouter.selectAction([StateFeatures.TASK_SECURITY]);
        if (result.action === Actions.GUARDIAN) guardianCount++;
      }

      // With 1% exploration, low temperature, and Guardian having highest Q-value,
      // we should see Guardian selected most of the time (>80%)
      assert.ok(
        guardianCount >= 16,
        `Guardian should be selected most often for security tasks: ${guardianCount}/20`
      );
    });
  });

  describe('without persistence', () => {
    it('should not throw when persisting without persistence layer', async () => {
      const noPersistRouter = new QLearningRouter({});
      noPersistRouter.qTable.set([StateFeatures.TASK_SECURITY], Actions.GUARDIAN, 0.9);

      // Should not throw
      await noPersistRouter._persist();
    });

    it('should return false when loading without persistence layer', async () => {
      const noPersistRouter = new QLearningRouter({});
      const loaded = await noPersistRouter.load();
      assert.strictEqual(loaded, false);
    });
  });
});

// =============================================================================
// E2E SESSION CONTINUITY TEST
// =============================================================================

describe('Q-Learning Session Continuity (E2E)', () => {
  it('should maintain learning continuity across multiple save→load cycles', async () => {
    const persistence = createMockPersistence();

    // Session 1: Initial learning
    const session1 = new QLearningRouter({ persistence });
    session1.qTable.set([StateFeatures.TASK_SECURITY], Actions.GUARDIAN, 0.7);
    session1.qTable.visit([StateFeatures.TASK_SECURITY], Actions.GUARDIAN);
    session1.explorationRate = 0.3;
    await session1._persist();

    // Session 2: Continue learning
    const session2 = new QLearningRouter({ persistence });
    await session2.load();
    session2.qTable.set([StateFeatures.TASK_SECURITY], Actions.GUARDIAN, 0.85);
    session2.qTable.visit([StateFeatures.TASK_SECURITY], Actions.GUARDIAN);
    session2.qTable.set([StateFeatures.TASK_ANALYSIS], Actions.ANALYST, 0.8);
    session2.qTable.visit([StateFeatures.TASK_ANALYSIS], Actions.ANALYST);
    session2.explorationRate = 0.2;
    await session2._persist();

    // Session 3: Continue learning
    const session3 = new QLearningRouter({ persistence });
    await session3.load();
    session3.qTable.set([StateFeatures.TASK_SECURITY], Actions.GUARDIAN, 0.95);
    session3.qTable.visit([StateFeatures.TASK_SECURITY], Actions.GUARDIAN);
    session3.explorationRate = 0.1;
    await session3._persist();

    // Session 4: Verify all learning accumulated
    const session4 = new QLearningRouter({ persistence });
    await session4.load();

    // Q-values should reflect the latest persisted value
    const finalSecurityQ = session4.qTable.get([StateFeatures.TASK_SECURITY], Actions.GUARDIAN);
    const finalAnalysisQ = session4.qTable.get([StateFeatures.TASK_ANALYSIS], Actions.ANALYST);

    assert.ok(finalSecurityQ >= 0.9, `Security Q-value should be high: ${finalSecurityQ}`);
    assert.ok(finalAnalysisQ > 0, `Analysis Q-value should exist: ${finalAnalysisQ}`);
    assert.strictEqual(session4.explorationRate, 0.1, 'Final exploration rate should be from session 3');

    // Verify visit counts accumulated across sessions
    const securityVisits = session4.qTable.getVisits([StateFeatures.TASK_SECURITY], Actions.GUARDIAN);
    assert.ok(securityVisits >= 3, `Visit count should accumulate: ${securityVisits}`);
  });
});
