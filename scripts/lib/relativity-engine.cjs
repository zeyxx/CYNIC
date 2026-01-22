/**
 * CYNIC Relativity Engine Module (Phase 11A)
 *
 * "Πάντα πρός τι - all things are relative" - κυνικός
 *
 * Implements relativistic truth concepts:
 * - No absolute frame of reference
 * - Truth depends on context/perspective
 * - Lorentz-like transformations between viewpoints
 * - User's reference frame matters
 *
 * Einstein showed physics has no privileged observer.
 * CYNIC shows code has no privileged perspective.
 *
 * @module cynic/lib/relativity-engine
 */

'use strict';

const fs = require('fs');
const path = require('path');
const os = require('os');

// Import φ constants
const phiMath = require('./phi-math.cjs');
const { PHI, PHI_INV, PHI_INV_2, PHI_INV_3 } = phiMath;

// =============================================================================
// CONSTANTS (φ-derived)
// =============================================================================

/** Speed of understanding (max rate of perspective shift) */
const C_UNDERSTANDING = PHI;

/** Time dilation factor for deep work */
const TIME_DILATION_FACTOR = PHI_INV;

/** Maximum simultaneous frames - φ × 3 ≈ 5 */
const MAX_REFERENCE_FRAMES = Math.round(PHI * 3);

/** Perspective shift probability - φ⁻² */
const PERSPECTIVE_SHIFT_PROBABILITY = PHI_INV_2;

// =============================================================================
// STORAGE
// =============================================================================

const RELATIVITY_DIR = path.join(os.homedir(), '.cynic', 'relativity');
const STATE_FILE = path.join(RELATIVITY_DIR, 'state.json');
const FRAMES_FILE = path.join(RELATIVITY_DIR, 'frames.json');

// =============================================================================
// REFERENCE FRAMES
// =============================================================================

/**
 * Predefined reference frames (perspectives)
 */
const REFERENCE_FRAMES = {
  // Technical perspectives
  developer: {
    id: 'developer',
    name: 'Développeur',
    priorities: ['maintainability', 'readability', 'testability'],
    biases: ['over-engineering', 'premature-abstraction'],
    timeScale: 'days',
  },

  architect: {
    id: 'architect',
    name: 'Architecte',
    priorities: ['scalability', 'modularity', 'patterns'],
    biases: ['big-design-upfront', 'complexity-love'],
    timeScale: 'months',
  },

  operator: {
    id: 'operator',
    name: 'Ops/SRE',
    priorities: ['reliability', 'observability', 'deployability'],
    biases: ['stability-over-features', 'risk-aversion'],
    timeScale: 'hours',
  },

  security: {
    id: 'security',
    name: 'Sécurité',
    priorities: ['vulnerability', 'access-control', 'audit'],
    biases: ['paranoia', 'friction-acceptance'],
    timeScale: 'always',
  },

  // Business perspectives
  product: {
    id: 'product',
    name: 'Product',
    priorities: ['user-value', 'time-to-market', 'features'],
    biases: ['feature-creep', 'deadline-pressure'],
    timeScale: 'sprints',
  },

  business: {
    id: 'business',
    name: 'Business',
    priorities: ['revenue', 'cost', 'growth'],
    biases: ['short-termism', 'metric-obsession'],
    timeScale: 'quarters',
  },

  // User perspectives
  endUser: {
    id: 'endUser',
    name: 'Utilisateur',
    priorities: ['usability', 'speed', 'reliability'],
    biases: ['feature-blindness', 'habit-dependence'],
    timeScale: 'seconds',
  },

  newUser: {
    id: 'newUser',
    name: 'Nouveau',
    priorities: ['simplicity', 'guidance', 'forgiveness'],
    biases: ['overwhelm', 'different-mental-model'],
    timeScale: 'minutes',
  },

  // Meta perspectives
  futureYou: {
    id: 'futureYou',
    name: 'Toi dans 6 mois',
    priorities: ['documentation', 'clarity', 'context'],
    biases: ['amnesia', 'context-loss'],
    timeScale: '6-months',
  },

  cynic: {
    id: 'cynic',
    name: 'CYNIC',
    priorities: ['truth', 'simplicity', 'skepticism'],
    biases: ['over-questioning', 'pessimism'],
    timeScale: 'eternal',
  },
};

