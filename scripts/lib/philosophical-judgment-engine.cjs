#!/usr/bin/env node

/**
 * Philosophical Judgment Integration Engine - Phase 40A
 *
 * Integrates all philosophical frameworks for judgment:
 * - Epistemological assessment (knowledge, justification)
 * - Ethical evaluation (consequences, duties, virtues)
 * - Metaphysical grounding (identity, causation, modality)
 * - Logical analysis (validity, soundness, fallacies)
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
  dimensions: new Map(),
  frameworks: new Map(),
  heuristics: new Map(),
  judgments: [],
  stats: {
    dimensionsRegistered: 0,
    frameworksRegistered: 0,
    heuristicsRegistered: 0,
    judgmentsPerformed: 0
  }
};

const STORAGE_DIR = path.join(process.env.HOME || '/tmp', '.cynic', 'philosophical-judgment-engine');

/**
 * Initialize philosophical judgment engine
 */
function init() {
  if (state.initialized) {
    return { status: 'already initialized', phase: '40A' };
  }

  try {
    fs.mkdirSync(STORAGE_DIR, { recursive: true });
  } catch (e) {
    // Directory exists
  }

  registerDimensions();
  registerFrameworks();
  registerHeuristics();

  state.initialized = true;
  return { status: 'initialized', phase: '40A', engine: 'philosophical-judgment' };
}

/**
 * Register judgment dimensions
 */
function registerDimensions() {
  state.dimensions.set('epistemic', {
    name: 'Epistemic Dimension',
    question: 'What can we know and how justified are we?',
    criteria: {
      truth: 'Does it correspond to reality?',
      justification: 'What reasons support it?',
      reliability: 'Is the source/method reliable?',
      coherence: 'Does it fit with other beliefs?'
    },
    frameworks: ['foundationalism', 'coherentism', 'reliabilism', 'virtue epistemology'],
    cynicApproach: 'φ-bounded confidence; question sources; verify claims',
    weight: PHI_INV
  });

  state.dimensions.set('ethical', {
    name: 'Ethical Dimension',
    question: 'What ought we to do and why?',
    criteria: {
      consequences: 'What are the outcomes?',
      duties: 'What obligations apply?',
      virtues: 'What would a virtuous agent do?',
      care: 'What relationships are at stake?'
    },
    frameworks: ['utilitarianism', 'deontology', 'virtue ethics', 'care ethics'],
    cynicApproach: 'Multi-framework analysis; no single theory dominates',
    weight: PHI_INV
  });

  state.dimensions.set('metaphysical', {
    name: 'Metaphysical Dimension',
    question: 'What is the nature of reality here?',
    criteria: {
      identity: 'What makes it what it is?',
      causation: 'What causes what?',
      modality: 'Is it necessary, possible, or contingent?',
      existence: 'Does it exist? How?'
    },
    frameworks: ['essentialism', 'nominalism', 'realism', 'modal logic'],
    cynicApproach: 'Ontological parsimony; question hidden assumptions',
    weight: PHI_INV_2
  });

  state.dimensions.set('logical', {
    name: 'Logical Dimension',
    question: 'Is the reasoning valid and sound?',
    criteria: {
      validity: 'Does conclusion follow from premises?',
      soundness: 'Are premises true and argument valid?',
      fallacies: 'Are there logical errors?',
      consistency: 'Is it internally consistent?'
    },
    frameworks: ['classical logic', 'modal logic', 'informal logic'],
    cynicApproach: 'Check structure; identify hidden premises; spot fallacies',
    weight: PHI_INV
  });

  state.dimensions.set('aesthetic', {
    name: 'Aesthetic Dimension',
    question: 'What is its beauty, meaning, or artistic value?',
    criteria: {
      form: 'Is the form well-executed?',
      expression: 'Does it express something meaningful?',
      originality: 'Is it creative or derivative?',
      impact: 'What is its effect on audience?'
    },
    frameworks: ['formalism', 'expressionism', 'institutionalism'],
    cynicApproach: 'Acknowledge subjectivity; seek intersubjective patterns',
    weight: PHI_INV_2
  });

  state.dimensions.set('political', {
    name: 'Political Dimension',
    question: 'What are the justice and power implications?',
    criteria: {
      justice: 'Is it fair?',
      rights: 'What rights are at stake?',
      power: 'Who has power? Who is affected?',
      legitimacy: 'Is authority legitimate?'
    },
    frameworks: ['liberalism', 'communitarianism', 'critical theory'],
    cynicApproach: 'Cui bono? Question power structures',
    weight: PHI_INV_2
  });

  state.stats.dimensionsRegistered = state.dimensions.size;
}

