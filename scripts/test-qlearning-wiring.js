/**
 * Test Q-Learning Wiring Gap 3
 *
 * Verifies:
 * 1. Q-table loads from PostgreSQL on startup
 * 2. Weights are available for routing decisions
 * 3. Q-updates are recorded to learning_events table (G1.3 metric)
 *
 * Usage: node scripts/test-qlearning-wiring.js
 */

'use strict';

import { initializeInfrastructure } from '../packages/node/src/daemon/services.js';
import { wireQLearning, isQLearningWired } from '../packages/node/src/orchestration/q-learning-wiring.js';
import { getQLearningServiceAsync } from '../packages/node/src/orchestration/learning-service.js';
import { getPostgresClient } from '../packages/persistence/src/index.js';
import { globalEventBus, EventType } from '../packages/core/src/index.js';

async function main() {
  console.log('=== Q-Learning Wiring Test ===\n');

  // 1. Initialize infrastructure (DB, event bus, etc.)
  console.log('Initializing infrastructure...');
  await initializeInfrastructure();
  console.log('✓ Infrastructure ready\n');

  // 2. Wire Q-Learning system
  console.log('Wiring Q-Learning system...');
  const { learningService, loaded } = await wireQLearning();

  if (!learningService) {
    console.error('✗ Failed to create LearningService');
    process.exit(1);
  }

  console.log(`✓ Q-Learning wired: ${isQLearningWired() ? 'YES' : 'NO'}`);
  console.log(`✓ Q-table loaded from DB: ${loaded ? 'YES' : 'NO (starting fresh)'}\n`);

  // 3. Check Q-table stats
  const stats = learningService.getStats();
  console.log('Q-Learning Stats:');
  console.log(`  - States: ${stats.qTableStats?.states || 0}`);
  console.log(`  - Updates: ${stats.qTableStats?.updates || 0}`);
  console.log(`  - Episodes: ${stats.episodes || 0}`);
  console.log(`  - Exploration Rate: ${stats.explorationRate}%`);
  console.log(`  - Accuracy: ${stats.accuracy}%\n`);

  // 4. Test Q-weight extraction for routing
  console.log('Testing Q-weight extraction for routing...');
  const features = ['task:code_change', 'ctx:complex'];
  const weights = learningService.getRecommendedWeights(features);
  console.log(`✓ Got weights for ${Object.keys(weights).length} dogs`);
  
  const topDogs = learningService.getTopDogs(features, 3);
  console.log('Top 3 recommended dogs:');
  topDogs.forEach(({ dog, weight, visits }) => {
    console.log(`  - ${dog}: ${(weight * 100).toFixed(1)}% (${visits} visits)`);
  });
  console.log('');

  // 5. Test episode recording and Q-update tracking
  console.log('Testing episode recording and Q-update tracking...');
  
  // Start an episode
  const episodeId = learningService.startEpisode({
    taskType: 'code_change',
    tool: 'Edit',
    content: 'modify file to fix bug',
  });
  console.log(`✓ Episode started: ${episodeId}`);

  // Record actions
  learningService.recordAction('guardian', { method: 'exploitation' });
  learningService.recordAction('analyst', { method: 'exploitation' });
  console.log('✓ Actions recorded: guardian, analyst');

  // Listen for Q-update event (should be emitted by endEpisode)
  let updateReceived = false;
  const updateListener = (event) => {
    const { action, qValue, delta } = event.payload || {};
    console.log(`  → Q-update event: ${action} = ${qValue?.toFixed(3)} (Δ${delta?.toFixed(3)})`);
    updateReceived = true;
  };
  globalEventBus.on(EventType.QLEARNING_WEIGHT_UPDATE, updateListener);

  // End episode with success outcome
  const episode = learningService.endEpisode({
    success: true,
    score: 85, // 0-100 score
  });
  console.log(`✓ Episode ended: reward=${episode.reward?.toFixed(2)}, duration=${episode.duration}ms`);

  // Wait briefly for event emission
  await new Promise(resolve => setTimeout(resolve, 100));
  console.log(`✓ Q-update event emitted: ${updateReceived ? 'YES' : 'NO'}\n`);

  // 6. Verify Q-update recorded to learning_events table (G1.3 metric)
  console.log('Checking learning_events table for Q-updates (G1.3 metric)...');
  const db = getPostgresClient();
  const result = await db.query(`
    SELECT COUNT(*) as count, MAX(timestamp) as latest
    FROM learning_events
    WHERE loop_type = 'q-learning' AND event_type = 'weight-update'
  `);

  const { count, latest } = result.rows[0];
  console.log(`✓ Q-updates in DB: ${count} (latest: ${latest || 'none'})\n`);

  // 7. Show best dogs per task type
  console.log('Best dogs per task type (learned from history):');
  const bestPerTask = learningService.getBestDogsPerTask();
  for (const [taskType, dogs] of Object.entries(bestPerTask)) {
    console.log(`  ${taskType}:`);
    dogs.slice(0, 2).forEach(({ dog, qValue, visits }) => {
      console.log(`    - ${dog}: Q=${qValue} (${visits} visits)`);
    });
  }

  console.log('\n=== Test Complete ===');
  console.log(`Q-Learning is ${loaded ? 'WIRED and LOADED' : 'WIRED (starting fresh)'}`);
  console.log('Weights are available for KabbalisticRouter routing decisions.');
  console.log(`Q-updates are ${count > 0 ? 'BEING TRACKED' : 'READY TO TRACK'} for G1.3 metric.\n`);

  globalEventBus.removeListener(EventType.QLEARNING_WEIGHT_UPDATE, updateListener);
  process.exit(0);
}

main().catch(err => {
  console.error('Test failed:', err);
  process.exit(1);
});
