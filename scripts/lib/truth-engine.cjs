/**
 * Truth Engine - Theories of Truth
 *
 * "'Snow is white' is true if and only if snow is white."
 * — Tarski's T-schema
 *
 * Implements:
 * - Correspondence theory
 * - Coherence theory
 * - Pragmatic theory
 * - Deflationism (disquotation)
 * - Tarski's semantic conception
 *
 * φ guides confidence in truth evaluations.
 */

const fs = require('fs');
const path = require('path');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;      // 61.8% - max confidence
const PHI_INV_2 = 0.381966011250105;    // 38.2%
const PHI_INV_3 = 0.236067977499790;    // 23.6%

// Storage
const STORAGE_DIR = path.join(require('os').homedir(), '.cynic', 'truth');

// Major theories of truth
const TRUTH_THEORIES = {
  correspondence: {
    name: 'Correspondence Theory',
    slogan: 'Truth is correspondence with facts',
    definition: 'A proposition is true iff it corresponds to a fact',
    proponents: ['Aristotle', 'Russell', 'early Wittgenstein'],
    strengths: ['Intuitive', 'Explains truth-making'],
    weaknesses: ['What are facts?', 'What is correspondence?'],
    aristoteQuote: 'To say of what is that it is, and of what is not that it is not, is true'
  },
  coherence: {
    name: 'Coherence Theory',
    slogan: 'Truth is coherence with a system of beliefs',
    definition: 'A proposition is true iff it coheres with a specified set of propositions',
    proponents: ['Bradley', 'Blanshard', 'Rescher'],
    strengths: ['Explains justification', 'No need for inaccessible facts'],
    weaknesses: ['Coherent fictions?', 'Which system?'],
    note: 'Often paired with idealism'
  },
  pragmatic: {
    name: 'Pragmatic Theory',
    slogan: 'Truth is what works',
    definition: 'A proposition is true iff believing it leads to successful practice',
    proponents: ['James', 'Peirce', 'Dewey'],
    strengths: ['Connects truth to practice', 'Anti-metaphysical'],
    weaknesses: ['Useful falsehoods?', 'Truth for whom?'],
    jamesQuote: 'The true is only the expedient in our way of thinking'
  },
  deflationary: {
    name: 'Deflationary Theory',
    slogan: 'Truth is not a substantial property',
    definition: '"p is true" says no more than p',
    proponents: ['Ramsey', 'Quine', 'Horwich'],
    strengths: ['Metaphysically parsimonious', 'Explains T-schema'],
    weaknesses: ['Too thin?', 'What about truth-bearers?'],
    key: 'Disquotational: "Snow is white" is true iff snow is white'
  },
  semantic: {
    name: 'Semantic Theory (Tarski)',
    slogan: 'Truth is a semantic property definable for formalized languages',
    definition: 'Convention T: adequate theory entails all T-sentences',
    proponents: ['Tarski', 'Davidson'],
    strengths: ['Rigorous', 'Avoids paradoxes'],
    weaknesses: ['Only formal languages?', 'Not a theory of truth itself?'],
    tSchema: '"P" is true in L iff P'
  },
  identity: {
    name: 'Identity Theory',
    slogan: 'True propositions are identical with facts',
    definition: 'A true proposition doesn\'t correspond to a fact—it IS the fact',
    proponents: ['Moore (early)', 'Dodd'],
    strengths: ['Avoids correspondence relation'],
    weaknesses: ['What about false propositions?']
  }
};

// Truth-bearers
const TRUTH_BEARERS = {
  sentences: {
    name: 'Sentences',
    description: 'Linguistic expressions (tokens or types)',
    issues: ['Context-sensitivity', 'Translation'],
    example: '"Snow is white" (the sentence)'
  },
  propositions: {
    name: 'Propositions',
    description: 'Abstract objects expressed by sentences',
    issues: ['Metaphysical status', 'Identity conditions'],
    example: 'The proposition that snow is white'
  },
  beliefs: {
    name: 'Beliefs',
    description: 'Mental states',
    issues: ['Only derivatively true?', 'Animal beliefs?'],
    example: 'The belief that snow is white'
  },
  judgments: {
    name: 'Judgments',
    description: 'Acts of judging',
    issues: ['Event-like', 'Speaker-relative'],
    example: 'S\'s judgment that snow is white'
  }
};

