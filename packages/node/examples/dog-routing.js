#!/usr/bin/env node
/**
 * CYNIC Example: Dog Routing (Kabbalistic Router)
 *
 * Demonstrates how the KabbalisticRouter routes tasks to the appropriate
 * Dogs (Sefirot) in the Collective Pack based on task type.
 *
 * The "Lightning Flash" (Seder Hishtalshelut) flows through the Tree of Life:
 *
 *                    Keter (CYNIC)
 *                   /      |      \
 *           Binah     Daat      Chochmah
 *         (Analyst) (Scholar)   (Sage)
 *                   \      |      /
 *           Gevurah   Tiferet   Chesed
 *         (Guardian)  (Oracle) (Architect)
 *                   \      |      /
 *             Hod      Yesod     Netzach
 *          (Deployer) (Janitor)  (Scout)
 *                   \      |      /
 *                    Malkhut
 *                 (Cartographer)
 *
 * Run: node packages/node/examples/dog-routing.js
 *
 * "L'arbre vit" - The tree lives.
 */

'use strict';

import { KabbalisticRouter, CollectivePack, DOGS } from '@cynic/node';

// =============================================================================
// SETUP
// =============================================================================

console.log('═══════════════════════════════════════════════════════════════════');
console.log('🐕 CYNIC Dog Routing Example (Kabbalistic Router)');
console.log('═══════════════════════════════════════════════════════════════════\n');

// Create the collective pack and router
const pack = new CollectivePack();
const router = new KabbalisticRouter({ pack });

// =============================================================================
// EXAMPLE 1: List All Dogs
// =============================================================================

console.log('── THE COLLECTIVE PACK (11 Dogs × Sefirot) ────────────────────────');
console.log('');

const dogs = [
  { name: 'CYNIC',        sefira: 'Keter',    role: 'Crown - Orchestration' },
  { name: 'Analyst',      sefira: 'Binah',    role: 'Understanding - Pattern detection' },
  { name: 'Scholar',      sefira: 'Daat',     role: 'Knowledge - Documentation' },
  { name: 'Sage',         sefira: 'Chochmah', role: 'Wisdom - Guidance' },
  { name: 'Guardian',     sefira: 'Gevurah',  role: 'Judgment - Security' },
  { name: 'Oracle',       sefira: 'Tiferet',  role: 'Beauty - Insights' },
  { name: 'Architect',    sefira: 'Chesed',   role: 'Kindness - Design' },
  { name: 'Deployer',     sefira: 'Hod',      role: 'Splendor - CI/CD' },
  { name: 'Janitor',      sefira: 'Yesod',    role: 'Foundation - Cleanup' },
  { name: 'Scout',        sefira: 'Netzach',  role: 'Victory - Exploration' },
  { name: 'Cartographer', sefira: 'Malkhut',  role: 'Kingdom - Mapping' },
];

for (const dog of dogs) {
  console.log(`   ${DOGS[dog.name.toUpperCase()]?.icon || '🐕'} ${dog.name.padEnd(14)} ${dog.sefira.padEnd(10)} ${dog.role}`);
}
console.log('');

// =============================================================================
// EXAMPLE 2: Task Type Routing
// =============================================================================

console.log('── TASK TYPE → DOG ROUTING ────────────────────────────────────────');
console.log('');

const taskTypes = [
  { type: 'protection', description: 'Security scan, blocking dangerous operations' },
  { type: 'analysis',   description: 'Pattern detection, code analysis' },
  { type: 'design',     description: 'Architecture decisions, system design' },
  { type: 'wisdom',     description: 'Teaching, guidance, philosophical questions' },
  { type: 'knowledge',  description: 'Documentation, knowledge extraction' },
  { type: 'visualization', description: 'Dashboards, insights, reports' },
  { type: 'exploration', description: 'Code search, discovery, reconnaissance' },
  { type: 'cleanup',    description: 'Code hygiene, linting, refactoring' },
  { type: 'deployment', description: 'CI/CD, infrastructure, releases' },
  { type: 'mapping',    description: 'Codebase structure, ecosystem mapping' },
];

for (const task of taskTypes) {
  const dog = router.getEntryPoint(task.type);
  const icon = DOGS[dog?.toUpperCase()]?.icon || '🐕';
  console.log(`   ${task.type.padEnd(14)} → ${icon} ${(dog || 'guardian').padEnd(12)} ${task.description}`);
}
console.log('');

// =============================================================================
// EXAMPLE 3: Route a Task
// =============================================================================

console.log('── ROUTING A TASK ─────────────────────────────────────────────────');
console.log('');

const task = {
  type: 'protection',
  content: 'Check if rm -rf / is safe',
  context: { toolName: 'Bash', command: 'rm -rf /' },
};

console.log(`Task: "${task.content}"`);
console.log(`Type: ${task.type}`);
console.log('');

const result = await router.route(task);

console.log('Routing Result:');
console.log(`   Entry Dog:     ${result.entryDog?.name || 'Guardian'}`);
console.log(`   Consulted:     ${result.consultations?.map(c => c.name).join(' → ') || 'None'}`);
console.log(`   Final Decision: ${result.decision || 'Block'}`);
console.log(`   Confidence:    ${(result.confidence * 100).toFixed(1)}%`);
console.log('');

// =============================================================================
// EXAMPLE 4: Consultation Chain
// =============================================================================

console.log('── CONSULTATION CHAIN (Lightning Flash) ──────────────────────────');
console.log('');

// Simulate a complex task that requires consultation
const complexTask = {
  type: 'design',
  content: 'Design a new authentication system',
  context: {
    requiresSecurity: true,
    requiresDocumentation: true,
    complexity: 'high',
  },
};

console.log(`Task: "${complexTask.content}"`);
console.log('');

// The router will consult multiple dogs
const consultationResult = await router.route(complexTask);

console.log('Consultation Chain:');
if (consultationResult.consultations && consultationResult.consultations.length > 0) {
  for (let i = 0; i < consultationResult.consultations.length; i++) {
    const c = consultationResult.consultations[i];
    const arrow = i === 0 ? '┌─' : i === consultationResult.consultations.length - 1 ? '└─' : '├─';
    console.log(`   ${arrow} ${DOGS[c.name?.toUpperCase()]?.icon || '🐕'} ${c.name}: ${c.decision || 'consulted'}`);
  }
} else {
  console.log(`   ${DOGS[consultationResult.entryDog?.name?.toUpperCase()]?.icon || '🐕'} ${consultationResult.entryDog?.name || 'Guardian'} handled it alone`);
}
console.log('');

// =============================================================================
// EXAMPLE 5: φ-Aligned Thresholds
// =============================================================================

console.log('── φ-ALIGNED THRESHOLDS ───────────────────────────────────────────');
console.log('');
console.log('   CONSENSUS:   61.8% (φ⁻¹) - Agreement needed for consensus');
console.log('   ESCALATION:  38.2% (φ⁻²) - Below this triggers escalation');
console.log('   CERTAINTY:   55.6% (φ⁻¹ × 0.9) - High confidence threshold');
console.log('');
console.log('   Max consultations: 5 (circuit breaker)');
console.log('   Max depth: 3 levels');
console.log('');

console.log('═══════════════════════════════════════════════════════════════════');
console.log('*tail wag* The tree lives. L\'arbre vit.');
console.log('═══════════════════════════════════════════════════════════════════');
