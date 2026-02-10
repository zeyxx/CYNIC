/**
 * InjectionProfile — Adaptive Injection Decisions
 *
 * Replaces 30 hardcoded decision points in perceive.js with learned weights.
 * Each injection topic gets a Thompson Sampling arm. Over time, CYNIC learns:
 * - Which topics the user engages with (inject more)
 * - Which topics get ignored (inject less)
 * - Optimal activation thresholds (replace hardcoded 0.4, 0.5)
 * - Optimal modulo frequencies (replace promptCount % 5)
 *
 * Learning signal: observe.js records which injected topics correlate with
 * user engagement (topic keywords appear in subsequent prompts) and
 * session quality (feedback score, judgment quality).
 *
 * Persisted at: ~/.cynic/context/injection-profile.json
 *
 * "Le chien apprend à qui donner sa patte" — κυνικός
 *
 * @module @cynic/node/services/injection-profile
 */

'use strict';

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { join } from 'path';
import { homedir } from 'os';
import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

const CONTEXT_DIR = join(homedir(), '.cynic', 'context');
const PROFILE_FILE = join(CONTEXT_DIR, 'injection-profile.json');

/**
 * Default activation rates per topic (φ-aligned).
 * These are the STARTING priors — learned rates override them.
 * Format: [alpha_prior, beta_prior] for Beta distribution.
 *
 * Higher alpha = more likely to activate.
 * Beta(2,3) ≈ 40% activation rate (matches old 0.4 threshold)
 * Beta(3,5) ≈ 37.5% (close to φ⁻²)
 * Beta(2,6) ≈ 25% (close to φ⁻³)
 */
const DEFAULT_PRIORS = {
  // Awareness (old: promptCount % 5 = 20% baseline)
  ecosystem_status: [2, 8],   // ~20% — most users don't care every 5 prompts
  social_status: [1, 9],      // ~10% — even fewer care about social
  accounting_status: [1, 9],  // ~10% — rarely useful

  // Socratic (old: Math.random() < φ⁻²/φ⁻³)
  elenchus: [3, 5],           // ~37.5% — was φ⁻² (38.2%)
  chria_wisdom: [3, 5],       // ~37.5% — was φ⁻²
  role_reversal: [2, 6],      // ~25% — was φ⁻³ (23.6%)
  hypothesis: [3, 5],         // ~37.5% — was φ⁻²
  dog_hint: [2, 6],           // ~25% — was φ⁻³

  // Temporal signals (old: confidence > 0.4)
  temporal_late_night: [2, 3], // ~40% — most users appreciate the warning
  temporal_frustration: [2, 3],
  temporal_fatigue: [2, 3],
  temporal_flow: [2, 6],       // ~25% — don't interrupt flow too often
  temporal_stuck: [3, 5],      // ~37.5% — stuck users need help
  temporal_weekend: [1, 9],    // ~10% — weekend warnings are usually annoying

  // Error signals
  error_circuit_breaker: [4, 1], // ~80% — circuit breakers are critical
  error_high_rate: [3, 2],      // ~60% — high error rates are useful to flag

  // Routing
  auto_dispatch: [4, 2],        // ~66% — was φ⁻¹ (61.8%)
  guardian_emergency: [5, 1],   // ~83% — guardian should almost always fire

  // Complexity analysis (old: disabled by compressor)
  complexity_analysis: [1, 9],  // ~10% — rarely useful
  optimize_analysis: [1, 9],    // ~10% — rarely useful
};

/**
 * Keyword families for engagement detection.
 * When observe.js sees these keywords in the user's next prompt,
 * it means the user ENGAGED with the corresponding injection.
 */
const ENGAGEMENT_KEYWORDS = {
  ecosystem_status: /\b(ecosystem|repo|deploy|render|github|status|service|build|ci)\b/i,
  social_status: /\b(twitter|tweet|x\.com|social|community|follower|sentiment|post)\b/i,
  accounting_status: /\b(cost|accounting|burn|token|spent|budget|economic)\b/i,
  elenchus: /\b(question|why|reason|because|think|rethink|reconsider|doubt)\b/i,
  chria_wisdom: /\b(wisdom|philosophy|principle|axiom|lesson|insight)\b/i,
  hypothesis: /\b(hypothesis|assume|believe|test|verify|prove|evidence)\b/i,
  temporal_late_night: /\b(tired|break|stop|sleep|later|tomorrow|continue)\b/i,
  temporal_frustration: /\b(stuck|different|approach|try|alternative|help)\b/i,
  temporal_fatigue: /\b(break|pause|rest|tired|enough|stop)\b/i,
  error_circuit_breaker: /\b(error|fix|debug|broken|wrong|issue|problem)\b/i,
};

/** Maximum number of tracked topics before pruning */
const MAX_TOPICS = 40;

