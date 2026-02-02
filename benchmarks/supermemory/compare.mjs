#!/usr/bin/env node
/**
 * SUPERMEMORY Comparison Tool
 *
 * Compares baseline vs enhanced benchmark results.
 * Shows improvement percentages and validates kill criteria.
 *
 * Usage:
 *   node benchmarks/supermemory/compare.mjs results/baseline.json results/enhanced.json
 *
 * "φ distrusts φ" - Measure the improvement
 *
 * @module benchmarks/supermemory/compare
 */

'use strict';

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

// φ constants
const PHI_INV = 0.618033988749895;

// Parse args
const args = process.argv.slice(2);

if (args.length < 2) {
  console.log('');
  console.log('Usage: node compare.mjs <baseline.json> <enhanced.json>');
  console.log('');
  console.log('Example:');
  console.log('  node benchmarks/supermemory/compare.mjs results/baseline.json results/enhanced.json');
  console.log('');
  process.exit(1);
}

const baselinePath = args[0];
const enhancedPath = args[1];

// Get project root
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const PROJECT_ROOT = path.resolve(__dirname, '../..');

// Load files
const loadJson = (filePath) => {
  const fullPath = path.resolve(PROJECT_ROOT, filePath);
  if (!fs.existsSync(fullPath)) {
    console.error(`File not found: ${fullPath}`);
    process.exit(1);
  }
  return JSON.parse(fs.readFileSync(fullPath, 'utf8'));
};

const baseline = loadJson(baselinePath);
const enhanced = loadJson(enhancedPath);

console.log('');
console.log('═══════════════════════════════════════════════════════════════');
console.log('  SUPERMEMORY COMPARISON');
console.log('  Baseline vs Enhanced');
console.log('═══════════════════════════════════════════════════════════════');
console.log('');
console.log(`  Baseline: ${baselinePath} (${baseline.timestamp})`);
console.log(`  Enhanced: ${enhancedPath} (${enhanced.timestamp})`);
console.log('');

// Helper for delta calculation
const delta = (before, after, isLowerBetter = false) => {
  if (before === null || after === null) return { value: null, percent: null, improved: null };
  const diff = after - before;
  const percent = before !== 0 ? (diff / before) * 100 : (after !== 0 ? 100 : 0);
  const improved = isLowerBetter ? diff < 0 : diff > 0;
  return { value: diff, percent, improved };
};

// Helper for formatting
const formatDelta = (d, unit = '', isLowerBetter = false) => {
  if (d.value === null) return 'N/A';
  const sign = d.value >= 0 ? '+' : '';
  const emoji = d.improved ? '✅' : (d.value === 0 ? '─' : '❌');
  return `${emoji} ${sign}${d.value.toFixed(1)}${unit} (${sign}${d.percent.toFixed(1)}%)`;
};

// =============================================================================
// CODEBASE STATS
// =============================================================================

console.log('── Codebase Stats ─────────────────────────────────────────────');
console.log('');
console.log(`  Total JS files:    ${baseline.codebaseStats.totalJsFiles} → ${enhanced.codebaseStats.totalJsFiles}`);
console.log(`  Total lines:       ${baseline.codebaseStats.totalLines.toLocaleString()} → ${enhanced.codebaseStats.totalLines.toLocaleString()}`);
console.log(`  Packages:          ${baseline.codebaseStats.packages} → ${enhanced.codebaseStats.packages}`);
console.log('');

// =============================================================================
// CODEBASE INDEXER
// =============================================================================

console.log('── CodebaseIndexer Performance ────────────────────────────────');
console.log('');

const coverageDelta = delta(baseline.codebaseIndexer.coveragePercent, enhanced.codebaseIndexer.coveragePercent);
const indexTimeDelta = delta(baseline.codebaseIndexer.indexTimeMs, enhanced.codebaseIndexer.indexTimeMs, true);
const factsDelta = delta(baseline.codebaseIndexer.factsGenerated, enhanced.codebaseIndexer.factsGenerated);
const memoryDelta = delta(baseline.codebaseIndexer.memoryUsedMb, enhanced.codebaseIndexer.memoryUsedMb, true);

console.log(`  Coverage:      ${baseline.codebaseIndexer.coveragePercent}% → ${enhanced.codebaseIndexer.coveragePercent}%`);
console.log(`                 ${formatDelta(coverageDelta, '%')}`);
console.log('');
console.log(`  Index Time:    ${baseline.codebaseIndexer.indexTimeMs || 'N/A'}ms → ${enhanced.codebaseIndexer.indexTimeMs || 'N/A'}ms`);
console.log(`                 ${formatDelta(indexTimeDelta, 'ms', true)}`);
console.log('');
console.log(`  Facts:         ${baseline.codebaseIndexer.factsGenerated} → ${enhanced.codebaseIndexer.factsGenerated}`);
console.log(`                 ${formatDelta(factsDelta)}`);
console.log('');
console.log(`  Memory:        ${baseline.codebaseIndexer.memoryUsedMb || 'N/A'}MB → ${enhanced.codebaseIndexer.memoryUsedMb || 'N/A'}MB`);
console.log(`                 ${formatDelta(memoryDelta, 'MB', true)}`);
console.log('');

// =============================================================================
// FACTS REPOSITORY
// =============================================================================

console.log('── FactsRepository Performance ────────────────────────────────');
console.log('');

const ftsP50Delta = delta(baseline.factsRepository.ftsSearchP50Ms, enhanced.factsRepository.ftsSearchP50Ms, true);
const ftsP95Delta = delta(baseline.factsRepository.ftsSearchP95Ms, enhanced.factsRepository.ftsSearchP95Ms, true);

