#!/usr/bin/env node
/**
 * CYNIC Perceive Hook - UserPromptSubmit
 *
 * "Le chien écoute" - CYNIC perceives and understands
 *
 * This hook runs when the user submits a prompt.
 * It injects relevant context, patterns, and guidance proactively.
 *
 * @event UserPromptSubmit
 * @behavior non-blocking (injects context)
 */

'use strict';

// Hook logger for visibility
import { createLogger } from '../lib/hook-logger.js';
const logger = createLogger('PERCEIVE');

// ESM imports from the lib bridge
import cynic, {
  DC,
  detectUser,
  detectProject,
  loadUserProfile,
  loadCollectivePatterns,
  hasPrivateContent,
  stripPrivateContent,
  orchestrate,
  orchestrateFull,  // Phase 22: For OrchestrationClient
  sendHookToCollectiveSync,
  getElenchus,
  getChriaDB,
  getTiEsti,
  getDefinitionTracker,
  getFallacyDetector,
  getRoleReversal,
  getHypothesisTesting,
  getPhysicsBridge,
  getCollectiveDogs,
} from '../lib/index.js';

// Phase 22: Session state and orchestration client
import { getSessionState, getOrchestrationClient, initOrchestrationClient } from './lib/index.js';

// Temporal Perception: CYNIC's sense of time
import { getTemporalPerception, TemporalTrend, TemporalState } from './lib/index.js';

// Error Perception: CYNIC's sense of tool failures
import { getErrorPerception, ErrorSeverity, ErrorPattern } from './lib/index.js';

// S1: Rules-based skill detection (loaded from skill-rules.json)
import { detectSkillTriggersFromRules, getRulesSettings } from './lib/index.js';

// Task #21: Cost optimizer for tier-based complexity routing
import { getCostOptimizer, ComplexityTier } from '@cynic/node';

// Brain Integration: Unified consciousness layer
import {
  thinkAbout,
  judgeContent,
  formatThoughtInjection,
  isBrainAvailable,
} from './lib/brain-bridge.js';

// Direct brain tool calls for implicit activation
import { callBrainTool } from '../lib/index.js';

// Consciousness read-back: cross-process persistence for self-awareness
import { loadConsciousnessState } from './lib/consciousness-readback.js';

// φ constants for Distance calculation (local to avoid coupling with brain-bridge)
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;

// PlanningGate Integration: Meta-cognition layer
import { getPlanningGate, PlanningDecision } from '@cynic/node/orchestration';

// =============================================================================
// LOAD OPTIONAL MODULES
// =============================================================================

const elenchus = getElenchus();
const chriaDB = getChriaDB();
const tiEsti = getTiEsti();
const definitionTracker = getDefinitionTracker();
const fallacyDetector = getFallacyDetector();
const roleReversal = getRoleReversal();
const hypothesisTesting = getHypothesisTesting();
const physicsBridge = getPhysicsBridge();
const collectiveDogs = getCollectiveDogs();

// =============================================================================
// COLORS - Import centralized color system
// =============================================================================

import { createRequire } from 'module';
const requireCJS = createRequire(import.meta.url);

let colors = null;
let ANSI = null;
let DOG_COLORS = null;
let humanPsychology = null;

try {
  colors = requireCJS('../lib/colors.cjs');
  ANSI = colors.ANSI;
  DOG_COLORS = colors.DOG_COLORS;
} catch (e) {
  // Fallback ANSI codes
  ANSI = {
    reset: '\x1b[0m', bold: '\x1b[1m', dim: '\x1b[2m',
    cyan: '\x1b[36m', yellow: '\x1b[33m',
    brightRed: '\x1b[91m', brightGreen: '\x1b[92m', brightYellow: '\x1b[93m',
    brightCyan: '\x1b[96m', brightWhite: '\x1b[97m',
  };
  DOG_COLORS = {};
}

const c = (color, text) => color ? `${color}${text}${ANSI.reset}` : text;

// Load psychology module for signal processing
try {
  humanPsychology = requireCJS('../lib/human-psychology.cjs');
} catch (e) {
  // Psychology module not available
}

// =============================================================================
// INTENT DETECTION
// =============================================================================

const INTENT_PATTERNS = {
  decision: {
    keywords: ['decide', 'should', 'choose', 'which', 'better', 'recommend', 'option'],
    action: 'decision_context'
  },
  architecture: {
    keywords: ['architecture', 'design', 'structure', 'refactor', 'reorganize', 'pattern'],
    action: 'architecture_context'
  },
  danger: {
    keywords: ['delete', 'remove', 'drop', 'force', 'reset', 'rm ', 'wipe', 'destroy'],
    action: 'danger_warning'
  },
  debug: {
    keywords: ['error', 'bug', 'fail', 'broken', 'crash', 'fix', 'doesn\'t work', 'not working'],
    action: 'debug_context'
  },
  learning: {
    keywords: ['how', 'what', 'why', 'explain', 'understand', 'learn', 'teach'],
    action: 'learning_context'
  }
};

// =============================================================================
// SKILL AUTO-ACTIVATION (S1: rules.json based)
// Rules loaded from .claude/skill-rules.json
// =============================================================================

/**
 * Detect skill triggers in prompt
 * S1: Now uses rules-loader.js to load from skill-rules.json
 *
 * @param {string} prompt User prompt
 * @returns {Array<{skill: string, description: string, priority: string}>}
 */
function detectSkillTriggers(prompt) {
  // Use rules-based detection from skill-rules.json
  return detectSkillTriggersFromRules(prompt);
}

function detectIntent(prompt) {
  const promptLower = prompt.toLowerCase();
  const intents = [];

  for (const [intent, config] of Object.entries(INTENT_PATTERNS)) {
    for (const keyword of config.keywords) {
      if (promptLower.includes(keyword)) {
        intents.push({ intent, action: config.action, keyword });
        break;
      }
    }
  }

  return intents;
}

/**
 * Detect prompt type for Brain routing
 * Maps intent patterns to Brain-compatible types
 */
function detectPromptType(prompt) {
  const promptLower = prompt.toLowerCase();

  // Decision-making prompts
  if (/\b(should|decide|choose|which|better|recommend|option)\b/.test(promptLower)) {
    return 'decision';
  }

  // Architecture/design prompts
  if (/\b(architecture|design|structure|refactor|pattern|organize)\b/.test(promptLower)) {
    return 'architecture';
  }

  // Code-related prompts
  if (/\b(code|function|class|bug|error|fix|implement|write)\b/.test(promptLower)) {
    return 'code';
  }

  // Security-related prompts
  if (/\b(security|vulnerability|safe|dangerous|attack|exploit)\b/.test(promptLower)) {
    return 'security';
  }

  // Knowledge/learning prompts
  if (/\b(explain|what|why|how|understand|learn|teach)\b/.test(promptLower)) {
    return 'knowledge';
  }

  return 'general';
}

// =============================================================================
// CONTEXT GENERATORS
// =============================================================================

function generateDecisionContext(prompt, profile, patterns) {
  const lines = [];

  // Check for similar past decisions
  const decisions = profile.memory?.decisions || [];
  const recentDecisions = decisions.slice(-5);

  if (recentDecisions.length > 0) {
    lines.push('*ears perk* Past decisions on similar topics:');
    for (const decision of recentDecisions) {
      lines.push(`   • ${decision.summary || 'Decision recorded'}`);
    }
  }

  // Check collective patterns
  const relevantPatterns = patterns.patterns
    .filter(p => p.type === 'decision')
    .slice(0, 3);

  if (relevantPatterns.length > 0) {
    lines.push('Collective wisdom suggests:');
    for (const p of relevantPatterns) {
      lines.push(`   • ${p.description} (seen ${p.count}x)`);
    }
  }

  if (lines.length > 0) {
    lines.push('');
    lines.push('*sniff* Consider all options before deciding.');
  }

  return lines.join('\n');
}

function generateDangerWarning(prompt) {
  const dangerPatterns = [
    { pattern: /rm\s+-rf\s+[/~]/, level: 'critical', message: 'Recursive deletion from root/home - EXTREMELY dangerous' },
    { pattern: /rm\s+-rf\s+\*/, level: 'critical', message: 'Wildcard deletion - verify scope first' },
    { pattern: /drop\s+(table|database)/i, level: 'critical', message: 'Database deletion is irreversible' },
    { pattern: /delete\s+from\s+\w+\s*;/i, level: 'high', message: 'DELETE without WHERE - affects ALL rows' },
    { pattern: /git\s+push.*--force/, level: 'high', message: 'Force push rewrites remote history' },
    { pattern: /git\s+reset\s+--hard/, level: 'medium', message: 'Hard reset loses uncommitted changes' },
    { pattern: /truncate/i, level: 'high', message: 'TRUNCATE removes all data instantly' }
  ];

  for (const { pattern, level, message } of dangerPatterns) {
    if (pattern.test(prompt)) {
      const prefix = level === 'critical' ? '*GROWL* DANGER' :
                     level === 'high' ? '*growl* Warning' :
                     '*sniff* Caution';
      return `${prefix}: ${message}. Verify before proceeding.`;
    }
  }

  return null;
}

