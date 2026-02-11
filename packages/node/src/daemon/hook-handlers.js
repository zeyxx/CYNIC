/**
 * CYNIC Daemon Hook Handlers
 *
 * Pure functions that handle hook events using in-memory singletons.
 * No MCP calls — all state is in RAM. Events flow to downstream consumers.
 *
 * Phase 4: Full thin hook coverage — SubagentStart/Stop, Error, Notification handlers
 *
 * "Le chien pense vite quand il vit déjà" - CYNIC
 *
 * @module @cynic/node/daemon/hook-handlers
 */

'use strict';

import fs from 'fs';
import { createLogger, PHI_INV, globalEventBus, EventType } from '@cynic/core';
import { validateIdentity, hasForbiddenPhrase, hasDogVoice } from '@cynic/core';
import { classifyPrompt } from '@cynic/core';
import { contextCompressor } from '../services/context-compressor.js';
import { injectionProfile } from '../services/injection-profile.js';
import { getCostLedger } from '../accounting/cost-ledger.js';
import { getModelIntelligence } from '../learning/model-intelligence.js';
import { getQLearningService } from '../orchestration/learning-service.js';
import { formatRichBanner, formatDigestMarkdown, saveDigest } from './digest-formatter.js';

const log = createLogger('DaemonHandlers');

// ═══════════════════════════════════════════════════════════════════════════════
// DANGER PATTERNS (extracted from guard.js)
// ═══════════════════════════════════════════════════════════════════════════════

const BASH_DANGER_PATTERNS = [
  { pattern: /rm\s+-rf\s+[/~]/, severity: 'critical', message: 'Recursive deletion from root or home directory', action: 'block' },
  { pattern: /rm\s+-rf\s+\*/, severity: 'critical', message: 'Wildcard recursive deletion', action: 'block' },
  { pattern: /rm\s+-rf\s+\.$/, severity: 'critical', message: 'Recursive deletion of current directory', action: 'block' },
  { pattern: /:\(\)\{\s*:\|:&\s*\};:/, severity: 'critical', message: 'Fork bomb detected', action: 'block' },
  { pattern: />\s*\/dev\/sd[a-z]/, severity: 'critical', message: 'Direct disk write', action: 'block' },
  { pattern: /mkfs\./, severity: 'critical', message: 'Filesystem format command', action: 'block' },
  { pattern: /dd\s+.*of=\/dev\/sd/, severity: 'critical', message: 'Direct disk write with dd', action: 'block' },
  { pattern: /git\s+push.*--force/, severity: 'high', message: 'Force push will rewrite remote history', action: 'warn' },
  { pattern: /git\s+push.*-f\s/, severity: 'high', message: 'Force push will rewrite remote history', action: 'warn' },
  { pattern: /git\s+reset\s+--hard/, severity: 'high', message: 'Hard reset will discard uncommitted changes', action: 'warn' },
  { pattern: /npm\s+publish/, severity: 'medium', message: 'Publishing to npm registry', action: 'warn' },
  { pattern: /DROP\s+(TABLE|DATABASE)/i, severity: 'critical', message: 'Database DROP command', action: 'block' },
  { pattern: /TRUNCATE/i, severity: 'high', message: 'TRUNCATE removes all data', action: 'warn' },
];

const WRITE_SENSITIVE_PATTERNS = [
  { pattern: /\.env/, message: 'Environment file with potential secrets' },
  { pattern: /credentials/, message: 'Credentials file' },
  { pattern: /\.pem$|\.key$/, message: 'Key/certificate file' },
  { pattern: /secret/i, message: 'File with "secret" in path' },
];

// ═══════════════════════════════════════════════════════════════════════════════
// SUBAGENT → DOG MAPPING (ported from scripts/hooks/spawn.js)
// ═══════════════════════════════════════════════════════════════════════════════

const SUBAGENT_TO_DOG = {
  'Explore': { dog: 'SCOUT', sefirah: 'Netzach' },
  'Plan': { dog: 'ARCHITECT', sefirah: 'Chesed' },
  'Bash': { dog: 'CARTOGRAPHER', sefirah: 'Malkhut' },
  'general-purpose': { dog: 'CYNIC', sefirah: 'Keter' },
  'cynic-guardian': { dog: 'GUARDIAN', sefirah: 'Gevurah' },
  'cynic-architect': { dog: 'ARCHITECT', sefirah: 'Chesed' },
  'cynic-analyst': { dog: 'ANALYST', sefirah: 'Binah' },
  'cynic-scout': { dog: 'SCOUT', sefirah: 'Netzach' },
  'cynic-sage': { dog: 'SAGE', sefirah: 'Chochmah' },
  'cynic-scholar': { dog: 'SCHOLAR', sefirah: 'Daat' },
  'cynic-oracle': { dog: 'ORACLE', sefirah: 'Tiferet' },
  'cynic-deployer': { dog: 'DEPLOYER', sefirah: 'Hod' },
  'cynic-janitor': { dog: 'JANITOR', sefirah: 'Yesod' },
  'cynic-cartographer': { dog: 'CARTOGRAPHER', sefirah: 'Malkhut' },
  'cynic-reviewer': { dog: 'ANALYST', sefirah: 'Binah' },
  'cynic-tester': { dog: 'GUARDIAN', sefirah: 'Gevurah' },
  'cynic-simplifier': { dog: 'JANITOR', sefirah: 'Yesod' },
  'cynic-integrator': { dog: 'CARTOGRAPHER', sefirah: 'Malkhut' },
  'cynic-doc': { dog: 'SCHOLAR', sefirah: 'Daat' },
  'cynic-librarian': { dog: 'SCHOLAR', sefirah: 'Daat' },
  'cynic-solana-expert': { dog: 'SAGE', sefirah: 'Chochmah' },
  'cynic-archivist': { dog: 'SCHOLAR', sefirah: 'Daat' },
};

