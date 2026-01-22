#!/usr/bin/env node

/**
 * Phase 31 Integration Test - Social & Political Philosophy
 *
 * Tests:
 * - 31A: Justice Engine (Rawls/Nozick)
 * - 31B: Social Contract Engine (Hobbes/Locke/Rousseau)
 * - 31C: Rights Engine (natural/positive/negative)
 */

const justiceEngine = require('./lib/justice-engine.cjs');
const socialContractEngine = require('./lib/social-contract-engine.cjs');
const rightsEngine = require('./lib/rights-engine.cjs');

console.log('═══════════════════════════════════════════════════════════');
console.log('  PHASE 31: SOCIAL & POLITICAL PHILOSOPHY');
console.log('  *tail wag* Testing justice, contracts, and rights...');
console.log('═══════════════════════════════════════════════════════════\n');

// Initialize all engines
console.log('── INITIALIZATION ─────────────────────────────────────────\n');

const justiceInit = justiceEngine.init();
console.log(`Justice Engine: ${justiceInit.status}`);

const contractInit = socialContractEngine.init();
console.log(`Social Contract Engine: ${contractInit.status}`);

const rightsInit = rightsEngine.init();
console.log(`Rights Engine: ${rightsInit.status}`);

// ═══════════════════════════════════════════════════════════════════
// 31A: JUSTICE ENGINE TESTS
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  31A: JUSTICE ENGINE (Rawls/Nozick)');
console.log('═══════════════════════════════════════════════════════════\n');

// Create distributions for evaluation
console.log('── Creating Distributions ─────────────────────────────────\n');

const equalDist = justiceEngine.createDistribution('equal-society', {
  description: 'Highly equal society',
  holdings: {
    worker1: 50,
    worker2: 52,
    worker3: 48,
    manager: 55,
    executive: 60
  }
});

console.log(`Created: ${equalDist.id}`);
console.log(`  Gini: ${equalDist.giniCoefficient.toFixed(3)}`);
console.log(`  Least advantaged: ${equalDist.leastAdvantaged}`);

const unequalDist = justiceEngine.createDistribution('unequal-society', {
  description: 'Highly unequal society',
  holdings: {
    worker1: 10,
    worker2: 12,
    worker3: 8,
    manager: 70,
    executive: 300
  }
});

console.log(`Created: ${unequalDist.id}`);
console.log(`  Gini: ${unequalDist.giniCoefficient.toFixed(3)}`);
console.log(`  Least advantaged: ${unequalDist.leastAdvantaged}`);

// Rawlsian evaluation
console.log('\n── Rawlsian Evaluation ────────────────────────────────────\n');

const rawlsEqual = justiceEngine.evaluateRawlsian('equal-society');
console.log(`Equal society (Rawlsian): ${rawlsEqual.verdict}`);
console.log(`  Difference principle: ${rawlsEqual.differencePrinciple.assessment}`);

const rawlsUnequal = justiceEngine.evaluateRawlsian('unequal-society');
console.log(`Unequal society (Rawlsian): ${rawlsUnequal.verdict}`);
console.log(`  Difference principle: ${rawlsUnequal.differencePrinciple.assessment}`);

// Nozickian evaluation
console.log('\n── Nozickian Evaluation ───────────────────────────────────\n');

const nozickHistory = [
  { type: 'acquisition', just: true, agent: 'founder' },
  { type: 'transfer', voluntary: true, from: 'founder', to: 'worker1' },
  { type: 'transfer', voluntary: true, from: 'founder', to: 'worker2' }
];

const nozickEval = justiceEngine.evaluateNozickian('equal-society', nozickHistory);
console.log(`Equal society (Nozickian): ${nozickEval.verdict}`);
console.log(`  Just in acquisition: ${nozickEval.entitlement.justInAcquisition}`);
console.log(`  Just in transfer: ${nozickEval.entitlement.justInTransfer}`);

// Wilt Chamberlain test
console.log('\n── Wilt Chamberlain Argument ──────────────────────────────\n');

const wiltTest = justiceEngine.wiltChamberlainTest('equal-society', [
  { from: 'worker1', to: 'wilt', amount: 5, voluntary: true },
  { from: 'worker2', to: 'wilt', amount: 5, voluntary: true },
  { from: 'worker3', to: 'wilt', amount: 5, voluntary: true }
]);