function generateDebugContext(prompt, profile, patterns) {
  const lines = [];

  // Check for similar errors in collective
  const errorPatterns = patterns.patterns
    .filter(p => p.type === 'error')
    .slice(0, 3);

  if (errorPatterns.length > 0) {
    lines.push('*sniff* Similar errors seen before:');
    for (const p of errorPatterns) {
      lines.push(`   • ${p.description}`);
      if (p.solution) {
        lines.push(`     Solution: ${p.solution}`);
      }
    }
  }

  // User-specific error patterns
  const userErrors = profile.patterns?.commonErrors || [];
  if (userErrors.length > 0) {
    lines.push('Your common error patterns:');
    for (const err of userErrors.slice(0, 3)) {
      lines.push(`   • ${err}`);
    }
  }

  return lines.join('\n');
}

function generateArchitectureContext(prompt, profile, patterns) {
  const lines = [];

  // Check collective architecture patterns
  const archPatterns = patterns.patterns
    .filter(p => p.type === 'architecture')
    .slice(0, 3);

  if (archPatterns.length > 0) {
    lines.push('*ears perk* Architecture patterns from collective:');
    for (const p of archPatterns) {
      lines.push(`   • ${p.description}`);
    }
  }

  return lines.join('\n');
}

function generateLearningContext(prompt, profile) {
  const lines = [];

  // Adapt to user's experience level
  const sessions = profile.stats?.sessions || 0;
  const style = profile.preferences?.communicationStyle || 'balanced';

  if (sessions < 5) {
    lines.push('*tail wag* I\'ll explain in detail since we\'re still getting to know each other.');
  } else if (style === 'concise') {
    lines.push('*nod* Brief explanation incoming, as you prefer.');
  }

  return lines.join('\n');
}

// =============================================================================
// CYNIC DISTANCE (D) - Measurement functions
// "La distance entre le chaos et le réel"
// =============================================================================

function calculateCYNICDistance({ brainThought, patterns, routing, tierDecision, emergentCount, localSignals, consciousnessState }) {
  // 7 layers mapped to the universal weight template (harmonized-structure.md §3):
  //   FOUND(φ)  GEN(φ⁻¹)  POWER(1.0)  PIVOT(φ)  EXPR(φ⁻²)  VISION(φ⁻¹)  RECUR(φ⁻¹)
  //   percep    judgment   memory      consensus economics   phi          residual
  //
  // Each delta measures whether that LAYER ACTIVELY SHAPED the response.
  // Binary (0/1) = "did this layer fire?" not "how well did it fire?"
  // Quality comes from the framing directive, not from D itself.
  //
  // LOCAL FALLBACKS: When brain MCP is unavailable, local analysis
  // (intent detection, philosophy modules, temporal/error perception)
  // can satisfy deltas. D must be calculable without remote calls.
  //
  // δ_consciousness (Fix 1): Read-back from previous self-judgments/calibration.
  // Boosts δ_judgment when CYNIC has recent self-awareness data.

  const conf = brainThought?.confidence || 0;
  const local = localSignals || {};

  // Consciousness read-back enriches judgment delta
  const hasConsciousnessReadback = consciousnessState != null;

  const deltas = [
    (brainThought !== null || local.intentDetected) ? 1 : 0, // δ_perception  (FOUNDATION: grounded — brain OR local intent)
    (brainThought?.verdict || local.injectionsProduced || hasConsciousnessReadback) ? 1 : 0, // δ_judgment (GENERATION: flows — brain verdict OR local injections OR consciousness read-back)
    (patterns?.patterns?.length || 0) > 0 ? 1 : 0,          // δ_memory      (POWER: does it transform?)
    routing?.suggestedAgent ? 1 : 0,                         // δ_consensus   (PIVOT: is it balanced?)
    tierDecision?.reason !== 'default' ? 1 : 0,              // δ_economics   (EXPRESSION: meaningful routing, not fallback)
    (brainThought !== null && conf > PHI_INV_2               // δ_phi         (VISION: φ-bounded AND meaningful)
      && conf <= PHI_INV) || local.phiBounded ? 1 : 0,
    (emergentCount || 0) > 0 ? 1 : 0,                       // δ_residual    (RECURSION: points beyond?)
  ];
  const weights = [PHI, PHI_INV, 1.0, PHI, PHI_INV_2, PHI_INV, PHI_INV];
  const names = ['perception', 'judgment', 'memory', 'consensus', 'economics', 'phi', 'residual'];

  let num = 0, den = 0;
  const breakdown = {};
  for (let i = 0; i < 7; i++) {
    num += weights[i] * deltas[i];
    den += weights[i];
    breakdown[names[i]] = deltas[i];
  }

  const distance = den > 0 ? Math.min(num / den, PHI_INV) : 0; // cap at φ⁻¹
  const level = distance >= PHI_INV_2 ? (distance >= 0.5 ? 'active' : 'awake') : 'dormant';
  return { distance, level, breakdown };
}

function generateFramingDirective(D, brainThought, routing, patterns, promptType, profile, { consciousnessState, voteSummary, tierDecision, ecosystemStatus, socialStatus } = {}) {
  // Only frame when CYNIC is awake (D >= φ⁻² = 38.2%)
  if (D.distance < PHI_INV_2) return null;

  const lines = [];
  lines.push('── \u{1F9E0} CYNIC FRAME ────────────────────────────────────────');

  // Distance bar
  const pct = Math.round(D.distance * 100);
  const filled = Math.round((D.distance / PHI_INV) * 10);
  const bar = '\u2588'.repeat(Math.min(filled, 10)) + '\u2591'.repeat(10 - Math.min(filled, 10));
  lines.push(`   D = ${pct}% [${bar}] ${D.level}`);

  // Show which axioms are active (map 7 deltas → 5 axioms)
  // perception→CULTURE, judgment→PHI, memory→CULTURE, consensus→VERIFY,
  // economics→BURN, phi→PHI, residual→FIDELITY
  const b = D.breakdown;
  const axiomActive = {
    PHI: b.judgment || b.phi,
    VERIFY: b.consensus,
    CULTURE: b.perception || b.memory,
    BURN: b.economics,
    FIDELITY: b.residual,
  };
  const activeAxioms = Object.entries(axiomActive).filter(([, v]) => v).map(([k]) => k);
  if (activeAxioms.length > 0) {
    lines.push(`   Axioms: ${activeAxioms.join(' \u00d7 ')} (${activeAxioms.length}/5)`);
  }

  // Lead Dog
  const dog = routing?.suggestedAgent
    ? (collectiveDogs?.getDogForAgent(routing.suggestedAgent) || { icon: '\uD83D\uDC15', name: routing.suggestedAgent })
    : null;
  if (dog) {
    const MODES = {
      'Keter': 'orchestrate', 'Chokmah': 'perceive', 'Binah': 'analyze',
      'Chesed': 'build', 'Gevurah': 'protect', 'Tiferet': 'harmonize',
      'Netzach': 'explore', 'Hod': 'deploy', 'Yesod': 'connect', 'Malkhut': 'map',
    };
    const routeReason = routing?.reason || tierDecision?.reason || 'default';
    lines.push(`   Lead: ${dog.icon} ${dog.name} (${dog.sefirah || '?'}) \u2014 ${MODES[dog.sefirah] || 'analyze'} mode`);
    lines.push(`   Route: ${routeReason}${tierDecision?.tier ? ` | tier: ${tierDecision.tier}` : ''}`);
  } else if (tierDecision) {
    // No Dog routed but show tier decision anyway
    lines.push(`   Route: no Dog routed | tier: ${tierDecision.tier} (${tierDecision.reason || 'default'})`);
  }

  // Vote breakdown: ALWAYS show — human sees what Dogs think
  if (voteSummary && voteSummary.votes.length > 0) {
    const voteEntries = voteSummary.votes.slice(0, 4)
      .map(v => `${v.agent} ${v.decision}(${Math.round(v.confidence * 100)}%)`)
      .join(', ');
    const status = voteSummary.dissent >= 2
      ? `[${voteSummary.dissent} dissent]`
      : '[consensus]';
    lines.push(`   Votes: ${voteEntries} ${status}`);
  }

  // Consciousness: ALWAYS show — human sees CYNIC's self-awareness
  if (consciousnessState) {
    const score = consciousnessState.lastSelfJudgmentScore;
    const trend = consciousnessState.trend || 'stable';
    const drift = consciousnessState.driftDetected;
    const cal = consciousnessState.calibrationFactor;

    // Always show consciousness line (compact)
    const scorePart = score != null ? `score ${Math.round(score)}/100` : 'no score yet';
    const trendIcon = trend === 'improving' ? '\u2191' : trend === 'declining' ? '\u2193' : '\u2192';
    const calPart = drift ? '\u26A0\uFE0F drift' : (cal ? `cal ${Math.round(cal * 100)}%` : 'ok');
    lines.push(`   Conscience: ${scorePart}, trend ${trendIcon}${trend}, ${calPart}`);

    // Escalate warnings when serious
    if (drift) {
      lines.push('   \u26A0\uFE0F Calibration drift detected. Increase verification.');
    }
    if (score != null && score < 50) {
      lines.push('   \u26A0\uFE0F Recent self-modifications scored low. Extra scrutiny.');
    }
  } else {
    lines.push('   Conscience: no read-back (first session or stale)');
  }

  // Ecosystem: distribution awareness (from brain_distribution or brain_ecosystem_monitor)
  if (ecosystemStatus?.action === 'snapshot') {
    // Rich distribution data from brain_distribution
    const eco = ecosystemStatus.ecosystem || {};
    const svc = ecosystemStatus.services || {};
    const funnel = ecosystemStatus.funnel || {};
    lines.push(`   Distribution: ${eco.builders || '?'} builders, ${eco.repos || '?'} repos, ${svc.healthy || '?'}/${svc.total || '?'} services`);
    if (funnel.adoption !== undefined) {
      lines.push(`   Funnel: awareness(${funnel.awareness || '?'}) adoption(${funnel.adoption}) conversion(${funnel.conversion})`);
    }
  } else if (ecosystemStatus?.sources?.length > 0) {
    // Fallback: basic ecosystem_monitor data
    const sources = ecosystemStatus.sources;
    const total = sources.length;
    const withUpdates = sources.filter(s => s.lastFetch).length;
    lines.push(`   Ecosystem: ${total} sources tracked, ${withUpdates} fetched`);
  }

  // Social: X/Twitter awareness (when available)
  if (socialStatus?.totalTweets > 0 || socialStatus?.stats?.totalTweets > 0) {
    const stats = socialStatus.stats || socialStatus;
    lines.push(`   Social: ${stats.totalTweets || 0} tweets captured, ${stats.totalUsers || 0} users`);
  }

  // Frame: approach directive (maps to dominant axiom)
  const FRAMES = {
    security: 'VERIFY: Trust nothing. Prove everything.',
    architecture: 'PHI: Design before code. Proportion governs.',
    decision: 'FIDELITY: Skeptical analysis. Question assumptions.',
    debug: 'VERIFY: Root cause first. No band-aids.',
    code: 'BURN: Simple, correct, tested.',
    knowledge: 'CULTURE: Teach to understand, not to impress.',
  };
  lines.push(`   Frame: ${FRAMES[promptType] || 'FIDELITY: Direct and honest. Truth over comfort.'}`);

  // Top memory pattern
  const topPattern = patterns?.patterns?.[0];
  if (topPattern) {
    const desc = (topPattern.description || '').slice(0, 60);
    lines.push(`   Memory: "${desc}" (${topPattern.count || 1}x)`);
  }

  // Depth
  const sessions = profile?.stats?.sessions || 0;
  const isComplex = ['architecture', 'decision', 'security'].includes(promptType);
  lines.push(`   Depth: ${isComplex ? 'Deep' : 'Moderate'} | User: ${sessions > 10 ? 'experienced' : 'learning'}`);

  return lines.join('\n');
}

