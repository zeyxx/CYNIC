#!/usr/bin/env node

/**
 * Buddhist Engine - Phase 37A
 * 
 * Buddhist philosophy:
 * - Four Noble Truths and Eightfold Path
 * - Emptiness (sunyata) and dependent origination
 * - No-self (anatman) and impermanence
 * - Schools: Theravada, Mahayana, Vajrayana
 * 
 * φ-bounded: max 61.8% confidence
 */

const fs = require('fs');
const path = require('path');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;
const PHI_INV_3 = 0.236067977499790;

// State
const state = {
  initialized: false,
  teachings: new Map(),
  schools: new Map(),
  concepts: new Map(),
  analyses: [],
  stats: {
    teachingsRegistered: 0,
    schoolsRegistered: 0,
    conceptsRegistered: 0,
    analysesPerformed: 0
  }
};

const STORAGE_DIR = path.join(process.env.HOME || '/tmp', '.cynic', 'buddhist-engine');

/**
 * Initialize Buddhist engine
 */
function init() {
  if (state.initialized) {
    return { status: 'already initialized', phase: '37A' };
  }
  
  try {
    fs.mkdirSync(STORAGE_DIR, { recursive: true });
  } catch (e) {
    // Directory exists
  }
  
  registerTeachings();
  registerSchools();
  registerConcepts();
  
  state.initialized = true;
  return { status: 'initialized', phase: '37A', engine: 'buddhist' };
}

/**
 * Register core Buddhist teachings
 */
function registerTeachings() {
  state.teachings.set('four-noble-truths', {
    name: 'Four Noble Truths',
    sanskrit: 'Catvari Aryasatyani',
    source: 'First sermon at Deer Park',
    truths: [
      {
        number: 1,
        name: 'Dukkha',
        english: 'Suffering/Unsatisfactoriness',
        meaning: 'Life involves suffering, dissatisfaction, and impermanence',
        examples: ['Birth', 'Aging', 'Illness', 'Death', 'Not getting what one wants']
      },
      {
        number: 2,
        name: 'Samudaya',
        english: 'Origin of Suffering',
        meaning: 'Suffering arises from craving and attachment',
        types: ['Craving for sensual pleasures', 'Craving for existence', 'Craving for non-existence']
      },
      {
        number: 3,
        name: 'Nirodha',
        english: 'Cessation of Suffering',
        meaning: 'Suffering can end through elimination of craving',
        goal: 'Nirvana - liberation from the cycle of rebirth'
      },
      {
        number: 4,
        name: 'Magga',
        english: 'Path to Cessation',
        meaning: 'The Eightfold Path leads to the end of suffering',
        path: 'Noble Eightfold Path'
      }
    ],
    significance: 'Foundation of Buddhist teaching',
    strength: PHI_INV
  });
  
  state.teachings.set('eightfold-path', {
    name: 'Noble Eightfold Path',
    sanskrit: 'Arya Astangika Marga',
    categories: {
      wisdom: [
        { name: 'Right View', sanskrit: 'Samma Ditthi', meaning: 'Understanding the Four Noble Truths' },
        { name: 'Right Intention', sanskrit: 'Samma Sankappa', meaning: 'Commitment to ethical and mental self-improvement' }
      ],
      ethics: [
        { name: 'Right Speech', sanskrit: 'Samma Vaca', meaning: 'Avoiding lying, divisive speech, harsh words' },
        { name: 'Right Action', sanskrit: 'Samma Kammanta', meaning: 'Ethical conduct, non-harm' },
        { name: 'Right Livelihood', sanskrit: 'Samma Ajiva', meaning: 'Earning living without harming others' }
      ],
      meditation: [
        { name: 'Right Effort', sanskrit: 'Samma Vayama', meaning: 'Cultivating wholesome states' },
        { name: 'Right Mindfulness', sanskrit: 'Samma Sati', meaning: 'Awareness of body, feelings, mind, phenomena' },
        { name: 'Right Concentration', sanskrit: 'Samma Samadhi', meaning: 'Meditative absorption (jhana)' }
      ]
    },
    nature: 'Not sequential but mutually supporting',
    strength: PHI_INV
  });
  
  state.teachings.set('three-marks', {
    name: 'Three Marks of Existence',
    sanskrit: 'Tilakkhana',
    marks: [
      {
        name: 'Anicca',
        english: 'Impermanence',
        meaning: 'All conditioned phenomena are impermanent',
        implication: 'Attachment to impermanent things causes suffering'
      },
      {
        name: 'Dukkha',
        english: 'Suffering/Unsatisfactoriness',
        meaning: 'Conditioned existence is inherently unsatisfactory',
        implication: 'Even pleasant experiences are tinged with impermanence'
      },
      {
        name: 'Anatta',
        english: 'Non-self',
        meaning: 'There is no permanent, unchanging self',
        implication: 'The "self" is a conventional designation for aggregates'
      }
    ],
    significance: 'Understanding these leads to liberation',
    strength: PHI_INV
  });
  
  state.teachings.set('dependent-origination', {
    name: 'Dependent Origination',
    sanskrit: 'Pratityasamutpada',
    formula: 'When this exists, that comes to be; with the arising of this, that arises',
    twelveLinks: [
      'Ignorance', 'Formations', 'Consciousness', 'Name-and-form',
      'Six sense bases', 'Contact', 'Feeling', 'Craving',
      'Clinging', 'Becoming', 'Birth', 'Aging and death'
    ],
    implication: 'Nothing exists independently; all is interconnected',
    relation: 'Basis for emptiness (sunyata) doctrine',
    strength: PHI_INV
  });
  
  state.stats.teachingsRegistered = state.teachings.size;
}

