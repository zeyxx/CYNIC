#!/usr/bin/env node
/**
 * Debug test for orchestrator fix
 */

import { DogOrchestrator, SharedMemory } from '@cynic/node';
import { EngineOrchestrator, globalEngineRegistry } from '@cynic/core/engines';
import { createUnifiedOrchestrator } from '@cynic/node/orchestration/unified-orchestrator.js';

console.log('');
console.log('═══════════════════════════════════════════════════════════════');
console.log('  Debug: Orchestrator Components');
console.log('═══════════════════════════════════════════════════════════════');
console.log('');

// Create orchestrators
const sharedMemory = new SharedMemory();
const dogOrchestrator = new DogOrchestrator({
  sharedMemory,
  mode: 'parallel',
  consensusThreshold: 0.618,
});
const engineOrchestrator = new EngineOrchestrator(globalEngineRegistry, {
  defaultStrategy: 'weighted-average',
  timeout: 5000,
});

// Check what methods exist
console.log('DogOrchestrator methods:');
console.log('  - judge:', typeof dogOrchestrator.judge);
console.log('  - getStats:', typeof dogOrchestrator.getStats);
console.log('');

console.log('EngineOrchestrator methods:');
console.log('  - consult:', typeof engineOrchestrator.consult);
console.log('');

// Test dogOrchestrator.judge directly
console.log('Testing dogOrchestrator.judge() directly...');
const testCode = `function bad() { eval(x); }`;

try {
  const judgeResult = await dogOrchestrator.judge({ content: testCode, type: 'code' });
  console.log('Judge result:', JSON.stringify(judgeResult, null, 2));
} catch (e) {
  console.error('Judge error:', e.message);
}

console.log('');

// Test engineOrchestrator.consult directly
console.log('Testing engineOrchestrator.consult() directly...');
try {
  const consultResult = await engineOrchestrator.consult('Is this code safe?', {
    domains: ['ethics', 'logic'],
  });
  console.log('Consult result:', JSON.stringify(consultResult, null, 2).slice(0, 500));
} catch (e) {
  console.error('Consult error:', e.message);
}

console.log('');

// Create unified and check internal state
const unified = createUnifiedOrchestrator({
  dogOrchestrator,
  engineOrchestrator,
  persistence: null,
});

console.log('UnifiedOrchestrator internal state:');
console.log('  - dogOrchestrator set:', !!unified.dogOrchestrator);
console.log('  - engineOrchestrator set:', !!unified.engineOrchestrator);
console.log('');
