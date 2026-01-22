/**
 * Art Ontology - Danto/Dickie
 *
 * "To see something as art requires something the eye cannot descry—
 *  an atmosphere of artistic theory, a knowledge of the history of art:
 *  an artworld." — Arthur Danto
 *
 * Implements:
 * - Institutional theory of art (Dickie)
 * - Artworld membership and status conferral
 * - Transfiguration of the commonplace (Danto)
 * - Interpretation and aboutness
 *
 * φ guides the thresholds for artistic status.
 */

const fs = require('fs');
const path = require('path');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;      // 61.8% - max confidence
const PHI_INV_2 = 0.381966011250105;    // 38.2%
const PHI_INV_3 = 0.236067977499790;    // 23.6%

// Storage
const STORAGE_DIR = path.join(require('os').homedir(), '.cynic', 'art-ontology');

// Artworld roles (Dickie's institutional theory)
const ARTWORLD_ROLES = {
  artist: {
    name: 'Artist',
    description: 'Creates artifacts for presentation',
    conferralPower: PHI_INV,
    requirements: ['intentionality', 'skill']
  },
  presenter: {
    name: 'Presenter',
    description: 'Museums, galleries, theaters',
    conferralPower: PHI_INV,
    requirements: ['institutional_affiliation']
  },
  critic: {
    name: 'Critic',
    description: 'Evaluates and interprets',
    conferralPower: PHI_INV_2,
    requirements: ['expertise', 'publication']
  },
  theorist: {
    name: 'Art Theorist',
    description: 'Develops frameworks of understanding',
    conferralPower: PHI_INV_2,
    requirements: ['theoretical_contribution']
  },
  historian: {
    name: 'Art Historian',
    description: 'Contextualizes within tradition',
    conferralPower: PHI_INV_3,
    requirements: ['historical_knowledge']
  },
  public: {
    name: 'Artworld Public',
    description: 'Informed audience',
    conferralPower: PHI_INV_3,
    requirements: ['engagement']
  }
};

// Danto's conditions for art
const DANTO_CONDITIONS = {
  aboutness: {
    name: 'Aboutness',
    description: 'The work is about something (has content)',
    weight: PHI_INV,
    required: true
  },
  embodiment: {
    name: 'Embodiment',
    description: 'Meaning is embodied in the work',
    weight: PHI_INV_2,
    required: true
  },
  rhetoric: {
    name: 'Rhetorical Ellipsis',
    description: 'Engages viewer to complete meaning',
    weight: PHI_INV_2,
    required: false
  },
  style: {
    name: 'Style',
    description: 'Manner of representation matters',
    weight: PHI_INV_3,
    required: false
  },
  history: {
    name: 'Art-Historical Context',
    description: 'Positioned within art history',
    weight: PHI_INV_3,
    required: false
  }
};

// Art categories
const ART_CATEGORIES = {
  fine_art: { name: 'Fine Art', status: 'paradigmatic', examples: ['painting', 'sculpture'] },
  decorative: { name: 'Decorative Art', status: 'contested', examples: ['furniture', 'ceramics'] },
  craft: { name: 'Craft', status: 'borderline', examples: ['pottery', 'weaving'] },
  readymade: { name: 'Readymade', status: 'transfigured', examples: ['Fountain', 'Brillo Boxes'] },
  conceptual: { name: 'Conceptual Art', status: 'paradigmatic', examples: ['instruction pieces'] },
  performance: { name: 'Performance Art', status: 'paradigmatic', examples: ['happenings', 'actions'] },
  digital: { name: 'Digital Art', status: 'emerging', examples: ['NFTs', 'generative'] },
  found: { name: 'Found Art', status: 'contested', examples: ['objet trouvé'] }
};

// State
const state = {
  artworks: new Map(),
  artworld: new Map(),        // members and their roles
  interpretations: new Map(),
  conferrals: [],             // history of status conferrals
  traditions: new Map()       // art historical traditions
};

/**
 * Initialize the art ontology engine
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
      if (saved.artworks) state.artworks = new Map(Object.entries(saved.artworks));
      if (saved.artworld) state.artworld = new Map(Object.entries(saved.artworld));
      if (saved.interpretations) state.interpretations = new Map(Object.entries(saved.interpretations));
      if (saved.conferrals) state.conferrals = saved.conferrals;
      if (saved.traditions) state.traditions = new Map(Object.entries(saved.traditions));
    } catch {
      // Start fresh
    }
  }

  return { status: 'initialized', artworks: state.artworks.size };
}

/**
 * Save state
 */