/**
 * Register Buddhist schools
 */
function registerSchools() {
  state.schools.set('theravada', {
    name: 'Theravada',
    meaning: 'Way of the Elders',
    regions: ['Sri Lanka', 'Thailand', 'Myanmar', 'Cambodia', 'Laos'],
    canon: 'Pali Canon (Tipitaka)',
    emphasis: [
      'Individual liberation (arhat ideal)',
      'Monastic practice',
      'Abhidharma analysis',
      'Vipassana meditation'
    ],
    goal: 'Nibbana through personal practice',
    strength: PHI_INV
  });
  
  state.schools.set('mahayana', {
    name: 'Mahayana',
    meaning: 'Great Vehicle',
    regions: ['China', 'Japan', 'Korea', 'Vietnam', 'Tibet'],
    texts: ['Prajnaparamita Sutras', 'Lotus Sutra', 'Heart Sutra'],
    emphasis: [
      'Bodhisattva ideal (liberation for all)',
      'Emptiness (sunyata)',
      'Buddha-nature',
      'Skillful means (upaya)'
    ],
    subSchools: ['Zen', 'Pure Land', 'Tiantai', 'Huayan'],
    strength: PHI_INV
  });
  
  state.schools.set('vajrayana', {
    name: 'Vajrayana',
    meaning: 'Diamond/Thunderbolt Vehicle',
    regions: ['Tibet', 'Mongolia', 'Nepal', 'Bhutan'],
    texts: ['Tantras', 'Tibetan Book of the Dead'],
    emphasis: [
      'Tantra and ritual',
      'Guru-disciple relationship',
      'Visualization practices',
      'Rapid path to enlightenment'
    ],
    distinctive: ['Mantra', 'Mandala', 'Mudra', 'Deity yoga'],
    strength: PHI_INV_2
  });
  
  state.schools.set('zen', {
    name: 'Zen/Chan',
    meaning: 'Meditation',
    regions: ['Japan', 'China', 'Korea'],
    emphasis: [
      'Direct pointing to mind',
      'Zazen (sitting meditation)',
      'Koans (paradoxical questions)',
      'Sudden enlightenment (satori)'
    ],
    principles: [
      'Not relying on words and letters',
      'Pointing directly to the human mind',
      'Seeing one\'s nature and becoming Buddha'
    ],
    subSchools: ['Rinzai (koan)', 'Soto (shikantaza)'],
    strength: PHI_INV
  });
  
  state.stats.schoolsRegistered = state.schools.size;
}