/** Minimum observations before adaptive rates override defaults */
const MIN_OBSERVATIONS = 5;

/** Maximum state file size (10KB) */
const MAX_FILE_SIZE = 10 * 1024;

// ═══════════════════════════════════════════════════════════════════════════
// INJECTION PROFILE
// ═══════════════════════════════════════════════════════════════════════════

class InjectionProfile {
  constructor() {
    this._running = false;

    // Thompson arms per topic: { alpha, beta, lastUpdate, engagements, ignores }
    this._arms = new Map();

    // Learned thresholds: { value, adjustments, lastAdjusted }
    this._thresholds = new Map();

    // Session tracking
    this._sessionInjections = new Set(); // topics injected this session
    this._lastSessionInjections = [];    // previous session's injections (persisted for cross-process engagement)
    this._sessionPromptCount = 0;

    // Lifetime stats
    this._totalSessions = 0;
    this._totalEngagements = 0;
    this._totalIgnores = 0;
  }

  // ═══════════════════════════════════════════════════════════════════════
  // LIFECYCLE
  // ═══════════════════════════════════════════════════════════════════════

  start() {
    if (this._running) return;
    this._running = true;
    this._loadState();
    this._totalSessions += 1;
    this._sessionInjections = new Set();
    this._sessionPromptCount = 0;
  }

  stop() {
    if (!this._running) return;
    this._persistState();
    this._running = false;
  }

  // ═══════════════════════════════════════════════════════════════════════
  // CORE: ACTIVATION DECISIONS
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Should this topic activate? Replaces `Math.random() < CONSTANT`.
   *
   * Uses Thompson Sampling: draws from the topic's Beta posterior.
   * New topics use default priors. Experienced topics use learned rates.
   *
   * @param {string} topic - Topic identifier (must be in DEFAULT_PRIORS)
   * @returns {{ activate: boolean, rate: number, reason: string }}
   */
  shouldActivate(topic) {
    if (!this._running) return { activate: true, rate: 1, reason: 'not_running' };

    const arm = this._getOrCreateArm(topic);
    const rate = this._getActivationRate(arm);

    // Draw from posterior
    const draw = Math.random();
    const activate = draw < rate;

    if (activate) {
      this._sessionInjections.add(topic);
    }

    return {
      activate,
      rate: Math.round(rate * 1000) / 1000,
      reason: arm.engagements + arm.ignores < MIN_OBSERVATIONS ? 'prior' : 'learned',
    };
  }

  /**
   * Get the learned activation rate for a topic.
   * Returns the expected value of the Beta posterior, capped at φ⁻¹.
   *
   * @param {string} topic
   * @returns {number} Activation rate (0..φ⁻¹)
   */
  getRate(topic) {
    const arm = this._getOrCreateArm(topic);
    return this._getActivationRate(arm);
  }

  /**
   * Should this periodic topic activate NOW based on learned frequency?
   * Replaces `promptCount % N === K` patterns.
   *
   * Instead of fixed modulo, uses learned interest rate to determine
   * how many prompts between activations.
   *
   * @param {string} topic
   * @param {number} promptCount - Current prompt number in session
   * @returns {{ activate: boolean, rate: number, interval: number }}
   */
  shouldActivatePeriodic(topic, promptCount) {
    if (!this._running) return { activate: true, rate: 1, interval: 1 };

    this._sessionPromptCount = promptCount;
    const arm = this._getOrCreateArm(topic);
    const rate = this._getActivationRate(arm);

    // Convert rate to interval: rate 0.2 = every ~5 prompts, rate 0.5 = every ~2
    // interval = max(1, round(1 / rate))
    const interval = Math.max(1, Math.round(1 / Math.max(rate, 0.01)));

    // Activate on first prompt, then every `interval` prompts
    const activate = promptCount === 1 || (promptCount > 1 && promptCount % interval === 0);

    if (activate) {
      this._sessionInjections.add(topic);
    }

    return {
      activate,
      rate: Math.round(rate * 1000) / 1000,
      interval,
    };
  }

  /**
   * Get a learned confidence threshold. Replaces hardcoded 0.4, 0.5.
   *
   * Over time, thresholds adjust based on whether the signal
   * was useful (lower threshold) or noisy (raise threshold).
   *
   * @param {string} name - Threshold identifier
   * @param {number} defaultValue - Original hardcoded value
   * @returns {number} Learned threshold
   */
  getThreshold(name, defaultValue) {
    if (!this._running) return defaultValue;

    const threshold = this._thresholds.get(name);
    if (!threshold || threshold.adjustments < MIN_OBSERVATIONS) {
      return defaultValue; // Not enough data yet
    }
    return threshold.value;
  }

