#!/usr/bin/env node

/**
 * Phase 45 Integration Test - Cognitive Philosophy
 *
 * Tests:
 * - 45A: Embodied Cognition Engine (4E cognition, enactivism, extended mind)
 * - 45B: Philosophy of Perception Engine (direct realism, representationalism)
 * - 45C: Philosophy of Emotion Engine (feeling theories, cognitivism, affect)
 *
 * φ-bounded: max 61.8% confidence
 */

const embodiedEngine = require('./lib/embodied-cognition-engine.cjs');
const perceptionEngine = require('./lib/philosophy-of-perception-engine.cjs');
const emotionEngine = require('./lib/philosophy-of-emotion-engine.cjs');

console.log('═══════════════════════════════════════════════════════════');
console.log('  PHASE 45: COGNITIVE PHILOSOPHY');
console.log('  *sniff* Body, perception, emotion - the lived mind');
console.log('═══════════════════════════════════════════════════════════\n');

// Initialize all engines
console.log('── INITIALIZATION ─────────────────────────────────────────\n');

const embodiedInit = embodiedEngine.init();
console.log('Embodied Cognition Engine: ' + embodiedInit.status);

const perceptionInit = perceptionEngine.init();
console.log('Philosophy of Perception Engine: ' + perceptionInit.status);

const emotionInit = emotionEngine.init();
console.log('Philosophy of Emotion Engine: ' + emotionInit.status);

// ═══════════════════════════════════════════════════════════════════
// 45A: EMBODIED COGNITION ENGINE TESTS
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  45A: EMBODIED COGNITION');
console.log('═══════════════════════════════════════════════════════════\n');

// Paradigms
console.log('── Cognitive Paradigms ────────────────────────────────────\n');

const paradigms = embodiedEngine.listParadigms();
paradigms.forEach(p => {
  console.log(p.name + ':');
  console.log('  ' + p.thesis);
});

// Extended Mind
console.log('\n── Extended Mind ──────────────────────────────────────────\n');

const extended = embodiedEngine.getParadigm('extended');
console.log('Thesis: ' + extended.thesis);
console.log('Slogan: ' + extended.slogan);
console.log('Parity Principle: ' + extended.parityPrinciple);
console.log('Otto\'s Notebook:');
console.log('  Scenario: ' + extended.ottosNotebook.scenario);
console.log('  Conclusion: ' + extended.ottosNotebook.conclusion);

// Enactivism
console.log('\n── Enactivism ─────────────────────────────────────────────\n');

const enactive = embodiedEngine.getParadigm('enactive');
console.log('Founders: ' + enactive.founders.join(', '));
console.log('Thesis: ' + enactive.thesis);
console.log('Key Ideas:');
Object.entries(enactive.keyIdeas).slice(0, 3).forEach(([k, v]) => {
  console.log('  ' + k + ': ' + v);
});

// Key Thinkers
console.log('\n── Key Embodied Cognition Thinkers ────────────────────────\n');

const clark = embodiedEngine.getThinker('clark');
console.log(clark.name + ' (' + clark.dates + '):');
console.log('  Extended: ' + clark.contributions.extended);
console.log('  Cyborg: ' + clark.contributions.cyborg);

const varela = embodiedEngine.getThinker('varela');
console.log('\n' + varela.name + ' (' + varela.dates + '):');
console.log('  Enactivism: ' + varela.contributions.enactivism);
console.log('  Autopoiesis: ' + varela.contributions.autopoiesis);

// Concepts
console.log('\n── Key Concepts ───────────────────────────────────────────\n');

const affordances = embodiedEngine.getConcept('affordances');
console.log(affordances.name + ':');
console.log('  Definition: ' + affordances.definition);
console.log('  Examples:');
Object.entries(affordances.examples).slice(0, 3).forEach(([k, v]) => {
  console.log('    ' + k + ': ' + v);
});