// =============================================================================
// STATE
// =============================================================================

const relativityState = {
  // Current reference frame
  currentFrame: 'developer',

  // Active frames being considered
  activeFrames: ['developer'],

  // Frame history
  frameHistory: [],

  // Perspective transforms applied
  transforms: [],

  stats: {
    frameShifts: 0,
    transformsApplied: 0,
    perspectivesConsidered: 0,
    conflictsDetected: 0,
  },
};

// =============================================================================
// FILE OPERATIONS
// =============================================================================

function ensureDir() {
  if (!fs.existsSync(RELATIVITY_DIR)) {
    fs.mkdirSync(RELATIVITY_DIR, { recursive: true });
  }
}

function loadState() {
  ensureDir();
  if (!fs.existsSync(STATE_FILE)) {
    return null;
  }
  try {
    return JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
  } catch {
    return null;
  }
}

function saveState() {
  ensureDir();
  fs.writeFileSync(STATE_FILE, JSON.stringify({
    currentFrame: relativityState.currentFrame,
    activeFrames: relativityState.activeFrames,
    stats: relativityState.stats,
  }, null, 2));
}

// =============================================================================
// FRAME OPERATIONS
// =============================================================================

/**
 * Set the current reference frame
 *
 * @param {string} frameId - Frame ID
 * @returns {Object} Frame info
 */
function setFrame(frameId) {
  const frame = REFERENCE_FRAMES[frameId];
  if (!frame) {
    return { error: `Unknown frame: ${frameId}` };
  }

  const previousFrame = relativityState.currentFrame;
  relativityState.currentFrame = frameId;

  if (!relativityState.activeFrames.includes(frameId)) {
    relativityState.activeFrames.push(frameId);
    if (relativityState.activeFrames.length > MAX_REFERENCE_FRAMES) {
      relativityState.activeFrames.shift();
    }
  }

  relativityState.frameHistory.push({
    from: previousFrame,
    to: frameId,
    timestamp: Date.now(),
  });

  relativityState.stats.frameShifts++;
  saveState();

  return {
    frame,
    previousFrame,
    message: `*head tilt* Changement de perspective: ${REFERENCE_FRAMES[previousFrame]?.name || previousFrame} → ${frame.name}`,
  };
}

/**
 * Add a frame to active consideration
 *
 * @param {string} frameId - Frame ID
 * @returns {Object} Result
 */
function addActiveFrame(frameId) {
  const frame = REFERENCE_FRAMES[frameId];
  if (!frame) {
    return { error: `Unknown frame: ${frameId}` };
  }

  if (relativityState.activeFrames.includes(frameId)) {
    return { alreadyActive: true, frame };
  }

  relativityState.activeFrames.push(frameId);
  if (relativityState.activeFrames.length > MAX_REFERENCE_FRAMES) {
    const removed = relativityState.activeFrames.shift();
    return {
      added: frame,
      removed: REFERENCE_FRAMES[removed],
      message: `Added ${frame.name}, removed ${REFERENCE_FRAMES[removed]?.name} (max ${MAX_REFERENCE_FRAMES} frames)`,
    };
  }

  relativityState.stats.perspectivesConsidered++;
  saveState();

  return { added: frame };
}

/**
 * Get current frame info
 *
 * @returns {Object} Current frame
 */
function getCurrentFrame() {
  return REFERENCE_FRAMES[relativityState.currentFrame] || REFERENCE_FRAMES.developer;
}

