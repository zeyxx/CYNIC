#!/usr/bin/env node
/**
 * CYNIC Auto-Judge - Autonomous Judgment System
 *
 * "Le chien juge" - CYNIC observes and judges without being asked
 *
 * Autonomous judgment triggers:
 * - Error patterns (repeated failures)
 * - Code quality signals (from writes/edits)
 * - Behavioral anomalies (unusual tool patterns)
 * - Security concerns (from guard observations)
 *
 * @module @cynic/auto-judge
 */

'use strict';

const fs = require('fs');
const path = require('path');
const os = require('os');

// =============================================================================
// CONSTANTS (φ-aligned)
// =============================================================================

const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;   // 61.8% max confidence
const PHI_INV_2 = 0.381966011250105; // 38.2%
const PHI_INV_3 = 0.236067977499790; // 23.6%

// Auto-judgment thresholds
const ERROR_THRESHOLD = 3;           // Judge after 3 similar errors
const SUCCESS_STREAK = 5;            // Judge positive after 5 successes
const ANOMALY_THRESHOLD = PHI_INV_2; // 38.2% deviation triggers judgment
const JUDGMENT_COOLDOWN_MS = 30000;  // 30s between auto-judgments

// Observation categories
const ObservationType = {
  ERROR: 'error',
  SUCCESS: 'success',
  CODE_CHANGE: 'code_change',
  SECURITY: 'security',
  PATTERN: 'pattern',
  ANOMALY: 'anomaly',
};

// State persistence
const STATE_DIR = path.join(os.homedir(), '.cynic');
const STATE_FILE = path.join(STATE_DIR, 'auto-judge.json');
const JUDGMENTS_FILE = path.join(STATE_DIR, 'judgments.jsonl');

// =============================================================================
// STATE MANAGEMENT
// =============================================================================

function ensureStateDir() {
  if (!fs.existsSync(STATE_DIR)) {
    fs.mkdirSync(STATE_DIR, { recursive: true });
  }
}

function loadState() {
  ensureStateDir();
  try {
    if (fs.existsSync(STATE_FILE)) {
      return JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
    }
  } catch (e) { /* ignore */ }

  return {
    observations: [],        // Recent observations
    judgments: [],           // Recent auto-judgments
    lastJudgment: null,      // Last judgment timestamp
    stats: {
      totalObservations: 0,
      totalJudgments: 0,
      byType: {},
    },
  };
}

function saveState(state) {
  ensureStateDir();
  try {
    // Keep only last 100 observations
    state.observations = state.observations.slice(-100);
    state.judgments = state.judgments.slice(-50);
    fs.writeFileSync(STATE_FILE, JSON.stringify(state, null, 2));
  } catch (e) { /* ignore */ }
}

function appendJudgment(judgment) {
  ensureStateDir();
  try {
    fs.appendFileSync(JUDGMENTS_FILE, JSON.stringify(judgment) + '\n');
  } catch (e) { /* ignore */ }
}

// =============================================================================
// OBSERVATION RECORDING
// =============================================================================

/**
 * Record an observation from tool usage
 */
function observe(observation) {
  const state = loadState();
  const now = Date.now();

  const obs = {
    ...observation,
    timestamp: now,
    id: `obs_${now}_${Math.random().toString(36).slice(2, 8)}`,
  };

  state.observations.push(obs);
  state.stats.totalObservations++;
  state.stats.byType[obs.type] = (state.stats.byType[obs.type] || 0) + 1;

  // Check if we should auto-judge
  const judgment = checkForAutoJudgment(state, obs);

  if (judgment) {
    state.judgments.push(judgment);
    state.lastJudgment = now;
    state.stats.totalJudgments++;
    appendJudgment(judgment);
  }

  saveState(state);
  return { observation: obs, judgment };
}

// =============================================================================
// AUTO-JUDGMENT LOGIC
// =============================================================================

/**
 * Check if observations warrant an automatic judgment
 */
