/**
 * CYNIC Signal Collector Module
 *
 * "Observer pour comprendre, comprendre pour aider" - κυνικός
 *
 * Collects behavioral signals from all sources and routes them
 * to the psychology module. Acts as the sensory system.
 *
 * Signal Sources:
 *   - Timing: Action intervals, session duration
 *   - Actions: Tool usage patterns, success/failure
 *   - Git: Commits, branches, conflict resolution
 *   - Semantic: Code patterns, complexity changes
 *   - CYNIC: Response patterns, intervention effects
 *
 * @module cynic/lib/signal-collector
 */

'use strict';

const fs = require('fs');
const path = require('path');
const os = require('os');

// Import φ constants
const phiMath = require('./phi-math.cjs');
const { PHI, PHI_INV, PHI_INV_2, PHI_INV_3 } = phiMath;

// Psychology module - lazy loaded to avoid circular dependency
let psychology = null;

// =============================================================================
// CONSTANTS (φ-derived)
// =============================================================================

/** Fast action threshold in ms - φ⁻² × 10000 = ~3.8s */
const FAST_ACTION_MS = PHI_INV_2 * 10000;

/** Slow action threshold in ms - φ × 60000 = ~97s */
const SLOW_ACTION_MS = PHI * 60000;

/** Context switch gap in ms - φ⁻¹ × 60000 = ~37s */
const CONTEXT_SWITCH_MS = PHI_INV * 60000;

/** Long session threshold in ms - φ × 60 × 60000 = ~97 min */
const LONG_SESSION_MS = PHI * 60 * 60000;

/** Break threshold in ms - φ⁻³ × 100 × 60000 = ~23.6 min */
const BREAK_THRESHOLD_MS = PHI_INV_3 * 100 * 60000;

/** Repeated failure count threshold */
const REPEATED_FAILURE_COUNT = 3;

/** Signal buffer max size before flush */
const BUFFER_MAX_SIZE = Math.round(PHI * 10); // ~16

// =============================================================================
// STORAGE
// =============================================================================

const COLLECTOR_DIR = path.join(os.homedir(), '.cynic', 'signals');
const BUFFER_FILE = path.join(COLLECTOR_DIR, 'buffer.jsonl');
const STATS_FILE = path.join(COLLECTOR_DIR, 'stats.json');

// =============================================================================
// STATE
// =============================================================================

const collectorState = {
  lastActionTime: null,
  lastActionType: null,
  sessionStart: null,
  failureStreak: 0,
  recentTools: [],
  recentFiles: [],
  recentIntervals: [], // For time entropy
  signalBuffer: [],
  stats: {
    totalSignals: 0,
    signalsByType: {},
  },
  entropy: {
    tool: 0,
    file: 0,
    time: 0,
    combined: 0,
  },
};

// =============================================================================
// FILE OPERATIONS
// =============================================================================

function ensureDir() {
  if (!fs.existsSync(COLLECTOR_DIR)) {
    fs.mkdirSync(COLLECTOR_DIR, { recursive: true });
  }
}

function loadStats() {
  ensureDir();
  if (!fs.existsSync(STATS_FILE)) {
    return { totalSignals: 0, signalsByType: {} };
  }
  try {
    return JSON.parse(fs.readFileSync(STATS_FILE, 'utf8'));
  } catch {
    return { totalSignals: 0, signalsByType: {} };
  }
}

function saveStats(stats) {
  ensureDir();
  fs.writeFileSync(STATS_FILE, JSON.stringify(stats, null, 2));
}

function appendToBuffer(signal) {
  ensureDir();
  const line = JSON.stringify({ ...signal, bufferedAt: Date.now() }) + '\n';
  fs.appendFileSync(BUFFER_FILE, line);
}

// =============================================================================
// SIGNAL GENERATION
// =============================================================================

/**
 * Generate timing-based signals from action interval
 * @param {number} interval - Time since last action in ms
 * @returns {Array} Generated signals
 */
