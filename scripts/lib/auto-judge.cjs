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
 * Now integrated with adaptive-learn.cjs for BURN-based continuous learning.
 * Thresholds adapt from observations instead of magic numbers.
 *
 * @module @cynic/auto-judge
 */

'use strict';

const fs = require('fs');
const path = require('path');
const os = require('os');

// Load adaptive learning system
const adaptiveLearnPath = path.join(__dirname, 'adaptive-learn.cjs');
let adaptiveLearn = null;
try {
  adaptiveLearn = require(adaptiveLearnPath);
} catch (e) {
  // Adaptive learning not available - will use static thresholds
}

// Load LLM judgment bridge (for autonomous improvement)
const llmBridgePath = path.join(__dirname, 'llm-judgment-bridge.cjs');
let llmBridge = null;
try {
  llmBridge = require(llmBridgePath);
} catch (e) {
  // LLM bridge not available - will use static judgment
}

// =============================================================================
// CONSTANTS (φ-aligned)
// =============================================================================

const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;   // 61.8% max confidence
const PHI_INV_2 = 0.381966011250105; // 38.2%
const PHI_INV_3 = 0.236067977499790; // 23.6%

// Static fallbacks (used when adaptive learning unavailable)
const STATIC_ERROR_THRESHOLD = 3;
const STATIC_SUCCESS_STREAK = 5;
const STATIC_CODE_CHANGE_THRESHOLD = 3;

// φ-derived thresholds (always static)
const ANOMALY_THRESHOLD = PHI_INV_2; // 38.2% deviation triggers judgment
const JUDGMENT_COOLDOWN_MS = 30000;  // 30s between auto-judgments

/**
 * Get adaptive threshold or fall back to static
 * @param {string} category - Threshold category
 * @param {string} key - Threshold key
 * @param {number} fallback - Static fallback value
 * @returns {number} The threshold value
 */
function getAdaptiveThreshold(category, key, fallback) {
  if (adaptiveLearn) {
    const adaptive = adaptiveLearn.getThreshold(category, key);
    if (adaptive !== undefined && adaptive !== null) {
      return adaptive;
    }
  }
  return fallback;
}

// Adaptive threshold getters
function getErrorThreshold() {
  return getAdaptiveThreshold('error', 'count', STATIC_ERROR_THRESHOLD);
}

function getSuccessStreak() {
  return getAdaptiveThreshold('success', 'count', STATIC_SUCCESS_STREAK);
}

