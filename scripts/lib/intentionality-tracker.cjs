/**
 * Intentionality Tracker - Purpose and Directedness Detection
 *
 * Philosophy: Husserl's phenomenology - consciousness is always
 * consciousness OF something. Every mental act has directedness.
 *
 * Key concepts:
 * - Intentionality: Aboutness, directedness toward objects
 * - Noesis: The act of intending (perceiving, judging, wishing)
 * - Noema: The object as intended (not thing-in-itself)
 * - Horizon: Background expectations around any intention
 * - Fulfillment: When intentions meet their objects
 *
 * In CYNIC: Track what actions are directed toward, detect
 * implicit purposes, and measure intention-fulfillment alignment.
 *
 * @module intentionality-tracker
 */

const fs = require('fs');
const path = require('path');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.6180339887498949;
const PHI_INV_2 = 0.3819660112501051;
const PHI_INV_3 = 0.2360679774997897;

// Storage
const CYNIC_DIR = path.join(process.env.HOME || '/tmp', '.cynic');
const INTENT_DIR = path.join(CYNIC_DIR, 'intentionality');
const STATE_FILE = path.join(INTENT_DIR, 'state.json');
const HISTORY_FILE = path.join(INTENT_DIR, 'history.jsonl');

// Constants
const MAX_ACTIVE_INTENTIONS = Math.round(PHI * 25);  // ~40
const FULFILLMENT_THRESHOLD = PHI_INV;               // 0.618
const HORIZON_DECAY = PHI_INV;                       // How fast expectations fade

/**
 * Types of intentional acts (noesis types)
 */
const NOESIS_TYPES = {
  perceiving: {
    name: 'Perceiving',
    description: 'Taking in information, observing',
    symbol: '👁️',
    verb: 'observe',
  },
  judging: {
    name: 'Judging',
    description: 'Evaluating, assessing truth/value',
    symbol: '⚖️',
    verb: 'evaluate',
  },
  wishing: {
    name: 'Wishing',
    description: 'Desiring a future state',
    symbol: '✨',
    verb: 'desire',
  },
  willing: {
    name: 'Willing',
    description: 'Actively choosing, deciding',
    symbol: '💪',
    verb: 'choose',
  },
  remembering: {
    name: 'Remembering',
    description: 'Directing toward the past',
    symbol: '🔙',
    verb: 'recall',
  },
  anticipating: {
    name: 'Anticipating',
    description: 'Directing toward expected future',
    symbol: '🔜',
    verb: 'expect',
  },
  imagining: {
    name: 'Imagining',
    description: 'Directing toward possibilities',
    symbol: '💭',
    verb: 'envision',
  },
};

/**
 * Fulfillment states
 */
const FULFILLMENT_STATES = {
  empty: {
    description: 'Intention without object',
    range: [0, PHI_INV_3],
    symbol: '○',
  },
  partial: {
    description: 'Some aspects fulfilled',
    range: [PHI_INV_3, PHI_INV_2],
    symbol: '◔',
  },
  adequate: {
    description: 'Working fulfillment',
    range: [PHI_INV_2, PHI_INV],
    symbol: '◑',
  },
  fulfilled: {
    description: 'Intention meets object',
    range: [PHI_INV, 1],
    symbol: '●',
  },
};

// In-memory state
let state = {
  intentions: {},      // Active intentions
  horizons: {},        // Background expectations
  fulfillments: [],    // Fulfillment events
  purposePatterns: {}, // Detected purpose patterns
  stats: {
    intentionsTracked: 0,
    fulfillmentsRecorded: 0,
    horizonsUpdated: 0,
    purposesInferred: 0,
  },
};

/**
 * Initialize the intentionality tracker
 */
function init() {
  if (!fs.existsSync(INTENT_DIR)) {
    fs.mkdirSync(INTENT_DIR, { recursive: true });
  }

  if (fs.existsSync(STATE_FILE)) {
    try {
      state = JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
    } catch (e) {
      // Start fresh
    }
  }
}

/**
 * Save state to disk
 */
function saveState() {
  fs.writeFileSync(STATE_FILE, JSON.stringify(state, null, 2));
}

/**
 * Log to history
 */
function logHistory(event) {
  const entry = { timestamp: Date.now(), ...event };
  fs.appendFileSync(HISTORY_FILE, JSON.stringify(entry) + '\n');
}

/**
 * Register an intention (what is consciousness directed toward?)
 *
 * @param {string} noesisType - Type of intentional act
 * @param {object} noema - The intended object
 * @param {object} config - Additional configuration
 * @returns {object} Registered intention
 */
