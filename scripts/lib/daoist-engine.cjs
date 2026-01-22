#!/usr/bin/env node

/**
 * Daoist Engine - Phase 37B
 * 
 * Daoist philosophy:
 * - Dao (the Way) and De (virtue/power)
 * - Wu-wei (non-action, effortless action)
 * - Yin-yang and naturalness (ziran)
 * - Key texts and schools
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
  concepts: new Map(),
  texts: new Map(),
  practices: new Map(),
  analyses: [],
  stats: {
    conceptsRegistered: 0,
    textsRegistered: 0,
    practicesRegistered: 0,
    analysesPerformed: 0
  }
};

const STORAGE_DIR = path.join(process.env.HOME || '/tmp', '.cynic', 'daoist-engine');

/**
 * Initialize Daoist engine
 */
function init() {
  if (state.initialized) {
    return { status: 'already initialized', phase: '37B' };
  }
  
  try {
    fs.mkdirSync(STORAGE_DIR, { recursive: true });
  } catch (e) {
    // Directory exists
  }
  
  registerConcepts();
  registerTexts();
  registerPractices();
  
  state.initialized = true;
  return { status: 'initialized', phase: '37B', engine: 'daoist' };
}

/**
 * Register core Daoist concepts
 */
function registerConcepts() {
  state.concepts.set('dao', {
    name: 'Dao (Tao)',
    chinese: '道',
    meaning: 'The Way',
    aspects: {
      cosmological: 'The source and pattern of all things',
      metaphysical: 'The nameless, formless ultimate reality',
      ethical: 'The way one should live',
      natural: 'The way things naturally are'
    },
    paradox: 'The Dao that can be spoken is not the eternal Dao',
    daodejing: 'Chapter 1: The Dao that can be told is not the eternal Dao',
    characteristics: [
      'Nameless and formless',
      'Prior to all distinctions',
      'Source of yin and yang',
      'Present in all things'
    ],
    strength: PHI_INV
  });
  
  state.concepts.set('de', {
    name: 'De (Te)',
    chinese: '德',
    meaning: 'Virtue, Power, Potency',
    aspects: {
      cosmic: 'The power/virtue received from Dao',
      individual: 'One\'s inherent nature and capacity',
      ethical: 'Virtue that arises naturally, not forced'
    },
    relation: 'De is the manifestation of Dao in particular things',
    contrast: 'Unlike Confucian virtue, Daoist de is natural, not cultivated',
    strength: PHI_INV
  });
  
  state.concepts.set('wu-wei', {
    name: 'Wu-wei',
    chinese: '無為',
    meaning: 'Non-action, Effortless Action',
    notMeaning: [
      'Not passivity or laziness',
      'Not doing nothing',
      'Not withdrawal from world'
    ],
    meaning: [
      'Action without forcing',
      'Going with the grain',
      'Effortless effectiveness',
      'Acting in harmony with natural patterns'
    ],
    examples: [
      'Water flowing around obstacles',
      'The uncarved block',
      'The skilled craftsman who works effortlessly'
    ],
    political: 'The best ruler governs by wu-wei, barely noticed',
    strength: PHI_INV
  });
  
  state.concepts.set('yin-yang', {
    name: 'Yin-Yang',
    chinese: '陰陽',
    meaning: 'Complementary opposites',
    pairs: [
      { yin: 'Dark', yang: 'Light' },
      { yin: 'Passive', yang: 'Active' },
      { yin: 'Female', yang: 'Male' },
      { yin: 'Cold', yang: 'Hot' },
      { yin: 'Earth', yang: 'Heaven' },
      { yin: 'Yielding', yang: 'Firm' }
    ],
    principles: [
      'Opposites are interdependent',
      'Each contains the seed of the other',
      'They transform into each other',
      'Balance is dynamic, not static'
    ],
    cosmology: 'From Dao came One, from One came Two (yin-yang), from Two came Three, from Three came all things',
    strength: PHI_INV
  });
  
  state.concepts.set('ziran', {
    name: 'Ziran',
    chinese: '自然',
    meaning: 'Naturalness, Self-so, Spontaneity',
    aspects: {
      metaphysical: 'Things being as they naturally are',
      ethical: 'Acting from one\'s true nature',
      aesthetic: 'Unforced, spontaneous beauty'
    },
    contrast: 'Opposed to artificiality, convention, forcing',
    daodejing: 'Humans follow Earth, Earth follows Heaven, Heaven follows Dao, Dao follows ziran',
    implication: 'Highest good is to be natural, not to impose',
    strength: PHI_INV
  });
  
  state.concepts.set('pu', {
    name: 'Pu',
    chinese: '樸',
    meaning: 'The Uncarved Block',
    metaphor: 'Raw wood before it is shaped',
    represents: [
      'Original simplicity',
      'Potential before differentiation',
      'Natural state uncorrupted by society'
    ],
    ethical: 'Return to simplicity and naturalness',
    contrast: 'Civilization and learning obscure original nature',
    strength: PHI_INV_2
  });
  
  state.stats.conceptsRegistered = state.concepts.size;
}

