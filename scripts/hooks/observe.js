#!/usr/bin/env node
/**
 * CYNIC Observe Hook - PostToolUse
 *
 * "Le chien observe" - CYNIC watches and learns
 *
 * This hook runs after every tool execution.
 * It silently observes patterns and learns from outcomes.
 *
 * @event PostToolUse
 * @behavior non-blocking (never interferes)
 */

'use strict';

import path from 'path';

// ESM imports from the lib bridge
import cynic, {
  DC,
  detectUser,
  detectProject,
  loadUserProfile,
  updateUserProfile,
  saveCollectivePattern,
  orchestrateFull,  // Phase 21: Full orchestration with UnifiedOrchestrator
  sendHookToCollectiveSync,
  callBrainTool,
  sendTestFeedback,
  sendCommitFeedback,
  sendBuildFeedback,
  getTaskEnforcer,
  getAutoJudge,
  getSelfRefinement,
  getContributorDiscovery,
  getConsciousness,
  getHarmonyAnalyzer,
  getSignalCollector,
  getCognitiveBiases,
  getTopologyTracker,
  getInterventionEngine,
  getPsychology,
  getThermodynamics,
  getCosmopolitan,
  getVoluntaryPoverty,
  getDialectic,
  getInferenceEngine,
  getPhysicsBridge,
} from '../lib/index.js';

// Phase 22: Session state and feedback/suggestion engines
import {
  getSessionState,
  getFeedbackCollector,
  getSuggestionEngine,
  detectErrorType,  // From pattern-detector.js
  getReasoningBank,  // P1.2: Trajectory learning
  getFactExtractor,  // M2: Auto fact extraction to PostgreSQL
  getTelemetryCollector,  // Usage stats, frictions, benchmarking
  recordMetric,
  recordTiming,
  recordFriction,
  getAutoOrchestratorSync,  // Auto-orchestration: Dogs consultation
  // Phase 23: Harmonic Feedback System (Kabbalah + CIA + Cybernetics + Thompson)
  getHarmonicFeedback,
  getImplicitFeedback,
  SEFIROT_CHANNELS,
  // Task #84: Q-Learning with persistence
  getQLearningServiceWithPersistence,
  // BURN extractions from observe.js
  antiPatternState,
  calculateRealReward,
  detectAntiPatterns,
  getActiveDog,
  formatActiveDog,
  generateInlineStatus,
  COLLECTIVE_DOGS,
  // Phase 2 BURN extractions
  extractFacts,
  storeFacts,
  processTriggerEvent,
  extractCommitMessage,
  // Task #25: LOUD debug mode
  isDebugMode,
  isLoudMode,
  debugLog,
  debugError,
  debugTiming,
  getErrorBuffer,
  // Task #27-28: Trigger Engine - Proactive Suggestions
  getTriggerEngine,
  TRIGGER_TYPES,
} from './lib/index.js';

// =============================================================================
// LOAD OPTIONAL MODULES
// =============================================================================

const enforcer = getTaskEnforcer();
const autoJudge = getAutoJudge();
const selfRefinement = getSelfRefinement();
const contributorDiscovery = getContributorDiscovery();
const consciousness = getConsciousness();
const harmonyAnalyzer = getHarmonyAnalyzer();
const signalCollector = getSignalCollector();
const cognitiveBiases = getCognitiveBiases();
const topologyTracker = getTopologyTracker();
const interventionEngine = getInterventionEngine();
const psychology = getPsychology();
const thermodynamics = getThermodynamics();
const cosmopolitan = getCosmopolitan();
const voluntaryPoverty = getVoluntaryPoverty();
const dialectic = getDialectic();
const inferenceEngine = getInferenceEngine();
const physicsBridge = getPhysicsBridge();

// Phase 22: Feedback and suggestion engines
const feedbackCollector = getFeedbackCollector();
const suggestionEngine = getSuggestionEngine();

// Phase 23: Harmonic Feedback System (Kabbalah + CIA + Cybernetics + Thompson Sampling)
const harmonicFeedback = getHarmonicFeedback();
const implicitFeedback = getImplicitFeedback();

// Task #28-30: Trigger Engine - Proactive Suggestions with Dogs Voting
const triggerEngine = getTriggerEngine({
  enabled: true,
  // Task #30: Wire Dogs voting via auto-orchestrator's CollectivePack
  getCollectivePack: async () => {
    const autoOrchestrator = getAutoOrchestratorSync();
    if (autoOrchestrator?.getCollectivePack) {
      return autoOrchestrator.getCollectivePack();
    }
    return null;
  },
});

// ═══════════════════════════════════════════════════════════════════════════
// FIL 2 (Task #84): Wire harmonic feedback → learning service
// Two paths: brain_learning (patterns) + Q-Learning (episodes)
// ═══════════════════════════════════════════════════════════════════════════
if (harmonicFeedback && harmonicFeedback.setLearningCallback) {
  harmonicFeedback.setLearningCallback(async (feedback) => {
    // Path 1: Call brain_learning tool to process feedback (increments pattern frequency)
    callBrainTool('brain_learning', {
      action: 'feedback',
      outcome: feedback.outcome,
      context: feedback.sourceContext,
    }).catch(() => {
      // Silently fail - MCP server might not be available
    });

    // Path 2: Record directly to Q-Learning for episode tracking
    const qlearning = getQLearningServiceWithPersistence();
    if (qlearning) {
      try {
        qlearning.recordAction(feedback.sourceContext?.type || 'harmonic_feedback', {
          success: feedback.outcome === 'correct',
          source: 'harmonic',
          confidence: feedback.sourceContext?.confidence || 0.5,
          sentiment: feedback.sourceContext?.sentiment,
          sefirah: feedback.sourceContext?.sefirah,
        });
      } catch (e) {
        // Q-Learning recording is best-effort
      }
    }
  });
}

// =============================================================================
// ANTI-PATTERN DETECTOR - Now imported from ./lib/anti-pattern-detector.js
// antiPatternState, calculateRealReward, detectAntiPatterns are now imported
// =============================================================================

// =============================================================================
// COLLECTIVE DOGS (SEFIROT) - Now imported from ./lib/active-dog.js
// COLLECTIVE_DOGS, getActiveDog, formatActiveDog, generateInlineStatus are imported
// =============================================================================

// =============================================================================
// SYMBIOSIS CACHE: Last routing/judgment data for visibility (Task #4 + #5)
// FIX: Now persisted to file for cross-invocation reads
// "L'humain VOIT ce que CYNIC pense" - Human sees CYNIC's reasoning
// =============================================================================
import { existsSync, readFileSync, writeFileSync, mkdirSync } from 'fs';
import { homedir } from 'os';

const SYMBIOSIS_CACHE_FILE = path.join(homedir(), '.cynic', 'symbiosis-cache.json');

/**
 * Load symbiosis cache from file (previous invocation's data)
 * Each hook invocation is a separate process — this bridges them.
 */
function loadSymbiosisCache() {
  try {
    if (existsSync(SYMBIOSIS_CACHE_FILE)) {
      const data = JSON.parse(readFileSync(SYMBIOSIS_CACHE_FILE, 'utf8'));
      // Only use if less than 5 minutes old
      if (data.updatedAt && (Date.now() - data.updatedAt) < 300000) {
        return data;
      }
    }
  } catch { /* ignore */ }
  return { lastJudgment: null, lastConsensus: null, lastRouting: null, updatedAt: null };
}

/**
 * Persist symbiosis cache to file for next invocation
 */
function persistSymbiosisCache(cache) {
  try {
    const dir = path.join(homedir(), '.cynic');
    if (!existsSync(dir)) mkdirSync(dir, { recursive: true });
    writeFileSync(SYMBIOSIS_CACHE_FILE, JSON.stringify(cache));
  } catch { /* ignore */ }
}

const symbiosisCache = loadSymbiosisCache();

/**
 * Update symbiosis cache (called by async handlers)
 * @param {string} type - 'judgment' | 'consensus' | 'routing'
 * @param {Object} data - The data to cache
 */
function updateSymbiosisCache(type, data) {
  if (type === 'judgment' && data) {
    symbiosisCache.lastJudgment = {
      qScore: data.qScore ?? data.Q ?? data.score,
      verdict: data.verdict,
      confidence: data.confidence,
      axiomScores: data.axiomScores || data.breakdown,
      refined: data.refined,
    };
    symbiosisCache.updatedAt = Date.now();
  } else if (type === 'consensus' && data) {
    symbiosisCache.lastConsensus = {
      votes: data.agentResults || data.votes || [],
      leader: data.leader || data.dominantDog,
      consultations: data.consultations || [],
      blocked: data.blocked || false,
    };
    symbiosisCache.updatedAt = Date.now();
  } else if (type === 'routing' && data) {
    symbiosisCache.lastRouting = {
      path: data.path || [],
      entrySefirah: data.entrySefirah,
      decisions: data.decisions || [],
      escalations: data.escalations || [],
      blocked: data.blocked || false,
    };
    symbiosisCache.updatedAt = Date.now();
  }
}

import { createRequire } from 'module';
const requireCJS = createRequire(import.meta.url);

// Load colors module (still needed for other parts of observe.js)
let colors = null;
let ANSI = null;
let DOG_COLORS = null;

try {
  colors = requireCJS('../lib/colors.cjs');
  ANSI = colors.ANSI;
  DOG_COLORS = colors.DOG_COLORS;
} catch (e) {
  // Fallback ANSI codes
  ANSI = {
    reset: '\x1b[0m', bold: '\x1b[1m', dim: '\x1b[2m',
    red: '\x1b[31m', green: '\x1b[32m', yellow: '\x1b[33m', cyan: '\x1b[36m',
    brightRed: '\x1b[91m', brightGreen: '\x1b[92m', brightYellow: '\x1b[93m',
    brightCyan: '\x1b[96m', brightWhite: '\x1b[97m',
  };
  DOG_COLORS = {};
}

// Helper for colorizing
const c = (color, text) => color ? `${color}${text}${ANSI.reset}` : text;

// Self-Judge module for meta-awareness
let selfJudge = null;
try {
  selfJudge = requireCJS('../lib/self-judge.cjs');
} catch (e) {
  // Self-judge not available — meta-awareness disabled
  if (process.env.CYNIC_DEBUG) console.error('[OBSERVE] Self-judge module not available:', e.message);
}

// =============================================================================
// INLINE STATUS BAR - Now imported from ./lib/active-dog.js
// generateInlineStatus, getActiveDog, formatActiveDog are now imported
// =============================================================================

const PHI_INV = 0.618;

// =============================================================================
// P0.2: FACT EXTRACTION - Now imported from ./lib/fact-extractor-local.js
// extractFacts, storeFacts are now imported
// =============================================================================

// =============================================================================
// TRIGGER PROCESSOR - Now imported from ./lib/trigger-processor.js
// processTriggerEvent, extractCommitMessage are now imported
// =============================================================================

// =============================================================================
// PATTERN DETECTION
// =============================================================================

function detectToolPattern(toolName, toolInput, toolOutput, isError) {
  const patterns = [];

  // Error patterns
  if (isError) {
    const errorText = typeof toolOutput === 'string' ? toolOutput :
                      toolOutput?.error || toolOutput?.message || '';

    // Common error signatures
    if (errorText.includes('ENOENT')) {
      patterns.push({
        type: 'error',
        signature: 'file_not_found',
        description: 'File not found error',
        context: { tool: toolName }
      });
    } else if (errorText.includes('EACCES') || errorText.includes('Permission denied')) {
      patterns.push({
        type: 'error',
        signature: 'permission_denied',
        description: 'Permission denied error',
        context: { tool: toolName }
      });
    } else if (errorText.includes('ECONNREFUSED')) {
      patterns.push({
        type: 'error',
        signature: 'connection_refused',
        description: 'Connection refused - service not running?',
        context: { tool: toolName }
      });
    } else if (errorText.includes('SyntaxError')) {
      patterns.push({
        type: 'error',
        signature: 'syntax_error',
        description: 'Syntax error in code',
        context: { tool: toolName }
      });
    } else if (errorText.includes('TypeError')) {
      patterns.push({
        type: 'error',
        signature: 'type_error',
        description: 'Type error - check variable types',
        context: { tool: toolName }
      });
    }
  }

  // Tool usage patterns
  patterns.push({
    type: 'tool_usage',
    signature: toolName,
    description: `${toolName} usage`,
    context: {
      success: !isError,
      inputSize: JSON.stringify(toolInput).length
    }
  });

  // File extension patterns (for Write/Edit)
  if ((toolName === 'Write' || toolName === 'Edit') && toolInput.file_path) {
    const ext = path.extname(toolInput.file_path || toolInput.filePath || '');
    if (ext) {
      patterns.push({
        type: 'language_usage',
        signature: ext,
        description: `Working with ${ext} files`
      });
    }
  }

  // Git patterns
  if (toolName === 'Bash') {
    const command = toolInput.command || '';
    if (command.startsWith('git ')) {
      const gitCmd = command.split(' ')[1];
      patterns.push({
        type: 'git_usage',
        signature: `git_${gitCmd}`,
        description: `Git ${gitCmd} command`
      });
    }
  }

  return patterns;
}