function registerIntention(noesisType, noema, config = {}) {
  if (!NOESIS_TYPES[noesisType]) {
    return { error: `Unknown noesis type: ${noesisType}` };
  }

  // Prune if needed
  if (Object.keys(state.intentions).length >= MAX_ACTIVE_INTENTIONS) {
    pruneOldIntentions();
  }

  const id = `int-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;

  const intention = {
    id,
    noesis: {
      type: noesisType,
      info: NOESIS_TYPES[noesisType],
    },
    noema: {
      content: noema.content,
      description: noema.description || '',
      domain: noema.domain || 'general',
    },
    horizon: {
      expectations: config.expectations || [],
      context: config.context || '',
      background: config.background || [],
    },
    fulfillment: {
      score: 0,
      state: 'empty',
      events: [],
    },
    purpose: config.purpose || null,  // Inferred or explicit purpose
    active: true,
    createdAt: Date.now(),
    lastUpdated: Date.now(),
  };

  state.intentions[id] = intention;
  state.stats.intentionsTracked++;

  // Update horizon
  updateHorizon(intention);

  logHistory({
    type: 'intention_registered',
    id,
    noesisType,
    noema: noema.content,
    purpose: intention.purpose,
  });

  saveState();

  return intention;
}

/**
 * Prune old intentions
 */
function pruneOldIntentions() {
  const sorted = Object.entries(state.intentions)
    .filter(([, i]) => i.active)
    .sort((a, b) => a[1].lastUpdated - b[1].lastUpdated);

  const toRemove = sorted.slice(0, Math.round(MAX_ACTIVE_INTENTIONS * PHI_INV_3));
  for (const [id] of toRemove) {
    state.intentions[id].active = false;
  }
}

/**
 * Update horizon (background expectations)
 */
function updateHorizon(intention) {
  const domain = intention.noema.domain;

  if (!state.horizons[domain]) {
    state.horizons[domain] = {
      expectations: [],
      lastUpdated: Date.now(),
    };
  }

  // Add new expectations to horizon
  for (const exp of intention.horizon.expectations) {
    if (!state.horizons[domain].expectations.includes(exp)) {
      state.horizons[domain].expectations.push(exp);
    }
  }

  // Apply decay - older expectations fade
  state.horizons[domain].lastUpdated = Date.now();
  state.stats.horizonsUpdated++;
}

/**
 * Record a fulfillment event
 *
 * @param {string} intentionId - Intention ID
 * @param {object} fulfillment - Fulfillment details
 * @returns {object} Updated intention
 */
function recordFulfillment(intentionId, fulfillment) {
  const intention = state.intentions[intentionId];
  if (!intention) return { error: 'Intention not found' };

  const event = {
    aspect: fulfillment.aspect || 'general',
    degree: Math.min(fulfillment.degree || 0.5, 1),
    description: fulfillment.description || '',
    timestamp: Date.now(),
  };

  intention.fulfillment.events.push(event);

  // Calculate overall fulfillment score
  const totalDegree = intention.fulfillment.events.reduce((sum, e) => sum + e.degree, 0);
  const avgDegree = totalDegree / intention.fulfillment.events.length;

  // Weight by recency and accumulation
  const accumulationBonus = Math.min(0.2, intention.fulfillment.events.length * PHI_INV_3 / 10);
  intention.fulfillment.score = Math.min(1, avgDegree + accumulationBonus);

  // Determine state
  intention.fulfillment.state = determineFulfillmentState(intention.fulfillment.score);
  intention.lastUpdated = Date.now();

  // Track fulfillment
  state.fulfillments.push({
    intentionId,
    event,
    newScore: intention.fulfillment.score,
    newState: intention.fulfillment.state,
  });

  state.stats.fulfillmentsRecorded++;

  // Keep fulfillments bounded
  if (state.fulfillments.length > Math.round(PHI * 50)) {
    state.fulfillments = state.fulfillments.slice(-Math.round(PHI * 40));
  }

  logHistory({
    type: 'fulfillment_recorded',
    intentionId,
    aspect: event.aspect,
    degree: event.degree,
    newScore: intention.fulfillment.score,
  });

  saveState();

  return {
    intention,
    event,
    fulfillmentScore: Math.round(intention.fulfillment.score * 100),
    fulfillmentState: intention.fulfillment.state,
    isFulfilled: intention.fulfillment.score >= FULFILLMENT_THRESHOLD,
  };
}

/**
 * Determine fulfillment state from score
 */
function determineFulfillmentState(score) {
  for (const [name, config] of Object.entries(FULFILLMENT_STATES)) {
    if (score >= config.range[0] && score < config.range[1]) {
      return name;
    }
  }
  return 'fulfilled';  // Score >= PHI_INV
}

/**
 * Infer purpose from action patterns
 *
 * @param {array} actions - Recent actions
 * @param {string} domain - Domain context
 * @returns {object} Inferred purpose
 */
function inferPurpose(actions, domain = 'general') {
  if (!actions || actions.length === 0) {
    return { error: 'No actions to analyze' };
  }

  // Look for patterns in action types
  const actionTypes = {};
  for (const action of actions) {
    const type = action.type || 'unknown';
    actionTypes[type] = (actionTypes[type] || 0) + 1;
  }

  // Find dominant action type
  const sorted = Object.entries(actionTypes).sort((a, b) => b[1] - a[1]);
  const dominant = sorted[0];

  // Map action patterns to purposes
  const purposeMap = {
    read: 'understanding',
    write: 'creation',
    edit: 'refinement',
    delete: 'simplification',
    search: 'discovery',
    test: 'verification',
    debug: 'correction',
    refactor: 'improvement',
  };

  const inferredPurpose = purposeMap[dominant[0]] || 'exploration';
  const confidence = Math.min(PHI_INV, dominant[1] / actions.length);

  // Record pattern
  const patternKey = `${domain}:${dominant[0]}`;
  if (!state.purposePatterns[patternKey]) {
    state.purposePatterns[patternKey] = {
      domain,
      actionType: dominant[0],
      purpose: inferredPurpose,
      occurrences: 0,
    };
  }
  state.purposePatterns[patternKey].occurrences++;

  state.stats.purposesInferred++;

  saveState();

  return {
    purpose: inferredPurpose,
    confidence: Math.round(confidence * 100),
    dominantAction: dominant[0],
    actionCount: dominant[1],
    totalActions: actions.length,
    verb: NOESIS_TYPES[purposeToNoesis(inferredPurpose)]?.verb || 'act',
  };
}

/**
 * Map purpose to noesis type
 */
function purposeToNoesis(purpose) {
  const map = {
    understanding: 'perceiving',
    creation: 'willing',
    refinement: 'judging',
    simplification: 'willing',
    discovery: 'perceiving',
    verification: 'judging',
    correction: 'willing',
    improvement: 'willing',
    exploration: 'imagining',
  };
  return map[purpose] || 'perceiving';
}

/**
 * Check if intention is fulfilled
 */
function isFulfilled(intentionId) {
  const intention = state.intentions[intentionId];
  if (!intention) return { error: 'Intention not found' };

  return {
    intentionId,
    isFulfilled: intention.fulfillment.score >= FULFILLMENT_THRESHOLD,
    score: Math.round(intention.fulfillment.score * 100),
    state: intention.fulfillment.state,
    symbol: FULFILLMENT_STATES[intention.fulfillment.state].symbol,
  };
}

/**
 * Get active intentions
 */
function getActiveIntentions() {
  return Object.values(state.intentions).filter(i => i.active);
}

/**
 * Get statistics
 */
function getStats() {
  const active = getActiveIntentions();
  const fulfilled = active.filter(i => i.fulfillment.score >= FULFILLMENT_THRESHOLD);

  return {
    ...state.stats,
    activeIntentions: active.length,
    fulfilledIntentions: fulfilled.length,
    fulfillmentRate: active.length > 0
      ? Math.round((fulfilled.length / active.length) * 100)
      : 0,
    horizons: Object.keys(state.horizons).length,
  };
}

/**
 * Format status for display
 */
function formatStatus() {
  const stats = getStats();
  const stateSymbol = s => FULFILLMENT_STATES[s]?.symbol || '?';

  let status = `💭 Intentionality Tracker\n`;
  status += `  Active intentions: ${stats.activeIntentions}\n`;
  status += `  Fulfilled: ${stats.fulfilledIntentions} (${stats.fulfillmentRate}%)\n`;
  status += `  Horizons: ${stats.horizons}\n`;
  status += `  Purposes inferred: ${stats.purposesInferred}\n`;

  // Show recent active intentions
  const active = getActiveIntentions().slice(0, 3);
  if (active.length > 0) {
    status += `  Recent:\n`;
    for (const i of active) {
      const noesisInfo = NOESIS_TYPES[i.noesis.type];
      status += `    ${noesisInfo.symbol} ${i.noema.content.slice(0, 25)} ${stateSymbol(i.fulfillment.state)}\n`;
    }
  }

  return status;
}

module.exports = {
  init,
  registerIntention,
  recordFulfillment,
  inferPurpose,
  isFulfilled,
  getActiveIntentions,
  getStats,
  formatStatus,
  NOESIS_TYPES,
  FULFILLMENT_STATES,
};