/**
 * Register Daoist texts
 */
function registerTexts() {
  state.texts.set('daodejing', {
    name: 'Daodejing (Tao Te Ching)',
    chinese: '道德經',
    author: 'Laozi (legendary)',
    date: '4th-6th century BCE',
    chapters: 81,
    themes: [
      'Nature of Dao',
      'Wu-wei in governance',
      'Simplicity and humility',
      'Paradox and reversal'
    ],
    famousPassages: [
      'The Dao that can be told is not the eternal Dao',
      'The softest thing overcomes the hardest',
      'Act without acting, work without effort',
      'Those who know do not speak; those who speak do not know'
    ],
    influence: 'One of the most translated books in history',
    strength: PHI_INV
  });
  
  state.texts.set('zhuangzi', {
    name: 'Zhuangzi',
    chinese: '莊子',
    author: 'Zhuangzi (369-286 BCE)',
    characteristics: [
      'Philosophical parables and stories',
      'Humor and paradox',
      'Skepticism about knowledge',
      'Celebration of spontaneity'
    ],
    famousStories: [
      { name: 'Butterfly Dream', theme: 'Uncertainty about reality and identity' },
      { name: 'Cook Ding', theme: 'Skill through wu-wei' },
      { name: 'Useless Tree', theme: 'Value of uselessness' },
      { name: 'Fish Happiness', theme: 'Limits of knowledge' }
    ],
    philosophy: {
      epistemology: 'Skeptical about fixed perspectives',
      ethics: 'Freedom and spontaneity over rigid rules',
      metaphysics: 'Transformation and flux'
    },
    strength: PHI_INV
  });
  
  state.texts.set('liezi', {
    name: 'Liezi',
    chinese: '列子',
    author: 'Attributed to Liezi',
    themes: ['Fate and destiny', 'Relativism', 'Spiritual cultivation'],
    note: 'Later text, possibly 3rd-4th century CE compilation',
    strength: PHI_INV_2
  });
  
  state.stats.textsRegistered = state.texts.size;
}

/**
 * Register Daoist practices
 */
function registerPractices() {
  state.practices.set('meditation', {
    name: 'Daoist Meditation',
    types: [
      { name: 'Zuowang', meaning: 'Sitting and forgetting', goal: 'Empty the mind' },
      { name: 'Neiguan', meaning: 'Inner observation', goal: 'Observe internal processes' },
      { name: 'Cunsi', meaning: 'Visualization', goal: 'Visualize deities or energy' }
    ],
    goal: 'Return to naturalness, cultivate de',
    strength: PHI_INV_2
  });
  
  state.practices.set('qigong', {
    name: 'Qigong',
    chinese: '氣功',
    meaning: 'Working with qi (vital energy)',
    elements: ['Breathing', 'Movement', 'Meditation'],
    goal: 'Cultivate and balance vital energy',
    relation: 'Connected to both Daoist and medical traditions',
    strength: PHI_INV_2
  });
  
  state.practices.set('taiji', {
    name: 'Taijiquan',
    chinese: '太極拳',
    meaning: 'Supreme ultimate fist',
    characteristics: [
      'Slow, flowing movements',
      'Balance of yin and yang',
      'Soft overcomes hard',
      'Wu-wei in motion'
    ],
    philosophy: 'Embodies Daoist principles in physical practice',
    strength: PHI_INV_2
  });
  
  state.stats.practicesRegistered = state.practices.size;
}

/**
 * Get a concept
 */
function getConcept(conceptId) {
  return state.concepts.get(conceptId) || null;
}

/**
 * Get a text
 */
function getText(textId) {
  return state.texts.get(textId) || null;
}

/**
 * Get a practice
 */
function getPractice(practiceId) {
  return state.practices.get(practiceId) || null;
}

/**
 * List all concepts
 */
