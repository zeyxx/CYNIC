/**
 * ContextCompressor — Experience Curve for Context Injection
 *
 * The missing architectural piece: CYNIC should consume LESS context over time,
 * not more. This module tracks injection history, accepts maturity signals from
 * learning modules, and decides what to skip or compress.
 *
 * Session 1:   Full context — CYNIC is learning
 * Session 100: Minimal context — CYNIC remembers
 *
 * Core principle: φ⁻¹ bounds everything.
 * After enough experience, context budget converges to φ⁻² of initial.
 *
 * Persisted at: ~/.cynic/context/compressor-state.json
 *
 * "Le chien qui sait n'a pas besoin qu'on lui répète" — κυνικός
 *
 * @module @cynic/node/services/context-compressor
 */

'use strict';

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { join } from 'path';
import { homedir } from 'os';
import { PHI_INV, PHI_INV_2 } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

const CONTEXT_DIR = join(homedir(), '.cynic', 'context');
const STATE_FILE = join(CONTEXT_DIR, 'compressor-state.json');

/** Maximum topic entries before pruning oldest */
const MAX_TOPICS = 50;

/** Session threshold for "experienced" — after this, compression kicks in */
const EXPERIENCED_THRESHOLD = 10;

/** Session threshold for "expert" — maximum compression */
const EXPERT_THRESHOLD = 50;

/** Maximum session outcomes to retain (rolling window) */
const MAX_OUTCOMES = 20;

/** Number of recent sessions to evaluate for backoff */
const BACKOFF_WINDOW = 3;

/** Quality threshold below which backoff triggers */
const BACKOFF_QUALITY_THRESHOLD = 0.4;

/** Number of sessions a backoff lasts */
const BACKOFF_DURATION = 5;

/**
 * Known injection topics and their properties.
 * staleTTL: how long (ms) before this topic can be re-injected
 * compressible: whether this topic supports compressed output
 * disableAfter: session count after which this topic is disabled entirely
 */
const TOPIC_CONFIG = {
  // SILENT MCP tools — zero routing impact, pure waste
  complexity_analysis: { staleTTL: Infinity, compressible: false, disableAfter: 0 },
  optimize_analysis: { staleTTL: Infinity, compressible: false, disableAfter: 0 },

  // Periodic awareness — once per session is enough
  ecosystem_status: { staleTTL: 300_000, compressible: true, disableAfter: null },  // 5min
  social_status: { staleTTL: 300_000, compressible: true, disableAfter: null },      // 5min
  accounting_status: { staleTTL: 300_000, compressible: true, disableAfter: null },  // 5min

  // Core perception — always inject but compress with experience
  framing_directive: { staleTTL: 0, compressible: true, disableAfter: null },
  consciousness_state: { staleTTL: 60_000, compressible: true, disableAfter: null }, // 1min
  dog_routing: { staleTTL: 30_000, compressible: true, disableAfter: null },         // 30s
  pattern_memory: { staleTTL: 120_000, compressible: true, disableAfter: null },     // 2min

  // Socratic modules — reduce frequency with experience
  planning_gate: { staleTTL: 60_000, compressible: false, disableAfter: null },
  maieutic: { staleTTL: 180_000, compressible: false, disableAfter: null },          // 3min
  hypothesis: { staleTTL: 180_000, compressible: false, disableAfter: null },        // 3min
  elenchus: { staleTTL: 120_000, compressible: false, disableAfter: null },          // 2min
  physics: { staleTTL: 300_000, compressible: false, disableAfter: null },           // 5min

  // Temporal/error — inject only when relevant
  temporal: { staleTTL: 60_000, compressible: true, disableAfter: null },
  error_perception: { staleTTL: 0, compressible: false, disableAfter: null },

  // Intent-specific context
  intent_danger: { staleTTL: 0, compressible: false, disableAfter: null },
  intent_decision: { staleTTL: 60_000, compressible: true, disableAfter: null },
  intent_debug: { staleTTL: 30_000, compressible: false, disableAfter: null },
  intent_architecture: { staleTTL: 60_000, compressible: true, disableAfter: null },
  intent_learning: { staleTTL: 120_000, compressible: true, disableAfter: null },
};