/** Active subagents — persistent in daemon RAM across hook invocations */
const activeSubagents = new Map();

// ═══════════════════════════════════════════════════════════════════════════════
// ERROR PATTERNS (ported from scripts/hooks/error.js)
// ═══════════════════════════════════════════════════════════════════════════════

const ERROR_PATTERNS = [
  { pattern: /ENOENT|no such file|file not found/i, type: 'file_not_found', severity: 'medium', recoverable: true },
  { pattern: /EACCES|permission denied|access denied/i, type: 'permission_denied', severity: 'high', recoverable: false },
  { pattern: /EEXIST|already exists/i, type: 'file_exists', severity: 'low', recoverable: true },
  { pattern: /ENOSPC|no space left/i, type: 'disk_full', severity: 'critical', recoverable: false },
  { pattern: /ECONNREFUSED|connection refused/i, type: 'connection_refused', severity: 'high', recoverable: true },
  { pattern: /ETIMEDOUT|timed out|timeout/i, type: 'timeout', severity: 'medium', recoverable: true },
  { pattern: /ENOTFOUND|DNS|getaddrinfo/i, type: 'dns_error', severity: 'high', recoverable: true },
  { pattern: /SyntaxError|Unexpected token/i, type: 'syntax_error', severity: 'high', recoverable: false },
  { pattern: /TypeError/i, type: 'type_error', severity: 'high', recoverable: false },
  { pattern: /ReferenceError|is not defined/i, type: 'reference_error', severity: 'high', recoverable: false },
  { pattern: /fatal:|error: pathspec/i, type: 'git_error', severity: 'medium', recoverable: true },
  { pattern: /merge conflict|CONFLICT/i, type: 'merge_conflict', severity: 'high', recoverable: false },
  { pattern: /command not found|not recognized/i, type: 'command_not_found', severity: 'medium', recoverable: false },
  { pattern: /npm ERR!|yarn error/i, type: 'package_manager_error', severity: 'medium', recoverable: true },
  { pattern: /exit code [1-9]|exited with code/i, type: 'exit_code_error', severity: 'medium', recoverable: true },
  { pattern: /401|unauthorized|authentication/i, type: 'auth_error', severity: 'high', recoverable: true },
  { pattern: /403|forbidden/i, type: 'forbidden', severity: 'high', recoverable: false },
  { pattern: /429|rate limit|too many requests/i, type: 'rate_limit', severity: 'high', recoverable: true },
  { pattern: /500|internal server error/i, type: 'server_error', severity: 'high', recoverable: true },
  { pattern: /heap out of memory|ENOMEM/i, type: 'out_of_memory', severity: 'critical', recoverable: false },
  { pattern: /EMFILE|too many open files/i, type: 'too_many_files', severity: 'high', recoverable: true },
];

/** Error history — persistent in daemon RAM for loop detection */
const errorHistory = [];
const MAX_ERROR_HISTORY = 50;

/** Consecutive error counter — resets on success */
let consecutiveErrors = 0;

// ═══════════════════════════════════════════════════════════════════════════════
// NOTIFICATION TYPES (ported from scripts/hooks/notify.js)
// ═══════════════════════════════════════════════════════════════════════════════

const NOTIFICATION_TYPES = {
  error: { severity: 'high', category: 'error' },
  warning: { severity: 'medium', category: 'warning' },
  info: { severity: 'low', category: 'info' },
  success: { severity: 'low', category: 'success' },
  progress: { severity: 'low', category: 'progress' },
  complete: { severity: 'low', category: 'complete' },
  system: { severity: 'medium', category: 'system' },
  timeout: { severity: 'high', category: 'timeout' },
  security: { severity: 'critical', category: 'security' },
  blocked: { severity: 'high', category: 'blocked' },
};

/** Notification burst tracking — persistent in daemon RAM */
const notificationHistory = [];
const BURST_WINDOW_MS = 5 * 60 * 1000;
const BURST_THRESHOLD = 5;

// ═══════════════════════════════════════════════════════════════════════════════
// EVENT ROUTER — dispatches hook events to handlers
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Route a hook event to the appropriate handler
 *
 * @param {string} event - Hook event name (UserPromptSubmit, PreToolUse, etc.)
 * @param {Object} hookInput - Parsed JSON from hook stdin
 * @returns {Promise<Object>} Hook output JSON
 */
