#!/usr/bin/env node
/**
 * Phase 1 Benchmark: Collective vs Single
 *
 * Objective metrics only:
 * - Response time (ms)
 * - Token usage (input/output)
 * - Consistency (variance across runs)
 *
 * No quality judgment - that's Phase 2.
 */

import Anthropic from '@anthropic-ai/sdk';
import { readFileSync, writeFileSync, readdirSync } from 'fs';
import { join, basename } from 'path';
import { performance } from 'perf_hooks';

// Config
const RUNS_PER_CONDITION = 3;
const MODEL = 'claude-sonnet-4-20250514';
const MAX_TOKENS = 2000;

const client = new Anthropic();

// =============================================================================
// PROMPTS
// =============================================================================

const SINGLE_PROMPT = `Review this code for bugs, security issues, and improvements. Be concise.`;

const COLLECTIVE_PROMPT = `You are CYNIC, a collective of specialized reviewers:
- Guardian: Security vulnerabilities
- Analyst: Logic errors, edge cases
- Sage: Best practices, patterns
- Janitor: Code quality, maintainability

Each reviewer examines the code from their perspective. Synthesize findings.
Be concise.`;

// =============================================================================
// BENCHMARK FUNCTIONS
// =============================================================================

async function runSingle(code, filename) {
  const start = performance.now();

  const response = await client.messages.create({
    model: MODEL,
    max_tokens: MAX_TOKENS,
    messages: [{
      role: 'user',
      content: `${SINGLE_PROMPT}\n\nFile: ${filename}\n\`\`\`\n${code}\n\`\`\``
    }]
  });

  const elapsed = performance.now() - start;

  return {
    condition: 'single',
    time_ms: Math.round(elapsed),
    input_tokens: response.usage.input_tokens,
    output_tokens: response.usage.output_tokens,
    output: response.content[0].text,
    stop_reason: response.stop_reason
  };
}

async function runCollective(code, filename) {
  const start = performance.now();

  const response = await client.messages.create({
    model: MODEL,
    max_tokens: MAX_TOKENS,
    system: COLLECTIVE_PROMPT,
    messages: [{
      role: 'user',
      content: `Review this code.\n\nFile: ${filename}\n\`\`\`\n${code}\n\`\`\``
    }]
  });

  const elapsed = performance.now() - start;

  return {
    condition: 'collective',
    time_ms: Math.round(elapsed),
    input_tokens: response.usage.input_tokens,
    output_tokens: response.usage.output_tokens,
    output: response.content[0].text,
    stop_reason: response.stop_reason
  };
}

// =============================================================================
// CONSISTENCY METRIC
// =============================================================================

function calculateConsistency(outputs) {
  // Simple: count unique "themes" mentioned
  // More sophisticated: embedding similarity
  const themes = outputs.map(o => {
    const lower = o.toLowerCase();
    return {
      mentions_security: /security|vulnerab|inject|xss|sql/i.test(lower),
      mentions_performance: /perform|slow|optim|cache|memory/i.test(lower),
      mentions_logic: /logic|bug|error|null|undefined|edge/i.test(lower),
      mentions_quality: /clean|maintain|read|style|naming/i.test(lower),
    };
  });

  // Calculate agreement rate across runs
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

  return comparisons > 0 ? (agreements / comparisons * 100).toFixed(1) : 0;
}

// =============================================================================
// MAIN
// =============================================================================

async function benchmarkFile(filepath) {
  const code = readFileSync(filepath, 'utf8');
  const filename = basename(filepath);

  console.log(`\n  Benchmarking: ${filename}`);

  const singleResults = [];
  const collectiveResults = [];

  // Run Single
  process.stdout.write('    Single:     ');
  for (let i = 0; i < RUNS_PER_CONDITION; i++) {
    const result = await runSingle(code, filename);
    singleResults.push(result);
    process.stdout.write(`${result.time_ms}ms `);
  }
  console.log();

  // Run Collective
  process.stdout.write('    Collective: ');
  for (let i = 0; i < RUNS_PER_CONDITION; i++) {
    const result = await runCollective(code, filename);
    collectiveResults.push(result);
    process.stdout.write(`${result.time_ms}ms `);
  }
  console.log();

  // Aggregate
  const avgSingle = {
    time_ms: Math.round(singleResults.reduce((s, r) => s + r.time_ms, 0) / RUNS_PER_CONDITION),
    input_tokens: Math.round(singleResults.reduce((s, r) => s + r.input_tokens, 0) / RUNS_PER_CONDITION),
    output_tokens: Math.round(singleResults.reduce((s, r) => s + r.output_tokens, 0) / RUNS_PER_CONDITION),
    consistency: calculateConsistency(singleResults.map(r => r.output)),
  };

  const avgCollective = {
    time_ms: Math.round(collectiveResults.reduce((s, r) => s + r.time_ms, 0) / RUNS_PER_CONDITION),
    input_tokens: Math.round(collectiveResults.reduce((s, r) => s + r.input_tokens, 0) / RUNS_PER_CONDITION),
    output_tokens: Math.round(collectiveResults.reduce((s, r) => s + r.output_tokens, 0) / RUNS_PER_CONDITION),
    consistency: calculateConsistency(collectiveResults.map(r => r.output)),
  };

  return {
    file: filename,
    code_lines: code.split('\n').length,
    single: avgSingle,
    collective: avgCollective,
    raw: { singleResults, collectiveResults }
  };
}

