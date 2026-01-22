/**
 * Action Theory Engine - Davidson
 *
 * "Actions are events that are intentional under some description."
 * — Donald Davidson
 *
 * Complements agency-engine.cjs with Davidson's specific contributions:
 * - Actions as events under descriptions
 * - Reasons as causes
 * - The accordion effect
 * - Primary reasons (belief + pro-attitude)
 *
 * φ guides confidence in action attributions.
 */

const fs = require('fs');
const path = require('path');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;      // 61.8% - max confidence
const PHI_INV_2 = 0.381966011250105;    // 38.2%
const PHI_INV_3 = 0.236067977499790;    // 23.6%

// Storage
const STORAGE_DIR = path.join(require('os').homedir(), '.cynic', 'action-theory');

// Davidson's key theses
const DAVIDSON_THESES = {
  actions_are_events: {
    name: 'Actions Are Events',
    claim: 'Actions are a subclass of events',
    implication: 'Actions can be described in multiple ways',
    quote: 'Actions are events that are intentional under some description'
  },
  reasons_are_causes: {
    name: 'Reasons Are Causes',
    claim: 'The reasons for which an agent acts are causes of the action',
    structure: 'Primary reason = pro-attitude + belief',
    response_to: 'Logical connection argument (reasons seem conceptually tied to actions)',
    quote: 'A reason rationalizes an action only if it is also a cause'
  },
  anomalous_monism: {
    name: 'Anomalous Monism',
    claim: 'Mental events are physical events, but there are no strict psychophysical laws',
    implications: ['Token identity', 'No type-type reduction', 'Mental causation preserved']
  },
  principle_of_charity: {
    name: 'Principle of Charity',
    claim: 'Interpret others as mostly rational and correct',
    role: 'Constitutive of meaning and mental content'
  }
};

// Event identity criteria
const EVENT_IDENTITY = {
  coarse_grained: {
    name: 'Coarse-Grained (Davidson)',
    criterion: 'Events are identical if they have the same causes and effects',
    implication: 'Flipping switch = turning on light (same event, different descriptions)',
    proponent: 'Davidson'
  },
  fine_grained: {
    name: 'Fine-Grained (Goldman)',
    criterion: 'Events are identical only if they have the same properties',
    implication: 'Flipping switch ≠ turning on light (different act-types)',
    proponent: 'Goldman'
  }
};

// Accordion effect examples
const ACCORDION_EXAMPLES = {
  assassin: {
    descriptions: [
      'Moving finger',
      'Pulling trigger',
      'Firing gun',
      'Shooting victim',
      'Killing victim',
      'Assassinating archduke',
      'Starting WWI'
    ],
    intentional_under: ['Moving finger', 'Pulling trigger', 'Firing gun', 'Shooting victim', 'Killing victim'],
    not_intentional_under: ['Starting WWI'],
    davidson_point: 'Same action, intentional under some descriptions but not others'
  },
  light_switch: {
    descriptions: [
      'Moving finger',
      'Flipping switch',
      'Turning on light',
      'Illuminating room',
      'Alerting burglar'
    ],
    intentional_under: ['Moving finger', 'Flipping switch', 'Turning on light', 'Illuminating room'],
    not_intentional_under: ['Alerting burglar'],
    davidson_point: 'Action expands and contracts like an accordion'
  }
};

// State
const state = {
  events: new Map(),
  descriptions: new Map(),      // Event -> descriptions
  primaryReasons: new Map(),    // Event -> primary reason
  causalChains: [],
  analyses: []
};

/**
 * Initialize the action theory engine
 */
function init() {
  if (!fs.existsSync(STORAGE_DIR)) {
    fs.mkdirSync(STORAGE_DIR, { recursive: true });
  }

  const statePath = path.join(STORAGE_DIR, 'state.json');
  if (fs.existsSync(statePath)) {
    try {
      const saved = JSON.parse(fs.readFileSync(statePath, 'utf8'));
      if (saved.events) state.events = new Map(Object.entries(saved.events));
      if (saved.descriptions) state.descriptions = new Map(Object.entries(saved.descriptions));
      if (saved.primaryReasons) state.primaryReasons = new Map(Object.entries(saved.primaryReasons));
      if (saved.causalChains) state.causalChains = saved.causalChains;
      if (saved.analyses) state.analyses = saved.analyses;
    } catch {
      // Start fresh
    }
  }

  return { status: 'initialized', events: state.events.size };
}

/**
 * Save state
 */
