#!/usr/bin/env node
/**
 * CYNIC Example: Pattern Detection
 *
 * Demonstrates how to use the PatternDetector from @cynic/emergence
 * to identify emergent patterns in observations.
 *
 * Pattern Types:
 * - SEQUENCE:  Repeated sequences of events
 * - ANOMALY:   Unusual values or behaviors
 * - TREND:     Gradual changes over time
 * - CLUSTER:   Groups of related events
 * - EMERGENCE: New patterns forming from combinations
 *
 * Run: node packages/node/examples/pattern-detection.js
 *
 * "The crown observes all" - κυνικός
 */

'use strict';

import {
  PatternDetector,
  createPatternDetector,
  PatternType,
  SIGNIFICANCE_THRESHOLDS,
} from '@cynic/emergence';

// =============================================================================
// SETUP
// =============================================================================

console.log('═══════════════════════════════════════════════════════════════════');
console.log('🔮 CYNIC Pattern Detection Example');
console.log('═══════════════════════════════════════════════════════════════════\n');

// Create pattern detector
const detector = createPatternDetector({
  windowSize: 10,         // Look at last 10 observations
  minSequenceLength: 3,   // Minimum events to form a sequence
  anomalyThreshold: 2.0,  // Standard deviations for anomaly
});

// =============================================================================
// EXAMPLE 1: Sequence Detection
// =============================================================================

console.log('── SEQUENCE DETECTION ─────────────────────────────────────────────');
console.log('');

// Feed a repeating sequence
const sequence = ['Read', 'Edit', 'Write', 'Read', 'Edit', 'Write', 'Read', 'Edit', 'Write'];
console.log('Feeding sequence: Read → Edit → Write (3 times)');
console.log('');

for (const action of sequence) {
  detector.observe({
    type: 'TOOL_USE',
    value: action,
    timestamp: Date.now(),
  });
}

// Detect patterns
let patterns = detector.detect();
console.log(`Patterns detected: ${patterns.length}`);

for (const pattern of patterns) {
  if (pattern.type === PatternType.SEQUENCE) {
    console.log(`   🔄 SEQUENCE: ${pattern.sequence?.join(' → ') || 'unknown'}`);
    console.log(`      Occurrences: ${pattern.occurrences}`);
    console.log(`      Significance: ${(pattern.significance * 100).toFixed(1)}%`);
  }
}
console.log('');

// =============================================================================
// EXAMPLE 2: Anomaly Detection
// =============================================================================

console.log('── ANOMALY DETECTION ──────────────────────────────────────────────');
console.log('');

// Create a new detector for scores
const scoreDetector = createPatternDetector({
  windowSize: 20,
  anomalyThreshold: 1.5,
});

// Feed normal scores (50-70 range)
const normalScores = [55, 60, 58, 62, 57, 63, 59, 61, 56, 64];
console.log('Normal scores: 55-64 range');

for (const score of normalScores) {
  scoreDetector.observe({
    type: 'SCORE',
    value: score,
    timestamp: Date.now(),
  });
}

// Now feed an anomaly
console.log('Adding anomaly: score = 15');
scoreDetector.observe({
  type: 'SCORE',
  value: 15,  // Way below normal
  timestamp: Date.now(),
});

patterns = scoreDetector.detect();
console.log(`Patterns detected: ${patterns.length}`);

for (const pattern of patterns) {
  if (pattern.type === PatternType.ANOMALY) {
    console.log(`   ⚠️ ANOMALY: value=${pattern.value}, expected=${pattern.expected?.toFixed(1)}`);
    console.log(`      Deviation: ${pattern.deviation?.toFixed(2)} std`);
    console.log(`      Significance: ${(pattern.significance * 100).toFixed(1)}%`);
  }
}
console.log('');

// =============================================================================
// EXAMPLE 3: Trend Detection
// =============================================================================

console.log('── TREND DETECTION ────────────────────────────────────────────────');
console.log('');

