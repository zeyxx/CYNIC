#!/usr/bin/env node
/**
 * Phase 30 Integration Test: Philosophy of Action
 *
 * Tests:
 * - 30A: Action Theory Engine (Davidson)
 * - 30B: Free Will Engine (determinism debate)
 * - 30C: Practical Reason Engine (Aristotle/Kant)
 */

console.log('=== Phase 30 Integration Test ===\n');

// 30A: Action Theory Engine
console.log('30A: Action Theory Engine');
const actionTheory = require('./lib/action-theory-engine.cjs');
actionTheory.init();

// Register an event with multiple descriptions (Davidson's insight)
const event = actionTheory.registerEvent('assassin_001', {
  description: 'Moving finger',
  agent: 'Gavrilo Princip',
  descriptions: ['Moving finger', 'Pulling trigger', 'Firing gun', 'Shooting archduke'],
  intentionalUnder: ['Moving finger', 'Pulling trigger', 'Firing gun', 'Shooting archduke']
});
console.log(`  registerEvent: "${event.baseDescription}" (${event.descriptions.length} descriptions)`);

// Add more descriptions (accordion effect)
const accordion = actionTheory.addDescription('assassin_001', 'Starting WWI', { intentional: false });
console.log(`  addDescription: "${accordion.descriptions[accordion.descriptions.length - 1]}" (intentional: false)`);

// Classify as action
const classification = actionTheory.classifyAsAction('assassin_001');
console.log(`  classifyAsAction: isAction = ${classification.isAction}`);
console.log(`    Davidson criterion: hasAgent = ${classification.davidsonCriterion.hasAgent}, intentional = ${classification.davidsonCriterion.intentionalUnderSomeDescription}`);

// Assign primary reason (Davidson: reasons as causes)
const primaryReason = actionTheory.assignPrimaryReason('assassin_001', {
  proAttitude: 'Desire to free Bosnia from Austrian rule',
  belief: 'Killing the archduke will advance the cause',
  confidence: 0.38
});
console.log(`  assignPrimaryReason:`);
console.log(`    Pro-attitude: ${primaryReason.proAttitude}`);
console.log(`    Belief: ${primaryReason.belief}`);
console.log(`    Davidson insight: ${primaryReason.davidsonInsight}`);

// Analyze accordion effect
const accordionAnalysis = actionTheory.analyzeAccordionEffect('assassin_001');
console.log(`  analyzeAccordionEffect:`);
console.log(`    Expansion: ${accordionAnalysis.accordion.expansion}`);
console.log(`    Coarse-grained: ${accordionAnalysis.eventIdentity.coarse}`);

// Test deviant causal chain
const deviantTest = actionTheory.testDeviantCausalChain('assassin_001', {
  deviant: false,
  scenario: 'standard assassination'
});
console.log(`  testDeviantCausalChain: ${deviantTest.verdict}`);

// Compare event identity theories
const identityComparison = actionTheory.compareEventIdentity(['Moving finger', 'Pulling trigger', 'Killing archduke']);
console.log(`  compareEventIdentity: ${identityComparison.coarseGrained.verdict} vs ${identityComparison.fineGrained.verdict}`);

console.log('  ✓ Action Theory Engine OK\n');

// 30B: Free Will Engine
console.log('30B: Free Will Engine');
const freeWill = require('./lib/free-will-engine.cjs');
freeWill.init();

// Register an agent
const agent = freeWill.registerAgent('jones_001', {
  name: 'Jones',
  type: 'human'
});
console.log(`  registerAgent: ${agent.name} (type: ${agent.type})`);

// Register an action (not a decision)
const action = freeWill.registerAction('vote_001', {
  description: 'Vote for candidate A',
  agent: 'jones_001',
  couldHaveDoneOtherwise: true,
  knewAction: true,
  knewConsequences: true
});
console.log(`  registerAction: "${action.description}"`);

// Assess responsibility
const responsibility = freeWill.assessResponsibility('vote_001');
console.log(`  assessResponsibility: responsible = ${responsibility.responsible}`);
console.log(`    Conditions: control = ${responsibility.conditions.control.met}, epistemic = ${responsibility.conditions.epistemic.met}`);

