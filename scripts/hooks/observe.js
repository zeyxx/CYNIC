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

// ═══════════════════════════════════════════════════════════════════════════
// FIL 2 (Task #84): Wire harmonic feedback → learning service via brain_learning
// ═══════════════════════════════════════════════════════════════════════════
if (harmonicFeedback && harmonicFeedback.setLearningCallback) {
  harmonicFeedback.setLearningCallback(async (feedback) => {
    // Call brain_learning tool to process feedback
    await callBrainTool('brain_learning', {
      action: 'feedback',
      outcome: feedback.outcome,
      context: feedback.sourceContext,
    }).catch(() => {
      // Silently fail - MCP server might not be available
    });
  });
}

// =============================================================================
// ANTI-PATTERN DETECTOR - Error loops & bad workflows
// "Le chien détecte les mauvaises habitudes"
// =============================================================================

// Session-scoped state for pattern detection
const antiPatternState = {
  // Error tracking (detect loops)
  recentErrors: [],           // Last N errors: { type, file, timestamp }
  errorLoopThreshold: 3,      // Same error 3x = loop
  errorWindowMs: 5 * 60000,   // 5 minute window

  // Tool sequence tracking (detect anti-patterns)
  recentTools: [],            // Last N tools: { name, file, timestamp }
  filesRead: new Set(),       // Files that have been Read
  filesEdited: new Set(),     // Files that have been Edited

  // Test tracking
  lastTestRun: 0,             // Timestamp of last test
  commitsSinceTest: 0,        // Commits without testing
};

/**
 * Detect anti-patterns in tool usage
 * Returns warnings if anti-patterns detected
 */
function detectAntiPatterns(toolName, toolInput, isError, toolOutput) {
  const warnings = [];
  const now = Date.now();
  const filePath = toolInput?.file_path || toolInput?.filePath || '';
  const command = toolInput?.command || '';

  // === ERROR LOOP DETECTION ===
  if (isError) {
    const errorText = typeof toolOutput === 'string' ? toolOutput :
                      toolOutput?.error || toolOutput?.message || '';
    const errorType = detectErrorType(errorText);

    // Add to recent errors
    antiPatternState.recentErrors.push({
      type: errorType,
      file: filePath || command.slice(0, 50),
      timestamp: now,
    });

    // Prune old errors
    antiPatternState.recentErrors = antiPatternState.recentErrors.filter(
      e => now - e.timestamp < antiPatternState.errorWindowMs
    );

    // Check for error loop (same error type 3+ times)
    const sameTypeErrors = antiPatternState.recentErrors.filter(e => e.type === errorType);
    if (sameTypeErrors.length >= antiPatternState.errorLoopThreshold) {
      warnings.push({
        type: 'error_loop',
        severity: 'high',
        message: `*GROWL* Tu tournes en rond - "${errorType}" ${sameTypeErrors.length}x en ${Math.round(antiPatternState.errorWindowMs/60000)} min`,
        suggestion: 'Prends du recul. Le problème est peut-être ailleurs.',
        data: { errorType, count: sameTypeErrors.length },
      });
      // Reset to avoid spamming
      antiPatternState.recentErrors = antiPatternState.recentErrors.filter(e => e.type !== errorType);
    }

    // Check for file hotspot (same file causing errors)
    if (filePath) {
      const sameFileErrors = antiPatternState.recentErrors.filter(e => e.file === filePath);
      if (sameFileErrors.length >= 3) {
        warnings.push({
          type: 'file_hotspot',
          severity: 'medium',
          message: `*sniff* Ce fichier pose problème: ${path.basename(filePath)} (${sameFileErrors.length} erreurs)`,
          suggestion: 'Peut-être revoir l\'approche sur ce fichier?',
          data: { file: filePath, count: sameFileErrors.length },
        });
      }
    }
  }

  // === TOOL SEQUENCE ANTI-PATTERNS ===

  // Track tool usage
  antiPatternState.recentTools.push({ name: toolName, file: filePath, timestamp: now });
  if (antiPatternState.recentTools.length > 20) {
    antiPatternState.recentTools.shift();
  }

  // Edit without Read anti-pattern
  if ((toolName === 'Edit' || toolName === 'Write') && filePath) {
    if (!antiPatternState.filesRead.has(filePath)) {
      warnings.push({
        type: 'edit_without_read',
        severity: 'low',
        message: `*ears perk* Edit sans Read: ${path.basename(filePath)}`,
        suggestion: 'Lire avant d\'écrire évite les erreurs.',
        data: { file: filePath },
      });
    }
    antiPatternState.filesEdited.add(filePath);
  }

  // Track reads
  if (toolName === 'Read' && filePath) {
    antiPatternState.filesRead.add(filePath);
  }

  // === COMMIT WITHOUT TEST ===

  // Track test runs
  if (toolName === 'Bash' && command.match(/npm\s+(run\s+)?test|jest|vitest|mocha|pytest/i)) {
    antiPatternState.lastTestRun = now;
    antiPatternState.commitsSinceTest = 0;
  }

  // Track commits
  if (toolName === 'Bash' && command.startsWith('git commit')) {
    antiPatternState.commitsSinceTest++;

    // Warn if multiple commits without testing
    if (antiPatternState.commitsSinceTest >= 2) {
      warnings.push({
        type: 'commit_without_test',
        severity: 'medium',
        message: `*sniff* ${antiPatternState.commitsSinceTest} commits sans test`,
        suggestion: 'npm test avant de continuer?',
        data: { commits: antiPatternState.commitsSinceTest },
      });
    }
  }

  return warnings;
}

