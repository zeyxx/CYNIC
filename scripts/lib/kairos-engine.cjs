/**
 * CYNIC Kairos Engine (Phase 13C)
 *
 * "Καιρός - the opportune moment" - κυνικός
 *
 * Determines the right time for intervention:
 * - Kairos vs Chronos (quality vs quantity of time)
 * - Windows of opportunity
 * - User receptivity to input
 * - Momentum assessment
 * - Natural rhythm detection
 *
 * "The right word at the right time is worth a kingdom."
 *
 * @module cynic/lib/kairos-engine
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

/** Optimal receptivity threshold - φ⁻¹ */
const RECEPTIVITY_THRESHOLD = PHI_INV;

/** Momentum decay rate per minute - φ⁻³ */
const MOMENTUM_DECAY = PHI_INV_3;

/** Maximum pending interventions - φ × 5 ≈ 8 */
const MAX_PENDING = Math.round(PHI * 5);

/** Window of opportunity duration (ms) - φ × 30000 ≈ 49s */
const WINDOW_DURATION = Math.round(PHI * 30000);

/** Cooldown between interventions (ms) - φ × 60000 ≈ 97s */
const INTERVENTION_COOLDOWN = Math.round(PHI * 60000);

// =============================================================================
// INTERVENTION TYPES
// =============================================================================

const INTERVENTION_TYPES = {
  suggestion: {
    name: 'Suggestion',
    urgency: 'low',
    requiredReceptivity: PHI_INV_2,
    canWait: true,
  },
  warning: {
    name: 'Warning',
    urgency: 'medium',
    requiredReceptivity: PHI_INV_3,
    canWait: true,
  },
  alert: {
    name: 'Alert',
    urgency: 'high',
    requiredReceptivity: 0.1,
    canWait: false,
  },
  insight: {
    name: 'Insight',
    urgency: 'low',
    requiredReceptivity: PHI_INV,
    canWait: true,
  },
  question: {
    name: 'Question',
    urgency: 'medium',
    requiredReceptivity: PHI_INV_2,
    canWait: true,
  },
  celebration: {
    name: 'Celebration',
    urgency: 'low',
    requiredReceptivity: PHI_INV_3,
    canWait: true,
  },
};

// =============================================================================
// KAIROS SIGNALS
// =============================================================================

/**
 * Signals that indicate kairos (opportune moment)
 */
const KAIROS_SIGNALS = {
  // Positive signals (increase receptivity)
  positive: {
    pauseAfterAction: { weight: 0.3, description: 'Natural pause after completing action' },
    questionAsked: { weight: 0.5, description: 'User explicitly asked something' },
    errorEncountered: { weight: 0.4, description: 'User hit an error (teachable moment)' },
    taskCompleted: { weight: 0.3, description: 'Task completion (reflection moment)' },
    contextSwitch: { weight: 0.2, description: 'Switching between tasks' },
    sessionStart: { weight: 0.4, description: 'Fresh session start' },
  },

  // Negative signals (decrease receptivity)
  negative: {
    rapidEditing: { weight: -0.4, description: 'User is in flow, rapid edits' },
    deepFocus: { weight: -0.5, description: 'Deep concentration detected' },
    frustration: { weight: -0.3, description: 'Frustration signals present' },
    timeConstraint: { weight: -0.4, description: 'User mentioned deadline/rush' },
    recentIntervention: { weight: -0.3, description: 'We just intervened' },
  },
};

// =============================================================================
// STORAGE
// =============================================================================

const KAIROS_DIR = path.join(os.homedir(), '.cynic', 'kairos');
const STATE_FILE = path.join(KAIROS_DIR, 'state.json');
const HISTORY_FILE = path.join(KAIROS_DIR, 'history.jsonl');

// =============================================================================
// STATE
// =============================================================================