// Tarski's T-sentences
const T_SCHEMA = {
  pattern: '"P" is true if and only if P',
  examples: [
    { quoted: '"Snow is white"', condition: 'snow is white' },
    { quoted: '"Grass is green"', condition: 'grass is green' },
    { quoted: '"2 + 2 = 4"', condition: '2 + 2 = 4' }
  ],
  conventionT: 'An adequate truth definition must entail all T-sentences',
  note: 'Object language vs metalanguage distinction crucial'
};

// Liar paradox
const LIAR_PARADOX = {
  statement: 'This sentence is false',
  analysis: {
    if_true: 'Then what it says is the case, so it\'s false',
    if_false: 'Then what it says is not the case, so it\'s true',
    conclusion: 'Contradiction either way'
  },
  responses: {
    tarski: 'Distinguish object language from metalanguage',
    kripke: 'Truth predicate is partially defined (grounded)',
    paraconsistent: 'Accept true contradictions (dialethism)',
    revision: 'Truth is a circular concept (Gupta)'
  }
};

// State
const state = {
  propositions: new Map(),     // Propositions being evaluated
  evaluations: new Map(),      // Truth evaluations
  coherenceSystems: new Map(), // Systems for coherence checking
  tSentences: [],              // Generated T-sentences
  paradoxAnalyses: []          // Paradox analyses
};

/**
 * Initialize the truth engine
 */
function init() {
  if (!fs.existsSync(STORAGE_DIR)) {
    fs.mkdirSync(STORAGE_DIR, { recursive: true });
  }

  // Load persisted state
  const statePath = path.join(STORAGE_DIR, 'state.json');
  if (fs.existsSync(statePath)) {
    try {
      const saved = JSON.parse(fs.readFileSync(statePath, 'utf8'));
      if (saved.propositions) state.propositions = new Map(Object.entries(saved.propositions));
      if (saved.evaluations) state.evaluations = new Map(Object.entries(saved.evaluations));
      if (saved.coherenceSystems) state.coherenceSystems = new Map(Object.entries(saved.coherenceSystems));
      if (saved.tSentences) state.tSentences = saved.tSentences;
      if (saved.paradoxAnalyses) state.paradoxAnalyses = saved.paradoxAnalyses;
    } catch {
      // Start fresh
    }
  }

  return { status: 'initialized', propositions: state.propositions.size };
}

/**
 * Save state
 */
function saveState() {
  const statePath = path.join(STORAGE_DIR, 'state.json');
  const toSave = {
    propositions: Object.fromEntries(state.propositions),
    evaluations: Object.fromEntries(state.evaluations),
    coherenceSystems: Object.fromEntries(state.coherenceSystems),
    tSentences: state.tSentences,
    paradoxAnalyses: state.paradoxAnalyses
  };
  fs.writeFileSync(statePath, JSON.stringify(toSave, null, 2));
}

/**
 * Register a proposition for truth evaluation
 */
function registerProposition(id, spec = {}) {
  const proposition = {
    id,
    content: spec.content || id,
    bearer: spec.bearer || 'proposition',
    bearerInfo: TRUTH_BEARERS[spec.bearer || 'proposition'],

    // Domain
    domain: spec.domain || 'empirical',  // empirical, mathematical, ethical, etc.

    // Status
    truthValue: null,
    evaluations: [],

    registeredAt: Date.now()
  };

  state.propositions.set(id, proposition);
  saveState();

  return proposition;
}

/**
 * Evaluate truth according to a specific theory
 */
