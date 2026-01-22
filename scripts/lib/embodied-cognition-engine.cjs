#!/usr/bin/env node

/**
 * Embodied Cognition Engine - Phase 45A
 *
 * 4E Cognition: Embodied, Embedded, Enacted, Extended.
 * Mind beyond the brain, cognition in action.
 *
 * φ-bounded: max 61.8% confidence
 *
 * *sniff* The mind is not a computer. The body thinks.
 */

const path = require('path');
const os = require('os');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;
const PHI_INV_3 = 0.236067977499790;

// Storage
const STORAGE_DIR = path.join(os.homedir(), '.cynic', 'embodied-cognition');

// State
const state = {
  initialized: false,
  paradigms: new Map(),
  thinkers: new Map(),
  concepts: new Map(),
  debates: new Map(),
  experiments: new Map()
};

/**
 * Initialize the Embodied Cognition Engine
 */
function init() {
  if (state.initialized) {
    return { status: 'already initialized', phi: PHI_INV };
  }

  initializeParadigms();
  initializeThinkers();
  initializeConcepts();
  initializeDebates();
  initializeExperiments();

  state.initialized = true;
  return { status: 'initialized', phi: PHI_INV };
}

/**
 * Initialize cognitive paradigms
 */
function initializeParadigms() {
  // Classical Cognitivism (contrast)
  state.paradigms.set('cognitivism', {
    name: 'Classical Cognitivism',
    period: '1950s-present',
    thesis: 'Mind is computational; cognition is symbol manipulation',
    metaphor: 'Mind as computer',
    assumptions: {
      representation: 'Mind operates on internal representations',
      computation: 'Thinking is rule-governed symbol processing',
      modularity: 'Mind has specialized modules (Fodor)',
      disembodied: 'Body irrelevant to core cognition'
    },
    critique: 'Ignores role of body, environment, and action in cognition',
    strength: PHI_INV
  });

  // Embodied Cognition
  state.paradigms.set('embodied', {
    name: 'Embodied Cognition',
    thesis: 'Cognition is shaped by the body',
    slogan: 'The body shapes the mind',
    claims: {
      weak: 'Body influences cognition (uncontroversial)',
      moderate: 'Body constrains and structures cognition',
      strong: 'Body constitutes cognition (radical)'
    },
    evidence: [
      'Conceptual metaphors grounded in bodily experience',
      'Motor system involved in language comprehension',
      'Emotion and cognition deeply intertwined',
      'Spatial reasoning tied to bodily navigation'
    ],
    lakoffJohnson: 'Abstract concepts grounded in bodily metaphors',
    strength: PHI_INV
  });

  // Embedded Cognition
  state.paradigms.set('embedded', {
    name: 'Embedded Cognition',
    thesis: 'Cognition depends on environmental scaffolding',
    slogan: 'Mind in context',
    claims: {
      scaffolding: 'Environment provides cognitive support',
      offloading: 'We use environment to reduce cognitive load',
      niche: 'Organisms construct cognitive niches'
    },
    examples: [
      'Using pen and paper for calculation',
      'Road signs reducing navigation demands',
      'Social institutions structuring decisions'
    ],
    distinction: 'Environment aids cognition but may not constitute it',
    strength: PHI_INV
  });

  // Enacted/Enactive Cognition
  state.paradigms.set('enactive', {
    name: 'Enactivism',
    founders: ['Francisco Varela', 'Evan Thompson', 'Eleanor Rosch'],
    work: 'The Embodied Mind (1991)',
    thesis: 'Cognition is enacted through bodily interaction with environment',
    slogan: 'Cognition as action',
    keyIdeas: {
      autonomy: 'Living systems are self-organizing, autonomous',
      sensemaking: 'Organisms enact worlds of significance',
      structural: 'Cognition arises from structural coupling with environment',
      phenomenology: 'First-person experience central to understanding mind'
    },
    autopoiesis: 'Living systems self-produce and maintain their organization',
    sensorimotor: 'Perception and action are inseparable',
    strength: PHI_INV
  });

  // Extended Mind
  state.paradigms.set('extended', {
    name: 'Extended Mind',
    founders: ['Andy Clark', 'David Chalmers'],
    paper: 'The Extended Mind (1998)',
    thesis: 'Cognitive processes can extend beyond brain into world',
    slogan: 'Where does the mind stop and the world begin?',
    parityPrinciple: 'If external process plays same role as internal, treat as cognitive',
    ottosNotebook: {
      scenario: 'Otto uses notebook like Inga uses biological memory',
      conclusion: 'Otto\'s notebook is part of his cognitive system',
      criteria: ['Reliable', 'Accessible', 'Trusted', 'Retrieved']
    },
    implications: {
      self: 'Boundaries of self may extend into world',
      technology: 'Smartphones as cognitive extensions',
      responsibility: 'Who is responsible for extended cognition?'
    },
    strength: PHI_INV
  });

  // Radical Enactivism
  state.paradigms.set('radical-enactivism', {
    name: 'Radical Enactivism (REC)',
    founders: ['Daniel Hutto', 'Erik Myin'],
    thesis: 'Basic cognition is contentless; no representations needed',
    work: 'Radicalizing Enactivism (2013)',
    claims: {
      contentless: 'Basic minds have no representational content',
      extensive: 'Content-involving cognition is late achievement',
      natural: 'Explains how content could arise from contentless origins'
    },
    hardProblem: 'How does contentful thought emerge from contentless interaction?',
    strength: PHI_INV_2
  });

  // Predictive Processing
  state.paradigms.set('predictive', {
    name: 'Predictive Processing',
    founders: ['Karl Friston', 'Andy Clark', 'Jakob Hohwy'],
    thesis: 'Brain is prediction machine minimizing prediction error',
    keyIdeas: {
      prediction: 'Brain generates predictions about sensory input',
      error: 'Mismatch between prediction and input is prediction error',
      updating: 'Minimize error by updating predictions or acting',
      hierarchy: 'Predictions at multiple levels of abstraction'
    },
    freeEnergy: 'Organisms minimize free energy (surprise)',
    activeInference: 'Act to make predictions come true',
    embodied: 'Compatible with embodied/enactive approaches',
    strength: PHI_INV
  });
}