// ═══════════════════════════════════════════════════════════════════════════
// CONTEXT COMPRESSOR
// ═══════════════════════════════════════════════════════════════════════════

class ContextCompressor {
  constructor() {
    this._running = false;
    this._startedAt = null;

    // Per-topic injection tracking (persisted)
    this._topics = new Map(); // topic → { count, lastInjected, lastSkipped, totalChars }

    // Session-level tracking
    this._sessionInjections = 0;
    this._sessionSkips = 0;
    this._sessionCharsSaved = 0;

    // Global experience (persisted)
    this._totalSessions = 0;
    this._totalInjections = 0;
    this._totalSkips = 0;
    this._totalCharsSaved = 0;

    // Learning maturity signals (set by learning modules)
    this._maturitySignals = new Map(); // module → { maturity: 0..1, converged: bool, lastUpdate }

    // Last stable routing (for dedup)
    this._lastRouting = null;

    // Session outcome history (persisted) — rolling window for backoff circuit breaker
    this._sessionOutcomes = []; // [{ ts, level, quality, errorRate, frustration }]
    this._backoffUntilSession = 0; // session count until which backoff is active
  }

  // ═══════════════════════════════════════════════════════════════════════
  // LIFECYCLE
  // ═══════════════════════════════════════════════════════════════════════

  start() {
    if (this._running) return;
    this._running = true;
    this._startedAt = Date.now();
    this._loadState();
    this._totalSessions += 1;
  }

  stop() {
    if (!this._running) return;
    this._persistState();
    this._running = false;
  }

  // ═══════════════════════════════════════════════════════════════════════
  // CORE DECISIONS
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Should this topic be injected into the current prompt?
   *
   * Decision tree:
   * 1. Topic disabled entirely → NO
   * 2. Topic stale TTL not expired → NO (recently injected)
   * 3. Experience-based frequency reduction → MAYBE
   * 4. Maturity signal says converged → REDUCED frequency
   *
   * @param {string} topic - Topic identifier (must be in TOPIC_CONFIG)
   * @param {Object} [options] - Additional context
   * @param {number} [options.estimatedChars] - Estimated injection size
   * @param {boolean} [options.force] - Force injection regardless of rules
   * @returns {{ inject: boolean, reason: string }}
   */
  shouldInject(topic, options = {}) {
    if (!this._running) return { inject: true, reason: 'compressor_not_running' };
    if (options.force) return { inject: true, reason: 'forced' };

    const config = TOPIC_CONFIG[topic];
    if (!config) return { inject: true, reason: 'unknown_topic' };

    // Rule 1: Disabled topics
    if (config.disableAfter !== null && this._totalSessions >= config.disableAfter) {
      this._recordSkip(topic, options.estimatedChars || 0);
      return { inject: false, reason: 'disabled' };
    }

    // Rule 2: Stale TTL — was this topic injected recently?
    const topicData = this._topics.get(topic);
    if (topicData && config.staleTTL > 0) {
      const elapsed = Date.now() - topicData.lastInjected;
      if (elapsed < config.staleTTL) {
        this._recordSkip(topic, options.estimatedChars || 0);
        return { inject: false, reason: 'stale_ttl' };
      }
    }

    // Rule 3: Experience-based frequency reduction
    // After EXPERIENCED_THRESHOLD sessions, non-essential topics inject less often
    if (this._totalSessions > EXPERIENCED_THRESHOLD && topicData) {
      const experienceRatio = Math.min(this._totalSessions / EXPERT_THRESHOLD, 1);
      // Frequency multiplier: 1.0 (new) → PHI_INV_2 (expert)
      const freqMultiplier = 1 - (experienceRatio * (1 - PHI_INV_2));
      // Skip probabilistically based on experience
      if (topicData.count > 5 && Math.random() > freqMultiplier) {
        this._recordSkip(topic, options.estimatedChars || 0);
        return { inject: false, reason: 'experience_reduction' };
      }
    }

    // Rule 4: Learning maturity — if the related module is converged, reduce
    const maturity = this._getTopicMaturity(topic);
    if (maturity > PHI_INV && topicData && topicData.count > 10) {
      // High maturity + enough history → skip 38.2% of the time
      if (Math.random() < PHI_INV_2) {
        this._recordSkip(topic, options.estimatedChars || 0);
        return { inject: false, reason: 'maturity_converged' };
      }
    }

    // Inject — record it
    this._recordInjection(topic, options.estimatedChars || 0);
    return { inject: true, reason: 'allowed' };
  }