// =============================================================================
// COLLECTIVE DOGS (SEFIROT) - Load from shared module
// =============================================================================

import { createRequire } from 'module';
const requireCJS = createRequire(import.meta.url);

let collectiveDogsModule = null;
let COLLECTIVE_DOGS = {};
let colors = null;
let ANSI = null;
let DOG_COLORS = null;

// Load colors module
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

try {
  collectiveDogsModule = requireCJS('../lib/collective-dogs.cjs');
  COLLECTIVE_DOGS = collectiveDogsModule.COLLECTIVE_DOGS;
  // Use DOG_COLORS from collective-dogs if colors module didn't load
  if (!colors && collectiveDogsModule.DOG_COLORS) {
    DOG_COLORS = collectiveDogsModule.DOG_COLORS;
  }
} catch (e) {
  // Fallback to inline definitions if module not available
  COLLECTIVE_DOGS = {
    CYNIC:        { icon: '🧠', name: 'CYNIC', sefirah: 'Keter', domain: 'orchestration' },
    SCOUT:        { icon: '🔍', name: 'Scout', sefirah: 'Netzach', domain: 'exploration' },
    GUARDIAN:     { icon: '🛡️', name: 'Guardian', sefirah: 'Gevurah', domain: 'protection' },
    DEPLOYER:     { icon: '🚀', name: 'Deployer', sefirah: 'Hod', domain: 'deployment' },
    ARCHITECT:    { icon: '🏗️', name: 'Architect', sefirah: 'Chesed', domain: 'building' },
    JANITOR:      { icon: '🧹', name: 'Janitor', sefirah: 'Yesod', domain: 'cleanup' },
    ORACLE:       { icon: '🔮', name: 'Oracle', sefirah: 'Tiferet', domain: 'insight' },
    ANALYST:      { icon: '📊', name: 'Analyst', sefirah: 'Binah', domain: 'analysis' },
    SAGE:         { icon: '🦉', name: 'Sage', sefirah: 'Chochmah', domain: 'wisdom' },
    SCHOLAR:      { icon: '📚', name: 'Scholar', sefirah: 'Daat', domain: 'knowledge' },
    CARTOGRAPHER: { icon: '🗺️', name: 'Cartographer', sefirah: 'Malkhut', domain: 'mapping' },
  };
}

// =============================================================================
// INLINE STATUS BAR - Make CYNIC's thinking visible
// "Da'at = Union through shared knowledge"
// =============================================================================

const PHI_INV = 0.618;

/**
 * Generate compact inline status bar showing CYNIC's internal state
 * Format: [🛡️ Guardian │ 58% │ 12 patterns]
 *
 * This is the first step toward symbiosis: making the invisible visible.
 */