/**
 * Initialize thinkers
 */
function initializeThinkers() {
  state.thinkers.set('clark', {
    name: 'Andy Clark',
    dates: '1957-present',
    works: ['Being There', 'Supersizing the Mind', 'Surfing Uncertainty'],
    contributions: {
      extended: 'Extended mind thesis (with Chalmers)',
      scaffolding: 'Cognitive scaffolding concept',
      predictive: 'Popularized predictive processing',
      cyborg: '"We are natural-born cyborgs"'
    },
    position: 'Liberal functionalism about mind',
    phi: PHI_INV
  });

  state.thinkers.set('varela', {
    name: 'Francisco Varela',
    dates: '1946-2001',
    works: ['The Embodied Mind', 'Principles of Biological Autonomy'],
    contributions: {
      autopoiesis: 'Co-developed autopoiesis with Maturana',
      enactivism: 'Founded enactive cognitive science',
      neurophenomenology: 'First-person methods for cognitive science',
      buddhism: 'Integrated Buddhist philosophy with cognitive science'
    },
    influence: 'Major figure bridging biology, phenomenology, and mind',
    phi: PHI_INV
  });

  state.thinkers.set('thompson', {
    name: 'Evan Thompson',
    dates: '1962-present',
    works: ['The Embodied Mind', 'Mind in Life', 'Waking Dreaming Being'],
    contributions: {
      enactivism: 'Developed enactivist theory',
      life: 'Deep continuity between life and mind',
      phenomenology: 'Applied Husserl and Merleau-Ponty to cognitive science',
      consciousness: 'Phenomenological approach to consciousness'
    },
    phi: PHI_INV
  });

  state.thinkers.set('noe', {
    name: 'Alva Noë',
    dates: '1964-present',
    works: ['Action in Perception', 'Out of Our Heads'],
    contributions: {
      enactive: 'Enactive approach to perception',
      sensorimotor: 'Perception requires sensorimotor knowledge',
      antiInternalism: 'Consciousness is not in the brain',
      skillful: 'Perceiving is a skillful activity'
    },
    slogan: 'Consciousness is something we do, not something that happens in us',
    phi: PHI_INV
  });

  state.thinkers.set('gallagher', {
    name: 'Shaun Gallagher',
    dates: '1948-present',
    works: ['How the Body Shapes the Mind', 'Enactivist Interventions'],
    contributions: {
      bodyImage: 'Body image vs. body schema distinction',
      intersubjectivity: 'Embodied approach to understanding others',
      interaction: 'Interaction theory of social cognition',
      phenomenology: 'Applied phenomenology to embodied cognition'
    },
    phi: PHI_INV
  });

  state.thinkers.set('dreyfus', {
    name: 'Hubert Dreyfus',
    dates: '1929-2017',
    works: ['What Computers Can\'t Do', 'Being-in-the-World'],
    contributions: {
      aiCritique: 'Critique of GOFAI (Good Old-Fashioned AI)',
      heidegger: 'Applied Heidegger to cognitive science',
      skillful: 'Skillful coping as basic mode of engagement',
      expertise: 'Phenomenology of skill acquisition'
    },
    legacy: 'Anticipated embodied AI movement',
    phi: PHI_INV
  });

  state.thinkers.set('lakoff', {
    name: 'George Lakoff',
    dates: '1941-present',
    works: ['Metaphors We Live By', 'Philosophy in the Flesh'],
    contributions: {
      conceptualMetaphor: 'Abstract thought grounded in bodily metaphors',
      imageSchemas: 'Recurring patterns from bodily experience',
      embodiedReason: 'Reason is embodied, not disembodied',
      framing: 'Frames structure political and moral thought'
    },
    examples: ['ARGUMENT IS WAR', 'TIME IS MONEY', 'LIFE IS A JOURNEY'],
    phi: PHI_INV
  });

  state.thinkers.set('gibson', {
    name: 'James J. Gibson',
    dates: '1904-1979',
    works: ['The Ecological Approach to Visual Perception'],
    contributions: {
      affordances: 'Environment offers action possibilities',
      direct: 'Perception is direct, not mediated by representations',
      ecological: 'Study perception in relation to environment',
      ambient: 'Ambient optic array contains information'
    },
    influence: 'Foundational for embodied and ecological approaches',
    phi: PHI_INV
  });

  state.thinkers.set('merleau-ponty', {
    name: 'Maurice Merleau-Ponty',
    dates: '1908-1961',
    works: ['Phenomenology of Perception'],
    contributions: {
      livedBody: 'Body-subject, not body-object',
      motorIntentionality: 'Bodily know-how precedes explicit knowledge',
      flesh: 'Intertwining of perceiver and perceived',
      ambiguity: 'Embodied existence is inherently ambiguous'
    },
    influence: 'Philosophical foundation for embodied cognition',
    phi: PHI_INV
  });
}