export async function handleHookEvent(event, hookInput) {
  const startTime = Date.now();

  try {
    let result;

    switch (event) {
      case 'UserPromptSubmit':
        result = await handlePerceive(hookInput);
        break;
      case 'PreToolUse':
        result = await handleGuard(hookInput);
        break;
      case 'PostToolUse':
        result = await handleObserve(hookInput);
        break;
      case 'SessionStart':
        result = await handleAwaken(hookInput);
        break;
      case 'SessionEnd':
        result = await handleSleep(hookInput);
        break;
      case 'Stop':
        result = await handleStop(hookInput);
        break;
      case 'SubagentStart':
        result = await handleSubagentStart(hookInput);
        break;
      case 'SubagentStop':
        result = await handleSubagentStop(hookInput);
        break;
      case 'Error':
        result = await handleError(hookInput);
        break;
      case 'Notification':
        result = await handleNotification(hookInput);
        break;
      default:
        result = { continue: true };
        break;
    }

    const duration = Date.now() - startTime;
    log.debug(`Hook ${event} handled`, { duration });

    return result;
  } catch (err) {
    log.error(`Hook ${event} failed`, { error: err.message });
    return { continue: true, error: err.message };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// PERCEIVE — UserPromptSubmit
// Classifies prompt, injects context, detects dangers
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Handle UserPromptSubmit — classify and inject context
 *
 * @param {Object} hookInput - { prompt, ... }
 * @returns {Promise<Object>} { continue: true, message?: string }
 */
async function handlePerceive(hookInput) {
  const prompt = hookInput?.prompt || '';
  if (!prompt.trim()) return { continue: true };

  const sections = [];

  // 1. Classify prompt
  let classification;
  try {
    classification = classifyPrompt(prompt, {
      sessionHistory: [],
      hasActivePlan: false,
    });
  } catch {
    classification = { intent: 'build', domain: 'general', complexity: 'medium' };
  }

  // 2. Danger detection
  const dangerWarning = detectDanger(prompt);
  if (dangerWarning) {
    sections.push(dangerWarning);
  }

  // 3. Context compression — inject less as CYNIC learns
  let compressionActive = false;
  try {
    contextCompressor.start();
    compressionActive = true;
  } catch { /* non-blocking */ }

  // 4. Injection profile — learned activation rates
  try {
    injectionProfile.start();
  } catch { /* non-blocking */ }

  // 5. Emit perception event for learning loops
  try {
    globalEventBus.emit(EventType.USER_FEEDBACK, {
      type: 'prompt_perceived',
      prompt: prompt.substring(0, 200),
      classification,
      timestamp: Date.now(),
    });
  } catch { /* non-blocking */ }

  // 6. Build system-reminder message
  if (sections.length > 0) {
    return {
      continue: true,
      message: sections.join('\n\n'),
    };
  }

  return { continue: true };
}

// ═══════════════════════════════════════════════════════════════════════════════
// GUARD — PreToolUse
// Blocks dangerous operations, warns on risky ones
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Handle PreToolUse — guard against danger
 *
 * @param {Object} hookInput - { tool_name, tool_input, ... }
 * @returns {Promise<Object>} { continue, blocked?, blockReason?, issues[] }
 */
async function handleGuard(hookInput) {
  const toolName = hookInput?.tool_name || '';
  const toolInput = hookInput?.tool_input || {};
  const output = {
    continue: true,
    blocked: false,
    blockReason: null,
    issues: [],
  };

  // Check Bash commands for danger patterns
  if (toolName === 'Bash' && toolInput.command) {
    const command = toolInput.command;

    for (const { pattern, severity, message, action } of BASH_DANGER_PATTERNS) {
      if (pattern.test(command)) {
        output.issues.push({ severity, message, action });

        if (action === 'block') {
          output.blocked = true;
          output.continue = false;
          output.blockReason = `*GROWL* BLOCKED: ${message}`;
          output.decision = 'block';
          output.reason = message;
          log.warn('Dangerous command blocked', { command: command.substring(0, 100), message });
          return output;
        }
      }
    }
  }

  // Check Write/Edit for sensitive paths
  if ((toolName === 'Write' || toolName === 'Edit') && toolInput.file_path) {
    for (const { pattern, message } of WRITE_SENSITIVE_PATTERNS) {
      if (pattern.test(toolInput.file_path)) {
        output.issues.push({ severity: 'medium', message, action: 'warn' });
      }
    }
  }

  return output;
}

// ═══════════════════════════════════════════════════════════════════════════════
// OBSERVE — PostToolUse
// Silently learns from tool outcomes
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Handle PostToolUse — observe and learn
 *
 * @param {Object} hookInput - { tool_name, tool_input, tool_output, ... }
 * @returns {Promise<Object>} { continue: true }
 */
async function handleObserve(hookInput) {
  const toolName = hookInput?.tool_name || '';
  const toolInput = hookInput?.tool_input || {};

  // Emit tool observation for learning loops
  try {
    globalEventBus.emit(EventType.TOOL_CALLED || 'tool:called', {
      tool: toolName,
      input: typeof toolInput === 'object' ? toolInput : {},
      timestamp: Date.now(),
    });
  } catch { /* non-blocking */ }

  return { continue: true };
}

// ═══════════════════════════════════════════════════════════════════════════════
// AWAKEN — SessionStart
// Boots daemon-side services, prepares injection
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Handle SessionStart — wake up
 *
 * @param {Object} hookInput - Session context
 * @returns {Promise<Object>} { continue: true, message?: string }
 */
async function handleAwaken(hookInput) {
  log.info('Session starting — daemon already warm');

  // Context compression: start tracking this session
  try { contextCompressor.start(); } catch { /* non-blocking */ }
  try { injectionProfile.start(); } catch { /* non-blocking */ }

  return {
    continue: true,
    message: '*sniff* CYNIC daemon is awake. Singletons warm. Ready.',
  };
}

// ═══════════════════════════════════════════════════════════════════════════════
// SLEEP — SessionEnd
// Persists session state, flushes buffers
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Handle SessionEnd — go to sleep (but daemon stays running)
 *
 * @param {Object} hookInput - Session end context
 * @returns {Promise<Object>} { continue: true }
 */
async function handleSleep(hookInput) {
  log.info('Session ending — daemon stays warm');

  // Record session quality for context compression outcome verification
  try {
    contextCompressor.recordSessionEnd();
  } catch { /* non-blocking */ }

  // CostLedger session boundary — persist lifetime stats
  try {
    getCostLedger().endSession();
  } catch { /* non-blocking */ }

  return { continue: true };
}

// ═══════════════════════════════════════════════════════════════════════════════
// STOP — Stop event
// Two-phase: (1) Ralph-loop check (can block), (2) Response digest (non-blocking)
// ═══════════════════════════════════════════════════════════════════════════════

/** Ralph-loop state file (relative to project root) */
const RALPH_STATE_FILE = '.claude/ralph-loop.local.md';

/**
 * Handle Stop event
 *
 * Phase 1: Ralph-loop check — can BLOCK the stop if a loop is active.
 * Phase 2: Response quality judgment + session digest (non-blocking).
 * Phase 3: Q-Learning endEpisode + markdown export + rich banner.
 *
 * @param {Object} hookInput - Stop context (transcript_path, etc.)
 * @returns {Promise<Object>} { continue: true } or { decision: 'block', reason: string }
 */
async function handleStop(hookInput) {
  // Phase 1: Ralph-loop check (can block)
  try {
    const ralphResult = checkRalphLoop(hookInput);
    if (ralphResult) {
      log.info('Ralph loop blocking stop', { iteration: ralphResult.iteration });
      return ralphResult;
    }
  } catch (err) {
    log.debug('Ralph-loop check failed', { error: err.message });
  }

  // Phase 2: Response digest (non-blocking — always continues)
  const digest = await buildSessionDigest(hookInput);

  // Phase 3: Q-Learning endEpisode + flush
  await endQLearningEpisode(digest);

  // Emit rich SESSION_ENDED for downstream consumers
  try {
    globalEventBus.emit(EventType.SESSION_ENDED || 'session:ended', {
      type: 'session_ended',
      digest: {
        identity: digest.identity,
        sessionStats: digest.sessionStats,
        qLearning: digest.qLearning,
      },
      timestamp: Date.now(),
    });
  } catch { /* non-blocking */ }

  // Export markdown digest to ~/.cynic/digests/
  try {
    const markdown = formatDigestMarkdown(digest);
    const savedPath = saveDigest(markdown);
    if (savedPath) {
      log.debug('Digest exported', { path: savedPath });
    }
  } catch { /* non-blocking — never block session end */ }

  // Return with rich digest banner
  if (digest.banner) {
    return { continue: true, message: digest.banner };
  }

  return { continue: true };
}

// ═══════════════════════════════════════════════════════════════════════════════
// RALPH-LOOP — Ported from scripts/hooks/ralph-loop.js
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Check if a ralph-loop is active and should block the stop.
 *
 * Reads .claude/ralph-loop.local.md, checks iteration vs max,
 * reads last assistant message, checks completion promise.
 *
 * @param {Object} hookInput - { transcript_path, ... }
 * @returns {Object|null} Block result or null to continue
 */
function checkRalphLoop(hookInput) {
  // Check if ralph-loop state file exists
  if (!fs.existsSync(RALPH_STATE_FILE)) {
    return null; // No active loop
  }

  // Read and parse state file
  let stateContent;
  try {
    stateContent = fs.readFileSync(RALPH_STATE_FILE, 'utf-8');
  } catch {
    cleanupRalphState();
    return null;
  }

  const frontmatter = parseRalphFrontmatter(stateContent);
  const iteration = parseInt(frontmatter.iteration, 10);
  const maxIterations = parseInt(frontmatter.max_iterations, 10);
  const completionPromise = frontmatter.completion_promise;

  // Validate numeric fields
  if (isNaN(iteration) || isNaN(maxIterations)) {
    cleanupRalphState();
    return null;
  }

  // Check if max iterations reached
  if (maxIterations > 0 && iteration >= maxIterations) {
    cleanupRalphState();
    return null;
  }

  // Get transcript path from hook input
  const transcriptPath = hookInput?.transcript_path;
  if (!transcriptPath || !fs.existsSync(transcriptPath)) {
    cleanupRalphState();
    return null;
  }

  // Read last assistant message
  const lastOutput = getLastAssistantMessage(transcriptPath);
  if (!lastOutput) {
    cleanupRalphState();
    return null;
  }

  // Check for completion promise
  if (completionPromise && completionPromise !== 'null') {
    const promiseMatch = lastOutput.match(/<promise>([\s\S]*?)<\/promise>/);
    if (promiseMatch) {
      const promiseText = promiseMatch[1].trim().replace(/\s+/g, ' ');
      if (promiseText === completionPromise) {
        cleanupRalphState();
        return null; // Promise fulfilled — allow stop
      }
    }
  }

  // Not complete — block stop and continue loop
  const nextIteration = iteration + 1;
  const promptText = extractRalphPrompt(stateContent);

  if (!promptText) {
    cleanupRalphState();
    return null;
  }

  // Update iteration in state file
  try {
    const updatedContent = stateContent.replace(
      /^iteration: .*/m,
      `iteration: ${nextIteration}`
    );
    fs.writeFileSync(RALPH_STATE_FILE, updatedContent, 'utf-8');
  } catch {
    cleanupRalphState();
    return null;
  }

  // Build system message
  let systemMsg;
  if (completionPromise && completionPromise !== 'null') {
    systemMsg = `Ralph iteration ${nextIteration} | To stop: output <promise>${completionPromise}</promise> (ONLY when TRUE)`;
  } else {
    systemMsg = `Ralph iteration ${nextIteration} | No completion promise — loop runs until max_iterations`;
  }

  return {
    decision: 'block',
    reason: promptText,
    systemMessage: systemMsg,
    iteration: nextIteration,
  };
}

/**
 * Parse YAML frontmatter from ralph-loop state file.
 * @param {string} content
 * @returns {Object}
 */
function parseRalphFrontmatter(content) {
  const match = content.match(/^---\n([\s\S]*?)\n---/);
  if (!match) return {};

  const result = {};
  for (const line of match[1].split('\n')) {
    const colonIndex = line.indexOf(':');
    if (colonIndex > 0) {
      const key = line.slice(0, colonIndex).trim();
      let value = line.slice(colonIndex + 1).trim();
      if ((value.startsWith('"') && value.endsWith('"')) ||
          (value.startsWith("'") && value.endsWith("'"))) {
        value = value.slice(1, -1);
      }
      result[key] = value;
    }
  }
  return result;
}

/**
 * Extract prompt text (everything after closing ---).
 * @param {string} content
 * @returns {string}
 */
function extractRalphPrompt(content) {
  const parts = content.split(/^---$/m);
  if (parts.length >= 3) {
    return parts.slice(2).join('---').trim();
  }
  return '';
}

/**
 * Read last assistant message from JSONL transcript.
 * @param {string} transcriptPath
 * @returns {string|null}
 */
function getLastAssistantMessage(transcriptPath) {
  try {
    const content = fs.readFileSync(transcriptPath, 'utf-8');
    const lines = content.trim().split('\n');

    for (let i = lines.length - 1; i >= 0; i--) {
      try {
        const entry = JSON.parse(lines[i]);
        if (entry.role === 'assistant' || entry.message?.role === 'assistant') {
          const message = entry.message || entry;
          if (message.content && Array.isArray(message.content)) {
            return message.content
              .filter(block => block.type === 'text')
              .map(block => block.text)
              .join('\n');
          }
        }
      } catch { /* skip invalid JSON */ }
    }
  } catch { /* transcript read failed */ }

  return null;
}

/**
 * Clean up ralph-loop state file.
 */
function cleanupRalphState() {
  try {
    if (fs.existsSync(RALPH_STATE_FILE)) {
      fs.unlinkSync(RALPH_STATE_FILE);
    }
  } catch { /* ignore */ }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SESSION DIGEST — Response quality + session stats + Q-Learning
// Replaces scripts/hooks/digest.js (1213 lines → daemon-native)
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Build a full session digest for the Stop event.
 *
 * Scores response quality, gathers session stats from warm singletons,
 * and formats a rich digest banner for console output.
 *
 * @param {Object} hookInput - { transcript_path, ... }
 * @returns {Promise<Object>} Digest data with optional banner
 */
async function buildSessionDigest(hookInput) {
  const digest = {
    identity: null,
    sessionStats: null,
    qLearning: null,
    banner: null,
  };

  // 1. Identity validation on last response (if transcript available)
  const transcriptPath = hookInput?.transcript_path;
  if (transcriptPath) {
    try {
      const lastMessage = getLastAssistantMessage(transcriptPath);
      if (lastMessage) {
        digest.identity = validateIdentity(lastMessage, {
          requireDogVoice: true,
          checkConfidence: true,
          checkForbidden: true,
          isSubstantive: lastMessage.length > 50,
        });
      }
    } catch { /* non-blocking */ }
  }

  // 2. Session stats from warm singletons
  try {
    const costLedger = getCostLedger();
    digest.sessionStats = {
      cost: costLedger.getSessionSummary(),
    };
  } catch { /* non-blocking */ }

  try {
    const mi = getModelIntelligence();
    if (digest.sessionStats) {
      digest.sessionStats.modelIntelligence = mi.getStats();
    }
  } catch { /* non-blocking */ }

  // 3. Q-Learning stats (if service is warm)
  try {
    const qlService = getQLearningService();
    if (qlService) {
      const stats = qlService.getStats();
      digest.qLearning = {
        states: stats.qTableStats?.states || 0,
        episodes: stats.episodes || 0,
        accuracy: stats.accuracy,
        flushed: false,
      };
    }
  } catch { /* non-blocking — Q-Learning may not be initialized */ }

  // 4. Format rich banner (from digest-formatter)
  try {
    digest.banner = formatRichBanner(digest);
  } catch {
    // Fallback to compact banner if rich formatting fails
    digest.banner = formatCompactBanner(digest);
  }

  return digest;
}

/**
 * End Q-Learning episode with session outcome, then flush to disk.
 *
 * Ported from scripts/hooks/digest.js (lines 1030-1073).
 * The Q-Learning service tracks state→action→reward sequences;
 * endEpisode closes the current episode with a terminal reward.
 *
 * @param {Object} digest - Digest data from buildSessionDigest()
 */
async function endQLearningEpisode(digest) {
  try {
    const qlService = getQLearningService();
    if (!qlService) return;

    // Build outcome from digest data
    const cost = digest.sessionStats?.cost;
    const identity = digest.identity;
    const outcome = {
      success: identity?.valid !== false,
      confidence: identity?.compliance ?? 0.5,
      qScore: Math.round((identity?.compliance ?? 0.5) * 100),
      toolsUsed: cost?.operations ?? 0,
      error: false,
    };

    // End episode
    if (qlService.endEpisode) {
      await qlService.endEpisode(outcome);
    }

    // Flush Q-table to disk
    if (qlService.flush) {
      await qlService.flush();
      if (digest.qLearning) {
        digest.qLearning.flushed = true;
      }
    }

    log.debug('Q-Learning episode ended + flushed');
  } catch (err) {
    log.debug('Q-Learning endEpisode failed', { error: err.message });
  }
}

/**
 * Compact fallback banner (one-liner) when rich formatting fails.
 *
 * @param {Object} digest - From buildSessionDigest()
 * @returns {string|null} Formatted banner or null
 */
function formatCompactBanner(digest) {
  const parts = [];

  if (digest.identity) {
    const { valid, compliance, violations, warnings } = digest.identity;
    if (!valid) {
      const violationSummary = violations.map(v => v.found || v.type).join(', ');
      parts.push(`Identity: ${violations.length} violations (${violationSummary})`);
    } else if (warnings?.length > 0) {
      parts.push(`Identity: warnings (${warnings.map(w => w.type).join(', ')})`);
    } else {
      parts.push(`Identity: clean (${Math.round((compliance || 0) * 100)}%)`);
    }
  }

  if (digest.sessionStats?.cost) {
    const { operations, cost, durationMinutes } = digest.sessionStats.cost;
    if (operations > 0) {
      parts.push(`Session: ${operations} ops, $${cost.total.toFixed(4)}, ${durationMinutes}min`);
    }
  }

  if (parts.length === 0) return null;
  return `*yawn* Session digest: ${parts.join(' | ')}`;
}

// ═══════════════════════════════════════════════════════════════════════════════
// SUBAGENT START — SubagentStart event
// Maps subagent types to Sefirot dogs, tracks active pack
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Handle SubagentStart — a dog joins the pack
 *
 * @param {Object} hookInput - { agent_id, subagent_type, prompt, model, ... }
 * @returns {Promise<Object>} { continue: true, agentInfo, message }
 */
async function handleSubagentStart(hookInput) {
  const agentId = hookInput?.agent_id || hookInput?.agentId || `agent_${Date.now()}`;
  const subagentType = hookInput?.subagent_type || hookInput?.subagentType || 'general-purpose';
  const prompt = hookInput?.prompt || '';
  const model = hookInput?.model || 'default';

  const mapping = SUBAGENT_TO_DOG[subagentType] || { dog: 'CYNIC', sefirah: 'Keter' };

  const agentInfo = {
    id: agentId,
    type: subagentType,
    dog: mapping.dog,
    sefirah: mapping.sefirah,
    model,
    startTime: Date.now(),
    promptLength: prompt.length,
  };
  activeSubagents.set(agentId, agentInfo);

  // Emit for learning loops
  try {
    globalEventBus.emit(EventType.SUBAGENT_STARTED || 'subagent:started', {
      type: 'subagent_spawn',
      agentId,
      subagentType,
      dog: mapping.dog,
      sefirah: mapping.sefirah,
      model,
      promptLength: prompt.length,
      timestamp: Date.now(),
    });
  } catch { /* non-blocking */ }

  log.debug('Subagent started', { agentId, dog: mapping.dog });

  return {
    continue: true,
    agentInfo,
    message: `${mapping.dog} (${mapping.sefirah}) dispatched`,
  };
}

// ═══════════════════════════════════════════════════════════════════════════════
// SUBAGENT STOP — SubagentStop event
// Records outcome, cleans up tracking
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Handle SubagentStop — a dog returns from the field
 *
 * @param {Object} hookInput - { agent_id, success, result, duration_ms, ... }
 * @returns {Promise<Object>} { continue: true, agentInfo, message }
 */
async function handleSubagentStop(hookInput) {
  const agentId = hookInput?.agent_id || hookInput?.agentId;
  const success = hookInput?.success !== false;
  const durationMs = hookInput?.duration_ms || hookInput?.durationMs || 0;

  const agentInfo = activeSubagents.get(agentId) || {
    dog: 'UNKNOWN', sefirah: 'Unknown', type: 'unknown', startTime: Date.now() - durationMs,
  };
  const actualDuration = durationMs || (Date.now() - agentInfo.startTime);

  activeSubagents.delete(agentId);

  // Emit for learning loops
  try {
    globalEventBus.emit(EventType.SUBAGENT_STOPPED || 'subagent:stopped', {
      type: 'subagent_complete',
      agentId,
      dog: agentInfo.dog,
      sefirah: agentInfo.sefirah,
      success,
      durationMs: actualDuration,
      timestamp: Date.now(),
    });
  } catch { /* non-blocking */ }

  // Reset consecutive errors on successful subagent
  if (success) consecutiveErrors = 0;

  log.debug('Subagent stopped', { agentId, dog: agentInfo.dog, success, durationMs: actualDuration });

  return {
    continue: true,
    agentInfo,
    message: success
      ? `${agentInfo.dog} returns (${Math.round(actualDuration / 1000)}s)`
      : `${agentInfo.dog} encountered issues (${Math.round(actualDuration / 1000)}s)`,
  };
}

// ═══════════════════════════════════════════════════════════════════════════════
// ERROR — Error event
// Classifies errors, detects loops, tracks escalation
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Handle Error event — the dog smells trouble
 *
 * @param {Object} hookInput - { tool_name, error, tool_input, ... }
 * @returns {Promise<Object>} { continue: true, classification, context, patterns, escalation }
 */
async function handleError(hookInput) {
  const toolName = hookInput?.tool_name || hookInput?.toolName || '';
  const errorMessage = hookInput?.error || hookInput?.error_message || hookInput?.errorMessage || '';
  const toolInput = hookInput?.tool_input || hookInput?.toolInput || {};

  // Classify error
  const classification = classifyError(errorMessage);

  // Extract context
  const context = extractErrorContext(errorMessage, toolInput);

  // Track consecutive errors
  consecutiveErrors++;

  // Detect repeated patterns (loop detection)
  const patterns = detectErrorPatterns(errorMessage, classification);

  // Build escalation level
  let escalation = null;
  if (consecutiveErrors >= 5) {
    escalation = 'strict';
  } else if (consecutiveErrors >= 3) {
    escalation = 'cautious';
  }

  // Emit for learning loops
  try {
    globalEventBus.emit(EventType.ERROR_OCCURRED || 'error:occurred', {
      type: 'tool_error',
      tool: toolName,
      errorType: classification.type,
      severity: classification.severity,
      recoverable: classification.recoverable,
      consecutiveErrors,
      patterns: patterns.map(p => p.signature),
      timestamp: Date.now(),
    });
  } catch { /* non-blocking */ }

  log.debug('Error handled', { tool: toolName, type: classification.type, consecutive: consecutiveErrors });

  // Build output
  const output = {
    continue: true,
    tool: { name: toolName },
    error: errorMessage.slice(0, 500),
    classification,
    context,
    patterns,
    escalation,
    consecutiveErrors,
  };

  // Add message for high severity or repeated errors
  if (classification.severity === 'critical' || patterns.length > 0) {
    const parts = [`${classification.type} (${classification.severity})`];
    if (context.suggestion) parts.push(context.suggestion);
    if (patterns.length > 0) parts.push(patterns[0].suggestion);
    output.message = `*growl* Error: ${parts.join(' | ')}`;
  }

  return output;
}

// ═══════════════════════════════════════════════════════════════════════════════
// NOTIFICATION — Notification event
// Classifies, detects bursts, escalates high-severity
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Handle Notification event — the dog listens
 *
 * @param {Object} hookInput - { type, title, message, source, ... }
 * @returns {Promise<Object>} { continue: true, classification, burst }
 */
async function handleNotification(hookInput) {
  const notifType = (hookInput?.notification_type || hookInput?.type || 'info').toLowerCase();
  const title = hookInput?.title || '';
  const message = hookInput?.message || hookInput?.content || '';
  const source = hookInput?.source || 'unknown';

  // Classify
  const classification = classifyNotification(notifType, title, message);

  // Detect burst
  const burst = detectNotificationBurst(classification.type);

  // Emit for learning loops
  try {
    globalEventBus.emit(EventType.NOTIFICATION_RECEIVED || 'notification:received', {
      type: 'notification',
      notificationType: classification.type,
      category: classification.category,
      severity: classification.severity,
      source,
      burst: burst.detected,
      timestamp: Date.now(),
    });
  } catch { /* non-blocking */ }

  log.debug('Notification handled', { type: classification.type, severity: classification.severity });

  const output = {
    continue: true,
    classification: {
      type: classification.type,
      category: classification.category,
      severity: classification.severity,
    },
    burst: burst.detected ? burst : null,
  };

  // Show message for high severity or bursts
  if (classification.severity === 'high' || classification.severity === 'critical' || burst.detected) {
    const burstNote = burst.detected ? ` (BURST: ${burst.count}x in 5min)` : '';
    output.message = `*ears perk* ${classification.type.toUpperCase()}: ${title || message.substring(0, 60)}${burstNote}`;
  }

  return output;
}

// ═══════════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Classify an error message against known patterns.
 * @param {string} errorMessage
 * @returns {{ type: string, severity: string, recoverable: boolean }}
 */
function classifyError(errorMessage) {
  for (const { pattern, type, severity, recoverable } of ERROR_PATTERNS) {
    if (pattern.test(errorMessage)) {
      return { type, severity, recoverable };
    }
  }
  return { type: 'unknown', severity: 'medium', recoverable: true };
}

/**
 * Extract context from error (file, line, command, suggestion).
 * @param {string} error
 * @param {Object} toolInput
 * @returns {Object}
 */
function extractErrorContext(error, toolInput) {
  const context = { file: null, line: null, command: null, suggestion: null };

  const fileMatch = error.match(/(?:at |in |file |path[: ]*)([^\s:]+\.[a-z]+)/i);
  if (fileMatch) context.file = fileMatch[1];

  const lineMatch = error.match(/:(\d+)(?::(\d+))?/);
  if (lineMatch) context.line = parseInt(lineMatch[1], 10);

  if (toolInput?.command) context.command = toolInput.command.slice(0, 100);

  const classification = classifyError(error);
  const suggestions = {
    file_not_found: 'Check file path spelling. Use Glob to find it.',
    permission_denied: 'Check file permissions.',
    syntax_error: 'Review code for syntax issues.',
    timeout: 'Consider increasing timeout or checking network.',
    rate_limit: 'Wait before retrying.',
    merge_conflict: 'Resolve merge conflicts manually.',
    command_not_found: 'Install required package or check PATH.',
  };
  context.suggestion = suggestions[classification.type] || 'Review the error and adjust approach.';

  return context;
}

/**
 * Detect repeated error patterns (loop detection).
 * @param {string} errorMessage
 * @param {{ type: string }} classification
 * @returns {Array}
 */
function detectErrorPatterns(errorMessage, classification) {
  const patterns = [];

  errorHistory.push({ type: classification.type, timestamp: Date.now() });
  if (errorHistory.length > MAX_ERROR_HISTORY) errorHistory.shift();

  // Same error type 3+ times in last 5 entries = loop
  const recent = errorHistory.slice(-5);
  const sameCount = recent.filter(e => e.type === classification.type).length;
  if (sameCount >= 3) {
    patterns.push({
      type: 'repeated_error',
      signature: `repeated_${classification.type}`,
      description: `Same error type repeated ${sameCount} times`,
      severity: 'high',
      suggestion: 'Breaking loop: try a different approach',
    });
  }

  // 5+ high/critical in last 10 = escalating
  const severities = errorHistory.slice(-10);
  const critCount = severities.filter(e => {
    const cls = classifyError(e.type); // Re-classify to get severity
    return cls.severity === 'critical' || cls.severity === 'high';
  }).length;
  if (critCount >= 5) {
    patterns.push({
      type: 'escalating_errors',
      signature: 'escalating_severity',
      description: 'Multiple high-severity errors in succession',
      severity: 'critical',
      suggestion: 'Consider pausing and reviewing the approach',
    });
  }

  return patterns;
}

/**
 * Classify a notification by type and content.
 * @param {string} type
 * @param {string} title
 * @param {string} message
 * @returns {{ type: string, category: string, severity: string }}
 */
function classifyNotification(type, title, message) {
  if (NOTIFICATION_TYPES[type]) {
    return { type, ...NOTIFICATION_TYPES[type] };
  }

  const text = `${title} ${message}`.toLowerCase();
  if (text.includes('error') || text.includes('failed')) return { type: 'error', ...NOTIFICATION_TYPES.error };
  if (text.includes('warning') || text.includes('deprecated')) return { type: 'warning', ...NOTIFICATION_TYPES.warning };
  if (text.includes('timeout') || text.includes('timed out')) return { type: 'timeout', ...NOTIFICATION_TYPES.timeout };
  if (text.includes('security') || text.includes('vulnerability')) return { type: 'security', ...NOTIFICATION_TYPES.security };
  if (text.includes('blocked') || text.includes('denied')) return { type: 'blocked', ...NOTIFICATION_TYPES.blocked };
  if (text.includes('success') || text.includes('completed')) return { type: 'success', ...NOTIFICATION_TYPES.success };

  return { type: 'info', ...NOTIFICATION_TYPES.info };
}

/**
 * Detect notification bursts (same type repeatedly in window).
 * @param {string} type
 * @returns {{ detected: boolean, type?: string, count?: number, message?: string }}
 */
function detectNotificationBurst(type) {
  const now = Date.now();

  notificationHistory.push({ type, timestamp: now });

  // Prune old entries
  while (notificationHistory.length > 0 && now - notificationHistory[0].timestamp > BURST_WINDOW_MS) {
    notificationHistory.shift();
  }

  const sameType = notificationHistory.filter(n => n.type === type);
  if (sameType.length >= BURST_THRESHOLD) {
    return {
      detected: true,
      type,
      count: sameType.length,
      message: `Notification burst: ${type} (${sameType.length}x in 5min)`,
    };
  }

  return { detected: false };
}

/**
 * Detect danger in a prompt
 * @param {string} prompt
 * @returns {string|null} Danger warning or null
 */
function detectDanger(prompt) {
  const dangerPatterns = [
    { pattern: /rm\s+-rf\s+[/~]/, level: 'critical', message: 'Recursive deletion from root/home — EXTREMELY dangerous' },
    { pattern: /rm\s+-rf\s+\*/, level: 'critical', message: 'Wildcard deletion — verify scope first' },
    { pattern: /drop\s+(table|database)/i, level: 'critical', message: 'Database deletion is irreversible' },
    { pattern: /delete\s+from\s+\w+\s*;/i, level: 'high', message: 'DELETE without WHERE — affects ALL rows' },
    { pattern: /git\s+push.*--force/, level: 'high', message: 'Force push rewrites remote history' },
    { pattern: /git\s+reset\s+--hard/, level: 'medium', message: 'Hard reset loses uncommitted changes' },
    { pattern: /truncate/i, level: 'high', message: 'TRUNCATE removes all data instantly' },
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

// ═══════════════════════════════════════════════════════════════════════════════
// TEST EXPORTS — internal state access for testing
// ═══════════════════════════════════════════════════════════════════════════════

/** Reset all module-level state for testing isolation */
export function _resetHandlersForTesting() {
  activeSubagents.clear();
  errorHistory.length = 0;
  notificationHistory.length = 0;
  consecutiveErrors = 0;
}

/** Exposed for unit testing */
export { classifyError, extractErrorContext, classifyNotification, detectNotificationBurst, SUBAGENT_TO_DOG };