function checkForAutoJudgment(state, currentObs) {
  const now = Date.now();

  // Cooldown check
  if (state.lastJudgment && (now - state.lastJudgment) < JUDGMENT_COOLDOWN_MS) {
    return null;
  }

  // Get recent observations (last 5 minutes)
  const recent = state.observations.filter(o => o.timestamp > now - 300000);

  // === TRIGGER 1: Error Pattern ===
  if (currentObs.type === ObservationType.ERROR) {
    const similarErrors = recent.filter(o =>
      o.type === ObservationType.ERROR &&
      o.signature === currentObs.signature
    );

    if (similarErrors.length >= ERROR_THRESHOLD) {
      return createJudgment({
        trigger: 'error_pattern',
        verdict: 'GROWL',
        subject: currentObs.signature,
        reason: `Erreur répétée ${similarErrors.length}x: ${currentObs.description}`,
        confidence: Math.min(PHI_INV, similarErrors.length * PHI_INV_3),
        observations: similarErrors.map(o => o.id),
        recommendation: 'Investiguer la cause racine. Pattern d\'erreur détecté.',
      });
    }
  }

  // === TRIGGER 2: Success Streak ===
  const recentSuccesses = recent.filter(o => o.type === ObservationType.SUCCESS);
  if (recentSuccesses.length >= SUCCESS_STREAK) {
    // Check if we haven't already judged this streak
    const lastSuccessJudgment = state.judgments.find(j =>
      j.trigger === 'success_streak' &&
      j.timestamp > now - 600000 // 10 min
    );

    if (!lastSuccessJudgment) {
      return createJudgment({
        trigger: 'success_streak',
        verdict: 'WAG',
        subject: 'session_progress',
        reason: `Série de ${recentSuccesses.length} opérations réussies`,
        confidence: PHI_INV_2,
        observations: recentSuccesses.slice(-SUCCESS_STREAK).map(o => o.id),
        recommendation: 'Bon momentum. Continuer sur cette lancée.',
      });
    }
  }

  // === TRIGGER 3: Security Concern ===
  if (currentObs.type === ObservationType.SECURITY) {
    return createJudgment({
      trigger: 'security_concern',
      verdict: currentObs.severity === 'critical' ? 'HOWL' : 'GROWL',
      subject: currentObs.signature,
      reason: currentObs.description,
      confidence: PHI_INV,
      observations: [currentObs.id],
      recommendation: currentObs.recommendation || 'Vérifier la sécurité.',
    });
  }

  // === TRIGGER 4: Code Quality Signal ===
  if (currentObs.type === ObservationType.CODE_CHANGE) {
    const recentChanges = recent.filter(o => o.type === ObservationType.CODE_CHANGE);

    // Too many rapid changes to same file
    const fileChanges = recentChanges.filter(o => o.file === currentObs.file);
    if (fileChanges.length >= 3) {
      return createJudgment({
        trigger: 'rapid_changes',
        verdict: 'BARK',
        subject: currentObs.file,
        reason: `${fileChanges.length} modifications rapides sur ${path.basename(currentObs.file)}`,
        confidence: PHI_INV_2,
        observations: fileChanges.map(o => o.id),
        recommendation: 'Beaucoup de changements. Prendre du recul et vérifier la cohérence?',
      });
    }
  }

  // === TRIGGER 5: Anomaly Detection ===
  if (currentObs.type === ObservationType.ANOMALY) {
    return createJudgment({
      trigger: 'anomaly_detected',
      verdict: 'BARK',
      subject: currentObs.signature,
      reason: currentObs.description,
      confidence: PHI_INV_2,
      observations: [currentObs.id],
      recommendation: 'Comportement inhabituel détecté. À surveiller.',
    });
  }

  return null;
}

/**
 * Create a judgment object
 */
function createJudgment({ trigger, verdict, subject, reason, confidence, observations, recommendation }) {
  const now = Date.now();

  return {
    id: `judge_${now}_${Math.random().toString(36).slice(2, 8)}`,
    timestamp: now,
    autonomous: true,
    trigger,
    verdict,
    subject,
    reason,
    confidence: Math.round(confidence * 1000) / 1000,
    maxConfidence: PHI_INV,
    observations,
    recommendation,
    // Q-Score simplified (based on verdict)
    qScore: verdict === 'WAG' ? 75 :
            verdict === 'BARK' ? 50 :
            verdict === 'GROWL' ? 25 :
            verdict === 'HOWL' ? 10 : 50,
  };
}

// =============================================================================
// OBSERVATION HELPERS (for integration with hooks)
// =============================================================================

/**
 * Create error observation
 */
function observeError(toolName, errorType, description) {
  return observe({
    type: ObservationType.ERROR,
    signature: `${toolName}:${errorType}`,
    tool: toolName,
    errorType,
    description,
  });
}

/**
 * Create success observation
 */
function observeSuccess(toolName, description) {
  return observe({
    type: ObservationType.SUCCESS,
    signature: toolName,
    tool: toolName,
    description,
  });
}

/**
 * Create code change observation
 */
function observeCodeChange(file, changeType, linesChanged) {
  return observe({
    type: ObservationType.CODE_CHANGE,
    signature: `change:${path.extname(file)}`,
    file,
    changeType,
    linesChanged,
    description: `${changeType} ${path.basename(file)} (${linesChanged} lines)`,
  });
}

/**
 * Create security observation
 */
function observeSecurity(signature, description, severity, recommendation) {
  return observe({
    type: ObservationType.SECURITY,
    signature,
    description,
    severity,
    recommendation,
  });
}

/**
 * Create anomaly observation
 */
function observeAnomaly(signature, description, context) {
  return observe({
    type: ObservationType.ANOMALY,
    signature,
    description,
    context,
  });
}

// =============================================================================
// QUERY FUNCTIONS
// =============================================================================

/**
 * Get recent judgments
 */
function getRecentJudgments(limit = 10) {
  const state = loadState();
  return state.judgments.slice(-limit);
}

/**
 * Get statistics
 */
function getStats() {
  const state = loadState();
  return {
    ...state.stats,
    recentObservations: state.observations.length,
    recentJudgments: state.judgments.length,
    lastJudgment: state.lastJudgment,
  };
}

/**
 * Format judgment for display
 */
function formatJudgment(judgment) {
  const emoji = {
    WAG: '🐕 *tail wag*',
    BARK: '🐕 *bark*',
    GROWL: '🐕 *growl*',
    HOWL: '🐕 *HOWL*',
  };

  return `
${emoji[judgment.verdict] || '🐕'} AUTO-JUDGMENT
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Sujet: ${judgment.subject}
Verdict: ${judgment.verdict} (Q-Score: ${judgment.qScore})
Confiance: ${Math.round(judgment.confidence * 100)}% (max ${Math.round(judgment.maxConfidence * 100)}%)

${judgment.reason}

💡 ${judgment.recommendation}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
`.trim();
}

/**
 * Reset state
 */
function reset() {
  ensureStateDir();
  saveState({
    observations: [],
    judgments: [],
    lastJudgment: null,
    stats: { totalObservations: 0, totalJudgments: 0, byType: {} },
  });
  return { success: true };
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Core functions
  observe,

  // Typed observers
  observeError,
  observeSuccess,
  observeCodeChange,
  observeSecurity,
  observeAnomaly,

  // Query functions
  getRecentJudgments,
  getStats,
  formatJudgment,
  reset,

  // Types
  ObservationType,

  // Constants
  PHI_INV,
  ERROR_THRESHOLD,
  SUCCESS_STREAK,
};