  /**
   * Compress full injection data based on experience and maturity.
   *
   * Returns either the full data (new sessions) or a compressed version (experienced).
   *
   * @param {string} topic - Topic identifier
   * @param {string} fullData - Full injection text
   * @returns {string} Compressed or full data
   */
  compress(topic, fullData) {
    if (!this._running || !fullData) return fullData;
    if (typeof fullData !== 'string') return fullData;

    const config = TOPIC_CONFIG[topic];
    if (!config || !config.compressible) return fullData;

    // No compression for new users
    if (this._totalSessions <= EXPERIENCED_THRESHOLD) return fullData;

    // Compression level: 0 (none) to 1 (maximum)
    const level = this._getCompressionLevel();

    // Apply topic-specific compression
    return this._compressTopic(topic, fullData, level);
  }

  // ═══════════════════════════════════════════════════════════════════════
  // MATURITY SIGNALS
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Report learning maturity from a module.
   * Called by learning modules when they update their convergence state.
   *
   * @param {string} module - Module name (e.g., 'thompson', 'dpo', 'router')
   * @param {Object} signal
   * @param {number} signal.maturity - 0..1 how converged this module is
   * @param {boolean} signal.converged - Whether the module considers itself converged
   */
  reportMaturity(module, signal) {
    this._maturitySignals.set(module, {
      maturity: Math.min(signal.maturity || 0, PHI_INV), // φ-bounded
      converged: signal.converged || false,
      lastUpdate: Date.now(),
    });
  }

  /**
   * Report stable routing (for dedup).
   * If the same dog is leading N times in a row, compress routing injection.
   *
   * @param {string} leadDog - Current lead dog name
   * @param {string} reason - Routing reason
   */
  reportRouting(leadDog, reason) {
    if (!this._lastRouting) {
      this._lastRouting = { dog: leadDog, reason, count: 1 };
      return;
    }
    if (this._lastRouting.dog === leadDog) {
      this._lastRouting.count += 1;
    } else {
      this._lastRouting = { dog: leadDog, reason, count: 1 };
    }
  }

  /**
   * Check if routing is stable (same dog leading 3+ times).
   * @returns {boolean}
   */
  isRoutingStable() {
    return !!(this._lastRouting && this._lastRouting.count >= 3);
  }

  // ═══════════════════════════════════════════════════════════════════════
  // STATS & DIAGNOSTICS
  // ═══════════════════════════════════════════════════════════════════════

  getStats() {
    const topicStats = {};
    for (const [topic, data] of this._topics) {
      topicStats[topic] = {
        count: data.count,
        lastInjected: data.lastInjected,
        totalChars: data.totalChars,
      };
    }

    return {
      running: this._running,
      uptime: this._running ? Date.now() - this._startedAt : 0,
      totalSessions: this._totalSessions,
      experienceLevel: this.getEffectiveExperienceLevel(),
      rawExperienceLevel: this._getExperienceLevel(),
      backoff: this.getBackoffStatus(),
      compressionLevel: this._running ? Math.round(this._getCompressionLevel() * 100) : 0,
      session: {
        injections: this._sessionInjections,
        skips: this._sessionSkips,
        charsSaved: this._sessionCharsSaved,
        compressionRatio: this._sessionInjections > 0
          ? Math.round((this._sessionSkips / (this._sessionInjections + this._sessionSkips)) * 100)
          : 0,
      },
      lifetime: {
        totalInjections: this._totalInjections,
        totalSkips: this._totalSkips,
        totalCharsSaved: this._totalCharsSaved,
      },
      maturitySignals: Object.fromEntries(this._maturitySignals),
      topics: topicStats,
      routing: this._lastRouting,
    };
  }

