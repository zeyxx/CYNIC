#!/usr/bin/env node
/**
 * Phase 27 Integration Test: Aesthetics & Value
 *
 * Tests:
 * - 27A: Beauty Engine (Kant)
 * - 27B: Art Ontology (Danto/Dickie)
 * - 27C: Taste Engine (Hume/Kant)
 */

console.log('=== Phase 27 Integration Test ===\n');

// 27A: Beauty Engine
console.log('27A: Beauty Engine');
const beauty = require('./lib/beauty-engine.cjs');
beauty.init();

// Register an artwork
const sunset = beauty.registerObject('sunset_001', {
  name: 'Turner Sunset',
  type: 'artistic'
});
console.log(`  registerObject: ${sunset.name} - ${sunset.type}`);

// Judge beauty using Kant's four moments
const judgment = beauty.judgeBeauty('sunset_001', {
  quality: { disinterested: true, score: 0.9 },
  quantity: { universal: true, score: 0.85 },
  relation: { purposive: true, score: 0.8 },
  modality: { necessary: true, score: 0.75 }
});
console.log(`  judgeBeauty: verdict ${judgment.verdict} (score: ${(judgment.score * 100).toFixed(1)}%)`);

// Golden ratio analysis (measurements should form φ ratio)
const goldenAnalysis = beauty.analyzeGoldenRatio('sunset_001', [1618, 1000]);
console.log(`  goldenRatio: φ present = ${goldenAnalysis.goldenRatioPresent}`);

// Record sublime experience
const sublime = beauty.recordSublime('sunset_001', {
  type: 'dynamic',
  intensity: 0.9,
  response: 'overwhelming vastness'
});
console.log(`  recordSublime: ${sublime.type} sublime - "${sublime.response}"`);

console.log('  ✓ Beauty Engine OK\n');

// 27B: Art Ontology
console.log('27B: Art Ontology');
const art = require('./lib/art-ontology.cjs');
art.init();

// Register artworld members
const curator = art.registerMember('curator_001', {
  name: 'Museum Curator',
  roles: ['presenter', 'critic'],
  institution: 'Metropolitan Museum'
});
console.log(`  registerMember: ${curator.name} (power: ${(curator.conferralPower * 100).toFixed(1)}%)`);

// Register art candidate (Duchamp's Fountain as example)
const fountain = art.registerCandidate('fountain_001', {
  title: 'Fountain',
  creator: 'R. Mutt (Duchamp)',
  category: 'readymade',
  year: 1917,
  aboutness: 'The nature of art itself',
  historicalContext: 'Dada, rejection of bourgeois art'
});
console.log(`  registerCandidate: ${fountain.title} - ${fountain.category}`);

// Confer art status
const conferral = art.conferStatus('fountain_001', 'curator_001', 'Revolutionary challenge to art definition');
console.log(`  conferStatus: ${conferral.newStatus} (total power: ${(conferral.totalPower * 100).toFixed(1)}%)`);

// Evaluate Danto conditions
const dantoEval = art.evaluateDantoConditions('fountain_001');
console.log(`  evaluateDantoConditions: ${dantoEval.meetsThreshold ? 'MEETS' : 'FAILS'} threshold`);
console.log(`    Insight: "${dantoEval.insight.substring(0, 60)}..."`);

// Analyze transfiguration
const transfig = art.analyzeTransfiguration('fountain_001');
console.log(`  analyzeTransfiguration: ${transfig.verdict}`);

console.log('  ✓ Art Ontology OK\n');

// 27C: Taste Engine
console.log('27C: Taste Engine');
const taste = require('./lib/taste-engine.cjs');
taste.init();

// Register critics with Hume's qualities
const idealCritic = taste.registerCritic('critic_001', {
  name: 'Experienced Critic',
  delicacy: 0.9,
  practice: 0.85,
  comparison: 0.8,
  freedom_prejudice: 0.75,
  good_sense: 0.8
});
console.log(`  registerCritic: ${idealCritic.name}`);
console.log(`    Ideal critic: ${idealCritic.isIdealCritic} (score: ${(idealCritic.normalizedScore * 100).toFixed(1)}%)`);

// Make aesthetic judgment
const tasteJudgment = taste.judgeAesthetic('fountain_001', 'critic_001', {
  category: 'sublime',
  rating: 0.6,
  quality: true,    // Disinterested
  quantity: true,   // Claims universality
  relation: true,   // Purposive without purpose
  modality: false,  // Not necessary for all
  reasons: ['Conceptual audacity', 'Historical significance']
});
console.log(`  judgeAesthetic: ${tasteJudgment.category} (rating: ${(tasteJudgment.rating * 100).toFixed(1)}%)`);

// Analyze Kantian judgment
const kantAnalysis = taste.analyzeKantianJudgment(tasteJudgment);
console.log(`  analyzeKantianJudgment: ${kantAnalysis.verdict}`);

// Get taste verdict
const verdict = taste.getTasteVerdict('fountain_001');
console.log(`  getTasteVerdict: ${verdict.verdict}`);
console.log(`    Hume: "${verdict.humeInsight.substring(0, 50)}..."`);

console.log('  ✓ Taste Engine OK\n');

console.log('=== Phase 27 Integration Test PASSED ===\n');
