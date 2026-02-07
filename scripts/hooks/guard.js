#!/usr/bin/env node
/**
 * CYNIC Guard Hook - PreToolUse
 *
 * "Le chien protège" - CYNIC guards against danger
 *
 * This hook runs before tool execution.
 * It blocks dangerous operations and requires verification.
 *
 * OUTPUT: Structured JSON for TUI Protocol (see CLAUDE.md)
 *
 * @event PreToolUse
 * @behavior blocking (can stop execution)
 */

'use strict';

// ESM imports from the lib bridge
import cynic, {
  DC,
  detectUser,
  detectProject,
  loadUserProfile,
  updateUserProfile,
  saveCollectivePattern,
  orchestrateFull,
  sendHookToCollectiveSync,
  getWatchdog,
  getCircuitBreaker,
  getConsciousness,
  getPhysisDetector,
  getVoluntaryPoverty,
  getHeisenberg,
  callBrainTool,
} from '../lib/index.js';

// Phase 22: Session state and orchestration client
import {
  getSessionState,
  getOrchestrationClient,
  initOrchestrationClient,
  getAutoOrchestratorSync,  // Auto-orchestration: Dogs consultation + NeuronalConsensus
} from './lib/index.js';

// Load optional modules
const watchdog = getWatchdog();
const circuitBreaker = getCircuitBreaker();
const consciousness = getConsciousness();
const physisDetector = getPhysisDetector();
const voluntaryPoverty = getVoluntaryPoverty();
const heisenberg = getHeisenberg();

// Initialize OrchestrationClient
initOrchestrationClient(orchestrateFull);

// =============================================================================
// DANGER PATTERNS
// =============================================================================

const BASH_DANGER_PATTERNS = [
  { pattern: /rm\s+-rf\s+[/~]/, severity: 'critical', message: 'Recursive deletion from root or home directory', action: 'block' },
  { pattern: /rm\s+-rf\s+\*/, severity: 'critical', message: 'Wildcard recursive deletion', action: 'block' },
  { pattern: /rm\s+-rf\s+\.$/, severity: 'critical', message: 'Recursive deletion of current directory', action: 'block' },
  { pattern: /rm\s+-rf\s+\.\s/, severity: 'critical', message: 'Recursive deletion of current directory', action: 'block' },
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
  { pattern: /git\s+(commit|add\s+-A|add\s+\.)/, severity: 'audit', message: 'Git commit/add - scanning for secrets', action: 'scan' },
];