/**
 * Register key Buddhist concepts
 */
function registerConcepts() {
  state.concepts.set('sunyata', {
    name: 'Sunyata',
    english: 'Emptiness',
    school: 'Mahayana (especially Madhyamaka)',
    philosopher: 'Nagarjuna',
    meaning: 'All phenomena lack inherent existence (svabhava)',
    notMeaning: [
      'Not nihilism (things exist conventionally)',
      'Not nothingness',
      'Not mere absence'
    ],
    twoTruths: {
      conventional: 'Things exist as dependent designations',
      ultimate: 'Things lack inherent, independent existence'
    },
    heartSutra: 'Form is emptiness, emptiness is form',
    strength: PHI_INV
  });
  
  state.concepts.set('anatman', {
    name: 'Anatman/Anatta',
    english: 'Non-self',
    meaning: 'There is no permanent, unchanging self or soul',
    fiveAggregates: {
      name: 'Skandhas',
      components: ['Form', 'Sensation', 'Perception', 'Mental formations', 'Consciousness'],
      point: 'The "self" is just these aggregates, none of which is a self'
    },
    contrast: 'Opposed to Hindu concept of Atman (eternal self)',
    implication: 'Liberation comes from seeing through the illusion of self',
    strength: PHI_INV
  });
  
  state.concepts.set('nirvana', {
    name: 'Nirvana/Nibbana',
    english: 'Extinction, Blowing Out',
    meaning: 'Liberation from the cycle of rebirth (samsara)',
    characteristics: [
      'Cessation of craving and attachment',
      'End of suffering',
      'Beyond description in ordinary terms'
    ],
    types: {
      sopadhisesa: 'Nirvana with remainder (still embodied)',
      anupadhisesa: 'Nirvana without remainder (final liberation)'
    },
    notMeaning: 'Not annihilation, not eternal existence',
    strength: PHI_INV_2
  });
  
  state.concepts.set('karma', {
    name: 'Karma',
    english: 'Action',
    meaning: 'Intentional actions have consequences',
    buddhist: {
      emphasis: 'Intention (cetana) is karma',
      mechanism: 'Actions create tendencies that shape future experience',
      liberation: 'Enlightened action leaves no karmic residue'
    },
    types: ['Bodily', 'Verbal', 'Mental'],
    note: 'Buddhist karma differs from Hindu karma in emphasizing intention',
    strength: PHI_INV
  });
  
  state.concepts.set('bodhisattva', {
    name: 'Bodhisattva',
    english: 'Enlightenment Being',
    meaning: 'One who seeks enlightenment for all sentient beings',
    school: 'Central to Mahayana',
    vows: [
      'To save all sentient beings',
      'To destroy all afflictions',
      'To master all dharmas',
      'To attain Buddhahood'
    ],
    contrast: 'Arhat ideal (personal liberation) in Theravada',
    examples: ['Avalokiteshvara (compassion)', 'Manjushri (wisdom)'],
    strength: PHI_INV
  });
  
  state.stats.conceptsRegistered = state.concepts.size;
}

/**
 * Get a teaching
 */
function getTeaching(teachingId) {
  return state.teachings.get(teachingId) || null;
}

/**
 * Get a school
 */
function getSchool(schoolId) {
  return state.schools.get(schoolId) || null;
}

/**
 * Get a concept
 */
function getConcept(conceptId) {
  return state.concepts.get(conceptId) || null;
}

/**
 * List all teachings
 */
function listTeachings() {
  return Array.from(state.teachings.entries()).map(([id, t]) => ({ id, ...t }));
}

/**
 * List all schools
 */
function listSchools() {
  return Array.from(state.schools.entries()).map(([id, s]) => ({ id, ...s }));
}

/**
 * List all concepts
 */
