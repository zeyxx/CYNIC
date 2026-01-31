#!/usr/bin/env node
/**
 * Direct Benchmark: Collective Dogs vs Single (No HTTP)
 *
 * Tests the orchestrator directly without needing MCP server
 */

import { readFileSync } from 'fs';
import { DogOrchestrator, SharedMemory } from '@cynic/node';
import { EngineOrchestrator, globalEngineRegistry } from '@cynic/core/engines';
import { createUnifiedOrchestrator } from '@cynic/node/orchestration/unified-orchestrator.js';

// Sample codes from the benchmark
const samples = [
  {
    name: 'sample1.js',
    code: readFileSync('benchmarks/collective-vs-single/code-samples/sample1.js', 'utf8'),
  },
  {
    name: 'sample2.js',
    code: readFileSync('benchmarks/collective-vs-single/code-samples/sample2.js', 'utf8'),
  },
  {
    name: 'sample3.js',
    code: readFileSync('benchmarks/collective-vs-single/code-samples/sample3.js', 'utf8'),
  },
];

console.log('');
console.log('═══════════════════════════════════════════════════════════════');
console.log('  CYNIC Collective Benchmark (Direct - No HTTP)');
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
const unified = createUnifiedOrchestrator({
  dogOrchestrator,
  engineOrchestrator,
  persistence: null,
});

console.log('Orchestrators initialized:');
console.log('  - DogOrchestrator (11 dogs, φ-consensus)');
console.log('  - EngineOrchestrator (73 engines)');
console.log('  - UnifiedOrchestrator');
console.log('');

for (const sample of samples) {
  console.log('───────────────────────────────────────────────────────────────');
  console.log(`  ${sample.name} (${sample.code.split('\n').length} lines)`);
  console.log('───────────────────────────────────────────────────────────────');

  const start = performance.now();

  try {
    const result = await unified.process({
      eventType: 'code_review',
      content: sample.code,
      source: 'benchmark',
      userContext: { userId: 'benchmark-user' },
      requestJudgment: true,
      requestSynthesis: false,
    });

    const elapsed = Math.round(performance.now() - start);

    console.log('');
    console.log('  Result:');
    console.log(`    Time:      ${elapsed}ms`);
    console.log(`    Outcome:   ${result.outcome}`);
    console.log(`    Routing:   ${result.routing?.domain || 'general'} → ${result.routing?.sefirah || 'Keter'}`);

    if (result.judgment) {
      console.log(`    Judgment:  ✅`);
      console.log(`      Score:     ${result.judgment.global_score?.toFixed(1) || result.judgment.score || 'N/A'}`);
      console.log(`      Verdict:   ${result.judgment.verdict}`);
      console.log(`      Consensus: ${result.judgment.consensus?.ratio?.toFixed(2) || 'N/A'} (threshold: ${result.judgment.consensus?.threshold || 0.618})`);
      console.log(`      Dogs:      ${result.judgment.consensus?.votingDogs || result.judgment.votes?.length || 0}/11`);

      // Show top votes
      if (result.judgment.votes && result.judgment.votes.length > 0) {
        console.log('      Top votes:');
        const sorted = [...result.judgment.votes].sort((a, b) => b.score - a.score);
        for (const vote of sorted.slice(0, 3)) {
          console.log(`        - ${vote.dog}: ${vote.score?.toFixed(1)} (${vote.verdict})`);
        }
      }

      // Show insights if any
      if (result.judgment.insights && result.judgment.insights.length > 0) {
        console.log('      Insights:');
        for (const insight of result.judgment.insights.slice(0, 3)) {
          console.log(`        - ${insight}`);
        }
      }
    } else {
      console.log('    Judgment:  ❌ null');
    }

    console.log('');

  } catch (e) {
    console.error(`  Error: ${e.message}`);
    console.log('');
  }
}

console.log('═══════════════════════════════════════════════════════════════');
console.log('  Benchmark Complete');
console.log('═══════════════════════════════════════════════════════════════');
console.log('');