function generateTimingSignals(interval) {
  const signals = [];

  if (interval < FAST_ACTION_MS) {
    signals.push({
      type: 'fast_actions',
      confidence: PHI_INV_2,
      data: { interval },
    });
  } else if (interval > SLOW_ACTION_MS) {
    signals.push({
      type: 'slow_actions',
      confidence: PHI_INV_3, // Lower confidence - could be thinking
      data: { interval },
    });
  }

  return signals;
}

/**
 * Generate session-based signals
 * @param {number} sessionDuration - Session duration in ms
 * @returns {Array} Generated signals
 */
function generateSessionSignals(sessionDuration) {
  const signals = [];

  if (sessionDuration > LONG_SESSION_MS) {
    signals.push({
      type: 'long_session',
      confidence: PHI_INV,
      data: { duration: sessionDuration },
    });
  }

  return signals;
}

/**
 * Generate context-switch signals from tool/file changes
 * @param {string} currentTool - Current tool name
 * @param {string} currentFile - Current file path
 * @returns {Array} Generated signals
 */
function generateContextSignals(currentTool, currentFile) {
  const signals = [];

  // Check tool switching pattern
  if (collectorState.recentTools.length >= 3) {
    const uniqueRecent = new Set(collectorState.recentTools.slice(-3));
    if (uniqueRecent.size === 3) {
      signals.push({
        type: 'context_switch',
        confidence: PHI_INV_2,
        data: { tools: [...uniqueRecent] },
      });
    }
  }

  // Check file switching pattern
  if (currentFile && collectorState.recentFiles.length >= 3) {
    const recentDirs = collectorState.recentFiles
      .slice(-3)
      .map(f => path.dirname(f));
    const uniqueDirs = new Set(recentDirs);
    if (uniqueDirs.size === 3) {
      signals.push({
        type: 'context_switch',
        confidence: PHI_INV_2,
        data: { directories: [...uniqueDirs] },
      });
    }
  }

  return signals;
}

// =============================================================================
// ENTROPY CALCULATION (Phase 6A - Physics Integration)
// =============================================================================

/**
 * Calculate Shannon entropy from a frequency distribution
 * H = -Σ p(x) * log₂(p(x))
 * @param {Object} frequencies - Map of item -> count
 * @returns {number} Entropy in bits
 */
function shannonEntropy(frequencies) {
  const total = Object.values(frequencies).reduce((a, b) => a + b, 0);
  if (total === 0) return 0;

  let entropy = 0;
  for (const count of Object.values(frequencies)) {
    if (count > 0) {
      const p = count / total;
      entropy -= p * Math.log2(p);
    }
  }
  return entropy;
}

/**
 * Calculate normalized entropy (0-1 scale)
 * @param {number} entropy - Raw entropy
 * @param {number} maxEntropy - Maximum possible entropy (log₂(n))
 * @returns {number} Normalized entropy 0-1
 */
function normalizeEntropy(entropy, maxEntropy) {
  if (maxEntropy === 0) return 0;
  return Math.min(1, entropy / maxEntropy);
}

/**
 * Calculate tool entropy - diversity of tools used
 * High entropy = exploration mode, low entropy = focused work
 * @returns {number} Normalized tool entropy 0-1
 */
function calculateToolEntropy() {
  const frequencies = {};
  for (const tool of collectorState.recentTools) {
    frequencies[tool] = (frequencies[tool] || 0) + 1;
  }

  const entropy = shannonEntropy(frequencies);
  const uniqueTools = Object.keys(frequencies).length;
  const maxEntropy = uniqueTools > 0 ? Math.log2(uniqueTools) : 0;

  return normalizeEntropy(entropy, maxEntropy);
}

/**
 * Calculate file entropy - diversity of files/directories accessed
 * High entropy = jumping around, low entropy = focused on few files
 * @returns {number} Normalized file entropy 0-1
 */