// Apply Frankfurt case
const frankfurtCase = freeWill.applyFrankfurtCase('vote_001', {
  intervener: 'Black',
  actuallyIntervened: false
});
console.log(`  applyFrankfurtCase:`);
console.log(`    Agent acted on own: ${frankfurtCase.structure.agentActedOnOwn}`);
console.log(`    Verdict: ${frankfurtCase.verdict}`);

// Evaluate positions
const determinism = freeWill.evaluatePosition('hard_determinism');
console.log(`  evaluatePosition (hard_determinism): FW = ${determinism.commitments.freeWill}, MR = ${determinism.commitments.moralResponsibility}`);

const compatibilism = freeWill.evaluatePosition('compatibilism');
console.log(`  evaluatePosition (compatibilism): FW = ${compatibilism.commitments.freeWill}, MR = ${compatibilism.commitments.moralResponsibility}`);

const libertarianism = freeWill.evaluatePosition('libertarianism');
console.log(`  evaluatePosition (libertarianism): FW = ${libertarianism.commitments.freeWill}, MR = ${libertarianism.commitments.moralResponsibility}`);

console.log('  ✓ Free Will Engine OK\n');

// 30C: Practical Reason Engine
console.log('30C: Practical Reason Engine');
const practicalReason = require('./lib/practical-reason-engine.cjs');
practicalReason.init();

// Register an agent
const prAgent = practicalReason.registerAgent('alice_001', {
  name: 'Alice',
  type: 'human',
  ends: ['health', 'happiness'],
  values: ['well-being']
});
console.log(`  registerAgent: ${prAgent.name} (ends: ${prAgent.ends.join(', ')})`);

// Construct practical syllogism (Aristotle)
const syllogism = practicalReason.constructSyllogism('alice_001', {
  desire: 'I want to be healthy',
  belief: 'Walking promotes health',
  action: 'I should walk'
});
console.log(`  constructSyllogism (Aristotle):`);
console.log(`    Desire: ${syllogism.majorPremise}`);
console.log(`    Belief: ${syllogism.minorPremise}`);
console.log(`    Action: ${syllogism.conclusion}`);
console.log(`    Valid: ${syllogism.valid}`);

// Test categorical imperative (Kant)
const ciTest1 = practicalReason.testCategoricalImperative('Make false promises when convenient');
console.log(`  testCategoricalImperative: "${ciTest1.maxim}"`);
console.log(`    Universalizable: ${ciTest1.universalLaw.universalizable}`);
console.log(`    Verdict: ${ciTest1.verdict}`);

const ciTest2 = practicalReason.testCategoricalImperative('Help others in need');
console.log(`  testCategoricalImperative: "${ciTest2.maxim}"`);
console.log(`    Universalizable: ${ciTest2.universalLaw.universalizable}`);
console.log(`    Verdict: ${ciTest2.verdict}`);

// Classify imperatives
const hypothetical = practicalReason.classifyImperative('If you want to lose weight, you should exercise');
console.log(`  classifyImperative: "${hypothetical.imperative.substring(0, 30)}..."`);
console.log(`    Type: ${hypothetical.type}`);

const categorical = practicalReason.classifyImperative('You must always tell the truth');
console.log(`  classifyImperative: "${categorical.imperative}"`);
console.log(`    Type: ${categorical.type}`);

// Analyze akrasia
const akrasia = practicalReason.analyzeAkrasia({
  judgment: 'I should not eat the cake',
  action: 'Eating the cake',
  voluntary: true
});
console.log(`  analyzeAkrasia:`);
console.log(`    Judgment: ${akrasia.judgment}`);
console.log(`    Action: ${akrasia.action}`);
console.log(`    isAkrasia: ${akrasia.isAkrasia}`);
console.log(`    Socratic: ${akrasia.perspectives.socratic.verdict}`);
console.log(`    Aristotelian: ${akrasia.perspectives.aristotelian.verdict}`);

console.log('  ✓ Practical Reason Engine OK\n');

console.log('=== Phase 30 Integration Test PASSED ===\n');