const bodySchema = embodiedEngine.getConcept('body-schema');
console.log('\n' + bodySchema.name + ':');
console.log('  Schema: ' + bodySchema.distinction.schema.definition);
console.log('  Image: ' + bodySchema.distinction.image.definition);

// Debates
console.log('\n── Key Debates ────────────────────────────────────────────\n');

const extVsEmb = embodiedEngine.getDebate('extended-vs-embedded');
console.log(extVsEmb.name + ':');
console.log('  Question: ' + extVsEmb.question);
console.log('  Extended: ' + extVsEmb.positions.extended);
console.log('  Embedded: ' + extVsEmb.positions.embedded);

// Embodied Analysis
console.log('\n── Embodied Cognition Analysis ────────────────────────────\n');

const embodiedAnalysis = embodiedEngine.analyzeEmbodied('memory');
console.log('Topic: ' + embodiedAnalysis.topic);
console.log('Perspectives:');
Object.entries(embodiedAnalysis.perspectives).forEach(([k, v]) => {
  console.log('  ' + k + ': ' + v);
});
console.log('CYNIC: ' + embodiedAnalysis.cynicNote);

// ═══════════════════════════════════════════════════════════════════
// 45B: PHILOSOPHY OF PERCEPTION ENGINE TESTS
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  45B: PHILOSOPHY OF PERCEPTION');
console.log('═══════════════════════════════════════════════════════════\n');

// Theories
console.log('── Theories of Perception ─────────────────────────────────\n');

const perceptionTheories = perceptionEngine.listTheories();
perceptionTheories.slice(0, 6).forEach(t => {
  console.log(t.name + ':');
  console.log('  ' + t.thesis);
});

// Direct Realism
console.log('\n── Direct Realism ─────────────────────────────────────────\n');

const directRealism = perceptionEngine.getTheory('direct-realism');
console.log('Thesis: ' + directRealism.thesis);
console.log('Slogan: ' + directRealism.slogan);
console.log('Claims:');
Object.entries(directRealism.claims).forEach(([k, v]) => {
  console.log('  ' + k + ': ' + v);
});
console.log('Challenge: ' + directRealism.challenge);

// Disjunctivism
console.log('\n── Disjunctivism ──────────────────────────────────────────\n');

const disjunctivism = perceptionEngine.getTheory('disjunctivism');
console.log('Thesis: ' + disjunctivism.thesis);
console.log('Claims:');
Object.entries(disjunctivism.claims).slice(0, 2).forEach(([k, v]) => {
  console.log('  ' + k + ': ' + v);
});
console.log('Motivation: ' + disjunctivism.motivation);

// Key Thinkers
console.log('\n── Key Perception Philosophers ────────────────────────────\n');

const mp = perceptionEngine.getThinker('merleau-ponty');
console.log(mp.name + ' (' + mp.dates + '):');
console.log('  Lived Body: ' + mp.contributions.livedBody);
console.log('  Motor Intentionality: ' + mp.contributions.motorIntentionality);

const gibson = perceptionEngine.getThinker('gibson');
console.log('\n' + gibson.name + ' (' + gibson.dates + '):');
console.log('  Affordances: ' + gibson.contributions.affordances);
console.log('  Direct: ' + gibson.contributions.direct);

// Concepts
console.log('\n── Key Concepts ───────────────────────────────────────────\n');

const qualia = perceptionEngine.getConcept('qualia');
console.log(qualia.name + ':');
console.log('  Definition: ' + qualia.definition);
console.log('  Properties:');
Object.entries(qualia.properties).slice(0, 3).forEach(([k, v]) => {
  console.log('    ' + k + ': ' + v);
});

const transparency = perceptionEngine.getConcept('transparency');
console.log('\n' + transparency.name + ':');
console.log('  Thesis: ' + transparency.thesis);

// Arguments
console.log('\n── Key Arguments ──────────────────────────────────────────\n');