  // ═══════════════════════════════════════════════════════════════════════
  // LEARNING: ENGAGEMENT SIGNALS
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Record that the user engaged with an injected topic.
   * Called by observe.js when user's prompt correlates with an injection.
   *
   * @param {string} topic
   */
  recordEngagement(topic) {
    const arm = this._getOrCreateArm(topic);
    arm.alpha += 1;
    arm.engagements += 1;
    arm.lastUpdate = Date.now();
    this._totalEngagements += 1;
  }

  /**
   * Record that the user ignored an injected topic.
   * Called by observe.js when user's prompt does NOT correlate.
   *
   * @param {string} topic
   */
  recordIgnore(topic) {
    const arm = this._getOrCreateArm(topic);
    arm.beta += 1;
    arm.ignores += 1;
    arm.lastUpdate = Date.now();
    this._totalIgnores += 1;
  }

  /**
   * Bulk-update engagement for all topics injected this session.
   * Called at session end. Checks which topics had engagement keywords
   * appear in the user's prompts during the session.
   *
   * @param {string[]} sessionPrompts - All user prompts from this session
   */
  updateFromSession(sessionPrompts) {
    if (!sessionPrompts || sessionPrompts.length === 0) return;

    const combinedText = sessionPrompts.join(' ');

    for (const topic of this._sessionInjections) {
      const keywords = ENGAGEMENT_KEYWORDS[topic];
      if (!keywords) continue;

      if (keywords.test(combinedText)) {
        this.recordEngagement(topic);
      } else {
        this.recordIgnore(topic);
      }
    }
  }

  /**
   * Adjust a threshold based on outcome.
   *
   * @param {string} name - Threshold name
   * @param {number} defaultValue - Original hardcoded value
   * @param {boolean} useful - Was the signal useful at current threshold?
   */
  adjustThreshold(name, defaultValue, useful) {
    let threshold = this._thresholds.get(name);
    if (!threshold) {
      threshold = { value: defaultValue, adjustments: 0, lastAdjusted: Date.now() };
    }

    // Small adjustments: useful → lower threshold (catch more), useless → raise (filter noise)
    const step = 0.02; // 2% per adjustment
    if (useful) {
      threshold.value = Math.max(0.1, threshold.value - step);
    } else {
      threshold.value = Math.min(PHI_INV, threshold.value + step);
    }

    threshold.adjustments += 1;
    threshold.lastAdjusted = Date.now();
    this._thresholds.set(name, threshold);
  }

  /**
   * Get the set of topics injected this session.
   * Used by observe.js to know what to check engagement for.
   *
   * @returns {string[]}
   */
  getSessionInjections() {
    return [...this._sessionInjections];
  }

  /**
   * Get the PREVIOUS session's injections (persisted across processes).
   * Used by perceive.js to detect engagement on the next hook invocation.
   *
   * @returns {string[]}
   */
  getLastSessionInjections() {
    return [...this._lastSessionInjections];
  }

  // ═══════════════════════════════════════════════════════════════════════
  // STATS & DIAGNOSTICS
  // ═══════════════════════════════════════════════════════════════════════