function saveState() {
  const statePath = path.join(STORAGE_DIR, 'state.json');
  const toSave = {
    events: Object.fromEntries(state.events),
    descriptions: Object.fromEntries(state.descriptions),
    primaryReasons: Object.fromEntries(state.primaryReasons),
    causalChains: state.causalChains,
    analyses: state.analyses
  };
  fs.writeFileSync(statePath, JSON.stringify(toSave, null, 2));
}

/**
 * Register an event with multiple descriptions (Davidson)
 */
function registerEvent(id, spec = {}) {
  const event = {
    id,
    baseDescription: spec.description || id,
    agent: spec.agent || null,
    time: spec.time || Date.now(),

    // Multiple descriptions (Davidson's key insight)
    descriptions: spec.descriptions || [spec.description || id],
    intentionalUnder: spec.intentionalUnder || [],

    // Primary reason (if action)
    primaryReason: null,

    // Causal relations
    causes: spec.causes || [],
    effects: spec.effects || [],

    // Classification
    isAction: null,

    registeredAt: Date.now()
  };

  state.events.set(id, event);
  state.descriptions.set(id, event.descriptions);
  saveState();

  return event;
}

/**
 * Add description to event (accordion effect)
 */
function addDescription(eventId, description, spec = {}) {
  const event = state.events.get(eventId);
  if (!event) {
    return { error: 'Event not found' };
  }

  event.descriptions.push(description);

  if (spec.intentional) {
    event.intentionalUnder.push(description);
  }

  state.descriptions.set(eventId, event.descriptions);
  saveState();

  return {
    event: eventId,
    descriptions: event.descriptions,
    intentionalUnder: event.intentionalUnder,
    accordion: 'Same event, multiple descriptions (Davidson)'
  };
}

/**
 * Classify event as action (Davidson's criterion)
 */
function classifyAsAction(eventId) {
  const event = state.events.get(eventId);
  if (!event) {
    return { error: 'Event not found' };
  }

  const classification = {
    eventId,
    descriptions: event.descriptions,

    // Davidson's criterion
    davidsonCriterion: {
      hasAgent: Boolean(event.agent),
      intentionalUnderSomeDescription: event.intentionalUnder.length > 0
    },

    isAction: false,
    reasoning: null,

    timestamp: Date.now()
  };

  // Apply criterion
  if (classification.davidsonCriterion.hasAgent &&
      classification.davidsonCriterion.intentionalUnderSomeDescription) {
    classification.isAction = true;
    classification.reasoning = `Event is intentional under: "${event.intentionalUnder.join('", "')}"`;
  } else if (classification.davidsonCriterion.hasAgent) {
    classification.isAction = false;
    classification.reasoning = 'Agent involved but not intentional under any description';
  } else {
    classification.isAction = false;
    classification.reasoning = 'No agent involvement - mere event';
  }

  event.isAction = classification.isAction;
  saveState();

  return classification;
}

/**
 * Assign primary reason (Davidson's causal theory)
 */
function assignPrimaryReason(eventId, reason) {
  const event = state.events.get(eventId);
  if (!event) {
    return { error: 'Event not found' };
  }

  const primaryReason = {
    eventId,
    action: event.baseDescription,
    agent: event.agent,

    // Davidson's structure
    proAttitude: reason.proAttitude || reason.desire,  // What agent wanted
    belief: reason.belief,                              // What agent believed

    // The two roles
    rationalizes: true,   // Makes action intelligible
    causes: true,         // Causally produces action

    // Davidson's key insight
    davidsonInsight: 'The reason both rationalizes AND causes the action',

    confidence: Math.min(reason.confidence || PHI_INV_2, PHI_INV),

    timestamp: Date.now()
  };

  event.primaryReason = primaryReason;
  state.primaryReasons.set(eventId, primaryReason);
  saveState();

  return primaryReason;
}

/**
 * Analyze accordion effect for an event
 */
function analyzeAccordionEffect(eventId) {
  const event = state.events.get(eventId);
  if (!event) {
    return { error: 'Event not found' };
  }

  const analysis = {
    eventId,
    baseDescription: event.baseDescription,

    // All descriptions
    descriptions: event.descriptions,
    count: event.descriptions.length,

    // Intentionality map
    intentionalityMap: {},

    // Accordion insight
    accordion: {
      narrowest: event.descriptions[0],
      broadest: event.descriptions[event.descriptions.length - 1],
      expansion: `${event.descriptions[0]} → ${event.descriptions[event.descriptions.length - 1]}`,
      davidson: 'Actions expand like an accordion under redescription'
    },

    // Same event?
    eventIdentity: {
      coarse: 'All descriptions refer to SAME event (Davidson)',
      fine: 'Different act-types (Goldman would disagree)'
    },

    timestamp: Date.now()
  };

  // Build intentionality map
  for (const desc of event.descriptions) {
    analysis.intentionalityMap[desc] = event.intentionalUnder.includes(desc)
      ? 'intentional'
      : 'not intentional';
  }

  state.analyses.push(analysis);
  saveState();

  return analysis;
}