async function main() {
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('  PHASE 1: Collective vs Single - Objective Metrics');
  console.log('═══════════════════════════════════════════════════════════════');
  console.log(`  Model: ${MODEL}`);
  console.log(`  Runs per condition: ${RUNS_PER_CONDITION}`);

  // Use real files from the codebase
  const testFiles = [
    'packages/core/src/judgment/aggregator.js',
    'packages/node/src/services/automation-executor.js',
    'packages/mcp/src/tools/domains/judgment.js',
    'packages/persistence/src/postgres/client.js',
    'scripts/hooks/guard.js',
  ];

  const results = [];

  for (const file of testFiles) {
    try {
      const result = await benchmarkFile(file);
      results.push(result);
    } catch (e) {
      console.log(`  Error on ${file}: ${e.message}`);
    }
  }

  // Summary
  console.log('\n═══════════════════════════════════════════════════════════════');
  console.log('  RESULTS SUMMARY');
  console.log('═══════════════════════════════════════════════════════════════\n');

  console.log('  File                          │ Single      │ Collective  │ Δ Time');
  console.log('  ──────────────────────────────┼─────────────┼─────────────┼────────');

  let totalSingleTime = 0;
  let totalCollectiveTime = 0;
  let totalSingleTokens = 0;
  let totalCollectiveTokens = 0;

  for (const r of results) {
    const name = r.file.padEnd(30).slice(0, 30);
    const sTime = `${r.single.time_ms}ms`.padStart(7);
    const cTime = `${r.collective.time_ms}ms`.padStart(7);
    const delta = r.collective.time_ms - r.single.time_ms;
    const deltaStr = `${delta > 0 ? '+' : ''}${delta}ms`.padStart(7);

    console.log(`  ${name} │ ${sTime}     │ ${cTime}     │ ${deltaStr}`);

    totalSingleTime += r.single.time_ms;
    totalCollectiveTime += r.collective.time_ms;
    totalSingleTokens += r.single.input_tokens + r.single.output_tokens;
    totalCollectiveTokens += r.collective.input_tokens + r.collective.output_tokens;
  }

  console.log('  ──────────────────────────────┼─────────────┼─────────────┼────────');
  console.log(`  ${'AVERAGE'.padEnd(30)} │ ${Math.round(totalSingleTime/results.length).toString().padStart(5)}ms   │ ${Math.round(totalCollectiveTime/results.length).toString().padStart(5)}ms   │`);

  console.log('\n  TOKENS:');
  console.log(`    Single avg:     ${Math.round(totalSingleTokens/results.length)} tokens/file`);
  console.log(`    Collective avg: ${Math.round(totalCollectiveTokens/results.length)} tokens/file`);
  console.log(`    Overhead:       ${((totalCollectiveTokens/totalSingleTokens - 1) * 100).toFixed(1)}%`);

  console.log('\n  CONSISTENCY (theme agreement across runs):');
  const avgSingleConsistency = results.reduce((s, r) => s + parseFloat(r.single.consistency), 0) / results.length;
  const avgCollectiveConsistency = results.reduce((s, r) => s + parseFloat(r.collective.consistency), 0) / results.length;
  console.log(`    Single:     ${avgSingleConsistency.toFixed(1)}%`);
  console.log(`    Collective: ${avgCollectiveConsistency.toFixed(1)}%`);

  // Save results
  const outputPath = 'benchmarks/collective-vs-single/results/phase1-results.json';
  writeFileSync(outputPath, JSON.stringify(results, null, 2));
  console.log(`\n  Results saved to: ${outputPath}`);

  console.log('\n═══════════════════════════════════════════════════════════════\n');
}

main().catch(console.error);
