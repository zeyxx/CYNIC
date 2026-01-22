#!/usr/bin/env node
/**
 * Phase 29 Integration Test: Philosophy of Language
 *
 * Tests:
 * - 29A: Meaning Engine (Frege/Kripke)
 * - 29B: Speech Act Engine (Austin/Searle)
 * - 29C: Truth Engine (theories of truth)
 */

console.log('=== Phase 29 Integration Test ===\n');

// 29A: Meaning Engine
console.log('29A: Meaning Engine');
const meaning = require('./lib/meaning-engine.cjs');
meaning.init();

// Register expressions (Frege's puzzle)
const hesperus = meaning.registerExpression('hesperus', {
  surface: 'Hesperus',
  type: 'proper_name',
  isRigid: true
});
console.log(`  registerExpression: ${hesperus.surface} (rigid: ${hesperus.isRigid})`);

const phosphorus = meaning.registerExpression('phosphorus', {
  surface: 'Phosphorus',
  type: 'proper_name',
  isRigid: true
});
console.log(`  registerExpression: ${phosphorus.surface} (rigid: ${phosphorus.isRigid})`);

// Assign senses (different modes of presentation)
meaning.assignSense('hesperus', { content: 'the evening star', mode: 'visible in evening' });
meaning.assignSense('phosphorus', { content: 'the morning star', mode: 'visible in morning' });
console.log(`  assignSense: Hesperus = "the evening star"`);
console.log(`  assignSense: Phosphorus = "the morning star"`);

// Assign same reference (Venus)
meaning.assignReference('hesperus', { object: 'Venus' });
meaning.assignReference('phosphorus', { object: 'Venus' });
console.log(`  assignReference: Both refer to Venus`);

// Analyze Frege's puzzle
const fregeAnalysis = meaning.analyzeIdentity('hesperus', 'phosphorus');
console.log(`  analyzeIdentity: puzzle = ${fregeAnalysis.puzzle}`);
console.log(`    Same reference: ${fregeAnalysis.sameReference}, Same sense: ${fregeAnalysis.sameSense}`);
if (fregeAnalysis.kripke) {
  console.log(`    Kripke: ${fregeAnalysis.kripke.modal_status} ${fregeAnalysis.kripke.epistemic_status}`);
}

// Necessary a posteriori
const kripkeAnalysis = meaning.analyzeNecessaryAPosteriori('Hesperus is Phosphorus');
console.log(`  analyzeNecessaryAPosteriori: ${kripkeAnalysis.kripkeCategory}`);

console.log('  ✓ Meaning Engine OK\n');

// 29B: Speech Act Engine
console.log('29B: Speech Act Engine');
const speechAct = require('./lib/speech-act-engine.cjs');
speechAct.init();

// Analyze a promise (performative)
const promise = speechAct.analyzeUtterance('promise_001', {
  text: 'I promise to pay you tomorrow',
  speaker: 'Alice'
});
console.log(`  analyzeUtterance: "${promise.text}"`);

// Classify illocutionary force
const classification = speechAct.classifyIllocution('promise_001');
console.log(`  classifyIllocution: ${classification.category} - ${classification.force}`);
console.log(`    Direction of fit: ${classification.direction_of_fit}`);

// Check felicity conditions
const felicity = speechAct.checkFelicity('promise_001', {
  propositional_content: true,
  preparatory: true,
  sincerity: true,
  essential: true
});
console.log(`  checkFelicity: felicitous = ${felicity.felicitous}`);

// Detect indirect speech act
const indirect = speechAct.analyzeUtterance('indirect_001', {
  text: 'Can you pass the salt?',
  speaker: 'Bob'
});
const indirectAnalysis = speechAct.detectIndirectAct('indirect_001');
console.log(`  detectIndirectAct: "${indirect.text}"`);
console.log(`    Literal: ${indirectAnalysis.literalAct}`);
console.log(`    Indirect: ${indirectAnalysis.indirectAct}`);

// Analyze performative
const performativeAnalysis = speechAct.analyzePerformative('promise_001');
console.log(`  analyzePerformative: explicit = ${performativeAnalysis.isExplicit}, verb = ${performativeAnalysis.performativeVerb}`);

console.log('  ✓ Speech Act Engine OK\n');

// 29C: Truth Engine
console.log('29C: Truth Engine');
const truth = require('./lib/truth-engine.cjs');
truth.init();

// Register propositions
const snowWhite = truth.registerProposition('snow_white', {
  content: 'Snow is white',
  bearer: 'proposition',
  domain: 'empirical'
});
console.log(`  registerProposition: "${snowWhite.content}"`);

// Evaluate by correspondence
const corrEval = truth.evaluateByTheory('snow_white', 'correspondence', {
  corresponds: true,
  fact: 'the whiteness of snow',
  confidence: 0.9
});
console.log(`  evaluateByTheory (correspondence): ${corrEval.result} (${(corrEval.confidence * 100).toFixed(1)}%)`);

// Generate T-sentence (Tarski)
const tSentence = truth.generateTSentence('Snow is white');
console.log(`  generateTSentence: ${tSentence.tSentence}`);

// Evaluate by deflationary theory
const defEval = truth.evaluateByTheory('snow_white', 'deflationary', { obtains: 'true' });
console.log(`  evaluateByTheory (deflationary): "${defEval.note}"`);

// Analyze liar paradox
const liar = truth.analyzeLiarParadox();
console.log(`  analyzeLiarParadox: "${liar.statement}"`);
console.log(`    Tarski's response: ${liar.responses.tarski}`);

// Compare theories
const comparison = truth.compareTheories('correspondence', 'deflationary');
console.log(`  compareTheories: ${comparison.theory1.name} vs ${comparison.theory2.name}`);

console.log('  ✓ Truth Engine OK\n');

console.log('=== Phase 29 Integration Test PASSED ===\n');
