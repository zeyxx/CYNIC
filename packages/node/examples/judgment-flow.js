#!/usr/bin/env node
/**
 * CYNIC Example: Judgment Flow
 *
 * Demonstrates how to use the CYNICJudge to evaluate items
 * across the 25 dimensions grouped by the 4 axioms.
 *
 * Run: node packages/node/examples/judgment-flow.js
 *
 * "φ distrusts φ" - Max confidence 61.8%
 */

'use strict';

import { CYNICJudge, getAllDimensions } from '@cynic/node';

// =============================================================================
// EXAMPLE 1: Basic Judgment
// =============================================================================

console.log('═══════════════════════════════════════════════════════════════════');
console.log('🐕 CYNIC Judgment Flow Example');
console.log('═══════════════════════════════════════════════════════════════════\n');

// Create a judge instance
const judge = new CYNICJudge();

// Item to judge - could be a token, code, decision, etc.
const item = {
  name: 'Example Token',
  type: 'token',
  // PHI axiom signals
  coherence: 0.72,          // Structural integrity
  elegance: 0.65,           // Simplicity
  // VERIFY axiom signals
  verified: true,           // Has been verified
  auditTrail: ['step1', 'step2'],
  // CULTURE axiom signals
  communityBenefit: true,   // Benefits community
  openSource: true,         // Open source
  // BURN axiom signals
  extractive: false,        // Not extractive
  contributions: 5,         // Number of contributions
};

console.log('── ITEM TO JUDGE ──────────────────────────────────────────────────');
console.log(JSON.stringify(item, null, 2));
console.log('');

// =============================================================================
// EXAMPLE 2: Execute Judgment
// =============================================================================

console.log('── JUDGMENT RESULT ────────────────────────────────────────────────');

const judgment = await judge.judge(item);

console.log(`Q-Score:  ${judgment.qScore.toFixed(1)}`);
console.log(`Verdict:  ${judgment.verdict}`);

// Interpret verdict
const verdictMeanings = {
  'HOWL': '🎉 Exceptional! (≥80)',
  'WAG':  '✅ Passes! (≥50)',
  'GROWL': '⚠️ Needs work (≥38.2)',
  'BARK': '❌ Critical (<38.2)',
};
console.log(`Meaning:  ${verdictMeanings[judgment.verdict]}`);
console.log('');

// =============================================================================
// EXAMPLE 3: Axiom Breakdown
// =============================================================================

console.log('── AXIOM SCORES ───────────────────────────────────────────────────');

const axioms = ['PHI', 'VERIFY', 'CULTURE', 'BURN'];
for (const axiom of axioms) {
  const score = judgment.axiomScores?.[axiom] || 'N/A';
  const bar = typeof score === 'number'
    ? '█'.repeat(Math.round(score / 10)) + '░'.repeat(10 - Math.round(score / 10))
    : '░░░░░░░░░░';
  console.log(`${axiom.padEnd(8)} [${bar}] ${typeof score === 'number' ? score.toFixed(1) : score}`);
}
console.log('');

// =============================================================================
// EXAMPLE 4: Dimension Scores
// =============================================================================

console.log('── DIMENSION SCORES (Top 10) ──────────────────────────────────────');

const dimensions = getAllDimensions();
const dimensionScores = judgment.dimensionScores || {};

// Get top 10 dimensions by score
const sortedDimensions = Object.entries(dimensionScores)
  .sort(([, a], [, b]) => b - a)
  .slice(0, 10);

for (const [dim, score] of sortedDimensions) {
  const dimInfo = dimensions[dim];
  const axiom = dimInfo?.axiom || '?';
  const bar = '█'.repeat(Math.round(score / 10)) + '░'.repeat(10 - Math.round(score / 10));
  console.log(`${dim.padEnd(20)} [${bar}] ${score.toFixed(1)} (${axiom})`);
}
console.log('');

// =============================================================================
// EXAMPLE 5: Weaknesses Analysis
// =============================================================================

if (judgment.weaknesses && judgment.weaknesses.length > 0) {
  console.log('── WEAKNESSES ─────────────────────────────────────────────────────');
  for (const weakness of judgment.weaknesses) {
    console.log(`⚠️ ${weakness.dimension}: ${weakness.score.toFixed(1)} - ${weakness.suggestion || 'Improve this dimension'}`);
  }
  console.log('');
}

// =============================================================================
// EXAMPLE 6: Multiple Items Comparison
// =============================================================================

console.log('── COMPARISON ─────────────────────────────────────────────────────');

const items = [
  { name: 'Good Token', verified: true, openSource: true, contributions: 10 },
  { name: 'Mid Token', verified: true, openSource: false, contributions: 3 },
  { name: 'Bad Token', verified: false, extractive: true, hiddenCosts: true },
];

for (const testItem of items) {
  const result = await judge.judge(testItem);
  const icon = result.verdict === 'HOWL' ? '🎉' :
               result.verdict === 'WAG' ? '✅' :
               result.verdict === 'GROWL' ? '⚠️' : '❌';
  console.log(`${icon} ${testItem.name.padEnd(15)} Q:${result.qScore.toFixed(1).padStart(5)} ${result.verdict}`);
}

console.log('');
console.log('═══════════════════════════════════════════════════════════════════');
console.log('*tail wag* Judgment complete. φ distrusts φ.');
console.log('═══════════════════════════════════════════════════════════════════');