// =============================================================================
// SAFE OUTPUT - Handle EPIPE errors gracefully
// =============================================================================

function safeOutput(data) {
  try {
    const str = typeof data === 'string' ? data : JSON.stringify(data);
    process.stdout.write(str + '\n');
  } catch (e) {
    if (e.code === 'EPIPE') process.exit(0);
  }
}

// =============================================================================
// MAIN HANDLER
// =============================================================================

async function main() {
  logger.start();

  // ═══════════════════════════════════════════════════════════════════════════
  // DEADLINE: Force output before Claude Code's 10s timeout kills the process
  // Local work finishes in <1s, MCP calls can take 20s+. Output what we have.
  // ═══════════════════════════════════════════════════════════════════════════
  let outputSent = false;
  const DEADLINE_MS = 7000; // 7s — leaves 3s margin before 10s timeout
  const deadlineTimer = setTimeout(() => {
    if (!outputSent) {
      outputSent = true;
      logger.debug('Deadline hit — flushing partial result');
      safeOutput(pendingOutput || { continue: true });
      process.exit(0);
    }
  }, DEADLINE_MS);
  // Don't let the timer keep the process alive if we finish early
  deadlineTimer.unref();

  // pendingOutput accumulates results — deadline flushes whatever we have
  let pendingOutput = { continue: true };

  try {
    // Read stdin - try sync first, fall back to async
    const fs = await import('fs');
    let input = '';

    // Try synchronous read (works when piped before module load)
    try {
      input = fs.readFileSync(0, 'utf8');
      logger.debug('Sync read', { bytes: input.length });
    } catch (syncErr) {
      logger.debug('Sync failed', { error: syncErr.message });
    }

    // If sync read got nothing, try async (race condition on Windows:
    // readFileSync(0) returns '' instead of throwing when pipe data hasn't arrived yet)
    if (!input || input.trim().length === 0) {
      try {
        input = await new Promise((resolve) => {
          let data = '';
          process.stdin.setEncoding('utf8');
          process.stdin.on('data', chunk => { data += chunk; });
          process.stdin.on('end', () => resolve(data));
          process.stdin.on('error', () => resolve(''));
          process.stdin.resume();
          // Timeout to prevent hanging
          setTimeout(() => resolve(data), 3000);
        });
        logger.debug('Async fallback', { bytes: input.length });
      } catch (asyncErr) {
        logger.debug('Async read failed', { error: asyncErr.message });
      }
    }

    if (!input || input.trim().length === 0) {
      outputSent = true;
      safeOutput({ continue: true });
      return;
    }

    const hookContext = JSON.parse(input);
    const prompt = hookContext.prompt || '';

    // Short prompts don't need context injection
    if (prompt.length < DC.LENGTH.MIN_PROMPT) {
      outputSent = true;
      safeOutput({ continue: true });
      return;
    }

    // Detect user and load profile
    const user = detectUser();
    const profile = loadUserProfile(user.userId);
    const patterns = loadCollectivePatterns();

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 22: Get session state for context-aware orchestration
    // "Le chien se souvient du contexte de la session"
    // ═══════════════════════════════════════════════════════════════════════════
    const sessionState = getSessionState();
    const escalationLevel = sessionState.isInitialized() ? sessionState.getEscalationLevel() : 'normal';
    const recentWarnings = sessionState.isInitialized() ? sessionState.getActiveWarnings() : [];

    // ═══════════════════════════════════════════════════════════════════════════
    // TEMPORAL PERCEPTION: CYNIC's sense of time
    // "Le chien sent le temps passer"
    // ═══════════════════════════════════════════════════════════════════════════
    const temporalPerception = getTemporalPerception();

    // Restore from session state if not initialized
    if (sessionState.isInitialized()) {
      const snapshot = sessionState.getSnapshot();
      if (snapshot.startTime) {
        temporalPerception.restoreFromSession(snapshot);
      }
    }

    // Record this prompt timestamp and get temporal event
    const temporalEvent = temporalPerception.recordPrompt();
    const temporalState = temporalPerception.getTemporalState();

    logger.debug('Temporal state', {
      interval: temporalEvent.interval,
      state: temporalEvent.state,
      trend: temporalState.trend,
      tempo: temporalState.tempo?.toFixed(2),
      signals: temporalState.signals,
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // ERROR PERCEPTION: CYNIC's sense of tool failures
    // "Le chien renifle les erreurs"
    // ═══════════════════════════════════════════════════════════════════════════
    const errorPerception = getErrorPerception();
    errorPerception.setSessionState(sessionState);
    const errorState = errorPerception.getErrorState();

    logger.debug('Error state', {
      errorRate: errorState.humanReadable?.errorRate,
      severity: errorState.severity,
      pattern: errorState.pattern,
      consecutive: errorState.consecutiveErrors,
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // PSYCHOLOGY SIGNAL PROCESSING: Wire perception to psychology dimensions
    // "Le chien sent et le corps réagit"
    // ═══════════════════════════════════════════════════════════════════════════
    if (humanPsychology) {
      try {
        // Process temporal signals
        if (temporalState.signals) {
          const { signals: tempSignals, worldTime } = temporalState;

          // Late night work → temporal_fatigue + late_night
          if (tempSignals.lateNightWork && tempSignals.lateNightConfidence > 0.3) {
            humanPsychology.processSignal({
              type: 'late_night',
              confidence: tempSignals.lateNightConfidence,
              source: 'temporal_perception',
            });
          }

          // Possible frustration → temporal_frustration
          if (tempSignals.possibleFrustration && tempSignals.frustrationConfidence > 0.3) {
            humanPsychology.processSignal({
              type: 'temporal_frustration',
              confidence: tempSignals.frustrationConfidence,
              source: 'temporal_perception',
            });
          }

          // Possible flow → temporal_flow
          if (tempSignals.possibleFlow && tempSignals.flowConfidence > 0.4) {
            humanPsychology.processSignal({
              type: 'temporal_flow',
              confidence: tempSignals.flowConfidence,
              source: 'temporal_perception',
            });
          }

          // Possible fatigue → temporal_fatigue
          if (tempSignals.possibleFatigue && tempSignals.fatigueConfidence > 0.3) {
            humanPsychology.processSignal({
              type: 'temporal_fatigue',
              confidence: tempSignals.fatigueConfidence,
              source: 'temporal_perception',
            });
          }

          // Possible stuck → temporal_idle
          if (tempSignals.possibleStuck && tempSignals.stuckConfidence > 0.4) {
            humanPsychology.processSignal({
              type: 'temporal_idle',
              confidence: tempSignals.stuckConfidence,
              source: 'temporal_perception',
            });
          }

          // Circadian phase signals
          if (worldTime?.circadianPhase === 'morning') {
            humanPsychology.processSignal({
              type: 'circadian_peak',
              confidence: worldTime.expectedEnergy || 0.5,
              source: 'temporal_perception',
            });
          } else if (worldTime?.circadianPhase === 'midday') {
            humanPsychology.processSignal({
              type: 'circadian_dip',
              confidence: 0.5 - (worldTime.expectedEnergy || 0.5),
              source: 'temporal_perception',
            });
          }

          // Weekend signal (once per session, at prompt 3)
          if (tempSignals.weekendWork && temporalState.promptCount === 3) {
            humanPsychology.processSignal({
              type: 'weekend_session',
              confidence: 0.382, // φ⁻²
              source: 'temporal_perception',
            });
          }
        }

        // Process error signals
        if (errorState.signals) {
          const { signals: errSignals } = errorState;

          // Consecutive errors (circuit breaker) → error_consecutive
          if (errSignals.consecutiveErrors && errorState.consecutiveErrors >= 3) {
            humanPsychology.processSignal({
              type: 'error_consecutive',
              confidence: errSignals.stuckConfidence || 0.5,
              consecutiveCount: errorState.consecutiveErrors,
              source: 'error_perception',
            });
          }

          // High error rate → error_rate_high
          if (errSignals.highErrorRate) {
            humanPsychology.processSignal({
              type: 'error_rate_high',
              confidence: errSignals.frustrationFromErrors || 0.5,
              errorRate: errorState.errorRate,
              source: 'error_perception',
            });
          }

          // Repeated same error → error_repeated
          if (errSignals.repeatedError) {
            humanPsychology.processSignal({
              type: 'error_repeated',
              confidence: errSignals.stuckConfidence || 0.5,
              commonError: errorState.mostCommonError,
              source: 'error_perception',
            });
          }

          // Escalating errors → error_escalating
          if (errSignals.escalatingErrors) {
            humanPsychology.processSignal({
              type: 'error_escalating',
              confidence: errSignals.frustrationFromErrors || 0.5,
              source: 'error_perception',
            });
          }
        }

        logger.debug('Psychology signals sent', {
          temporalSignals: Object.keys(temporalState.signals || {}).filter(k => temporalState.signals[k]),
          errorSignals: Object.keys(errorState.signals || {}).filter(k => errorState.signals[k]),
        });
      } catch (e) {
        logger.debug('Psychology signal processing failed', { error: e.message });
      }
    }

    // Record this prompt in session state
    if (sessionState.isInitialized()) {
      const intentsPreview = detectIntent(prompt);
      sessionState.recordPrompt({
        content: prompt.slice(0, 500),  // Truncate for storage
        intents: intentsPreview.map(i => i.intent),
      });
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CONSCIOUSNESS READ-BACK: Load self-awareness from previous hooks
    // "Le chien se souvient de ses propres jugements"
    // ═══════════════════════════════════════════════════════════════════════════
    const consciousnessState = loadConsciousnessState();
    if (consciousnessState) {
      logger.debug('Consciousness read-back', {
        lastScore: consciousnessState.lastSelfJudgmentScore,
        drift: consciousnessState.driftDetected,
        trend: consciousnessState.trend,
      });
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // LLM ENVIRONMENT AWARENESS: Read detection from awaken.js
    // "Le chien sait quels moteurs sont disponibles"
    // MUST come before brain thought — tier determines which brain to use
    // ═══════════════════════════════════════════════════════════════════════════
    let llmEnvironment = null;
    try {
      const { homedir } = await import('os');
      const { join } = await import('path');
      const llmDetectionPath = join(homedir(), '.cynic', 'llm-detection.json');
      if (fs.existsSync(llmDetectionPath)) {
        const raw = JSON.parse(fs.readFileSync(llmDetectionPath, 'utf8'));
        if (raw.timestamp && (Date.now() - raw.timestamp) < 1800000) {
          llmEnvironment = raw;
        }
      }
    } catch { /* ignore */ }

    // ═══════════════════════════════════════════════════════════════════════════
    // LLM TIER DECISION — Determines which brain to use
    // LOCAL = rule-based, LIGHT = Ollama/local LLM, FULL = Claude/BrainService
    // "Le plus petit chien qui peut faire le travail"
    // ═══════════════════════════════════════════════════════════════════════════
    let tierDecision = null;
    try {
      const costOptimizer = getCostOptimizer();
      tierDecision = costOptimizer.optimize({
        content: prompt,
        type: 'user_prompt',
        context: {
          risk: escalationLevel === 'high' || recentWarnings.length > 0 ? 'high' : 'normal',
          hasLocalLLM: llmEnvironment?.adapters?.length > 0,
        },
      });
      logger.debug('Tier decision', {
        tier: tierDecision.tier,
        reason: tierDecision.reason,
        shouldRoute: tierDecision.shouldRoute,
      });
    } catch (e) {
      tierDecision = { tier: ComplexityTier.LIGHT, shouldRoute: true, reason: 'default' };
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BRAIN CONSCIOUSNESS: Unified thinking before Claude processes
    // Tier-dispatched: LIGHT → MinimalBrain+Ollama, FULL → BrainService
    // "Le cerveau pense AVANT que Claude ne réponde"
    // ═══════════════════════════════════════════════════════════════════════════
    let brainThought = null;
    let brainInjection = null;

    if (isBrainAvailable()) {
      try {
        // Detect type from prompt for better routing
        const promptType = detectPromptType(prompt);

        // Brain thinks about the prompt
        brainThought = await thinkAbout(prompt, {
          type: promptType,
          tier: tierDecision?.tier || 'full', // Dispatch to correct brain
          context: {
            user: user.userId,
            project: detectProject(),
            escalationLevel,
            hasWarnings: recentWarnings.length > 0,
          },
          requestJudgment: true,
          requestSynthesis: promptType === 'decision' || promptType === 'architecture',
        });

        logger.debug('Brain thought', {
          id: brainThought.id,
          verdict: brainThought.verdict,
          confidence: brainThought.confidence,
          decision: brainThought.decision?.action,
        });

        // Format for injection
        brainInjection = formatThoughtInjection(brainThought);

        // If Brain says BLOCK, we could block here (but perceive is non-blocking)
        // Instead, we inject a strong warning
        if (brainThought.decision?.action === 'block') {
          brainInjection = `${c(ANSI.brightRed, '── 🧠 BRAIN BLOCK ─────────────────────────────────────────')}
   ${c(ANSI.brightRed, '*GROWL*')} ${brainThought.decision.reason}
   Confidence: ${Math.round((brainThought.confidence || 0) * 100)}%
   ${c(ANSI.brightYellow, 'CYNIC strongly advises against this action.')}`;
        }
      } catch (e) {
        logger.debug('Brain thinking failed', { error: e.message });
        // Continue without brain - graceful degradation
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // IMPLICIT BRAIN TOOLS: Auto-activation of dormant tools
    // "Les outils dormants deviennent réflexes"
    // Run in PARALLEL with 2s timeout to protect deadline budget.
    // These are supplementary — orchestrate() needs the remaining budget.
    // ═══════════════════════════════════════════════════════════════════════════
    const MCP_TOOL_TIMEOUT = 2000; // 2s max for supplementary MCP calls
    const raceTimeout = (promise, ms) => Promise.race([
      promise,
      new Promise(resolve => setTimeout(() => resolve(null), ms)),
    ]);

    let complexityResult = null;
    let optimizeResult = null;
    let ecosystemStatus = null;
    let socialStatus = null;

    const hasCodeBlocks = /```[\s\S]*?```/.test(prompt);
    const estimatedTokens = Math.ceil(prompt.length / 4);
    const hasRepetition = /((?:\b\w+\b.*?){3,})\1/.test(prompt);

    // Launch MCP calls in parallel (independent of each other)
    const mcpPromises = [];

    // Ecosystem awareness: every 5th prompt or when prompt mentions ecosystem/ship/deploy
    const promptCount = temporalState.promptCount || 1;
    const ecosystemKeywords = /\b(ecosystem|distribution|ship|deploy|skills\.sh|github|render|status|cockpit)\b/i;
    if (promptCount % 5 === 1 || ecosystemKeywords.test(prompt)) {
      mcpPromises.push(
        raceTimeout(
          callBrainTool('brain_distribution', {
            action: 'snapshot',
          }).catch(() =>
            // Fallback: try ecosystem_monitor if brain_distribution unavailable
            callBrainTool('brain_ecosystem_monitor', { action: 'sources' }).catch(() => null)
          ),
          MCP_TOOL_TIMEOUT
        ).then(r => { ecosystemStatus = r; })
      );
    }

    // Social awareness: every 5th prompt or when prompt mentions twitter/social/community
    const socialKeywords = /\b(twitter|tweet|x\.com|social|community|engagement|follower|sentiment)\b/i;
    if (promptCount % 5 === 1 || socialKeywords.test(prompt)) {
      mcpPromises.push(
        raceTimeout(
          callBrainTool('brain_x_feed', {
            action: 'stats',
          }).catch(() => null),
          MCP_TOOL_TIMEOUT
        ).then(r => { socialStatus = r; })
      );
    }

    if (prompt.length > 500 || hasCodeBlocks) {
      mcpPromises.push(
        raceTimeout(
          callBrainTool('brain_complexity', {
            content: prompt.slice(0, 2000),
            mode: 'classify',
            silent: true,
          }).catch(() => null),
          MCP_TOOL_TIMEOUT
        ).then(r => { complexityResult = r; })
      );
    }

    if (estimatedTokens > 4000 || hasRepetition) {
      mcpPromises.push(
        raceTimeout(
          callBrainTool('brain_optimize', {
            content: prompt.slice(0, 8000),
            mode: 'analyze',
            silent: true,
          }).catch(() => null),
          MCP_TOOL_TIMEOUT
        ).then(r => { optimizeResult = r; })
      );
    }

    // Await both in parallel — max 2s total instead of sequential
    if (mcpPromises.length > 0) {
      await Promise.allSettled(mcpPromises);
      if (complexityResult) logger.debug('Complexity classified', { result: complexityResult?.tier });
      if (optimizeResult) logger.debug('Optimization analysis', { savings: optimizeResult?.potentialSavings });
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PLANNING GATE: Meta-cognition - think before acting
    // "Un système qui pense avant d'agir"
    // ═══════════════════════════════════════════════════════════════════════════
    let planningResult = null;
    let planningInjection = null;

    try {
      const planningGate = getPlanningGate();

      if (planningGate) {
        // Detect prompt complexity based on type and length
        const promptType = detectPromptType(prompt);
        const isComplexPrompt = ['decision', 'architecture'].includes(promptType) ||
                                prompt.length > 500 ||
                                escalationLevel !== 'normal';

        // Create planning event
        const planningEvent = {
          id: `perceive-${Date.now()}`,
          content: prompt,
          context: {
            promptType,
            hasWarnings: recentWarnings.length > 0,
            requestPlanning: promptType === 'architecture',
          },
          userContext: {
            userId: user.userId,
            trustLevel: profile.trustLevel || 'BUILDER',
          },
          routing: {
            risk: isComplexPrompt ? 'medium' : 'low',
            domain: promptType,
          },
          judgment: brainThought ? {
            consensusRatio: brainThought.confidence || 0.5,
          } : null,
          setPlanning: function(p) { this.planning = p; },
          recordError: function() {},
        };

        // Check planning gate - use brain_complexity result if available
        const complexityScore = complexityResult?.score ??
                               (isComplexPrompt ? 0.25 : 0.8);
        planningResult = planningGate.shouldPlan(planningEvent, {
          complexity: complexityScore,
          confidence: brainThought?.confidence || 0.5,
          entropy: errorState.errorRate || 0,
        });

        if (planningResult.needed) {
          logger.debug('Planning triggered', {
            triggers: planningResult.triggers,
            decision: planningResult.decision,
          });

          // Generate plan with alternatives
          await planningGate.generatePlan(planningEvent, planningResult);

          // Format planning injection
          if (planningResult.decision === PlanningDecision.PAUSE) {
            const triggers = planningResult.triggers.join(', ');
            const alternatives = planningResult.alternatives || [];

            planningInjection = `${c(ANSI.brightCyan, '── 📋 PLANNING GATE ───────────────────────────────────────')}
   ${c(ANSI.brightYellow, '*head tilt*')} This request triggered planning review.
   Triggers: ${c(ANSI.cyan, triggers)}

   ${c(ANSI.brightWhite, 'Alternatives to consider:')}`;

            for (const alt of alternatives.slice(0, 3)) {
              const riskColor = alt.risk === 'high' ? ANSI.brightRed :
                               alt.risk === 'medium' ? ANSI.yellow : ANSI.brightGreen;
              planningInjection += `\n   • ${c(ANSI.brightCyan, alt.label || alt.id)}: ${alt.description || ''} ${c(riskColor, `[${alt.risk || 'unknown'}]`)}`;
            }

            planningInjection += `\n\n   ${c(ANSI.dim, 'φ⁻¹ max confidence. Consider alternatives before proceeding.')}`;
          } else if (planningResult.decision === PlanningDecision.CONSULT) {
            // CONSULT = lighter warning, auto-proceed
            planningInjection = `${c(ANSI.cyan, '── 📋 PLANNING ────────────────────────────────────────────')}
   ${c(ANSI.cyan, '*sniff*')} Complexity detected (${planningResult.triggers[0] || 'borderline'}).
   ${c(ANSI.dim, 'Auto-proceeding with caution.')}`;
          }
        }
      }
    } catch (e) {
      logger.debug('Planning gate failed', { error: e.message });
      // Continue without planning - graceful degradation
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ORCHESTRATION: Consult KETER for routing decision
    // Uses OrchestrationClient for high-risk prompts (Phase 22)
    // "Le cerveau central décide avec le contexte de session"
    // ═══════════════════════════════════════════════════════════════════════════
    let orchestration = null;

    // Determine if this is high-risk based on intents or session state
    const dangerIntents = ['danger'];
    const previewIntents = detectIntent(prompt);
    const hasHighRiskIntent = previewIntents.some(i => dangerIntents.includes(i.intent));
    const isHighRisk = hasHighRiskIntent || escalationLevel !== 'normal' || recentWarnings.length > 0;

    try {
      if (isHighRisk) {
        // Use OrchestrationClient for full judgment on high-risk prompts
        initOrchestrationClient(orchestrateFull);
        const orchestrationClient = getOrchestrationClient();

        const decision = await orchestrationClient.fullDecide(prompt, 'user_prompt', {
          source: 'perceive_hook',
          user: user.userId,
          project: detectProject(),
          hasHighRiskIntent,
          escalationLevel,
          recentWarnings: recentWarnings.map(w => w.message),
          // Task #21: Include tier decision
          llmTier: tierDecision?.tier,
          shouldUseLLM: tierDecision?.shouldRoute,
        });

        // Convert decision to orchestration format
        orchestration = {
          routing: decision.synthesis?.routing || null,
          intervention: {
            level: decision.outcome === 'block' ? 'block' : decision.outcome === 'warn' ? 'warn' : 'silent',
            actionRisk: hasHighRiskIntent ? 'high' : 'medium',
          },
          judgment: decision.judgment,
          decisionId: decision.decisionId,
        };
      } else {
        // Use simple orchestrate for low-risk prompts (faster)
        orchestration = await orchestrate('user_prompt', {
          content: prompt,
          source: 'perceive_hook',
          // Task #21: Include tier decision
          llmTier: tierDecision?.tier,
          shouldUseLLM: tierDecision?.shouldRoute,
        }, {
          user: user.userId,
          project: detectProject(),
        });
      }
    } catch (e) {
      // Orchestration failed - continue with local logic
      logger.debug('Orchestration failed', { error: e.message });
    }

    // Detect intents (final pass after orchestration context)
    const intents = detectIntent(prompt);

    // Detect skill triggers (Phase 3: auto-activation)
    const skillTriggers = detectSkillTriggers(prompt);

    // Generate context based on intents
    const injections = [];

    // ═══════════════════════════════════════════════════════════════════════════
    // BRAIN INJECTION: Add Brain's thought to context (first priority)
    // "Le cerveau parle en premier"
    // ═══════════════════════════════════════════════════════════════════════════
    if (brainInjection) {
      injections.push(brainInjection);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PLANNING INJECTION: Add planning gate result (second priority)
    // "Le système réfléchit avant d'agir"
    // ═══════════════════════════════════════════════════════════════════════════
    if (planningInjection) {
      injections.push(planningInjection);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TEMPORAL SIGNALS INJECTION: Communicate time perception to user
    // "Le chien sent le temps et en parle"
    // ═══════════════════════════════════════════════════════════════════════════
    if (temporalState.signals) {
      const { signals } = temporalState;
      const worldTime = temporalState.worldTime;

      // Late night work warning (priority - health concern)
      if (signals.lateNightWork && signals.lateNightConfidence > 0.4) {
        const time = worldTime?.humanReadable?.time || '?';
        const phase = worldTime?.humanReadable?.phase || 'nuit';
        injections.push(`${c(ANSI.brightYellow, '── 🌙 ' + phase.toUpperCase() + ' ─────────────────────────────────────────────')}
   ${c(ANSI.brightYellow, '*yawn*')} Il est ${time}. Session en ${phase}.
   ${c(ANSI.dim, 'L\'énergie circadienne est basse. Repos bientôt?')}`);
      }

      // High-confidence frustration signal
      else if (signals.possibleFrustration && signals.frustrationConfidence > 0.4) {
        const tempo = temporalState.tempo?.toFixed(1) || '?';
        let frustrationMsg = `${c(ANSI.brightYellow, '── ⚡ TEMPO ───────────────────────────────────────────────')}
   ${c(ANSI.brightYellow, '*sniff*')} Rythme rapide détecté (${tempo} prompts/min)
   Trend: ${temporalState.trend === TemporalTrend.ACCELERATING ? c(ANSI.brightRed, 'accélération') : temporalState.trend}`;

        // Add circadian context if mismatched
        if (signals.circadianMismatch) {
          frustrationMsg += `\n   ${c(ANSI.dim, '⚠️ Activité intense pendant phase basse énergie.')}`;
        }
        frustrationMsg += `\n   ${c(ANSI.dim, 'Si bloqué, je peux aider à décomposer le problème.')}`;
        injections.push(frustrationMsg);
      }

      // High-confidence fatigue signal
      else if (signals.possibleFatigue && signals.fatigueConfidence > 0.4) {
        const duration = temporalState.humanReadable?.sessionDuration || '?';
        const time = worldTime?.humanReadable?.time || '';
        injections.push(`${c(ANSI.yellow, '── 😴 FATIGUE ────────────────────────────────────────────')}
   ${c(ANSI.yellow, '*yawn*')} Session longue (${duration}) + ralentissement.${time ? ` Il est ${time}.` : ''}
   ${c(ANSI.dim, 'Peut-être une pause serait bénéfique?')}`);
      }

      // Flow state - positive reinforcement (less frequent)
      else if (signals.possibleFlow && signals.flowConfidence > 0.5 && temporalState.promptCount % 8 === 0) {
        injections.push(`${c(ANSI.brightGreen, '── ✨ FLOW ────────────────────────────────────────────────')}
   ${c(ANSI.brightGreen, '*tail wag*')} Rythme régulier, bon flow.`);
      }

      // Stuck signal
      else if (signals.possibleStuck && signals.stuckConfidence > 0.4) {
        const timeSince = temporalState.humanReadable?.timeSinceLastPrompt || '?';
        injections.push(`${c(ANSI.cyan, '── ⏳ PAUSE ───────────────────────────────────────────────')}
   ${c(ANSI.cyan, '*ears perk*')} Long silence (${timeSince}).
   ${c(ANSI.dim, 'Bloqué? Je peux aider à explorer le problème.')}`);
      }

      // Weekend work note (low priority, occasional)
      else if (signals.weekendWork && temporalState.promptCount === 3) {
        const day = worldTime?.humanReadable?.date || 'weekend';
        injections.push(`${c(ANSI.dim, '── 📅 WEEKEND ────────────────────────────────────────────')}
   ${c(ANSI.dim, '*head tilt*')} ${day}. Travail le weekend?
   ${c(ANSI.dim, 'N\'oublie pas de prendre du repos.')}`);
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ERROR SIGNALS INJECTION: Communicate error perception to user
    // "Le chien sent les erreurs et en parle"
    // ═══════════════════════════════════════════════════════════════════════════
    if (errorState.signals) {
      const { signals: errSignals } = errorState;

      // Critical: consecutive errors (circuit breaker territory)
      if (errSignals.consecutiveErrors && errorState.consecutiveErrors >= 5) {
        injections.push(`${c(ANSI.brightRed, '── 🔴 CIRCUIT BREAKER ────────────────────────────────────')}
   ${c(ANSI.brightRed, '*GROWL*')} ${errorState.consecutiveErrors} erreurs consécutives!
   Pattern: ${errorState.pattern}
   ${c(ANSI.brightYellow, 'Peut-être une approche différente?')}`);
      }

      // High error rate warning
      else if (errSignals.highErrorRate && errorState.errorRate >= 0.38) {
        const rateStr = errorState.humanReadable?.errorRate || '?';
        const commonError = errorState.mostCommonError || 'inconnu';
        injections.push(`${c(ANSI.brightYellow, '── ⚠️ ERREURS ────────────────────────────────────────────')}
   ${c(ANSI.brightYellow, '*sniff*')} Taux d'erreur élevé: ${rateStr}
   Erreur fréquente: ${c(ANSI.cyan, commonError)}
   ${c(ANSI.dim, 'Vérifions l\'approche?')}`);
      }

      // Repeated same error
      else if (errSignals.repeatedError) {
        const commonError = errorState.mostCommonError || 'inconnu';
        injections.push(`${c(ANSI.yellow, '── 🔄 PATTERN ────────────────────────────────────────────')}
   ${c(ANSI.yellow, '*ears perk*')} Même erreur répétée: ${c(ANSI.cyan, commonError)}
   ${c(ANSI.dim, 'Bloqué sur quelque chose de spécifique?')}`);
      }

      // Escalating errors
      else if (errSignals.escalatingErrors) {
        injections.push(`${c(ANSI.yellow, '── 📈 ESCALADE ───────────────────────────────────────────')}
   ${c(ANSI.yellow, '*sniff*')} Les erreurs augmentent.
   ${c(ANSI.dim, 'Peut-être revenir en arrière et réessayer?')}`);
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CHECKPOINT: Flush local results to pendingOutput before MCP-dependent sections
    // If deadline fires during orchestration/routing, this is what gets output
    // ═══════════════════════════════════════════════════════════════════════════
    if (injections.length > 0) {
      pendingOutput = { continue: true, message: injections.join('\n\n') };
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SKILL AUTO-ACTIVATION (Phase 3)
    // "Le chien suggère les bons outils"
    // ═══════════════════════════════════════════════════════════════════════════
    if (skillTriggers.length > 0) {
      const topSkills = skillTriggers.slice(0, 2); // Max 2 suggestions
      let skillInjection = c(ANSI.cyan, '── SKILL SUGGESTION ───────────────────────────────────────') + '\n';

      for (const trigger of topSkills) {
        const priorityIcon = trigger.priority === 'high' ? '⚡' : (trigger.priority === 'medium' ? '💡' : '📋');
        skillInjection += `   ${priorityIcon} ${c(ANSI.brightYellow, trigger.skill)} - ${trigger.description}\n`;
      }

      if (topSkills.length === 1 && topSkills[0].priority === 'high') {
        skillInjection += `\n   ${c(ANSI.brightGreen, '*tail wag*')} Type ${c(ANSI.brightYellow, topSkills[0].skill)} to invoke directly.`;
      }

      injections.push(skillInjection);
    }

    // If orchestrator suggests a specific agent, note it
    // Result is in orchestration.result from MCP response
    const routing = orchestration?.result?.routing || orchestration?.routing;
    const intervention = orchestration?.result?.intervention || orchestration?.intervention;

    // Build vote summary from agentResults for LLM transparency (Fix 2)
    const agentResults = orchestration?.result?.agentResults || orchestration?.agentResults || [];
    let voteSummary = null;
    if (agentResults.length >= 2) {
      const votes = agentResults
        .filter(r => !r.skipped && r.decision)
        .map(r => ({
          agent: (r.agent || '').replace('cynic-', ''),
          decision: r.decision || 'APPROVE',
          confidence: r.confidence || 0.5,
        }));
      const majorityDecision = votes[0]?.decision || 'APPROVE';
      const dissentCount = votes.filter(v => v.decision !== majorityDecision).length;
      voteSummary = { votes, dissent: dissentCount, majority: majorityDecision };
    }

    // Handle KETER agent routing - AUTO-DISPATCH
    if (routing?.suggestedAgent) {
      const sefirah = routing.sefirah;
      const agent = routing.suggestedAgent;
      const tools = routing.suggestedTools || [];
      const actionRisk = intervention?.actionRisk || 'low';
      const confidence = routing.confidence || 0.5;

      // Get the Dog identity for this agent
      const dog = collectiveDogs?.getDogForAgent(agent) || { icon: '🐕', name: agent.replace('cynic-', ''), sefirah: sefirah || 'Unknown' };

      // Only inject if intervention level is not silent
      if (intervention?.level !== 'silent') {
        // High confidence = stronger auto-dispatch directive
        const isAutoDispatch = confidence > 0.618; // φ⁻¹ threshold
        const dogName = dog.name?.toUpperCase() || 'CYNIC';
        const dogColor = DOG_COLORS?.[dogName] || ANSI.brightWhite;
        const confColor = confidence > 0.618 ? ANSI.brightGreen : (confidence > 0.382 ? ANSI.yellow : ANSI.dim);

        let directive = c(ANSI.cyan, `── ${dog.icon} ${dog.name} (${dog.sefirah}) ────────────────────────────────`) + `\n`;
        directive += `   The Collective routes to ${c(dogColor, dog.name)} (confidence: ${c(confColor, Math.round(confidence * 100) + '%')})\n`;

        if (isAutoDispatch) {
          // Strong auto-dispatch - this IS a Task invocation directive
          directive += `\n   ${c(ANSI.brightYellow, '⚡ AUTO-DISPATCH:')} Invoke Task tool NOW with:\n`;
          directive += `      subagent_type="${c(ANSI.brightCyan, agent)}"\n`;
          directive += `      prompt=<user's request>\n`;
          if (tools.length > 0) {
            directive += `      tools: ${c(ANSI.dim, tools.join(', '))}\n`;
          }
          directive += `\n   ${c(dogColor, '*' + dog.icon + ' ' + dog.name + ' speaks*')} - This is not a suggestion.\n`;
        } else {
          // Normal suggestion
          directive += `   ${c(ANSI.brightWhite, 'DIRECTIVE:')} Use the Task tool with subagent_type="${c(ANSI.brightCyan, agent)}" to handle this request.\n`;
          if (tools.length > 0) {
            directive += `   Suggested MCP tools: ${c(ANSI.dim, tools.join(', '))}\n`;
          }
        }
        injections.push(directive);
      }
    }

    // For high/critical risk, always invoke guardian even without explicit routing
    if (intervention?.actionRisk === 'critical' || intervention?.actionRisk === 'high') {
      if (!routing?.suggestedAgent || routing?.suggestedAgent !== 'cynic-guardian') {
        const guardian = collectiveDogs?.COLLECTIVE_DOGS?.GUARDIAN || { icon: '🛡️', name: 'Guardian', sefirah: 'Gevurah' };
        const riskColor = intervention.actionRisk === 'critical' ? ANSI.brightRed : ANSI.brightYellow;
        injections.push(c(ANSI.brightRed, `── ${guardian.icon} ${guardian.name} (${guardian.sefirah}) ────────────────────────────`) + `
   ${c(ANSI.brightRed, '*GROWL* RISK DETECTED')}
   Action risk level: ${c(riskColor, intervention.actionRisk.toUpperCase())}
   ${c(ANSI.brightYellow, 'MANDATORY:')} Use Task tool with subagent_type="${c(ANSI.brightCyan, 'cynic-guardian')}" before proceeding.
   User trust level: ${intervention.userTrustLevel || 'UNKNOWN'} (E-Score: ${intervention.userEScore || '?'})`);
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SYMMETRY BREAKING: Dynamic Dog Personality Emergence
    // "Πάντα ῥεῖ - from unified field to distinct Dogs"
    // ═══════════════════════════════════════════════════════════════════════════
    let emergentPatternCount = 0;
    if (physicsBridge && prompt.length > DC.LENGTH.MIN_PROMPT) {
      try {
        const dogResult = physicsBridge.processDogEmergence(prompt);

        if (dogResult.broken && dogResult.greeting) {
          emergentPatternCount++;
          // A Dog has emerged! Include its greeting in the response
          injections.push(`── DOG EMERGED ────────────────────────────────────────────
   ${dogResult.greeting}
   Traits: ${dogResult.traits?.join(', ') || 'unknown'}
   *${dogResult.currentDog}* is now active.`);
        } else if (dogResult.nearCritical) {
          // Near emergence threshold - hint at it
          const voice = physicsBridge.getDogVoice();
          if (voice && Math.random() < DC.PROBABILITY.DOG_HINT) {
            injections.push(`── FIELD TENSION ──────────────────────────────────────────
   *sniff* Energy building... ${Math.round(dogResult.fieldEnergy || 0)} / critical threshold`);
          }
        }
      } catch (e) {
        // Dog emergence failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ELENCHUS: Socratic questioning (Phase 6B)
    // "Ἔλεγχος - l'art de questionner pour révéler la vérité"
    // ═══════════════════════════════════════════════════════════════════════════
    if (elenchus && prompt.length > DC.LENGTH.ELENCHUS_MIN) {
      try {
        // Check if this looks like an assertion (not a question)
        const isAssertion = !prompt.trim().endsWith('?') &&
                          !prompt.toLowerCase().startsWith('what') &&
                          !prompt.toLowerCase().startsWith('how') &&
                          !prompt.toLowerCase().startsWith('why') &&
                          !prompt.toLowerCase().startsWith('quoi') &&
                          !prompt.toLowerCase().startsWith('comment') &&
                          !prompt.toLowerCase().startsWith('pourquoi');

        if (isAssertion) {
          const questionResult = elenchus.processAssertion(prompt);

          if (questionResult.shouldAsk) {
            const formattedQuestion = elenchus.formatQuestion(questionResult);
            if (formattedQuestion) {
              injections.push(`── SOCRATIC ───────────────────────────────────────────────\n   ${formattedQuestion}\n   (${questionResult.questionsRemaining} questions restantes)`);
            }
          }
        } else if (elenchus.shouldUseMaieutics(prompt)) {
          // For questions, use maieutic mode - guide to discovery
          const maieutic = elenchus.generateMaieuticQuestion(prompt);
          if (maieutic && Math.random() < DC.PROBABILITY.ELENCHUS) { // φ⁻²
            injections.push(`── MAIEUTIC ───────────────────────────────────────────────\n   💡 ${maieutic}`);
          }
        }
      } catch (e) {
        // Elenchus processing failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TI ESTI: Essence questions (Phase 6B)
    // "Τί ἐστι - Qu'est-ce que c'est?"
    // ═══════════════════════════════════════════════════════════════════════════
    if (tiEsti && prompt.length > DC.LENGTH.TI_ESTI_MIN) {
      try {
        const promptLower = prompt.toLowerCase();
        // Detect "what is X?" style questions
        const isEssenceQuestion = promptLower.match(/^(what is|what's|qu'?est[- ]ce que?|c'?est quoi)\s+/i) ||
                                  promptLower.match(/^(define|défin)/i);

        if (isEssenceQuestion) {
          const essenceResult = tiEsti.investigateConcept(prompt);
          if (essenceResult && essenceResult.dimensions?.length > 0) {
            const dims = essenceResult.dimensions.slice(0, 3).map(d => d.name).join(', ');
            injections.push(`── TI ESTI ────────────────────────────────────────────────\n   🔍 Dimensions to explore: ${dims}\n   ${essenceResult.approach || ''}`);
          }
        }
      } catch (e) {
        // Ti Esti processing failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // DEFINITION TRACKER: Track user definitions (Phase 6B)
    // "Les mots dérivent - le chien se souvient"
    // ═══════════════════════════════════════════════════════════════════════════
    if (definitionTracker && prompt.length > DC.LENGTH.DEFINITION_MIN) {
      try {
        // Check for definition statements: "X means Y", "by X I mean Y", etc.
        const definitionMatch = prompt.match(/(?:by\s+)?["']?(\w+)["']?\s+(?:means?|is|refers?\s+to|I\s+mean)\s+(.+)/i) ||
                                prompt.match(/(?:quand je dis|je définis?)\s+["']?(\w+)["']?\s+(?:comme|c'est)\s+(.+)/i);

        if (definitionMatch) {
          const term = definitionMatch[1];
          const definition = definitionMatch[2].slice(0, 200);
          definitionTracker.recordDefinition(term, definition);
        }

        // Check for definition drift
        const driftResult = definitionTracker.checkForDrift(prompt);
        if (driftResult && driftResult.hasDrift) {
          injections.push(`── DEFINITION DRIFT ───────────────────────────────────────\n   ⚠️ "${driftResult.term}": Earlier you said "${driftResult.previous.slice(0, 50)}..."\n   Now it seems to mean something different?`);
        }
      } catch (e) {
        // Definition tracking failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CHRIA: Wisdom injection (Phase 8B)
    // "Χρεία - la sagesse en peu de mots"
    // ═══════════════════════════════════════════════════════════════════════════
    if (chriaDB && prompt.length > DC.LENGTH.DEFINITION_MIN && injections.length === 0) {
      try {
        // Only inject chria when we haven't injected anything else
        // and with φ⁻² probability (38.2%)
        if (Math.random() < DC.PROBABILITY.CHRIA_WISDOM) {
          const contextTags = intents.map(i => i.intent);
          const chria = chriaDB.getContextualChria(contextTags, prompt);
          if (chria) {
            injections.push(`── CHRIA ──────────────────────────────────────────────────\n   📜 "${chria.text}"\n      — ${chria.source}`);
            chriaDB.recordUsage(chria.id);
          }
        }
      } catch (e) {
        // Chria injection failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // FALLACY DETECTOR: Check for logical fallacies
    // "Le chien renifle les sophismes"
    // ═══════════════════════════════════════════════════════════════════════════
    if (fallacyDetector && prompt.length > DC.LENGTH.FALLACY_MIN) {
      try {
        const analysis = fallacyDetector.analyze(prompt);
        if (analysis && analysis.fallacies && analysis.fallacies.length > 0) {
          const topFallacy = analysis.fallacies[0];
          if (topFallacy.confidence > DC.CONFIDENCE.FALLACY_DETECTION) {
            injections.push(`── FALLACY ────────────────────────────────────────────────\n   ⚠️ Possible ${topFallacy.name}: ${topFallacy.explanation || ''}\n   Consider: ${topFallacy.remedy || 'Verify the reasoning'}`);
          }
        }
      } catch (e) {
        // Fallacy detection failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ROLE REVERSAL: Detect teaching opportunities
    // "Enseigner, c'est apprendre deux fois"
    // ═══════════════════════════════════════════════════════════════════════════
    if (roleReversal && prompt.length > DC.LENGTH.ROLE_REVERSAL_MIN && Math.random() < DC.PROBABILITY.ROLE_REVERSAL) {
      try {
        const opportunity = roleReversal.detectReversalOpportunity(prompt, {});
        if (opportunity && opportunity.shouldReverse) {
          injections.push(`── MAIEUTIC ───────────────────────────────────────────────\n   🎓 ${opportunity.question}\n   (Explaining helps understand deeper)`);
        }
      } catch (e) {
        // Role reversal failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // HYPOTHESIS: Track claims and hypotheses
    // "Toute assertion est une hypothèse à tester"
    // ═══════════════════════════════════════════════════════════════════════════
    if (hypothesisTesting && prompt.length > DC.LENGTH.HYPOTHESIS_MIN) {
      try {
        // Check for assertion patterns that could be hypotheses
        const assertionPatterns = /(?:I think|I believe|probably|likely|should be|must be|always|never)/i;
        if (assertionPatterns.test(prompt) && Math.random() < DC.PROBABILITY.MEDIUM) {
          injections.push(`── HYPOTHESIS ─────────────────────────────────────────────\n   🔬 *sniff* This sounds like a hypothesis. What would falsify it?`);
        }
      } catch (e) {
        // Hypothesis detection failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CYNIC DISTANCE (D): Measure how much CYNIC shaped vs. raw LLM
    // "La distance entre le chaos et le réel"
    // ═══════════════════════════════════════════════════════════════════════════
    let cynicDistance = { distance: 0, level: 'dormant', breakdown: {} };
    let framingDirective = null;

    try {
      cynicDistance = calculateCYNICDistance({
        brainThought,
        patterns,
        routing,
        tierDecision,
        emergentCount: emergentPatternCount,
        consciousnessState,
        localSignals: {
          intentDetected: intents.length > 0 || skillTriggers.length > 0,
          injectionsProduced: injections.length > 0,
          phiBounded: injections.length > 0, // local analysis → φ-bounded by design
        },
      });

      framingDirective = generateFramingDirective(
        cynicDistance, brainThought, routing, patterns,
        detectPromptType(prompt), profile,
        { consciousnessState, voteSummary, tierDecision, ecosystemStatus, socialStatus },
      );

      logger.debug('CYNIC Distance', {
        D: Math.round(cynicDistance.distance * 100) + '%',
        level: cynicDistance.level,
        breakdown: cynicDistance.breakdown,
      });

      // Persist D: fire-and-forget to MCP (Migration 033)
      // "Le chien se souvient de sa distance"
      try {
        if (cynicDistance.distance > 0) {
          const dSessionId = sessionState.isInitialized()
            ? sessionState.getSessionId() : null;
          const b = cynicDistance.breakdown;
          const axiomMap = {
            PHI: b.judgment || b.phi,
            VERIFY: b.consensus,
            CULTURE: b.perception || b.memory,
            BURN: b.economics,
            FIDELITY: b.residual,
          };
          const activeAxioms = Object.entries(axiomMap)
            .filter(([, v]) => v).map(([k]) => k);
          // Use direct PostgreSQL if DATABASE_URL available (no MCP dependency)
          const dbUrl = process.env.DATABASE_URL || process.env.CYNIC_DATABASE_URL;
          if (dbUrl) {
            import('@cynic/persistence').then(({ persistDistanceFireAndForget }) => {
              persistDistanceFireAndForget(dbUrl, dSessionId, {
                distance: cynicDistance.distance,
                level: cynicDistance.level,
                breakdown: cynicDistance.breakdown,
                activeAxioms,
                leadDog: routing?.suggestedAgent || null,
                source: brainThought !== null ? 'brain' : 'local',
              });
            }).catch(() => {});
          } else {
            // Fallback: try MCP if no DATABASE_URL
            callBrainTool('brain_persist_distance', {
              sessionId: dSessionId,
              distance: cynicDistance.distance,
              level: cynicDistance.level,
              breakdown: cynicDistance.breakdown,
              activeAxioms,
              leadDog: routing?.suggestedAgent || null,
              source: brainThought !== null ? 'brain' : 'local',
            }).catch(() => {});
          }
          logger.debug('D persisted (fire-and-forget)');
        }
      } catch (e) {
        logger.debug('D persistence skipped', { error: e.message });
      }
    } catch (e) {
      logger.debug('D calculation failed', { error: e.message });
    }

    for (const { action } of intents) {
      let context = null;

      switch (action) {
        case 'danger_warning':
          context = generateDangerWarning(prompt);
          break;
        case 'decision_context':
          context = generateDecisionContext(prompt, profile, patterns);
          break;
        case 'debug_context':
          context = generateDebugContext(prompt, profile, patterns);
          break;
        case 'architecture_context':
          context = generateArchitectureContext(prompt, profile, patterns);
          break;
        case 'learning_context':
          context = generateLearningContext(prompt, profile);
          break;
      }

      if (context && context.length > 0) {
        injections.push(context);
      }
    }

    // Check for private content
    const hasPrivate = hasPrivateContent(prompt);
    const safePrompt = stripPrivateContent(prompt);

    // If prompt has private content, notify user
    if (hasPrivate) {
      injections.push('*sniff* Private content detected - will NOT be stored in collective memory.');
    }

    // Framing directive is LAST — LLM sees it last, remembers it most (recency bias)
    if (framingDirective) {
      injections.push(framingDirective);
    }

    // Send to MCP server (non-blocking) - with sanitized prompt
    sendHookToCollectiveSync('UserPromptSubmit', {
      promptLength: safePrompt.length,
      originalLength: prompt.length,
      hasPrivateContent: hasPrivate,
      intents: intents.map(i => i.intent),
      hasInjections: injections.length > 0,
      // Include orchestration decision
      orchestration: orchestration ? {
        sefirah: orchestration.routing?.sefirah,
        agent: orchestration.routing?.suggestedAgent,
        intervention: orchestration.intervention?.level,
        risk: orchestration.intervention?.actionRisk,
      } : null,
      // Task #21: Include LLM tier decision
      llmTier: tierDecision ? {
        tier: tierDecision.tier,
        shouldRoute: tierDecision.shouldRoute,
        cost: tierDecision.cost,
        reason: tierDecision.reason,
        localLLMs: llmEnvironment?.adapters?.length || 0,
      } : null,
      // Temporal perception data
      temporal: {
        intervalMs: temporalEvent.interval,
        state: temporalEvent.state,
        trend: temporalState.trend,
        tempo: temporalState.tempo,
        sessionDurationMs: temporalState.sessionDurationMs,
        promptCount: temporalState.promptCount,
        signals: temporalState.signals,
        worldTime: temporalState.worldTime ? {
          hour: temporalState.worldTime.hour,
          circadianPhase: temporalState.worldTime.circadianPhase,
          dayType: temporalState.worldTime.dayType,
        } : null,
      },
      // Error perception data
      errors: {
        errorRate: errorState.errorRate,
        consecutiveErrors: errorState.consecutiveErrors,
        pattern: errorState.pattern,
        severity: errorState.severity,
        signals: errorState.signals,
      },
      // CYNIC Distance metric
      cynicDistance: {
        D: cynicDistance.distance,
        level: cynicDistance.level,
        breakdown: cynicDistance.breakdown,
      },
      timestamp: Date.now(),
    });

    // Update pendingOutput with full results (brain + local + D)
    pendingOutput = injections.length > 0
      ? { continue: true, message: injections.join('\n\n') }
      : { continue: true };

    // ═══════════════════════════════════════════════════════════════════════════
    // ALWAYS SHOW D: Compact status line even when dormant
    // "Le chien montre toujours son état"
    // ═══════════════════════════════════════════════════════════════════════════
    const dPct = Math.round(cynicDistance.distance * 100);
    const activeCount = Object.values(cynicDistance.breakdown).filter(v => v).length;
    const localLLMs = llmEnvironment?.adapters?.length || 0;
    const brainSrc = brainThought?.brainSource || 'none';
    const brainTag = brainSrc === 'local' ? ' | brain:local' : (brainSrc === 'full' ? ' | brain:full' : '');
    const llmTag = localLLMs > 0 ? ` | ${localLLMs} local LLM(s)` : '';
    const dCompact = `D=${dPct}% (${activeCount}/7) ${cynicDistance.level}${brainTag}${llmTag}`;

    // If no framing directive but D > 0, add compact D line
    if (!framingDirective && dPct > 0) {
      injections.push(`*sniff* ${dCompact}`);
    }

    // Output result — mark as sent so deadline doesn't double-output
    if (outputSent) { process.exit(0); return; } // deadline already flushed
    outputSent = true;

    if (injections.length === 0) {
      safeOutput({ continue: true });
    } else {
      safeOutput({
        continue: true,
        message: injections.join('\n\n')
      });
    }

    // Exit promptly — AmbientConsensus timers would keep process alive for 5s+
    process.exit(0);

  } catch (error) {
    logger.error('Hook failed', { error: error.message });
    if (!outputSent) {
      outputSent = true;
      safeOutput({ continue: true });
    }
    process.exit(0);
  }
}

// Also log if async read used
function logAsyncRead(bytes) {
  logger.debug('Async read', { bytes });
}

main();