function generateInlineStatus(activeDog, options = {}) {
  const parts = [];

  // 1. Active Dog with icon
  if (activeDog) {
    const dogName = activeDog.name || 'CYNIC';
    parts.push(`${activeDog.icon} ${dogName}`);
  }

  // 2. Confidence from harmonic feedback (Thompson Sampler state)
  if (harmonicFeedback) {
    try {
      const state = harmonicFeedback.getState?.();
      if (state) {
        const confidence = Math.round((state.coherence || 0.5) * 100);
        const confidenceColor = confidence > 50 ? ANSI.brightGreen :
                               confidence > 30 ? ANSI.yellow : ANSI.brightRed;
        parts.push(c(confidenceColor, `${confidence}%`));
      }
    } catch { /* continue without */ }
  }

  // 3. Active patterns count
  if (harmonicFeedback) {
    try {
      const stats = harmonicFeedback.thompsonSampler?.getStats?.();
      if (stats && stats.armCount > 0) {
        parts.push(c(ANSI.cyan, `${stats.armCount} patterns`));
      }
    } catch { /* continue without */ }
  }

  // 4. Psychology state (if available and noteworthy)
  if (psychology && options.showPsychology !== false) {
    try {
      const summary = psychology.getSummary?.();
      if (summary) {
        if (summary.composites?.flow) {
          parts.push(c(ANSI.brightGreen, '✨ flow'));
        } else if (summary.composites?.burnoutRisk) {
          parts.push(c(ANSI.brightRed, '⚠️ burnout'));
        } else if (summary.frustration?.value > 0.5) {
          parts.push(c(ANSI.yellow, '😤 friction'));
        }
      }
    } catch { /* continue without */ }
  }

  // 5. Multi-LLM indicator (if active)
  // TODO: Add when LLMRouter is wired

  if (parts.length === 0) {
    return null;
  }

  // Format: [🛡️ Guardian │ 58% │ 12 patterns │ ✨ flow]
  return `${c(ANSI.dim, '[')}${parts.join(c(ANSI.dim, ' │ '))}${c(ANSI.dim, ']')}`;
}

/**
 * Determine which Dog is most relevant for the current action
 * "Le Collectif observe - un Chien répond"
 */
function getActiveDog(toolName, toolInput, isError) {
  const command = toolInput?.command || '';
  const filePath = toolInput?.file_path || toolInput?.filePath || '';

  // Errors activate the Guardian
  if (isError) {
    return COLLECTIVE_DOGS.GUARDIAN;
  }

  // Tool-specific mappings
  switch (toolName) {
    // Exploration tools → Scout
    case 'Read':
    case 'Glob':
    case 'Grep':
    case 'LS':
      return COLLECTIVE_DOGS.SCOUT;

    // Building tools → Architect
    case 'Write':
    case 'Edit':
    case 'NotebookEdit':
      // Unless it's cleanup/deletion
      if (toolInput?.new_string === '' || toolInput?.content?.length < (toolInput?.old_string?.length || 0)) {
        return COLLECTIVE_DOGS.JANITOR;
      }
      return COLLECTIVE_DOGS.ARCHITECT;

    // Task/Agent dispatch → CYNIC (orchestration)
    case 'Task':
      return COLLECTIVE_DOGS.CYNIC;

    // Web research → Scholar
    case 'WebFetch':
    case 'WebSearch':
      return COLLECTIVE_DOGS.SCHOLAR;

    // Bash commands need deeper analysis
    case 'Bash':
      // Git operations
      if (command.startsWith('git ')) {
        const gitCmd = command.split(' ')[1];
        if (['push', 'deploy', 'publish'].includes(gitCmd)) {
          return COLLECTIVE_DOGS.DEPLOYER;
        }
        if (['log', 'diff', 'show', 'blame'].includes(gitCmd)) {
          return COLLECTIVE_DOGS.ANALYST;
        }
        if (['clean', 'gc', 'prune'].includes(gitCmd)) {
          return COLLECTIVE_DOGS.JANITOR;
        }
        return COLLECTIVE_DOGS.CARTOGRAPHER; // git status, branch, etc.
      }
      // Test commands → Guardian (verification)
      if (command.match(/test|jest|vitest|mocha|pytest|cargo\s+test/i)) {
        return COLLECTIVE_DOGS.GUARDIAN;
      }
      // Build/compile → Architect
      if (command.match(/build|compile|tsc|webpack|vite/i)) {
        return COLLECTIVE_DOGS.ARCHITECT;
      }
      // Deploy commands → Deployer
      if (command.match(/deploy|publish|release/i)) {
        return COLLECTIVE_DOGS.DEPLOYER;
      }
      // Cleanup → Janitor
      if (command.match(/clean|rm\s|del\s|remove/i)) {
        return COLLECTIVE_DOGS.JANITOR;
      }
      // Default bash → Cartographer (mapping/exploration)
      return COLLECTIVE_DOGS.CARTOGRAPHER;

    // MCP tools → depends on the tool
    default:
      // MCP brain tools
      if (toolName.startsWith('mcp__')) {
        if (toolName.includes('search') || toolName.includes('query')) {
          return COLLECTIVE_DOGS.SCOUT;
        }
        if (toolName.includes('memory') || toolName.includes('learn')) {
          return COLLECTIVE_DOGS.SCHOLAR;
        }
        if (toolName.includes('judge') || toolName.includes('refine')) {
          return COLLECTIVE_DOGS.ORACLE;
        }
        if (toolName.includes('pattern')) {
          return COLLECTIVE_DOGS.ANALYST;
        }
      }
      // Default → Scout (exploration)
      return COLLECTIVE_DOGS.SCOUT;
  }
}

