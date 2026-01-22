/**
 * Semiotic Decoder - Sign, Object, Interpretant Relationships
 *
 * Philosophy: Charles Peirce's semiotics - meaning arises from
 * the triadic relationship between sign, object, and interpretant.
 *
 * Key concepts:
 * - Sign (Representamen): The form that represents
 * - Object: What the sign refers to
 * - Interpretant: The meaning/sense made of the sign
 * - Unlimited semiosis: Each interpretant becomes a new sign
 *
 * Sign types:
 * - Icon: Resembles its object (diagram, photo)
 * - Index: Causal/physical connection (smoke→fire)
 * - Symbol: Arbitrary convention (words, code)
 *
 * In CYNIC: Decode how code symbols, patterns, and conventions
 * relate to their meanings and track interpretation chains.
 *
 * @module semiotic-decoder
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
const SEMIOTIC_DIR = path.join(CYNIC_DIR, 'semiotic');
const STATE_FILE = path.join(SEMIOTIC_DIR, 'state.json');
const HISTORY_FILE = path.join(SEMIOTIC_DIR, 'history.jsonl');

// Constants
const MAX_SEMIOSIS_DEPTH = Math.round(PHI * 5);  // ~8 - interpretation chain depth
const MAX_SIGNS = Math.round(PHI * 50);          // ~81
const CLARITY_THRESHOLD = PHI_INV;               // 0.618

/**
 * Sign types (Peircean classification)
 */
const SIGN_TYPES = {
  icon: {
    name: 'Icon',
    description: 'Resembles its object through similarity',
    symbol: '🎨',
    examples: ['diagram', 'flowchart', 'visual pattern'],
    groundingStrength: PHI_INV,  // Moderate - depends on resemblance
  },
  index: {
    name: 'Index',
    description: 'Points to object through causal/physical connection',
    symbol: '👆',
    examples: ['error message', 'stack trace', 'log entry'],
    groundingStrength: PHI_INV + PHI_INV_3,  // Strong - causal link
  },
  symbol: {
    name: 'Symbol',
    description: 'Represents by arbitrary convention',
    symbol: '📜',
    examples: ['variable name', 'function name', 'keyword'],
    groundingStrength: PHI_INV_2,  // Weak - requires learning
  },
};

/**
 * Object categories
 */
const OBJECT_CATEGORIES = {
  immediate: {
    description: 'Object as represented in the sign',
    symbol: '◇',
  },
  dynamic: {
    description: 'Object as it really is, independent of sign',
    symbol: '◆',
  },
};

/**
 * Interpretant types
 */
const INTERPRETANT_TYPES = {
  immediate: {
    name: 'Immediate',
    description: 'First impression, surface meaning',
    depth: 1,
  },
  dynamic: {
    name: 'Dynamic',
    description: 'Actual effect on interpreter',
    depth: 2,
  },
  final: {
    name: 'Final',
    description: 'Ultimate meaning if inquiry completed',
    depth: 3,
  },
};

// In-memory state
let state = {
  signs: {},           // Registered signs
  triads: [],          // Sign-Object-Interpretant relationships
  semiosisChains: [],  // Unlimited semiosis chains
  conventions: {},     // Learned symbol conventions
  stats: {
    signsRegistered: 0,
    triadsFormed: 0,
    chainsTracked: 0,
    conventionsLearned: 0,
  },
};

/**
 * Initialize the semiotic decoder
 */
