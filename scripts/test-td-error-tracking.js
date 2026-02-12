#!/usr/bin/env node
/**
 * Test TD Error tracking (LV-1)
 *
 * Simulates Q-Learning updates and validates:
 * 1. TD-Error calculation
 * 2. Rolling average computation
 * 3. Convergence detection
 * 4. Drift detection
 */

import { QLearningService } from '../packages/node/src/orchestration/learning-service.js';

console.log('Testing TD Error Tracking (LV-1)...\n');

// Create service with test config
const service = new QLearningService({
  serviceId: 'td-test',
  tdWindowSize: 10,
  tdConvergenceThreshold: 0.1,
  tdDriftThreshold: 0.3,
  tdMinUpdatesForConvergence: 5,
  config: {
    learningRate: 0.618,
    discountFactor: 0.382,
  },
});

// Simulate converging Q-updates (TD-Error decreasing)
console.log('=== Phase 1: Convergence (TD-Error decreasing) ===');
service.startEpisode({ content: 'test task', taskType: 'analysis' });
service.recordAction('ANALYST', 'test');

// Simulate decreasing TD-Errors (learning converging)
const convergingUpdates = [
  { currentQ: 0, target: 1.0, reward: 1.0 },      // TD-Error = 1.0
  { currentQ: 0.5, target: 0.9, reward: 0.8 },    // TD-Error = 0.4
  { currentQ: 0.7, target: 0.85, reward: 0.8 },   // TD-Error = 0.15
  { currentQ: 0.75, target: 0.82, reward: 0.8 },  // TD-Error = 0.07
  { currentQ: 0.78, target: 0.81, reward: 0.8 },  // TD-Error = 0.03
  { currentQ: 0.79, target: 0.80, reward: 0.8 },  // TD-Error = 0.01
];

for (const update of convergingUpdates) {
  const tdError = Math.abs(update.target - update.currentQ);
  service._trackTDError(tdError, {
    state: 'test-state',
    action: 'ANALYST',
    ...update,
    newQ: update.currentQ + 0.618 * (update.target - update.currentQ),
  });
  service.stats.updates++;
  
  const status = service.getTDConvergenceStatus();
  console.log(`Update ${service.stats.updates}: TD-Error=${tdError.toFixed(4)}, RollingAvg=${status.rollingAvgTDError?.toFixed(4) || 'N/A'}, Converged=${status.isConverged}`);
}

let status = service.getTDConvergenceStatus();
console.log(`\nConvergence Status: ${status.isConverged ? 'CONVERGED ✓' : 'NOT CONVERGED ✗'}`);
console.log(`Rolling Avg TD-Error: ${status.rollingAvgTDError?.toFixed(4)}`);
console.log(`Threshold: ${status.convergenceThreshold}`);

// Simulate drift (sudden TD-Error spike after convergence)
console.log('\n=== Phase 2: Drift Detection (TD-Error spike) ===');

const driftUpdates = [
  { currentQ: 0.80, target: 0.81, reward: 0.8 },  // TD-Error = 0.01 (stable)
  { currentQ: 0.80, target: 0.81, reward: 0.8 },  // TD-Error = 0.01 (stable)
  { currentQ: 0.30, target: 0.85, reward: 0.9 },  // TD-Error = 0.55 (DRIFT!)
  { currentQ: 0.25, target: 0.90, reward: 1.0 },  // TD-Error = 0.65 (DRIFT!)
];

for (const update of driftUpdates) {
  const tdError = Math.abs(update.target - update.currentQ);
  service._trackTDError(tdError, {
    state: 'test-state',
    action: 'ANALYST',
    ...update,
    newQ: update.currentQ + 0.618 * (update.target - update.currentQ),
  });
  service.stats.updates++;
  
  const status = service.getTDConvergenceStatus();
  console.log(`Update ${service.stats.updates}: TD-Error=${tdError.toFixed(4)}, RollingAvg=${status.rollingAvgTDError?.toFixed(4)}, Converged=${status.isConverged}, Drift=${status.lastDriftAt ? 'DETECTED' : 'None'}`);
}

status = service.getTDConvergenceStatus();
console.log(`\nFinal Status:`);
console.log(`  Converged: ${status.isConverged}`);
console.log(`  Rolling Avg TD-Error: ${status.rollingAvgTDError?.toFixed(4)}`);
console.log(`  Drift Detected: ${status.lastDriftAt ? 'YES at ' + status.lastDriftAt.toISOString() : 'NO'}`);
console.log(`  Total Updates: ${status.totalUpdates}`);
console.log(`  Recent TD-Errors: [${status.recentTDErrors.map(e => e.toFixed(4)).join(', ')}]`);

console.log('\n✓ TD Error Tracking test complete');
