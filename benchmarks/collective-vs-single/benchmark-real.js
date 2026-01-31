#!/usr/bin/env node
/**
 * Phase 1 Benchmark: REAL Collective vs Single
 *
 * Compares:
 * - Single: Direct Anthropic API call with simple prompt
 * - Collective: Actual CYNIC orchestration (orchestrateFull)
 *
 * Objective metrics only:
 * - Response time (ms)
 * - Token usage (estimated from output length for collective)
 * - Consistency (theme variance across runs)
 */

import Anthropic from '@anthropic-ai/sdk';
import { readFileSync, writeFileSync } from 'fs';
import { basename } from 'path';
import { performance } from 'perf_hooks';

// Import CYNIC orchestration
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const { orchestrateFull } = require('../../scripts/lib/cynic-core.cjs');

// Config
const RUNS_PER_CONDITION = 3;
const MODEL = 'claude-sonnet-4-20250514';
const MAX_TOKENS = 2000;

const client = new Anthropic();

// =============================================================================
// SINGLE: Direct API call
// =============================================================================

async function runSingle(code, filename) {
  const start = performance.now();

  const response = await client.messages.create({
    model: MODEL,
    max_tokens: MAX_TOKENS,
    messages: [{
      role: 'user',
      content: `Review this code for bugs, security issues, and quality problems. Be specific and concise.

File: ${filename}
\`\`\`
${code}
\`\`\``
    }]
  });

  const elapsed = performance.now() - start;

  return {
    condition: 'single',
    time_ms: Math.round(elapsed),
    input_tokens: response.usage.input_tokens,
    output_tokens: response.usage.output_tokens,
    total_tokens: response.usage.input_tokens + response.usage.output_tokens,
    output: response.content[0].text,
    stop_reason: response.stop_reason
  };
}

// =============================================================================
// COLLECTIVE: Real CYNIC orchestration
// =============================================================================

async function runCollective(code, filename) {
  const start = performance.now();

  try {
    const result = await orchestrateFull({
      content: code,
      eventType: 'code_review',
      requestJudgment: true,
      metadata: {
        filename,
        source: 'benchmark',
        benchmark: true
      }
    });

    const elapsed = performance.now() - start;

    // Extract output from orchestration result
    const output = result?.judgment?.reasoning?.join('\n') ||
                   result?.reasoning?.join('\n') ||
                   JSON.stringify(result, null, 2);

    // Estimate tokens from output (rough: 1 token ≈ 4 chars)
    const estimatedOutputTokens = Math.round(output.length / 4);
    const estimatedInputTokens = Math.round(code.length / 4) + 500; // +500 for system prompts

    return {
      condition: 'collective',
      time_ms: Math.round(elapsed),
      input_tokens: estimatedInputTokens,
      output_tokens: estimatedOutputTokens,
      total_tokens: estimatedInputTokens + estimatedOutputTokens,
      output,
      qScore: result?.judgment?.qScore || result?.qScore,
      verdict: result?.judgment?.verdict || result?.verdict,
      outcome: result?.outcome,
      dogs_involved: result?.dogs || result?.agents || []
    };
  } catch (error) {
    const elapsed = performance.now() - start;
    return {
      condition: 'collective',
      time_ms: Math.round(elapsed),
      error: error.message,
      output: `Error: ${error.message}`
    };
  }
}

// =============================================================================
// CONSISTENCY METRIC
// =============================================================================

function calculateConsistency(outputs) {
  const themes = outputs.map(o => {
    const lower = (o || '').toLowerCase();
    return {
      security: /security|vulnerab|inject|xss|sql|auth/i.test(lower),
      performance: /perform|slow|optim|cache|memory|leak/i.test(lower),
      logic: /logic|bug|error|null|undefined|edge|race/i.test(lower),
      quality: /clean|maintain|read|style|naming|dead|duplicate/i.test(lower),
    };
  });

  let agreements = 0;
  let comparisons = 0;

  for (let i = 0; i < themes.length; i++) {
    for (let j = i + 1; j < themes.length; j++) {
      for (const key of Object.keys(themes[i])) {
        if (themes[i][key] === themes[j][key]) agreements++;
        comparisons++;
      }
    }
  }

  return comparisons > 0 ? (agreements / comparisons * 100).toFixed(1) : '0';
}

// =============================================================================
// MAIN
// =============================================================================