/**
 * Get all active frames
 *
 * @returns {Object[]} Active frames
 */
function getActiveFrames() {
  return relativityState.activeFrames.map(id => REFERENCE_FRAMES[id]).filter(Boolean);
}

// =============================================================================
// PERSPECTIVE TRANSFORMATION
// =============================================================================

/**
 * Transform an assessment from one frame to another
 *
 * @param {Object} assessment - Original assessment
 * @param {string} fromFrame - Source frame
 * @param {string} toFrame - Target frame
 * @returns {Object} Transformed assessment
 */
function transformPerspective(assessment, fromFrame, toFrame) {
  const from = REFERENCE_FRAMES[fromFrame];
  const to = REFERENCE_FRAMES[toFrame];

  if (!from || !to) {
    return assessment;
  }

  // Calculate "velocity" between frames (how different they are)
  const priorityOverlap = from.priorities.filter(p =>
    to.priorities.includes(p)
  ).length;
  const velocity = 1 - (priorityOverlap / Math.max(from.priorities.length, to.priorities.length));

  // Lorentz-like factor: γ = 1/√(1 - v²/c²)
  const gamma = 1 / Math.sqrt(1 - (velocity * velocity) / (C_UNDERSTANDING * C_UNDERSTANDING));

  // Transform confidence (time dilation analog)
  const transformedConfidence = assessment.confidence
    ? assessment.confidence / gamma
    : PHI_INV / gamma;

  // Transform priorities
  const transformedPriorities = to.priorities.map(priority => {
    const wasOriginalPriority = from.priorities.includes(priority);
    return {
      priority,
      importance: wasOriginalPriority ? 'shared' : 'new',
      weight: wasOriginalPriority ? PHI_INV : PHI_INV_2,
    };
  });

  // Note perspective biases
  const biasWarnings = to.biases.map(bias => ({
    bias,
    warning: `Attention au biais "${bias}" dans la perspective ${to.name}`,
  }));

  relativityState.transforms.push({
    from: fromFrame,
    to: toFrame,
    velocity,
    gamma,
    timestamp: Date.now(),
  });

  relativityState.stats.transformsApplied++;
  saveState();

  return {
    ...assessment,
    originalFrame: fromFrame,
    transformedFrame: toFrame,
    confidence: transformedConfidence,
    priorities: transformedPriorities,
    biasWarnings,
    lorentzGamma: gamma,
    perspectiveVelocity: velocity,
  };
}

/**
 * Evaluate something from multiple perspectives
 *
 * @param {string} subject - Subject to evaluate
 * @param {Object} context - Context
 * @returns {Object} Multi-perspective evaluation
 */
function evaluateFromAllPerspectives(subject, context = {}) {
  const evaluations = [];
  const conflicts = [];

  for (const frameId of relativityState.activeFrames) {
    const frame = REFERENCE_FRAMES[frameId];
    if (!frame) continue;

    const evaluation = {
      frame: frameId,
      frameName: frame.name,
      priorities: frame.priorities,
      timeScale: frame.timeScale,
      assessment: generateFrameAssessment(subject, frame, context),
    };

    evaluations.push(evaluation);
  }

  // Detect conflicts between perspectives
  for (let i = 0; i < evaluations.length; i++) {
    for (let j = i + 1; j < evaluations.length; j++) {
      const conflict = detectConflict(evaluations[i], evaluations[j]);
      if (conflict) {
        conflicts.push(conflict);
        relativityState.stats.conflictsDetected++;
      }
    }
  }

  return {
    subject,
    evaluations,
    conflicts,
    consensus: conflicts.length === 0,
    recommendation: generateRecommendation(evaluations, conflicts),
  };
}

/**
 * Generate assessment from a specific frame
 *
 * @param {string} subject - Subject
 * @param {Object} frame - Reference frame
 * @param {Object} context - Context
 * @returns {Object} Assessment
 */