console.log(`  FTS p50:       ${baseline.factsRepository.ftsSearchP50Ms}ms → ${enhanced.factsRepository.ftsSearchP50Ms}ms`);
console.log(`                 ${formatDelta(ftsP50Delta, 'ms', true)}`);
console.log('');
console.log(`  FTS p95:       ${baseline.factsRepository.ftsSearchP95Ms}ms → ${enhanced.factsRepository.ftsSearchP95Ms}ms`);
console.log(`                 ${formatDelta(ftsP95Delta, 'ms', true)}`);
console.log('');

// =============================================================================
// CARTOGRAPHER
// =============================================================================

console.log('── Cartographer Performance ───────────────────────────────────');
console.log('');

const classifDelta = delta(baseline.cartographer.classificationAccuracy, enhanced.cartographer.classificationAccuracy);
const cycleDelta = delta(baseline.cartographer.cycleDetectionMs, enhanced.cartographer.cycleDetectionMs, true);

console.log(`  Classification: ${baseline.cartographer.classificationAccuracy || 'N/A'}% → ${enhanced.cartographer.classificationAccuracy || 'N/A'}%`);
console.log(`                  ${formatDelta(classifDelta, '%')}`);
console.log('');
console.log(`  Cycle detect:   ${baseline.cartographer.cycleDetectionMs || 'N/A'}ms → ${enhanced.cartographer.cycleDetectionMs || 'N/A'}ms`);
console.log(`                  ${formatDelta(cycleDelta, 'ms', true)}`);
console.log('');

// =============================================================================
// KILL CRITERIA VALIDATION
// =============================================================================

console.log('═══════════════════════════════════════════════════════════════');
console.log('  KILL CRITERIA VALIDATION');
console.log('═══════════════════════════════════════════════════════════════');
console.log('');

const killCriteria = {
  coverage95: {
    name: 'Coverage > 95%',
    target: 95,
    baseline: baseline.codebaseIndexer.coveragePercent,
    enhanced: enhanced.codebaseIndexer.coveragePercent,
    passed: enhanced.codebaseIndexer.coveragePercent > 95,
  },
  indexTime10s: {
    name: 'Index Time < 10s',
    target: 10000,
    baseline: baseline.codebaseIndexer.indexTimeMs,
    enhanced: enhanced.codebaseIndexer.indexTimeMs,
    passed: enhanced.codebaseIndexer.indexTimeMs !== null && enhanced.codebaseIndexer.indexTimeMs < 10000,
  },
  ftsP95_50ms: {
    name: 'FTS p95 < 50ms',
    target: 50,
    baseline: baseline.factsRepository.ftsSearchP95Ms,
    enhanced: enhanced.factsRepository.ftsSearchP95Ms,
    passed: enhanced.factsRepository.ftsSearchP95Ms < 50,
  },
  memory100mb: {
    name: 'Memory < 100MB',
    target: 100,
    baseline: baseline.codebaseIndexer.memoryUsedMb,
    enhanced: enhanced.codebaseIndexer.memoryUsedMb,
    passed: enhanced.codebaseIndexer.memoryUsedMb !== null && enhanced.codebaseIndexer.memoryUsedMb < 100,
  },
  cartographer80: {
    name: 'Cartographer >= 80%',
    target: 80,
    baseline: baseline.cartographer.classificationAccuracy,
    enhanced: enhanced.cartographer.classificationAccuracy,
    passed: enhanced.cartographer.classificationAccuracy >= 80,
  },
};

// New enhanced criteria (dependency resolution)
if (enhanced.codebaseIndexer.dependencyResolutionMs !== undefined) {
  killCriteria.depResolution = {
    name: 'Dependency Resolution < 100ms',
    target: 100,
    baseline: baseline.codebaseIndexer.dependencyResolutionMs || 'N/A',
    enhanced: enhanced.codebaseIndexer.dependencyResolutionMs,
    passed: enhanced.codebaseIndexer.dependencyResolutionMs < 100,
  };
}

let passed = 0;
let total = 0;

for (const [key, criteria] of Object.entries(killCriteria)) {
  total++;
  if (criteria.passed) passed++;

  const status = criteria.passed ? '✅' : '❌';
  const baselineVal = criteria.baseline !== null ? criteria.baseline : 'N/A';
  const enhancedVal = criteria.enhanced !== null ? criteria.enhanced : 'N/A';

  console.log(`  ${status} ${criteria.name}`);
  console.log(`     Baseline: ${baselineVal} → Enhanced: ${enhancedVal} (target: ${criteria.target})`);
  console.log('');
}

console.log('───────────────────────────────────────────────────────────────');
console.log('');

const allPassed = passed === total;
const passPercent = Math.round((passed / total) * 100);

console.log(`  Kill Criteria: ${passed}/${total} (${passPercent}%)`);
console.log('');

if (allPassed) {
  console.log('  ════════════════════════════════════════════════════════════');
  console.log('  ✅ ALL KILL CRITERIA MET - SUPERMEMORY ENHANCEMENT COMPLETE');
  console.log('  ════════════════════════════════════════════════════════════');
} else {
  console.log('  ════════════════════════════════════════════════════════════');
  console.log(`  ⚠️  ${total - passed} CRITERIA NOT MET - CONTINUE ENHANCEMENT`);
  console.log('  ════════════════════════════════════════════════════════════');
}

console.log('');
console.log(`  φ⁻¹ = 61.8% max confidence`);
console.log('');
