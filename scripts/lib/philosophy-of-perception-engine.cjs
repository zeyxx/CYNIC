#!/usr/bin/env node

/**
 * Philosophy of Perception Engine - Phase 45B
 *
 * How we perceive the world: direct realism, representationalism,
 * sense-data, phenomenology, disjunctivism.
 *
 * φ-bounded: max 61.8% confidence
 *
 * *sniff* Do we perceive the world or our representations of it?
 */

const path = require('path');
const os = require('os');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;
const PHI_INV_3 = 0.236067977499790;

// Storage
const STORAGE_DIR = path.join(os.homedir(), '.cynic', 'philosophy-of-perception');

// State
const state = {
  initialized: false,
  theories: new Map(),
  thinkers: new Map(),
  concepts: new Map(),
  arguments: new Map(),
  phenomena: new Map()
};

/**
 * Initialize the Philosophy of Perception Engine
 */
function init() {
  if (state.initialized) {
    return { status: 'already initialized', phi: PHI_INV };
  }

  initializeTheories();
  initializeThinkers();
  initializeConcepts();
  initializeArguments();
  initializePhenomena();

  state.initialized = true;
  return { status: 'initialized', phi: PHI_INV };
}

/**
 * Initialize theories of perception
 */
function initializeTheories() {
  // Naive/Direct Realism
  state.theories.set('direct-realism', {
    name: 'Direct Realism (Naive Realism)',
    thesis: 'We directly perceive mind-independent objects and their properties',
    slogan: 'What you see is what you get',
    claims: {
      direct: 'Perception puts us in direct contact with world',
      mindindependent: 'Objects exist as perceived independently of perception',
      transparency: 'Perception is transparent to its objects'
    },
    appeal: 'Preserves common-sense view of perception',
    challenge: 'How to handle illusions and hallucinations',
    defenders: ['J.L. Austin', 'John McDowell', 'Bill Brewer'],
    strength: PHI_INV
  });

  // Indirect/Representative Realism
  state.theories.set('indirect-realism', {
    name: 'Indirect Realism (Representative Realism)',
    thesis: 'We perceive the world via mental representations',
    slogan: 'We perceive ideas of things, not things themselves',
    claims: {
      mediated: 'Perception mediated by mental representations',
      causation: 'External objects cause internal representations',
      inference: 'Knowledge of world involves inference from representations'
    },
    locke: 'Ideas in mind caused by primary qualities of objects',
    problem: 'Veil of perception: how do we know representations are accurate?',
    strength: PHI_INV
  });

  // Sense-Data Theory
  state.theories.set('sense-data', {
    name: 'Sense-Data Theory',
    period: 'Early 20th century',
    thesis: 'Immediate objects of perception are sense-data, not physical objects',
    proponents: ['Russell', 'Moore', 'Broad', 'Price'],
    claims: {
      immediate: 'We immediately perceive sense-data',
      mental: 'Sense-data are mental particulars',
      properties: 'Sense-data have properties they appear to have'
    },
    motivation: 'Argument from illusion shows we don\'t directly perceive objects',
    decline: 'Fell out of favor due to multiple problems',
    strength: PHI_INV_2
  });

  // Intentionalism/Representationalism
  state.theories.set('representationalism', {
    name: 'Intentionalism (Representationalism)',
    thesis: 'Perceptual experience is a form of representation',
    proponents: ['Fred Dretske', 'Michael Tye', 'Gilbert Harman'],
    claims: {
      content: 'Perception has representational content',
      accuracy: 'Content can be accurate or inaccurate',
      character: 'Phenomenal character supervenes on content'
    },
    transparency: 'When introspecting, we see only represented objects',
    strong: 'Phenomenal character = representational content',
    moderate: 'Phenomenal character involves but isn\'t exhausted by content',
    strength: PHI_INV
  });

  // Disjunctivism
  state.theories.set('disjunctivism', {
    name: 'Disjunctivism',
    thesis: 'Veridical perception and hallucination are fundamentally different states',
    proponents: ['J.M. Hinton', 'John McDowell', 'M.G.F. Martin'],
    claims: {
      disjunction: 'Perceptual states are either veridical OR merely apparent',
      noCommonKind: 'No common mental state type shared by perception and hallucination',
      relational: 'Veridical perception is constitutively relational to objects'
    },
    motivation: 'Preserve direct realism against argument from hallucination',
    challenge: 'How to explain subjective indistinguishability',
    strength: PHI_INV
  });

  // Phenomenological Approach
  state.theories.set('phenomenological', {
    name: 'Phenomenology of Perception',
    founders: ['Husserl', 'Merleau-Ponty', 'Heidegger'],
    thesis: 'Perception must be studied from first-person perspective',
    keyIdeas: {
      intentionality: 'Perception is always of something',
      horizons: 'Perceived objects given with horizons of possible experience',
      livedBody: 'Body is not just object but subject of perception',
      worldhood: 'Perception discloses a meaningful world'
    },
    merleauPonty: {
      ambiguity: 'Perception involves inherent ambiguity',
      motor: 'Perception and movement inseparable',
      gestalt: 'Figure-ground structure of perception'
    },
    strength: PHI_INV
  });

  // Ecological/Gibsonian
  state.theories.set('ecological', {
    name: 'Ecological Perception',
    founder: 'James J. Gibson',
    thesis: 'Perception is direct pickup of information from environment',
    keyIdeas: {
      affordances: 'Perceive possibilities for action',
      ambient: 'Information in ambient optic array',
      noDirect: 'No need for internal representations',
      active: 'Perception involves active exploration'
    },
    antiCognitivist: 'Rejects computational approach',
    influence: 'Major influence on embodied cognition',
    strength: PHI_INV
  });

  // Enactive/Sensorimotor
  state.theories.set('enactive-perception', {
    name: 'Enactive Theory of Perception',
    proponents: ['O\'Regan', 'Noë', 'Thompson'],
    thesis: 'Perception is constituted by sensorimotor knowledge',
    keyIdeas: {
      sensorimotor: 'Perceiving requires mastery of sensorimotor contingencies',
      active: 'Perception is active exploration, not passive reception',
      skillful: 'Perception is a skillful bodily activity',
      world: 'World is its own best model'
    },
    noe: 'Seeing is a way of acting, not having internal pictures',
    strength: PHI_INV
  });
}

