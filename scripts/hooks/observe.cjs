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

const path = require('path');

// Load core library
const libPath = path.join(__dirname, '..', 'lib', 'cynic-core.cjs');
const cynic = require(libPath);

// Load task enforcer
const enforcerPath = path.join(__dirname, '..', 'lib', 'task-enforcer.cjs');
const enforcer = require(enforcerPath);

// Load auto-judge (autonomous judgments)
const autoJudge = require(path.join(__dirname, '..', 'lib', 'auto-judge.cjs'));

// Load contributor discovery (les rails dans le cerveau)
const contributorDiscoveryPath = path.join(__dirname, '..', 'lib', 'contributor-discovery.cjs');
let contributorDiscovery = null;
try {
  contributorDiscovery = require(contributorDiscoveryPath);
} catch (e) {
  // Contributor discovery not available - continue without
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
  cynic.callBrainTool('brain_triggers', {
    action: 'process',
    event,
  }).catch(() => {
    // Silently ignore errors - triggers should never block hooks
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
      cynic.sendTestFeedback(testResult).catch(() => {});
    }

    // Detect successful commits
    if (cmd.startsWith('git commit') && !isError) {
      const commitHash = extractCommitHash(output);
      cynic.sendCommitFeedback({
        success: true,
        commitHash,
        hooksPassed: true,
        message: extractCommitMessage(cmd),
      }).catch(() => {});
    }

    // Detect build commands
    if (cmd.match(/npm\s+run\s+build|tsc|webpack|vite\s+build|cargo\s+build|go\s+build/i)) {
      cynic.sendBuildFeedback({
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
  const match = output.match(/\[[\w\-\/]+\s+([a-f0-9]{7,})\]|^([a-f0-9]{40})$/im);
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

/**
 * Detect error type from error text
 */
function detectErrorType(errorText) {
  if (!errorText) return 'unknown';

  const lower = errorText.toLowerCase();

  if (lower.includes('enoent') || lower.includes('no such file')) return 'file_not_found';
  if (lower.includes('eacces') || lower.includes('permission denied')) return 'permission';
  if (lower.includes('econnrefused')) return 'connection';
  if (lower.includes('timeout')) return 'timeout';
  if (lower.includes('syntaxerror')) return 'syntax';
  if (lower.includes('typeerror')) return 'type';
  if (lower.includes('referenceerror')) return 'reference';
  if (lower.includes('eslint') || lower.includes('lint')) return 'lint';
  if (lower.includes('test') && lower.includes('fail')) return 'test_failure';
  if (lower.includes('build') && lower.includes('fail')) return 'build_failure';

  return 'generic';
}

// =============================================================================
// MAIN HANDLER
// =============================================================================

async function main() {
  try {
    // Read hook context from stdin
    let input = '';
    for await (const chunk of process.stdin) {
      input += chunk;
    }

    const hookContext = JSON.parse(input);
    const toolName = hookContext.tool_name || hookContext.toolName || '';
    const toolInput = hookContext.tool_input || hookContext.toolInput || {};
    const toolOutput = hookContext.tool_output || hookContext.toolOutput || {};
    const isError = hookContext.is_error || hookContext.isError || false;

    // Detect user and load profile
    const user = cynic.detectUser();
    const profile = cynic.loadUserProfile(user.userId);

    // Detect patterns
    const patterns = detectToolPattern(toolName, toolInput, toolOutput, isError);

    // Save patterns to local collective
    for (const pattern of patterns) {
      cynic.saveCollectivePattern(pattern);
    }

    // Track todos for Task Continuation Enforcer
    if (toolName === 'TodoWrite' && toolInput.todos) {
      const sessionId = process.env.CYNIC_SESSION_ID || hookContext.session_id || 'default';
      enforcer.updateTodosFromTool(sessionId, toolInput.todos);
    }

    // Update user profile stats
    const updates = updateUserToolStats(profile, toolName, isError);
    cynic.updateUserProfile(profile, updates);

    // Send to MCP server (non-blocking)
    cynic.sendHookToCollectiveSync('PostToolUse', {
      toolName,
      isError,
      patterns,
      inputSize: JSON.stringify(toolInput).length,
      timestamp: Date.now(),
    });

    // Process through Auto-Judgment Triggers (non-blocking)
    // This enables automatic judgments on errors, commits, decisions, etc.
    processTriggerEvent(toolName, toolInput, toolOutput, isError);

    // ==========================================================================
    // AUTONOMOUS JUDGMENT SYSTEM
    // ==========================================================================
    let autoJudgmentResult = null;

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
        if (linesChanged > 50) {
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

    // If auto-judgment was triggered, output it
    if (autoJudgmentResult?.judgment) {
      const judgment = autoJudgmentResult.judgment;
      const formatted = autoJudge.formatJudgment(judgment);

      // Send to MCP server for persistence
      cynic.callBrainTool('brain_save_judgment', {
        judgment: {
          ...judgment,
          source: 'auto-judge',
        },
      }).catch(() => {});

      // Output the judgment (will be shown to user)
      console.log(JSON.stringify({
        continue: true,
        message: formatted,
      }));
      return;
    }

    // Observer never blocks - always continue silently
    console.log(JSON.stringify({ continue: true }));

  } catch (error) {
    // Observer must never fail - silent continuation
    console.log(JSON.stringify({ continue: true }));
  }
}

main();