  /**
   * Get overall maturity score (0..φ⁻¹).
   * Aggregates all maturity signals.
   * @returns {number}
   */
  getOverallMaturity() {
    if (this._maturitySignals.size === 0) return 0;
    let sum = 0;
    for (const [, signal] of this._maturitySignals) {
      sum += signal.maturity;
    }
    return Math.min(sum / this._maturitySignals.size, PHI_INV);
  }

  // ═══════════════════════════════════════════════════════════════════════
  // OUTCOME VERIFICATION: Circuit breaker for compression safety
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Record session quality outcome. Called at session end (sleep.js).
   * If quality is low at current compression level, triggers backoff.
   *
   * @param {{ quality: number, errorRate: number, frustration: number }} outcome
   *   quality: 0..1 composite score (1 = perfect session)
   *   errorRate: 0..1 tool error rate
   *   frustration: 0..1 psychology frustration signal
   */
  recordSessionOutcome(outcome) {
    const level = this._getExperienceLevel();
    const entry = {
      ts: Date.now(),
      session: this._totalSessions,
      level,
      quality: Math.max(0, Math.min(1, outcome.quality || 0)),
      errorRate: outcome.errorRate || 0,
      frustration: outcome.frustration || 0,
    };

    this._sessionOutcomes.push(entry);

    // Rolling window: keep only last MAX_OUTCOMES entries
    if (this._sessionOutcomes.length > MAX_OUTCOMES) {
      this._sessionOutcomes = this._sessionOutcomes.slice(-MAX_OUTCOMES);
    }

    // Evaluate backoff: check last BACKOFF_WINDOW sessions at current level
    const recentAtLevel = this._sessionOutcomes
      .filter(o => o.level === level)
      .slice(-BACKOFF_WINDOW);

    if (recentAtLevel.length >= BACKOFF_WINDOW) {
      const avgQuality = recentAtLevel.reduce((sum, o) => sum + o.quality, 0) / recentAtLevel.length;
      if (avgQuality < BACKOFF_QUALITY_THRESHOLD) {
        // Trigger backoff: temporarily degrade experience level
        this._backoffUntilSession = this._totalSessions + BACKOFF_DURATION;
      }
    }

    this._persistState();
  }

  /**
   * Get effective experience level, accounting for backoff.
   * This is what awaken.js should use instead of raw _getExperienceLevel().
   * @returns {string} 'new' | 'learning' | 'experienced' | 'expert'
   */
  getEffectiveExperienceLevel() {
    const raw = this._getExperienceLevel();

    // If backoff is active, degrade by 1 level
    if (this._backoffUntilSession > this._totalSessions) {
      const LEVELS = ['new', 'learning', 'experienced', 'expert'];
      const idx = LEVELS.indexOf(raw);
      if (idx > 0) return LEVELS[idx - 1];
      return raw; // Can't degrade below 'new'
    }

    return raw;
  }

  /**
   * Check if backoff is currently active.
   * @returns {{ active: boolean, remaining: number, reason: string }}
   */
  getBackoffStatus() {
    const active = this._backoffUntilSession > this._totalSessions;
    return {
      active,
      remaining: active ? this._backoffUntilSession - this._totalSessions : 0,
      reason: active ? 'quality_degradation' : 'none',
      rawLevel: this._getExperienceLevel(),
      effectiveLevel: this.getEffectiveExperienceLevel(),
    };
  }

  /**
   * Get session outcome history.
   * @returns {Array<{ ts, session, level, quality, errorRate, frustration }>}
   */
  getSessionOutcomes() {
    return [...this._sessionOutcomes];
  }