const kairosState = {
  // Current receptivity score (0-1)
  receptivity: 0.5,

  // Current momentum (-1 to 1, negative = declining)
  momentum: 0,

  // Active signals
  activeSignals: new Set(),

  // Pending interventions (waiting for kairos)
  pending: [],

  // Recently delivered interventions
  delivered: [],

  // Last intervention timestamp
  lastIntervention: null,

  // Rhythm tracking (activity patterns)
  rhythm: {
    actionsPerMinute: [],
    pauseDurations: [],
    lastAction: null,
  },

  // Statistics
  stats: {
    kairosDetected: 0,
    interventionsDelivered: 0,
    interventionsDeferred: 0,
    windowsMissed: 0,
    averageWaitTime: 0,
  },
};

// =============================================================================
// FILE OPERATIONS
// =============================================================================

function ensureDir() {
  if (!fs.existsSync(KAIROS_DIR)) {
    fs.mkdirSync(KAIROS_DIR, { recursive: true });
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
    stats: kairosState.stats,
    lastIntervention: kairosState.lastIntervention,
    rhythm: kairosState.rhythm,
  }, null, 2));
}

function appendHistory(event) {
  ensureDir();
  const line = JSON.stringify({ ...event, timestamp: Date.now() }) + '\n';
  fs.appendFileSync(HISTORY_FILE, line);
}

// =============================================================================
// SIGNAL PROCESSING
// =============================================================================

/**
 * Register a signal (event that affects kairos)
 *
 * @param {string} signal - Signal name
 * @returns {Object} Updated state
 */
function registerSignal(signal) {
  // Check positive signals
  if (KAIROS_SIGNALS.positive[signal]) {
    kairosState.activeSignals.add(signal);
    kairosState.receptivity += KAIROS_SIGNALS.positive[signal].weight;
  }

  // Check negative signals
  if (KAIROS_SIGNALS.negative[signal]) {
    kairosState.activeSignals.add(signal);
    kairosState.receptivity += KAIROS_SIGNALS.negative[signal].weight;
  }

  // Clamp receptivity
  kairosState.receptivity = Math.max(0, Math.min(1, kairosState.receptivity));

  // Track rhythm
  trackRhythm(signal);

  // Check if kairos occurred
  const kairos = checkKairos();

  return {
    signal,
    receptivity: Math.round(kairosState.receptivity * 100),
    kairos,
  };
}

/**
 * Clear a signal
 *
 * @param {string} signal - Signal to clear
 */
function clearSignal(signal) {
  kairosState.activeSignals.delete(signal);

  // Reverse the weight
  if (KAIROS_SIGNALS.positive[signal]) {
    kairosState.receptivity -= KAIROS_SIGNALS.positive[signal].weight * PHI_INV;
  }
  if (KAIROS_SIGNALS.negative[signal]) {
    kairosState.receptivity -= KAIROS_SIGNALS.negative[signal].weight * PHI_INV;
  }

  kairosState.receptivity = Math.max(0, Math.min(1, kairosState.receptivity));
}

/**
 * Track activity rhythm
 */
function trackRhythm(signal) {
  const now = Date.now();

  if (kairosState.rhythm.lastAction) {
    const gap = now - kairosState.rhythm.lastAction;

    // Track pause durations
    if (gap > 5000) { // More than 5 seconds is a pause
      kairosState.rhythm.pauseDurations.push(gap);
      if (kairosState.rhythm.pauseDurations.length > 20) {
        kairosState.rhythm.pauseDurations.shift();
      }
    }
  }

  kairosState.rhythm.lastAction = now;
}

// =============================================================================
// KAIROS DETECTION
// =============================================================================

/**
 * Check if this is a kairos moment
 *
 * @returns {Object|null} Kairos info if detected
 */
