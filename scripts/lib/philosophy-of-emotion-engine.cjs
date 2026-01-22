#!/usr/bin/env node

/**
 * Philosophy of Emotion Engine - Phase 45C
 *
 * What are emotions? Feelings, judgments, perceptions, affects?
 * From James-Lange to constructionism.
 *
 * φ-bounded: max 61.8% confidence
 *
 * *ears perk* Emotions are not irrational. They are how we care.
 */

const path = require('path');
const os = require('os');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;
const PHI_INV_3 = 0.236067977499790;

// Storage
const STORAGE_DIR = path.join(os.homedir(), '.cynic', 'philosophy-of-emotion');

// State
const state = {
  initialized: false,
  theories: new Map(),
  thinkers: new Map(),
  concepts: new Map(),
  emotions: new Map(),
  debates: new Map()
};

/**
 * Initialize the Philosophy of Emotion Engine
 */
function init() {
  if (state.initialized) {
    return { status: 'already initialized', phi: PHI_INV };
  }

  initializeTheories();
  initializeThinkers();
  initializeConcepts();
  initializeEmotions();
  initializeDebates();

  state.initialized = true;
  return { status: 'initialized', phi: PHI_INV };
}

/**
 * Initialize theories of emotion
 */
function initializeTheories() {
  // Feeling Theory (James-Lange)
  state.theories.set('feeling-theory', {
    name: 'Feeling Theory (James-Lange)',
    founders: ['William James', 'Carl Lange'],
    period: '1884',
    thesis: 'Emotions are perceptions of bodily changes',
    jamesQuote: 'We don\'t cry because we\'re sad; we\'re sad because we cry',
    claims: {
      bodily: 'Emotion is perception of bodily states',
      causal: 'Event → Bodily response → Feeling of emotion',
      necessary: 'No bodily change, no emotion'
    },
    implications: 'Change body to change emotion (facial feedback)',
    critique: 'Same bodily states, different emotions (Cannon-Bard)',
    strength: PHI_INV
  });

  // Cognitivism
  state.theories.set('cognitivism', {
    name: 'Cognitive Theory of Emotion',
    proponents: ['Martha Nussbaum', 'Robert Solomon', 'Aaron Ben-Ze\'ev'],
    thesis: 'Emotions are or involve evaluative judgments',
    claims: {
      judgment: 'Emotions involve judgments about value/significance',
      intentional: 'Emotions are about something (intentional)',
      rational: 'Emotions can be rational or irrational'
    },
    nussbaum: 'Emotions are judgments of value concerning our flourishing',
    solomon: 'Emotions are judgments, not mere feelings',
    critique: 'Animals and infants have emotions without judgments?',
    strength: PHI_INV
  });

  // Perceptualism
  state.theories.set('perceptualism', {
    name: 'Perceptual Theory of Emotion',
    proponents: ['Jesse Prinz', 'Ronald de Sousa', 'Sabine Döring'],
    thesis: 'Emotions are perceptions of value or significance',
    claims: {
      perception: 'Emotions perceive evaluative properties',
      noncognitive: 'No judgment required; perceptual, not propositional',
      embodied: 'Grounded in bodily perception (Prinz)'
    },
    prinz: 'Emotions are gut reactions that represent core relational themes',
    analogy: 'As vision perceives colors, emotion perceives values',
    strength: PHI_INV
  });

  // Basic Emotions / Affect Program
  state.theories.set('affect-program', {
    name: 'Affect Program Theory',
    proponents: ['Paul Ekman', 'Jaak Panksepp'],
    thesis: 'Basic emotions are evolved, universal affect programs',
    claims: {
      universal: 'Basic emotions universal across cultures',
      evolved: 'Products of natural selection',
      automatic: 'Triggered automatically, hard to suppress',
      discrete: 'Distinct programs, not dimensions'
    },
    ekman: ['Anger', 'Fear', 'Sadness', 'Happiness', 'Disgust', 'Surprise'],
    evidence: 'Cross-cultural facial expression studies',
    critique: 'Cultural variation suggests construction, not nature',
    strength: PHI_INV
  });

  // Constructionism
  state.theories.set('constructionism', {
    name: 'Psychological Constructionism',
    proponents: ['Lisa Feldman Barrett', 'James Russell'],
    thesis: 'Emotions are constructed from core affect + conceptual interpretation',
    claims: {
      coreAffect: 'Primitive states of pleasure/displeasure, arousal',
      constructed: 'Emotion categories learned, not innate',
      conceptual: 'Concepts construct discrete emotions from affect',
      variable: 'Emotion categories vary across cultures'
    },
    barrett: 'Emotions are predictions, not reactions',
    contrast: 'Against natural kinds view of basic emotions',
    strength: PHI_INV
  });

  // Somatic Marker Hypothesis
  state.theories.set('somatic-marker', {
    name: 'Somatic Marker Hypothesis',
    founder: 'Antonio Damasio',
    work: 'Descartes\' Error (1994)',
    thesis: 'Emotions guide rational decision-making via bodily signals',
    claims: {
      embodied: 'Emotions involve bodily states (somatic markers)',
      decisional: 'Emotions necessary for good decisions',
      unconscious: 'Can influence without conscious awareness'
    },
    evidence: 'Patients with prefrontal damage: impaired emotion and decision',
    implications: 'Reason requires emotion; Descartes was wrong',
    strength: PHI_INV
  });

  // Evaluative Theory
  state.theories.set('evaluative', {
    name: 'Evaluative Theory',
    proponents: ['Robert Roberts', 'Bennett Helm'],
    thesis: 'Emotions are concern-based construals',
    claims: {
      construal: 'Emotions construe situations under evaluative aspects',
      concern: 'Linked to what we care about',
      reason: 'Responsive to reasons'
    },
    roberts: 'Emotion is seeing-as colored by concern',
    helm: 'Emotions constitute caring about things',
    strength: PHI_INV
  });

  // Enactive/Embodied Emotion
  state.theories.set('enactive-emotion', {
    name: 'Enactive/Embodied Theory',
    proponents: ['Giovanna Colombetti', 'Jan Slaby'],
    thesis: 'Emotions are bodily engagements with meaningful situations',
    claims: {
      situated: 'Emotions arise in situations, not in heads',
      action: 'Emotions are action-oriented',
      extended: 'Emotions can extend into environment'
    },
    colombetti: 'Affectivity is primordial mode of being embodied',
    implications: 'Emotion not separate from cognition or action',
    strength: PHI_INV
  });
}

