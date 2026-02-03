#!/usr/bin/env node
/**
 * Perception Modules Benchmark
 *
 * Measures performance overhead of CYNIC's perception system.
 * Target: < 10ms per operation for real-time hook usage.
 *
 * "Le chien rapide attrape l'os" - CYNIC
 *
 * @module scripts/hooks/test/perception.bench
 */

import {
  HarmonicFeedbackSystem,
  ThompsonSampler,
  ConfidenceCalibrator,
  getHarmonicFeedback,
  resetHarmonicFeedback,
} from '../lib/harmonic-feedback.js';

const PHI_INV = 0.618;

// ═══════════════════════════════════════════════════════════════════════════
// BENCHMARK UTILITIES
// ═══════════════════════════════════════════════════════════════════════════

function benchmark(name, fn, iterations = 1000) {
  // Warmup
  for (let i = 0; i < 100; i++) fn();

  // Measure
  const start = performance.now();
  for (let i = 0; i < iterations; i++) fn();
  const end = performance.now();

  const totalMs = end - start;
  const avgMs = totalMs / iterations;
  const opsPerSec = Math.round(iterations / (totalMs / 1000));

  return { name, iterations, totalMs, avgMs, opsPerSec };
}

function formatResult(result) {
  const status = result.avgMs < 1 ? '✅' : result.avgMs < 10 ? '⚠️' : '❌';
  return `${status} ${result.name.padEnd(40)} ${result.avgMs.toFixed(4)}ms avg | ${result.opsPerSec.toLocaleString()} ops/s`;
}

// ═══════════════════════════════════════════════════════════════════════════
// BENCHMARKS
// ═══════════════════════════════════════════════════════════════════════════

console.log('');
console.log('═══════════════════════════════════════════════════════════════════════════');
console.log('  CYNIC PERCEPTION BENCHMARK');
console.log('  Target: < 1ms per operation for real-time hooks');
console.log('═══════════════════════════════════════════════════════════════════════════');
console.log('');

const results = [];

// ─────────────────────────────────────────────────────────────────────────────
// Thompson Sampler Benchmarks
// ─────────────────────────────────────────────────────────────────────────────

console.log('── Thompson Sampler ──────────────────────────────────────────────────────');

let sampler = new ThompsonSampler();

results.push(benchmark('ThompsonSampler.initArm()', () => {
  sampler.initArm(`arm_${Math.random()}`);
}));

sampler = new ThompsonSampler();
for (let i = 0; i < 100; i++) sampler.initArm(`arm_${i}`);

results.push(benchmark('ThompsonSampler.update()', () => {
  sampler.update(`arm_${Math.floor(Math.random() * 100)}`, Math.random() > 0.5);
}));

results.push(benchmark('ThompsonSampler.selectArm()', () => {
  sampler.selectArm();
}));

results.push(benchmark('ThompsonSampler.getExpectedValue()', () => {
  sampler.getExpectedValue(`arm_${Math.floor(Math.random() * 100)}`);
}));

results.push(benchmark('ThompsonSampler.getStats()', () => {
  sampler.getStats();
}));

results.push(benchmark('ThompsonSampler.exportState()', () => {
  sampler.exportState();
}));

console.log(results.slice(-6).map(formatResult).join('\n'));
console.log('');

// ─────────────────────────────────────────────────────────────────────────────
// Confidence Calibrator Benchmarks
// ─────────────────────────────────────────────────────────────────────────────

console.log('── Confidence Calibrator ─────────────────────────────────────────────────');

let calibrator = new ConfidenceCalibrator();

results.push(benchmark('ConfidenceCalibrator.calibrate()', () => {
  calibrator.calibrate(Math.random());
}));

results.push(benchmark('ConfidenceCalibrator.record()', () => {
  calibrator.record(Math.random(), Math.random() > 0.5);
}));

// Pre-fill some data
for (let i = 0; i < 100; i++) {
  calibrator.record(Math.random(), Math.random() > 0.5);
}

results.push(benchmark('ConfidenceCalibrator.getReliabilityDiagram()', () => {
  calibrator.getReliabilityDiagram();
}));

results.push(benchmark('ConfidenceCalibrator.getRecommendations()', () => {
  calibrator.getRecommendations();
}));

results.push(benchmark('ConfidenceCalibrator.exportState()', () => {
  calibrator.exportState();
}));

console.log(results.slice(-5).map(formatResult).join('\n'));
console.log('');

// ─────────────────────────────────────────────────────────────────────────────
// Harmonic Feedback System Benchmarks
// ─────────────────────────────────────────────────────────────────────────────

console.log('── Harmonic Feedback System ──────────────────────────────────────────────');

resetHarmonicFeedback();
let harmonic = getHarmonicFeedback();

results.push(benchmark('HarmonicFeedbackSystem.processFeedback()', () => {
  harmonic.processFeedback({
    type: `pattern_${Math.floor(Math.random() * 50)}`,
    sentiment: Math.random() > 0.5 ? 'positive' : 'negative',
    confidence: Math.random() * PHI_INV,
    source: 'benchmark',
  });
}));

results.push(benchmark('HarmonicFeedbackSystem.getState()', () => {
  harmonic.getState();
}));

results.push(benchmark('HarmonicFeedbackSystem.getInsights()', () => {
  harmonic.getInsights();
}));

results.push(benchmark('HarmonicFeedbackSystem.getPromotionStats()', () => {
  harmonic.getPromotionStats();
}));

results.push(benchmark('HarmonicFeedbackSystem.introspect()', () => {
  harmonic.introspect();
}));

results.push(benchmark('HarmonicFeedbackSystem.exportState()', () => {
  harmonic.exportState();
}));

// Review patterns (rate limited, so do fewer iterations)
harmonic.lastReviewTime = 0;
results.push(benchmark('HarmonicFeedbackSystem.reviewPatterns()', () => {
  harmonic.lastReviewTime = 0;
  harmonic.reviewPatterns();
}, 100));

console.log(results.slice(-7).map(formatResult).join('\n'));
console.log('');

// ─────────────────────────────────────────────────────────────────────────────
// Summary
// ─────────────────────────────────────────────────────────────────────────────

console.log('═══════════════════════════════════════════════════════════════════════════');
console.log('  SUMMARY');
console.log('═══════════════════════════════════════════════════════════════════════════');

const allAvg = results.reduce((sum, r) => sum + r.avgMs, 0) / results.length;
const maxAvg = Math.max(...results.map(r => r.avgMs));
const slowest = results.find(r => r.avgMs === maxAvg);

console.log(`  Total benchmarks:    ${results.length}`);
console.log(`  Average time:        ${allAvg.toFixed(4)}ms`);
console.log(`  Slowest operation:   ${slowest.name} (${maxAvg.toFixed(4)}ms)`);
console.log('');

const passing = results.filter(r => r.avgMs < 1).length;
const warning = results.filter(r => r.avgMs >= 1 && r.avgMs < 10).length;
const failing = results.filter(r => r.avgMs >= 10).length;

console.log(`  ✅ < 1ms:   ${passing} operations`);
console.log(`  ⚠️ 1-10ms: ${warning} operations`);
console.log(`  ❌ > 10ms: ${failing} operations`);
console.log('');

if (allAvg < 1) {
  console.log('  🎯 VERDICT: Perception system is FAST (< 1ms average)');
} else if (allAvg < 10) {
  console.log('  ⚠️ VERDICT: Perception system is ACCEPTABLE (< 10ms average)');
} else {
  console.log('  ❌ VERDICT: Perception system needs OPTIMIZATION');
}

console.log('');
console.log('═══════════════════════════════════════════════════════════════════════════');