/**
 * Format the active Dog display
 */
function formatActiveDog(dog, action = '') {
  const actionText = action ? ` - ${action}` : '';
  return `${dog.icon} ${dog.name} (${dog.sefirah})${actionText}`;
}

// =============================================================================
// P0.2: FACT EXTRACTION (MoltBrain-style)
// "Le chien extrait les faits" - Extract factual knowledge from tool outputs
// =============================================================================

/**
 * Extract facts from tool outputs for semantic memory
 * Facts are discrete pieces of knowledge that can be retrieved later
 * @returns {Array<{type: string, content: string, confidence: number, context: object}>}
 */
function extractFacts(toolName, toolInput, toolOutput, isError) {
  const facts = [];
  const outputStr = typeof toolOutput === 'string' ? toolOutput : JSON.stringify(toolOutput || {});
  const filePath = toolInput?.file_path || toolInput?.filePath || '';

  // Don't extract facts from errors (they're handled separately)
  if (isError) return facts;

  // 1. File structure facts from Read/Glob
  if (toolName === 'Read' && filePath) {
    const ext = path.extname(filePath).toLowerCase();

    // Extract function/class definitions from code files
    if (['.js', '.ts', '.tsx', '.jsx', '.py', '.rs', '.go'].includes(ext)) {
      // JavaScript/TypeScript functions
      const funcMatches = outputStr.match(/(?:function|const|let|var)\s+(\w+)\s*(?:=\s*(?:async\s*)?\(|[\(<])/g);
      if (funcMatches && funcMatches.length > 0) {
        const funcNames = funcMatches.slice(0, 5).map(m => m.match(/(?:function|const|let|var)\s+(\w+)/)?.[1]).filter(Boolean);
        if (funcNames.length > 0) {
          facts.push({
            type: 'file_structure',
            content: `File ${path.basename(filePath)} contains functions: ${funcNames.join(', ')}`,
            confidence: 0.8,
            context: { file: filePath, functions: funcNames },
          });
        }
      }

      // Class definitions
      const classMatches = outputStr.match(/class\s+(\w+)/g);
      if (classMatches && classMatches.length > 0) {
        const classNames = classMatches.map(m => m.match(/class\s+(\w+)/)?.[1]).filter(Boolean);
        facts.push({
          type: 'file_structure',
          content: `File ${path.basename(filePath)} defines classes: ${classNames.join(', ')}`,
          confidence: 0.9,
          context: { file: filePath, classes: classNames },
        });
      }

      // Export facts
      const exportMatches = outputStr.match(/export\s+(?:default\s+)?(?:const|function|class|let|var)?\s*(\w+)/g);
      if (exportMatches && exportMatches.length > 0) {
        const exports = exportMatches.slice(0, 10).map(m => m.match(/(\w+)$/)?.[1]).filter(Boolean);
        facts.push({
          type: 'module_exports',
          content: `${path.basename(filePath)} exports: ${exports.join(', ')}`,
          confidence: 0.9,
          context: { file: filePath, exports },
        });
      }
    }

    // Extract API endpoints from code
    const apiMatches = outputStr.match(/['"`](\/api\/[^'"`]+)['"`]|router\.(get|post|put|delete)\s*\(\s*['"`]([^'"`]+)['"`]/gi);
    if (apiMatches && apiMatches.length > 0) {
      const endpoints = [...new Set(apiMatches.map(m => m.match(/['"`]([^'"`]+)['"`]/)?.[1]).filter(Boolean))];
      if (endpoints.length > 0) {
        facts.push({
          type: 'api_endpoint',
          content: `API endpoints in ${path.basename(filePath)}: ${endpoints.join(', ')}`,
          confidence: 0.85,
          context: { file: filePath, endpoints },
        });
      }
    }

    // Extract environment variable references
    const envMatches = outputStr.match(/process\.env\.(\w+)|import\.meta\.env\.(\w+)|\$\{(\w+)\}/g);
    if (envMatches && envMatches.length > 0) {
      const envVars = [...new Set(envMatches.map(m => m.match(/\.(\w+)|\{(\w+)\}/)?.[1] || m.match(/\.(\w+)|\{(\w+)\}/)?.[2]).filter(Boolean))];
      if (envVars.length > 0) {
        facts.push({
          type: 'env_dependency',
          content: `${path.basename(filePath)} uses env vars: ${envVars.slice(0, 10).join(', ')}`,
          confidence: 0.9,
          context: { file: filePath, envVars },
        });
      }
    }
  }

  // 2. Git facts from Bash git commands
  if (toolName === 'Bash' && toolInput.command?.startsWith('git ')) {
    const command = toolInput.command;

    // Branch facts
    if (command.includes('git branch') && outputStr) {
      const currentBranch = outputStr.match(/\*\s+(\S+)/)?.[1];
      if (currentBranch) {
        facts.push({
          type: 'git_state',
          content: `Current branch: ${currentBranch}`,
          confidence: 1.0,
          context: { branch: currentBranch },
        });
      }
    }

    // Commit facts
    if (command.startsWith('git log') && outputStr) {
      const commitMatches = outputStr.match(/commit\s+([a-f0-9]+)/gi);
      if (commitMatches && commitMatches.length > 0) {
        const recentCommit = commitMatches[0].match(/([a-f0-9]+)/)?.[1];
        facts.push({
          type: 'git_state',
          content: `Recent commit: ${recentCommit?.substring(0, 7)}`,
          confidence: 1.0,
          context: { commit: recentCommit },
        });
      }
    }
  }

  // 3. Test facts from test outputs
  if (toolName === 'Bash' && toolInput.command?.match(/npm\s+(run\s+)?test|jest|vitest|pytest/i)) {
    // Extract test count
    const passMatch = outputStr.match(/(\d+)\s*(?:tests?\s+)?pass(?:ed|ing)?/i);
    const failMatch = outputStr.match(/(\d+)\s*(?:tests?\s+)?fail(?:ed|ing)?/i);
    if (passMatch || failMatch) {
      const passed = parseInt(passMatch?.[1] || '0', 10);
      const failed = parseInt(failMatch?.[1] || '0', 10);
      facts.push({
        type: 'test_state',
        content: `Test results: ${passed} passed, ${failed} failed`,
        confidence: 0.95,
        context: { passed, failed, total: passed + failed },
      });
    }

    // Extract failing test names
    const failingTests = outputStr.match(/(?:FAIL|✕|×)\s+(.+?)(?:\n|$)/gi);
    if (failingTests && failingTests.length > 0) {
      const testNames = failingTests.slice(0, 5).map(t => t.replace(/(?:FAIL|✕|×)\s+/, '').trim());
      facts.push({
        type: 'test_failure',
        content: `Failing tests: ${testNames.join(', ')}`,
        confidence: 0.9,
        context: { failingTests: testNames },
      });
    }
  }

  // 4. Dependency facts from package.json reads
  if (toolName === 'Read' && filePath.endsWith('package.json')) {
    try {
      const pkg = JSON.parse(outputStr);
      if (pkg.dependencies) {
        const deps = Object.keys(pkg.dependencies).slice(0, 10);
        facts.push({
          type: 'dependency',
          content: `Project dependencies: ${deps.join(', ')}${Object.keys(pkg.dependencies).length > 10 ? '...' : ''}`,
          confidence: 1.0,
          context: { dependencies: Object.keys(pkg.dependencies) },
        });
      }
      if (pkg.scripts) {
        facts.push({
          type: 'npm_scripts',
          content: `Available scripts: ${Object.keys(pkg.scripts).join(', ')}`,
          confidence: 1.0,
          context: { scripts: Object.keys(pkg.scripts) },
        });
      }
    } catch (e) {
      // Not valid JSON - ignore
    }
  }

  // 5. Write/Edit facts - what was changed
  if ((toolName === 'Write' || toolName === 'Edit') && filePath) {
    facts.push({
      type: 'file_modified',
      content: `Modified: ${path.basename(filePath)}`,
      confidence: 1.0,
      context: { file: filePath, action: toolName.toLowerCase() },
    });
  }

  return facts;
}

/**
 * Store extracted facts to brain memory for semantic retrieval
 */
async function storeFacts(facts, context = {}) {
  if (!facts || facts.length === 0) return;

  for (const fact of facts) {
    try {
      // Store to brain memory with embedding for semantic search
      await callBrainTool('brain_memory_store', {
        type: fact.type,
        content: fact.content,
        confidence: fact.confidence,
        context: {
          ...fact.context,
          ...context,
          extractedAt: Date.now(),
        },
        // Flag as extracted fact (vs observed pattern)
        metadata: { source: 'fact_extraction', version: 'p0.2' },
      });
    } catch (e) {
      // Storage failed - continue with other facts
    }
  }
}

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
// AUTO-JUDGMENT TRIGGERS INTEGRATION
// =============================================================================

/**
 * Map tool names and outcomes to trigger event types
 */
function mapToTriggerEventType(toolName, isError, toolInput) {
  // Error events
  if (isError) {
    return 'TOOL_ERROR';
  }

  // Git events via Bash
  if (toolName === 'Bash') {
    const command = toolInput.command || '';
    if (command.startsWith('git commit')) return 'COMMIT';
    if (command.startsWith('git push')) return 'PUSH';
    if (command.startsWith('git merge')) return 'MERGE';
  }

  // Code change events
  if (toolName === 'Write' || toolName === 'Edit') {
    return 'CODE_CHANGE';
  }

  // Default: generic tool use
  return 'TOOL_USE';
}

/**
 * Process tool event through the Trigger system
 * Non-blocking - fires async request
 */
function processTriggerEvent(toolName, toolInput, toolOutput, isError) {
  const eventType = mapToTriggerEventType(toolName, isError, toolInput);

  const event = {
    type: eventType,
    source: toolName,
    data: {
      tool: toolName,
      success: !isError,
      inputSize: JSON.stringify(toolInput).length,
      // Include relevant context based on event type
      ...(eventType === 'COMMIT' && { message: extractCommitMessage(toolInput.command) }),
      ...(eventType === 'CODE_CHANGE' && { file: toolInput.file_path || toolInput.filePath }),
      ...(isError && { error: extractErrorSummary(toolOutput) }),
    },
    timestamp: Date.now(),
  };

  // Fire async request to process triggers (non-blocking)
  callBrainTool('brain_triggers', {
    action: 'process',
    event,
  }).catch(() => {
    // Silently ignore errors - triggers should never block hooks
  });

  // Record pattern to brain memory (non-blocking)
  callBrainTool('brain_patterns', {
    action: 'record',
    pattern: {
      type: eventType.toLowerCase(),
      tool: toolName,
      success: !isError,
      timestamp: Date.now(),
    },
  }).catch(() => {
    // Silently ignore - pattern recording is optional
  });

  // ==========================================================================
  // LEARNING FEEDBACK - External validation (Ralph-inspired)
  // ==========================================================================

  // Send learning feedback for test results
  if (toolName === 'Bash' && toolInput.command) {
    const cmd = toolInput.command;
    const output = typeof toolOutput === 'string' ? toolOutput : '';

    // Detect test commands
    if (cmd.match(/npm\s+(run\s+)?test|jest|vitest|mocha|pytest|cargo\s+test|go\s+test/i)) {
      const testResult = parseTestOutput(output, isError);
      sendTestFeedback(testResult).catch(() => {});
    }

    // Detect successful commits
    if (cmd.startsWith('git commit') && !isError) {
      const commitHash = extractCommitHash(output);
      sendCommitFeedback({
        success: true,
        commitHash,
        hooksPassed: true,
        message: extractCommitMessage(cmd),
      }).catch(() => {});
    }

    // Detect build commands
    if (cmd.match(/npm\s+run\s+build|tsc|webpack|vite\s+build|cargo\s+build|go\s+build/i)) {
      sendBuildFeedback({
        success: !isError,
        duration: null,
      }).catch(() => {});
    }
  }
}

/**
 * Parse test output to extract pass/fail counts
 */
function parseTestOutput(output, isError) {
  let passed = !isError;
  let passCount = 0;
  let failCount = 0;
  let testSuite = 'unknown';

  // Jest/Vitest format
  const jestMatch = output.match(/Tests?:\s*(\d+)\s*passed(?:,\s*(\d+)\s*failed)?/i);
  if (jestMatch) {
    passCount = parseInt(jestMatch[1], 10) || 0;
    failCount = parseInt(jestMatch[2], 10) || 0;
    testSuite = output.includes('vitest') ? 'vitest' : 'jest';
    passed = failCount === 0;
    return { passed, passCount, failCount, testSuite };
  }

  // Mocha format
  const mochaMatch = output.match(/(\d+)\s*passing(?:.*?(\d+)\s*failing)?/i);
  if (mochaMatch) {
    passCount = parseInt(mochaMatch[1], 10) || 0;
    failCount = parseInt(mochaMatch[2], 10) || 0;
    testSuite = 'mocha';
    passed = failCount === 0;
    return { passed, passCount, failCount, testSuite };
  }

  // Fallback based on error state
  if (isError) {
    failCount = 1;
    passed = false;
  } else if (output.includes('PASS') || output.includes('passed') || output.includes('ok')) {
    passCount = 1;
    passed = true;
  }

  return { passed, passCount, failCount, testSuite };
}

/**
 * Extract commit hash from git output
 */
function extractCommitHash(output) {
  if (!output) return null;
  // Match: [branch abc1234] or abc1234
  const match = output.match(/\[[\w\-/]+\s+([a-f0-9]{7,})\]|^([a-f0-9]{40})$/im);
  return match ? (match[1] || match[2]) : null;
}

/**
 * Extract commit message from git commit command
 */
function extractCommitMessage(command) {
  if (!command) return '';
  const match = command.match(/-m\s+["']([^"']+)["']/);
  return match ? match[1] : '';
}

/**
 * Extract error summary from tool output
 */
function extractErrorSummary(output) {
  if (typeof output === 'string') {
    // First 200 chars of error
    return output.slice(0, 200);
  }
  return output?.error || output?.message || 'Unknown error';
}

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
    // ORCHESTRATION: Full orchestration through UnifiedOrchestrator (Phase 21)
    // Includes: Decision tracing, pattern recording, learning feedback
    // "Le chien observe et rapporte au cerveau collectif"
    // ═══════════════════════════════════════════════════════════════════════════
    let orchestration = null;
    orchestrateFull(
      `${toolName}: ${isError ? 'ERROR' : 'SUCCESS'}`,
      {
        eventType: 'tool_result',  // Post-tool event
        requestJudgment: isError,  // Only judge errors
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
    }).catch(() => {
      // Silently ignore - observation is best-effort
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // AUTO-ORCHESTRATOR: Automatic Dog consultation via CollectivePack
    // "Le collectif analyse" - All 11 Dogs see every tool result
    // ═══════════════════════════════════════════════════════════════════════════
    const autoOrchestrator = getAutoOrchestratorSync();
    autoOrchestrator.postAnalyze({
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
    }).catch(() => {
      // Best-effort - don't fail the hook
    });

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
      } catch (e) {
        // Harmonic feedback failed - continue without
        if (process.env.CYNIC_DEBUG) {
          console.error('[OBSERVE] Harmonic feedback error:', e.message);
        }
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

      // Output the judgment (will be shown to user)
      safeOutput({
        continue: true,
        message: formatted + refinementNote + thermoNote + dogNote,
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

    // Only show Dog if there's significant activity (not just silent observation)
    const showDog = isError || toolName === 'Write' || toolName === 'Edit' ||
                   toolName === 'Task' || (toolName === 'Bash' && toolInput.command?.length > 10);

    // ═══════════════════════════════════════════════════════════════════════════
    // INLINE STATUS BAR - Da'at: Making invisible visible
    // Shows: [🛡️ Guardian │ 58% │ 12 patterns │ ✨ flow]
    // ═══════════════════════════════════════════════════════════════════════════
    if (showDog && activeDog) {
      const inlineStatus = generateInlineStatus(activeDog, { showPsychology: true });
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
      } catch {
        // Learning visibility failed - continue without
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
