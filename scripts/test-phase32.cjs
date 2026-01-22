#!/usr/bin/env node

/**
 * Phase 32 Integration Test - Philosophy of Science
 *
 * Tests:
 * - 32A: Scientific Method Engine (Popper/Kuhn)
 * - 32B: Explanation Engine (DN model, causation)
 * - 32C: Theory Change Engine (paradigm shifts, Lakatos)
 */

const scientificMethodEngine = require('./lib/scientific-method-engine.cjs');
const explanationEngine = require('./lib/explanation-engine.cjs');
const theoryChangeEngine = require('./lib/theory-change-engine.cjs');

console.log('═══════════════════════════════════════════════════════════');
console.log('  PHASE 32: PHILOSOPHY OF SCIENCE');
console.log('  *tail wag* Testing scientific method, explanation, theory change...');
console.log('═══════════════════════════════════════════════════════════\n');

// Initialize all engines
console.log('── INITIALIZATION ─────────────────────────────────────────\n');

const methodInit = scientificMethodEngine.init();
console.log('Scientific Method Engine: ' + methodInit.status);

const explainInit = explanationEngine.init();
console.log('Explanation Engine: ' + explainInit.status);

const changeInit = theoryChangeEngine.init();
console.log('Theory Change Engine: ' + changeInit.status);

// ═══════════════════════════════════════════════════════════════════
// 32A: SCIENTIFIC METHOD ENGINE TESTS
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  32A: SCIENTIFIC METHOD ENGINE (Popper/Kuhn)');
console.log('═══════════════════════════════════════════════════════════\n');

// Register theories
console.log('── Registering Theories ───────────────────────────────────\n');

const relativity = scientificMethodEngine.registerTheory('general-relativity', {
  name: 'General Relativity',
  description: 'Einstein theory of gravity as spacetime curvature',
  falsifiable: true,
  falsificationConditions: ['Light does not bend near massive objects'],
  predictions: [
    { content: 'Light bends near sun', testable: true },
    { content: 'Gravitational time dilation', testable: true },
    { content: 'Gravitational waves', testable: true }
  ],
  paradigm: 'relativistic'
});

console.log('Registered: ' + relativity.name);
console.log('  Falsifiable: ' + relativity.falsifiable);
console.log('  Predictions: ' + relativity.predictions.length);

const psychoanalysis = scientificMethodEngine.registerTheory('psychoanalysis', {
  name: 'Freudian Psychoanalysis',
  description: 'Theory of unconscious mind',
  falsifiable: false,
  falsificationConditions: [],
  predictions: [
    { content: 'Behavior explained by unconscious', testable: false }
  ]
});

console.log('Registered: ' + psychoanalysis.name);
console.log('  Falsifiable: ' + psychoanalysis.falsifiable);

// Assess falsifiability
console.log('\n── Falsifiability Assessment (Popper) ─────────────────────\n');

const relFalsif = scientificMethodEngine.assessFalsifiability('general-relativity');
console.log('General Relativity: ' + relFalsif.popperVerdict);
console.log('  Scientific: ' + relFalsif.scientific);

const psyFalsif = scientificMethodEngine.assessFalsifiability('psychoanalysis');
console.log('Psychoanalysis: ' + psyFalsif.popperVerdict);
console.log('  Scientific: ' + psyFalsif.scientific);

// Record tests
console.log('\n── Recording Tests ────────────────────────────────────────\n');

const test1 = scientificMethodEngine.recordTest('general-relativity', {
  description: '1919 Eddington eclipse expedition',
  prediction: 'Light bends near sun',
  observation: 'Light observed to bend',
  severity: 'high',
  riskLevel: 'high',
  result: 'passed',
  novel: true
});

console.log('Test: ' + test1.description);
console.log('  Result: ' + test1.result);
console.log('  Severe test: ' + test1.analysis.severeTest);

// Evaluate corroboration
console.log('\n── Corroboration Evaluation ───────────────────────────────\n');

const corrob = scientificMethodEngine.evaluateCorroboration('general-relativity');
console.log('General Relativity corroboration:');
console.log('  Level: ' + corrob.corroborationLevel);
console.log('  Tests passed: ' + corrob.testSummary.passed);
console.log('  Popper caveat: ' + corrob.popperCaveat.substring(0, 50) + '...');

// Analyze paradigm (Kuhn)
console.log('\n── Paradigm Analysis (Kuhn) ───────────────────────────────\n');