function evaluateByTheory(propositionId, theoryId, evidence = {}) {
  const proposition = state.propositions.get(propositionId);
  const theory = TRUTH_THEORIES[theoryId];

  if (!proposition) {
    return { error: 'Proposition not found' };
  }
  if (!theory) {
    return { error: 'Theory not found', available: Object.keys(TRUTH_THEORIES) };
  }

  const evaluation = {
    id: `eval_${Date.now()}`,
    propositionId,
    content: proposition.content,
    theory: theoryId,
    theoryName: theory.name,

    // Theory-specific evaluation
    result: null,
    confidence: PHI_INV_2,
    reasoning: null,

    timestamp: Date.now()
  };

  // Evaluate based on theory
  switch (theoryId) {
    case 'correspondence':
      evaluation.reasoning = {
        question: `Does "${proposition.content}" correspond to a fact?`,
        factRequired: evidence.fact || 'unspecified',
        correspondence: evidence.corresponds ?? null
      };
      if (evidence.corresponds === true) {
        evaluation.result = 'true';
        evaluation.confidence = Math.min(evidence.confidence || PHI_INV_2, PHI_INV);
      } else if (evidence.corresponds === false) {
        evaluation.result = 'false';
        evaluation.confidence = Math.min(evidence.confidence || PHI_INV_2, PHI_INV);
      } else {
        evaluation.result = 'undetermined';
        evaluation.confidence = PHI_INV_3;
      }
      break;

    case 'coherence':
      evaluation.reasoning = {
        question: `Does "${proposition.content}" cohere with the belief system?`,
        system: evidence.system || 'default',
        coherence: evidence.coheres ?? null
      };
      if (evidence.coheres === true) {
        evaluation.result = 'true';
        evaluation.confidence = Math.min(evidence.confidence || PHI_INV_2, PHI_INV);
      } else if (evidence.coheres === false) {
        evaluation.result = 'false';
        evaluation.confidence = Math.min(evidence.confidence || PHI_INV_2, PHI_INV);
      } else {
        evaluation.result = 'undetermined';
        evaluation.confidence = PHI_INV_3;
      }
      evaluation.warning = 'Coherent fictions are possible';
      break;

    case 'pragmatic':
      evaluation.reasoning = {
        question: `Does believing "${proposition.content}" lead to successful practice?`,
        utility: evidence.utility || null,
        verification: evidence.verification || null
      };
      if (evidence.utility === 'high' || evidence.works === true) {
        evaluation.result = 'true';
        evaluation.confidence = PHI_INV_2;  // Pragmatic truth is revisable
      } else if (evidence.utility === 'low' || evidence.works === false) {
        evaluation.result = 'false';
        evaluation.confidence = PHI_INV_2;
      } else {
        evaluation.result = 'pending';
        evaluation.confidence = PHI_INV_3;
      }
      evaluation.jamesNote = 'Truth happens to an idea';
      break;

    case 'deflationary':
      evaluation.reasoning = {
        question: `"${proposition.content}" is true iff ${proposition.content}`,
        deflation: 'Truth ascription adds nothing to the proposition',
        schema: `"P" is true = P`
      };
      // Deflationism: truth predicate is merely disquotational
      evaluation.result = evidence.obtains ?? 'depends on world';
      evaluation.confidence = PHI_INV;  // High confidence in the schema
      evaluation.note = 'Truth is not a substantive property';
      break;

    case 'semantic':
      evaluation.reasoning = {
        question: 'Apply Tarski\'s T-schema',
        tSentence: `"${proposition.content}" is true iff ${proposition.content}`,
        language: evidence.language || 'natural'
      };
      evaluation.result = 'given by T-sentence';
      evaluation.confidence = PHI_INV;
      evaluation.tarskiNote = 'Truth definable for formalized languages';
      break;

    case 'identity':
      evaluation.reasoning = {
        question: `Is "${proposition.content}" identical with a fact?`,
        identity: evidence.isIdentical ?? null
      };
      evaluation.result = evidence.isIdentical ? 'true' : 'undetermined';
      evaluation.confidence = PHI_INV_3;
      break;
  }

  // Store evaluation
  proposition.evaluations.push(evaluation);
  if (evaluation.result === 'true' || evaluation.result === 'false') {
    proposition.truthValue = evaluation.result;
  }

  state.evaluations.set(evaluation.id, evaluation);
  saveState();

  return evaluation;
}

