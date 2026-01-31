#!/usr/bin/env node
/**
 * Collective Dogs Runner
 *
 * Runs CYNIC's 11-dog collective for each code sample.
 * Uses DogOrchestrator with φ-consensus (0.618 threshold).
 *
 * IMPORTANT: Must pass collectivePack with real agents, otherwise
 * DogOrchestrator falls back to mock random scores!
 */

import { readFileSync, writeFileSync, mkdirSync } from 'fs';
import { DogOrchestrator, SharedMemory, createCollectivePack } from '@cynic/node';

// Load samples
const dataset = JSON.parse(
  readFileSync('benchmarks/collective-vs-single/dataset/samples.json', 'utf8')
);

// Create shared memory and collective pack with REAL agents
const sharedMemory = new SharedMemory();
const collectivePack = createCollectivePack({
  sharedMemory,
  enableAutonomous: false, // Disable for benchmark
});

// Create orchestrator WITH collective pack (not mock!)
const orchestrator = new DogOrchestrator({
  sharedMemory,
  collectivePack,  // ← This enables real agent analysis
  mode: 'parallel',
  consensusThreshold: 0.618,
});

async function runCollective(sample) {
  const start = performance.now();

  try {
    const judgment = await orchestrator.judge({
      content: sample.code,
      type: 'code',
      context: {
        language: sample.language,
        category: sample.category,
        task: 'security_review'
      }
    });

    const elapsed = performance.now() - start;

    // Extract issues from dog votes
    const issues = [];
    const seenIssues = new Set();

    if (judgment.votes) {
      for (const vote of judgment.votes) {
        if (vote.issues) {
          for (const issue of vote.issues) {
            const key = `${issue.type}:${issue.line || 0}`;
            if (!seenIssues.has(key)) {
              seenIssues.add(key);
              issues.push({
                type: issue.type,
                line: issue.line || null,
                description: issue.description || issue.message,
                severity: issue.severity || 'medium',
                detectedBy: vote.dog
              });
            }
          }
        }
      }
    }

    return {
      sampleId: sample.id,
      system: 'collective',
      latencyMs: Math.round(elapsed),
      result: {
        issues,
        verdict: judgment.verdict,
        score: judgment.global_score || judgment.score || 50,
        consensus: judgment.consensus,
        votes: judgment.votes?.map(v => ({
          dog: v.dog,
          verdict: v.verdict,
          score: v.score,
          issueCount: v.issues?.length || 0
        })) || []
      },
      success: true
    };

  } catch (error) {
    return {
      sampleId: sample.id,
      system: 'collective',
      latencyMs: Math.round(performance.now() - start),
      error: error.message,
      success: false
    };
  }
}

async function main() {
  console.log('');
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('  CYNIC COLLECTIVE (11 Dogs, φ-Consensus)');
  console.log(`  Samples: ${dataset.samples.length}`);
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('');

  const results = [];
  let totalLatency = 0;

  for (let i = 0; i < dataset.samples.length; i++) {
    const sample = dataset.samples[i];
    process.stdout.write(`  [${i + 1}/${dataset.samples.length}] ${sample.id}... `);

    const result = await runCollective(sample);
    results.push(result);

    if (result.success) {
      totalLatency += result.latencyMs;
      const consensus = result.result.consensus?.ratio?.toFixed(2) || 'N/A';
      const dogs = result.result.votes?.length || 0;
      console.log(`${result.latencyMs}ms | ${result.result.verdict} | Score: ${result.result.score} | Consensus: ${consensus} | Dogs: ${dogs}`);
    } else {
      console.log(`ERROR: ${result.error}`);
    }
  }

  // Summary
  const successful = results.filter(r => r.success);
  console.log('');
  console.log('───────────────────────────────────────────────────────────────');
  console.log('  SUMMARY');
  console.log('───────────────────────────────────────────────────────────────');
  console.log(`  Total samples:     ${dataset.samples.length}`);
  console.log(`  Successful:        ${successful.length}`);
  console.log(`  Failed:            ${results.filter(r => !r.success).length}`);
  console.log(`  Total latency:     ${totalLatency}ms`);
  console.log(`  Avg latency:       ${Math.round(totalLatency / successful.length)}ms`);

  // Dog participation stats
  const dogStats = {};
  for (const r of successful) {
    for (const vote of r.result.votes || []) {
      dogStats[vote.dog] = (dogStats[vote.dog] || 0) + 1;
    }
  }
  console.log('');
  console.log('  Dog participation:');
  for (const [dog, count] of Object.entries(dogStats).sort((a, b) => b[1] - a[1])) {
    console.log(`    ${dog}: ${count}/${successful.length} samples`);
  }
  console.log('');

  // Save results
  mkdirSync('benchmarks/collective-vs-single/results', { recursive: true });
  const outputPath = `benchmarks/collective-vs-single/results/collective-${Date.now()}.json`;
  writeFileSync(outputPath, JSON.stringify({
    system: 'collective',
    config: {
      mode: 'parallel',
      consensusThreshold: 0.618,
      dogs: 11
    },
    timestamp: new Date().toISOString(),
    summary: {
      totalSamples: dataset.samples.length,
      successful: successful.length,
      failed: results.filter(r => !r.success).length,
      totalLatencyMs: totalLatency,
      avgLatencyMs: Math.round(totalLatency / successful.length),
      dogStats
    },
    results
  }, null, 2));

  console.log(`  Results saved to: ${outputPath}`);
  console.log('');
}

main().catch(console.error);