function checkKairos() {
  // Check receptivity threshold
  if (kairosState.receptivity < RECEPTIVITY_THRESHOLD) {
    return null;
  }

  // Check cooldown
  if (kairosState.lastIntervention) {
    const timeSince = Date.now() - kairosState.lastIntervention;
    if (timeSince < INTERVENTION_COOLDOWN) {
      return null;
    }
  }

  // Calculate kairos quality
  const quality = calculateKairosQuality();

  if (quality > PHI_INV_2) {
    kairosState.stats.kairosDetected++;

    const kairos = {
      detected: true,
      quality: Math.round(quality * 100),
      receptivity: Math.round(kairosState.receptivity * 100),
      momentum: Math.round(kairosState.momentum * 100),
      windowEnds: Date.now() + WINDOW_DURATION,
      activeSignals: Array.from(kairosState.activeSignals),
    };

    appendHistory({
      type: 'kairos_detected',
      quality: kairos.quality,
      receptivity: kairos.receptivity,
    });

    // Process pending interventions
    processPending(kairos);

    return kairos;
  }

  return null;
}

/**
 * Calculate quality of current kairos moment
 */
function calculateKairosQuality() {
  let quality = kairosState.receptivity;

  // Momentum boost
  if (kairosState.momentum > 0) {
    quality += kairosState.momentum * PHI_INV_3;
  }

  // Signal diversity bonus
  const signalCount = kairosState.activeSignals.size;
  if (signalCount > 1) {
    quality += signalCount * 0.05;
  }

  // Rhythm alignment bonus
  const rhythmScore = calculateRhythmAlignment();
  quality += rhythmScore * PHI_INV_3;

  return Math.min(1, quality);
}

/**
 * Calculate how well current moment aligns with natural rhythm
 */
function calculateRhythmAlignment() {
  const pauses = kairosState.rhythm.pauseDurations;
  if (pauses.length < 3) return 0.5;

  // Check if we're at a natural pause point
  const avgPause = pauses.reduce((a, b) => a + b, 0) / pauses.length;
  const lastPause = pauses[pauses.length - 1] || 0;

  // Good alignment if current pause is close to average
  const deviation = Math.abs(lastPause - avgPause) / avgPause;
  return Math.max(0, 1 - deviation);
}

// =============================================================================
// INTERVENTION MANAGEMENT
// =============================================================================

/**
 * Schedule an intervention for delivery at kairos
 *
 * @param {string} type - Intervention type
 * @param {Object} content - Intervention content
 * @returns {Object} Scheduled intervention
 */
function scheduleIntervention(type, content) {
  if (!INTERVENTION_TYPES[type]) {
    return { error: `Unknown intervention type: ${type}` };
  }

  const config = INTERVENTION_TYPES[type];

  // Check if this can wait
  if (!config.canWait) {
    // Immediate delivery
    return deliverNow(type, content);
  }

  // Check capacity
  if (kairosState.pending.length >= MAX_PENDING) {
    // Prune oldest non-urgent
    const oldestNonUrgent = kairosState.pending.findIndex(
      p => INTERVENTION_TYPES[p.type].urgency === 'low'
    );
    if (oldestNonUrgent >= 0) {
      kairosState.pending.splice(oldestNonUrgent, 1);
      kairosState.stats.windowsMissed++;
    } else {
      return { error: 'Intervention queue full' };
    }
  }

  const intervention = {
    id: `int-${Date.now()}-${Math.random().toString(36).slice(2, 4)}`,
    type,
    typeName: config.name,
    urgency: config.urgency,
    requiredReceptivity: config.requiredReceptivity,
    content,
    scheduledAt: Date.now(),
    expiresAt: Date.now() + WINDOW_DURATION * 10, // 10 windows max wait
    status: 'pending',
  };

  kairosState.pending.push(intervention);
  kairosState.stats.interventionsDeferred++;

  // Check if we can deliver now
  if (kairosState.receptivity >= config.requiredReceptivity) {
    return checkAndDeliver(intervention);
  }

  return {
    scheduled: true,
    intervention,
    message: `*ears perk* Intervention scheduled. Waiting for kairos...`,
  };
}

/**
 * Deliver intervention immediately
 */