function saveState() {
  const statePath = path.join(STORAGE_DIR, 'state.json');
  const toSave = {
    artworks: Object.fromEntries(state.artworks),
    artworld: Object.fromEntries(state.artworld),
    interpretations: Object.fromEntries(state.interpretations),
    conferrals: state.conferrals,
    traditions: Object.fromEntries(state.traditions)
  };
  fs.writeFileSync(statePath, JSON.stringify(toSave, null, 2));
}

/**
 * Register an artworld member (Dickie's institutional theory)
 */
function registerMember(id, spec = {}) {
  const member = {
    id,
    name: spec.name || id,
    roles: spec.roles || ['public'],
    institution: spec.institution || null,
    expertise: spec.expertise || [],
    conferralHistory: [],
    registeredAt: Date.now()
  };

  // Calculate conferral power based on roles
  let totalPower = 0;
  for (const roleId of member.roles) {
    const role = ARTWORLD_ROLES[roleId];
    if (role) {
      totalPower += role.conferralPower;
    }
  }
  member.conferralPower = Math.min(totalPower, PHI_INV); // Cap at φ⁻¹

  state.artworld.set(id, member);
  saveState();

  return member;
}

/**
 * Register a candidate for art status
 */
function registerCandidate(id, spec = {}) {
  const candidate = {
    id,
    title: spec.title || id,
    creator: spec.creator || 'unknown',
    medium: spec.medium || 'mixed',
    category: spec.category || 'fine_art',
    year: spec.year || new Date().getFullYear(),

    // Danto's conditions
    aboutness: spec.aboutness || null,        // What it's about
    embodiment: spec.embodiment || null,      // How meaning is embodied
    rhetoric: spec.rhetoric || null,          // Viewer engagement
    style: spec.style || null,
    historicalContext: spec.historicalContext || null,

    // Status
    artStatus: 'candidate',
    conferrals: [],
    interpretations: [],

    // Metadata
    registeredAt: Date.now()
  };

  state.artworks.set(id, candidate);
  saveState();

  return candidate;
}

/**
 * Confer art status (Dickie's institutional act)
 */
function conferStatus(artworkId, memberId, reason) {
  const artwork = state.artworks.get(artworkId);
  const member = state.artworld.get(memberId);

  if (!artwork) {
    return { error: 'Artwork not found' };
  }

  if (!member) {
    return { error: 'Artworld member not found' };
  }

  const conferral = {
    artworkId,
    memberId,
    memberRoles: member.roles,
    conferralPower: member.conferralPower,
    reason,
    timestamp: Date.now()
  };

  artwork.conferrals.push(conferral);
  member.conferralHistory.push(conferral);
  state.conferrals.push(conferral);

  // Calculate cumulative conferral power
  const totalPower = artwork.conferrals.reduce((sum, c) => sum + c.conferralPower, 0);

  // Status thresholds based on φ
  if (totalPower >= PHI_INV) {
    artwork.artStatus = 'paradigmatic';
  } else if (totalPower >= PHI_INV_2) {
    artwork.artStatus = 'accepted';
  } else if (totalPower >= PHI_INV_3) {
    artwork.artStatus = 'contested';
  } else {
    artwork.artStatus = 'candidate';
  }

  saveState();

  return {
    artwork: artwork.title,
    conferredBy: member.name,
    power: conferral.conferralPower,
    totalPower,
    newStatus: artwork.artStatus
  };
}

/**
 * Evaluate Danto conditions for an artwork
 */
function evaluateDantoConditions(artworkId) {
  const artwork = state.artworks.get(artworkId);
  if (!artwork) {
    return { error: 'Artwork not found' };
  }

  const evaluation = {
    artwork: artwork.title,
    conditions: {},
    score: 0,
    maxScore: 0,
    requiredMet: true
  };

  for (const [key, condition] of Object.entries(DANTO_CONDITIONS)) {
    const value = artwork[key] || artwork.aboutness; // Map to artwork properties
    const met = Boolean(value);

    evaluation.conditions[key] = {
      name: condition.name,
      description: condition.description,
      met,
      required: condition.required,
      weight: condition.weight
    };

    evaluation.maxScore += condition.weight;
    if (met) {
      evaluation.score += condition.weight;
    }

    if (condition.required && !met) {
      evaluation.requiredMet = false;
    }
  }

  evaluation.percentage = evaluation.score / evaluation.maxScore;
  evaluation.meetsThreshold = evaluation.requiredMet && evaluation.percentage >= PHI_INV_2;

  // Danto's insight
  if (evaluation.requiredMet && artwork.aboutness) {
    evaluation.insight = `"${artwork.title}" has aboutness: ${artwork.aboutness}. It is transfigured from mere object to artwork.`;
  } else if (!artwork.aboutness) {
    evaluation.insight = 'Without aboutness, this remains a mere real thing, not art.';
  }

  return evaluation;
}