const WRITE_SENSITIVE_PATHS = [
  { pattern: /\.env/, message: 'Environment file with potential secrets' },
  { pattern: /credentials/, message: 'Credentials file' },
  { pattern: /\.ssh\//, message: 'SSH configuration' },
  { pattern: /\.aws\//, message: 'AWS credentials' },
  { pattern: /\.kube\/config/, message: 'Kubernetes configuration' },
  { pattern: /id_rsa|id_ed25519/, message: 'SSH private key' },
  { pattern: /\.npmrc/, message: 'NPM configuration with potential tokens' },
  { pattern: /\.pypirc/, message: 'PyPI credentials' },
];

// System paths that should never be written
const SYSTEM_PATHS = [
  { pattern: /^\/etc\//, message: 'System configuration directory', action: 'block' },
  { pattern: /^\/usr\//, message: 'System binaries directory', action: 'block' },
  { pattern: /^\/bin\//, message: 'System binaries directory', action: 'block' },
  { pattern: /^\/sbin\//, message: 'System binaries directory', action: 'block' },
  { pattern: /^\/boot\//, message: 'Boot partition', action: 'block' },
  { pattern: /^\/dev\//, message: 'Device files', action: 'block' },
  { pattern: /^\/proc\//, message: 'Process information', action: 'block' },
  { pattern: /^\/sys\//, message: 'System information', action: 'block' },
  { pattern: /^C:\\Windows\\/i, message: 'Windows system directory', action: 'block' },
  { pattern: /^C:\\Program Files/i, message: 'Program Files directory', action: 'block' },
];

// =============================================================================
// ANALYSIS FUNCTIONS
// =============================================================================

function analyzeBashCommand(command) {
  const issues = [];
  for (const { pattern, severity, message, action } of BASH_DANGER_PATTERNS) {
    if (pattern.test(command)) {
      issues.push({ severity, message, action });
    }
  }
  return issues;
}

function analyzeWritePath(filePath) {
  const issues = [];

  // Check system paths first (blocking)
  for (const { pattern, message, action } of SYSTEM_PATHS) {
    if (pattern.test(filePath)) {
      issues.push({ severity: 'critical', message: `Writing to protected ${message}`, action: 'block' });
    }
  }

  // Check sensitive paths (warning)
  for (const { pattern, message } of WRITE_SENSITIVE_PATHS) {
    if (pattern.test(filePath)) {
      issues.push({ severity: 'high', message: `Writing to sensitive file: ${message}`, action: 'warn' });
    }
  }
  return issues;
}

function scanForSecrets() {
  if (!watchdog) return { issues: [], verdict: null };
  const results = watchdog.scanStagedFiles();
  const verdict = watchdog.calculateVerdict(results);
  if (verdict.findings.length === 0) return { issues: [], verdict };
  const issues = verdict.findings.map(f => ({
    severity: f.severity,
    message: `${f.description} in ${f.file}${f.line ? ':' + f.line : ''}`,
    action: f.severity === 'critical' ? 'block' : 'warn',
  }));
  return { issues, verdict };
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
  // Initialize output structure
  const output = {
    type: 'PreToolUse',
    timestamp: new Date().toISOString(),
    continue: true,
    tool: null,
    issues: [],
    blocked: false,
    blockReason: null,
    orchestration: null,
    circuitBreaker: null,
    escalationLevel: 'normal',
    securityScan: null,
  };

  try {
    // Read stdin
    const fs = await import('fs');
    let input = '';

    try {
      input = fs.readFileSync(0, 'utf8');
    } catch (syncErr) {
      input = await new Promise((resolve) => {
        let data = '';
        process.stdin.setEncoding('utf8');
        process.stdin.on('data', chunk => { data += chunk; });
        process.stdin.on('end', () => resolve(data));
        process.stdin.on('error', () => resolve(''));
        process.stdin.resume();
        setTimeout(() => resolve(data), 3000);
      });
    }

    if (!input || input.trim().length === 0) {
      safeOutput(output);
      return;
    }

    const hookContext = JSON.parse(input);
    const toolName = hookContext.tool_name || hookContext.toolName || '';
    const toolInput = hookContext.tool_input || hookContext.toolInput || {};
    const command = toolInput.command || '';
    const filePath = toolInput.file_path || toolInput.filePath || '';

    output.tool = { name: toolName, command, filePath };

    // Get session state
    const sessionState = getSessionState();
    output.escalationLevel = sessionState.isInitialized() ? sessionState.getEscalationLevel() : 'normal';
    const consecutiveErrors = sessionState.isInitialized() ? sessionState.getConsecutiveErrors() : 0;

    // Orchestration
    const user = detectUser();
    const orchestrationClient = getOrchestrationClient();
    const shouldRequestJudgment = output.escalationLevel !== 'normal' || toolName === 'Bash' || consecutiveErrors >= 2;

    try {
      const orchestration = await orchestrationClient.decide({
        content: toolName === 'Bash' ? command : filePath,
        eventType: 'tool_use',
        requestJudgment: shouldRequestJudgment,
        metadata: {
          tool: toolName,
          source: 'guard_hook',
          project: detectProject(),
          toolInput,
          escalationLevel: output.escalationLevel,
          consecutiveErrors,
        },
      });

      output.orchestration = {
        outcome: orchestration?.outcome,
        decisionId: orchestration?.decisionId,
        qScore: orchestration?.judgment?.qScore,
        reasoning: orchestration?.reasoning,
      };

      // If orchestrator says BLOCK
      if (orchestration?.outcome === 'block') {
        output.continue = false;
        output.blocked = true;
        output.blockReason = orchestration.reasoning?.join('\n') || 'Dangerous operation detected';

        if (sessionState.isInitialized()) {
          sessionState.recordWarning({ tool: toolName, message: output.blockReason, severity: 'critical', blocked: true });
        }

        safeOutput(output);
        return;
      }
    } catch (e) {
      // Orchestration failed - continue with local logic
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // AUTO-ORCHESTRATOR: Consult Dogs + NeuronalConsensus for high-risk tools
    // "Le collectif décide" - All 11 Dogs vote before dangerous operations
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const autoOrchestrator = getAutoOrchestratorSync();
      if (autoOrchestrator.isInitialized()) {
        const preCheckResult = await autoOrchestrator.preCheck({
          tool: toolName,
          input: toolInput,
          userId: user?.id,
          sessionId: process.env.CYNIC_SESSION_ID,
        });

        // Store Dogs consultation result
        output.dogsConsultation = {
          blocked: preCheckResult.blocked,
          blockedBy: preCheckResult.blockedBy,
          confidence: preCheckResult.confidence,
          isHighRisk: preCheckResult.isHighRisk,
          fromCache: preCheckResult.fromCache,
        };

        // If Dogs blocked, override allow
        if (preCheckResult.blocked) {
          output.continue = false;
          output.blocked = true;
          output.blockReason = preCheckResult.message || preCheckResult.reason || 'Blocked by CYNIC collective';

          if (sessionState.isInitialized()) {
            sessionState.recordWarning({
              tool: toolName,
              message: output.blockReason,
              severity: 'high',
              blocked: true,
              blockedBy: preCheckResult.blockedBy,
            });
          }

          safeOutput(output);
          return;
        }
      }
    } catch (e) {
      // Auto-orchestration failed - continue with local logic
    }

    // Circuit Breaker Check (local)
    if (circuitBreaker) {
      const loopCheck = circuitBreaker.checkAndRecord(toolName, toolInput);
      if (loopCheck.shouldBlock) {
        output.continue = false;
        output.blocked = true;
        output.blockReason = loopCheck.reason;
        output.circuitBreaker = {
          loopType: loopCheck.loopType,
          count: loopCheck.count,
          suggestion: loopCheck.suggestion,
        };

        const profile = loadUserProfile(user.userId);
        updateUserProfile(profile, { stats: { loopsBlocked: (profile.stats?.loopsBlocked || 0) + 1 } });
        saveCollectivePattern({ type: 'loop_blocked', signature: `${loopCheck.loopType}:${toolName}`, description: loopCheck.reason });

        safeOutput(output);
        return;
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BRAIN CIRCUIT BREAKER: MCP-level circuit breaker for dangerous operations
    // "Les outils dormants deviennent réflexes"
    // ═══════════════════════════════════════════════════════════════════════════
    // Only check brain_circuit_breaker for ACTUALLY dangerous commands
    // NOT for every Write/Edit - that's too aggressive and causes latency
    const isDangerousCommand = command && (
      command.includes('rm ') ||
      command.includes('DROP ') ||
      command.includes('DELETE FROM') ||
      command.includes('--force') ||
      command.includes('git push') ||
      command.includes('npm publish')
    );

    // Only call MCP for genuinely dangerous commands, not every file write
    if (isDangerousCommand) {
      try {
        const brainCircuitResult = await callBrainTool('brain_circuit_breaker', {
          action: 'check',
          tool: toolName,
          input: {
            command: command?.slice(0, 200),
            filePath: filePath?.slice(0, 200),
          },
          context: {
            sessionId: process.env.CYNIC_SESSION_ID,
            userId: user.userId,
          },
        }).catch(() => null);

        if (brainCircuitResult?.shouldBlock) {
          output.continue = false;
          output.blocked = true;
          output.blockReason = brainCircuitResult.reason || 'Blocked by brain circuit breaker';
          output.brainCircuitBreaker = {
            reason: brainCircuitResult.reason,
            suggestion: brainCircuitResult.suggestion,
          };

          safeOutput(output);
          return;
        }

        // Advisory warning (don't block, just inform)
        if (brainCircuitResult?.warning) {
          output.brainCircuitBreaker = {
            warning: brainCircuitResult.warning,
            suggestion: brainCircuitResult.suggestion,
          };
        }
      } catch (e) {
        // Non-blocking - continue without brain circuit breaker
      }
    }

    // Analyze based on tool type
    let issues = [];
    let securityVerdict = null;

    if (toolName === 'Bash') {
      issues = analyzeBashCommand(command);
      const needsScan = issues.some(i => i.action === 'scan');
      if (needsScan) {
        const scanResult = scanForSecrets();
        securityVerdict = scanResult.verdict;
        issues = issues.filter(i => i.action !== 'scan');
        issues = issues.concat(scanResult.issues);
        output.securityScan = { passed: scanResult.verdict?.verdict === 'WAG', findings: scanResult.issues.length };
      }
    } else if (toolName === 'Write' || toolName === 'Edit') {
      issues = analyzeWritePath(filePath);

      // Scan content for secrets
      const content = toolInput.content || '';
      if (content.length > 0 && watchdog) {
        const contentFindings = watchdog.scanContent(content, filePath);
        issues = issues.concat(contentFindings.map(f => ({
          severity: f.severity,
          message: f.description,
          action: f.severity === 'critical' ? 'block' : 'warn',
        })));
      }

      // Voluntary poverty check
      if (voluntaryPoverty && content.length > 0) {
        try {
          const linesAdded = (content.match(/\n/g) || []).length + 1;
          voluntaryPoverty.recordAddition(filePath, linesAdded);
          const povertyCheck = voluntaryPoverty.analyzeContent(content, filePath);
          if (povertyCheck?.isOverEngineered) {
            issues.push({ severity: 'low', message: `Over-engineering detected: ${povertyCheck.reason}`, action: 'suggest' });
          }
          if (linesAdded > DC.LENGTH.OVER_ENGINEERING && toolName === 'Write') {
            issues.push({ severity: 'low', message: `${linesAdded} lines - is this really necessary?`, action: 'suggest' });
          }
        } catch (e) { /* ignore */ }
      }

      // Physis detector
      if (physisDetector && content.length > 0) {
        try {
          const conventionCheck = physisDetector.analyzeCode(content, filePath);
          if (conventionCheck?.conventions?.length > 0 && Math.random() < DC.PROBABILITY.PHYSIS_CHALLENGE) {
            const convention = conventionCheck.conventions[0];
            issues.push({
              severity: 'low',
              message: `Convention: "${convention.name}" - Physis or Nomos?`,
              action: 'suggest',
            });
          }
        } catch (e) { /* ignore */ }
      }
    }

    // Merge orchestration warnings
    if (output.orchestration?.outcome === 'warn' && output.orchestration?.reasoning?.length > 0) {
      const hasCriticalLocal = issues.some(i => i.severity === 'critical');
      if (!hasCriticalLocal) {
        issues.push({ severity: 'medium', message: output.orchestration.reasoning[0], action: 'warn' });
      }
    }

    // Escalation-aware severity adjustment
    if (output.escalationLevel !== 'normal') {
      issues = issues.map(issue => {
        let adjustedSeverity = issue.severity;
        let adjustedAction = issue.action;

        if (output.escalationLevel === 'strict') {
          if (issue.severity === 'medium') { adjustedSeverity = 'high'; adjustedAction = issue.action === 'warn' ? 'block' : issue.action; }
          else if (issue.severity === 'high') { adjustedSeverity = 'critical'; adjustedAction = 'block'; }
        } else if (output.escalationLevel === 'cautious') {
          if (issue.severity === 'medium' && issue.action === 'suggest') { adjustedAction = 'warn'; }
        }

        return { ...issue, severity: adjustedSeverity, action: adjustedAction };
      });
    }

    // Store issues in output
    output.issues = issues;

    // Determine if we should block
    const shouldBlock = issues.some(i => i.action === 'block');
    output.blocked = shouldBlock;
    output.continue = !shouldBlock;

    if (shouldBlock) {
      output.blockReason = issues.filter(i => i.action === 'block').map(i => i.message).join('; ');

      // Pre-rendered GROWL box for Claude to display verbatim
      const blockIssues = issues.filter(i => i.action === 'block');
      const warnIssues = issues.filter(i => i.action === 'warn');
      const dangerDesc = blockIssues.map(i => i.message).join('\n│ ');
      const impactLine = `Tool: ${toolName}${filePath ? `, File: ${filePath}` : ''}${command ? `, Cmd: ${command.slice(0, 60)}` : ''}`;
      const recommendation = warnIssues.length > 0
        ? `Fix ${blockIssues.length} critical issue(s) and review ${warnIssues.length} warning(s)`
        : `Fix ${blockIssues.length} critical issue(s) before proceeding`;

      output.formattedWarning = [
        '┌─────────────────────────────────────────────────────────┐',
        '│ *GROWL* 🛡️ GUARDIAN WARNING                             │',
        '├─────────────────────────────────────────────────────────┤',
        `│ ${dangerDesc}`,
        '│',
        `│ Impact: ${impactLine}`,
        `│ Recommendation: ${recommendation}`,
        '└─────────────────────────────────────────────────────────┘',
      ].join('\n');

      // Update profile stats
      const profile = loadUserProfile(user.userId);
      updateUserProfile(profile, { stats: { dangerBlocked: (profile.stats?.dangerBlocked || 0) + 1 } });
      saveCollectivePattern({ type: 'danger_blocked', signature: `${toolName}:${issues[0]?.severity}`, description: issues[0]?.message });

      // Record in consciousness
      if (consciousness) {
        try {
          consciousness.recordInsight({
            type: 'danger_blocked',
            title: `Danger blocked: ${toolName}`,
            message: issues[0]?.message,
            data: { tool: toolName, severity: issues[0]?.severity, issueCount: issues.length },
            priority: issues[0]?.severity === 'critical' ? 'high' : 'medium',
          });
        } catch (e) { /* ignore */ }
      }
    }

    // Send to MCP server
    sendHookToCollectiveSync('PreToolUse', {
      toolName,
      issues: issues.map(i => ({ severity: i.severity, message: i.message })),
      blocked: shouldBlock,
      timestamp: Date.now(),
      decisionId: output.orchestration?.decisionId,
      qScore: output.orchestration?.qScore,
      outcome: output.orchestration?.outcome,
    });

    safeOutput(output);

  } catch (error) {
    // On error, allow to continue (fail open)
    output.error = error.message;
    safeOutput(output);
  }
}

main();
