/**
 * Philosophy Engine Catalog
 *
 * Complete mapping of 73 philosophical engines to domains and capabilities.
 * Used by the loader to register all engines.
 *
 * "The library of wisdom" - κυνικός
 *
 * @module @cynic/core/engines/philosophy/catalog
 */

'use strict';

import { EngineDomain } from '../engine.js';

/**
 * Engine catalog - maps legacy engines to new system
 *
 * Structure:
 * {
 *   id: unique identifier
 *   file: filename (without .cjs)
 *   domain: primary EngineDomain
 *   subdomains: additional domains
 *   capabilities: what this engine can do
 *   tradition: philosophical school/tradition
 *   description: brief description
 * }
 */
export const PHILOSOPHY_ENGINE_CATALOG = [
  // ═══════════════════════════════════════════════════════════════════════════
  // LOGIC & EPISTEMOLOGY
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'inference-engine',
    file: 'inference-engine',
    domain: EngineDomain.LOGIC,
    capabilities: ['deduction', 'induction', 'abduction', 'inference-rules'],
    description: 'Logical inference and reasoning patterns',
  },
  {
    id: 'evidence-engine',
    file: 'evidence-engine',
    domain: EngineDomain.EPISTEMOLOGY,
    capabilities: ['evidence-evaluation', 'confirmation', 'bayesian-reasoning'],
    description: 'Evidence evaluation and confirmation theory',
  },
  {
    id: 'truth-engine',
    file: 'truth-engine',
    domain: EngineDomain.EPISTEMOLOGY,
    capabilities: ['truth-theories', 'correspondence', 'coherence', 'pragmatic-truth'],
    description: 'Theories of truth and verification',
  },
  {
    id: 'epistemic-engine',
    file: 'epistemic-engine',
    domain: EngineDomain.EPISTEMOLOGY,
    capabilities: ['knowledge-analysis', 'justification', 'epistemic-virtue'],
    description: 'Knowledge, justification, and epistemic norms',
  },
  {
    id: 'counterfactual-engine',
    file: 'counterfactual-engine',
    domain: EngineDomain.LOGIC,
    subdomains: [EngineDomain.METAPHYSICS],
    capabilities: ['counterfactual-reasoning', 'possible-worlds', 'causation'],
    description: 'Counterfactual conditionals and modal reasoning',
  },
  {
    id: 'explanation-engine',
    file: 'explanation-engine',
    domain: EngineDomain.EPISTEMOLOGY,
    subdomains: [EngineDomain.SCIENCE],
    capabilities: ['explanation-models', 'understanding', 'why-questions'],
    description: 'Scientific and philosophical explanation',
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // METAPHYSICS
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'causation-metaphysics-engine',
    file: 'causation-metaphysics-engine',
    domain: EngineDomain.METAPHYSICS,
    capabilities: ['causation', 'causal-theories', 'determinism'],
    description: 'Theories of causation and causal reasoning',
  },
  {
    id: 'time-engine',
    file: 'time-engine',
    domain: EngineDomain.METAPHYSICS,
    capabilities: ['philosophy-of-time', 'temporal-logic', 'persistence'],
    description: 'Philosophy of time and temporal reasoning',
  },
  {
    id: 'identity-engine',
    file: 'identity-engine',
    domain: EngineDomain.METAPHYSICS,
    capabilities: ['personal-identity', 'numerical-identity', 'persistence'],
    description: 'Identity, persistence, and change',
  },
  {
    id: 'modality-engine',
    file: 'modality-engine',
    domain: EngineDomain.METAPHYSICS,
    subdomains: [EngineDomain.LOGIC],
    capabilities: ['modal-logic', 'necessity', 'possibility', 'possible-worlds'],
    description: 'Modal metaphysics and possible worlds',
  },
  {
    id: 'existence-engine',
    file: 'existence-engine',
    domain: EngineDomain.METAPHYSICS,
    capabilities: ['ontology', 'existence', 'being', 'nothingness'],
    description: 'Questions of existence and being',
  },
  {
    id: 'free-will-engine',
    file: 'free-will-engine',
    domain: EngineDomain.METAPHYSICS,
    subdomains: [EngineDomain.ETHICS],
    capabilities: ['free-will', 'determinism', 'compatibilism', 'moral-responsibility'],
    description: 'Free will, determinism, and moral responsibility',
  },
  {
    id: 'process-philosophy-engine',
    file: 'process-philosophy-engine',
    domain: EngineDomain.METAPHYSICS,
    capabilities: ['process-metaphysics', 'becoming', 'events', 'whitehead'],
    tradition: 'process',
    description: 'Process philosophy and Whitehead',
  },
  {
    id: 'duration-engine',
    file: 'duration-engine',
    domain: EngineDomain.METAPHYSICS,
    capabilities: ['duration', 'bergson', 'intuition', 'lived-time'],
    tradition: 'bergson',
    description: 'Bergsonian duration and intuition',
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // ETHICS & VALUE
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'justice-engine',
    file: 'justice-engine',
    domain: EngineDomain.ETHICS,
    subdomains: [EngineDomain.POLITICS],
    capabilities: ['distributive-justice', 'rawls', 'nozick', 'fairness'],
    description: 'Theories of justice (Rawls, Nozick)',
  },
  {
    id: 'rights-engine',
    file: 'rights-engine',
    domain: EngineDomain.ETHICS,
    subdomains: [EngineDomain.LAW],
    capabilities: ['rights-theory', 'natural-rights', 'human-rights'],
    description: 'Rights theory and human rights',
  },
  {
    id: 'practical-reason-engine',
    file: 'practical-reason-engine',
    domain: EngineDomain.ETHICS,
    capabilities: ['practical-reasoning', 'action-guidance', 'moral-judgment'],
    description: 'Practical reason and moral judgment',
  },
  {
    id: 'bioethics-engine',
    file: 'bioethics-engine',
    domain: EngineDomain.ETHICS,
    capabilities: ['bioethics', 'medical-ethics', 'research-ethics', 'life-ethics'],
    description: 'Bioethics and medical ethics',
  },
  {
    id: 'environmental-ethics-engine',
    file: 'environmental-ethics-engine',
    domain: EngineDomain.ETHICS,
    capabilities: ['environmental-ethics', 'animal-ethics', 'sustainability'],
    description: 'Environmental and ecological ethics',
  },
  {
    id: 'tech-ethics-engine',
    file: 'tech-ethics-engine',
    domain: EngineDomain.ETHICS,
    capabilities: ['tech-ethics', 'ai-ethics', 'digital-ethics', 'privacy'],
    description: 'Technology and AI ethics',
  },
  {
    id: 'evil-engine',
    file: 'evil-engine',
    domain: EngineDomain.ETHICS,
    subdomains: [EngineDomain.METAPHYSICS],
    capabilities: ['problem-of-evil', 'theodicy', 'moral-evil'],
    description: 'Problem of evil and theodicy',
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // MIND & COGNITION
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'consciousness-engine',
    file: 'consciousness-engine',
    domain: EngineDomain.MIND,
    capabilities: ['consciousness', 'qualia', 'hard-problem', 'phenomenal-experience'],
    description: 'Philosophy of consciousness',
  },
  {
    id: 'mental-state-engine',
    file: 'mental-state-engine',
    domain: EngineDomain.MIND,
    capabilities: ['mental-states', 'beliefs', 'desires', 'folk-psychology'],
    description: 'Mental states and folk psychology',
  },
  {
    id: 'intentionality-engine',
    file: 'intentionality-engine',
    domain: EngineDomain.MIND,
    capabilities: ['intentionality', 'aboutness', 'mental-content', 'representation'],
    description: 'Intentionality and mental content',
  },
  {
    id: 'embodied-cognition-engine',
    file: 'embodied-cognition-engine',
    domain: EngineDomain.MIND,
    capabilities: ['embodied-cognition', 'enactivism', 'situated-cognition'],
    description: 'Embodied and situated cognition',
  },
  {
    id: 'agency-engine',
    file: 'agency-engine',
    domain: EngineDomain.MIND,
    subdomains: [EngineDomain.ETHICS],
    capabilities: ['agency', 'action-theory', 'reasons', 'intentions'],
    description: 'Agency and action theory',
  },
  {
    id: 'action-theory-engine',
    file: 'action-theory-engine',
    domain: EngineDomain.MIND,
    capabilities: ['action-theory', 'intention', 'practical-reasoning'],
    description: 'Philosophy of action',
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // AESTHETICS
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'beauty-engine',
    file: 'beauty-engine',
    domain: EngineDomain.AESTHETICS,
    capabilities: ['beauty', 'aesthetic-value', 'sublime', 'aesthetic-experience'],
    description: 'Philosophy of beauty and aesthetic value',
  },
  {
    id: 'taste-engine',
    file: 'taste-engine',
    domain: EngineDomain.AESTHETICS,
    capabilities: ['aesthetic-judgment', 'taste', 'criticism', 'art-evaluation'],
    description: 'Aesthetic judgment and taste',
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // LANGUAGE & MEANING
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'meaning-engine',
    file: 'meaning-engine',
    domain: EngineDomain.LANGUAGE,
    capabilities: ['meaning', 'reference', 'semantics', 'theories-of-meaning'],
    description: 'Philosophy of meaning and reference',
  },
  {
    id: 'semantics-engine',
    file: 'semantics-engine',
    domain: EngineDomain.LANGUAGE,
    capabilities: ['semantics', 'truth-conditions', 'compositionality'],
    description: 'Formal and philosophical semantics',
  },
  {
    id: 'speech-act-engine',
    file: 'speech-act-engine',
    domain: EngineDomain.LANGUAGE,
    capabilities: ['speech-acts', 'pragmatics', 'illocution', 'performatives'],
    description: 'Speech act theory and pragmatics',
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // EASTERN PHILOSOPHY
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'buddhist-engine',
    file: 'buddhist-engine',
    domain: EngineDomain.EASTERN,
    capabilities: ['buddhism', 'four-noble-truths', 'emptiness', 'dependent-origination'],
    tradition: 'buddhist',
    description: 'Buddhist philosophy and practice',
  },
  {
    id: 'daoist-engine',
    file: 'daoist-engine',
    domain: EngineDomain.EASTERN,
    capabilities: ['daoism', 'wu-wei', 'yin-yang', 'naturalness'],
    tradition: 'daoist',
    description: 'Daoist philosophy and wisdom',
  },
  {
    id: 'vedanta-engine',
    file: 'vedanta-engine',
    domain: EngineDomain.EASTERN,
    capabilities: ['vedanta', 'brahman', 'atman', 'moksha', 'advaita'],
    tradition: 'vedanta',
    description: 'Vedantic philosophy',
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // REGIONAL PHILOSOPHY
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'african-philosophy-engine',
    file: 'african-philosophy-engine',
    domain: EngineDomain.AFRICAN,
    capabilities: ['ubuntu', 'communalism', 'oral-tradition', 'african-ethics'],
    description: 'African philosophical traditions',
  },
  {
    id: 'american-philosophy-engine',
    file: 'american-philosophy-engine',
    domain: EngineDomain.SOCIAL,
    capabilities: ['pragmatism', 'american-thought', 'transcendentalism'],
    tradition: 'american',
    description: 'American philosophical tradition',
  },
  {
    id: 'islamic-philosophy-engine',
    file: 'islamic-philosophy-engine',
    domain: EngineDomain.ISLAMIC,
    capabilities: ['kalam', 'falsafa', 'islamic-ethics', 'sufi-philosophy'],
    description: 'Islamic philosophical tradition',
  },
  {
    id: 'latin-american-philosophy-engine',
    file: 'latin-american-philosophy-engine',
    domain: EngineDomain.LATIN_AMERICAN,
    capabilities: ['liberation-philosophy', 'decolonial-thought', 'mestizaje'],
    description: 'Latin American philosophy',
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // PHILOSOPHY OF SCIENCE
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'scientific-method-engine',
    file: 'scientific-method-engine',
    domain: EngineDomain.SCIENCE,
    capabilities: ['scientific-method', 'hypothesis-testing', 'confirmation', 'falsification'],
    description: 'Philosophy of scientific method',
  },
  {
    id: 'theory-change-engine',
    file: 'theory-change-engine',
    domain: EngineDomain.SCIENCE,
    capabilities: ['theory-change', 'paradigm-shifts', 'kuhn', 'scientific-revolutions'],
    description: 'Scientific theory change and revolutions',
  },
  {
    id: 'method-engine',
    file: 'method-engine',
    domain: EngineDomain.SCIENCE,
    capabilities: ['methodology', 'research-design', 'scientific-reasoning'],
    description: 'Scientific methodology',
  },
  {
    id: 'progress-engine',
    file: 'progress-engine',
    domain: EngineDomain.SCIENCE,
    capabilities: ['scientific-progress', 'cumulativism', 'realism-debate'],
    description: 'Scientific progress and realism',
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // PHILOSOPHY OF MATHEMATICS
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'math-foundations-engine',
    file: 'math-foundations-engine',
    domain: EngineDomain.MATHEMATICS,
    capabilities: ['foundations', 'logicism', 'formalism', 'intuitionism'],
    description: 'Foundations of mathematics',
  },
  {
    id: 'math-ontology-engine',
    file: 'math-ontology-engine',
    domain: EngineDomain.MATHEMATICS,
    subdomains: [EngineDomain.METAPHYSICS],
    capabilities: ['mathematical-objects', 'platonism', 'nominalism'],
    description: 'Ontology of mathematical objects',
  },
  {
    id: 'math-practice-engine',
    file: 'math-practice-engine',
    domain: EngineDomain.MATHEMATICS,
    capabilities: ['mathematical-practice', 'proof', 'mathematical-explanation'],
    description: 'Philosophy of mathematical practice',
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // LAW, ECONOMICS & SOCIETY
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'social-contract-engine',
    file: 'social-contract-engine',
    domain: EngineDomain.POLITICS,
    capabilities: ['social-contract', 'political-legitimacy', 'hobbes', 'locke', 'rousseau'],
    description: 'Social contract theory',
  },
  {
    id: 'philosophy-of-law-engine',
    file: 'philosophy-of-law-engine',
    domain: EngineDomain.LAW,
    capabilities: ['jurisprudence', 'legal-positivism', 'natural-law', 'legal-interpretation'],
    description: 'Philosophy of law and jurisprudence',
  },
  {
    id: 'law-economics-engine',
    file: 'law-economics-engine',
    domain: EngineDomain.LAW,
    subdomains: [EngineDomain.ECONOMICS],
    capabilities: ['law-and-economics', 'efficiency', 'rational-choice'],
    description: 'Law and economics',
  },
  {
    id: 'philosophy-of-economics-engine',
    file: 'philosophy-of-economics-engine',
    domain: EngineDomain.ECONOMICS,
    capabilities: ['economic-methodology', 'value-theory', 'rationality'],
    description: 'Philosophy of economics',
  },
  {
    id: 'critical-theory-engine',
    file: 'critical-theory-engine',
    domain: EngineDomain.SOCIAL,
    capabilities: ['critical-theory', 'frankfurt-school', 'ideology-critique'],
    tradition: 'critical-theory',
    description: 'Critical theory and Frankfurt School',
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // PERCEPTION & EXPERIENCE
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'philosophy-of-perception-engine',
    file: 'philosophy-of-perception-engine',
    domain: EngineDomain.MIND,
    capabilities: ['perception', 'sense-data', 'direct-realism', 'representationalism'],
    description: 'Philosophy of perception',
  },
  {
    id: 'philosophy-of-emotion-engine',
    file: 'philosophy-of-emotion-engine',
    domain: EngineDomain.MIND,
    capabilities: ['emotions', 'feeling-theories', 'cognitive-theories', 'affect'],
    description: 'Philosophy of emotion',
  },
  {
    id: 'phenomenology-engine',
    file: 'phenomenology-engine',
    domain: EngineDomain.MIND,
    subdomains: [EngineDomain.METAPHYSICS],
    capabilities: ['phenomenology', 'husserl', 'intentionality', 'lived-experience'],
    tradition: 'phenomenology',
    description: 'Phenomenological philosophy',
  },
  {
    id: 'existentialism-engine',
    file: 'existentialism-engine',
    domain: EngineDomain.METAPHYSICS,
    subdomains: [EngineDomain.ETHICS],
    capabilities: ['existentialism', 'authenticity', 'freedom', 'anxiety', 'absurd'],
    tradition: 'existentialist',
    description: 'Existentialist philosophy',
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // RELIGION & THEOLOGY
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'theism-engine',
    file: 'theism-engine',
    domain: EngineDomain.METAPHYSICS,
    capabilities: ['theism', 'arguments-for-god', 'divine-attributes', 'religious-epistemology'],
    description: 'Philosophy of religion and theism',
  },
  {
    id: 'faith-reason-engine',
    file: 'faith-reason-engine',
    domain: EngineDomain.EPISTEMOLOGY,
    subdomains: [EngineDomain.METAPHYSICS],
    capabilities: ['faith-and-reason', 'religious-belief', 'reformed-epistemology'],
    description: 'Faith and reason',
  },
  {
    id: 'apophatic-engine',
    file: 'apophatic-engine',
    domain: EngineDomain.METAPHYSICS,
    capabilities: ['apophatic-theology', 'negative-theology', 'mysticism', 'ineffability'],
    tradition: 'apophatic',
    description: 'Apophatic/negative theology',
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // SPECIAL & INTEGRATION
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'decision-theory-engine',
    file: 'decision-theory-engine',
    domain: EngineDomain.DECISION,
    capabilities: ['decision-theory', 'expected-utility', 'game-theory', 'rational-choice'],
    description: 'Decision theory and rational choice',
  },
  {
    id: 'pragmatism-engine',
    file: 'pragmatism-engine',
    domain: EngineDomain.EPISTEMOLOGY,
    subdomains: [EngineDomain.ETHICS],
    capabilities: ['pragmatism', 'truth-as-utility', 'inquiry', 'dewey', 'james'],
    tradition: 'pragmatist',
    description: 'Pragmatist philosophy',
  },
  {
    id: 'cross-domain-reasoning-engine',
    file: 'cross-domain-reasoning-engine',
    domain: EngineDomain.INTEGRATION,
    capabilities: ['interdisciplinary', 'synthesis', 'bridge-concepts'],
    description: 'Cross-domain philosophical reasoning',
  },
  {
    id: 'integration-engine',
    file: 'integration-engine',
    domain: EngineDomain.INTEGRATION,
    capabilities: ['synthesis', 'unification', 'meta-philosophy'],
    description: 'Philosophical integration and synthesis',
  },
  {
    id: 'phi-complete-engine',
    file: 'phi-complete-engine',
    domain: EngineDomain.INTEGRATION,
    capabilities: ['phi-reasoning', 'golden-ratio', 'cynic-axioms'],
    description: 'CYNIC φ-complete reasoning',
  },
  {
    id: 'philosophical-judgment-engine',
    file: 'philosophical-judgment-engine',
    domain: EngineDomain.INTEGRATION,
    capabilities: ['judgment', 'evaluation', 'philosophical-assessment'],
    description: 'Philosophical judgment and assessment',
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // SOCRATIC & CLASSICAL
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'elenchus-engine',
    file: 'elenchus-engine',
    domain: EngineDomain.LOGIC,
    capabilities: ['elenchus', 'socratic-method', 'refutation', 'dialectic'],
    tradition: 'socratic',
    description: 'Socratic elenchus and dialectic',
  },
  {
    id: 'ti-esti-engine',
    file: 'ti-esti-engine',
    domain: EngineDomain.METAPHYSICS,
    capabilities: ['essence', 'definition', 'what-is-x', 'socratic-questions'],
    tradition: 'socratic',
    description: 'Socratic "What is X?" questions',
  },
  {
    id: 'kairos-engine',
    file: 'kairos-engine',
    domain: EngineDomain.ETHICS,
    subdomains: [EngineDomain.LANGUAGE],
    capabilities: ['kairos', 'right-timing', 'opportune-moment', 'rhetoric'],
    tradition: 'classical',
    description: 'Kairos - the opportune moment',
  },
  {
    id: 'defacement-engine',
    file: 'defacement-engine',
    domain: EngineDomain.ETHICS,
    capabilities: ['cynicism', 'defacement', 'parrhesia', 'counter-cultural'],
    tradition: 'cynic',
    description: 'Cynic defacement and parrhesia',
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // PHYSICS & SPECIAL
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'relativity-engine',
    file: 'relativity-engine',
    domain: EngineDomain.PHYSICS,
    capabilities: ['relativity', 'spacetime', 'philosophy-of-physics'],
    description: 'Philosophy of relativity',
  },
  {
    id: 'entanglement-engine',
    file: 'entanglement-engine',
    domain: EngineDomain.PHYSICS,
    capabilities: ['quantum-entanglement', 'non-locality', 'quantum-philosophy'],
    description: 'Quantum entanglement philosophy',
  },
  {
    id: 'intervention-engine',
    file: 'intervention-engine',
    domain: EngineDomain.METAPHYSICS,
    subdomains: [EngineDomain.SCIENCE],
    capabilities: ['causal-intervention', 'manipulation', 'experimentation'],
    description: 'Causal intervention and manipulation',
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // DECISION & ROUTING (System engines)
  // ═══════════════════════════════════════════════════════════════════════════
  {
    id: 'decision-engine',
    file: 'decision-engine',
    domain: EngineDomain.DECISION,
    capabilities: ['decision-making', 'choice', 'deliberation', 'routing'],
    description: 'CYNIC decision coordination',
  },
  {
    id: 'routing-engine',
    file: 'routing-engine',
    domain: EngineDomain.DECISION,
    capabilities: ['routing', 'dispatch', 'engine-selection'],
    description: 'CYNIC routing and dispatch',
  },
];