/**
 * Initialize thinkers
 */
function initializeThinkers() {
  state.thinkers.set('locke', {
    name: 'John Locke',
    dates: '1632-1704',
    work: 'Essay Concerning Human Understanding',
    view: 'Indirect realism',
    contributions: {
      primarySecondary: 'Primary qualities (shape, size) vs. secondary (color, sound)',
      ideas: 'We perceive ideas, not objects directly',
      resemblance: 'Primary quality ideas resemble objects; secondary don\'t'
    },
    phi: PHI_INV
  });

  state.thinkers.set('berkeley', {
    name: 'George Berkeley',
    dates: '1685-1753',
    work: 'Three Dialogues, Principles of Human Knowledge',
    view: 'Idealism',
    contributions: {
      esse: 'Esse est percipi: to be is to be perceived',
      antiMatter: 'No matter, only minds and ideas',
      critique: 'Demolished primary/secondary distinction'
    },
    phi: PHI_INV
  });

  state.thinkers.set('husserl', {
    name: 'Edmund Husserl',
    dates: '1859-1938',
    work: 'Logical Investigations, Ideas',
    view: 'Phenomenology',
    contributions: {
      intentionality: 'Consciousness is always of something',
      noema: 'Intentional content of experience',
      horizons: 'Objects given with horizons of possible experience',
      epoche: 'Bracket natural attitude to study experience'
    },
    phi: PHI_INV
  });

  state.thinkers.set('merleau-ponty', {
    name: 'Maurice Merleau-Ponty',
    dates: '1908-1961',
    work: 'Phenomenology of Perception',
    view: 'Embodied phenomenology',
    contributions: {
      livedBody: 'Body as subject, not object',
      motorIntentionality: 'Bodily orientation to world',
      ambiguity: 'Perceptual experience is inherently ambiguous',
      intertwining: 'Perceiver and perceived intertwined'
    },
    influence: 'Foundation for embodied cognition',
    phi: PHI_INV
  });

  state.thinkers.set('gibson', {
    name: 'James J. Gibson',
    dates: '1904-1979',
    work: 'Ecological Approach to Visual Perception',
    view: 'Ecological direct realism',
    contributions: {
      affordances: 'Environment offers action possibilities',
      direct: 'No need for internal representations',
      ambient: 'Information in optic array',
      invariants: 'Perception extracts invariant structure'
    },
    phi: PHI_INV
  });

  state.thinkers.set('dretske', {
    name: 'Fred Dretske',
    dates: '1932-2013',
    works: ['Seeing and Knowing', 'Naturalizing the Mind'],
    view: 'Representationalism',
    contributions: {
      epistemic: 'Seeing is epistemic relation',
      information: 'Information-theoretic account of content',
      qualia: 'Qualia are representational properties'
    },
    phi: PHI_INV
  });

  state.thinkers.set('tye', {
    name: 'Michael Tye',
    dates: '1950-present',
    works: ['Ten Problems of Consciousness', 'Consciousness, Color, and Content'],
    view: 'Strong representationalism',
    contributions: {
      PANIC: 'Phenomenal character is Poised, Abstract, Nonconceptual, Intentional Content',
      transparency: 'Experience is transparent to its content',
      naturalism: 'Naturalistic account of phenomenal consciousness'
    },
    phi: PHI_INV
  });

  state.thinkers.set('mcdowell', {
    name: 'John McDowell',
    dates: '1942-present',
    work: 'Mind and World',
    view: 'Disjunctivist direct realism',
    contributions: {
      disjunctivism: 'Perception and hallucination fundamentally different',
      conceptualism: 'Perceptual content is conceptual',
      openness: 'Mind directly open to facts'
    },
    phi: PHI_INV
  });

  state.thinkers.set('noe', {
    name: 'Alva Noë',
    dates: '1964-present',
    works: ['Action in Perception', 'Out of Our Heads'],
    view: 'Enactivism',
    contributions: {
      sensorimotor: 'Perception requires sensorimotor knowledge',
      active: 'Perceiving is doing',
      external: 'Consciousness is not in the head'
    },
    phi: PHI_INV
  });
}