function getCodeChangeThreshold() {
  return getAdaptiveThreshold('codeChange', 'rapidCount', STATIC_CODE_CHANGE_THRESHOLD);
}

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

    const errorThreshold = getErrorThreshold();
    if (similarErrors.length >= errorThreshold) {
      // Record observation for adaptive learning
      feedAdaptiveLearning('error', 'count', similarErrors.length);

      return createJudgment({
        trigger: 'error_pattern',
        verdict: 'GROWL',
        subject: currentObs.signature,
        reason: `Erreur répétée ${similarErrors.length}x (seuil: ${errorThreshold}): ${currentObs.description}`,
        confidence: Math.min(PHI_INV, similarErrors.length * PHI_INV_3),
        observations: similarErrors.map(o => o.id),
        recommendation: 'Investiguer la cause racine. Pattern d\'erreur détecté.',
        adaptiveThreshold: errorThreshold,
      });
    }
  }

  // === TRIGGER 2: Success Streak ===
  const recentSuccesses = recent.filter(o => o.type === ObservationType.SUCCESS);
  const successStreak = getSuccessStreak();
  if (recentSuccesses.length >= successStreak) {
    // Check if we haven't already judged this streak
    const lastSuccessJudgment = state.judgments.find(j =>
      j.trigger === 'success_streak' &&
      j.timestamp > now - 600000 // 10 min
    );

    if (!lastSuccessJudgment) {
      // Record observation for adaptive learning
      feedAdaptiveLearning('success', 'count', recentSuccesses.length);

      return createJudgment({
        trigger: 'success_streak',
        verdict: 'WAG',
        subject: 'session_progress',
        reason: `Série de ${recentSuccesses.length} opérations réussies (seuil: ${successStreak})`,
        confidence: PHI_INV_2,
        observations: recentSuccesses.slice(-successStreak).map(o => o.id),
        recommendation: 'Bon momentum. Continuer sur cette lancée.',
        adaptiveThreshold: successStreak,
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
    const codeChangeThreshold = getCodeChangeThreshold();
    if (fileChanges.length >= codeChangeThreshold) {
      // Record observation for adaptive learning
      feedAdaptiveLearning('codeChange', 'rapidCount', fileChanges.length);

      return createJudgment({
        trigger: 'rapid_changes',
        verdict: 'BARK',
        subject: currentObs.file,
        reason: `${fileChanges.length} modifications rapides sur ${path.basename(currentObs.file)} (seuil: ${codeChangeThreshold})`,
        confidence: PHI_INV_2,
        observations: fileChanges.map(o => o.id),
        recommendation: 'Beaucoup de changements. Prendre du recul et vérifier la cohérence?',
        adaptiveThreshold: codeChangeThreshold,
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
 * Optionally enhanced with LLM reasoning
 */
function createJudgment({ trigger, verdict, subject, reason, confidence, observations, recommendation }) {
  const now = Date.now();

  const judgment = {
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
    // Q-Score simplified (based on verdict — φ-aligned bands)
    // HOWL ≥ 76, WAG 61-75, GROWL 38-60, BARK < 38
    qScore: verdict === 'HOWL' ? 88 :
            verdict === 'WAG' ? 68 :
            verdict === 'GROWL' ? 49 :
            verdict === 'BARK' ? 19 : 50,
    source: 'static', // Default: static rules
  };

  // Async LLM enhancement (non-blocking)
  if (llmBridge && process.env.CYNIC_LLM_ENHANCE !== 'false') {
    setImmediate(async () => {
      try {
        const llmResult = await llmBridge.llmRefine(judgment, {
          trigger,
          observations: observations?.length || 0,
        });

        if (llmResult.success && llmResult.shouldRefine) {
          // Store refined judgment for learning
          const refinedJudgment = {
            ...judgment,
            llmRefined: true,
            originalScore: judgment.qScore,
            qScore: llmResult.refinedScore || judgment.qScore,
            verdict: llmResult.refinedVerdict || judgment.verdict,
            confidence: Math.min(llmResult.refinedConfidence || judgment.confidence, PHI_INV),
            llmReason: llmResult.refinementReason,
            critiques: llmResult.critiques,
          };

          // Append to judgments file
          appendJudgment(refinedJudgment);
        }
      } catch (e) {
        // LLM enhancement failed - continue with static judgment
      }
    });
  }

  return judgment;
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
// ADAPTIVE LEARNING INTEGRATION
// =============================================================================

/**
 * Feed observation data to adaptive learning system
 * This allows thresholds to evolve based on actual patterns
 */
function feedAdaptiveLearning(type, context, value) {
  if (!adaptiveLearn) return;

  try {
    adaptiveLearn.recordObservation({
      type,
      context,
      value,
      metadata: {
        source: 'auto-judge',
        sessionId: process.env.CYNIC_SESSION_ID || 'unknown',
      },
    });
  } catch (e) {
    // Silently ignore - learning is optional enhancement
  }
}

/**
 * Provide feedback on a judgment (for calibration)
 * Call this when user confirms or rejects an auto-judgment
 * @param {string} judgmentId - The judgment being rated
 * @param {boolean} wasCorrect - Was the judgment accurate?
 * @param {string} [correction] - What should have happened instead?
 */
function provideFeedback(judgmentId, wasCorrect, correction = null) {
  if (!adaptiveLearn) {
    return { success: false, reason: 'Adaptive learning not available' };
  }

  // Find the judgment
  const state = loadState();
  const judgment = state.judgments.find(j => j.id === judgmentId);

  if (!judgment) {
    return { success: false, reason: 'Judgment not found' };
  }

  try {
    adaptiveLearn.recordFeedback({
      type: judgment.trigger,
      detectionId: judgmentId,
      correct: wasCorrect,
      correction,
    });

    return {
      success: true,
      message: wasCorrect
        ? 'Feedback recorded: judgment confirmed ✓'
        : 'Feedback recorded: threshold will adjust',
    };
  } catch (e) {
    return { success: false, reason: e.message };
  }
}

/**
 * Get current adaptive learning stats
 */
function getAdaptiveStats() {
  if (!adaptiveLearn) {
    return { available: false };
  }

  return {
    available: true,
    thresholds: {
      error: getErrorThreshold(),
      success: getSuccessStreak(),
      codeChange: getCodeChangeThreshold(),
    },
    calibration: adaptiveLearn.getCalibrationStats(),
    learningStats: adaptiveLearn.loadStats(),
  };
}

/**
 * Trigger a BURN cycle to clean old learning data
 */
function triggerBurn() {
  if (!adaptiveLearn) {
    return { success: false, reason: 'Adaptive learning not available' };
  }

  return adaptiveLearn.burnCycle();
}

// =============================================================================
// LLM JUDGMENT FUNCTIONS
// =============================================================================

/**
 * Check if LLM is available for enhanced judgment
 */
async function checkLLMAvailable() {
  if (!llmBridge) return false;
  return llmBridge.checkOllama();
}

/**
 * Get LLM bridge stats
 */
function getLLMStats() {
  if (!llmBridge) {
    return { available: false, reason: 'LLM bridge not loaded' };
  }
  return llmBridge.getStats();
}

/**
 * Perform LLM-enhanced judgment on an item
 * Use this for deep analysis (slower but richer)
 */
async function llmJudge(item, context = {}) {
  if (!llmBridge) {
    return { success: false, error: 'LLM bridge not available' };
  }

  const result = await llmBridge.llmJudge(item, context);

  if (result.success) {
    // Store to judgments file
    const judgment = {
      ...result.judgment,
      id: `llm_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
      timestamp: Date.now(),
      autonomous: true,
      trigger: 'llm_direct',
      source: 'llm',
    };
    appendJudgment(judgment);

    // Update state
    const state = loadState();
    state.judgments.push(judgment);
    state.lastJudgment = Date.now();
    state.stats.totalJudgments++;
    saveState(state);

    return { success: true, judgment };
  }

  return result;
}

/**
 * Analyze patterns with LLM
 */
async function llmAnalyzePattern(pattern, observations = []) {
  if (!llmBridge) {
    return { success: false, error: 'LLM bridge not available' };
  }

  const result = await llmBridge.llmAnalyzePattern(pattern, observations);

  // If significant, feed to adaptive learning
  if (result.success && result.significant && adaptiveLearn && result.suggestedThresholdChange) {
    const { category, delta } = result.suggestedThresholdChange;
    adaptiveLearn.recordObservation({
      type: category,
      context: 'llm_suggested',
      value: delta,
      metadata: {
        source: 'llm-analysis',
        learning: result.learning,
      },
    });
  }

  return result;
}

/**
 * Set the LLM model to use
 */
function setLLMModel(model) {
  if (!llmBridge) {
    return { success: false, error: 'LLM bridge not available' };
  }
  return llmBridge.setModel(model);
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

  // Adaptive learning integration
  provideFeedback,
  getAdaptiveStats,
  triggerBurn,
  getErrorThreshold,
  getSuccessStreak,
  getCodeChangeThreshold,

  // LLM judgment (autonomous improvement)
  checkLLMAvailable,
  getLLMStats,
  llmJudge,
  llmAnalyzePattern,
  setLLMModel,

  // Types
  ObservationType,

  // Constants (deprecated - use get*Threshold() functions instead)
  PHI_INV,
  ERROR_THRESHOLD: STATIC_ERROR_THRESHOLD,
  SUCCESS_STREAK: STATIC_SUCCESS_STREAK,
};