function calculateFileEntropy() {
  const frequencies = {};
  for (const file of collectorState.recentFiles) {
    // Use directory for broader grouping
    const dir = path.dirname(file);
    frequencies[dir] = (frequencies[dir] || 0) + 1;
  }

  const entropy = shannonEntropy(frequencies);
  const uniqueDirs = Object.keys(frequencies).length;
  const maxEntropy = uniqueDirs > 0 ? Math.log2(uniqueDirs) : 0;

  return normalizeEntropy(entropy, maxEntropy);
}

/**
 * Calculate time entropy - variability in action intervals
 * High entropy = erratic timing, low entropy = steady rhythm
 * Uses coefficient of variation as proxy for time "disorder"
 * @returns {number} Normalized time entropy 0-1
 */
function calculateTimeEntropy() {
  const intervals = collectorState.recentIntervals;
  if (intervals.length < 2) return 0;

  // Calculate mean
  const mean = intervals.reduce((a, b) => a + b, 0) / intervals.length;
  if (mean === 0) return 0;

  // Calculate standard deviation
  const variance = intervals.reduce((sum, x) => sum + Math.pow(x - mean, 2), 0) / intervals.length;
  const stdDev = Math.sqrt(variance);

  // Coefficient of variation (CV) as entropy proxy
  // CV = stdDev / mean, typically 0-2 range
  const cv = stdDev / mean;

  // Normalize to 0-1 using φ scaling
  // CV of 1 (stdDev = mean) maps to ~0.618
  return Math.min(1, cv * PHI_INV);
}

/**
 * Calculate combined session entropy
 * Weighted by φ ratios: tool (φ⁻¹), file (φ⁻²), time (φ⁻³)
 * @returns {Object} Entropy breakdown and combined score
 */
function calculateSessionEntropy() {
  const toolEntropy = calculateToolEntropy();
  const fileEntropy = calculateFileEntropy();
  const timeEntropy = calculateTimeEntropy();

  // φ-weighted combination
  // Tool diversity most important (immediate cognitive load)
  // File diversity second (navigation complexity)
  // Time variability third (rhythm disruption)
  const weights = {
    tool: PHI_INV,      // 0.618
    file: PHI_INV_2,    // 0.382
    time: PHI_INV_3,    // 0.236
  };

  const totalWeight = weights.tool + weights.file + weights.time;
  const combined = (
    toolEntropy * weights.tool +
    fileEntropy * weights.file +
    timeEntropy * weights.time
  ) / totalWeight;

  // Update state
  collectorState.entropy = {
    tool: toolEntropy,
    file: fileEntropy,
    time: timeEntropy,
    combined,
  };

  // Interpret the entropy level
  let interpretation;
  if (combined < PHI_INV_3) {
    interpretation = 'FOCUSED';      // < 23.6% - deep work mode
  } else if (combined < PHI_INV_2) {
    interpretation = 'PRODUCTIVE';   // 23.6-38.2% - balanced
  } else if (combined < PHI_INV) {
    interpretation = 'EXPLORING';    // 38.2-61.8% - healthy exploration
  } else {
    interpretation = 'CHAOTIC';      // > 61.8% - needs focus
  }

  return {
    tool: toolEntropy,
    file: fileEntropy,
    time: timeEntropy,
    combined,
    interpretation,
    // φ thresholds for display
    thresholds: {
      focused: PHI_INV_3,    // 0.236
      productive: PHI_INV_2, // 0.382
      exploring: PHI_INV,    // 0.618
    },
  };
}

/**
 * Get entropy emoji for display
 * @param {string} interpretation - Entropy interpretation
 * @returns {string} Emoji
 */
function getEntropyEmoji(interpretation) {
  const emojis = {
    FOCUSED: '🎯',
    PRODUCTIVE: '⚡',
    EXPLORING: '🔍',
    CHAOTIC: '🌀',
  };
  return emojis[interpretation] || '📊';
}