function init() {
  if (!fs.existsSync(SEMIOTIC_DIR)) {
    fs.mkdirSync(SEMIOTIC_DIR, { recursive: true });
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
 * Register a sign
 *
 * @param {string} representamen - The sign form (the signifier)
 * @param {string} signType - 'icon', 'index', or 'symbol'
 * @param {object} config - Additional configuration
 * @returns {object} Registered sign
 */
function registerSign(representamen, signType, config = {}) {
  if (!SIGN_TYPES[signType]) {
    return { error: `Unknown sign type: ${signType}` };
  }

  // Prune if needed
  if (Object.keys(state.signs).length >= MAX_SIGNS) {
    pruneOldSigns();
  }

  const id = `sign-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;

  const sign = {
    id,
    representamen,
    type: signType,
    typeInfo: SIGN_TYPES[signType],
    domain: config.domain || 'general',
    groundingStrength: SIGN_TYPES[signType].groundingStrength,
    interpretations: 0,
    createdAt: Date.now(),
    lastInterpreted: null,
  };

  state.signs[id] = sign;
  state.stats.signsRegistered++;

  logHistory({
    type: 'sign_registered',
    id,
    representamen,
    signType,
  });

  saveState();

  return sign;
}

/**
 * Prune oldest signs
 */
function pruneOldSigns() {
  const sorted = Object.entries(state.signs)
    .sort((a, b) => (a[1].lastInterpreted || a[1].createdAt) - (b[1].lastInterpreted || b[1].createdAt));

  const toRemove = sorted.slice(0, Math.round(MAX_SIGNS * PHI_INV_3));
  for (const [id] of toRemove) {
    delete state.signs[id];
  }
}

/**
 * Form a semiotic triad (sign → object → interpretant)
 *
 * @param {string} signId - Sign ID
 * @param {object} object - What the sign refers to
 * @param {object} interpretant - The meaning/interpretation
 * @returns {object} Triad relationship
 */
function formTriad(signId, object, interpretant) {
  const sign = state.signs[signId];
  if (!sign) return { error: 'Sign not found' };

  const triad = {
    id: `triad-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
    sign: {
      id: signId,
      representamen: sign.representamen,
      type: sign.type,
    },
    object: {
      referent: object.referent,
      category: object.category || 'immediate',
      description: object.description || '',
    },
    interpretant: {
      meaning: interpretant.meaning,
      type: interpretant.type || 'immediate',
      confidence: Math.min(interpretant.confidence || 0.5, PHI_INV),
    },
    clarity: calculateTriadClarity(sign, object, interpretant),
    formedAt: Date.now(),
  };

  state.triads.push(triad);
  state.stats.triadsFormed++;

  // Update sign
  sign.interpretations++;
  sign.lastInterpreted = Date.now();

  // Keep triads bounded
  if (state.triads.length > Math.round(PHI * 60)) {
    state.triads = state.triads.slice(-Math.round(PHI * 50));
  }

  logHistory({
    type: 'triad_formed',
    triadId: triad.id,
    signId,
    objectReferent: object.referent,
    meaning: interpretant.meaning,
  });

  saveState();

  return triad;
}

/**
 * Calculate clarity of triad relationship
 */
function calculateTriadClarity(sign, object, interpretant) {
  // Grounding strength from sign type
  const groundingScore = sign.groundingStrength;

  // Object specificity
  const objectScore = object.description ? 0.8 : 0.5;

  // Interpretant confidence
  const interpretantScore = interpretant.confidence || 0.5;

  // Combined clarity (φ-weighted)
  return groundingScore * PHI_INV_2 +
         objectScore * PHI_INV_3 +
         interpretantScore * PHI_INV_2;
}

/**
 * Track unlimited semiosis - interpretant becomes new sign
 *
 * @param {string} triadId - Starting triad ID
 * @param {array} chainLinks - Array of {object, interpretant} pairs
 * @returns {object} Semiosis chain
 */
function trackSemiosis(triadId, chainLinks = []) {
  const startTriad = state.triads.find(t => t.id === triadId);
  if (!startTriad) return { error: 'Starting triad not found' };

  const chain = {
    id: `chain-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
    startTriad: triadId,
    links: [{
      sign: startTriad.sign.representamen,
      object: startTriad.object.referent,
      interpretant: startTriad.interpretant.meaning,
    }],
    depth: 1,
    createdAt: Date.now(),
  };

  // Add chain links (each interpretant becomes new sign)
  for (const link of chainLinks.slice(0, MAX_SEMIOSIS_DEPTH - 1)) {
    const previousInterpretant = chain.links[chain.links.length - 1].interpretant;

    chain.links.push({
      sign: previousInterpretant,  // Previous interpretant is now sign
      object: link.object,
      interpretant: link.interpretant,
    });
    chain.depth++;
  }

  // Calculate chain coherence (how well meanings connect)
  chain.coherence = calculateChainCoherence(chain);

  state.semiosisChains.push(chain);
  state.stats.chainsTracked++;

  // Keep chains bounded
  if (state.semiosisChains.length > Math.round(PHI * 30)) {
    state.semiosisChains = state.semiosisChains.slice(-Math.round(PHI * 25));
  }

  logHistory({
    type: 'semiosis_tracked',
    chainId: chain.id,
    depth: chain.depth,
    coherence: chain.coherence,
  });

  saveState();

  return chain;
}

/**
 * Calculate coherence of semiosis chain
 */
function calculateChainCoherence(chain) {
  if (chain.links.length < 2) return 1;

  // Check if meanings connect logically
  // Simple heuristic: word overlap between consecutive links
  let connectionScore = 0;
  for (let i = 1; i < chain.links.length; i++) {
    const prevMeaning = chain.links[i - 1].interpretant.toLowerCase();
    const currSign = chain.links[i].sign.toLowerCase();

    // Some connection expected since interpretant became sign
    if (prevMeaning === currSign) {
      connectionScore += 1;
    } else {
      const prevWords = new Set(prevMeaning.split(/\s+/));
      const currWords = new Set(currSign.split(/\s+/));
      const overlap = [...prevWords].filter(w => currWords.has(w)).length;
      connectionScore += overlap > 0 ? 0.7 : 0.3;
    }
  }

  // Apply depth penalty (longer chains harder to maintain coherence)
  const depthPenalty = Math.pow(PHI_INV, chain.depth - 1);

  return (connectionScore / (chain.links.length - 1)) * depthPenalty;
}

/**
 * Learn a symbolic convention
 *
 * @param {string} symbol - The symbol (code convention)
 * @param {string} meaning - What it conventionally means
 * @param {string} domain - Domain of convention
 * @returns {object} Learned convention
 */
function learnConvention(symbol, meaning, domain = 'general') {
  const key = `${domain}:${symbol}`;

  if (!state.conventions[key]) {
    state.conventions[key] = {
      symbol,
      meanings: [],
      domain,
      learnedAt: Date.now(),
      uses: 0,
    };
    state.stats.conventionsLearned++;
  }

  // Add meaning if not already present
  if (!state.conventions[key].meanings.includes(meaning)) {
    state.conventions[key].meanings.push(meaning);
  }

  state.conventions[key].uses++;
  state.conventions[key].lastUsed = Date.now();

  saveState();

  return state.conventions[key];
}

/**
 * Decode a symbol using learned conventions
 *
 * @param {string} symbol - Symbol to decode
 * @param {string} domain - Domain context
 * @returns {object} Decoding result
 */
function decodeSymbol(symbol, domain = 'general') {
  // Check domain-specific first
  const domainKey = `${domain}:${symbol}`;
  const generalKey = `general:${symbol}`;

  const convention = state.conventions[domainKey] || state.conventions[generalKey];

  if (!convention) {
    return {
      symbol,
      decoded: false,
      suggestion: 'No convention learned. Register as new sign.',
    };
  }

  return {
    symbol,
    decoded: true,
    meanings: convention.meanings,
    primaryMeaning: convention.meanings[0],
    domain: convention.domain,
    confidence: Math.min(PHI_INV, convention.uses * PHI_INV_3 / 10),
    uses: convention.uses,
  };
}

/**
 * Find signs by type
 */
function findSignsByType(signType) {
  return Object.values(state.signs).filter(s => s.type === signType);
}

/**
 * Get statistics
 */
function getStats() {
  return {
    ...state.stats,
    activeSigns: Object.keys(state.signs).length,
    activeTriads: state.triads.length,
    activeChains: state.semiosisChains.length,
    conventions: Object.keys(state.conventions).length,
  };
}

/**
 * Format status for display
 */
function formatStatus() {
  const stats = getStats();

  let status = `📜 Semiotic Decoder\n`;
  status += `  Signs: ${stats.activeSigns}\n`;
  status += `  Triads: ${stats.activeTriads}\n`;
  status += `  Semiosis chains: ${stats.activeChains}\n`;
  status += `  Conventions: ${stats.conventions}\n`;

  // Sign type breakdown
  const byType = {};
  for (const sign of Object.values(state.signs)) {
    byType[sign.type] = (byType[sign.type] || 0) + 1;
  }

  if (Object.keys(byType).length > 0) {
    status += `  By type:\n`;
    for (const [type, count] of Object.entries(byType)) {
      const info = SIGN_TYPES[type];
      status += `    ${info.symbol} ${info.name}: ${count}\n`;
    }
  }

  return status;
}

module.exports = {
  init,
  registerSign,
  formTriad,
  trackSemiosis,
  learnConvention,
  decodeSymbol,
  findSignsByType,
  getStats,
  formatStatus,
  SIGN_TYPES,
  OBJECT_CATEGORIES,
  INTERPRETANT_TYPES,
};