  // ═══════════════════════════════════════════════════════════════════════
  // INTERNAL: Compression
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * Get compression level based on experience + maturity.
   * 0 = no compression, 1 = maximum compression.
   * @returns {number}
   */
  _getCompressionLevel() {
    // Experience component: 0 → 1 over EXPERT_THRESHOLD sessions
    const experienceLevel = Math.min(this._totalSessions / EXPERT_THRESHOLD, 1);

    // Maturity component: average of all signals
    const maturityLevel = this.getOverallMaturity();

    // Blend: 60% experience, 40% maturity (experience is the base, maturity accelerates)
    const raw = (experienceLevel * 0.6) + (maturityLevel * 0.4);

    // Cap at φ⁻¹ — never fully compress everything
    return Math.min(raw, PHI_INV);
  }

  /**
   * Get experience level label.
   * @returns {string} 'new' | 'learning' | 'experienced' | 'expert'
   */
  _getExperienceLevel() {
    if (this._totalSessions <= 3) return 'new';
    if (this._totalSessions <= EXPERIENCED_THRESHOLD) return 'learning';
    if (this._totalSessions <= EXPERT_THRESHOLD) return 'experienced';
    return 'expert';
  }

  /**
   * Apply topic-specific compression.
   * @param {string} topic
   * @param {string} fullData
   * @param {number} level - 0..1 compression level
   * @returns {string}
   */
  _compressTopic(topic, fullData, level) {
    if (level < 0.2) return fullData; // Too early to compress

    switch (topic) {
      case 'framing_directive':
        return this._compressFramingDirective(fullData, level);
      case 'consciousness_state':
        return this._compressConsciousness(fullData, level);
      case 'dog_routing':
        return this._compressDogRouting(fullData, level);
      case 'ecosystem_status':
      case 'social_status':
      case 'accounting_status':
        return this._compressAwareness(topic, fullData, level);
      case 'pattern_memory':
        return this._compressPatternMemory(fullData, level);
      default:
        return this._compressGeneric(fullData, level);
    }
  }

  /**
   * Compress framing directive: 15 lines → 3-5 lines.
   * Keep: D value, lead dog, frame. Drop: votes, depth, memory when stable.
   */
  _compressFramingDirective(fullData, level) {
    const lines = fullData.split('\n');
    if (lines.length <= 5) return fullData; // Already compact

    // Always keep: header (line 0), D bar (line 1), Frame (last meaningful)
    const keep = [];
    for (const line of lines) {
      const trimmed = line.trim();
      // Always keep
      if (trimmed.startsWith('──') || trimmed.startsWith('D =') || trimmed.startsWith('Frame:')) {
        keep.push(line);
        continue;
      }
      // Keep at medium compression
      if (level < 0.4) {
        if (trimmed.startsWith('Lead:') || trimmed.startsWith('Axioms:') || trimmed.startsWith('Conscience:')) {
          keep.push(line);
        }
      }
      // Keep at low compression only
      if (level < 0.3) {
        if (trimmed.startsWith('Votes:') || trimmed.startsWith('Distribution:') ||
            trimmed.startsWith('Social:') || trimmed.startsWith('Accounting:') ||
            trimmed.startsWith('Memory:') || trimmed.startsWith('Depth:') ||
            trimmed.startsWith('Route:')) {
          keep.push(line);
        }
      }
    }

    return keep.length > 0 ? keep.join('\n') : fullData;
  }

  /**
   * Compress consciousness: full → 1 line when stable.
   */
  _compressConsciousness(fullData, level) {
    if (level < 0.3) return fullData;
    // Extract just the score and trend
    const scoreMatch = fullData.match(/score (\d+)/);
    const trendMatch = fullData.match(/trend ([\u2191\u2193\u2192]?\w+)/);
    if (scoreMatch && trendMatch) {
      return `   Conscience: ${scoreMatch[0]}, ${trendMatch[0]}`;
    }
    return fullData;
  }

  /**
   * Compress dog routing: skip if same as last time.
   */
  _compressDogRouting(fullData, level) {
    if (this.isRoutingStable() && level > 0.3) {
      return `   Routing: stable (${this._lastRouting.dog} ×${this._lastRouting.count})`;
    }
    return fullData;
  }

  /**
   * Compress awareness topics (ecosystem, social, accounting).
   */
  _compressAwareness(topic, fullData, level) {
    if (level < 0.3) return fullData;
    // Truncate to first line only
    const firstLine = fullData.split('\n')[0];
    return firstLine || fullData;
  }