/**
 * Add interpretation (meaning requires interpretation)
 */
function addInterpretation(artworkId, interpreterId, interpretation) {
  const artwork = state.artworks.get(artworkId);
  if (!artwork) {
    return { error: 'Artwork not found' };
  }

  const interp = {
    id: `interp_${Date.now()}`,
    artworkId,
    interpreterId,
    content: interpretation.content,
    framework: interpretation.framework || 'contextual',
    references: interpretation.references || [],
    timestamp: Date.now()
  };

  artwork.interpretations.push(interp);

  // Track interpretations
  if (!state.interpretations.has(artworkId)) {
    state.interpretations.set(artworkId, []);
  }
  state.interpretations.get(artworkId).push(interp);

  saveState();

  // Danto: Interpretation is constitutive
  return {
    artwork: artwork.title,
    interpretation: interp,
    insight: 'Interpretation partially constitutes the artwork (Danto)'
  };
}

/**
 * Analyze transfiguration (Danto's key concept)
 * How does an ordinary object become art?
 */
function analyzeTransfiguration(artworkId) {
  const artwork = state.artworks.get(artworkId);
  if (!artwork) {
    return { error: 'Artwork not found' };
  }

  const analysis = {
    artwork: artwork.title,
    category: artwork.category,
    factors: []
  };

  // Check for readymade/found art transfiguration
  if (artwork.category === 'readymade' || artwork.category === 'found') {
    analysis.type = 'radical_transfiguration';
    analysis.factors.push({
      factor: 'Duchampian gesture',
      description: 'Selection and presentation as art',
      significance: 'high'
    });
  }

  // Aboutness is key to transfiguration
  if (artwork.aboutness) {
    analysis.factors.push({
      factor: 'Aboutness',
      description: `The work is about: ${artwork.aboutness}`,
      significance: 'essential'
    });
  }

  // Historical context
  if (artwork.historicalContext) {
    analysis.factors.push({
      factor: 'Art-historical positioning',
      description: artwork.historicalContext,
      significance: 'high'
    });
  }

  // Institutional conferral
  if (artwork.conferrals.length > 0) {
    analysis.factors.push({
      factor: 'Institutional recognition',
      description: `${artwork.conferrals.length} conferrals from artworld`,
      significance: 'medium'
    });
  }

  // Interpretations
  if (artwork.interpretations.length > 0) {
    analysis.factors.push({
      factor: 'Interpretive depth',
      description: `${artwork.interpretations.length} interpretations`,
      significance: 'medium'
    });
  }

  // Transfiguration verdict
  const hasAboutness = Boolean(artwork.aboutness);
  const hasConferral = artwork.conferrals.length > 0;

  if (hasAboutness && hasConferral) {
    analysis.verdict = 'transfigured';
    analysis.explanation = 'Object has been transfigured into artwork through aboutness and institutional recognition.';
  } else if (hasAboutness) {
    analysis.verdict = 'potentially_transfigured';
    analysis.explanation = 'Has aboutness but lacks institutional recognition.';
  } else if (hasConferral) {
    analysis.verdict = 'institutionally_accepted';
    analysis.explanation = 'Accepted by artworld but aboutness unclear.';
  } else {
    analysis.verdict = 'mere_object';
    analysis.explanation = 'Remains a mere real thing, not yet transfigured into art.';
  }

  return analysis;
}

/**
 * Compare indiscernibles (Danto's thought experiment)
 * Two visually identical objects where one is art and one isn't
 */
function compareIndiscernibles(artworkId, objectId) {
  const artwork = state.artworks.get(artworkId);

  const comparison = {
    artwork: artwork ? artwork.title : artworkId,
    object: objectId,
    visualDifference: false,
    differences: []
  };

  if (!artwork) {
    comparison.differences.push({
      dimension: 'artworld status',
      artwork: 'not registered',
      object: 'mere object'
    });
    comparison.insight = 'Neither is registered as art candidate.';
    return comparison;
  }

  // The key differences are non-visual
  comparison.differences.push({
    dimension: 'aboutness',
    artwork: artwork.aboutness || 'none',
    object: 'none (mere object)',
    significance: 'essential'
  });

  comparison.differences.push({
    dimension: 'art-historical context',
    artwork: artwork.historicalContext || 'present',
    object: 'absent',
    significance: 'high'
  });

  comparison.differences.push({
    dimension: 'institutional status',
    artwork: artwork.artStatus,
    object: 'not art',
    significance: 'high'
  });

  comparison.differences.push({
    dimension: 'interpretability',
    artwork: `${artwork.interpretations.length} interpretations`,
    object: 'not applicable',
    significance: 'medium'
  });

  // Danto's key insight
  comparison.insight = `Visually identical, yet one is art and one is not. ` +
    `The difference lies in the "is" of artistic identification—a matter of theory, not perception. ` +
    `"To see something as art requires an artworld."`;

  return comparison;
}