/**
 * Apply deviant causal chain test
 */
function testDeviantCausalChain(eventId, spec = {}) {
  const event = state.events.get(eventId);
  if (!event) {
    return { error: 'Event not found' };
  }

  const test = {
    eventId,
    action: event.baseDescription,

    // The problem of deviant causal chains
    scenario: spec.scenario || 'standard',
    causalChain: spec.chain || [event.primaryReason, event.baseDescription],

    // Is the chain deviant?
    deviant: spec.deviant || false,
    devianceType: spec.devianceType || null,

    // Davidson's problem
    davidsonProblem: {
      description: 'Reasons can cause actions in the wrong way',
      example: 'Nervousness about climbing causes trembling that causes fall (not intentional climbing)',
      challenge: 'How to specify "the right way" without circularity?'
    },

    // Verdict
    verdict: null,

    timestamp: Date.now()
  };

  if (test.deviant) {
    test.verdict = 'Not a genuine action - causal chain is deviant';
    test.devianceType = spec.devianceType || 'consequential';
  } else {
    test.verdict = 'Genuine action - causal chain is non-deviant';
  }

  state.causalChains.push(test);
  saveState();

  return test;
}

/**
 * Compare event identity theories
 */
function compareEventIdentity(descriptions) {
  const comparison = {
    descriptions,

    coarseGrained: {
      theory: EVENT_IDENTITY.coarse_grained.name,
      verdict: 'All descriptions refer to ONE event',
      criterion: EVENT_IDENTITY.coarse_grained.criterion,
      proponent: 'Davidson'
    },

    fineGrained: {
      theory: EVENT_IDENTITY.fine_grained.name,
      verdict: `${descriptions.length} distinct act-types`,
      criterion: EVENT_IDENTITY.fine_grained.criterion,
      proponent: 'Goldman'
    },

    significance: 'Affects how we count actions and attribute responsibility',

    timestamp: Date.now()
  };

  return comparison;
}

/**
 * Format status for display
 */
function formatStatus() {
  const lines = [
    '═══════════════════════════════════════════════════════════',
    '🎬 ACTION THEORY ENGINE - "Reasons are causes"',
    '═══════════════════════════════════════════════════════════',
    '',
    '── DAVIDSON\'S THESES ──────────────────────────────────────'
  ];

  for (const [key, thesis] of Object.entries(DAVIDSON_THESES)) {
    lines.push(`   ${thesis.name}: ${thesis.claim}`);
  }

  lines.push('');
  lines.push('── PRIMARY REASON STRUCTURE ───────────────────────────────');
  lines.push('   Pro-attitude (desire) + Belief = Primary Reason');
  lines.push('   Reason rationalizes AND causes the action');

  lines.push('');
  lines.push('── ACCORDION EFFECT ───────────────────────────────────────');
  lines.push('   Moving finger → Flipping switch → Turning on light');
  lines.push('   Same event, different descriptions');

  lines.push('');
  lines.push('── STATISTICS ─────────────────────────────────────────────');
  lines.push(`   Events: ${state.events.size}`);
  lines.push(`   Primary reasons: ${state.primaryReasons.size}`);
  lines.push(`   Analyses: ${state.analyses.length}`);

  lines.push('');
  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('*sniff* "A reason rationalizes only if it is also a cause."');
  lines.push('═══════════════════════════════════════════════════════════');

  return lines.join('\n');
}

/**
 * Get statistics
 */
function getStats() {
  let actions = 0;
  for (const event of state.events.values()) {
    if (event.isAction) actions++;
  }

  return {
    events: state.events.size,
    actions,
    primaryReasons: state.primaryReasons.size,
    causalChainTests: state.causalChains.length,
    analyses: state.analyses.length
  };
}

module.exports = {
  // Core
  init,
  formatStatus,
  getStats,

  // Events
  registerEvent,
  addDescription,
  classifyAsAction,

  // Reasons (Davidson)
  assignPrimaryReason,
  DAVIDSON_THESES,

  // Accordion
  analyzeAccordionEffect,
  ACCORDION_EXAMPLES,

  // Causal chains
  testDeviantCausalChain,

  // Event identity
  compareEventIdentity,
  EVENT_IDENTITY,

  // Constants
  PHI,
  PHI_INV
};