/**
 * Get engines by domain
 */
export function getCatalogByDomain(domain) {
  return PHILOSOPHY_ENGINE_CATALOG.filter(e =>
    e.domain === domain || (e.subdomains && e.subdomains.includes(domain))
  );
}

/**
 * Get engines by capability
 */
export function getCatalogByCapability(capability) {
  return PHILOSOPHY_ENGINE_CATALOG.filter(e =>
    e.capabilities.includes(capability)
  );
}

/**
 * Get engines by tradition
 */
export function getCatalogByTradition(tradition) {
  return PHILOSOPHY_ENGINE_CATALOG.filter(e => e.tradition === tradition);
}

/**
 * Get catalog statistics
 */
export function getCatalogStats() {
  const domains = new Map();
  const capabilities = new Set();
  const traditions = new Set();

  for (const engine of PHILOSOPHY_ENGINE_CATALOG) {
    // Count domains
    domains.set(engine.domain, (domains.get(engine.domain) || 0) + 1);

    // Count capabilities
    engine.capabilities.forEach(c => capabilities.add(c));

    // Count traditions
    if (engine.tradition) traditions.add(engine.tradition);
  }

  return {
    totalEngines: PHILOSOPHY_ENGINE_CATALOG.length,
    domains: Object.fromEntries(domains),
    uniqueCapabilities: capabilities.size,
    uniqueTraditions: traditions.size,
    traditions: Array.from(traditions),
  };
}

export default PHILOSOPHY_ENGINE_CATALOG;
