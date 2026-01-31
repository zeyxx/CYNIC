#!/usr/bin/env node
/**
 * Phase 1 Benchmark: REAL Collective vs Single
 *
 * Single: Direct Anthropic API call via fetch
 * Collective: CYNIC orchestrateFull
 */

'use strict';

const { readFileSync, writeFileSync } = require('fs');
const { basename } = require('path');
const { performance } = require('perf_hooks');

// CYNIC
const { orchestrateFull } = require('../../scripts/lib/cynic-core.cjs');

// Config
const RUNS_PER_CONDITION = 3;
const MODEL = 'claude-sonnet-4-20250514';
const MAX_TOKENS = 2000;
const API_KEY = process.env.ANTHROPIC_API_KEY;

// =============================================================================
// SINGLE: Direct API call via fetch
// =============================================================================

async function runSingle(code, filename) {
  if (!API_KEY) {
    return { error: 'ANTHROPIC_API_KEY not set', time_ms: 0 };
  }

  const start = performance.now();

  const response = await fetch('https://api.anthropic.com/v1/messages', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'x-api-key': API_KEY,
      'anthropic-version': '2023-06-01'
    },
    body: JSON.stringify({
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
    })
  });

  const elapsed = performance.now() - start;
  const data = await response.json();

  if (data.error) {
    return { error: data.error.message, time_ms: Math.round(elapsed) };
  }

  return {
    condition: 'single',
    time_ms: Math.round(elapsed),
    input_tokens: data.usage?.input_tokens || 0,
    output_tokens: data.usage?.output_tokens || 0,
    total_tokens: (data.usage?.input_tokens || 0) + (data.usage?.output_tokens || 0),
    output: data.content?.[0]?.text || '',
    stop_reason: data.stop_reason
  };
}

// =============================================================================
// COLLECTIVE: Real CYNIC orchestration
// =============================================================================

async function runCollective(code, filename) {
  const start = performance.now();

  try {
    const result = await orchestrateFull(code, {
      eventType: 'code_review',
      requestJudgment: true,
      metadata: { filename, source: 'benchmark' }
    });

    const elapsed = performance.now() - start;

    const output = result?.judgment?.reasoning?.join('\n') ||
                   result?.reasoning?.join('\n') ||
                   JSON.stringify(result, null, 2);

    const estimatedOutputTokens = Math.round(output.length / 4);
    const estimatedInputTokens = Math.round(code.length / 4) + 500;

    return {
      condition: 'collective',
      time_ms: Math.round(elapsed),
      input_tokens: estimatedInputTokens,
      output_tokens: estimatedOutputTokens,
      total_tokens: estimatedInputTokens + estimatedOutputTokens,
      output,
      qScore: result?.judgment?.qScore || result?.qScore,
      verdict: result?.judgment?.verdict || result?.verdict
    };
  } catch (e) {
    return { error: e.message, time_ms: Math.round(performance.now() - start) };
  }
}

// =============================================================================
// CONSISTENCY
// =============================================================================