/**
 * Generate T-sentence (Tarski)
 */
function generateTSentence(content) {
  const tSentence = {
    objectLanguage: `"${content}"`,
    metalanguage: content,
    tSentence: `"${content}" is true if and only if ${content}`,
    schema: T_SCHEMA.pattern,
    tarskiNote: 'Material adequacy via Convention T',
    timestamp: Date.now()
  };

  state.tSentences.push(tSentence);
  saveState();

  return tSentence;
}

/**
 * Create coherence system
 */
function createCoherenceSystem(id, spec = {}) {
  const system = {
    id,
    name: spec.name || id,
    propositions: spec.propositions || [],
    constraints: spec.constraints || [],
    createdAt: Date.now()
  };

  state.coherenceSystems.set(id, system);
  saveState();

  return system;
}

/**
 * Check coherence
 */
function checkCoherence(propositionId, systemId) {
  const proposition = state.propositions.get(propositionId);
  const system = state.coherenceSystems.get(systemId);

  if (!proposition) {
    return { error: 'Proposition not found' };
  }
  if (!system) {
    return { error: 'Coherence system not found' };
  }

  const check = {
    propositionId,
    content: proposition.content,
    systemId,
    systemName: system.name,

    // Coherence check
    coheres: true,  // Default: assume coherent unless contradicted
    conflicts: [],
    supports: [],

    confidence: PHI_INV_2,
    timestamp: Date.now()
  };

  // Simple check: look for explicit contradictions
  for (const existingContent of system.propositions) {
    const lower = proposition.content.toLowerCase();
    const existingLower = existingContent.toLowerCase();

    // Check for negation
    if (lower === `not ${existingLower}` || existingLower === `not ${lower}` ||
        lower === `it is not the case that ${existingLower}`) {
      check.coheres = false;
      check.conflicts.push({
        conflictsWith: existingContent,
        type: 'contradiction'
      });
    }

    // Check for support (very simple)
    if (lower.includes(existingLower) || existingLower.includes(lower)) {
      check.supports.push({
        supportedBy: existingContent,
        type: 'entailment_candidate'
      });
    }
  }

  // Coherentist verdict
  check.verdict = check.coheres
    ? `"${proposition.content}" coheres with system "${system.name}"`
    : `"${proposition.content}" conflicts with propositions in "${system.name}"`;

  return check;
}

/**
 * Analyze liar paradox
 */
function analyzeLiarParadox() {
  const analysis = {
    statement: LIAR_PARADOX.statement,
    analysis: LIAR_PARADOX.analysis,
    responses: LIAR_PARADOX.responses,

    // CYNIC's analysis
    cynicAnalysis: {
      observation: 'Self-reference + negation = paradox',
      lesson: 'Truth may not be simply bivalent',
      phi_connection: 'Even truth has limits of certainty',
      confidence: PHI_INV_3
    },

    timestamp: Date.now()
  };

  state.paradoxAnalyses.push(analysis);
  saveState();

  return analysis;
}

/**
 * Compare truth theories
 */
