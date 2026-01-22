#!/usr/bin/env node

/**
 * Phase 37 Integration Test - Eastern Philosophy
 *
 * Tests:
 * - 37A: Buddhist Engine (four truths, emptiness, no-self)
 * - 37B: Daoist Engine (wu-wei, yin-yang, naturalness)
 * - 37C: Vedanta Engine (Brahman, Atman, maya)
 */

const buddhistEngine = require('./lib/buddhist-engine.cjs');
const daoistEngine = require('./lib/daoist-engine.cjs');
const vedantaEngine = require('./lib/vedanta-engine.cjs');

console.log('═══════════════════════════════════════════════════════════');
console.log('  PHASE 37: EASTERN PHILOSOPHY');
console.log('  *ears perk* Beyond the Western tradition...');
console.log('═══════════════════════════════════════════════════════════\n');

// Initialize all engines
console.log('── INITIALIZATION ─────────────────────────────────────────\n');

const buddhistInit = buddhistEngine.init();
console.log('Buddhist Engine: ' + buddhistInit.status);

const daoistInit = daoistEngine.init();
console.log('Daoist Engine: ' + daoistInit.status);

const vedantaInit = vedantaEngine.init();
console.log('Vedanta Engine: ' + vedantaInit.status);

// ═══════════════════════════════════════════════════════════════════
// 37A: BUDDHIST ENGINE TESTS
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  37A: BUDDHIST ENGINE (Four Truths, Emptiness, No-Self)');
console.log('═══════════════════════════════════════════════════════════\n');

// Four Noble Truths
console.log('── Four Noble Truths ──────────────────────────────────────\n');

const fourTruths = buddhistEngine.getTeaching('four-noble-truths');
fourTruths.truths.forEach(t => {
  console.log(t.number + '. ' + t.name + ' (' + t.english + ')');
  console.log('   ' + t.meaning);
});

// Three Marks
console.log('\n── Three Marks of Existence ───────────────────────────────\n');

const threeMarks = buddhistEngine.getTeaching('three-marks');
threeMarks.marks.forEach(m => {
  console.log(m.name + ' (' + m.english + '): ' + m.meaning);
});

// Schools
console.log('\n── Buddhist Schools ───────────────────────────────────────\n');

const schools = buddhistEngine.listSchools();
schools.forEach(s => {
  console.log(s.name + ' (' + s.meaning + ')');
  console.log('  Regions: ' + s.regions.join(', '));
});

// Emptiness concept
console.log('\n── Sunyata (Emptiness) ────────────────────────────────────\n');

const sunyata = buddhistEngine.getConcept('sunyata');
console.log('Meaning: ' + sunyata.meaning);
console.log('NOT: ' + sunyata.notMeaning.join(', '));
console.log('Heart Sutra: ' + sunyata.heartSutra);

// Suffering analysis
console.log('\n── Suffering Analysis ─────────────────────────────────────\n');

const suffering = buddhistEngine.analyzeSuffering('Loss of a loved one');
console.log('Situation: ' + suffering.situation);
console.log('First Truth: ' + suffering.firstTruth.recognition);
console.log('Second Truth question: ' + suffering.secondTruth.question);
console.log('CYNIC: ' + suffering.cynicNote);

// ═══════════════════════════════════════════════════════════════════
// 37B: DAOIST ENGINE TESTS
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  37B: DAOIST ENGINE (Wu-wei, Yin-Yang, Naturalness)');
console.log('═══════════════════════════════════════════════════════════\n');

// Core concepts
console.log('── Core Concepts ──────────────────────────────────────────\n');

const dao = daoistEngine.getConcept('dao');
console.log('Dao (道): ' + dao.paradox);

const wuwei = daoistEngine.getConcept('wu-wei');
console.log('\nWu-wei (無為): ' + wuwei.meaning[0]);
console.log('  NOT: ' + wuwei.notMeaning[0]);

const yinyang = daoistEngine.getConcept('yin-yang');
console.log('\nYin-Yang (陰陽):');
yinyang.pairs.slice(0, 3).forEach(p => {
  console.log('  ' + p.yin + ' / ' + p.yang);
});

// Key texts
console.log('\n── Key Texts ──────────────────────────────────────────────\n');

const daodejing = daoistEngine.getText('daodejing');
console.log(daodejing.name + ' by ' + daodejing.author);
console.log('Famous passages:');
daodejing.famousPassages.slice(0, 2).forEach(p => console.log('  - ' + p));