/**
 * Initialize thinkers
 */
function initializeThinkers() {
  state.thinkers.set('james', {
    name: 'William James',
    dates: '1842-1910',
    work: 'What is an Emotion? (1884)',
    view: 'Feeling theory',
    contributions: {
      peripheralism: 'Emotion is perception of bodily changes',
      reversal: 'We don\'t run because we\'re afraid; we\'re afraid because we run',
      subtraction: 'Subtract bodily feelings, nothing left'
    },
    influence: 'Set agenda for philosophy of emotion',
    phi: PHI_INV
  });

  state.thinkers.set('solomon', {
    name: 'Robert Solomon',
    dates: '1942-2007',
    works: ['The Passions', 'Not Passion\'s Slave'],
    view: 'Cognitivism',
    contributions: {
      judgment: 'Emotions are judgments',
      responsibility: 'We are responsible for our emotions',
      existential: 'Emotions express our values and choices'
    },
    phi: PHI_INV
  });

  state.thinkers.set('nussbaum', {
    name: 'Martha Nussbaum',
    dates: '1947-present',
    work: 'Upheavals of Thought (2001)',
    view: 'Neo-Stoic cognitivism',
    contributions: {
      eudaimonistic: 'Emotions are judgments about flourishing',
      needBased: 'Emotions reflect our neediness and vulnerability',
      intelligence: 'Emotions have cognitive intelligence'
    },
    influence: 'Major contemporary emotion theorist',
    phi: PHI_INV
  });

  state.thinkers.set('ekman', {
    name: 'Paul Ekman',
    dates: '1934-present',
    view: 'Basic emotions theory',
    contributions: {
      universality: 'Cross-cultural studies of facial expressions',
      basicEmotions: 'Six basic universal emotions',
      FACS: 'Facial Action Coding System',
      microexpressions: 'Brief involuntary expressions'
    },
    phi: PHI_INV
  });

  state.thinkers.set('damasio', {
    name: 'Antonio Damasio',
    dates: '1944-present',
    works: ['Descartes\' Error', 'Looking for Spinoza'],
    view: 'Somatic marker hypothesis',
    contributions: {
      somatic: 'Bodily states guide decision-making',
      asIf: 'As-if body loop: simulate bodily states',
      nested: 'Emotions, feelings, and consciousness nested'
    },
    phi: PHI_INV
  });

  state.thinkers.set('prinz', {
    name: 'Jesse Prinz',
    dates: '1967-present',
    work: 'Gut Reactions (2004)',
    view: 'Embodied appraisal theory',
    contributions: {
      embodied: 'Emotions are embodied appraisals',
      perceptual: 'Emotions perceive core relational themes',
      cultural: 'Culture shapes emotion through concepts'
    },
    phi: PHI_INV
  });

  state.thinkers.set('barrett', {
    name: 'Lisa Feldman Barrett',
    dates: '1963-present',
    work: 'How Emotions Are Made (2017)',
    view: 'Psychological constructionism',
    contributions: {
      construction: 'Emotions are predictions, not reactions',
      concepts: 'Emotion concepts construct experience',
      variation: 'No emotion fingerprints; high variability'
    },
    challenge: 'Challenges basic emotions paradigm',
    phi: PHI_INV
  });

  state.thinkers.set('de-sousa', {
    name: 'Ronald de Sousa',
    dates: '1940-present',
    work: 'The Rationality of Emotion (1987)',
    view: 'Perceptualism',
    contributions: {
      rationality: 'Emotions can be rational or irrational',
      paradigm: 'Paradigm scenarios shape emotion concepts',
      salience: 'Emotions focus attention on what matters'
    },
    phi: PHI_INV
  });

  state.thinkers.set('aristotle', {
    name: 'Aristotle',
    dates: '384-322 BCE',
    work: 'Rhetoric, Nicomachean Ethics',
    contributions: {
      cognitive: 'Emotions involve belief and judgment',
      appropriate: 'Right emotion at right time to right degree',
      virtue: 'Emotional responses can be virtuous',
      rhetoric: 'Analyzed emotions for persuasion'
    },
    influence: 'Ancient foundation for cognitivism',
    phi: PHI_INV
  });
}