const argIllusion = perceptionEngine.getArgument('illusion');
console.log(argIllusion.name + ':');
console.log('  Structure:');
console.log('    P1: ' + argIllusion.structure.p1);
console.log('    C: ' + argIllusion.structure.c);
console.log('  Supports: ' + argIllusion.supports);

// Perception Analysis
console.log('\n── Perception Analysis ────────────────────────────────────\n');

const perceptionAnalysis = perceptionEngine.analyzePerception('color perception');
console.log('Topic: ' + perceptionAnalysis.topic);
console.log('Perspectives:');
Object.entries(perceptionAnalysis.perspectives).slice(0, 3).forEach(([k, v]) => {
  console.log('  ' + k + ': ' + v);
});
console.log('CYNIC: ' + perceptionAnalysis.cynicNote);

// ═══════════════════════════════════════════════════════════════════
// 45C: PHILOSOPHY OF EMOTION ENGINE TESTS
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  45C: PHILOSOPHY OF EMOTION');
console.log('═══════════════════════════════════════════════════════════\n');

// Theories
console.log('── Theories of Emotion ────────────────────────────────────\n');

const emotionTheories = emotionEngine.listTheories();
emotionTheories.forEach(t => {
  console.log(t.name + ':');
  console.log('  ' + t.thesis);
});

// James-Lange
console.log('\n── Feeling Theory (James-Lange) ───────────────────────────\n');

const feelingTheory = emotionEngine.getTheory('feeling-theory');
console.log('Thesis: ' + feelingTheory.thesis);
console.log('James\'s Quote: ' + feelingTheory.jamesQuote);
console.log('Claims:');
Object.entries(feelingTheory.claims).slice(0, 2).forEach(([k, v]) => {
  console.log('  ' + k + ': ' + v);
});

// Cognitivism
console.log('\n── Cognitive Theory ───────────────────────────────────────\n');

const cognitivism = emotionEngine.getTheory('cognitivism');
console.log('Thesis: ' + cognitivism.thesis);
console.log('Nussbaum: ' + cognitivism.nussbaum);
console.log('Solomon: ' + cognitivism.solomon);

// Key Thinkers
console.log('\n── Key Emotion Philosophers ───────────────────────────────\n');

const nussbaum = emotionEngine.getThinker('nussbaum');
console.log(nussbaum.name + ' (' + nussbaum.dates + '):');
console.log('  Eudaimonistic: ' + nussbaum.contributions.eudaimonistic);
console.log('  Intelligence: ' + nussbaum.contributions.intelligence);

const damasio = emotionEngine.getThinker('damasio');
console.log('\n' + damasio.name + ' (' + damasio.dates + '):');
console.log('  Somatic: ' + damasio.contributions.somatic);
console.log('  Nested: ' + damasio.contributions.nested);

// Emotions
console.log('\n── Specific Emotions ──────────────────────────────────────\n');

const emotions = emotionEngine.listEmotions();
emotions.slice(0, 4).forEach(e => {
  console.log(e.name + ':');
  console.log('  Object: ' + e.formalObject);
  console.log('  Action: ' + e.actionTendency);
});

// Concepts
console.log('\n── Key Concepts ───────────────────────────────────────────\n');

const appraisal = emotionEngine.getConcept('appraisal');
console.log(appraisal.name + ':');
console.log('  Definition: ' + appraisal.definition);
console.log('  Dimensions:');
Object.entries(appraisal.dimensions).slice(0, 3).forEach(([k, v]) => {
  console.log('    ' + k + ': ' + v);
});

const actionTendencies = emotionEngine.getConcept('action-tendencies');
console.log('\n' + actionTendencies.name + ':');
console.log('  Definition: ' + actionTendencies.definition);
console.log('  Frijda: ' + actionTendencies.frijda);

// Debates
console.log('\n── Key Debates ────────────────────────────────────────────\n');