const zhuangzi = daoistEngine.getText('zhuangzi');
console.log('\n' + zhuangzi.name + ':');
zhuangzi.famousStories.slice(0, 2).forEach(s => {
  console.log('  - ' + s.name + ': ' + s.theme);
});

// Wu-wei analysis
console.log('\n── Wu-wei Analysis ────────────────────────────────────────\n');

const wuweiAnalysis = daoistEngine.analyzeWithWuWei('Struggling to complete a difficult project');
console.log('Wu-wei question: ' + wuweiAnalysis.daoistPerspective.wuWei.question);
console.log('Guidance: ' + wuweiAnalysis.daoistPerspective.wuWei.guidance);
console.log('CYNIC: ' + wuweiAnalysis.cynicNote);

// ═══════════════════════════════════════════════════════════════════
// 37C: VEDANTA ENGINE TESTS
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  37C: VEDANTA ENGINE (Brahman, Atman, Maya)');
console.log('═══════════════════════════════════════════════════════════\n');

// Core concepts
console.log('── Core Concepts ──────────────────────────────────────────\n');

const brahman = vedantaEngine.getConcept('brahman');
console.log('Brahman: ' + brahman.meaning);
console.log('  Sat-Chit-Ananda: Being, Consciousness, Bliss');

const atman = vedantaEngine.getConcept('atman');
console.log('\nAtman: ' + atman.meaning);
console.log('  Mahavakya: ' + atman.mahavakyas[0].sanskrit + ' - ' + atman.mahavakyas[0].meaning);

const maya = vedantaEngine.getConcept('maya');
console.log('\nMaya: ' + maya.meaning);
console.log('  Analogy: ' + maya.analogy);

// Schools
console.log('\n── Vedanta Schools ────────────────────────────────────────\n');

const vedantaSchools = vedantaEngine.listSchools();
vedantaSchools.forEach(s => {
  console.log(s.name + ' (' + s.founder + ')');
  console.log('  ' + s.coreTeaching);
});

// Compare schools
console.log('\n── School Comparison ──────────────────────────────────────\n');

const comparison = vedantaEngine.compareSchools();
console.log('Question: ' + comparison.question);
comparison.positions.forEach(p => {
  console.log('  ' + p.school + ': ' + p.formula);
});

// Self-inquiry
console.log('\n── Self-Inquiry Analysis ──────────────────────────────────\n');

const selfInquiry = vedantaEngine.analyzeSelfInquiry('Who am I?');
console.log('Method: ' + selfInquiry.vedanticApproach.method);
selfInquiry.vedanticApproach.process.slice(0, 2).forEach(p => console.log('  ' + p));
console.log('Mahavakya: ' + selfInquiry.mahavakya.teaching + ' - ' + selfInquiry.mahavakya.meaning);

// ═══════════════════════════════════════════════════════════════════
// STATUS DISPLAYS
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  ENGINE STATUS');
console.log('═══════════════════════════════════════════════════════════\n');

console.log(buddhistEngine.formatStatus());
console.log('\n');
console.log(daoistEngine.formatStatus());
console.log('\n');
console.log(vedantaEngine.formatStatus());

// ═══════════════════════════════════════════════════════════════════
// SUMMARY
// ═══════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  PHASE 37 SUMMARY');
console.log('═══════════════════════════════════════════════════════════\n');

const buddhistStats = buddhistEngine.getStats();
const daoistStats = daoistEngine.getStats();
const vedantaStats = vedantaEngine.getStats();

console.log('37A Buddhist Engine:');
console.log('  Teachings: ' + buddhistStats.teachings);
console.log('  Schools: ' + buddhistStats.schools);
console.log('  Concepts: ' + buddhistStats.concepts);

console.log('\n37B Daoist Engine:');
console.log('  Concepts: ' + daoistStats.concepts);
console.log('  Texts: ' + daoistStats.texts);
console.log('  Practices: ' + daoistStats.practices);

console.log('\n37C Vedanta Engine:');
console.log('  Concepts: ' + vedantaStats.concepts);
console.log('  Schools: ' + vedantaStats.schools);
console.log('  Texts: ' + vedantaStats.texts);

console.log('\n═══════════════════════════════════════════════════════════');
console.log('  *tail wag* PHASE 37 COMPLETE');
console.log('  Eastern Philosophy operational.');
console.log('  Three great traditions, one φ-bounded system.');
console.log('  φ-bounded confidence: max 61.8%');
console.log('═══════════════════════════════════════════════════════════\n');