function updateUserToolStats(profile, toolName, isError) {
  const commonTools = profile.patterns?.commonTools || {};
  commonTools[toolName] = (commonTools[toolName] || 0) + 1;

  const updates = {
    stats: {
      toolCalls: (profile.stats?.toolCalls || 0) + 1,
      errorsEncountered: (profile.stats?.errorsEncountered || 0) + (isError ? 1 : 0)
    },
    patterns: {
      commonTools
    }
  };

  // Track working hours
  const hour = new Date().getHours();
  const workingHours = profile.patterns?.workingHours || {};
  workingHours[hour] = (workingHours[hour] || 0) + 1;
  updates.patterns.workingHours = workingHours;

  return updates;
}

// =============================================================================
// AUTO-JUDGMENT TRIGGERS - Now imported from ./lib/trigger-processor.js
// processTriggerEvent, parseTestOutput, extractCommitHash, extractCommitMessage,
// extractErrorSummary are now imported
// =============================================================================

// Note: detectErrorType is now imported from ./lib/index.js (pattern-detector.js)

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
  try {
    // Read stdin - try sync first, fall back to async (ESM stdin fix)
    const fs = await import('fs');
    let input = '';

    try {
      input = fs.readFileSync(0, 'utf8');
      if (process.env.CYNIC_DEBUG) console.error('[OBSERVE] Sync read:', input.length, 'bytes');
    } catch (syncErr) {
      if (process.env.CYNIC_DEBUG) console.error('[OBSERVE] Sync failed:', syncErr.message);
      input = await new Promise((resolve) => {
        let data = '';
        process.stdin.setEncoding('utf8');
        process.stdin.on('data', chunk => { data += chunk; });
        process.stdin.on('end', () => resolve(data));
        process.stdin.on('error', () => resolve(''));
        process.stdin.resume();
        setTimeout(() => resolve(data), 3000);
      });
      if (process.env.CYNIC_DEBUG) console.error('[OBSERVE] Async read:', input.length, 'bytes');
    }

    if (!input || input.trim().length === 0) {
      safeOutput({ continue: true });
      return;
    }

    const hookContext = JSON.parse(input);
    const toolName = hookContext.tool_name || hookContext.toolName || '';
    const toolInput = hookContext.tool_input || hookContext.toolInput || {};
    const toolOutput = hookContext.tool_output || hookContext.toolOutput || {};
    const isError = hookContext.is_error || hookContext.isError || false;

    // Detect user and load profile
    const user = detectUser();
    const profile = loadUserProfile(user.userId);

    // ═══════════════════════════════════════════════════════════════════════════
    // TELEMETRY: Record tool usage for benchmarking and fine-tuning
    // "φ mesure tout, φ apprend de tout"
    // ═══════════════════════════════════════════════════════════════════════════
    const telemetry = getTelemetryCollector();
    if (telemetry) {
      // Record tool usage
      telemetry.recordToolUse({
        tool: toolName,
        success: !isError,
        error: isError ? (typeof toolOutput === 'string' ? toolOutput.slice(0, 200) : toolOutput?.error) : null,
      });

      // Record frictions for errors
      if (isError) {
        const errorText = typeof toolOutput === 'string' ? toolOutput : toolOutput?.error || '';
        const errorType = detectErrorType(errorText);
        const severity = errorType === 'connection_refused' ? 'high' :
                        errorType === 'permission_denied' ? 'high' :
                        errorType === 'syntax_error' ? 'medium' :
                        'low';
        recordFriction(`tool_error_${toolName}`, severity, {
          category: 'tool',
          tool: toolName,
          errorType,
          error: errorText.slice(0, 200),
        });
      }

      // Increment counters
      recordMetric('tool_calls_total', 1, { tool: toolName, category: 'tool' });
      if (isError) {
        recordMetric('tool_errors_total', 1, { tool: toolName, category: 'tool' });
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // META-AWARENESS: CYNIC observes its OWN code being modified
    // "Le chien se regarde dans le miroir" — Self-consciousness in real-time
    // Gap 1 Fix: CYNIC must know itself across all dimensions (25-dimension self-judgment)
    // ═══════════════════════════════════════════════════════════════════════════
    const filePath = toolInput.file_path || toolInput.path || '';
    const isSelfModification = (toolName === 'Edit' || toolName === 'Write') && filePath && (
      filePath.includes('/CYNIC/') ||
      filePath.includes('\\CYNIC\\') ||
      filePath.includes('/cynic/') ||
      filePath.includes('\\cynic\\') ||
      filePath.includes('packages/node/') ||
      filePath.includes('packages/mcp/') ||
      filePath.includes('packages/core/') ||
      filePath.includes('packages/persistence/') ||
      filePath.includes('scripts/hooks/')
    );

    if (isSelfModification && !isError && selfJudge) {
      try {
        // CYNIC is modifying its own code — trigger FULL 25-dimension self-judgment
        const content = toolInput.new_string || toolInput.content || '';
        const oldContent = toolInput.old_string || '';

        // Create self-judgment item
        const selfItem = selfJudge.createSelfJudgmentItem({
          filePath,
          content,
          toolName,
          oldContent,
        });

        // Judge the self-modification
        const selfJudgment = selfJudge.judgeSelfModification(selfItem);

        // ═══════════════════════════════════════════════════════════════════
        // FIX (Task #20): Store Q-Score for real reward calculation
        // This feeds into Q-Learning so CYNIC learns from self-modifications
        // ═══════════════════════════════════════════════════════════════════
        antiPatternState.lastSelfModScore = selfJudgment.qScore;

        // Output formatted judgment to stderr (visible to developer)
        const outputLines = selfJudge.formatJudgmentOutput(selfJudgment);
        console.error(outputLines.join('\n'));

        // Record telemetry
        if (telemetry) {
          recordMetric('self_modifications_total', 1, {
            component: selfItem.component.name,
            category: 'meta',
          });
          recordMetric('self_judgment_qscore', selfJudgment.qScore, {
            component: selfItem.component.name,
            verdict: selfJudgment.verdict.verdict,
            category: 'meta',
          });

          // Record critical risks as frictions
          const criticalRisks = selfItem.risks.filter(r => r.severity === 'high');
          if (criticalRisks.length > 0) {
            recordFriction('self_mod_fractal_risk', 'high', {
              category: 'meta',
              component: selfItem.component.name,
              risks: criticalRisks.map(r => r.risk).join(','),
              qScore: selfJudgment.qScore,
            });
          }
        }

        // ═══════════════════════════════════════════════════════════════════
        // LOG TO PATTERNS FOR LEARNING
        // Record self-judgment as a pattern for collective learning
        // "Le chien apprend de ses propres modifications"
        // ═══════════════════════════════════════════════════════════════════
        if (orchestrateFull) {
          try {
            orchestrateFull(
              `self_modification: ${selfItem.component.name} (Q:${selfJudgment.qScore})`,
              {
                type: 'self_modification',
                component: selfItem.component.name,
                domain: selfItem.component.domain,
                qScore: selfJudgment.qScore,
                verdict: selfJudgment.verdict.verdict,
                weakestAxiom: selfJudgment.weakest.axiom,
                weakestScore: selfJudgment.weakest.score,
                axiomScores: selfJudgment.axiomScores,
                risks: criticalRisks?.map(r => r.risk) || [],
                timestamp: Date.now(),
              }
            );
          } catch { /* best-effort pattern recording */ }
        }

        // If Q-Score is below threshold, add warning to antiPatternState
        if (selfJudgment.qScore < 50) {
          antiPatternState.selfModWarnings = antiPatternState.selfModWarnings || [];
          antiPatternState.selfModWarnings.push({
            file: path.basename(filePath),
            qScore: selfJudgment.qScore,
            verdict: selfJudgment.verdict.verdict,
            weakest: selfJudgment.weakest,
            risks: selfItem.criticalRiskCount,
            timestamp: Date.now(),
          });
        }

        // ═══════════════════════════════════════════════════════════════════
        // PHASE B1 (Task #21): Feed non-commutative evaluator
        // Track dimension evaluation order to detect order-dependent effects
        // ═══════════════════════════════════════════════════════════════════
        if (harmonicFeedback?.nonCommutativeEvaluator && selfJudgment.axiomScores) {
          try {
            // Get dimension names from axiom scores
            const dimensionOrder = Object.keys(selfJudgment.axiomScores);
            const dimensionScores = selfJudgment.axiomScores;

            // Record evaluation for non-commutativity tracking
            harmonicFeedback.recordDimensionEvaluation({
              order: dimensionOrder,
              scores: dimensionScores,
              finalScore: selfJudgment.qScore,
              context: `self_mod_${selfItem.component.name}`,
            });
          } catch { /* best-effort non-commutative tracking */ }
        }

      } catch (selfJudgeErr) {
        // Self-judge failed — fall back to simple detection
        debugError('OBSERVE', 'Self-judge failed', selfJudgeErr, 'warning');
        // Minimal output
        console.error(`🪞 SELF-MOD: ${path.basename(filePath)} (self-judge unavailable)`);
      }
    } else if (isSelfModification && !isError && !selfJudge) {
      // Self-judge module not loaded — minimal output
      console.error(`🪞 SELF-MOD: ${path.basename(filePath)} (25-dim judgment disabled)`);
      if (telemetry) {
        recordMetric('self_modifications_total', 1, { component: 'unknown', category: 'meta' });
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // JUDGMENT ID TRACKING: Capture IDs from brain_judge results for feedback
    // linkage. This enables the training pipeline to tie feedback (test/commit/
    // build outcomes) to specific judgments for supervised learning data.
    // "Le chien se souvient de son dernier jugement"
    // ═══════════════════════════════════════════════════════════════════════════
    if (toolName.includes('brain_judge') || toolName.includes('brain_cynic_judge')) {
      try {
        const outputText = typeof toolOutput === 'string' ? toolOutput : JSON.stringify(toolOutput || '');
        // Extract judgment_id from MCP tool output (format: jdg_xxxxx)
        const jidMatch = outputText.match(/jdg_[a-zA-Z0-9_]+/);
        if (jidMatch) {
          antiPatternState.lastJudgmentId = jidMatch[0];
          antiPatternState.lastJudgmentAt = Date.now();
        }
      } catch { /* best-effort — never block on tracking */ }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ORCHESTRATION: Full orchestration through UnifiedOrchestrator (Phase 21)
    // FIX: Store promises to await before output (was fire-and-forget → race condition)
    // FIX: requestJudgment: true (was isError → 90% of tool uses never judged)
    // "Le chien observe et rapporte au cerveau collectif"
    // ═══════════════════════════════════════════════════════════════════════════
    let orchestration = null;
    const orchestrationPromise = orchestrateFull(
      `${toolName}: ${isError ? 'ERROR' : 'SUCCESS'}`,
      {
        eventType: 'tool_result',  // Post-tool event
        requestJudgment: true,     // Always judge — CYNIC sees everything
        metadata: {
          tool: toolName,
          source: 'observe_hook',
          isError,
          outputLength: typeof toolOutput === 'string' ? toolOutput.length : JSON.stringify(toolOutput || {}).length,
          project: detectProject(),
        },
      }
    ).then(result => {
      orchestration = result;
      // Store decision ID for later reference
      if (result?.decisionId) {
        process.env.CYNIC_LAST_DECISION_ID = result.decisionId;
      }
      // SYMBIOSIS: Cache routing and judgment data for visibility (Task #4 + #5)
      if (result?.routing) {
        updateSymbiosisCache('routing', result.routing);
      }
      if (result?.judgment) {
        updateSymbiosisCache('judgment', result.judgment);
      }
      return result;
    }).catch(() => null);

    // ═══════════════════════════════════════════════════════════════════════════
    // AUTO-ORCHESTRATOR: Automatic Dog consultation via CollectivePack
    // FIX: Store promise to await before output (was fire-and-forget)
    // "Le collectif analyse" - All 11 Dogs see every tool result
    // ═══════════════════════════════════════════════════════════════════════════
    const autoOrchestrator = getAutoOrchestratorSync();
    const autoOrchestratorPromise = autoOrchestrator.postAnalyze({
      tool: toolName,
      input: toolInput,
      output: toolOutput,
      duration: 0,  // Not tracked in PostToolUse
      success: !isError,
      userId: detectUser()?.id,
      sessionId: process.env.CYNIC_SESSION_ID,
    }).then(result => {
      // Log patterns/anomalies detected by Dogs
      if (result?.anomalies?.length > 0) {
        for (const anomaly of result.anomalies) {
          console.error(`[${anomaly.agent}] ${anomaly.type}: ${anomaly.message}`);
        }
      }
      // SYMBIOSIS: Cache consensus data for visibility (Task #5)
      // AgentResults contain individual Dog responses
      if (result?.agentResults || result?.anomalies) {
        updateSymbiosisCache('consensus', {
          agentResults: result.agentResults || [],
          votes: result.agentResults?.map(r => ({
            dog: r.agent,
            vote: r.response,
            support: r.confidence || 0.5,
          })) || [],
          anomalies: result.anomalies || [],
          leader: result.leader || (result.agentResults?.[0]?.agent),
        });
      }
      return result;
    }).catch(() => null);

    // ═══════════════════════════════════════════════════════════════════════════
    // Q-LEARNING: Record episode for learning pipeline (Task #84 + Task #20)
    // "Le chien apprend qui appeler" - Q-Learning feeds weight optimization
    // FIX: Now uses REAL rewards from self-judge, execution time, and outcomes
    // ═══════════════════════════════════════════════════════════════════════════
    const qlearning = getQLearningServiceWithPersistence();
    if (qlearning) {
      try {
        // Determine task type from context
        const taskType =
          isError ? 'debug' :
          toolName === 'Bash' && toolInput.command?.match(/test|jest|vitest/i) ? 'test' :
          toolName === 'Bash' && toolInput.command?.match(/deploy|publish/i) ? 'deployment' :
          toolName === 'Bash' && toolInput.command?.startsWith('git ') ? 'exploration' :
          (toolName === 'Write' || toolName === 'Edit') ? 'code_change' :
          (toolName === 'Read' || toolName === 'Glob' || toolName === 'Grep') ? 'exploration' :
          toolName === 'Task' ? 'analysis' :
          'exploration';

        // Start episode
        qlearning.startEpisode({
          taskType,
          tool: toolName,
          content: toolInput.command || toolInput.file_path || '',
          isError,
        });

        // Record which dog should handle this
        const activeDog = getActiveDog(toolName, toolInput, isError);
        if (activeDog) {
          qlearning.recordAction(activeDog.name.toLowerCase(), {
            tool: toolName,
            success: !isError,
            source: 'observe_hook',
          });
        }

        // ═══════════════════════════════════════════════════════════════════
        // REAL REWARD CALCULATION (Task #20)
        // φ-aligned: Combines multiple signals into a nuanced score
        // ═══════════════════════════════════════════════════════════════════
        const realRewardScore = calculateRealReward({
          isError,
          toolName,
          toolInput,
          toolOutput,
          // Self-judge score: self-modification score OR previous invocation's real Judge Q-Score
          // FIX: symbiosisCache.lastJudgment has the REAL 25-dim Judge score from N-1 invocation
          // Lagging indicator, but contextually meaningful within a session
          selfJudgeScore: antiPatternState.lastSelfModScore || symbiosisCache.lastJudgment?.qScore || null,
          // Execution duration if tracked
          durationMs: toolOutput?.duration_ms || antiPatternState.lastToolDuration || null,
          // Pattern match bonus
          patternMatch: antiPatternState.matchedPattern || null,
          // Blocked by Guardian = positive reward
          wasBlocked: antiPatternState.blockedByGuardian || false,
        });

        // End episode with REAL outcome score (not just success/failure)
        qlearning.endEpisode({
          success: !isError,
          type: isError ? 'failure' : 'success',
          tool: toolName,
          score: realRewardScore, // FIX: Pass real score (0-100), converted by learning-service
        });

        // Clear transient state
        antiPatternState.lastSelfModScore = null;
        antiPatternState.lastToolDuration = null;
        antiPatternState.matchedPattern = null;
        antiPatternState.blockedByGuardian = false;

      } catch (e) {
        // Q-Learning recording failed - continue without
        debugError('OBSERVE', 'Q-Learning recording failed', e, 'warning');
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // IMPLICIT BRAIN TOOLS: Auto-activation of dormant tools
    // "Les outils dormants deviennent réflexes"
    // ═══════════════════════════════════════════════════════════════════════════

    // brain_sona: SONA dimension adaptation (after significant decisions)
    // Triggers: sessionDurationMin > 30, significantDecisions >= 3
    const sessionStart = parseInt(process.env.CYNIC_SESSION_START || '0', 10);
    const sessionDurationMin = sessionStart ? (Date.now() - sessionStart) / 60000 : 0;
    const significantDecisions = parseInt(process.env.CYNIC_SIGNIFICANT_DECISIONS || '0', 10);

    if (sessionDurationMin > 30 || significantDecisions >= 3) {
      try {
        // Only call periodically (every 10 tool calls) to avoid overhead
        const toolCallCount = parseInt(process.env.CYNIC_TOOL_CALL_COUNT || '0', 10) + 1;
        process.env.CYNIC_TOOL_CALL_COUNT = String(toolCallCount);

        if (toolCallCount % 10 === 0) {
          callBrainTool('brain_sona', {
            action: 'correlate',
            sessionDuration: sessionDurationMin,
            significantDecisions,
            silent: true,
          }).catch(() => {
            // Silent failure - non-blocking
          });
        }
      } catch (e) {
        // Silent failure
      }
    }

    // brain_metrics: Record session metrics (implicit, on significant work)
    if (isError || toolName === 'Bash' || toolName === 'Write' || toolName === 'Edit') {
      try {
        callBrainTool('brain_metrics', {
          action: 'record',
          metric: 'tool_activity',
          tool: toolName,
          success: !isError,
          timestamp: Date.now(),
          silent: true,
        }).catch(() => {
          // Silent failure
        });
      } catch (e) {
        // Silent failure
      }
    }

    // Track significant decisions (errors, writes, edits)
    if (isError || toolName === 'Write' || toolName === 'Edit') {
      process.env.CYNIC_SIGNIFICANT_DECISIONS = String(significantDecisions + 1);
    }

    // Detect patterns
    const patterns = detectToolPattern(toolName, toolInput, toolOutput, isError);

    // Save patterns to local collective AND session state (for cross-session persistence)
    // "Le chien se souvient de tout" - patterns survive across sessions via PostgreSQL
    const sessionState = getSessionState();
    for (const pattern of patterns) {
      saveCollectivePattern(pattern);
      // Also record to session state for persistence via sleep.js → PostgreSQL
      if (sessionState.isInitialized()) {
        sessionState.recordPattern(pattern);
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // P0.2: FACT EXTRACTION - Extract and store facts for semantic retrieval
    // "Le chien extrait les faits" - Facts are discrete knowledge units
    // ═══════════════════════════════════════════════════════════════════════════
    const facts = extractFacts(toolName, toolInput, toolOutput, isError);
    if (facts.length > 0) {
      // Store facts asynchronously (non-blocking)
      storeFacts(facts, {
        tool: toolName,
        project: detectProject(),
        userId: user.userId,
      }).catch(() => {
        // Fact storage failed - continue without
      });

      // Record fact extraction pattern for monitoring
      const factPattern = {
        type: 'fact_extraction',
        signature: `facts_${toolName}`,
        description: `Extracted ${facts.length} facts from ${toolName}`,
        context: { factTypes: facts.map(f => f.type) },
      };
      saveCollectivePattern(factPattern);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // M2: PERSISTENT FACT EXTRACTION - Store facts to PostgreSQL via FactExtractor
    // "Le chien se souvient de tout" - Facts survive across sessions
    // ═══════════════════════════════════════════════════════════════════════════
    const factExtractor = getFactExtractor();
    if (factExtractor) {
      try {
        // Extract and persist facts using the FactExtractor service
        const outputStr = typeof toolOutput === 'string' ? toolOutput : JSON.stringify(toolOutput || {});
        factExtractor.extract({
          tool: toolName,
          input: toolInput,
          output: outputStr,
        }).then(persistedFacts => {
          if (persistedFacts.length > 0 && process.env.CYNIC_DEBUG) {
            console.error(`[OBSERVE] M2: Persisted ${persistedFacts.length} facts to PostgreSQL`);
          }
        }).catch(() => {
          // Fact persistence failed - continue without
        });

        // Extract error resolution if this was an error that got resolved
        if (!isError && antiPatternState.recentErrors.length > 0) {
          const lastError = antiPatternState.recentErrors[antiPatternState.recentErrors.length - 1];
          if (lastError && Date.now() - lastError.timestamp < 60000) { // Within 1 minute
            factExtractor.extractErrorResolution({
              error: lastError.type,
              solution: `Used ${toolName} successfully`,
              tool: toolName,
              file: toolInput.file_path || toolInput.filePath,
            }).catch(() => {});
          }
        }
      } catch (e) {
        // FactExtractor failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // P1.2: TRAJECTORY LEARNING - Store state → action → outcome for replay
    // "Le chien apprend des chemins" - Learn from successful paths
    // ═══════════════════════════════════════════════════════════════════════════
    const reasoningBank = getReasoningBank();
    if (reasoningBank) {
      try {
        // Determine trajectory type from tool
        const trajectoryType =
          toolName === 'Bash' && toolInput.command?.match(/test|jest|vitest/i) ? 'problem' :
          (toolName === 'Write' || toolName === 'Edit') ? 'code' :
          toolName === 'Bash' && toolInput.command?.startsWith('git ') ? 'deployment' :
          isError ? 'recovery' :
          'judgment';

        // Create trajectory from this tool use
        const trajectory = reasoningBank.startTrajectory(trajectoryType, {
          tool: toolName,
          inputSummary: JSON.stringify(toolInput).substring(0, 200),
          project: detectProject(),
          timestamp: Date.now(),
        });

        // Add action
        trajectory.addAction({
          type: toolName.toLowerCase(),
          tool: toolName,
          input: {
            file: toolInput.file_path || toolInput.filePath,
            command: toolInput.command?.substring(0, 100),
          },
          dog: getActiveDog(toolName, toolInput, isError)?.name,
          confidence: isError ? 0.3 : 0.7,
        });

        // Set outcome
        trajectory.setOutcome({
          type: isError ? 'failure' : 'success',
          success: !isError,
          error: isError ? (typeof toolOutput === 'string' ? toolOutput.substring(0, 200) : toolOutput?.error) : null,
          metrics: {
            successRate: isError ? 0 : 1,
            outputSize: typeof toolOutput === 'string' ? toolOutput.length : JSON.stringify(toolOutput || {}).length,
          },
        });

        // Store successful trajectories (async, non-blocking)
        if (!isError) {
          trajectory.userId = user.userId;
          trajectory.projectId = detectProject()?.name;
          reasoningBank.store(trajectory).catch(() => {});
        }

        // Check for replay suggestions on errors
        if (isError) {
          const suggestions = reasoningBank.getSuggestions({
            type: 'error',
            content: {
              tool: toolName,
              error: typeof toolOutput === 'string' ? toolOutput.substring(0, 100) : 'error',
            },
          }, { limit: 2, minReward: 0.5 });

          if (suggestions.length > 0) {
            // Store suggestion for later output
            process.env.CYNIC_TRAJECTORY_SUGGESTION = JSON.stringify(suggestions[0].suggestion);
          }
        }
      } catch (e) {
        // Trajectory learning failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ANTI-PATTERN DETECTION - Error loops & bad workflows
    // "Le chien détecte les mauvaises habitudes"
    // ═══════════════════════════════════════════════════════════════════════════
    const antiPatternWarnings = detectAntiPatterns(toolName, toolInput, isError, toolOutput);

    // Output anti-pattern warnings immediately (high priority)
    if (antiPatternWarnings.length > 0) {
      const highPriority = antiPatternWarnings.filter(w => w.severity === 'high');
      if (highPriority.length > 0) {
        // Error loops are critical - output immediately
        const warning = highPriority[0];
        safeOutput({
          continue: true,
          message: `\n┌─────────────────────────────────────────────────────────┐\n│ 🛡️ GUARDIAN - ANTI-PATTERN DÉTECTÉ                      │\n├─────────────────────────────────────────────────────────┤\n│ ${warning.message.padEnd(55)} │\n│ 💡 ${warning.suggestion.padEnd(52)} │\n└─────────────────────────────────────────────────────────┘\n`,
        });
        return;
      }

      // Medium/low priority - record for later
      for (const warning of antiPatternWarnings) {
        if (consciousness) {
          consciousness.recordInsight({
            type: 'anti_pattern',
            title: warning.type,
            message: warning.message,
            data: warning.data,
            priority: warning.severity === 'medium' ? 'medium' : 'low',
          });
        }
        const antiPattern = {
          type: `antipattern_${warning.type}`,
          signature: warning.type,
          description: warning.message,
        };
        saveCollectivePattern(antiPattern);
        // Also record to session state for cross-session persistence
        if (sessionState.isInitialized()) {
          sessionState.recordPattern(antiPattern);
        }
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 22: Feedback Collection - Record outcome for learning
    // "Le chien se souvient des erreurs pour mieux protéger"
    // ═══════════════════════════════════════════════════════════════════════════
    let feedbackResult = null;
    let proactiveSuggestion = null;

    if (feedbackCollector) {
      try {
        // Determine error type if this was an error
        let errorType = null;
        let errorMessage = null;
        if (isError) {
          const outputText = typeof toolOutput === 'string' ? toolOutput :
                            toolOutput?.error || toolOutput?.message || '';
          errorType = detectErrorType(outputText);
          errorMessage = outputText.slice(0, 200);
        }

        // Record the outcome
        feedbackResult = feedbackCollector.record(toolName, {
          success: !isError,
          errorType,
          errorMessage,
          input: toolInput,
        });

        // Check if we should emit a proactive suggestion
        if (suggestionEngine && feedbackResult.antiPattern) {
          proactiveSuggestion = suggestionEngine.generateSuggestion();
        }
      } catch (e) {
        // Feedback collection failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 23: HARMONIC FEEDBACK - Kabbalah + CIA + Cybernetics + Thompson
    // "Ohr descends, Kelim receives" - Light flows, vessels learn
    // Detects implicit feedback from user actions and updates Thompson Samplers
    // ═══════════════════════════════════════════════════════════════════════════
    if (harmonicFeedback && implicitFeedback) {
      try {
        // 1. Observe user action for implicit feedback detection
        implicitFeedback.observeAction({
          type: toolName.toLowerCase(),
          tool: toolName,
          file: toolInput.file_path || toolInput.filePath,
          command: toolInput.command,
          success: !isError,
          timestamp: Date.now(),
        });

        // ═══════════════════════════════════════════════════════════════════════
        // FIL 1 (Task #83): Wire EVERY tool observation to harmonic feedback
        // Previously only processed feedback when suggestions were followed/ignored
        // Now: Thompson Sampling learns from ALL tool outcomes
        // ═══════════════════════════════════════════════════════════════════════
        harmonicFeedback.processFeedback({
          type: `tool_${toolName.toLowerCase()}`,
          sentiment: isError ? 'negative' : 'positive',
          confidence: isError ? 0.7 : 0.5, // Errors are clearer signals
          source: 'tool_observation',
          action: { tool: toolName, success: !isError, file: toolInput.file_path },
        });

        // 2. Detect implicit feedback from recent suggestions
        const detectedFeedback = implicitFeedback.detectFeedback();

        if (detectedFeedback.length > 0) {
          for (const fb of detectedFeedback) {
            // 3. Process through harmonic system (Thompson + Sefirot + CIA)
            harmonicFeedback.processFeedback({
              type: fb.type,
              sentiment: fb.sentiment,
              suggestionId: fb.suggestionId,
              confidence: fb.confidence,
              action: fb.userAction,
              source: 'implicit',
            });

            // 4. Record for telemetry
            if (telemetry) {
              recordMetric('implicit_feedback_detected', 1, {
                type: fb.type,
                sentiment: fb.sentiment,
              });
            }

            // 5. Record pattern for learning
            const feedbackPattern = {
              type: 'implicit_feedback',
              signature: `${fb.type}_${fb.sentiment}`,
              description: `User ${fb.sentiment === 'positive' ? 'followed' : 'ignored'} suggestion`,
              context: { suggestionType: fb.suggestionType, confidence: fb.confidence },
            };
            saveCollectivePattern(feedbackPattern);
            if (sessionState.isInitialized()) {
              sessionState.recordPattern(feedbackPattern);
            }
          }
        }

        // 6. When suggestions are made, record them for later feedback tracking
        if (proactiveSuggestion) {
          implicitFeedback.recordSuggestion({
            id: proactiveSuggestion.id || `sug_${Date.now()}`,
            type: proactiveSuggestion.type,
            content: proactiveSuggestion.message,
            timestamp: Date.now(),
            context: { tool: toolName, isError },
          });
        }

        // 7. Get harmonic state for consciousness integration
        const harmonicState = harmonicFeedback.getState();
        if (consciousness && harmonicState.coherence > 0.5) {
          // Record high coherence moments as insights
          if (harmonicState.coherence > 0.7) {
            consciousness.recordInsight({
              type: 'harmonic_coherence',
              title: 'High feedback coherence detected',
              message: `Coherence: ${Math.round(harmonicState.coherence * 100)}%, Resonance: ${Math.round(harmonicState.resonance * 100)}%`,
              data: harmonicState,
              priority: 'low',
            });
          }
        }

        // ═══════════════════════════════════════════════════════════════════════
        // PHASE B1 (Task #21): Feed mathematical systems for 95% feedback loop
        // FFT temporal + Antifragility + Girsanov + Non-commutative
        // ═══════════════════════════════════════════════════════════════════════

        // 8. Feed Antifragility tracker with volatility signals
        // Volatility = combination of recent errors, tool complexity, model uncertainty
        if (harmonicFeedback.antifragilityTracker) {
          const errorRate = antiPatternState.recentErrors.length > 0
            ? antiPatternState.recentErrors.filter(e => Date.now() - e.timestamp < 5 * 60000).length / 10
            : 0;
          const toolComplexity = ['Bash', 'Write', 'Edit'].includes(toolName) ? 0.7 : 0.3;
          const uncertainty = harmonicFeedback.thompsonSampler?.getUncertainty?.(`tool_${toolName.toLowerCase()}`) * 4 || 0.5;

          harmonicFeedback.antifragilityTracker.record(
            isError ? 0 : 1,  // Performance: 1 = success, 0 = failure
            { errorRate, complexity: toolComplexity, uncertainty }
          );
        }

        // 9. Get Girsanov adaptive confidence for context-aware decisions
        // Use risk-averse measure for dangerous tools
        if (harmonicFeedback.girsanovTransformer) {
          const isDangerous = ['Bash', 'Write', 'Edit'].includes(toolName);
          const rawConfidence = harmonicFeedback.thompsonSampler?.getExpectedValue?.(`tool_${toolName.toLowerCase()}`) || 0.5;
          const adaptiveConf = harmonicFeedback.getAdaptiveConfidence?.(rawConfidence, { isDangerous });

          // Store for telemetry
          if (telemetry && adaptiveConf !== undefined) {
            recordMetric('girsanov_adaptive_confidence', adaptiveConf, { tool: toolName, isDangerous });
          }
        }

        // 10. Check temporal patterns and optimal timing
        if (harmonicFeedback.temporalAnalyzer?.analysisCount > 0) {
          const timing = harmonicFeedback.getOptimalTiming?.();
          if (timing?.recommendation === 'rest_recommended' && !isError) {
            // User is working during low energy - note for future suggestions
            if (sessionState.isInitialized()) {
              sessionState.recordPattern({
                type: 'timing_insight',
                signature: 'working_during_low_energy',
                description: 'User active during predicted low energy period',
                context: timing,
              });
            }
          }
        }

        // 11. Antifragility insights for consciousness
        if (harmonicFeedback.antifragilityTracker?.metrics?.trend && consciousness) {
          const afMetrics = harmonicFeedback.antifragilityTracker.metrics;
          if (afMetrics.trend === 'antifragile' && afMetrics.index > 0.2) {
            consciousness.recordInsight({
              type: 'antifragility',
              title: 'System becoming antifragile',
              message: `Antifragility index: ${afMetrics.index.toFixed(3)} - gaining from stress`,
              data: afMetrics,
              priority: 'low',
            });
          } else if (afMetrics.trend === 'fragile' && afMetrics.index < -0.2) {
            consciousness.recordInsight({
              type: 'fragility_warning',
              title: 'System showing fragility',
              message: `Fragility index: ${afMetrics.index.toFixed(3)} - consider reducing stress`,
              data: afMetrics,
              priority: 'medium',
            });
          }
        }

      } catch (e) {
        // Harmonic feedback failed - continue without
        debugError('OBSERVE', 'Harmonic feedback failed', e, 'warning');
      }
    }

    // Track todos for Task Continuation Enforcer
    if (enforcer && toolName === 'TodoWrite' && toolInput.todos) {
      const sessionId = process.env.CYNIC_SESSION_ID || hookContext.session_id || 'default';
      enforcer.updateTodosFromTool(sessionId, toolInput.todos);
    }

    // Update user profile stats
    const updates = updateUserToolStats(profile, toolName, isError);
    updateUserProfile(profile, updates);

    // ═══════════════════════════════════════════════════════════════════════════
    // CONSCIOUSNESS: Track tool usage and detect human skills
    // ═══════════════════════════════════════════════════════════════════════════
    if (consciousness) {
      try {
        // Record tool usage in capability map
        consciousness.recordToolUsage(toolName, !isError, 0);

        // Detect human skills based on tool usage patterns
        if (toolName === 'Write' || toolName === 'Edit') {
          const filePath = toolInput.file_path || toolInput.filePath || '';
          const ext = path.extname(filePath).slice(1);

          // Map extensions to skills
          const skillMap = {
            'ts': 'typescript', 'tsx': 'typescript',
            'js': 'javascript', 'jsx': 'javascript',
            'py': 'python',
            'rs': 'rust',
            'go': 'go',
            'sol': 'solidity',
            'css': 'css', 'scss': 'css',
            'html': 'html',
            'json': 'json-config',
            'yaml': 'yaml-config', 'yml': 'yaml-config',
            'md': 'documentation',
            'sql': 'sql',
          };

          if (skillMap[ext]) {
            consciousness.observeHumanSkill(skillMap[ext], { type: 'code_edit' });
          }
        }

        // Detect git skills
        if (toolName === 'Bash' && toolInput.command?.startsWith('git ')) {
          consciousness.observeHumanSkill('git', { type: 'command' });
        }

        // Record working hour pattern
        const hour = new Date().getHours();
        consciousness.observeHumanPattern('lastActiveHour', hour);
      } catch (e) {
        // Consciousness tracking failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ENTANGLEMENT: Pattern Correlation Prediction
    // "Σύμπλεξις - bound together across space"
    // ═══════════════════════════════════════════════════════════════════════════
    const entanglementPredictions = [];
    if (physicsBridge) {
      try {
        // Observe the tool pattern
        for (const pattern of patterns) {
          const obsResult = physicsBridge.observePattern(
            pattern.signature,
            toolName,
            { type: pattern.type, isError }
          );

          // Collect predictions for related patterns
          if (obsResult.predictions && obsResult.predictions.length > 0) {
            entanglementPredictions.push(...obsResult.predictions);
          }

          // If new entanglements were created, note them
          if (obsResult.newPairs && obsResult.newPairs.length > 0) {
            for (const pair of obsResult.newPairs) {
              // Record to consciousness if available
              if (consciousness) {
                consciousness.recordInsight({
                  type: 'entanglement',
                  title: 'Pattern entanglement detected',
                  message: `${pair.patterns[0]} ⊗ ${pair.patterns[1]} (${Math.round(pair.correlation * 100)}%)`,
                  data: pair,
                  priority: 'low',
                });
              }
            }
          }
        }

        // Get predictions for what might appear next
        if (entanglementPredictions.length > 0) {
          // Sort by probability and take top 3
          const topPredictions = entanglementPredictions
            .sort((a, b) => b.probability - a.probability)
            .slice(0, 3);

          // Store for later use (e.g., proactive warnings)
          process.env.CYNIC_PREDICTED_PATTERNS = JSON.stringify(topPredictions);
        }
      } catch (e) {
        // Entanglement observation failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // COGNITIVE THERMODYNAMICS: Track heat/work/efficiency (Phase 10A)
    // "Ἐνέργεια - First Law: Energy is conserved"
    // ═══════════════════════════════════════════════════════════════════════════
    if (thermodynamics) {
      try {
        if (isError) {
          // Errors generate heat (frustration)
          // Map error types to thermodynamics events
          const errorType = detectErrorType(typeof toolOutput === 'string' ? toolOutput : '');
          const heatEventType = errorType === 'timeout' ? 'timeout' :
                                errorType === 'permission_denied' ? 'blocked' :
                                errorType === 'syntax_error' ? 'confusion' :
                                'error';
          thermodynamics.recordHeatEvent(heatEventType, { tool: toolName, errorType });
        } else {
          // Successful actions produce work
          // Map tool types to work events
          const workEventType = toolName === 'Write' ? 'codeWritten' :
                                toolName === 'Edit' ? 'codeWritten' :
                                toolName === 'Bash' && toolOutput?.includes?.('commit') ? 'commitMade' :
                                toolName === 'Bash' && toolOutput?.includes?.('PASS') ? 'testPassed' :
                                'questionAnswered';
          const magnitude = toolName === 'Write' || toolName === 'Edit' ? 1.5 : 1.0;
          thermodynamics.recordWorkEvent(workEventType, { tool: toolName, magnitude });
        }
      } catch (e) {
        // Thermodynamics tracking failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PSYCHOLOGY SIGNALS: Derive psychological state from tool outcomes
    // "Le chien comprend l'humain" — burnout detection, energy tracking
    // ═══════════════════════════════════════════════════════════════════════════
    if (psychology) {
      try {
        if (isError) {
          // Errors increase frustration, decrease energy
          const errorText = typeof toolOutput === 'string' ? toolOutput :
                            toolOutput?.error || toolOutput?.message || '';
          const errorType = detectErrorType(errorText);

          // Check for repeated failures (strong burnout signal)
          const recentErrorCount = antiPatternState.recentErrors.filter(
            e => Date.now() - e.timestamp < 5 * 60000
          ).length;

          if (recentErrorCount >= 3) {
            psychology.processSignal({
              type: 'repeated_failure',
              confidence: 0.5,
              context: { tool: toolName, errorType, count: recentErrorCount },
            });
          } else {
            psychology.processSignal({
              type: 'action_failure',
              confidence: 0.382,
              context: { tool: toolName, errorType },
            });
          }
        } else {
          // Successful actions boost energy and confidence
          psychology.processSignal({
            type: 'action_success',
            confidence: 0.382,
            context: { tool: toolName },
          });
        }

        // Derive energy from thermodynamics if available
        if (thermodynamics) {
          const thermoState = thermodynamics.getState();
          if (thermoState && thermoState.temperature > 50) {
            // High temperature = cognitive overload signal
            psychology.processSignal({
              type: 'high_cognitive_load',
              confidence: 0.382,
              context: { temperature: thermoState.temperature },
            });
          }
        }
      } catch (e) {
        // Psychology signal processing failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SYMBIOSIS LAYER (C5.*): Human wellbeing analysis
    // "Le chien protège son humain" — proactive care and intervention
    // ═══════════════════════════════════════════════════════════════════════════
    const humanAdvisor = null; // TODO: implement getHumanAdvisor in lib
    const humanAccountant = null;
    const humanLearning = null;

    if (humanAdvisor && psychology) {
      try {
        const psyState = psychology.getState() || {};
        const thermoState = thermodynamics?.getState?.() || {};

        // Map psychology state to HumanAdvisor format
        const humanState = {
          energy: psyState.energy || 0.5,
          focus: psyState.focus || 0.5,
          cognitiveLoad: thermoState.temperature ? Math.min(9, thermoState.temperature / 10) : 5,
          frustration: psyState.frustration || 0,
        };

        // Analyze and check for intervention
        const intervention = humanAdvisor.analyze(humanState, {
          toolName,
          isError,
          sessionDuration: Date.now() - (psychology._sessionStart || Date.now()),
        });

        if (intervention) {
          // Emit intervention for TUI display
          symbiosisCache.intervention = intervention;

          // Log for awareness
          DC.log('debug', '[SYMBIOSIS]', `Intervention: ${intervention.type} (${intervention.urgency})`);
        }

        // Track human activity
        if (humanAccountant) {
          humanAccountant.recordActivity({
            type: isError ? 'error' : 'success',
            tool: toolName,
            timestamp: Date.now(),
          });
        }

        // Track human skills from tool usage
        if (humanLearning && !isError) {
          const skillMap = {
            Write: 'file_creation',
            Edit: 'file_editing',
            Bash: 'command_line',
            Grep: 'code_search',
            Read: 'code_reading',
          };
          if (skillMap[toolName]) {
            humanLearning.recordSkillUsage(skillMap[toolName], { success: true });
          }
        }
      } catch (e) {
        // Symbiosis analysis failed - continue without
        DC.log('debug', '[SYMBIOSIS]', `Analysis error: ${e.message}`);
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REFLECTION MEMORY (System 2): Learn from own errors
    // "Le chien apprend de ses erreurs" — Self-correction detection
    // Detect: retry patterns (error → same tool → success) and repeated failures
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const factExtractorForReflection = getFactExtractor();
      if (factExtractorForReflection?.factsRepo) {
        const factsRepo = factExtractorForReflection.factsRepo;

        // Detect retry patterns (error → same tool → success = self-correction)
        if (!isError && antiPatternState.recentErrors.length > 0) {
          const lastError = antiPatternState.recentErrors[antiPatternState.recentErrors.length - 1];
          // Same tool that previously failed now succeeded within 2 minutes
          if (lastError && lastError.file && Date.now() - lastError.timestamp < 120000) {
            factsRepo.create({
              factType: 'reflection',
              subject: `Self-correction: ${toolName}`,
              content: `Failed then succeeded with ${toolName}. Previous error: ${lastError.type}. File: ${path.basename(lastError.file || '')}`,
              confidence: 0.5,
              userId: user.userId,
              tags: ['retry', 'self_correction', toolName],
            }).catch(() => { /* reflection is optional */ });
          }
        }

        // Detect repeated failures (3+ errors in 5min → high frustration signal)
        if (isError) {
          const recentErrorCount = antiPatternState.recentErrors.filter(
            e => Date.now() - e.timestamp < 5 * 60000
          ).length;

          if (recentErrorCount >= 3) {
            const errorText = typeof toolOutput === 'string' ? toolOutput : toolOutput?.error || '';
            const errorType = detectErrorType(errorText);
            factsRepo.create({
              factType: 'reflection',
              subject: `Pattern: repeated ${toolName} failures`,
              content: `${recentErrorCount} failures in 5min. Possible cause: ${errorType}. Consider different approach.`,
              confidence: 0.382,
              userId: user.userId,
              tags: ['repeated_failure', toolName],
            }).catch(() => { /* reflection is optional */ });
          }
        }
      }
    } catch { /* reflection is optional */ }

    // ═══════════════════════════════════════════════════════════════════════════
    // EVENT LEDGER: Append-only session event log for continuity
    // "Le chien trace chaque pas" - Enables handoff between sessions
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const { getEventLedger } = await import('../lib/event-ledger.js');
      const ledger = getEventLedger();
      const filePath = toolInput.file_path || toolInput.filePath || toolInput.path || null;
      ledger.append({
        type: isError ? 'ERROR' : 'TOOL_CALL',
        tool: toolName,
        sessionId: process.env.CYNIC_SESSION_ID,
        ...(filePath && { file: filePath }),
        ...(isError && { summary: (typeof toolOutput === 'string' ? toolOutput : toolOutput?.error || '')?.substring(0, 120) }),
      });
    } catch { /* ledger is optional */ }

    // ═══════════════════════════════════════════════════════════════════════════
    // COSMOPOLITAN LEARNING: Share patterns to collective (Phase 6C)
    // "Κοσμοπολίτης - learn from the world, share with the world"
    // ═══════════════════════════════════════════════════════════════════════════
    if (cosmopolitan && !isError) {
      try {
        // Only share patterns if user opted in
        if (cosmopolitan.isOptedIn()) {
          // Share successful tool patterns
          for (const pattern of patterns) {
            if (pattern.type !== 'error') {
              cosmopolitan.recordLocalPattern(pattern);
            }
          }

          // Check if it's time to sync (non-blocking)
          if (cosmopolitan.shouldSync()) {
            setImmediate(() => {
              try {
                cosmopolitan.sync().catch(() => {});
              } catch (e) { /* ignore */ }
            });
          }
        }
      } catch (e) {
        // Cosmopolitan learning failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // VOLUNTARY POVERTY: Celebrate deletions (Phase 10C)
    // "Diogène a jeté sa tasse - moins c'est plus"
    // ═══════════════════════════════════════════════════════════════════════════
    let deletionCelebration = null;
    if (voluntaryPoverty && !isError) {
      try {
        // Track deletions from git rm or rm commands
        if (toolName === 'Bash') {
          const command = toolInput.command || '';
          if (command.match(/\bgit\s+rm\b|\brm\s+/)) {
            voluntaryPoverty.recordDeletion('file', 1);
            deletionCelebration = voluntaryPoverty.getCelebration('file_deleted');
          }
        }

        // Track line deletions in Edit (when old_string > new_string)
        if (toolName === 'Edit') {
          const oldString = toolInput.old_string || '';
          const newString = toolInput.new_string || '';
          const linesDiff = (oldString.match(/\n/g) || []).length - (newString.match(/\n/g) || []).length;

          if (linesDiff > 0) {
            voluntaryPoverty.recordDeletion('lines', linesDiff);
            // Celebrate significant deletions (>5 lines)
            if (linesDiff > 5) {
              deletionCelebration = voluntaryPoverty.getCelebration('lines_deleted', linesDiff);
            }
          }
        }

        // Track successful refactoring (simplification)
        const ratio = voluntaryPoverty.getAdditionDeletionRatio();
        if (ratio && ratio < 1) {
          // More deletions than additions - this is good!
          voluntaryPoverty.recordSimplification();
        }
      } catch (e) {
        // Voluntary poverty tracking failed - continue without
      }
    }

    // Send to MCP server (non-blocking) - include decision tracing
    sendHookToCollectiveSync('PostToolUse', {
      toolName,
      isError,
      patterns,
      inputSize: JSON.stringify(toolInput).length,
      timestamp: Date.now(),
      // Phase 21: Include orchestration tracing
      decisionId: orchestration?.decisionId,
      qScore: orchestration?.judgment?.qScore,
    });

    // Process through Auto-Judgment Triggers (non-blocking)
    // This enables automatic judgments on errors, commits, decisions, etc.
    processTriggerEvent(toolName, toolInput, toolOutput, isError);

    // ═══════════════════════════════════════════════════════════════════════════
    // SIGNAL COLLECTOR: Feed psychological state tracking
    // "Observer pour comprendre, comprendre pour aider"
    // ═══════════════════════════════════════════════════════════════════════════
    if (signalCollector) {
      try {
        // Collect tool action signal
        signalCollector.collectToolAction(toolName, toolInput, !isError, toolOutput);

        // Collect git-specific signals
        if (toolName === 'Bash' && toolInput.command?.startsWith('git ')) {
          const gitCmd = toolInput.command.split(' ')[1];
          const gitActionMap = {
            'commit': 'commit',
            'push': 'push',
            'pull': 'pull',
            'merge': isError ? 'merge_conflict' : 'merge_success',
            'revert': 'revert',
          };
          if (gitActionMap[gitCmd]) {
            signalCollector.collectGitAction(gitActionMap[gitCmd], {
              command: toolInput.command,
            });
          }
        }

        // Collect semantic signals for code changes
        if ((toolName === 'Write' || toolName === 'Edit') && !isError) {
          const filePath = toolInput.file_path || toolInput.filePath || '';
          const content = toolInput.content || toolInput.new_string || '';
          const linesChanged = (content.match(/\n/g) || []).length + 1;

          // Detect patterns in code changes
          if (content.includes('test') || filePath.includes('test')) {
            signalCollector.collectSemanticSignal('test_added', { file: filePath, lines: linesChanged });
          }
          if (content.includes('/**') || content.includes('///') || content.includes('"""')) {
            signalCollector.collectSemanticSignal('documentation', { file: filePath });
          }
          if (linesChanged > DC.LENGTH.CONTRIBUTOR_PROFILE && toolName === 'Write') {
            signalCollector.collectSemanticSignal('new_abstraction', { file: filePath, lines: linesChanged });
          }
        }
      } catch (e) {
        // Signal collection failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // COGNITIVE BIASES: Track actions and detect biases
    // "Le chien voit ce que l'humain refuse de voir"
    // ═══════════════════════════════════════════════════════════════════════════
    let detectedBiases = [];
    if (cognitiveBiases) {
      try {
        // Record the action
        const filePath = toolInput.file_path || toolInput.filePath || null;
        cognitiveBiases.recordAction(toolName, {
          file: filePath,
          error: isError,
          errorType: isError ? detectErrorType(typeof toolOutput === 'string' ? toolOutput : '') : null,
        });

        // Run bias detection periodically (every N actions)
        const stats = cognitiveBiases.getStats();
        if (stats.actionHistorySize % DC.FREQUENCY.BIAS_CHECK_INTERVAL === 0) {
          detectedBiases = cognitiveBiases.detectBiases();
        }
      } catch (e) {
        // Bias detection failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TOPOLOGY TRACKER: Track task depth for rabbit hole detection
    // "Le chien garde le chemin"
    // ═══════════════════════════════════════════════════════════════════════════
    let topologyState = null;
    if (topologyTracker) {
      try {
        topologyState = topologyTracker.getState();
      } catch (e) {
        // Topology tracking failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // INTERVENTION ENGINE: Evaluate and trigger adaptive nudges
    // "Le chien sait quand parler et quand se taire"
    // ═══════════════════════════════════════════════════════════════════════════
    let intervention = null;
    if (interventionEngine) {
      try {
        // Get psychology state if available
        const psychologyState = psychology ? psychology.getState() : null;

        // Evaluate intervention
        intervention = interventionEngine.evaluate({
          psychology: psychologyState,
          biases: detectedBiases,
          topology: topologyState,
        });
      } catch (e) {
        // Intervention evaluation failed - continue without
      }
    }

    // ==========================================================================
    // AUTONOMOUS JUDGMENT SYSTEM
    // ==========================================================================
    let autoJudgmentResult = null;

    if (autoJudge) {
      // Observe errors
      if (isError) {
        const errorText = typeof toolOutput === 'string' ? toolOutput :
                          toolOutput?.error || toolOutput?.message || 'Unknown error';
        const errorType = detectErrorType(errorText);
        autoJudgmentResult = autoJudge.observeError(toolName, errorType, errorText.slice(0, 200));
      }
      // Observe successes
      else {
        autoJudgmentResult = autoJudge.observeSuccess(toolName, `${toolName} completed`);
      }

      // Observe code changes
      if ((toolName === 'Write' || toolName === 'Edit') && !isError) {
        const filePath = toolInput.file_path || toolInput.filePath || '';
        const content = toolInput.content || toolInput.new_string || '';
        const linesChanged = (content.match(/\n/g) || []).length + 1;
        autoJudge.observeCodeChange(filePath, toolName.toLowerCase(), linesChanged);
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // HARMONY ANALYZER: Real-time φ violation detection
    // "Le chien veille sur l'harmonie"
    // ═══════════════════════════════════════════════════════════════════════════
    if (harmonyAnalyzer && (toolName === 'Write' || toolName === 'Edit') && !isError) {
      const filePath = toolInput.file_path || toolInput.filePath || '';

      // Only analyze code files
      const ext = path.extname(filePath).toLowerCase();
      const codeExtensions = ['.js', '.cjs', '.mjs', '.ts', '.tsx', '.jsx', '.py', '.rs', '.go', '.sol'];

      if (codeExtensions.includes(ext)) {
        // Run analysis asynchronously to not block
        setImmediate(() => {
          try {
            const gaps = harmonyAnalyzer.analyzeFile(filePath);

            // Filter high/medium severity gaps
            const significantGaps = gaps.filter(g =>
              g.severity === 'high' || g.severity === 'medium'
            );

            if (significantGaps.length > 0 && consciousness) {
              // Group by principle
              const byPrinciple = {};
              for (const gap of significantGaps) {
                byPrinciple[gap.principle] = (byPrinciple[gap.principle] || 0) + 1;
              }

              // Record as insight if enough gaps
              if (significantGaps.length >= DC.FREQUENCY.HARMONY_GAP_MIN) {
                const principles = Object.entries(byPrinciple)
                  .sort((a, b) => b[1] - a[1])
                  .map(([p, n]) => `${p}:${n}`)
                  .join(', ');

                consciousness.recordInsight({
                  type: 'harmony_violation',
                  title: `φ gaps in ${path.basename(filePath)}`,
                  message: `Found ${significantGaps.length} harmony gaps (${principles})`,
                  data: {
                    file: filePath,
                    gapCount: significantGaps.length,
                    byPrinciple,
                    topGap: significantGaps[0]?.message,
                  },
                  priority: significantGaps.some(g => g.severity === 'high') ? 'high' : 'medium',
                });
              }

              // Record specific patterns for learning
              for (const gap of significantGaps.slice(0, 3)) {
                consciousness.observeHumanPattern('harmonyGaps', {
                  principle: gap.principle,
                  severity: gap.severity,
                  file: path.basename(filePath),
                });
              }
            }
          } catch (e) {
            // Harmony analysis failed - continue without
          }
        });
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CONTRIBUTOR PROFILE ENRICHMENT - "Les rails dans le cerveau"
    // Observe activity and enrich contributor profiles asynchronously
    // ═══════════════════════════════════════════════════════════════════════════
    if (contributorDiscovery && !isError) {
      // Observe commits to update contributor profiles
      if (toolName === 'Bash' && toolInput.command?.startsWith('git commit')) {
        // Trigger async profile refresh for current user
        setImmediate(async () => {
          try {
            await contributorDiscovery.getCurrentUserProfile();
          } catch (e) { /* ignore */ }
        });
      }

      // Observe significant code changes (many lines = more profile data)
      if ((toolName === 'Write' || toolName === 'Edit')) {
        const content = toolInput.content || toolInput.new_string || '';
        const linesChanged = (content.match(/\n/g) || []).length + 1;

        // Only enrich on significant changes (>50 lines)
        if (linesChanged > DC.LENGTH.CONTRIBUTOR_PROFILE) {
          setImmediate(async () => {
            try {
              const profile = await contributorDiscovery.getCurrentUserProfile();
              // Store enriched profile info in environment for other hooks
              if (profile?.insights?.phiScores) {
                process.env.CYNIC_CONTRIBUTOR_DEPTH = String(profile.insights.phiScores.depth);
              }
            } catch (e) { /* ignore */ }
          });
        }
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // AWAIT ORCHESTRATION: Make CYNIC's computation VISIBLE to Claude
    // FIX: Was fire-and-forget → symbiosisCache always empty → Claude blind
    // Now: await with 2s timeout → real Judge 25-dim + Dog votes in output
    // "Ohr needs Kelim" — Light needs vessels to be received
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      await Promise.race([
        Promise.allSettled([orchestrationPromise, autoOrchestratorPromise]),
        new Promise(resolve => setTimeout(resolve, 2000)),
      ]);
    } catch {
      // Timeout or error — continue with whatever data we have
    }

    // Persist symbiosis cache to file for cross-invocation fallback
    persistSymbiosisCache(symbiosisCache);

    // If intervention was triggered, output it (priority over auto-judgment)
    if (intervention?.message) {
      safeOutput({
        continue: true,
        message: intervention.message,
      });
      return;
    }

    // If biases detected but no intervention, show informative warning
    // "Le chien voit ce que l'humain refuse de voir"
    if (detectedBiases.length > 0 && cognitiveBiases) {
      const highConfidenceBiases = detectedBiases.filter(b => b.confidence >= DC.CONFIDENCE.BIAS_DETECTION);
      if (highConfidenceBiases.length > 0) {
        const biasWarning = highConfidenceBiases
          .map(b => cognitiveBiases.formatDetection(b))
          .join('\n');
        // Guardian speaks on bias detection
        const guardian = COLLECTIVE_DOGS.GUARDIAN;
        safeOutput({
          continue: true,
          message: `\n── ${guardian.icon} ${guardian.name} (${guardian.sefirah}) ────────────────────────────\n   COGNITIVE BIAS DETECTED\n${biasWarning}\n`,
        });
        return;
      }
    }

    // Check session entropy (Phase 6A - Physics integration)
    // "L'entropie révèle le chaos" - High entropy = chaotic session
    if (signalCollector) {
      try {
        const entropy = signalCollector.calculateSessionEntropy();
        // Only warn when entropy exceeds φ⁻¹ (61.8%) - truly chaotic
        if (entropy.combined > DC.CONFIDENCE.ENTROPY_WARNING && entropy.interpretation === 'CHAOTIC') {
          const emoji = signalCollector.getEntropyEmoji(entropy.interpretation);
          const toolBar = '█'.repeat(Math.round(entropy.tool * 10)) + '░'.repeat(10 - Math.round(entropy.tool * 10));
          const fileBar = '█'.repeat(Math.round(entropy.file * 10)) + '░'.repeat(10 - Math.round(entropy.file * 10));
          const timeBar = '█'.repeat(Math.round(entropy.time * 10)) + '░'.repeat(10 - Math.round(entropy.time * 10));
          safeOutput({
            continue: true,
            message: `\n── SESSION ENTROPY ────────────────────────────────────────\n   ${emoji} ${entropy.interpretation} (${Math.round(entropy.combined * 100)}%)\n   Tools: [${toolBar}] ${Math.round(entropy.tool * 100)}%\n   Files: [${fileBar}] ${Math.round(entropy.file * 100)}%\n   Time:  [${timeBar}] ${Math.round(entropy.time * 100)}%\n   💡 Consider focusing on fewer files/tools\n`,
          });
          return;
        }
      } catch (e) {
        // Entropy calculation failed - continue without
      }
    }

    // If rabbit hole detected but no intervention, show informative warning
    // "Le chien garde le chemin"
    if (topologyState?.rabbitHole) {
      const emoji = topologyState.rabbitHole.type === 'depth' ? '🐰' :
                    topologyState.rabbitHole.type === 'relevance' ? '🌀' : '⏰';
      // Cartographer speaks on navigation issues
      const cartographer = COLLECTIVE_DOGS.CARTOGRAPHER;
      safeOutput({
        continue: true,
        message: `\n── ${cartographer.icon} ${cartographer.name} (${cartographer.sefirah}) ────────────────────────────\n   RABBIT HOLE DETECTED\n   ${emoji} ${topologyState.rabbitHole.suggestion}\n`,
      });
      return;
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // VOLUNTARY POVERTY: Celebrate deletions (Phase 10C)
    // "Moins c'est plus" - Diogenes threw away his cup
    // ═══════════════════════════════════════════════════════════════════════════
    if (deletionCelebration && Math.random() < DC.PROBABILITY.DELETION_CELEBRATE) { // φ⁻¹
      safeOutput({
        continue: true,
        message: `\n── VOLUNTARY POVERTY ──────────────────────────────────────\n   🏺 ${deletionCelebration}\n   *tail wag* Less is more.\n`,
      });
      return;
    }

    // If auto-judgment was triggered, output it
    if (autoJudgmentResult?.judgment) {
      let judgment = autoJudgmentResult.judgment;
      let formatted = autoJudge.formatJudgment(judgment);
      let refinementNote = '';

      // ═══════════════════════════════════════════════════════════════════════════
      // SELF-REFINEMENT: Auto-critique and improve judgments (Phase 2)
      // "φ distrusts φ" - CYNIC critiques even itself
      // ═══════════════════════════════════════════════════════════════════════════
      if (selfRefinement && judgment.qScore !== undefined) {
        try {
          // Only refine judgments that might have issues (Q < 60 or GROWL/BARK)
          const shouldRefine = judgment.qScore < DC.QUALITY.Q_SCORE_REFINEMENT ||
                              judgment.verdict === 'GROWL' ||
                              judgment.verdict === 'BARK';

          if (shouldRefine) {
            // Prepare judgment for refinement
            const judgmentForRefinement = {
              Q: judgment.qScore,
              qScore: judgment.qScore,
              verdict: judgment.verdict,
              confidence: judgment.confidence || DC.MAX_CONFIDENCE,
              breakdown: judgment.axiomScores || {
                PHI: judgment.qScore,
                VERIFY: judgment.qScore,
                CULTURE: judgment.qScore,
                BURN: judgment.qScore,
              },
            };

            // Run self-refinement
            const refinementResult = selfRefinement.selfRefine(judgmentForRefinement, {}, { maxIterations: 2 });

            // If improved, update the judgment
            if (refinementResult.improved && refinementResult.totalImprovement > 0) {
              // Update judgment with refined values
              judgment = {
                ...judgment,
                qScore: refinementResult.final.Q,
                verdict: refinementResult.final.verdict,
                refined: true,
                originalQ: refinementResult.original.Q,
                improvement: refinementResult.totalImprovement,
              };

              // Update formatted output
              formatted = autoJudge.formatJudgment(judgment);
              refinementNote = `\n   🔄 Self-refined: ${refinementResult.original.Q}→${refinementResult.final.Q} (+${refinementResult.totalImprovement})`;

              // Record in consciousness
              if (consciousness) {
                consciousness.recordInsight({
                  type: 'self_refinement',
                  title: 'Auto-refinement applied',
                  message: `Improved Q-Score by ${refinementResult.totalImprovement} points`,
                  data: { original: refinementResult.original.Q, final: refinementResult.final.Q },
                  priority: 'low',
                });
              }

              // Save refinement pattern
              const refinementPattern = {
                type: 'self_refinement',
                signature: `${judgment.verdict}_improved`,
                description: `Self-refinement: ${refinementResult.original.verdict}→${refinementResult.final.verdict}`,
              };
              saveCollectivePattern(refinementPattern);
              // Also record to session state for cross-session persistence
              if (sessionState.isInitialized()) {
                sessionState.recordPattern(refinementPattern);
              }
            }
          }
        } catch (e) {
          // Self-refinement failed - continue with original judgment
        }
      }

      // Send to MCP server for persistence
      callBrainTool('brain_save_judgment', {
        judgment: {
          ...judgment,
          source: 'auto-judge',
        },
      }).catch(() => {});

      // Add thermodynamics status to judgment output
      let thermoNote = '';
      if (thermodynamics) {
        try {
          const thermoState = thermodynamics.getState();
          const recommendation = thermodynamics.getRecommendation();
          if (recommendation.level === 'CRITICAL') {
            thermoNote = `\n\n🔥 THERMAL: ${thermoState.heat}° - ${recommendation.message}`;
          } else if (recommendation.level !== 'GOOD' && thermoState.heat > 20) {
            thermoNote = `\n⚡ Eff: ${thermoState.efficiency}% │ Q:${thermoState.heat} W:${thermoState.work}`;
          }
        } catch (e) { /* ignore */ }
      }

      // Add active Dog to judgment output with personality
      const activeDog = getActiveDog(toolName, toolInput, isError);
      let dogNote = '';
      if (activeDog) {
        const quirk = collectiveDogsModule?.getDogQuirk?.(activeDog) || '*sniff*';
        const verb = collectiveDogsModule?.getDogVerb?.(activeDog) || 'observes';
        dogNote = `\n${activeDog.icon} ${activeDog.name} ${quirk} ${verb}.`;

        // Record Dog activity for session summary
        if (collectiveDogsModule?.recordDogActivity) {
          collectiveDogsModule.recordDogActivity(activeDog, isError ? 'judging' : 'observing');
        }
      }

      // ═══════════════════════════════════════════════════════════════════════
      // Task #92: Multi-LLM Consensus Visibility
      // "L'humain voit la validation multi-LLM"
      // ═══════════════════════════════════════════════════════════════════════
      let consensusNote = '';
      if (judgment?.multiLLM) {
        const llm = judgment.multiLLM;
        if (llm.hasConsensus) {
          const icon = llm.disagreement ? '⚠️' : '✅';
          const ratio = Math.round((llm.consensusRatio || 0) * 100);
          consensusNote = `\n${c(ANSI.cyan, `${icon} Multi-LLM:`)} ${ratio}% consensus (${llm.validators} validators)`;
          if (llm.disagreement) {
            consensusNote += ` - ${c(ANSI.yellow, `LLM says: ${llm.llmVerdict}`)}`;
          }
        } else if (llm.validators > 0) {
          consensusNote = `\n${c(ANSI.dim, '🔗 Multi-LLM:')} no consensus (${llm.validators} validators)`;
        } else if (llm.error) {
          consensusNote = `\n${c(ANSI.dim, '🔗 Multi-LLM:')} unavailable`;
        }
      }

      // Output the judgment (will be shown to user)
      safeOutput({
        continue: true,
        message: formatted + refinementNote + thermoNote + dogNote + consensusNote,
      });
      return;
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 23: CYNIC OBSERVES - Visible Feedback System
    // "Ohr needs Kelim" - Light needs vessels to be received
    // Shows observation summary, agent suggestions, and proactive insights
    // ═══════════════════════════════════════════════════════════════════════════

    const outputParts = [];

    // 0. Active Dog Display - Which Sefirot is speaking
    // "Le Collectif observe - un Chien répond"
    const activeDog = getActiveDog(toolName, toolInput, isError);

    // Determine action description
    let actionDesc = '';
    if (isError) {
      actionDesc = 'protecting';
    } else if (toolName === 'Read' || toolName === 'Glob' || toolName === 'Grep') {
      actionDesc = 'exploring';
    } else if (toolName === 'Write' || toolName === 'Edit') {
      actionDesc = 'building';
    } else if (toolName === 'Bash') {
      const cmd = toolInput.command || '';
      if (cmd.startsWith('git ')) actionDesc = 'tracking';
      else if (cmd.match(/test/i)) actionDesc = 'verifying';
      else actionDesc = 'executing';
    } else if (toolName === 'Task') {
      actionDesc = 'dispatching';
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Task #86: VISIBILITY - Show status for ALL significant operations
    // Da'at: "L'humain VOIT ce que CYNIC pense à chaque opération"
    // Only silent for trivial reads (Glob/Grep with no results shown)
    // ═══════════════════════════════════════════════════════════════════════════
    const trivialRead = (toolName === 'Glob' || toolName === 'Grep') && !isError;
    const showDog = !trivialRead; // Show for everything except trivial reads

    // ═══════════════════════════════════════════════════════════════════════════
    // INLINE STATUS BAR - Da'at: Making invisible visible
    // Shows: [🛡️ Guardian │ 58% │ 12 patterns │ ✨ flow │ Q:75 │ 🗳️ 9/11]
    // SYMBIOSIS: Now includes Q-Score and Dog consensus (Task #4 + #5)
    // ═══════════════════════════════════════════════════════════════════════════
    if (showDog && activeDog) {
      const inlineStatus = generateInlineStatus(activeDog, {
        showPsychology: true,
        thermodynamics,
        psychology,
        harmonicFeedback,
        // SYMBIOSIS: Pass cached judgment and consensus data
        judgment: symbiosisCache.lastJudgment,
        consensus: symbiosisCache.lastConsensus,
      });
      if (inlineStatus) {
        outputParts.push(`\n${inlineStatus} `);
      } else {
        // Fallback to simple Dog indicator
        const verb = collectiveDogsModule?.getDogVerb?.(activeDog) || actionDesc || 'observes';
        const dogName = activeDog.name?.toUpperCase() || 'CYNIC';
        const dogColor = DOG_COLORS?.[dogName] || ANSI.brightWhite;
        outputParts.push(`\n${c(dogColor, activeDog.icon + ' ' + verb)} `);
      }

      // Record Dog activity for session summary
      if (collectiveDogsModule?.recordDogActivity) {
        collectiveDogsModule.recordDogActivity(activeDog, actionDesc || toolName);
      }
    }

    // 2. Observation Summary (efficiency, escalation, patterns)
    if (suggestionEngine) {
      const observationSummary = suggestionEngine.getObservationSummary();
      const formattedObservation = suggestionEngine.formatObservationSummary(observationSummary);
      if (formattedObservation) {
        outputParts.push(formattedObservation);
      }
    }

    // 2. Agent Suggestion (Seder Hishtalshelut - Lightning Flash)
    // Suggest relevant agent based on error type
    if (suggestionEngine && isError) {
      const errorText = typeof toolOutput === 'string' ? toolOutput :
                        toolOutput?.error || toolOutput?.message || '';
      let errorType = null;

      // Detect error type for agent suggestion
      if (errorText.includes('ENOENT') || errorText.includes('not found') || errorText.includes('does not exist')) {
        errorType = 'file_not_found';
      } else if (errorText.includes('EACCES') || errorText.includes('Permission denied')) {
        errorType = 'permission_denied';
      } else if (errorText.includes('ECONNREFUSED')) {
        errorType = 'connection_refused';
      } else if (errorText.includes('SyntaxError')) {
        errorType = 'syntax_error';
      } else if (errorText.includes('TypeError')) {
        errorType = 'type_error';
      } else if (errorText.includes('test') && (errorText.includes('fail') || errorText.includes('FAIL'))) {
        errorType = 'test_failure';
      }

      if (errorType) {
        const agentSuggestion = suggestionEngine.suggestAgent(errorType, { tool: toolName });
        if (agentSuggestion) {
          const formattedAgent = suggestionEngine.formatAgentSuggestion(agentSuggestion);
          if (formattedAgent) {
            outputParts.push(formattedAgent);
          }
        }
      }
    }

    // 3. Proactive Suggestion (anti-patterns, escalation changes)
    if (proactiveSuggestion) {
      const formattedSuggestion = suggestionEngine.formatForOutput(proactiveSuggestion);
      if (formattedSuggestion) {
        outputParts.push(formattedSuggestion);
      }
    }

    // 4. Recovery message (de-escalation)
    if (suggestionEngine && !isError) {
      const recoveryMessage = suggestionEngine.getRecoveryMessage();
      if (recoveryMessage) {
        outputParts.push(`\n${recoveryMessage}\n`);
      }
    }

    // 5. Thermodynamics mini-display (after significant activity) - now with colors!
    // ═══════════════════════════════════════════════════════════════════════════
    // LEARNING VISIBILITY (Task #74: Make subconscious learning visible)
    // "Gam zo l'tova" - Even the subconscious works for good
    // ═══════════════════════════════════════════════════════════════════════════
    if (harmonicFeedback) {
      try {
        // Check for recent learning events (implicit feedback detected this call)
        const implicitDetected = implicitFeedback?.detectFeedback?.() || [];
        const recentFeedback = implicitDetected.slice(0, 2);

        // Show implicit learning indicators
        for (const fb of recentFeedback) {
          const sentiment = fb.sentiment === 'positive' ? '✅' : (fb.sentiment === 'negative' ? '❌' : '➡️');
          const action = fb.type === 'IMPLICIT_FOLLOWED' ? 'followed suggestion' :
                        fb.type === 'IMPLICIT_OPPOSITE' ? 'chose different approach' :
                        fb.type === 'IMPLICIT_IGNORED' ? 'skipped suggestion' :
                        'action observed';
          outputParts.push(`\n${c(ANSI.cyan, '🧠 Learning:')} ${sentiment} ${action}\n`);
        }

        // Periodically review patterns for promotion (every ~50 tool calls)
        if (Math.random() < 0.02) { // ~2% chance per call
          const reviewResult = harmonicFeedback.reviewPatterns?.();
          if (reviewResult?.promoted?.length > 0) {
            for (const patternId of reviewResult.promoted.slice(0, 2)) {
              outputParts.push(`\n${c(ANSI.brightGreen, '📈 Pattern promoted:')} ${patternId} → heuristic\n`);
            }
          }
          if (reviewResult?.demoted?.length > 0) {
            for (const patternId of reviewResult.demoted.slice(0, 2)) {
              outputParts.push(`\n${c(ANSI.yellow, '📉 Heuristic demoted:')} ${patternId} (underperforming)\n`);
            }
          }
        }

        // ═══════════════════════════════════════════════════════════════════════
        // THOMPSON SAMPLER VISIBILITY (Task #87)
        // "L'humain comprend ce que CYNIC a appris"
        // Show state more often (~10%) or when significant learning happened
        // ═══════════════════════════════════════════════════════════════════════
        if (Math.random() < 0.10) { // ~10% chance per call
          const stats = harmonicFeedback.thompsonSampler?.getStats?.();
          if (stats && stats.armCount > 0) {
            const topArms = harmonicFeedback.thompsonSampler?.getTopArms?.(3) || [];
            if (topArms.length > 0) {
              const armNames = topArms.map(a => {
                const ev = a.expectedValue ? ` (${Math.round(a.expectedValue * 100)}%)` : '';
                return `${a.name || a.id}${ev}`;
              }).join(', ');
              const exploitRate = stats.exploitation ? Math.round(stats.exploitation * 100) : null;
              const exploitInfo = exploitRate !== null ? ` │ exploit: ${exploitRate}%` : '';
              outputParts.push(`\n${c(ANSI.cyan, '🎰 Thompson:')} ${stats.armCount} patterns${exploitInfo}\n`);
              outputParts.push(`   ${c(ANSI.dim, 'Top:')} ${armNames}\n`);
            }
          }
        }

        // ═══════════════════════════════════════════════════════════════════════
        // FIL 3 (Task #85): Periodic learn() trigger for long sessions
        // Ensures weight adjustments happen even without session end
        // ~5% chance per call ≈ every 20 tool calls
        // ═══════════════════════════════════════════════════════════════════════
        if (Math.random() < 0.05) {
          callBrainTool('brain_learning', { action: 'learn' })
            .then(result => {
              if (result?.weightAdjustments && Object.keys(result.weightAdjustments).length > 0) {
                // Learning happened - could add visibility here in future
              }
            })
            .catch(() => {
              // Silently fail - learning is enhancement, not critical
            });
        }

        // ═══════════════════════════════════════════════════════════════════════
        // PHASE B1 (Task #21): Periodic mathematical system visibility
        // Antifragility, Temporal, Girsanov insights (~2% chance)
        // ═══════════════════════════════════════════════════════════════════════
        if (Math.random() < 0.02) {
          // Antifragility Index visibility
          const afStats = harmonicFeedback.getAntifragilityStats?.();
          if (afStats?.metrics?.trend && afStats.totalObservations > 10) {
            const idx = afStats.metrics.index.toFixed(3);
            const trend = afStats.metrics.trend;
            const emoji = trend === 'antifragile' ? '💪' : trend === 'fragile' ? '⚠️' : '🛡️';
            outputParts.push(`\n${c(ANSI.cyan, `${emoji} Antifragility:`)} ${idx} (${trend})\n`);
          }

          // Temporal patterns visibility
          const timing = harmonicFeedback.getOptimalTiming?.();
          if (timing?.recommendation && timing.recommendation !== 'no_pattern') {
            const phase = timing.currentPhase || '?';
            const energy = timing.energyLevel || 'UNKNOWN';
            outputParts.push(`   ${c(ANSI.dim, 'Temporal:')} phase ${phase}, energy: ${energy}\n`);
          }

          // Girsanov best measure visibility
          const gStats = harmonicFeedback.getGirsanovStats?.();
          if (gStats?.totalObservations > 5) {
            const best = gStats.bestMeasure || 'P';
            const brier = gStats.measures?.[best]?.brierScore?.toFixed(3) || '?';
            outputParts.push(`   ${c(ANSI.dim, 'Girsanov:')} best measure: ${best} (Brier: ${brier})\n`);
          }

          // Non-commutative pairs visibility
          const ncStats = harmonicFeedback.getNonCommutativeStats?.();
          if (ncStats?.metrics?.stronglyNonCommutative > 0) {
            const count = ncStats.metrics.stronglyNonCommutative;
            outputParts.push(`   ${c(ANSI.dim, 'Non-commutative:')} ${count} dimension pair(s) with order effects\n`);
          }

          // ═══════════════════════════════════════════════════════════════════
          // BRIDGE A (Tasks #26-29): Export harmonic state for TUI consumption
          // Writes to ~/.cynic/harmonic/state.json for psy-dashboard to read
          // ═══════════════════════════════════════════════════════════════════
          const exportState = async () => {
            try {
              const fs = await import('fs');
              const os = await import('os');
              const pathMod = await import('path');

              const harmonicDir = pathMod.default.join(os.default.homedir(), '.cynic', 'harmonic');
              const statePath = pathMod.default.join(harmonicDir, 'state.json');

              // Ensure directory exists
              await fs.promises.mkdir(harmonicDir, { recursive: true });

              // Gather all mathematical system states
              const state = {
                timestamp: Date.now(),
                // FFT Temporal (Task #26)
                temporal: harmonicFeedback.getOptimalTiming?.() || {},
                temporalAnalysis: harmonicFeedback.temporalAnalyzer?.lastAnalysis || null,
                // Girsanov (Task #27)
                girsanov: harmonicFeedback.getGirsanovStats?.() || {},
                // Antifragility (Task #28)
                antifragility: harmonicFeedback.getAntifragilityStats?.() || {},
                // Non-commutative (Task #29)
                nonCommutative: harmonicFeedback.getNonCommutativeStats?.() || {},
                // Core harmonic state
                harmonic: harmonicFeedback.getState?.() || {},
                // Thompson sampling
                thompson: harmonicFeedback.thompsonSampler?.getStats?.() || {},
              };

              await fs.promises.writeFile(statePath, JSON.stringify(state, null, 2));
            } catch {
              // Silent fail - state export is enhancement
            }
          };

          // Export state with ~5% probability (every ~20 tool calls)
          if (Math.random() < 0.05) {
            exportState();
          }
        }

      } catch {
        // Learning visibility failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TRIGGER ENGINE: Proactive Suggestions (Task #28: W2.2)
    // "Le chien anticipe" - CYNIC becomes proactive, not just reactive
    // ═══════════════════════════════════════════════════════════════════════════
    if (triggerEngine) {
      try {
        // Update context for trigger evaluation
        const thermoState = thermodynamics?.getState?.() || {};
        const psyState = psychology?.getState?.() || {};
        const sessionState = getSessionState();
        const goals = sessionState?.getActiveGoals?.() || [];

        triggerEngine.updateContext({
          userEnergy: thermoState.efficiency ? thermoState.efficiency / 100 : (psyState.energy || 1.0),
          userFocus: psyState.focus || 1.0,
          currentFocus: toolName === 'Edit' || toolName === 'Write'
            ? (toolInput.file_path || toolInput.filePath || null)
            : (toolName === 'Bash' ? 'terminal' : null),
          activeGoal: goals.find(g => g.status === 'active') || null,
          goals: goals,
          sessionDuration: sessionState?.getDuration?.() || 0,
        });

        // Task #31: Check for implicit acceptance of pending suggestions
        const resolvedSuggestions = triggerEngine.checkImplicitAcceptance();
        for (const resolved of resolvedSuggestions) {
          debugLog('observe', 'Suggestion resolved', {
            id: resolved.id,
            trigger: resolved.trigger,
            accepted: resolved.accepted,
            reason: resolved.reason,
          });
          // Record for telemetry
          if (telemetry) {
            telemetry.recordEvent?.('suggestion_outcome', {
              suggestionId: resolved.id,
              trigger: resolved.trigger,
              accepted: resolved.accepted,
              reason: resolved.reason,
            });
          }
        }

        // Evaluate all triggers
        const suggestions = triggerEngine.evaluateAll();

        // Debug: Log trigger engine activity (every 10 tool calls to avoid spam)
        if (Math.random() < 0.1 || suggestions.length > 0) {
          const state = triggerEngine.getState();
          debugLog('observe', 'TriggerEngine active', {
            enabled: state.enabled,
            pendingSuggestions: state.pendingSuggestions,
            contextEnergy: Math.round((state.context.userEnergy || 1) * 100),
            triggered: suggestions.length,
          });
        }

        // Task #30: Dogs voting on suggestions (with timeout to stay non-blocking)
        const VOTE_TIMEOUT_MS = 100; // Fast timeout - voting is optional
        for (const suggestion of suggestions) {
          // Try to get Dogs vote, but don't block
          let voteResult = { approved: true, reason: 'no_voting' };
          try {
            voteResult = await Promise.race([
              triggerEngine.voteSuggestion(suggestion),
              new Promise(resolve => setTimeout(() => resolve({ approved: true, reason: 'timeout' }), VOTE_TIMEOUT_MS)),
            ]);
          } catch {
            // Voting failed - default to showing (fail-open)
            voteResult = { approved: true, reason: 'vote_error' };
          }

          // Only show if Dogs approved (or voting unavailable)
          if (!voteResult.approved) {
            debugLog('observe', 'Suggestion rejected by Dogs', { suggestion: suggestion.id, reason: voteResult.reason });
            continue;
          }

          const urgencyColor = suggestion.urgency === 'URGENT' ? ANSI.brightRed :
                              suggestion.urgency === 'ACTIVE' ? ANSI.yellow :
                              ANSI.cyan;
          outputParts.push(`\n${c(urgencyColor, '💡 PROACTIVE:')} ${suggestion.message}\n`);

          // Record for telemetry
          if (telemetry) {
            telemetry.recordEvent?.('proactive_suggestion', {
              trigger: suggestion.trigger,
              urgency: suggestion.urgency,
              suggestionId: suggestion.id,
              voteResult: voteResult.reason,
            });
          }
        }
      } catch (e) {
        // Trigger engine failed - continue without (non-blocking)
        debugError('observe', 'TriggerEngine evaluation failed', e, 'warn');
      }
    }

    if (thermodynamics) {
      try {
        const thermoState = thermodynamics.getState();
        // Only show if there's meaningful activity and efficiency is noteworthy
        if (thermoState.work > 20 || thermoState.heat > 10) {
          const recommendation = thermodynamics.getRecommendation();
          // Show warning or critical states
          if (recommendation.level !== 'GOOD') {
            const effColor = thermoState.efficiency > 50 ? ANSI.brightGreen : (thermoState.efficiency > 30 ? ANSI.yellow : ANSI.brightRed);
            const heatColor = thermoState.heat > 80 ? ANSI.brightRed : (thermoState.heat > 50 ? ANSI.yellow : ANSI.green);
            outputParts.push(`\n${c(ANSI.cyan, '── ⚡ THERMO:')} ${c(effColor, thermoState.efficiency + '% eff')} │ ${c(heatColor, 'Q:' + thermoState.heat)} ${c(ANSI.brightGreen, 'W:' + thermoState.work)}\n`);
            const msgColor = recommendation.level === 'CRITICAL' ? ANSI.brightRed : ANSI.yellow;
            outputParts.push(`   ${c(msgColor, recommendation.message)}\n`);
          } else if (thermoState.isCritical) {
            outputParts.push(`\n${c(ANSI.brightRed, '🔥 THERMAL RUNAWAY: Heat ' + thermoState.heat + '° - TAKE A BREAK')}\n`);
          }
        }
      } catch (e) {
        // Thermodynamics display failed - continue without
      }
    }

    // Output combined message if we have anything to show
    if (outputParts.length > 0) {
      safeOutput({
        continue: true,
        message: outputParts.join(''),
      });
      return;
    }

    // Observer never blocks - always continue silently
    safeOutput({ continue: true });

  } catch (error) {
    // Observer must never fail - silent continuation
    safeOutput({ continue: true });
  }
}

main();