const paradigmAnalysis = scientificMethodEngine.analyzeParadigm({
  name: 'Quantum Mechanics',
  field: 'Physics',
  state: 'normal',
  sharedFramework: true,
  puzzleSolving: true,
  textbookConsensus: true,
  cumulativeProgress: true,
  anomalies: ['Measurement problem', 'Quantum gravity'],
  philosophicalDebates: true
});

console.log('Paradigm: ' + paradigmAnalysis.paradigmName);
console.log('  Diagnosis: ' + paradigmAnalysis.diagnosis);
console.log('  Kuhn analysis: ' + paradigmAnalysis.kuhnAnalysis);

// Compare Popper and Kuhn
console.log('\n── Popper vs Kuhn ─────────────────────────────────────────\n');

const popperKuhn = scientificMethodEngine.comparePopperKuhn();
console.log('Key difference - How does science progress?');
console.log('  Popper: ' + popperKuhn.keyDifferences[0].popper);
console.log('  Kuhn: ' + popperKuhn.keyDifferences[0].kuhn);

// ═══════════════════════════════════════════════════════════════════
// 32B: EXPLANATION ENGINE TESTS
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  32B: EXPLANATION ENGINE (DN Model, Causation)');
console.log('═══════════════════════════════════════════════════════════\n');

// Create DN explanation
console.log('── DN Explanation ─────────────────────────────────────────\n');

const dnExpl = explanationEngine.createDNExplanation('metal-expansion', {
  laws: ['All metals expand when heated'],
  conditions: ['This rod is metal', 'This rod was heated'],
  explanandum: 'This rod expanded',
  deductivelyValid: true,
  empiricalContent: true
});

console.log('DN Explanation: ' + dnExpl.id);
console.log('  Laws: ' + dnExpl.laws.length);
console.log('  Conditions: ' + dnExpl.conditions.length);
console.log('  Has laws: ' + dnExpl.validity.hasLaws);

// Evaluate DN explanation
console.log('\n── DN Evaluation ──────────────────────────────────────────\n');

const dnEval = explanationEngine.evaluateDNExplanation('metal-expansion');
console.log('Evaluation: ' + dnEval.assessment);
console.log('  Valid DN: ' + dnEval.validDN);
console.log('  Known problems: Asymmetry, Irrelevance, Accidents');

// Analyze causal claim
console.log('\n── Causal Claim Analysis ──────────────────────────────────\n');

const causalAnalysis = explanationEngine.analyzeCausalClaim({
  cause: 'Smoking',
  effect: 'Lung cancer',
  constantConjunction: true,
  counterfactualDependence: true,
  interventionWorks: true,
  causalProcess: true
});

console.log('Claim: ' + causalAnalysis.claim);
console.log('  Verdict: ' + causalAnalysis.causalClaim);
console.log('  Theories supporting:');
for (const [key, theory] of Object.entries(causalAnalysis.theories)) {
  if (theory.verdict === 'SUPPORTED') {
    console.log('    - ' + theory.name);
  }
}

// Compare explanation models
console.log('\n── Explanation Models Comparison ──────────────────────────\n');

const modelComp = explanationEngine.compareExplanationModels();
console.log('Models:');
console.log('  DN: ' + modelComp.dnModel.strength + ' / ' + modelComp.dnModel.weakness);
console.log('  Causal: ' + modelComp.causalMechanical.strength + ' / ' + modelComp.causalMechanical.weakness);
console.log('  Synthesis: ' + modelComp.synthesis.pluralism);

// ═══════════════════════════════════════════════════════════════════
// 32C: THEORY CHANGE ENGINE TESTS
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  32C: THEORY CHANGE ENGINE (Lakatos, Paradigm Shifts)');
console.log('═══════════════════════════════════════════════════════════\n');

// Create research programme
console.log('── Creating Research Programme (Lakatos) ──────────────────\n');

const newtonian = theoryChangeEngine.createResearchProgramme('newtonian', {
  name: 'Newtonian Mechanics',
  field: 'Physics',
  hardCore: ['Absolute space', 'Absolute time', 'Three laws of motion', 'Universal gravitation'],
  protectiveBelt: ['Perturbation calculations', 'Celestial mechanics'],
  positiveHeuristic: ['Apply to new domains', 'Increase precision'],
  novelPredictions: ['Return of Halley comet', 'Existence of Neptune', 'Precession of equinoxes'],
  confirmedPredictions: ['Halley comet', 'Neptune discovered'],
  anomalies: ['Mercury perihelion precession'],
  adHocModifications: ['Vulcan hypothesis']
});