const emotionReason = emotionEngine.getDebate('emotion-and-reason');
console.log(emotionReason.name + ':');
console.log('  Question: ' + emotionReason.question);
console.log('  Positions:');
Object.entries(emotionReason.positions).forEach(([k, v]) => {
  console.log('    ' + k + ': ' + v);
});
console.log('  Damasio: ' + emotionReason.damasio);

// Emotion Analysis
console.log('\n── Emotion Analysis ───────────────────────────────────────\n');

const fearAnalysis = emotionEngine.analyzeEmotion('fear');
console.log('Emotion: ' + fearAnalysis.emotion);
console.log('Formal Object: ' + fearAnalysis.formalObject);
console.log('Appraisal: ' + fearAnalysis.appraisal);
console.log('Action Tendency: ' + fearAnalysis.actionTendency);
console.log('CYNIC: ' + fearAnalysis.cynicNote);

// ═══════════════════════════════════════════════════════════════════
// STATUS DISPLAYS
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  ENGINE STATUS');
console.log('═══════════════════════════════════════════════════════════\n');

console.log(embodiedEngine.formatStatus());
console.log('\n');
console.log(perceptionEngine.formatStatus());
console.log('\n');
console.log(emotionEngine.formatStatus());

// ═══════════════════════════════════════════════════════════════════
// FINAL SUMMARY
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  PHASE 45 SUMMARY');
console.log('═══════════════════════════════════════════════════════════\n');

const embodiedStats = embodiedEngine.getStats();
const perceptionStats = perceptionEngine.getStats();
const emotionStats = emotionEngine.getStats();

console.log('45A Embodied Cognition Engine:');
console.log('  Paradigms: ' + embodiedStats.paradigms);
console.log('  Thinkers: ' + embodiedStats.thinkers);
console.log('  Concepts: ' + embodiedStats.concepts);
console.log('  Debates: ' + embodiedStats.debates);
console.log('  Experiments: ' + embodiedStats.experiments);

console.log('\n45B Philosophy of Perception Engine:');
console.log('  Theories: ' + perceptionStats.theories);
console.log('  Thinkers: ' + perceptionStats.thinkers);
console.log('  Concepts: ' + perceptionStats.concepts);
console.log('  Arguments: ' + perceptionStats.arguments);
console.log('  Phenomena: ' + perceptionStats.phenomena);

console.log('\n45C Philosophy of Emotion Engine:');
console.log('  Theories: ' + emotionStats.theories);
console.log('  Thinkers: ' + emotionStats.thinkers);
console.log('  Concepts: ' + emotionStats.concepts);
console.log('  Emotions: ' + emotionStats.emotions);
console.log('  Debates: ' + emotionStats.debates);

console.log('\n═══════════════════════════════════════════════════════════');
console.log('');
console.log('   ╔═══════════════════════════════════════════════════╗');
console.log('   ║                                                   ║');
console.log('   ║            *tail wag*                             ║');
console.log('   ║                                                   ║');
console.log('   ║   PHASE 45 COMPLETE - COGNITIVE PHILOSOPHY        ║');
console.log('   ║                                                   ║');
console.log('   ║   Embodied: 4E cognition, extended mind           ║');
console.log('   ║   Perception: Direct realism, qualia, phenomena   ║');
console.log('   ║   Emotion: Feeling, cognition, affect             ║');
console.log('   ║                                                   ║');
console.log('   ║   φ-bounded at 61.8% max confidence.              ║');
console.log('   ║                                                   ║');
console.log('   ║   *sniff* The mind is not a ghost in a machine.   ║');
console.log('   ║   The body thinks. Emotions reveal what we care.  ║');
console.log('   ║   Perception is action, not passive reception.    ║');
console.log('   ║                                                   ║');
console.log('   ╚═══════════════════════════════════════════════════╝');
console.log('');
console.log('═══════════════════════════════════════════════════════════\n');