/**
 * Initialize key concepts
 */
function initializeConcepts() {
  state.concepts.set('qualia', {
    name: 'Qualia',
    definition: 'The subjective, qualitative aspects of experience',
    examples: ['Redness of red', 'Painfulness of pain', 'Smell of coffee'],
    properties: {
      ineffable: 'Cannot be fully communicated',
      intrinsic: 'Not defined by relations to other things',
      private: 'Accessible only to subject',
      apprehensible: 'Known through direct acquaintance'
    },
    debate: 'Do qualia exist? Are they reducible?',
    phi: PHI_INV
  });

  state.concepts.set('phenomenal-character', {
    name: 'Phenomenal Character',
    definition: 'What it\'s like to have an experience',
    nagelPhrase: 'Something it is like to be',
    question: 'What determines phenomenal character?',
    answers: {
      qualia: 'Intrinsic non-representational properties',
      representational: 'Supervenes on representational content',
      relational: 'Determined by objects themselves (naïve realism)'
    },
    phi: PHI_INV
  });

  state.concepts.set('intentionality', {
    name: 'Perceptual Intentionality',
    definition: 'Directedness of perception toward objects',
    brentano: 'Mental phenomena characterized by intentionality',
    features: {
      aboutness: 'Perception is about something',
      content: 'What is represented',
      object: 'What perception is directed at'
    },
    perceptionVsThought: 'Perceptual intentionality differs from thought how?',
    phi: PHI_INV
  });

  state.concepts.set('veridicality', {
    name: 'Veridicality',
    definition: 'Accuracy of perception in representing the world',
    types: {
      veridical: 'Accurate perception of existing object',
      illusory: 'Misperception of existing object',
      hallucinatory: 'Experience as of non-existent object'
    },
    question: 'What do veridical and non-veridical perceptions have in common?',
    phi: PHI_INV
  });

  state.concepts.set('transparency', {
    name: 'Transparency of Experience',
    thesis: 'When introspecting experience, we see only represented objects',
    moore: 'The peculiar diaphanousness of sensations',
    implications: {
      proRepresentationalism: 'Supports representationalist view',
      antiSenseData: 'Against sense-data as objects of awareness'
    },
    objection: 'What about bodily sensations, moods?',
    phi: PHI_INV
  });

  state.concepts.set('perceptual-content', {
    name: 'Perceptual Content',
    definition: 'What perception represents about the world',
    debates: {
      conceptual: 'Is content conceptual or nonconceptual?',
      rich: 'How detailed is content?',
      object: 'Objects or properties or both?',
      abstract: 'Can we perceive abstract properties?'
    },
    siegel: 'Rich content view: we perceive kinds, causation',
    phi: PHI_INV
  });

  state.concepts.set('attention', {
    name: 'Attention in Perception',
    definition: 'Selective focusing of perceptual resources',
    phenomena: {
      inattentional: 'Inattentional blindness',
      change: 'Change blindness',
      load: 'Perceptual load theory'
    },
    question: 'What is perceived without attention?',
    phi: PHI_INV
  });

  state.concepts.set('multimodal', {
    name: 'Multimodal Perception',
    definition: 'Integration of information across senses',
    phenomena: {
      mcgurk: 'McGurk effect: vision affects heard speech',
      rubberHand: 'Vision and touch integrate for body sense',
      ventriloquism: 'Vision captures sound localization'
    },
    unity: 'How do senses combine into unified experience?',
    phi: PHI_INV
  });
}