function generateFrameAssessment(subject, frame, context) {
  // Check if subject aligns with frame priorities
  const alignments = frame.priorities.map(priority => ({
    priority,
    aligned: checkAlignment(subject, priority, context),
  }));

  const alignmentScore = alignments.filter(a => a.aligned).length / alignments.length;

  // Check for frame biases
  const biasRisks = frame.biases.map(bias => ({
    bias,
    risk: checkBiasRisk(subject, bias, context),
  }));

  return {
    alignmentScore,
    alignments,
    biasRisks,
    verdict: alignmentScore >= PHI_INV ? 'FAVORABLE' :
             alignmentScore >= PHI_INV_2 ? 'NEUTRAL' : 'UNFAVORABLE',
    timeRelevance: frame.timeScale,
  };
}

/**
 * Check alignment with a priority
 */
function checkAlignment(subject, priority, context) {
  // Simplified - would be more sophisticated in practice
  const subjectLower = subject.toLowerCase();
  const priorityPatterns = {
    maintainability: /maintain|readable|clean|simple/i,
    scalability: /scale|performance|load|growth/i,
    security: /secur|auth|protect|safe/i,
    usability: /user|ux|easy|intuitive/i,
    reliability: /reliab|stable|robust|fault/i,
    testability: /test|coverage|verify/i,
  };

  const pattern = priorityPatterns[priority];
  return pattern ? pattern.test(subjectLower) : Math.random() > PHI_INV;
}

/**
 * Check risk of a bias
 */
function checkBiasRisk(subject, bias, context) {
  return Math.random() < PHI_INV_2 ? 'low' : Math.random() < PHI_INV ? 'medium' : 'high';
}

/**
 * Detect conflict between two evaluations
 */
function detectConflict(eval1, eval2) {
  // Conflict if verdicts are opposite
  if ((eval1.assessment.verdict === 'FAVORABLE' && eval2.assessment.verdict === 'UNFAVORABLE') ||
      (eval1.assessment.verdict === 'UNFAVORABLE' && eval2.assessment.verdict === 'FAVORABLE')) {
    return {
      frames: [eval1.frameName, eval2.frameName],
      type: 'verdict_conflict',
      description: `${eval1.frameName} et ${eval2.frameName} ont des avis opposés`,
    };
  }

  return null;
}

/**
 * Generate recommendation from multi-perspective evaluation
 */
function generateRecommendation(evaluations, conflicts) {
  if (conflicts.length === 0) {
    return {
      type: 'consensus',
      message: '*tail wag* Toutes les perspectives sont alignées.',
    };
  }

  if (conflicts.length >= evaluations.length - 1) {
    return {
      type: 'divergent',
      message: '*head tilt* Perspectives très divergentes. Dialogue nécessaire.',
      suggestion: 'Organise une discussion entre les parties prenantes.',
    };
  }

  return {
    type: 'partial_conflict',
    message: `*sniff* ${conflicts.length} conflit(s) détecté(s). Tradeoffs à considérer.`,
    conflicts: conflicts.map(c => c.description),
  };
}

// =============================================================================
// PERSPECTIVE SUGGESTIONS
// =============================================================================

/**
 * Suggest a perspective shift
 *
 * @param {string} currentContext - Current context
 * @returns {Object|null} Suggestion or null
 */
function suggestPerspectiveShift(currentContext) {
  if (Math.random() > PERSPECTIVE_SHIFT_PROBABILITY) {
    return null;
  }

  const currentFrame = getCurrentFrame();
  const suggestions = [];

  // Find frames not currently active
  for (const [id, frame] of Object.entries(REFERENCE_FRAMES)) {
    if (!relativityState.activeFrames.includes(id)) {
      // Check if this frame would add value
      const wouldAdd = frame.priorities.some(p =>
        !currentFrame.priorities.includes(p)
      );
      if (wouldAdd) {
        suggestions.push({
          frame: id,
          name: frame.name,
          newPriorities: frame.priorities.filter(p =>
            !currentFrame.priorities.includes(p)
          ),
        });
      }
    }
  }

  if (suggestions.length === 0) {
    return null;
  }

  const suggestion = suggestions[Math.floor(Math.random() * suggestions.length)];

  return {
    suggestion,
    message: `*head tilt* As-tu considéré la perspective "${suggestion.name}"? ` +
             `Elle apporte: ${suggestion.newPriorities.join(', ')}`,
  };
}