function deliverNow(type, content) {
  const config = INTERVENTION_TYPES[type];

  const intervention = {
    id: `int-${Date.now()}-${Math.random().toString(36).slice(2, 4)}`,
    type,
    typeName: config.name,
    urgency: config.urgency,
    content,
    scheduledAt: Date.now(),
    deliveredAt: Date.now(),
    status: 'delivered',
    immediate: true,
  };

  kairosState.delivered.push(intervention);
  kairosState.lastIntervention = Date.now();
  kairosState.stats.interventionsDelivered++;

  // Apply cooldown signal
  registerSignal('recentIntervention');

  appendHistory({
    type: 'intervention_delivered',
    interventionId: intervention.id,
    interventionType: type,
    immediate: true,
  });

  saveState();

  return {
    delivered: true,
    intervention,
    message: content.message || content,
  };
}

/**
 * Check if intervention can be delivered and deliver it
 */
function checkAndDeliver(intervention) {
  const config = INTERVENTION_TYPES[intervention.type];

  if (kairosState.receptivity >= config.requiredReceptivity) {
    // Remove from pending
    const idx = kairosState.pending.findIndex(p => p.id === intervention.id);
    if (idx >= 0) {
      kairosState.pending.splice(idx, 1);
    }

    // Mark as delivered
    intervention.deliveredAt = Date.now();
    intervention.status = 'delivered';
    intervention.waitTime = intervention.deliveredAt - intervention.scheduledAt;

    kairosState.delivered.push(intervention);
    kairosState.lastIntervention = Date.now();
    kairosState.stats.interventionsDelivered++;

    // Update average wait time
    const totalWait = kairosState.stats.averageWaitTime *
      (kairosState.stats.interventionsDelivered - 1) + intervention.waitTime;
    kairosState.stats.averageWaitTime = totalWait / kairosState.stats.interventionsDelivered;

    // Apply cooldown signal
    registerSignal('recentIntervention');

    appendHistory({
      type: 'intervention_delivered',
      interventionId: intervention.id,
      interventionType: intervention.type,
      waitTime: intervention.waitTime,
    });

    saveState();

    return {
      delivered: true,
      intervention,
      waitTime: intervention.waitTime,
      message: intervention.content.message || intervention.content,
    };
  }

  return {
    delivered: false,
    intervention,
    reason: 'Receptivity too low',
    currentReceptivity: Math.round(kairosState.receptivity * 100),
    required: Math.round(config.requiredReceptivity * 100),
  };
}

/**
 * Process pending interventions on kairos
 */
function processPending(kairos) {
  const now = Date.now();
  const delivered = [];

  // Sort by urgency (high first)
  const urgencyOrder = { high: 0, medium: 1, low: 2 };
  const sorted = [...kairosState.pending].sort((a, b) =>
    urgencyOrder[a.urgency] - urgencyOrder[b.urgency]
  );

  for (const intervention of sorted) {
    // Check expiration
    if (now > intervention.expiresAt) {
      const idx = kairosState.pending.findIndex(p => p.id === intervention.id);
      if (idx >= 0) {
        kairosState.pending.splice(idx, 1);
        kairosState.stats.windowsMissed++;
      }
      continue;
    }

    // Try to deliver
    const result = checkAndDeliver(intervention);
    if (result.delivered) {
      delivered.push(result);
      break; // One intervention per kairos to avoid overload
    }
  }

  return delivered;
}

// =============================================================================
// MOMENTUM TRACKING
// =============================================================================

/**
 * Update momentum based on activity pattern
 *
 * @param {string} direction - 'up' or 'down'
 * @param {number} magnitude - Change magnitude
 */
function updateMomentum(direction, magnitude = 0.1) {
  if (direction === 'up') {
    kairosState.momentum = Math.min(1, kairosState.momentum + magnitude);
  } else {
    kairosState.momentum = Math.max(-1, kairosState.momentum - magnitude);
  }
}

/**
 * Apply momentum decay
 */
function applyMomentumDecay() {
  const now = Date.now();
  const lastAction = kairosState.rhythm.lastAction;

  if (lastAction) {
    const minutesSince = (now - lastAction) / (60 * 1000);
    const decay = MOMENTUM_DECAY * minutesSince;
    kairosState.momentum *= Math.exp(-decay);
  }
}