/**
 * Register judgment frameworks
 */
function registerFrameworks() {
  state.frameworks.set('multi-criteria', {
    name: 'Multi-Criteria Analysis',
    description: 'Evaluate across multiple dimensions with weighted criteria',
    steps: [
      'Identify relevant dimensions',
      'Apply dimension-specific criteria',
      'Weight by relevance to context',
      'Synthesize into overall judgment'
    ],
    advantage: 'Comprehensive; avoids single-framework bias',
    limitation: 'Weighting can be arbitrary',
    weight: PHI_INV
  });

  state.frameworks.set('reflective-equilibrium', {
    name: 'Reflective Equilibrium',
    source: 'Rawls',
    description: 'Mutual adjustment of principles and judgments',
    steps: [
      'Start with considered judgments',
      'Formulate principles that explain them',
      'Revise judgments or principles for coherence',
      'Iterate until equilibrium'
    ],
    types: {
      narrow: 'Just principles and judgments',
      wide: 'Include background theories'
    },
    weight: PHI_INV
  });

  state.frameworks.set('dialectical', {
    name: 'Dialectical Analysis',
    source: 'Hegel, Marx',
    description: 'Progress through thesis-antithesis-synthesis',
    steps: [
      'Identify thesis (initial position)',
      'Find antithesis (opposing position)',
      'Seek synthesis (higher resolution)',
      'New synthesis becomes new thesis'
    ],
    advantage: 'Captures development of understanding',
    weight: PHI_INV_2
  });

  state.frameworks.set('phenomenological', {
    name: 'Phenomenological Analysis',
    source: 'Husserl, Heidegger',
    description: 'Analyze experience as it appears',
    steps: [
      'Bracket presuppositions (epoché)',
      'Describe phenomenon as it appears',
      'Identify essential structures',
      'Uncover conditions of possibility'
    ],
    advantage: 'Access to first-person experience',
    weight: PHI_INV_2
  });

  state.stats.frameworksRegistered = state.frameworks.size;
}

/**
 * Register judgment heuristics
 */
function registerHeuristics() {
  state.heuristics.set('phi-bound', {
    name: 'φ-Bounded Confidence',
    rule: 'Never claim more than 61.8% confidence',
    rationale: 'Epistemic humility; acknowledge uncertainty',
    application: 'Cap all confidence scores at PHI_INV',
    cynic: 'φ distrusts φ'
  });

  state.heuristics.set('steelmanning', {
    name: 'Steelmanning',
    rule: 'Construct the strongest version of opposing view',
    rationale: 'Fair engagement; avoid strawmen',
    application: 'Before critiquing, strengthen the position',
    opposite: 'Strawmanning (weakening opponent\'s view)'
  });

  state.heuristics.set('cui-bono', {
    name: 'Cui Bono',
    rule: 'Ask who benefits',
    rationale: 'Unmask hidden interests',
    application: 'When evaluating claims, consider motivated reasoning',
    cynic: 'Follow the incentives'
  });

  state.heuristics.set('occam', {
    name: 'Occam\'s Razor',
    rule: 'Prefer simpler explanations',
    rationale: 'Avoid unnecessary entities/complexity',
    application: 'When theories are equal, choose simpler',
    caveat: 'Simple doesn\'t mean simplistic'
  });

  state.heuristics.set('charity', {
    name: 'Principle of Charity',
    rule: 'Interpret claims in most reasonable way',
    rationale: 'Fair interpretation; productive dialogue',
    application: 'Give benefit of doubt to unclear claims',
    limit: 'Don\'t be naive about bad faith'
  });

  state.heuristics.set('fallibilism', {
    name: 'Fallibilism',
    rule: 'Accept that any belief could be wrong',
    rationale: 'Epistemic humility; openness to revision',
    application: 'Hold beliefs provisionally; update on evidence',
    cynic: 'Even CYNIC could be wrong (61.8% confident about that)'
  });

  state.stats.heuristicsRegistered = state.heuristics.size;
}