// =============================================================================
// SIGNAL ROUTING
// =============================================================================

/**
 * Route a signal to the psychology module
 * @param {Object} signal - Signal to route
 */
function routeSignal(signal) {
  // Update stats
  collectorState.stats.totalSignals++;
  collectorState.stats.signalsByType[signal.type] =
    (collectorState.stats.signalsByType[signal.type] || 0) + 1;

  // Lazy load psychology module if not yet loaded
  if (!psychology) {
    try {
      psychology = require('./human-psychology.cjs');
    } catch (e) {
      // Psychology not available
    }
  }

  // Try to send to psychology module
  if (psychology) {
    try {
      psychology.processSignal(signal);
      return true;
    } catch (e) {
      // Buffer the signal for later
      collectorState.signalBuffer.push(signal);
    }
  } else {
    // Buffer the signal
    collectorState.signalBuffer.push(signal);
  }

  // Flush buffer if too large
  if (collectorState.signalBuffer.length >= BUFFER_MAX_SIZE) {
    flushBuffer();
  }

  return false;
}

/**
 * Flush buffered signals to file
 */
function flushBuffer() {
  for (const signal of collectorState.signalBuffer) {
    appendToBuffer(signal);
  }
  collectorState.signalBuffer = [];
  saveStats(collectorState.stats);
}

// =============================================================================
// PUBLIC API - SIGNAL COLLECTION
// =============================================================================

/**
 * Initialize the signal collector
 */
function init() {
  ensureDir();
  collectorState.stats = loadStats();
  collectorState.sessionStart = Date.now();
  collectorState.lastActionTime = Date.now();

  // Try to load psychology module if not loaded
  if (!psychology) {
    try {
      psychology = require('./human-psychology.cjs');
      psychology.init();
    } catch (e) {
      // Still not available
    }
  }
}

/**
 * Collect a tool action signal
 * @param {string} toolName - Name of the tool used
 * @param {Object} toolInput - Tool input parameters
 * @param {boolean} success - Whether the action succeeded
 * @param {Object} result - Action result (optional)
 */
function collectToolAction(toolName, toolInput, success, _result = null) {
  const now = Date.now();
  const interval = collectorState.lastActionTime
    ? now - collectorState.lastActionTime
    : 0;

  // Track interval for time entropy (Phase 6A)
  if (interval > 0) {
    collectorState.recentIntervals.push(interval);
    if (collectorState.recentIntervals.length > 20) {
      collectorState.recentIntervals.shift();
    }
  }

  // Generate timing signals
  const timingSignals = generateTimingSignals(interval);
  for (const signal of timingSignals) {
    routeSignal(signal);
  }

  // Generate session signals
  const sessionDuration = now - collectorState.sessionStart;
  const sessionSignals = generateSessionSignals(sessionDuration);
  for (const signal of sessionSignals) {
    routeSignal(signal);
  }

  // Track tool usage
  collectorState.recentTools.push(toolName);
  if (collectorState.recentTools.length > 10) {
    collectorState.recentTools.shift();
  }

  // Track file if applicable
  const filePath = toolInput?.file_path || toolInput?.filePath;
  if (filePath) {
    collectorState.recentFiles.push(filePath);
    if (collectorState.recentFiles.length > 10) {
      collectorState.recentFiles.shift();
    }
  }

  // Generate context signals
  const contextSignals = generateContextSignals(toolName, filePath);
  for (const signal of contextSignals) {
    routeSignal(signal);
  }

  // Generate success/failure signals
  if (success) {
    collectorState.failureStreak = 0;
    routeSignal({
      type: 'action_success',
      confidence: PHI_INV_2,
      data: { tool: toolName },
    });
  } else {
    collectorState.failureStreak++;
    if (collectorState.failureStreak >= REPEATED_FAILURE_COUNT) {
      routeSignal({
        type: 'repeated_failure',
        confidence: PHI_INV,
        data: { tool: toolName, count: collectorState.failureStreak },
      });
    } else {
      routeSignal({
        type: 'action_failure',
        confidence: PHI_INV_2,
        data: { tool: toolName },
      });
    }
  }

  // Detect creative actions (new file creation, refactoring)
  if (toolName === 'Write' && !fs.existsSync(filePath)) {
    routeSignal({
      type: 'creative_action',
      confidence: PHI_INV_2,
      data: { action: 'new_file', path: filePath },
    });
  }

  // Update state
  collectorState.lastActionTime = now;
  collectorState.lastActionType = toolName;
}

