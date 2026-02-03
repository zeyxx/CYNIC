/**
 * Brain Bridge - Connect hooks to CYNIC Brain
 *
 * This module bridges the hook system to the Brain consciousness layer.
 * It allows perceive.js and guard.js to use Brain.think() for unified
 * consciousness instead of scattered individual engine calls.
 *
 * "Le pont entre le corps et l'esprit" - κυνικός
 *
 * @module cynic/hooks/lib/brain-bridge
 */

'use strict';

import { createRequire } from 'module';
const requireCJS = createRequire(import.meta.url);

// φ constants
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;

/**
 * Lazy-loaded Brain Service instance
 * We load from @cynic/node when available, fallback to minimal brain
 */
let _brainService = null;
let _brainInitialized = false;
let _brainInitError = null;
let _useMinimalBrain = false;

/**
 * Initialize the Brain Service (lazy)
 * @returns {Object|null} Brain service or minimal brain, or null if unavailable
 */
async function initBrainService() {
  if (_brainInitialized) {
    return _brainService;
  }

  try {
    // Try to load the full BrainService from @cynic/node
    const { getBrainService } = requireCJS('@cynic/node');
    _brainService = getBrainService();
    await _brainService.initialize();
    _brainInitialized = true;
    _useMinimalBrain = false;
    return _brainService;
  } catch (e) {
    // @cynic/node not available - use minimal brain fallback
    try {
      _brainService = createMinimalBrain();
      _brainInitialized = true;
      _useMinimalBrain = true;
      return _brainService;
    } catch (e2) {
      _brainInitError = e2.message;
      _brainInitialized = true;
      return null;
    }
  }
}

/**
 * Get Brain (sync check, async init if needed)
 * @returns {Object|null}
 */
function getBrain() {
  if (_brainInitialized) {
    return _brainService;
  }
  // Return null if not initialized - caller should use async version
  return null;
}

/**
 * Create a minimal Brain using local engines
 * This is used when @cynic/node is not available (e.g., in hook context)
 */
function createMinimalBrain() {
  // Load individual engines we need
  let consciousness = null;
  let fallacyDetector = null;
  let decisionEngine = null;
  let chriaDB = null;

  try { consciousness = requireCJS('../../lib/consciousness.cjs'); } catch (e) { /* optional */ }
  try { fallacyDetector = requireCJS('../../lib/fallacy-detector.cjs'); } catch (e) { /* optional */ }
  try { decisionEngine = requireCJS('../../lib/decision-engine.cjs'); } catch (e) { /* optional */ }
  try { chriaDB = requireCJS('../../lib/chria-database.cjs'); } catch (e) { /* optional */ }

  return {
    /**
     * Think about input - minimal version
     * @param {Object} input
     * @param {Object} options
     * @returns {Object} Thought result
     */
    async think(input, options = {}) {
      const startTime = Date.now();
      const thought = {
        id: `thought-${Date.now()}`,
        timestamp: Date.now(),
        input,
        judgment: null,
        synthesis: null,
        patterns: [],
        confidence: 0,
        decision: null,
        duration: 0,
      };

      try {
        // 1. Check consciousness state
        if (consciousness) {
          const state = consciousness.getState?.() || {};
          thought.patterns.push({
            type: 'consciousness',
            state: state.state || 'unknown',
            confidence: state.confidence || PHI_INV_2,
          });
        }

        // 2. Check for fallacies
        if (fallacyDetector && input.content) {
          const analysis = fallacyDetector.analyze?.(input.content);
          if (analysis?.fallacies?.length > 0) {
            thought.judgment = {
              verdict: 'GROWL',
              reason: `Possible fallacy: ${analysis.fallacies[0].name}`,
              score: 40,
            };
          }
        }

        // 3. Get decision guidance
        if (decisionEngine && input.type === 'decision') {
          const guidance = decisionEngine.analyze?.(input.content);
          if (guidance) {
            thought.synthesis = {
              insight: guidance.recommendation || guidance.insight,
              confidence: guidance.confidence || PHI_INV_2,
            };
          }
        }

        // 4. Get wisdom from chria
        if (chriaDB && !thought.synthesis) {
          const chria = chriaDB.getRandom?.() || chriaDB.getContextual?.(input.content);
          if (chria) {
            thought.synthesis = {
              insight: `"${chria.text}" — ${chria.source}`,
              confidence: PHI_INV_2,
              type: 'chria',
            };
          }
        }

        // 5. Form decision
        if (thought.judgment?.verdict === 'BARK') {
          thought.decision = { action: 'block', reason: thought.judgment.reason, source: 'judgment' };
        } else if (thought.judgment?.verdict === 'GROWL') {
          thought.decision = { action: 'warn', reason: thought.judgment.reason, source: 'judgment' };
        } else {
          thought.decision = { action: 'allow', reason: 'No issues detected', source: 'default' };
        }

        // 6. Calculate confidence
        const scores = [];
        if (thought.judgment) scores.push(thought.judgment.score / 100 || PHI_INV_2);
        if (thought.synthesis) scores.push(thought.synthesis.confidence || PHI_INV_2);
        if (thought.patterns.length > 0) {
          scores.push(thought.patterns.reduce((sum, p) => sum + (p.confidence || 0.5), 0) / thought.patterns.length);
        }
        thought.confidence = scores.length > 0
          ? Math.min(scores.reduce((a, b) => a + b, 0) / scores.length, PHI_INV)
          : PHI_INV_2;

      } catch (e) {
        thought.decision = { action: 'allow', reason: `Brain error: ${e.message}`, source: 'error' };
        thought.confidence = 0;
      }

      thought.duration = Date.now() - startTime;
      return thought;
    },

    /**
     * Quick judgment
     */
    async judge(input) {
      return this.think(input, { requestJudgment: true, requestSynthesis: false });
    },

    /**
     * Deep synthesis
     */
    async synthesize(input) {
      return this.think(input, { requestJudgment: true, requestSynthesis: true });
    },

    /**
     * Get brain state
     */
    getState() {
      return {
        consciousness: PHI_INV,
        cognitiveLoad: 0,
        entropy: 0,
        patterns: [],
        minimal: true, // Flag that this is minimal brain
      };
    },
  };
}