// Create detector for trend analysis
const trendDetector = createPatternDetector({
  windowSize: 10,
  trendThreshold: 0.5,  // Minimum slope for trend
});

// Feed increasing scores (burnout risk pattern!)
const increasingScores = [10, 15, 20, 28, 35, 45, 52, 60, 70, 78];
console.log('Feeding increasing scores: 10 → 78 (burnout risk?)');

for (let i = 0; i < increasingScores.length; i++) {
  trendDetector.observe({
    type: 'HEAT',  // Temperature/heat tracking
    value: increasingScores[i],
    timestamp: Date.now() + i * 1000,  // 1 second apart
  });
}

patterns = trendDetector.detect();
console.log(`Patterns detected: ${patterns.length}`);

for (const pattern of patterns) {
  if (pattern.type === PatternType.TREND) {
    const direction = pattern.slope > 0 ? '📈 INCREASING' : '📉 DECREASING';
    console.log(`   ${direction}`);
    console.log(`      Slope: ${pattern.slope?.toFixed(2)}`);
    console.log(`      Duration: ${pattern.duration} observations`);
    console.log(`      Significance: ${(pattern.significance * 100).toFixed(1)}%`);
  }
}
console.log('');

// =============================================================================
// EXAMPLE 4: Error Cluster Detection
// =============================================================================

console.log('── ERROR CLUSTER DETECTION ────────────────────────────────────────');
console.log('');

const errorDetector = createPatternDetector({
  windowSize: 10,
  clusterThreshold: 3,  // 3 similar events = cluster
});

// Feed rapid errors (should trigger cluster detection)
console.log('Feeding 5 rapid errors...');
for (let i = 0; i < 5; i++) {
  errorDetector.observe({
    type: 'ERROR',
    value: 'npm test failed',
    category: 'test_failure',
    timestamp: Date.now() + i * 100,  // 100ms apart
  });
}

patterns = errorDetector.detect();
console.log(`Patterns detected: ${patterns.length}`);

for (const pattern of patterns) {
  if (pattern.type === PatternType.CLUSTER) {
    console.log(`   🔴 CLUSTER: ${pattern.category || 'errors'}`);
    console.log(`      Count: ${pattern.count}`);
    console.log(`      Time span: ${pattern.timeSpan}ms`);
    console.log(`      Significance: ${(pattern.significance * 100).toFixed(1)}%`);
  }
}
console.log('');

// =============================================================================
// EXAMPLE 5: Get All Patterns
// =============================================================================

console.log('── PATTERN SUMMARY ────────────────────────────────────────────────');
console.log('');

// Get patterns from all detectors
const allPatterns = [
  ...detector.getPatterns(),
  ...scoreDetector.getPatterns(),
  ...trendDetector.getPatterns(),
  ...errorDetector.getPatterns(),
];

const byType = {};
for (const p of allPatterns) {
  byType[p.type] = (byType[p.type] || 0) + 1;
}

console.log('Patterns by type:');
for (const [type, count] of Object.entries(byType)) {
  console.log(`   ${type}: ${count}`);
}
console.log('');

// =============================================================================
// EXAMPLE 6: Significance Thresholds
// =============================================================================

console.log('── SIGNIFICANCE THRESHOLDS ────────────────────────────────────────');
console.log('');
console.log(`   LOW:      ${SIGNIFICANCE_THRESHOLDS.LOW * 100}% (informational)`);
console.log(`   MEDIUM:   ${SIGNIFICANCE_THRESHOLDS.MEDIUM * 100}% (worth noting)`);
console.log(`   HIGH:     ${SIGNIFICANCE_THRESHOLDS.HIGH * 100}% (action needed)`);
console.log(`   CRITICAL: ${SIGNIFICANCE_THRESHOLDS.CRITICAL * 100}% (immediate attention)`);
console.log('');

console.log('═══════════════════════════════════════════════════════════════════');
console.log('*sniff* Patterns detected. The crown observes all.');
console.log('═══════════════════════════════════════════════════════════════════');