/**
 * Register an art tradition
 */
function registerTradition(id, spec = {}) {
  const tradition = {
    id,
    name: spec.name || id,
    period: spec.period || 'contemporary',
    characteristics: spec.characteristics || [],
    exemplars: spec.exemplars || [],
    influences: spec.influences || [],
    registeredAt: Date.now()
  };

  state.traditions.set(id, tradition);
  saveState();

  return tradition;
}

/**
 * Format status for display
 */
function formatStatus() {
  const lines = [
    '═══════════════════════════════════════════════════════════',
    '🎨 ART ONTOLOGY - "The transfiguration of the commonplace"',
    '═══════════════════════════════════════════════════════════',
    '',
    '── ARTWORLD ───────────────────────────────────────────────'
  ];

  // Show artworld members by role
  const roleCount = {};
  for (const member of state.artworld.values()) {
    for (const role of member.roles) {
      roleCount[role] = (roleCount[role] || 0) + 1;
    }
  }

  for (const [roleId, role] of Object.entries(ARTWORLD_ROLES)) {
    const count = roleCount[roleId] || 0;
    lines.push(`   ${role.name}: ${count} (power: ${(role.conferralPower * 100).toFixed(1)}%)`);
  }

  lines.push('');
  lines.push('── ARTWORKS BY STATUS ─────────────────────────────────────');

  const statusCount = { paradigmatic: 0, accepted: 0, contested: 0, candidate: 0 };
  for (const artwork of state.artworks.values()) {
    statusCount[artwork.artStatus] = (statusCount[artwork.artStatus] || 0) + 1;
  }

  lines.push(`   Paradigmatic: ${statusCount.paradigmatic}`);
  lines.push(`   Accepted:     ${statusCount.accepted}`);
  lines.push(`   Contested:    ${statusCount.contested}`);
  lines.push(`   Candidate:    ${statusCount.candidate}`);

  lines.push('');
  lines.push('── DANTO CONDITIONS ───────────────────────────────────────');
  for (const [key, condition] of Object.entries(DANTO_CONDITIONS)) {
    const req = condition.required ? '*' : ' ';
    lines.push(`   ${req}${condition.name}: ${condition.description}`);
  }

  lines.push('');
  lines.push('── STATISTICS ─────────────────────────────────────────────');
  lines.push(`   Total artworks: ${state.artworks.size}`);
  lines.push(`   Artworld members: ${state.artworld.size}`);
  lines.push(`   Conferrals: ${state.conferrals.length}`);
  lines.push(`   Traditions: ${state.traditions.size}`);

  lines.push('');
  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('*sniff* "To see something as art requires an artworld."');
  lines.push('═══════════════════════════════════════════════════════════');

  return lines.join('\n');
}

/**
 * Get statistics
 */
function getStats() {
  const statusDist = { paradigmatic: 0, accepted: 0, contested: 0, candidate: 0 };
  const categoryDist = {};

  for (const artwork of state.artworks.values()) {
    statusDist[artwork.artStatus] = (statusDist[artwork.artStatus] || 0) + 1;
    categoryDist[artwork.category] = (categoryDist[artwork.category] || 0) + 1;
  }

  return {
    totalArtworks: state.artworks.size,
    artworksByStatus: statusDist,
    artworksByCategory: categoryDist,
    artworldMembers: state.artworld.size,
    totalConferrals: state.conferrals.length,
    totalInterpretations: Array.from(state.interpretations.values())
      .reduce((sum, arr) => sum + arr.length, 0),
    traditions: state.traditions.size
  };
}

module.exports = {
  // Core
  init,
  formatStatus,
  getStats,

  // Artworld (Dickie)
  registerMember,
  conferStatus,
  ARTWORLD_ROLES,

  // Artworks
  registerCandidate,
  addInterpretation,

  // Danto
  evaluateDantoConditions,
  analyzeTransfiguration,
  compareIndiscernibles,
  DANTO_CONDITIONS,

  // Traditions
  registerTradition,
  ART_CATEGORIES,

  // Constants
  PHI,
  PHI_INV
};