console.log(`Scenario: ${wiltTest.name}`);
console.log(`  Wilt gains: ${wiltTest.step3.wiltGains}`);
console.log(`  Nozick's point: "${wiltTest.conclusion.nozickPoint}"`);

// ═══════════════════════════════════════════════════════════════════
// 31B: SOCIAL CONTRACT ENGINE TESTS
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  31B: SOCIAL CONTRACT ENGINE (Hobbes/Locke/Rousseau)');
console.log('═══════════════════════════════════════════════════════════\n');

// Analyze state of nature
console.log('── State of Nature Analysis ───────────────────────────────\n');

const stateOfNature = socialContractEngine.analyzeStateOfNature({
  description: 'Pre-political community',
  scarcity: true,
  competition: true,
  noAuthority: true,
  moralNorms: true,
  society: true,
  inequality: false
});

console.log('Hobbesian analysis:');
console.log(`  War likelihood: ${stateOfNature.hobbesianAnalysis.warLikelihood}`);
console.log(`  Sovereign needed: ${stateOfNature.hobbesianAnalysis.sovereignNeeded}`);

console.log('Lockean analysis:');
console.log(`  Natural law present: ${stateOfNature.lockeanAnalysis.naturalLawPresent}`);
console.log(`  Inconveniences: ${stateOfNature.lockeanAnalysis.inconveniences.join(', ')}`);

console.log('Rousseauian analysis:');
console.log(`  Corruption level: ${stateOfNature.rousseauianAnalysis.corruptionLevel}`);

// Compare theories
console.log('\n── Theory Comparison ──────────────────────────────────────\n');

const comparison = socialContractEngine.compareTheories();
console.log('Key difference - Is state of nature war?');
console.log(`  Hobbes: ${comparison.keyDifferences[0].hobbes}`);
console.log(`  Locke: ${comparison.keyDifferences[0].locke}`);
console.log(`  Rousseau: ${comparison.keyDifferences[0].rousseau}`);

// Evaluate legitimacy
console.log('\n── Legitimacy Evaluation ──────────────────────────────────\n');

const legitimacyEval = socialContractEngine.evaluateLegitimacy({
  name: 'Democratic Republic',
  providesSecurity: true,
  consent: true,
  protectsRights: true,
  generalWill: false,
  participation: true
});

console.log(`Entity: ${legitimacyEval.entity}`);
console.log(`  Legitimate by: ${legitimacyEval.overall.legitimateBy.join(', ')}`);
console.log(`  Not legitimate by: ${legitimacyEval.overall.notLegitimateBy.join(', ')}`);
console.log(`  Contested: ${legitimacyEval.overall.contested}`);

// Revolution assessment
console.log('\n── Revolution Assessment ──────────────────────────────────\n');

const revolutionAssess = socialContractEngine.assessRevolution({
  description: 'Tyrannical government violating rights',
  violatesTrust: true,
  attacksRights: true,
  systematic: true,
  peacefulMeansTried: true,
  personalThreat: false,
  popularWill: true
});

console.log('Revolution justified?');
console.log(`  Lockean: ${revolutionAssess.lockean.justified}`);
console.log(`  Hobbesian: ${revolutionAssess.hobbesian.justified}`);
console.log(`  Rousseauian: ${revolutionAssess.rousseauian.justified}`);
console.log(`  Overall: ${revolutionAssess.overall.justified ? 'YES' : 'NO'}`);

// ═══════════════════════════════════════════════════════════════════
// 31C: RIGHTS ENGINE TESTS
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  31C: RIGHTS ENGINE (Natural/Positive/Negative)');
console.log('═══════════════════════════════════════════════════════════\n');

// Define rights
console.log('── Defining Rights ────────────────────────────────────────\n');

const lifeRight = rightsEngine.defineRight('right-to-life', {
  name: 'Right to Life',
  description: 'Right not to be killed',
  ontology: 'natural',
  polarity: 'negative',
  hohfeldType: 'claim',
  holder: 'all persons',
  dutyBearer: 'all others',
  weight: 0.9,
  derogable: false
});

console.log(`Defined: ${lifeRight.name}`);
console.log(`  Ontology: ${lifeRight.ontology}`);
console.log(`  Polarity: ${lifeRight.polarity}`);
console.log(`  Derogable: ${lifeRight.derogable}`);