// =============================================================================
// PUBLIC API
// =============================================================================

/**
 * Initialize relativity engine
 */
function init() {
  ensureDir();
  const saved = loadState();
  if (saved) {
    relativityState.currentFrame = saved.currentFrame || 'developer';
    relativityState.activeFrames = saved.activeFrames || ['developer'];
    relativityState.stats = saved.stats || relativityState.stats;
  }
}

/**
 * Get all available frames
 *
 * @returns {Object} All frames
 */
function getAllFrames() {
  return { ...REFERENCE_FRAMES };
}

/**
 * Get statistics
 *
 * @returns {Object} Stats
 */
function getStats() {
  return {
    ...relativityState.stats,
    currentFrame: relativityState.currentFrame,
    activeFrames: relativityState.activeFrames,
    frameCount: Object.keys(REFERENCE_FRAMES).length,
  };
}

/**
 * Format status for display
 *
 * @returns {string} Formatted status
 */
function formatStatus() {
  const current = getCurrentFrame();
  const active = getActiveFrames();

  const lines = [
    '── RELATIVITY ENGINE ──────────────────────────────────────',
    `   Current frame: ${current.name}`,
    `   Priorities: ${current.priorities.join(', ')}`,
    `   Time scale: ${current.timeScale}`,
    '',
    `   Active frames (${active.length}/${MAX_REFERENCE_FRAMES}):`,
  ];

  for (const frame of active) {
    const marker = frame.id === relativityState.currentFrame ? '→' : ' ';
    lines.push(`   ${marker} ${frame.name}`);
  }

  lines.push('');
  lines.push(`   Shifts: ${relativityState.stats.frameShifts}`);
  lines.push(`   Conflicts: ${relativityState.stats.conflictsDetected}`);

  return lines.join('\n');
}

/**
 * Format multi-perspective evaluation
 *
 * @param {Object} evaluation - Evaluation result
 * @returns {string} Formatted display
 */
function formatEvaluation(evaluation) {
  const lines = [
    '── PERSPECTIVES ───────────────────────────────────────────',
    `   Subject: ${evaluation.subject}`,
    '',
  ];

  for (const ev of evaluation.evaluations) {
    const emoji = ev.assessment.verdict === 'FAVORABLE' ? '✅' :
                  ev.assessment.verdict === 'NEUTRAL' ? '➖' : '❌';
    lines.push(`   ${emoji} ${ev.frameName}: ${ev.assessment.verdict}`);
  }

  if (evaluation.conflicts.length > 0) {
    lines.push('');
    lines.push('   ⚠️ Conflits:');
    for (const conflict of evaluation.conflicts) {
      lines.push(`   • ${conflict.description}`);
    }
  }

  lines.push('');
  lines.push(`   ${evaluation.recommendation.message}`);

  return lines.join('\n');
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Constants
  C_UNDERSTANDING,
  MAX_REFERENCE_FRAMES,
  PERSPECTIVE_SHIFT_PROBABILITY,

  // Core functions
  init,
  getStats,
  getAllFrames,

  // Frame operations
  setFrame,
  addActiveFrame,
  getCurrentFrame,
  getActiveFrames,

  // Transformation
  transformPerspective,
  evaluateFromAllPerspectives,

  // Suggestions
  suggestPerspectiveShift,

  // Display
  formatStatus,
  formatEvaluation,
};