/**
 * Initialize key arguments
 */
function initializeArguments() {
  state.arguments.set('illusion', {
    name: 'Argument from Illusion',
    structure: {
      p1: 'In illusion, we perceive something that is not as the object is',
      p2: 'What we perceive in illusion must exist with those properties',
      p3: 'The physical object lacks those properties',
      c: 'Therefore, we perceive something mental, not the physical object'
    },
    supports: 'Sense-data theory, indirect realism',
    responses: {
      intentionalist: 'We misrepresent the object; no sense-data',
      disjunctivist: 'Illusion is not paradigm case',
      adverbialist: 'We sense in certain ways, not sense objects'
    },
    phi: PHI_INV
  });

  state.arguments.set('hallucination', {
    name: 'Argument from Hallucination',
    structure: {
      p1: 'In hallucination, experience seems just like veridical perception',
      p2: 'But there is no external object being perceived',
      p3: 'If indistinguishable, they share a common element',
      c: 'Therefore, even veridical perception involves non-physical elements'
    },
    supports: 'Sense-data theory, representationalism',
    responses: {
      disjunctivist: 'No common mental state; only disjunction',
      naive: 'Seeming the same doesn\'t mean being the same'
    },
    phi: PHI_INV
  });

  state.arguments.set('science', {
    name: 'Argument from Science',
    structure: {
      p1: 'Science tells us physical objects lack colors, sounds as experienced',
      p2: 'But we perceive colors, sounds',
      c: 'Therefore, what we perceive are not physical objects themselves'
    },
    galileo: 'Secondary qualities are in the mind, not world',
    responses: {
      realist: 'Science describes same properties differently',
      relationalist: 'Colors are relational properties',
      eliminativist: 'Folk conception of color is false'
    },
    phi: PHI_INV
  });

  state.arguments.set('time-lag', {
    name: 'Time-Lag Argument',
    structure: {
      p1: 'Light takes time to reach us from distant objects',
      p2: 'We see stars that no longer exist',
      p3: 'We cannot directly perceive what no longer exists',
      c: 'Therefore, we perceive representations, not objects directly'
    },
    response: 'Perception can relate us to temporally distant events',
    phi: PHI_INV_2
  });
}

/**
 * Initialize perceptual phenomena
 */