  getStats() {
    const arms = {};
    for (const [topic, arm] of this._arms) {
      arms[topic] = {
        rate: Math.round(this._getActivationRate(arm) * 1000) / 1000,
        alpha: arm.alpha,
        beta: arm.beta,
        engagements: arm.engagements,
        ignores: arm.ignores,
        learned: arm.engagements + arm.ignores >= MIN_OBSERVATIONS,
      };
    }

    const thresholds = {};
    for (const [name, t] of this._thresholds) {
      thresholds[name] = {
        value: Math.round(t.value * 1000) / 1000,
        adjustments: t.adjustments,
      };
    }

    return {
      running: this._running,
      totalSessions: this._totalSessions,
      totalEngagements: this._totalEngagements,
      totalIgnores: this._totalIgnores,
      sessionInjections: [...this._sessionInjections],
      arms,
      thresholds,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════
  // INTERNAL: Thompson Sampling
  // ═══════════════════════════════════════════════════════════════════════

  _getOrCreateArm(topic) {
    if (this._arms.has(topic)) return this._arms.get(topic);

    const priors = DEFAULT_PRIORS[topic] || [2, 3]; // Default: ~40%
    const arm = {
      alpha: priors[0],
      beta: priors[1],
      engagements: 0,
      ignores: 0,
      lastUpdate: Date.now(),
    };
    this._arms.set(topic, arm);
    return arm;
  }

  /**
   * Get activation rate from Beta posterior.
   * E[Beta(α, β)] = α / (α + β), capped at φ⁻¹.
   */
  _getActivationRate(arm) {
    const expected = arm.alpha / (arm.alpha + arm.beta);
    return Math.min(expected, PHI_INV);
  }

  // ═══════════════════════════════════════════════════════════════════════
  // PERSISTENCE
  // ═══════════════════════════════════════════════════════════════════════

  _loadState() {
    try {
      if (!existsSync(PROFILE_FILE)) return;
      const raw = readFileSync(PROFILE_FILE, 'utf-8');
      const state = JSON.parse(raw);

      this._totalSessions = state.totalSessions || 0;
      this._totalEngagements = state.totalEngagements || 0;
      this._totalIgnores = state.totalIgnores || 0;

      if (state.arms) {
        this._arms = new Map(Object.entries(state.arms));
        // Prune old topics
        if (this._arms.size > MAX_TOPICS) {
          const entries = [...this._arms.entries()]
            .sort((a, b) => b[1].lastUpdate - a[1].lastUpdate)
            .slice(0, MAX_TOPICS);
          this._arms = new Map(entries);
        }
      }

      if (state.thresholds) {
        this._thresholds = new Map(Object.entries(state.thresholds));
      }

      // Restore previous session's injections for cross-process engagement detection
      if (state.lastSessionInjections) {
        this._lastSessionInjections = state.lastSessionInjections;
      }
    } catch {
      // Fresh start
    }
  }

  _persistState() {
    try {
      if (!existsSync(CONTEXT_DIR)) {
        mkdirSync(CONTEXT_DIR, { recursive: true });
      }

      const arms = {};
      for (const [key, val] of this._arms) arms[key] = val;

      const thresholds = {};
      for (const [key, val] of this._thresholds) thresholds[key] = val;

      const state = {
        totalSessions: this._totalSessions,
        totalEngagements: this._totalEngagements,
        totalIgnores: this._totalIgnores,
        arms,
        thresholds,
        lastSessionInjections: [...this._sessionInjections],
        lastPersisted: Date.now(),
      };

      const serialized = JSON.stringify(state, null, 2);
      if (serialized.length > MAX_FILE_SIZE) {
        // Prune least-used arms until under limit
        const sorted = [...this._arms.entries()]
          .sort((a, b) => (a[1].engagements + a[1].ignores) - (b[1].engagements + b[1].ignores));
        while (sorted.length > 10 && JSON.stringify(state).length > MAX_FILE_SIZE) {
          const removed = sorted.shift();
          delete state.arms[removed[0]];
        }
      }

      writeFileSync(PROFILE_FILE, JSON.stringify(state, null, 2));
    } catch {
      // Non-blocking
    }
  }

  // ═══════════════════════════════════════════════════════════════════════
  // TESTING
  // ═══════════════════════════════════════════════════════════════════════

  _resetForTesting() {
    this._running = false;
    this._arms = new Map();
    this._thresholds = new Map();
    this._sessionInjections = new Set();
    this._lastSessionInjections = [];
    this._sessionPromptCount = 0;
    this._totalSessions = 0;
    this._totalEngagements = 0;
    this._totalIgnores = 0;
  }

  _setStatePath(path) {
    this._customStatePath = path;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

const injectionProfile = new InjectionProfile();

// Override persistence to support custom path (testing)
const origLoad = injectionProfile._loadState.bind(injectionProfile);
const origPersist = injectionProfile._persistState.bind(injectionProfile);

injectionProfile._loadState = function () {
  if (this._customStatePath) {
    try {
      if (!existsSync(this._customStatePath)) return;
      const raw = readFileSync(this._customStatePath, 'utf-8');
      const state = JSON.parse(raw);
      this._totalSessions = state.totalSessions || 0;
      this._totalEngagements = state.totalEngagements || 0;
      this._totalIgnores = state.totalIgnores || 0;
      if (state.arms) this._arms = new Map(Object.entries(state.arms));
      if (state.thresholds) this._thresholds = new Map(Object.entries(state.thresholds));
      if (state.lastSessionInjections) this._lastSessionInjections = state.lastSessionInjections;
    } catch { /* fresh start */ }
  } else {
    origLoad();
  }
};

injectionProfile._persistState = function () {
  const targetFile = this._customStatePath || PROFILE_FILE;
  const targetDir = join(targetFile, '..');
  try {
    if (!existsSync(targetDir)) mkdirSync(targetDir, { recursive: true });
    const arms = {};
    for (const [key, val] of this._arms) arms[key] = val;
    const thresholds = {};
    for (const [key, val] of this._thresholds) thresholds[key] = val;
    const state = {
      totalSessions: this._totalSessions,
      totalEngagements: this._totalEngagements,
      totalIgnores: this._totalIgnores,
      arms,
      thresholds,
      lastSessionInjections: [...this._sessionInjections],
      lastPersisted: Date.now(),
    };
    writeFileSync(targetFile, JSON.stringify(state, null, 2));
  } catch { /* non-blocking */ }
};

export { injectionProfile, DEFAULT_PRIORS, ENGAGEMENT_KEYWORDS, MIN_OBSERVATIONS };
export default injectionProfile;