/**
 * Initialize key concepts
 */
function initializeConcepts() {
  state.concepts.set('intentionality', {
    name: 'Emotional Intentionality',
    definition: 'Emotions are directed at objects',
    features: {
      aboutness: 'Fear is of something; anger is at someone',
      formalObject: 'Each emotion has characteristic object (fear: threat)',
      particular: 'Directed at particular objects/events'
    },
    debate: 'Are there objectless moods?',
    phi: PHI_INV
  });

  state.concepts.set('valence', {
    name: 'Valence',
    definition: 'Positive or negative quality of emotion',
    types: {
      positive: 'Joy, love, pride, gratitude',
      negative: 'Fear, anger, sadness, disgust',
      mixed: 'Bittersweet, jealousy'
    },
    question: 'Is valence primitive or derived from other features?',
    phi: PHI_INV
  });

  state.concepts.set('core-affect', {
    name: 'Core Affect',
    source: 'James Russell, Lisa Feldman Barrett',
    definition: 'Primitive state of pleasure/displeasure and arousal',
    dimensions: {
      valence: 'Pleasant to unpleasant',
      arousal: 'Activated to deactivated'
    },
    circumplex: 'Emotions plotted on valence × arousal space',
    role: 'Building block from which emotions constructed',
    phi: PHI_INV
  });

  state.concepts.set('appraisal', {
    name: 'Appraisal',
    definition: 'Evaluation of situation\'s significance for wellbeing',
    theories: ['Lazarus', 'Scherer', 'Frijda'],
    dimensions: {
      relevance: 'Does it matter to me?',
      goalConducive: 'Does it help or hinder my goals?',
      agency: 'Who is responsible?',
      coping: 'Can I handle it?'
    },
    types: 'Automatic vs. deliberate appraisal',
    phi: PHI_INV
  });

  state.concepts.set('action-tendencies', {
    name: 'Action Tendencies',
    source: 'Nico Frijda',
    definition: 'States of readiness to achieve or maintain relationship with object',
    examples: {
      fear: 'Tendency to flee, avoid',
      anger: 'Tendency to attack, aggress',
      sadness: 'Tendency to withdraw'
    },
    frijda: 'Emotions are defined by action tendencies',
    phi: PHI_INV
  });

  state.concepts.set('feeling', {
    name: 'Feeling',
    definition: 'Subjective, experiential aspect of emotion',
    distinction: 'Emotion (broader) vs. feeling (experiential component)',
    questions: {
      necessary: 'Are feelings necessary for emotion?',
      sufficient: 'Are feelings sufficient for emotion?',
      reducible: 'What are feelings? Bodily? Mental?'
    },
    phi: PHI_INV
  });

  state.concepts.set('emotional-expression', {
    name: 'Emotional Expression',
    definition: 'Outward manifestation of emotion',
    forms: ['Facial expressions', 'Voice', 'Posture', 'Action'],
    debates: {
      universal: 'Are expressions universal or cultural?',
      function: 'Communication or preparation for action?',
      feedback: 'Do expressions affect felt emotion?'
    },
    phi: PHI_INV
  });

  state.concepts.set('emotion-regulation', {
    name: 'Emotion Regulation',
    definition: 'Processes by which we influence our emotions',
    strategies: {
      situationSelection: 'Avoid or approach emotional situations',
      attentional: 'Redirect attention',
      reappraisal: 'Reinterpret situation',
      suppression: 'Inhibit expression'
    },
    gross: 'Process model of emotion regulation',
    phi: PHI_INV
  });
}