/**
 * Initialize concepts
 */
function initializeConcepts() {
  state.concepts.set('affordances', {
    name: 'Affordances',
    source: 'James Gibson',
    definition: 'Possibilities for action offered by environment',
    examples: {
      chair: 'Affords sitting',
      door: 'Affords passing through',
      cliff: 'Affords falling (negative affordance)'
    },
    properties: {
      relational: 'Relative to organism\'s capabilities',
      direct: 'Directly perceived, not inferred',
      action: 'Tied to potential actions'
    },
    extensions: {
      social: 'Social affordances (Rietveld)',
      cultural: 'Conventional affordances (Norman)'
    },
    phi: PHI_INV
  });

  state.concepts.set('body-schema', {
    name: 'Body Schema vs. Body Image',
    distinction: {
      schema: {
        definition: 'Sensorimotor capacities for action',
        nature: 'Prereflective, automatic',
        function: 'Regulates posture and movement'
      },
      image: {
        definition: 'Conscious representations of body',
        nature: 'Reflective, conceptual',
        function: 'Beliefs, attitudes about body'
      }
    },
    gallagher: 'Schema operates without representation',
    pathology: 'Phantom limbs, neglect reveal distinction',
    phi: PHI_INV
  });

  state.concepts.set('sensorimotor-contingencies', {
    name: 'Sensorimotor Contingencies',
    source: 'O\'Regan & Noë',
    definition: 'Laws governing sensory changes produced by actions',
    thesis: 'Perception requires mastery of sensorimotor contingencies',
    examples: {
      vision: 'Eye movements produce predictable visual changes',
      touch: 'Hand movements produce tactile patterns',
      audition: 'Head movements change auditory input'
    },
    implication: 'Perception is a form of skillful bodily activity',
    phi: PHI_INV
  });

  state.concepts.set('cognitive-offloading', {
    name: 'Cognitive Offloading',
    definition: 'Using external resources to reduce cognitive demands',
    types: {
      spatial: 'Arranging environment (e.g., to-do piles)',
      temporal: 'Using timers, calendars',
      social: 'Relying on others\' memory',
      technological: 'Using smartphones, notes'
    },
    benefits: 'Frees up cognitive resources',
    risks: 'Dependency, skill atrophy',
    phi: PHI_INV
  });

  state.concepts.set('structural-coupling', {
    name: 'Structural Coupling',
    source: 'Maturana & Varela',
    definition: 'History of recurrent interactions between organism and environment',
    features: {
      mutual: 'Both organism and environment change',
      historical: 'Shaped by interaction history',
      plastic: 'Allows for learning and adaptation'
    },
    cognition: 'Cognition emerges from structural coupling',
    phi: PHI_INV
  });

  state.concepts.set('autopoiesis', {
    name: 'Autopoiesis',
    source: 'Maturana & Varela',
    definition: 'Self-production: system produces components that produce it',
    features: {
      closure: 'Organizationally closed',
      boundary: 'Produces own boundary',
      autonomy: 'Self-determining, not input-output machine'
    },
    living: 'Defining feature of living systems',
    cognition: 'Mind as autopoietic organization',
    phi: PHI_INV
  });

  state.concepts.set('situated-action', {
    name: 'Situated Action',
    source: 'Lucy Suchman',
    definition: 'Actions emerge from concrete circumstances',
    thesis: 'Plans don\'t determine action; they\'re resources for situated action',
    contrast: {
      planning: 'Cognitivist view: action follows from plans',
      situated: 'Plans are post-hoc rationalizations or rough guides'
    },
    implications: 'Design must account for situated use',
    phi: PHI_INV
  });

  state.concepts.set('enactive-perception', {
    name: 'Enactive Perception',
    thesis: 'Perception is constituted by sensorimotor activity',
    contrast: {
      representationalist: 'Perception builds internal model',
      enactive: 'Perception is exploratory activity'
    },
    noe: 'We don\'t perceive by building internal pictures',
    evidence: ['Change blindness', 'Inattentional blindness', 'Active vision'],
    phi: PHI_INV
  });
}