function initializePhenomena() {
  state.phenomena.set('change-blindness', {
    name: 'Change Blindness',
    description: 'Failure to notice large changes in visual scenes',
    discovery: 'Simons & Levin, Rensink',
    implications: {
      representation: 'We don\'t represent all details',
      attention: 'Attention required to perceive change',
      enactive: 'Supports world as external memory'
    },
    phi: PHI_INV
  });

  state.phenomena.set('inattentional-blindness', {
    name: 'Inattentional Blindness',
    description: 'Failure to perceive unexpected stimuli when attending elsewhere',
    example: 'Invisible gorilla experiment',
    implications: 'Attention necessary for conscious perception?',
    phi: PHI_INV
  });

  state.phenomena.set('filling-in', {
    name: 'Perceptual Filling-In',
    description: 'Brain fills in missing information (blind spot, scotoma)',
    types: ['Blind spot filling', 'Neon color spreading', 'Phantom limb'],
    debate: 'Is there actual neural filling or just failure to notice absence?',
    phi: PHI_INV
  });

  state.phenomena.set('afterimages', {
    name: 'Afterimages',
    description: 'Visual impressions persisting after stimulus removal',
    properties: 'Complementary colors, move with eye',
    theoretical: 'What are they images of? Challenge for direct realism',
    phi: PHI_INV
  });

  state.phenomena.set('motion-aftereffect', {
    name: 'Motion Aftereffect',
    description: 'Stationary objects appear to move after viewing motion',
    example: 'Waterfall illusion',
    explanation: 'Adaptation of motion detectors',
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

function getArgument(id) {
  return state.arguments.get(id) || { error: 'Argument not found', query: id };
}

function getPhenomenon(id) {
  return state.phenomena.get(id) || { error: 'Phenomenon not found', query: id };
}

function listTheories() {
  return Array.from(state.theories.values()).map(t => ({
    name: t.name,
    thesis: t.thesis,
    slogan: t.slogan
  }));
}

function listArguments() {
  return Array.from(state.arguments.values()).map(a => ({
    name: a.name,
    supports: a.supports
  }));
}

// ═══════════════════════════════════════════════════════════════════
// ANALYSIS FUNCTIONS
// ═══════════════════════════════════════════════════════════════════

/**
 * Analyze perceptual topic
 */
function analyzePerception(topic) {
  const lower = topic.toLowerCase();

  const analysis = {
    topic,
    perspectives: {}
  };

  // Color perception
  if (lower.includes('color') || lower.includes('colour')) {
    analysis.perspectives.realist = 'Colors are objective properties of surfaces';
    analysis.perspectives.dispositionalist = 'Colors are dispositions to cause experiences';
    analysis.perspectives.eliminativist = 'Colors as we conceive them don\'t exist';
    analysis.perspectives.relationalist = 'Colors are relations between objects, observers, conditions';
  }

  // Illusion/hallucination
  if (lower.includes('illusion') || lower.includes('hallucin')) {
    analysis.perspectives.senseData = 'We perceive mental intermediaries';
    analysis.perspectives.representationalist = 'We misrepresent external objects';
    analysis.perspectives.disjunctivist = 'Fundamentally different from veridical perception';
  }

  // Objects of perception
  if (lower.includes('object') || lower.includes('see') || lower.includes('perceive')) {
    analysis.perspectives.direct = 'We perceive objects themselves directly';
    analysis.perspectives.indirect = 'We perceive via mental representations';
    analysis.perspectives.intentional = 'Perception represents objects under aspects';
  }

  // Default
  if (Object.keys(analysis.perspectives).length === 0) {
    analysis.perspectives.directRealist = 'What role for mind-independent objects?';
    analysis.perspectives.representationalist = 'What is the content of perception?';
    analysis.perspectives.phenomenologist = 'What is the structure of experience?';
    analysis.perspectives.enactive = 'How is perception constituted by action?';
  }

  analysis.questions = {
    metaphysical: 'What are we immediately aware of?',
    epistemological: 'How does perception justify belief?',
    phenomenal: 'What is it like to have this experience?',
    content: 'What does this perception represent?'
  };

  analysis.cynicNote = `*head tilt* Do we see the world or our ideas of it? φ-bounded: ${(PHI_INV * 100).toFixed(1)}% max confidence.`;
  analysis.phi = PHI_INV;

  return analysis;
}

// ═══════════════════════════════════════════════════════════════════
// STATUS AND STATS
// ═══════════════════════════════════════════════════════════════════

function formatStatus() {
  const lines = [
    '┌─────────────────────────────────────────────────────┐',
    '│     PHILOSOPHY OF PERCEPTION ENGINE                 │',
    '│     *sniff* What do we really see?                  │',
    '├─────────────────────────────────────────────────────┤',
    `│  Theories: ${state.theories.size.toString().padStart(2)}     Arguments: ${state.arguments.size.toString().padStart(2)}               │`,
    `│  Thinkers: ${state.thinkers.size.toString().padStart(2)}     Phenomena: ${state.phenomena.size.toString().padStart(2)}               │`,
    `│  Concepts: ${state.concepts.size.toString().padStart(2)}                                  │`,
    '├─────────────────────────────────────────────────────┤',
    `│  φ-bound: ${(PHI_INV * 100).toFixed(1)}% max confidence               │`,
    '│  The veil of perception or direct contact?          │',
    '└─────────────────────────────────────────────────────┘'
  ];
  return lines.join('\n');
}

function getStats() {
  return {
    theories: state.theories.size,
    thinkers: state.thinkers.size,
    concepts: state.concepts.size,
    arguments: state.arguments.size,
    phenomena: state.phenomena.size,
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
  getArgument,
  getPhenomenon,
  listTheories,
  listArguments,
  analyzePerception,
  formatStatus,
  getStats,
  PHI,
  PHI_INV,
  PHI_INV_2,
  PHI_INV_3
};