/**
 * Initialize specific emotions
 */
function initializeEmotions() {
  state.emotions.set('fear', {
    name: 'Fear',
    formalObject: 'Threat, danger',
    appraisal: 'Something bad may happen that I cannot control',
    actionTendency: 'Flight, avoidance, freezing',
    function: 'Protect from danger',
    pathology: 'Anxiety disorders, phobias',
    phi: PHI_INV
  });

  state.emotions.set('anger', {
    name: 'Anger',
    formalObject: 'Offense, wrong',
    appraisal: 'Someone has wronged me or mine',
    actionTendency: 'Attack, aggression, assertion',
    function: 'Rectify wrongs, assert boundaries',
    moral: 'Can be morally appropriate response to injustice',
    phi: PHI_INV
  });

  state.emotions.set('sadness', {
    name: 'Sadness',
    formalObject: 'Loss',
    appraisal: 'Something valued has been lost',
    actionTendency: 'Withdrawal, inactivity',
    function: 'Process loss, signal need for support',
    grief: 'Extended sadness in response to major loss',
    phi: PHI_INV
  });

  state.emotions.set('disgust', {
    name: 'Disgust',
    formalObject: 'Contamination, impurity',
    appraisal: 'This is contaminating or degrading',
    actionTendency: 'Rejection, expulsion, avoidance',
    domains: {
      core: 'Food, bodily products',
      interpersonal: 'Violators of norms',
      moral: 'Moral transgressions'
    },
    phi: PHI_INV
  });

  state.emotions.set('love', {
    name: 'Love',
    formalObject: 'Valued other',
    types: ['Romantic', 'Filial', 'Friendship', 'Agape'],
    features: {
      concern: 'Care for beloved\'s wellbeing',
      valuation: 'See beloved as valuable',
      union: 'Desire for union or closeness'
    },
    philosophy: 'Debates about nature, rationality, and ethics of love',
    phi: PHI_INV
  });

  state.emotions.set('shame', {
    name: 'Shame',
    formalObject: 'Failure to meet ideal self',
    appraisal: 'I have fallen short of who I should be',
    actionTendency: 'Hide, disappear',
    contrast: 'Shame (self) vs. guilt (action)',
    socialRole: 'Enforces social norms',
    phi: PHI_INV
  });

  state.emotions.set('guilt', {
    name: 'Guilt',
    formalObject: 'Wrong action',
    appraisal: 'I have done something wrong',
    actionTendency: 'Confess, repair, make amends',
    contrast: 'Guilt (action) vs. shame (self)',
    moral: 'Motivates moral behavior',
    phi: PHI_INV
  });
}