/**
 * Initialize debates
 */
function initializeDebates() {
  state.debates.set('extended-vs-embedded', {
    name: 'Extended vs. Embedded Mind',
    question: 'Is external scaffolding part of cognition or just a causal aid?',
    positions: {
      extended: 'External processes can be constitutive of cognition (Clark)',
      embedded: 'External processes aid but don\'t constitute cognition (Rupert)',
      hypothesis: 'Hypothesis of Embedded Cognition (HEC) vs. Extended (HEM)'
    },
    couplingConstitution: {
      argument: 'Causal coupling doesn\'t imply constitution',
      response: 'Parity principle: functional role matters'
    },
    phi: PHI_INV
  });

  state.debates.set('representations', {
    name: 'Do We Need Representations?',
    question: 'Does cognition require internal representations?',
    positions: {
      pro: {
        supporters: ['Fodor', 'Pylyshyn', 'Millikan'],
        argument: 'Productivity and systematicity require representations'
      },
      anti: {
        supporters: ['Brooks', 'Chemero', 'Hutto & Myin'],
        argument: 'Behavior explicable without representations'
      },
      moderate: {
        supporters: ['Clark', 'Wheeler'],
        argument: 'Some cognition is representational, some isn\'t'
      }
    },
    phi: PHI_INV
  });

  state.debates.set('mark-of-cognitive', {
    name: 'The Mark of the Cognitive',
    question: 'What distinguishes cognitive from non-cognitive processes?',
    problem: 'If cognition extends, what are its boundaries?',
    proposals: {
      intrinsic: 'Cognitive processes have intrinsic features',
      functional: 'Cognitive role defined functionally',
      none: 'No principled mark; cognition is not natural kind'
    },
    bloatingObjection: 'Without mark, everything becomes cognitive',
    phi: PHI_INV
  });

  state.debates.set('consciousness-embodiment', {
    name: 'Consciousness and Embodiment',
    question: 'Is embodiment necessary for consciousness?',
    positions: {
      necessary: 'Consciousness requires embodied interaction (Thompson)',
      contingent: 'Embodiment actual but not necessary (functionalism)',
      irrelevant: 'Consciousness is brain phenomenon (internalism)'
    },
    implications: 'Could disembodied AI be conscious?',
    phi: PHI_INV
  });

  state.debates.set('social-cognition', {
    name: 'Social Cognition: Theory or Interaction?',
    question: 'How do we understand other minds?',
    positions: {
      theoryTheory: 'We theorize about others\' mental states',
      simulation: 'We simulate being in others\' situation',
      interaction: 'Understanding emerges in embodied interaction (Gallagher)',
      direct: 'We directly perceive others\' mental states in behavior'
    },
    embodied: 'Primary intersubjectivity is bodily, not mentalistic',
    phi: PHI_INV
  });
}