const educationRight = rightsEngine.defineRight('right-to-education', {
  name: 'Right to Education',
  description: 'Right to receive education',
  ontology: 'legal',
  polarity: 'positive',
  hohfeldType: 'claim',
  holder: 'children',
  dutyBearer: 'state',
  weight: 0.6,
  derogable: true
});

console.log(`Defined: ${educationRight.name}`);
console.log(`  Ontology: ${educationRight.ontology}`);
console.log(`  Polarity: ${educationRight.polarity}`);
console.log(`  Derogable: ${educationRight.derogable}`);

// Hohfeldian analysis
console.log('\n── Hohfeldian Analysis ────────────────────────────────────\n');

const hohfeldAnalysis = rightsEngine.hohfeldianAnalysis('right-to-life');
console.log(`Analyzing: ${hohfeldAnalysis.name}`);
console.log(`  Primary: ${hohfeldAnalysis.primaryRelation.explanation}`);
console.log(`  Correlative: ${hohfeldAnalysis.correlativeRelation.explanation}`);
console.log(`  Picture: ${hohfeldAnalysis.hohfeldianPicture.relation}`);

// Compare positive/negative
console.log('\n── Positive vs Negative Rights ────────────────────────────\n');

const posNegCompare = rightsEngine.comparePositiveNegative();
console.log('Negative rights:');
console.log(`  Duty: ${posNegCompare.negative.correlativeDuty}`);
console.log(`  Cost: ${posNegCompare.negative.cost}`);
console.log('Positive rights:');
console.log(`  Duty: ${posNegCompare.positive.correlativeDuty}`);
console.log(`  Cost: ${posNegCompare.positive.cost}`);

// Analyze conflict
console.log('\n── Rights Conflict Analysis ───────────────────────────────\n');

const freeSpeeechRight = rightsEngine.defineRight('free-speech', {
  name: 'Freedom of Speech',
  ontology: 'natural',
  polarity: 'negative',
  hohfeldType: 'privilege',
  holder: 'all persons',
  weight: 0.7,
  derogable: true
});

const privacyRight = rightsEngine.defineRight('privacy', {
  name: 'Right to Privacy',
  ontology: 'natural',
  polarity: 'negative',
  hohfeldType: 'claim',
  holder: 'all persons',
  dutyBearer: 'all others',
  weight: 0.7,
  derogable: true
});

const conflict = rightsEngine.analyzeConflict('free-speech', 'privacy', {
  description: 'Journalist wants to publish private information'
});

console.log(`Conflict: ${conflict.scenario}`);
console.log(`  Genuine conflict: ${conflict.nature.genuine}`);
console.log(`  Absolutist: ${conflict.absolutist.result}`);
console.log(`  Balancing: ${conflict.balancing.result}`);
console.log(`  Recommendation: ${conflict.recommendation}`);

// ═══════════════════════════════════════════════════════════════════
// STATUS DISPLAYS
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  ENGINE STATUS');
console.log('═══════════════════════════════════════════════════════════\n');

console.log(justiceEngine.formatStatus());
console.log('\n');
console.log(socialContractEngine.formatStatus());
console.log('\n');
console.log(rightsEngine.formatStatus());

// ═══════════════════════════════════════════════════════════════════
// SUMMARY
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  PHASE 31 SUMMARY');
console.log('═══════════════════════════════════════════════════════════\n');

const justiceStats = justiceEngine.getStats();
const contractStats = socialContractEngine.getStats();
const rightsStats = rightsEngine.getStats();

console.log('31A Justice Engine:');
console.log(`  Distributions: ${justiceStats.distributions}`);
console.log(`  Evaluations: ${justiceStats.evaluations}`);

console.log('\n31B Social Contract Engine:');
console.log(`  Contracts: ${contractStats.contracts}`);
console.log(`  Legitimacy evaluations: ${contractStats.legitimacyEvaluations}`);
console.log(`  State of nature analyses: ${contractStats.stateOfNatureAnalyses}`);

console.log('\n31C Rights Engine:');
console.log(`  Rights defined: ${rightsStats.rights}`);
console.log(`  Conflicts analyzed: ${rightsStats.conflicts}`);
console.log(`  Analyses: ${rightsStats.analyses}`);

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  *tail wag* PHASE 31 COMPLETE');
console.log('  Social & Political Philosophy operational.');
console.log('  φ-bounded confidence: max 61.8%');
console.log('═══════════════════════════════════════════════════════════\n');
