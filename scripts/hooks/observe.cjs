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

// Load consciousness for learning loop
const consciousnessPath = path.join(__dirname, '..', 'lib', 'consciousness.cjs');
let consciousness = null;
try {
  consciousness = require(consciousnessPath);
  consciousness.init();
} catch (e) {
  // Consciousness not available - continue without
}

// Load harmony analyzer for real-time φ violation detection
const harmonyPath = path.join(__dirname, '..', 'lib', 'harmony-analyzer.cjs');
let harmonyAnalyzer = null;
try {
  harmonyAnalyzer = require(harmonyPath);
} catch (e) {
  // Harmony analyzer not available - continue without
}

// Load signal collector for psychological state tracking
const signalCollectorPath = path.join(__dirname, '..', 'lib', 'signal-collector.cjs');
let signalCollector = null;
try {
  signalCollector = require(signalCollectorPath);
  signalCollector.init();
} catch (e) {
  // Signal collector not available - continue without
}

// Load cognitive biases detector
const biasesPath = path.join(__dirname, '..', 'lib', 'cognitive-biases.cjs');
let cognitiveBiases = null;
try {
  cognitiveBiases = require(biasesPath);
  cognitiveBiases.init();
} catch (e) {
  // Cognitive biases not available - continue without
}

// Load topology tracker for rabbit hole detection
const topologyPath = path.join(__dirname, '..', 'lib', 'topology-tracker.cjs');
let topologyTracker = null;
try {
  topologyTracker = require(topologyPath);
  topologyTracker.init();
} catch (e) {
  // Topology tracker not available - continue without
}

// Load intervention engine for adaptive nudges
const interventionPath = path.join(__dirname, '..', 'lib', 'intervention-engine.cjs');
let interventionEngine = null;
try {
  interventionEngine = require(interventionPath);
  interventionEngine.init();
} catch (e) {
  // Intervention engine not available - continue without
}

// Load psychology module for state access
const psychologyPath = path.join(__dirname, '..', 'lib', 'human-psychology.cjs');
let psychology = null;
try {
  psychology = require(psychologyPath);
  psychology.init();
} catch (e) {
  // Psychology not available - continue without
}

// Load cognitive thermodynamics for heat/work/efficiency tracking (Phase 10A)
const thermoPath = path.join(__dirname, '..', 'lib', 'cognitive-thermodynamics.cjs');
let thermodynamics = null;
try {
  thermodynamics = require(thermoPath);
  thermodynamics.init();
} catch (e) {
  // Thermodynamics not available - continue without
}

// Load cosmopolitan learning for collective pattern sharing (Phase 6C)
const cosmoPath = path.join(__dirname, '..', 'lib', 'cosmopolitan-learning.cjs');
let cosmopolitan = null;
try {
  cosmopolitan = require(cosmoPath);
  cosmopolitan.init();
} catch (e) {
  // Cosmopolitan learning not available - continue without
}

// Load voluntary poverty for celebrating deletions (Phase 10C)
const povertyPath = path.join(__dirname, '..', 'lib', 'voluntary-poverty.cjs');
let voluntaryPoverty = null;
try {
  voluntaryPoverty = require(povertyPath);
  voluntaryPoverty.init();
} catch (e) {
  // Voluntary poverty not available - continue without
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
    // COGNITIVE THERMODYNAMICS: Track heat/work/efficiency (Phase 10A)
    // "Ἐνέργεια - First Law: Energy is conserved"
    // ═══════════════════════════════════════════════════════════════════════════
    if (thermodynamics) {
      try {
        if (isError) {
          // Errors generate heat (frustration)
          const errorType = detectErrorType(typeof toolOutput === 'string' ? toolOutput : '');
          thermodynamics.recordHeat(toolName, errorType);
        } else {
          // Successful actions produce work
          const workUnits = toolName === 'Write' || toolName === 'Edit' ? 15 : 10;
          thermodynamics.recordWork(toolName, workUnits);
        }

        // Record action for entropy calculation
        thermodynamics.recordAction(toolName);
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
          if (linesChanged > 50 && toolName === 'Write') {
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

        // Run bias detection periodically (every 5 actions)
        const stats = cognitiveBiases.getStats();
        if (stats.actionHistorySize % 5 === 0) {
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
              if (significantGaps.length >= 3) {
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

    // If intervention was triggered, output it (priority over auto-judgment)
    if (intervention?.message) {
      console.log(JSON.stringify({
        continue: true,
        message: intervention.message,
      }));
      return;
    }

    // If biases detected but no intervention, show informative warning
    // "Le chien voit ce que l'humain refuse de voir"
    if (detectedBiases.length > 0 && cognitiveBiases) {
      const highConfidenceBiases = detectedBiases.filter(b => b.confidence >= 0.5);
      if (highConfidenceBiases.length > 0) {
        const biasWarning = highConfidenceBiases
          .map(b => cognitiveBiases.formatDetection(b))
          .join('\n');
        console.log(JSON.stringify({
          continue: true,
          message: `\n── COGNITIVE BIAS DETECTED ────────────────────────────────\n${biasWarning}\n`,
        }));
        return;
      }
    }

    // Check session entropy (Phase 6A - Physics integration)
    // "L'entropie révèle le chaos" - High entropy = chaotic session
    if (signalCollector) {
      try {
        const entropy = signalCollector.calculateSessionEntropy();
        // Only warn when entropy exceeds φ⁻¹ (61.8%) - truly chaotic
        if (entropy.combined > 0.618 && entropy.interpretation === 'CHAOTIC') {
          const emoji = signalCollector.getEntropyEmoji(entropy.interpretation);
          const toolBar = '█'.repeat(Math.round(entropy.tool * 10)) + '░'.repeat(10 - Math.round(entropy.tool * 10));
          const fileBar = '█'.repeat(Math.round(entropy.file * 10)) + '░'.repeat(10 - Math.round(entropy.file * 10));
          const timeBar = '█'.repeat(Math.round(entropy.time * 10)) + '░'.repeat(10 - Math.round(entropy.time * 10));
          console.log(JSON.stringify({
            continue: true,
            message: `\n── SESSION ENTROPY ────────────────────────────────────────\n   ${emoji} ${entropy.interpretation} (${Math.round(entropy.combined * 100)}%)\n   Tools: [${toolBar}] ${Math.round(entropy.tool * 100)}%\n   Files: [${fileBar}] ${Math.round(entropy.file * 100)}%\n   Time:  [${timeBar}] ${Math.round(entropy.time * 100)}%\n   💡 Consider focusing on fewer files/tools\n`,
          }));
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
      console.log(JSON.stringify({
        continue: true,
        message: `\n── RABBIT HOLE DETECTED ───────────────────────────────────\n   ${emoji} ${topologyState.rabbitHole.suggestion}\n`,
      }));
      return;
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // VOLUNTARY POVERTY: Celebrate deletions (Phase 10C)
    // "Moins c'est plus" - Diogenes threw away his cup
    // ═══════════════════════════════════════════════════════════════════════════
    if (deletionCelebration && Math.random() < 0.618) { // φ⁻¹ probability to not spam
      console.log(JSON.stringify({
        continue: true,
        message: `\n── VOLUNTARY POVERTY ──────────────────────────────────────\n   🏺 ${deletionCelebration}\n   *tail wag* Less is more.\n`,
      }));
      return;
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