/**
 * Initialize key experiments and evidence
 */
function initializeExperiments() {
  state.experiments.set('rubber-hand', {
    name: 'Rubber Hand Illusion',
    researchers: 'Botvinick & Cohen (1998)',
    procedure: 'Synchronous stroking of hidden real hand and visible rubber hand',
    result: 'Subjects feel rubber hand is their own',
    significance: 'Body representation is malleable and multisensory',
    phi: PHI_INV
  });

  state.experiments.set('embodied-metaphor', {
    name: 'Embodied Metaphor Effects',
    researchers: 'Lakoff, Zhong & Liljenquist',
    examples: {
      warmth: 'Holding warm cup increases warm social judgments',
      cleanliness: 'Physical cleaning reduces moral guilt',
      weight: 'Heavy clipboard makes issues seem more important'
    },
    significance: 'Abstract concepts grounded in bodily experience',
    phi: PHI_INV
  });

  state.experiments.set('sensory-substitution', {
    name: 'Sensory Substitution',
    researchers: 'Bach-y-Rita',
    procedure: 'Visual information conveyed through tactile stimulation',
    result: 'Blind subjects can "see" through touch after training',
    significance: 'Perception is sensorimotor skill, not modality-specific',
    phi: PHI_INV
  });

  state.experiments.set('tool-use', {
    name: 'Tool Use and Body Schema',
    finding: 'Tool use extends body schema',
    evidence: {
      monkeys: 'Neurons extend receptive fields to include tool',
      humans: 'Reaching changes after tool use',
      crossmodal: 'Tool becomes extension of body representation'
    },
    implication: 'Body boundaries are plastic and extend into world',
    phi: PHI_INV
  });
}

// ═══════════════════════════════════════════════════════════════════
// QUERY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════

function getParadigm(id) {
  return state.paradigms.get(id) || { error: 'Paradigm not found', query: id };
}

function getThinker(id) {
  return state.thinkers.get(id) || { error: 'Thinker not found', query: id };
}

function getConcept(id) {
  return state.concepts.get(id) || { error: 'Concept not found', query: id };
}

function getDebate(id) {
  return state.debates.get(id) || { error: 'Debate not found', query: id };
}

function getExperiment(id) {
  return state.experiments.get(id) || { error: 'Experiment not found', query: id };
}

function listParadigms() {
  return Array.from(state.paradigms.values()).map(p => ({
    name: p.name,
    thesis: p.thesis,
    slogan: p.slogan
  }));
}

function listThinkers() {
  return Array.from(state.thinkers.values()).map(t => ({
    name: t.name,
    dates: t.dates,
    key: Object.keys(t.contributions)[0]
  }));
}

// ═══════════════════════════════════════════════════════════════════
// ANALYSIS FUNCTIONS
// ═══════════════════════════════════════════════════════════════════

/**
 * Analyze topic from embodied cognition perspective
 */