function calculateConsistency(outputs) {
  const themes = outputs.filter(o => o && !o.error).map(o => {
    const lower = (typeof o === 'string' ? o : o.output || '').toLowerCase();
    return {
      security: /security|vulnerab|inject|xss|sql/i.test(lower),
      performance: /perform|slow|optim|cache|memory/i.test(lower),
      logic: /logic|bug|error|null|undefined|edge/i.test(lower),
      quality: /clean|maintain|read|style|naming/i.test(lower),
    };
  });

  if (themes.length < 2) return '0';

  let agreements = 0, comparisons = 0;
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
  const lines = code.split('\n').length;

  console.log(`\n  ${filename} (${lines} lines)`);

  const singleResults = [];
  const collectiveResults = [];

  // Single
  process.stdout.write('    Single:     ');
  for (let i = 0; i < RUNS_PER_CONDITION; i++) {
    const result = await runSingle(code, filename);
    singleResults.push(result);
    process.stdout.write(result.error ? 'ERR ' : `${result.time_ms}ms `);
  }
  console.log();

  // Collective
  process.stdout.write('    Collective: ');
  for (let i = 0; i < RUNS_PER_CONDITION; i++) {
    const result = await runCollective(code, filename);
    collectiveResults.push(result);
    process.stdout.write(result.error ? 'ERR ' : `${result.time_ms}ms `);
  }
  console.log();

  const valid = arr => arr.filter(r => !r.error);
  const avg = (arr, key) => arr.length ? Math.round(arr.reduce((s, r) => s + (r[key] || 0), 0) / arr.length) : 0;

  return {
    file: filename,
    lines,
    single: {
      time_ms: avg(valid(singleResults), 'time_ms'),
      tokens: avg(valid(singleResults), 'total_tokens'),
      consistency: calculateConsistency(singleResults),
      runs: valid(singleResults).length,
      errors: singleResults.filter(r => r.error).length
    },
    collective: {
      time_ms: avg(valid(collectiveResults), 'time_ms'),
      tokens: avg(valid(collectiveResults), 'total_tokens'),
      consistency: calculateConsistency(collectiveResults),
      runs: valid(collectiveResults).length,
      errors: collectiveResults.filter(r => r.error).length
    },
    raw: { singleResults, collectiveResults }
  };
}

async function main() {
  console.log('');
  console.log('================================================================');
  console.log('  PHASE 1: Collective vs Single - Objective Metrics');
  console.log('================================================================');
  console.log(`  Single:     Direct API (${MODEL})`);
  console.log(`  Collective: CYNIC orchestrateFull`);
  console.log(`  Runs:       ${RUNS_PER_CONDITION}x each`);

  if (!API_KEY) {
    console.log('\n  WARNING: ANTHROPIC_API_KEY not set - Single will fail');
  }

  const testFiles = [
    'packages/core/src/judgment/aggregator.js',
    'packages/persistence/src/postgres/client.js',
    'scripts/hooks/guard.js',
  ];

  const results = [];
  for (const file of testFiles) {
    try {
      results.push(await benchmarkFile(file));
    } catch (e) {
      console.log(`  Error: ${e.message}`);
    }
  }

  // Summary
  console.log('');
  console.log('================================================================');
  console.log('  RESULTS');
  console.log('================================================================');
  console.log('');
  console.log('  File                    | Single      | Collective  | Delta');
  console.log('  ------------------------|-------------|-------------|--------');

  for (const r of results) {
    const name = r.file.slice(0, 24).padEnd(24);
    const s = r.single.runs > 0 ? `${r.single.time_ms}ms` : 'ERR';
    const c = r.collective.runs > 0 ? `${r.collective.time_ms}ms` : 'ERR';
    let delta = '';
    if (r.single.runs && r.collective.runs) {
      const d = r.collective.time_ms - r.single.time_ms;
      delta = `${d > 0 ? '+' : ''}${d}ms`;
    }
    console.log(`  ${name}| ${s.padEnd(11)} | ${c.padEnd(11)} | ${delta}`);
  }

  const validResults = results.filter(r => r.single.runs > 0 && r.collective.runs > 0);
  if (validResults.length > 0) {
    const avgS = Math.round(validResults.reduce((s, r) => s + r.single.time_ms, 0) / validResults.length);
    const avgC = Math.round(validResults.reduce((s, r) => s + r.collective.time_ms, 0) / validResults.length);
    const overhead = ((avgC / avgS - 1) * 100).toFixed(0);

    console.log('  ------------------------|-------------|-------------|--------');
    console.log(`  AVERAGE                 | ${avgS}ms`.padEnd(40) + `| ${avgC}ms`.padEnd(14) + `| ${overhead}%`);
  }

  writeFileSync(
    'benchmarks/collective-vs-single/results/phase1.json',
    JSON.stringify({ timestamp: new Date().toISOString(), results }, null, 2)
  );
  console.log('\n  Saved: benchmarks/collective-vs-single/results/phase1.json');
  console.log('');
}

main().catch(e => console.error('Fatal:', e));
