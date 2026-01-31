#!/usr/bin/env node
/**
 * Test that the orchestrator fix works
 * Direct test without HTTP server
 */

import { DogOrchestrator, SharedMemory } from '@cynic/node';
import { EngineOrchestrator, globalEngineRegistry } from '@cynic/core/engines';
import { createUnifiedOrchestrator } from '@cynic/node/orchestration/unified-orchestrator.js';

console.log('');
console.log('═══════════════════════════════════════════════════════════════');
console.log('  Testing Orchestrator Fix');
console.log('═══════════════════════════════════════════════════════════════');
console.log('');

// Create orchestrators directly (same as server.js now does)
const sharedMemory = new SharedMemory();
console.log('✅ SharedMemory created');

const dogOrchestrator = new DogOrchestrator({
  sharedMemory,
  mode: 'parallel',
  consensusThreshold: 0.618,
});
console.log('✅ DogOrchestrator created');

const engineOrchestrator = new EngineOrchestrator(globalEngineRegistry, {
  defaultStrategy: 'weighted-average',
  timeout: 5000,
});
console.log('✅ EngineOrchestrator created');

// Create unified orchestrator with both
const unifiedOrchestrator = createUnifiedOrchestrator({
  dogOrchestrator,
  engineOrchestrator,
  persistence: null,
});
console.log('✅ UnifiedOrchestrator created');

// Test code review
const testCode = `
function validateUser(user) {
  if (user.password == '123456') {
    return true;
  }
  return user.isAdmin || false;
}
`;

console.log('');
console.log('Testing code review...');
console.log('');

try {
  const result = await unifiedOrchestrator.process({
    eventType: 'code_review',
    content: testCode,
    source: 'test',
    userContext: { userId: 'test-user' },
    requestJudgment: true,
    requestSynthesis: true,
  });

  console.log('Result:');
  console.log('  outcome:', result.outcome);
  console.log('  routing:', result.routing?.domain, '→', result.routing?.sefirah);
  console.log('  judgment:', result.judgment ? '✅ Present' : '❌ NULL');
  if (result.judgment) {
    console.log('    score:', result.judgment.score);
    console.log('    verdict:', result.judgment.verdict);
    console.log('    consensus:', result.judgment.consensus);
  }
  console.log('  synthesis:', result.synthesis ? '✅ Present' : '❌ NULL');
  console.log('  trace steps:', result.trace?.length || 0);
  console.log('');

  if (result.judgment) {
    console.log('═══════════════════════════════════════════════════════════════');
    console.log('  ✅ FIX VERIFIED - Judgment is now populated!');
    console.log('═══════════════════════════════════════════════════════════════');
  } else {
    console.log('═══════════════════════════════════════════════════════════════');
    console.log('  ❌ FIX NOT WORKING - Judgment still null');
    console.log('═══════════════════════════════════════════════════════════════');
  }
} catch (e) {
  console.error('Error:', e.message);
  console.error(e.stack);
}

console.log('');
