/**
 * Hot-Swappable Learning Weights Test (A2)
 *
 * Tests that Q-Learning weight updates are applied LIVE to routing,
 * not just at boot or restart.
 *
 * "L'organisme apprend en temps réel" - CYNIC
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';

import { globalEventBus, EventType } from '@cynic/core';
import { QLearningService } from '../src/orchestration/learning-service.js';
import { KabbalisticRouter } from '../src/orchestration/kabbalistic-router.js';
import { RelationshipGraph } from '../src/agents/collective/relationship-graph.js';
import { createCollectivePack } from '../src/agents/collective/index.js';

describe('A2: Hot-Swappable Learning Weights', () => {
  let learningService;
  let router;
  let collectivePack;

  beforeEach(async () => {
    collectivePack = createCollectivePack();
    learningService = new QLearningService();
    const relationshipGraph = new RelationshipGraph();

    router = new KabbalisticRouter({
      collectivePack,
      learningService,
      relationshipGraph,
    });
  });

  afterEach(() => {
    // Cleanup subscriptions
    if (router._qLearningSubscription) {
      router._qLearningSubscription?.();
    }
  });

  it('should hot-swap Q-Learning weights during session (no restart)', async () => {
    // Track if hot-swap event handler was called
    let hotSwapCalled = false;
    const originalHandler = router._handleQLearningWeightUpdate;
    router._handleQLearningWeightUpdate = function(...args) {
      hotSwapCalled = true;
      return originalHandler.call(this, ...args);
    };

    // 1. Start episode
    learningService.startEpisode({
      taskType: 'test',
      tool: 'Read',
    });

    // 2. Record actions
    learningService.recordAction('scout', { success: true });
    learningService.recordAction('analyst', { success: true });

    // 3. End episode with success (triggers Q-Learning update + event emission)
    const episode = learningService.endEpisode({
      success: true,
      score: 80,
    });

    assert.ok(episode, 'Episode should complete');

    // 4. Wait for event propagation (async)
    await new Promise(resolve => setTimeout(resolve, 50));

    // 5. Verify hot-swap handler was called (weights updated DURING session)
    assert.ok(
      hotSwapCalled,
      'Hot-swap handler should be called when Q-Learning updates weights'
    );
  });

  it('should emit QLEARNING_WEIGHT_UPDATE event on Q-value update', (t, done) => {
    let eventReceived = false;

    const unsubscribe = globalEventBus.subscribe(
      EventType.QLEARNING_WEIGHT_UPDATE,
      (event) => {
        const { action, qValue, delta } = event.payload || {};
        assert.ok(action, 'Event should have action (dog name)');
        assert.ok(qValue !== undefined, 'Event should have qValue');
        eventReceived = true;
      }
    );

    // Trigger learning cycle
    learningService.startEpisode({ taskType: 'test', tool: 'Write' });
    learningService.recordAction('architect', { success: true });
    learningService.endEpisode({ success: true, score: 90 });

    // Give time for async event
    setTimeout(() => {
      unsubscribe();
      assert.ok(eventReceived, 'QLEARNING_WEIGHT_UPDATE event should be emitted');
      done();
    }, 100);
  });

  it('should update relationship graph immediately on weight update', async () => {
    // 1. Get initial relationship graph state
    const initialWeight = router.relationshipGraph.getWeight?.('cynic', 'guardian') || 0.5;

    // 2. Trigger learning cycle with guardian action
    learningService.startEpisode({ taskType: 'security', tool: 'Bash' });
    learningService.recordAction('guardian', { success: true });
    learningService.endEpisode({ success: true, score: 95 });

    // 3. Wait for hot-swap
    await new Promise(resolve => setTimeout(resolve, 50));

    // 4. Verify relationship graph was updated (not just Q-table)
    const updatedWeight = router.relationshipGraph.getWeight?.('cynic', 'guardian') || 0.5;

    // Should be different (learning applied to graph)
    assert.ok(
      updatedWeight !== initialWeight,
      `Guardian weight in relationship graph should update: ${initialWeight} → ${updatedWeight}`
    );
  });

  it('should handle multiple rapid weight updates', async () => {
    const episodes = 5;

    for (let i = 0; i < episodes; i++) {
      learningService.startEpisode({ taskType: 'test', tool: 'Edit' });
      learningService.recordAction('oracle', { success: true });
      learningService.endEpisode({ success: true, score: 85 });
    }

    // Wait for all events to propagate
    await new Promise(resolve => setTimeout(resolve, 100));

    // Verify learning happened (oracle weight should converge)
    const finalWeights = router.getLearnedWeights();
    assert.ok(finalWeights?.oracle !== undefined, 'Oracle should have learned weight');
  });
});