/**
 * Initialize debates
 */
function initializeDebates() {
  state.debates.set('cognitivism-vs-noncognitivism', {
    name: 'Cognitivism vs. Non-Cognitivism',
    question: 'Are emotions constitutively cognitive?',
    positions: {
      cognitivist: 'Emotions are or involve judgments',
      noncognitivist: 'Emotions are feelings, not judgments',
      hybrid: 'Emotions involve cognitive and non-cognitive elements'
    },
    stake: 'Rationality and responsibility for emotions',
    phi: PHI_INV
  });

  state.debates.set('basic-vs-constructed', {
    name: 'Basic Emotions vs. Constructionism',
    question: 'Are emotions natural kinds or constructed?',
    positions: {
      basicEmotions: 'Core emotions are evolved, universal',
      constructionist: 'Emotion categories are culturally constructed',
      hybrid: 'Some universal features, much cultural variation'
    },
    evidence: 'Cross-cultural studies, neuroimaging, development',
    phi: PHI_INV
  });

  state.debates.set('emotion-and-reason', {
    name: 'Emotion and Reason',
    question: 'Are emotions rational? Do they contribute to reason?',
    positions: {
      opposition: 'Emotions cloud reason (Stoics)',
      collaboration: 'Emotion and reason work together (Damasio)',
      emotional: 'Emotions are themselves rational (Solomon, de Sousa)'
    },
    damasio: 'Pure reason impossible; emotion necessary for decisions',
    phi: PHI_INV
  });

  state.debates.set('moral-emotions', {
    name: 'Emotions and Moral Judgment',
    question: 'What role do emotions play in morality?',
    positions: {
      sentimentalist: 'Emotions are basis of moral judgment (Hume)',
      rationalist: 'Reason determines morality; emotions implement',
      intuitionist: 'Moral intuitions are emotional responses'
    },
    haidt: 'Moral reasoning is post-hoc rationalization',
    phi: PHI_INV
  });

  state.debates.set('emotion-function', {
    name: 'Function of Emotions',
    question: 'What are emotions for?',
    proposals: {
      adaptive: 'Solve ancestral problems (Ekman, Tooby & Cosmides)',
      social: 'Coordinate social interaction',
      motivational: 'Motivate action toward goals',
      informational: 'Inform us about our concerns'
    },
    phi: PHI_INV
  });
}

// ═══════════════════════════════════════════════════════════════════
// QUERY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════

function getTheory(id) {
  return state.theories.get(id) || { error: 'Theory not found', query: id };
}

function getThinker(id) {
  return state.thinkers.get(id) || { error: 'Thinker not found', query: id };
}

function getConcept(id) {
  return state.concepts.get(id) || { error: 'Concept not found', query: id };
}

function getEmotion(id) {
  return state.emotions.get(id) || { error: 'Emotion not found', query: id };
}

function getDebate(id) {
  return state.debates.get(id) || { error: 'Debate not found', query: id };
}

function listTheories() {
  return Array.from(state.theories.values()).map(t => ({
    name: t.name,
    thesis: t.thesis
  }));
}

function listEmotions() {
  return Array.from(state.emotions.values()).map(e => ({
    name: e.name,
    formalObject: e.formalObject,
    actionTendency: e.actionTendency
  }));
}

// ═══════════════════════════════════════════════════════════════════
// ANALYSIS FUNCTIONS
// ═══════════════════════════════════════════════════════════════════

/**
 * Analyze emotion from multiple perspectives
 */