function analyzeEmbodied(topic) {
  const lower = topic.toLowerCase();

  const analysis = {
    topic,
    perspectives: {}
  };

  // Perception topics
  if (lower.includes('percept') || lower.includes('vision') || lower.includes('see')) {
    analysis.perspectives.enactive = 'Perception is skillful bodily activity, not passive reception';
    analysis.perspectives.sensorimotor = 'Requires mastery of sensorimotor contingencies';
    analysis.perspectives.direct = 'Information directly available in environment (Gibson)';
  }

  // Memory topics
  if (lower.includes('memory') || lower.includes('remember')) {
    analysis.perspectives.extended = 'Memory can extend into notebooks, phones, others';
    analysis.perspectives.embodied = 'Body participates in memory (procedural, emotional)';
    analysis.perspectives.distributed = 'Memory distributed across brain, body, environment';
  }

  // Action topics
  if (lower.includes('action') || lower.includes('behav') || lower.includes('movement')) {
    analysis.perspectives.situated = 'Action emerges from situation, not just plans';
    analysis.perspectives.affordance = 'Environment specifies action possibilities';
    analysis.perspectives.skillful = 'Skilled coping is basic mode of engagement';
  }

  // Thought/reasoning
  if (lower.includes('think') || lower.includes('reason') || lower.includes('concept')) {
    analysis.perspectives.grounded = 'Concepts grounded in bodily experience';
    analysis.perspectives.metaphor = 'Abstract thought structured by embodied metaphors';
    analysis.perspectives.simulation = 'Thinking involves motor simulation';
  }

  // Default
  if (Object.keys(analysis.perspectives).length === 0) {
    analysis.perspectives.embodied = 'How does the body shape this process?';
    analysis.perspectives.extended = 'Does this process extend beyond the brain?';
    analysis.perspectives.enactive = 'How is this enacted through interaction?';
    analysis.perspectives.ecological = 'What role does environment play?';
  }

  analysis.questions = {
    body: 'What role does the body play?',
    environment: 'How does environment participate?',
    action: 'What is the relation to action?',
    representation: 'Are representations necessary?'
  };

  analysis.cynicNote = `*sniff* The mind is not a ghost in a machine. The body thinks. φ-bounded: ${(PHI_INV * 100).toFixed(1)}% max confidence.`;
  analysis.phi = PHI_INV;

  return analysis;
}

// ═══════════════════════════════════════════════════════════════════
// STATUS AND STATS
// ═══════════════════════════════════════════════════════════════════

function formatStatus() {
  const lines = [
    '┌─────────────────────────────────────────────────────┐',
    '│     EMBODIED COGNITION ENGINE                       │',
    '│     *sniff* The body thinks                         │',
    '├─────────────────────────────────────────────────────┤',
    `│  Paradigms: ${state.paradigms.size.toString().padStart(2)}    Experiments: ${state.experiments.size.toString().padStart(2)}            │`,
    `│  Thinkers: ${state.thinkers.size.toString().padStart(2)}     Debates: ${state.debates.size.toString().padStart(2)}                   │`,
    `│  Concepts: ${state.concepts.size.toString().padStart(2)}                                  │`,
    '├─────────────────────────────────────────────────────┤',
    `│  φ-bound: ${(PHI_INV * 100).toFixed(1)}% max confidence               │`,
    '│  Mind beyond the brain. Cognition in action.        │',
    '└─────────────────────────────────────────────────────┘'
  ];
  return lines.join('\n');
}

function getStats() {
  return {
    paradigms: state.paradigms.size,
    thinkers: state.thinkers.size,
    concepts: state.concepts.size,
    debates: state.debates.size,
    experiments: state.experiments.size,
    phi: PHI_INV
  };
}

// ═══════════════════════════════════════════════════════════════════
// EXPORTS
// ═══════════════════════════════════════════════════════════════════

module.exports = {
  init,
  getParadigm,
  getThinker,
  getConcept,
  getDebate,
  getExperiment,
  listParadigms,
  listThinkers,
  analyzeEmbodied,
  formatStatus,
  getStats,
  PHI,
  PHI_INV,
  PHI_INV_2,
  PHI_INV_3
};