function listConcepts() {
  return Array.from(state.concepts.entries()).map(([id, c]) => ({ id, ...c }));
}

/**
 * List all texts
 */
function listTexts() {
  return Array.from(state.texts.entries()).map(([id, t]) => ({ id, ...t }));
}

/**
 * Analyze a situation through Daoist lens
 */
function analyzeWithWuWei(situation) {
  state.stats.analysesPerformed++;
  
  return {
    situation,
    daoistPerspective: {
      wuWei: {
        question: 'Are you forcing or flowing?',
        guidance: 'Find the path of least resistance that achieves the goal',
        caution: 'Effort is not always wrong; forced effort is'
      },
      yinYang: {
        question: 'Where is the balance?',
        guidance: 'Look for the complementary opposite; embrace both sides',
        caution: 'Extremes tend to reverse themselves'
      },
      ziran: {
        question: 'What would happen naturally?',
        guidance: 'Act in accordance with the natural pattern',
        caution: 'Distinguish natural from conventional'
      }
    },
    daodejingWisdom: [
      'The softest thing in the universe overcomes the hardest',
      'When you are content to be simply yourself, everyone will respect you',
      'In dwelling, be close to the land. In meditation, go deep in the heart'
    ],
    cynicNote: '*tail wag* Daoism suggests: stop pushing so hard. But don\'t stop moving.',
    confidence: PHI_INV_2
  };
}

/**
 * Compare Daoism with other traditions
 */
function compareWithOther(tradition) {
  state.stats.analysesPerformed++;
  
  const comparisons = {
    confucianism: {
      similarity: 'Both Chinese, concern with proper living',
      difference: 'Confucianism emphasizes social order, Daoism naturalness',
      daoist: 'Confucian virtues are artificial impositions',
      confucian: 'Society needs structure and ritual'
    },
    buddhism: {
      similarity: 'Both emphasize non-attachment, meditation',
      difference: 'Buddhism denies self; Daoism affirms natural self',
      synthesis: 'Chan/Zen Buddhism combines both',
      note: 'Historical interaction and mutual influence'
    },
    stoicism: {
      similarity: 'Both emphasize living according to nature',
      difference: 'Stoic nature is rational; Daoist nature is spontaneous',
      wuWei: 'Resembles Stoic acceptance but less rational'
    },
    existentialism: {
      similarity: 'Both value authenticity',
      difference: 'Existentialism emphasizes choice; Daoism emphasizes flow',
      note: 'Heidegger was interested in Daoist thought'
    }
  };
  
  return {
    tradition,
    comparison: comparisons[tradition.toLowerCase()] || { note: 'Comparison not found' },
    cynicNote: '*head tilt* Traditions differ, but wisdom often rhymes.',
    confidence: PHI_INV_2
  };
}

/**
 * Format engine status
 */
function formatStatus() {
  return `
┌─────────────────────────────────────────────────────────┐
│  DAOIST ENGINE                           Phase 37B     │
├─────────────────────────────────────────────────────────┤
│  Concepts: ${String(state.stats.conceptsRegistered).padStart(3)}                                      │
│  Texts: ${String(state.stats.textsRegistered).padStart(3)}                                         │
│  Practices: ${String(state.stats.practicesRegistered).padStart(3)}                                    │
│  Analyses: ${String(state.stats.analysesPerformed).padStart(3)}                                      │
├─────────────────────────────────────────────────────────┤
│  Core Concepts:                                         │
│    - Dao (the Way)                                      │
│    - Wu-wei (effortless action)                         │
│    - Yin-Yang (complementary opposites)                 │
│    - Ziran (naturalness)                                │
├─────────────────────────────────────────────────────────┤
│  φ-bounded: max ${(PHI_INV * 100).toFixed(1)}% confidence                      │
│  *sniff* The Dao that can be coded is not eternal Dao   │
└─────────────────────────────────────────────────────────┘`.trim();
}

/**
 * Get stats
 */
function getStats() {
  return {
    concepts: state.stats.conceptsRegistered,
    texts: state.stats.textsRegistered,
    practices: state.stats.practicesRegistered,
    analyses: state.stats.analysesPerformed
  };
}

module.exports = {
  init,
  getConcept,
  getText,
  getPractice,
  listConcepts,
  listTexts,
  analyzeWithWuWei,
  compareWithOther,
  formatStatus,
  getStats,
  PHI,
  PHI_INV,
  PHI_INV_2,
  PHI_INV_3
};