  /**
   * Compress pattern memory: keep only top pattern when experienced.
   */
  _compressPatternMemory(fullData, level) {
    if (level < 0.4) return fullData;
    const lines = fullData.split('\n');
    return lines[0] || fullData;
  }

  /**
   * Generic compression: truncate long injections.
   */
  _compressGeneric(fullData, level) {
    const maxChars = Math.round(500 * (1 - level * 0.6)); // 500 → 200 chars
    if (fullData.length <= maxChars) return fullData;
    return fullData.slice(0, maxChars) + '...';
  }

  // ═══════════════════════════════════════════════════════════════════════
  // INTERNAL: Tracking
  // ═══════════════════════════════════════════════════════════════════════

  _recordInjection(topic, chars) {
    const data = this._topics.get(topic) || { count: 0, lastInjected: 0, lastSkipped: 0, totalChars: 0 };
    data.count += 1;
    data.lastInjected = Date.now();
    data.totalChars += chars;
    this._topics.set(topic, data);
    this._sessionInjections += 1;
    this._totalInjections += 1;
  }

  _recordSkip(topic, estimatedChars) {
    const data = this._topics.get(topic) || { count: 0, lastInjected: 0, lastSkipped: 0, totalChars: 0 };
    data.lastSkipped = Date.now();
    this._topics.set(topic, data);
    this._sessionSkips += 1;
    this._totalSkips += 1;
    this._sessionCharsSaved += estimatedChars;
    this._totalCharsSaved += estimatedChars;
  }

  _getTopicMaturity(topic) {
    // Map topics to learning modules
    const topicModuleMap = {
      framing_directive: 'router',
      dog_routing: 'router',
      consciousness_state: 'consciousness',
      pattern_memory: 'emergence',
      ecosystem_status: 'distribution',
      social_status: 'social',
      accounting_status: 'accounting',
    };
    const module = topicModuleMap[topic];
    if (!module) return 0;
    const signal = this._maturitySignals.get(module);
    return signal ? signal.maturity : 0;
  }

  // ═══════════════════════════════════════════════════════════════════════
  // PERSISTENCE
  // ═══════════════════════════════════════════════════════════════════════

  _loadState() {
    try {
      if (!existsSync(STATE_FILE)) return;
      const raw = readFileSync(STATE_FILE, 'utf-8');
      const state = JSON.parse(raw);

      this._totalSessions = state.totalSessions || 0;
      this._totalInjections = state.totalInjections || 0;
      this._totalSkips = state.totalSkips || 0;
      this._totalCharsSaved = state.totalCharsSaved || 0;

      if (state.topics) {
        this._topics = new Map(Object.entries(state.topics));
        // Prune old topics
        if (this._topics.size > MAX_TOPICS) {
          const entries = [...this._topics.entries()]
            .sort((a, b) => b[1].lastInjected - a[1].lastInjected)
            .slice(0, MAX_TOPICS);
          this._topics = new Map(entries);
        }
      }

      if (state.lastRouting) {
        this._lastRouting = state.lastRouting;
      }

      // Restore maturity signals (cross-process: daemon writes, hooks read)
      if (state.maturitySignals) {
        this._maturitySignals = new Map(Object.entries(state.maturitySignals));
      }

      // Restore session outcomes + backoff
      if (Array.isArray(state.sessionOutcomes)) {
        this._sessionOutcomes = state.sessionOutcomes.slice(-MAX_OUTCOMES);
      }
      if (state.backoffUntilSession != null) {
        this._backoffUntilSession = state.backoffUntilSession;
      }
    } catch {
      // Fresh start — file corrupt or missing
    }
  }