// =============================================================================
// PUBLIC API
// =============================================================================

/**
 * Initialize kairos engine
 */
function init() {
  ensureDir();
  const saved = loadState();
  if (saved) {
    kairosState.stats = saved.stats || kairosState.stats;
    kairosState.lastIntervention = saved.lastIntervention || null;
    kairosState.rhythm = saved.rhythm || kairosState.rhythm;
  }
}

/**
 * Check if now is kairos (opportune moment)
 *
 * @returns {Object} Kairos assessment
 */
function isKairos() {
  applyMomentumDecay();

  return {
    isKairos: kairosState.receptivity >= RECEPTIVITY_THRESHOLD,
    receptivity: Math.round(kairosState.receptivity * 100),
    momentum: Math.round(kairosState.momentum * 100),
    quality: Math.round(calculateKairosQuality() * 100),
    pendingCount: kairosState.pending.length,
    cooldownRemaining: kairosState.lastIntervention
      ? Math.max(0, INTERVENTION_COOLDOWN - (Date.now() - kairosState.lastIntervention))
      : 0,
  };
}

/**
 * Get pending interventions
 *
 * @returns {Object[]} Pending interventions
 */
function getPending() {
  return [...kairosState.pending];
}

/**
 * Get statistics
 *
 * @returns {Object} Stats
 */
function getStats() {
  return {
    ...kairosState.stats,
    currentReceptivity: Math.round(kairosState.receptivity * 100),
    currentMomentum: Math.round(kairosState.momentum * 100),
    pendingCount: kairosState.pending.length,
    activeSignals: kairosState.activeSignals.size,
  };
}

/**
 * Format status for display
 *
 * @returns {string} Formatted status
 */
function formatStatus() {
  const stats = getStats();
  const kairos = isKairos();

  // Receptivity bar
  const recBar = '█'.repeat(Math.round(kairos.receptivity / 10)) +
                 '░'.repeat(10 - Math.round(kairos.receptivity / 10));

  // Momentum indicator
  const momIndicator = kairos.momentum > 20 ? '↑' :
                       kairos.momentum < -20 ? '↓' : '→';

  const lines = [
    '── KAIROS ENGINE ──────────────────────────────────────────',
    `   Receptivity: [${recBar}] ${kairos.receptivity}%`,
    `   Momentum:    ${kairos.momentum}% ${momIndicator}`,
    `   Quality:     ${kairos.quality}%`,
    `   Is Kairos:   ${kairos.isKairos ? 'YES ✨' : 'no'}`,
  ];

  if (stats.pendingCount > 0) {
    lines.push('');
    lines.push(`   Pending: ${stats.pendingCount} interventions waiting`);
  }

  if (kairosState.activeSignals.size > 0) {
    lines.push('');
    lines.push('   Active signals:');
    for (const signal of Array.from(kairosState.activeSignals).slice(0, 3)) {
      const isPositive = KAIROS_SIGNALS.positive[signal];
      const marker = isPositive ? '+' : '-';
      lines.push(`   ${marker} ${signal}`);
    }
  }

  lines.push('');
  lines.push(`   Interventions delivered: ${stats.interventionsDelivered}`);
  lines.push(`   Windows missed: ${stats.windowsMissed}`);
  lines.push('');
  lines.push('   *sniff* "The right word at the right time."');

  return lines.join('\n');
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Constants
  RECEPTIVITY_THRESHOLD,
  WINDOW_DURATION,
  INTERVENTION_COOLDOWN,
  INTERVENTION_TYPES,
  KAIROS_SIGNALS,

  // Core functions
  init,
  registerSignal,
  clearSignal,

  // Kairos detection
  isKairos,
  checkKairos,

  // Intervention management
  scheduleIntervention,
  getPending,

  // Momentum
  updateMomentum,

  // Stats and display
  getStats,
  formatStatus,
};