/**
 * Think about a prompt using the Brain
 *
 * This is the main entry point for hooks to use Brain.think()
 *
 * @param {string} content - Content to think about
 * @param {Object} options - Options
 * @param {string} [options.type] - Type of content (code, decision, etc.)
 * @param {Object} [options.context] - Additional context
 * @returns {Promise<Object>} Thought result
 */
export async function thinkAbout(content, options = {}) {
  // Initialize brain service (async)
  const brainService = await initBrainService();

  if (!brainService) {
    // Brain not available - return default response
    return {
      id: `thought-${Date.now()}`,
      confidence: 0,
      decision: { action: 'allow', reason: 'Brain unavailable', source: 'fallback' },
      verdict: null,
      patterns: [],
      error: _brainInitError,
    };
  }

  try {
    const input = {
      content,
      type: options.type || 'general',
      context: options.context || {},
    };

    // Use brainService.think() - works for both full service and minimal brain
    const thought = await brainService.think(input, {
      requestJudgment: options.requestJudgment !== false,
      requestSynthesis: options.requestSynthesis || false,
      checkPatterns: options.checkPatterns !== false,
    });

    return thought;
  } catch (e) {
    return {
      id: `thought-${Date.now()}`,
      confidence: 0,
      decision: { action: 'allow', reason: `Brain error: ${e.message}`, source: 'error' },
      verdict: null,
      patterns: [],
      error: e.message,
    };
  }
}

/**
 * Quick judgment on content
 *
 * @param {string} content
 * @param {Object} [context]
 * @returns {Promise<Object>}
 */
export async function judgeContent(content, context = {}) {
  return thinkAbout(content, {
    type: context.type || 'general',
    context,
    requestJudgment: true,
    requestSynthesis: false,
  });
}

/**
 * Get wisdom/synthesis for content
 *
 * @param {string} content
 * @param {Object} [context]
 * @returns {Promise<Object>}
 */
export async function synthesizeContent(content, context = {}) {
  return thinkAbout(content, {
    type: context.type || 'general',
    context,
    requestJudgment: true,
    requestSynthesis: true,
  });
}

/**
 * Check if Brain is available
 * Note: Returns true if brain CAN be initialized (always true for now)
 * The actual initialization happens on first use
 *
 * @returns {boolean}
 */
export function isBrainAvailable() {
  // Brain is always potentially available - init happens on first use
  // If already initialized, check if it worked
  if (_brainInitialized) {
    return _brainService !== null;
  }
  // Not initialized yet - assume it will work
  return true;
}

/**
 * Get Brain state (for debugging)
 *
 * @returns {Object|null}
 */
export function getBrainState() {
  if (!_brainInitialized || !_brainService) {
    return null;
  }
  // For full brain service, use getStatus()
  if (_brainService.getStatus) {
    return _brainService.getStatus();
  }
  // For minimal brain, use getState()
  return _brainService.getState?.() || null;
}

/**
 * Format thought for injection into prompt
 *
 * Converts a thought result into a formatted string for context injection.
 *
 * @param {Object} thought - Thought from Brain.think()
 * @returns {string|null} Formatted injection or null if no injection needed
 */
export function formatThoughtInjection(thought) {
  if (!thought) return null;

  const lines = [];

  // Add warning/block message if needed
  if (thought.decision?.action === 'block') {
    lines.push(`*GROWL* BRAIN BLOCK: ${thought.decision.reason}`);
  } else if (thought.decision?.action === 'warn') {
    lines.push(`*sniff* BRAIN WARNING: ${thought.decision.reason}`);
  }

  // Add synthesis insight if available
  if (thought.synthesis?.insight) {
    if (thought.synthesis.type === 'chria') {
      lines.push(`── WISDOM ─────────────────────────────────────────────────`);
      lines.push(`   ${thought.synthesis.insight}`);
    } else {
      lines.push(`── BRAIN INSIGHT ──────────────────────────────────────────`);
      lines.push(`   ${thought.synthesis.insight}`);
    }
  }

  // Add patterns if relevant
  if (thought.patterns?.length > 0) {
    const relevantPatterns = thought.patterns.filter(p => p.confidence > PHI_INV_2);
    if (relevantPatterns.length > 0) {
      lines.push(`── PATTERNS ───────────────────────────────────────────────`);
      for (const p of relevantPatterns.slice(0, 3)) {
        lines.push(`   • ${p.name || p.type}: ${p.description || 'detected'}`);
      }
    }
  }

  // Add confidence footer
  if (lines.length > 0 && thought.confidence > 0) {
    const confPct = Math.round(thought.confidence * 100);
    lines.push(`   [Brain confidence: ${confPct}%]`);
  }

  return lines.length > 0 ? lines.join('\n') : null;
}

export default {
  thinkAbout,
  judgeContent,
  synthesizeContent,
  isBrainAvailable,
  getBrainState,
  formatThoughtInjection,
};