async function benchmarkFile(filepath) {
  const code = readFileSync(filepath, 'utf8');
  const filename = basename(filepath);

  console.log(`\n  📄 ${filename} (${code.split('\n').length} lines)`);

  const singleResults = [];
  const collectiveResults = [];

  // Run Single
  process.stdout.write('     Single:     ');
  for (let i = 0; i < RUNS_PER_CONDITION; i++) {
    try {
      const result = await runSingle(code, filename);
      singleResults.push(result);
      process.stdout.write(`${result.time_ms}ms `);
    } catch (e) {
      process.stdout.write(`ERR `);
      singleResults.push({ error: e.message, time_ms: 0, output: '' });
    }
  }
  console.log();

  // Run Collective
  process.stdout.write('     Collective: ');
  for (let i = 0; i < RUNS_PER_CONDITION; i++) {
    const result = await runCollective(code, filename);
    collectiveResults.push(result);
    if (result.error) {
      process.stdout.write(`ERR `);
    } else {
      process.stdout.write(`${result.time_ms}ms `);
    }
  }
  console.log();

  // Filter successful runs
  const validSingle = singleResults.filter(r => !r.error);
  const validCollective = collectiveResults.filter(r => !r.error);

  // Aggregate
  const aggregate = (results) => {
    if (results.length === 0) return { time_ms: 0, total_tokens: 0, consistency: '0' };
    return {
      time_ms: Math.round(results.reduce((s, r) => s + r.time_ms, 0) / results.length),
      total_tokens: Math.round(results.reduce((s, r) => s + (r.total_tokens || 0), 0) / results.length),
      consistency: calculateConsistency(results.map(r => r.output)),
      runs: results.length
    };
  };

  return {
    file: filename,
    code_lines: code.split('\n').length,
    single: aggregate(validSingle),
    collective: aggregate(validCollective),
    raw: { singleResults, collectiveResults }
  };
}

async function main() {
  console.log('');
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('  🧪 PHASE 1: REAL Collective vs Single');
  console.log('═══════════════════════════════════════════════════════════════');
  console.log(`  Single:     Direct Anthropic API (${MODEL})`);
  console.log(`  Collective: CYNIC orchestrateFull (real infrastructure)`);
  console.log(`  Runs:       ${RUNS_PER_CONDITION} per condition`);
  console.log('───────────────────────────────────────────────────────────────');

  // Real files from the codebase
  const testFiles = [
    'packages/core/src/judgment/aggregator.js',
    'packages/persistence/src/postgres/client.js',
    'scripts/hooks/guard.js',
    'packages/mcp/src/tools/domains/judgment.js',
    'packages/node/src/services/automation-executor.js',
  ];

  const results = [];

  for (const file of testFiles) {
    try {
      const result = await benchmarkFile(file);
      results.push(result);
    } catch (e) {
      console.log(`  ❌ Error on ${file}: ${e.message}`);
    }
  }

  // Summary
  console.log('');
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('  📊 RESULTS');
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('');
  console.log('  File                          │ Single    │ Collective │   Δ');
  console.log('  ──────────────────────────────┼───────────┼────────────┼────────');

  let totalSingleTime = 0;
  let totalCollectiveTime = 0;
  let countSingle = 0;
  let countCollective = 0;

  for (const r of results) {
    const name = r.file.padEnd(30).slice(0, 30);
    const sTime = r.single.runs > 0 ? `${r.single.time_ms}ms` : 'ERR';
    const cTime = r.collective.runs > 0 ? `${r.collective.time_ms}ms` : 'ERR';

    let delta = '';
    if (r.single.runs > 0 && r.collective.runs > 0) {
      const d = r.collective.time_ms - r.single.time_ms;
      const pct = ((r.collective.time_ms / r.single.time_ms - 1) * 100).toFixed(0);
      delta = `${d > 0 ? '+' : ''}${d}ms (${d > 0 ? '+' : ''}${pct}%)`;
    }

    console.log(`  ${name} │ ${sTime.padStart(7)}   │ ${cTime.padStart(7)}    │ ${delta}`);

    if (r.single.runs > 0) { totalSingleTime += r.single.time_ms; countSingle++; }
    if (r.collective.runs > 0) { totalCollectiveTime += r.collective.time_ms; countCollective++; }
  }

  console.log('  ──────────────────────────────┴───────────┴────────────┴────────');

  if (countSingle > 0 && countCollective > 0) {
    const avgSingle = Math.round(totalSingleTime / countSingle);
    const avgCollective = Math.round(totalCollectiveTime / countCollective);
    const overhead = ((avgCollective / avgSingle - 1) * 100).toFixed(0);

    console.log('');
    console.log('  ┌─────────────────────────────────────────────────────────────┐');
    console.log(`  │  Average Single:     ${avgSingle.toString().padStart(6)}ms                           │`);
    console.log(`  │  Average Collective: ${avgCollective.toString().padStart(6)}ms                           │`);
    console.log(`  │  Overhead:           ${overhead.padStart(6)}%                            │`);
    console.log('  └─────────────────────────────────────────────────────────────┘');

    console.log('');
    console.log('  CONSISTENCY (theme agreement across runs):');
    const avgSingleCons = results.filter(r => r.single.runs > 0)
      .reduce((s, r) => s + parseFloat(r.single.consistency), 0) / countSingle;
    const avgCollCons = results.filter(r => r.collective.runs > 0)
      .reduce((s, r) => s + parseFloat(r.collective.consistency), 0) / countCollective;
    console.log(`    Single:     ${avgSingleCons.toFixed(1)}%`);
    console.log(`    Collective: ${avgCollCons.toFixed(1)}%`);
  }

  // Save results
  const outputPath = 'benchmarks/collective-vs-single/results/phase1-real-results.json';
  writeFileSync(outputPath, JSON.stringify({ timestamp: new Date().toISOString(), results }, null, 2));
  console.log(`\n  📁 Results saved to: ${outputPath}`);

  console.log('');
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('');
}

main().catch(e => {
  console.error('Fatal error:', e);
  process.exit(1);
});