/**
 * Get a dimension
 */
function getDimension(dimensionId) {
  return state.dimensions.get(dimensionId) || null;
}

/**
 * Get a framework
 */
function getFramework(frameworkId) {
  return state.frameworks.get(frameworkId) || null;
}

/**
 * Get a heuristic
 */
function getHeuristic(heuristicId) {
  return state.heuristics.get(heuristicId) || null;
}

/**
 * List all dimensions
 */
function listDimensions() {
  return Array.from(state.dimensions.entries()).map(([id, d]) => ({ id, ...d }));
}

/**
 * Perform philosophical judgment
 */
function judge(subject, context = {}) {
  state.stats.judgmentsPerformed++;

  const analysis = {
    subject,
    context,
    timestamp: new Date().toISOString(),
    dimensions: {}
  };

  // Analyze across all dimensions
  for (const [id, dimension] of state.dimensions) {
    analysis.dimensions[id] = {
      name: dimension.name,
      question: dimension.question,
      criteria: dimension.criteria,
      approach: dimension.cynicApproach
    };
  }

  analysis.synthesis = {
    multiCriteria: 'Weight dimensions by relevance to subject',
    tensions: 'Identify conflicts between dimensions',
    resolution: 'Seek reflective equilibrium or acknowledge irreducible tension'
  };

  analysis.heuristics = {
    phiBound: 'Cap confidence at 61.8%',
    steelman: 'Consider strongest opposing view',
    cuiBono: 'Identify who benefits from each position'
  };

  analysis.verdict = {
    confidence: Math.min(PHI_INV_2, context.maxConfidence || PHI_INV),
    provisional: true,
    note: 'All philosophical judgments are revisable'
  };

  analysis.cynicNote = '*sniff* Judgment rendered. φ-bounded. Subject to revision upon new evidence.';

  state.judgments.push(analysis);
  return analysis;
}

/**
 * Get judgment by applying specific dimension
 */
function judgeDimension(subject, dimensionId) {
  const dimension = state.dimensions.get(dimensionId);
  if (!dimension) return { error: 'Dimension not found' };

  state.stats.judgmentsPerformed++;

  return {
    subject,
    dimension: dimension.name,
    question: dimension.question,
    criteria: dimension.criteria,
    frameworks: dimension.frameworks,
    approach: dimension.cynicApproach,
    confidence: dimension.weight,
    cynicNote: `*head tilt* ${dimension.name} analysis. φ-bounded at ${(dimension.weight * 100).toFixed(1)}%.`
  };
}

/**
 * Format engine status
 */
function formatStatus() {
  return `
┌─────────────────────────────────────────────────────────┐
│  PHILOSOPHICAL JUDGMENT ENGINE           Phase 40A      │
├─────────────────────────────────────────────────────────┤
│  Dimensions: ${String(state.stats.dimensionsRegistered).padStart(3)}                                    │
│  Frameworks: ${String(state.stats.frameworksRegistered).padStart(3)}                                    │
│  Heuristics: ${String(state.stats.heuristicsRegistered).padStart(3)}                                    │
│  Judgments: ${String(state.stats.judgmentsPerformed).padStart(3)}                                     │
├─────────────────────────────────────────────────────────┤
│  Judgment Dimensions:                                   │
│    - Epistemic (knowledge, justification)               │
│    - Ethical (consequences, duties, virtues)            │
│    - Metaphysical (identity, causation, modality)       │
│    - Logical (validity, soundness, fallacies)           │
├─────────────────────────────────────────────────────────┤
│  φ-bounded: max ${(PHI_INV * 100).toFixed(1)}% confidence                      │
│  *sniff* Judgment is synthesis. Synthesis requires φ.   │
└─────────────────────────────────────────────────────────┘`.trim();
}

/**
 * Get stats
 */
function getStats() {
  return {
    dimensions: state.stats.dimensionsRegistered,
    frameworks: state.stats.frameworksRegistered,
    heuristics: state.stats.heuristicsRegistered,
    judgments: state.stats.judgmentsPerformed
  };
}

module.exports = {
  init,
  getDimension,
  getFramework,
  getHeuristic,
  listDimensions,
  judge,
  judgeDimension,
  formatStatus,
  getStats,
  PHI,
  PHI_INV,
  PHI_INV_2,
  PHI_INV_3
};