  _persistState() {
    try {
      if (!existsSync(CONTEXT_DIR)) {
        mkdirSync(CONTEXT_DIR, { recursive: true });
      }

      const topics = {};
      for (const [key, val] of this._topics) {
        topics[key] = val;
      }

      // Serialize maturity signals for cross-process sharing
      const maturitySignals = {};
      for (const [key, val] of this._maturitySignals) {
        maturitySignals[key] = val;
      }

      const state = {
        totalSessions: this._totalSessions,
        totalInjections: this._totalInjections,
        totalSkips: this._totalSkips,
        totalCharsSaved: this._totalCharsSaved,
        topics,
        maturitySignals,
        lastRouting: this._lastRouting,
        sessionOutcomes: this._sessionOutcomes.slice(-MAX_OUTCOMES),
        backoffUntilSession: this._backoffUntilSession,
        lastPersisted: Date.now(),
      };

      writeFileSync(STATE_FILE, JSON.stringify(state, null, 2));
    } catch {
      // Persistence failure is non-blocking
    }
  }

  // ═══════════════════════════════════════════════════════════════════════
  // TESTING
  // ═══════════════════════════════════════════════════════════════════════

  _resetForTesting() {
    this._running = false;
    this._startedAt = null;
    this._topics = new Map();
    this._sessionInjections = 0;
    this._sessionSkips = 0;
    this._sessionCharsSaved = 0;
    this._totalSessions = 0;
    this._totalInjections = 0;
    this._totalSkips = 0;
    this._totalCharsSaved = 0;
    this._maturitySignals = new Map();
    this._lastRouting = null;
    this._sessionOutcomes = [];
    this._backoffUntilSession = 0;
  }

  /**
   * Override state file path for testing (avoids clobbering real state).
   * @param {string} path
   */
  _setStatePath(path) {
    // Reassign the module-level constants via closure
    this._customStatePath = path;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

const contextCompressor = new ContextCompressor();

// Override persistence methods to respect custom path
const origLoad = contextCompressor._loadState.bind(contextCompressor);
const origPersist = contextCompressor._persistState.bind(contextCompressor);

contextCompressor._loadState = function () {
  if (this._customStatePath) {
    try {
      if (!existsSync(this._customStatePath)) return;
      const raw = readFileSync(this._customStatePath, 'utf-8');
      const state = JSON.parse(raw);
      this._totalSessions = state.totalSessions || 0;
      this._totalInjections = state.totalInjections || 0;
      this._totalSkips = state.totalSkips || 0;
      this._totalCharsSaved = state.totalCharsSaved || 0;
      if (state.topics) this._topics = new Map(Object.entries(state.topics));
      if (state.lastRouting) this._lastRouting = state.lastRouting;
      if (state.maturitySignals) this._maturitySignals = new Map(Object.entries(state.maturitySignals));
      if (Array.isArray(state.sessionOutcomes)) this._sessionOutcomes = state.sessionOutcomes.slice(-MAX_OUTCOMES);
      if (state.backoffUntilSession != null) this._backoffUntilSession = state.backoffUntilSession;
    } catch { /* fresh start */ }
  } else {
    origLoad();
  }
};

contextCompressor._persistState = function () {
  const targetFile = this._customStatePath || STATE_FILE;
  const targetDir = join(targetFile, '..');
  try {
    if (!existsSync(targetDir)) mkdirSync(targetDir, { recursive: true });
    const topics = {};
    for (const [key, val] of this._topics) topics[key] = val;
    const maturitySignals = {};
    for (const [key, val] of this._maturitySignals) maturitySignals[key] = val;
    const state = {
      totalSessions: this._totalSessions,
      totalInjections: this._totalInjections,
      totalSkips: this._totalSkips,
      totalCharsSaved: this._totalCharsSaved,
      topics,
      maturitySignals,
      lastRouting: this._lastRouting,
      sessionOutcomes: this._sessionOutcomes.slice(-MAX_OUTCOMES),
      backoffUntilSession: this._backoffUntilSession,
      lastPersisted: Date.now(),
    };
    writeFileSync(targetFile, JSON.stringify(state, null, 2));
  } catch { /* non-blocking */ }
};

export { contextCompressor, TOPIC_CONFIG, EXPERIENCED_THRESHOLD, EXPERT_THRESHOLD, BACKOFF_WINDOW, BACKOFF_QUALITY_THRESHOLD, BACKOFF_DURATION, MAX_OUTCOMES };
export default contextCompressor;