/**
 * Collect a git action signal
 * @param {string} action - Git action (commit, push, pull, merge, etc.)
 * @param {Object} data - Action data
 */
function collectGitAction(action, data = {}) {
  const signalMap = {
    commit: { type: 'action_success', confidence: PHI_INV },
    push: { type: 'action_success', confidence: PHI_INV_2 },
    pull: { type: 'action_success', confidence: PHI_INV_2 },
    merge_success: { type: 'action_success', confidence: PHI_INV },
    merge_conflict: { type: 'action_failure', confidence: PHI_INV },
    revert: { type: 'action_failure', confidence: PHI_INV_3 }, // Could be intentional
  };

  const signalInfo = signalMap[action];
  if (signalInfo) {
    routeSignal({
      ...signalInfo,
      data: { gitAction: action, ...data },
    });
  }
}

/**
 * Collect a break signal (gap detected)
 * @param {number} gapMs - Gap duration in milliseconds
 */
function collectBreak(gapMs) {
  if (gapMs >= BREAK_THRESHOLD_MS) {
    routeSignal({
      type: 'break_taken',
      confidence: PHI_INV,
      data: { duration: gapMs },
    });
    // Reset session on long break
    collectorState.sessionStart = Date.now();
  }
}

/**
 * Collect a semantic signal (code pattern detected)
 * @param {string} pattern - Pattern type
 * @param {Object} data - Pattern data
 */
function collectSemanticSignal(pattern, data = {}) {
  const patternMap = {
    refactoring: { type: 'creative_action', confidence: PHI_INV_2 },
    new_abstraction: { type: 'creative_action', confidence: PHI_INV },
    complexity_increase: { type: 'context_switch', confidence: PHI_INV_3 },
    test_added: { type: 'action_success', confidence: PHI_INV_2 },
    documentation: { type: 'action_success', confidence: PHI_INV_3 },
  };

  const signalInfo = patternMap[pattern];
  if (signalInfo) {
    routeSignal({
      ...signalInfo,
      data: { semantic: pattern, ...data },
    });
  }
}

/**
 * Get collector statistics
 * @returns {Object} Collector stats
 */
function getStats() {
  return {
    ...collectorState.stats,
    sessionDuration: Date.now() - collectorState.sessionStart,
    recentTools: collectorState.recentTools,
    failureStreak: collectorState.failureStreak,
    bufferSize: collectorState.signalBuffer.length,
  };
}

/**
 * Shutdown - flush all buffers
 */
function shutdown() {
  flushBuffer();
  saveStats(collectorState.stats);
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Constants
  FAST_ACTION_MS,
  SLOW_ACTION_MS,
  CONTEXT_SWITCH_MS,
  LONG_SESSION_MS,
  BREAK_THRESHOLD_MS,

  // Core functions
  init,
  shutdown,
  getStats,

  // Signal collection
  collectToolAction,
  collectGitAction,
  collectBreak,
  collectSemanticSignal,

  // Entropy (Phase 6A)
  calculateSessionEntropy,
  getEntropyEmoji,
  shannonEntropy,

  // For testing
  routeSignal,
  flushBuffer,
  generateTimingSignals,
  generateContextSignals,
};