function compareTheories(theory1Id, theory2Id) {
  const theory1 = TRUTH_THEORIES[theory1Id];
  const theory2 = TRUTH_THEORIES[theory2Id];

  if (!theory1 || !theory2) {
    return { error: 'One or both theories not found' };
  }

  const comparison = {
    theory1: {
      id: theory1Id,
      name: theory1.name,
      slogan: theory1.slogan
    },
    theory2: {
      id: theory2Id,
      name: theory2.name,
      slogan: theory2.slogan
    },

    differences: [],
    similarities: [],

    // Verdict
    pragmaticChoice: null,

    timestamp: Date.now()
  };

  // Compare aspects
  comparison.differences.push({
    aspect: 'Definition',
    theory1: theory1.definition,
    theory2: theory2.definition
  });

  // Strengths/weaknesses comparison
  comparison.differences.push({
    aspect: 'Key strength',
    theory1: theory1.strengths?.[0] || 'Not specified',
    theory2: theory2.strengths?.[0] || 'Not specified'
  });

  comparison.differences.push({
    aspect: 'Key weakness',
    theory1: theory1.weaknesses?.[0] || 'Not specified',
    theory2: theory2.weaknesses?.[0] || 'Not specified'
  });

  // No theory is clearly superior
  comparison.pragmaticChoice = {
    note: 'Different theories suit different purposes',
    correspondence: 'Good for explaining truth-making',
    coherence: 'Good for holistic epistemology',
    pragmatic: 'Good for anti-metaphysical contexts',
    deflationary: 'Good for logical/semantic purposes'
  };

  return comparison;
}

/**
 * Format status for display
 */
function formatStatus() {
  const lines = [
    '═══════════════════════════════════════════════════════════',
    '✓ TRUTH ENGINE - "Truth is correspondence with facts"',
    '═══════════════════════════════════════════════════════════',
    '',
    '── THEORIES OF TRUTH ──────────────────────────────────────'
  ];

  for (const [key, theory] of Object.entries(TRUTH_THEORIES)) {
    lines.push(`   ${theory.name}: "${theory.slogan}"`);
  }

  lines.push('');
  lines.push('── TARSKI\'S T-SCHEMA ──────────────────────────────────────');
  lines.push(`   ${T_SCHEMA.pattern}`);
  lines.push('   "Snow is white" is true iff snow is white');

  lines.push('');
  lines.push('── TRUTH-BEARERS ──────────────────────────────────────────');
  for (const [key, bearer] of Object.entries(TRUTH_BEARERS)) {
    lines.push(`   ${bearer.name}: ${bearer.description}`);
  }

  lines.push('');
  lines.push('── STATISTICS ─────────────────────────────────────────────');
  lines.push(`   Propositions: ${state.propositions.size}`);
  lines.push(`   Evaluations: ${state.evaluations.size}`);
  lines.push(`   T-sentences: ${state.tSentences.length}`);
  lines.push(`   Coherence systems: ${state.coherenceSystems.size}`);

  lines.push('');
  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('*sniff* "To say of what is that it is, is true." (Aristotle)');
  lines.push('═══════════════════════════════════════════════════════════');

  return lines.join('\n');
}

/**
 * Get statistics
 */
function getStats() {
  const theoryCount = {};
  for (const evaluation of state.evaluations.values()) {
    theoryCount[evaluation.theory] = (theoryCount[evaluation.theory] || 0) + 1;
  }

  const truthValues = { true: 0, false: 0, undetermined: 0 };
  for (const prop of state.propositions.values()) {
    if (prop.truthValue === 'true') truthValues.true++;
    else if (prop.truthValue === 'false') truthValues.false++;
    else truthValues.undetermined++;
  }

  return {
    propositions: state.propositions.size,
    evaluations: state.evaluations.size,
    evaluationsByTheory: theoryCount,
    truthValues,
    tSentences: state.tSentences.length,
    coherenceSystems: state.coherenceSystems.size,
    paradoxAnalyses: state.paradoxAnalyses.length
  };
}

module.exports = {
  // Core
  init,
  formatStatus,
  getStats,

  // Propositions
  registerProposition,
  evaluateByTheory,
  TRUTH_BEARERS,

  // Theories
  compareTheories,
  TRUTH_THEORIES,

  // Tarski
  generateTSentence,
  T_SCHEMA,

  // Coherence
  createCoherenceSystem,
  checkCoherence,

  // Paradox
  analyzeLiarParadox,
  LIAR_PARADOX,

  // Constants
  PHI,
  PHI_INV
};