function listConcepts() {
  return Array.from(state.concepts.entries()).map(([id, c]) => ({ id, ...c }));
}

/**
 * Analyze suffering (dukkha)
 */
function analyzeSuffering(situation) {
  state.stats.analysesPerformed++;
  
  return {
    situation,
    firstTruth: {
      recognition: 'This situation involves dukkha (unsatisfactoriness)',
      forms: ['Physical pain', 'Mental anguish', 'Impermanence of pleasant states']
    },
    secondTruth: {
      question: 'What craving or attachment underlies this suffering?',
      possibleCravings: ['Craving for permanence', 'Craving for control', 'Attachment to outcomes']
    },
    thirdTruth: {
      possibility: 'Suffering can cease through releasing attachment',
      notMeaning: 'Not suppression or denial, but transformation'
    },
    fourthTruth: {
      path: 'The Eightfold Path offers practical guidance',
      immediate: ['Right View: See situation clearly', 'Right Mindfulness: Observe without reaction']
    },
    cynicNote: '*sniff* Buddhism offers diagnosis and prescription. φ-bounded: not a magic cure.',
    confidence: PHI_INV_2
  };
}

/**
 * Compare Buddhist and Western concepts
 */
function compareWithWestern() {
  return {
    comparisons: [
      {
        buddhist: 'Anatman (no-self)',
        western: 'Personal identity debates',
        relation: 'Buddhism denies what Western philosophy debates',
        note: 'Parfit\'s reductionism resembles Buddhist view'
      },
      {
        buddhist: 'Sunyata (emptiness)',
        western: 'Anti-essentialism',
        relation: 'Both deny inherent natures',
        note: 'Nagarjuna anticipates Wittgenstein on essence'
      },
      {
        buddhist: 'Dependent origination',
        western: 'Process philosophy, systems theory',
        relation: 'Both emphasize relationality',
        note: 'Whitehead\'s process metaphysics has parallels'
      },
      {
        buddhist: 'Mindfulness',
        western: 'Phenomenology, attention studies',
        relation: 'Both examine conscious experience closely',
        note: 'Husserl\'s epoché resembles meditation'
      }
    ],
    caution: 'Comparisons can obscure differences; context matters',
    cynicVerdict: '*ears perk* Cross-cultural philosophy is tricky. Respect both similarities and differences.',
    confidence: PHI_INV_2
  };
}

/**
 * Format engine status
 */
function formatStatus() {
  return `
┌─────────────────────────────────────────────────────────┐
│  BUDDHIST ENGINE                         Phase 37A     │
├─────────────────────────────────────────────────────────┤
│  Teachings: ${String(state.stats.teachingsRegistered).padStart(3)}                                     │
│  Schools: ${String(state.stats.schoolsRegistered).padStart(3)}                                       │
│  Concepts: ${String(state.stats.conceptsRegistered).padStart(3)}                                      │
│  Analyses: ${String(state.stats.analysesPerformed).padStart(3)}                                      │
├─────────────────────────────────────────────────────────┤
│  Core Teachings:                                        │
│    - Four Noble Truths                                  │
│    - Eightfold Path                                     │
│    - Three Marks of Existence                           │
│    - Dependent Origination                              │
├─────────────────────────────────────────────────────────┤
│  φ-bounded: max ${(PHI_INV * 100).toFixed(1)}% confidence                      │
│  *head tilt* Emptiness is not nothingness               │
└─────────────────────────────────────────────────────────┘`.trim();
}

/**
 * Get stats
 */
function getStats() {
  return {
    teachings: state.stats.teachingsRegistered,
    schools: state.stats.schoolsRegistered,
    concepts: state.stats.conceptsRegistered,
    analyses: state.stats.analysesPerformed
  };
}

module.exports = {
  init,
  getTeaching,
  getSchool,
  getConcept,
  listTeachings,
  listSchools,
  listConcepts,
  analyzeSuffering,
  compareWithWestern,
  formatStatus,
  getStats,
  PHI,
  PHI_INV,
  PHI_INV_2,
  PHI_INV_3
};