function analyzeEmotion(emotionOrTopic) {
  const lower = emotionOrTopic.toLowerCase();

  // Check if it's a specific emotion
  const emotion = state.emotions.get(lower);
  if (emotion) {
    return {
      emotion: emotion.name,
      formalObject: emotion.formalObject,
      appraisal: emotion.appraisal,
      actionTendency: emotion.actionTendency,
      function: emotion.function,
      perspectives: {
        feeling: 'What does this emotion feel like?',
        cognitive: 'What judgment does this emotion involve?',
        perceptual: 'What does this emotion perceive?',
        functional: 'What adaptive function does it serve?'
      },
      cynicNote: `*ears perk* ${emotion.name} tells us what we care about. φ-bounded: ${(PHI_INV * 100).toFixed(1)}% max confidence.`,
      phi: PHI_INV
    };
  }

  // General analysis
  const analysis = {
    topic: emotionOrTopic,
    perspectives: {}
  };

  if (lower.includes('rational') || lower.includes('reason')) {
    analysis.perspectives.opposition = 'Emotions cloud rational judgment';
    analysis.perspectives.collaboration = 'Emotions necessary for good decisions';
    analysis.perspectives.emotional = 'Emotions themselves can be rational or irrational';
  }

  if (lower.includes('moral') || lower.includes('ethic')) {
    analysis.perspectives.sentimentalist = 'Emotions are basis of moral judgment';
    analysis.perspectives.rationalist = 'Reason grounds morality; emotions follow';
    analysis.perspectives.intuitionist = 'Moral intuitions are emotional';
  }

  if (lower.includes('body') || lower.includes('feel')) {
    analysis.perspectives.jamesian = 'Emotion is perception of bodily changes';
    analysis.perspectives.cognitive = 'Bodily feelings are effect, not essence';
    analysis.perspectives.somatic = 'Body guides decision-making via emotion';
  }

  // Default
  if (Object.keys(analysis.perspectives).length === 0) {
    analysis.perspectives.feeling = 'What is the experiential quality?';
    analysis.perspectives.cognitive = 'What evaluation or judgment is involved?';
    analysis.perspectives.functional = 'What purpose does it serve?';
    analysis.perspectives.social = 'How does it function in social life?';
  }

  analysis.cynicNote = `*sniff* Emotions are not irrational. They reveal what we care about. φ-bounded: ${(PHI_INV * 100).toFixed(1)}% max confidence.`;
  analysis.phi = PHI_INV;

  return analysis;
}

// ═══════════════════════════════════════════════════════════════════
// STATUS AND STATS
// ═══════════════════════════════════════════════════════════════════

function formatStatus() {
  const lines = [
    '┌─────────────────────────────────────────────────────┐',
    '│     PHILOSOPHY OF EMOTION ENGINE                    │',
    '│     *ears perk* Emotions reveal what we care about  │',
    '├─────────────────────────────────────────────────────┤',
    `│  Theories: ${state.theories.size.toString().padStart(2)}     Emotions: ${state.emotions.size.toString().padStart(2)}                │`,
    `│  Thinkers: ${state.thinkers.size.toString().padStart(2)}     Debates: ${state.debates.size.toString().padStart(2)}                  │`,
    `│  Concepts: ${state.concepts.size.toString().padStart(2)}                                  │`,
    '├─────────────────────────────────────────────────────┤',
    `│  φ-bound: ${(PHI_INV * 100).toFixed(1)}% max confidence               │`,
    '│  Feelings, judgments, or perceptions of value?      │',
    '└─────────────────────────────────────────────────────┘'
  ];
  return lines.join('\n');
}

function getStats() {
  return {
    theories: state.theories.size,
    thinkers: state.thinkers.size,
    concepts: state.concepts.size,
    emotions: state.emotions.size,
    debates: state.debates.size,
    phi: PHI_INV
  };
}

// ═══════════════════════════════════════════════════════════════════
// EXPORTS
// ═══════════════════════════════════════════════════════════════════

module.exports = {
  init,
  getTheory,
  getThinker,
  getConcept,
  getEmotion,
  getDebate,
  listTheories,
  listEmotions,
  analyzeEmotion,
  formatStatus,
  getStats,
  PHI,
  PHI_INV,
  PHI_INV_2,
  PHI_INV_3
};