console.log('Programme: ' + newtonian.name);
console.log('  Hard core: ' + newtonian.hardCore.length + ' assumptions');
console.log('  Novel predictions: ' + newtonian.novelPredictions.length);
console.log('  Confirmed: ' + newtonian.confirmedPredictions.length);

// Evaluate progressiveness
console.log('\n── Progressiveness Evaluation ─────────────────────────────\n');

const progEval = theoryChangeEngine.evaluateProgressiveness('newtonian');
console.log('Programme status: ' + progEval.status);
console.log('  Theoretical: ' + progEval.theoreticalProgress.assessment);
console.log('  Empirical: ' + progEval.empiricalProgress.assessment);
console.log('  Lakatos verdict: ' + progEval.lakatosVerdict);

// Record theory succession
console.log('\n── Theory Succession ──────────────────────────────────────\n');

const succession = theoryChangeEngine.recordSuccession({
  predecessorName: 'Newtonian Mechanics',
  predecessorProgramme: 'newtonian',
  successorName: 'General Relativity',
  hardCoreChanged: true,
  ontologyChanged: true,
  limitCase: true,
  anomalies: ['Mercury perihelion'],
  reasons: ['Explained Mercury anomaly', 'Predicted light bending']
});

console.log('Succession: ' + succession.predecessor.name + ' → ' + succession.successor.name);
console.log('  Change type: ' + succession.changeType);
console.log('  Hard core changed: ' + succession.analysis.hardCoreChanged);
console.log('  Limit case: ' + succession.correspondence.limitCase);

// Analyze paradigm shift
console.log('\n── Paradigm Shift Analysis ────────────────────────────────\n');

const shiftAnalysis = theoryChangeEngine.analyzeParadigmShift({
  description: 'Copernican Revolution',
  field: 'Astronomy',
  oldParadigm: 'Ptolemaic geocentric',
  newParadigm: 'Copernican heliocentric',
  crisis: true,
  anomalies: ['Retrograde motion complexity', 'Stellar parallax'],
  competingViews: true,
  gestaltShift: true,
  incommensurability: true,
  ontologyChange: true,
  methodChange: true
});

console.log('Analysis: ' + shiftAnalysis.description);
console.log('  Is paradigm shift: ' + shiftAnalysis.isParadigmShift);
console.log('  Classification: ' + shiftAnalysis.classification);
console.log('  Kuhnian score: ' + shiftAnalysis.kuhnianScore + '/5');

// Lakatos summary
console.log('\n── Lakatos Summary ────────────────────────────────────────\n');

const lakatosSummary = theoryChangeEngine.getLakatosSummary();
console.log('Key ideas:');
lakatosSummary.keyIdeas.slice(0, 2).forEach(idea => {
  console.log('  - ' + idea);
});

// ═══════════════════════════════════════════════════════════════════
// STATUS DISPLAYS
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  ENGINE STATUS');
console.log('═══════════════════════════════════════════════════════════\n');

console.log(scientificMethodEngine.formatStatus());
console.log('\n');
console.log(explanationEngine.formatStatus());
console.log('\n');
console.log(theoryChangeEngine.formatStatus());

// ═══════════════════════════════════════════════════════════════════
// SUMMARY
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  PHASE 32 SUMMARY');
console.log('═══════════════════════════════════════════════════════════\n');

const methodStats = scientificMethodEngine.getStats();
const explainStats = explanationEngine.getStats();
const changeStats = theoryChangeEngine.getStats();

console.log('32A Scientific Method Engine:');
console.log('  Theories: ' + methodStats.theories);
console.log('  Tests: ' + methodStats.tests);
console.log('  Demarcations: ' + methodStats.demarcations);

console.log('\n32B Explanation Engine:');
console.log('  Explanations: ' + explainStats.explanations);
console.log('  Causal claims: ' + explainStats.causalClaims);

console.log('\n32C Theory Change Engine:');
console.log('  Programmes: ' + changeStats.programmes);
console.log('  Successions: ' + changeStats.successions);
console.log('  Analyses: ' + changeStats.analyses);

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  *tail wag* PHASE 32 COMPLETE');
console.log('  Philosophy of Science operational.');
console.log('  φ-bounded confidence: max 61.8%');
console.log('═══════════════════════════════════════════════════════════\n');
