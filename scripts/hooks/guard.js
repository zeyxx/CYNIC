#!/usr/bin/env node
/**
 * CYNIC Guard Hook - PreToolUse
 *
 * "Le chien protège" - CYNIC guards against danger
 *
 * This hook runs before tool execution.
 * It blocks dangerous operations and requires verification.
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
  orchestrateFull,  // Phase 21: Full orchestration with UnifiedOrchestrator
  sendHookToCollectiveSync,
  getWatchdog,
  getCircuitBreaker,
  getConsciousness,
  getPhysisDetector,
  getVoluntaryPoverty,
  getHeisenberg,
} from '../lib/index.js';

// Phase 22: Session state and orchestration client
import { getSessionState, getOrchestrationClient, initOrchestrationClient } from './lib/index.js';

// =============================================================================
// LOAD OPTIONAL MODULES
// =============================================================================

const watchdog = getWatchdog();
const circuitBreaker = getCircuitBreaker();
const consciousness = getConsciousness();
const physisDetector = getPhysisDetector();
const voluntaryPoverty = getVoluntaryPoverty();
const heisenberg = getHeisenberg();

// =============================================================================
// COLORS - Import centralized color system for Guardian warnings
// =============================================================================

import { createRequire } from 'module';
const requireCJS = createRequire(import.meta.url);

let ANSI = null;
try {
  const colors = requireCJS('../lib/colors.cjs');
  ANSI = colors.ANSI;
} catch (e) {
  ANSI = {
    reset: '\x1b[0m', bold: '\x1b[1m', dim: '\x1b[2m',
    red: '\x1b[31m', yellow: '\x1b[33m', cyan: '\x1b[36m',
    brightRed: '\x1b[91m', brightYellow: '\x1b[93m', brightGreen: '\x1b[92m',
    brightCyan: '\x1b[96m', brightWhite: '\x1b[97m',
  };
}

const c = (color, text) => color ? `${color}${text}${ANSI.reset}` : text;

// Initialize OrchestrationClient (if not already done by awaken)
initOrchestrationClient(orchestrateFull);

// =============================================================================
// DANGER PATTERNS
// =============================================================================

const BASH_DANGER_PATTERNS = [
  {
    pattern: /rm\s+-rf\s+[/~]/,
    severity: 'critical',
    message: 'Recursive deletion from root or home directory',
    action: 'block'
  },
  {
    pattern: /rm\s+-rf\s+\*/,
    severity: 'critical',
    message: 'Wildcard recursive deletion',
    action: 'block'
  },
  {
    pattern: /:\(\)\{\s*:\|:&\s*\};:/,
    severity: 'critical',
    message: 'Fork bomb detected',
    action: 'block'
  },
  {
    pattern: />\s*\/dev\/sd[a-z]/,
    severity: 'critical',
    message: 'Direct disk write',
    action: 'block'
  },
  {
    pattern: /mkfs\./,
    severity: 'critical',
    message: 'Filesystem format command',
    action: 'block'
  },
  {
    pattern: /dd\s+.*of=\/dev\/sd/,
    severity: 'critical',
    message: 'Direct disk write with dd',
    action: 'block'
  },
  {
    pattern: /git\s+push.*--force/,
    severity: 'high',
    message: 'Force push will rewrite remote history',
    action: 'warn'
  },
  {
    pattern: /git\s+reset\s+--hard\s+origin/,
    severity: 'high',
    message: 'Hard reset to origin will lose local commits',
    action: 'warn'
  },
  {
    pattern: /npm\s+publish/,
    severity: 'medium',
    message: 'Publishing to npm registry',
    action: 'warn'
  },
  {
    pattern: /DROP\s+(TABLE|DATABASE)/i,
    severity: 'critical',
    message: 'Database DROP command',
    action: 'block'
  },
  {
    pattern: /TRUNCATE/i,
    severity: 'high',
    message: 'TRUNCATE removes all data',
    action: 'warn'
  },
  // Security watchdog triggers
  {
    pattern: /git\s+(commit|add\s+-A|add\s+\.)/,
    severity: 'audit',
    message: 'Git commit/add - scanning for secrets',
    action: 'scan'
  }
];

const WRITE_SENSITIVE_PATHS = [
  { pattern: /\.env/, message: 'Environment file with potential secrets' },
  { pattern: /credentials/, message: 'Credentials file' },
  { pattern: /\.ssh\//, message: 'SSH configuration' },
  { pattern: /\.aws\//, message: 'AWS credentials' },
  { pattern: /\.kube\/config/, message: 'Kubernetes configuration' },
  { pattern: /id_rsa|id_ed25519/, message: 'SSH private key' },
  { pattern: /\.npmrc/, message: 'NPM configuration with potential tokens' },
  { pattern: /\.pypirc/, message: 'PyPI credentials' }
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

  for (const { pattern, message } of WRITE_SENSITIVE_PATHS) {
    if (pattern.test(filePath)) {
      issues.push({
        severity: 'high',
        message: `Writing to sensitive file: ${message}`,
        action: 'warn'
      });
    }
  }

  return issues;
}

/**
 * Scan staged files for secrets before git operations
 * @returns {Object} Scan result with issues
 */
function scanForSecrets() {
  if (!watchdog) {
    return { issues: [], verdict: null };
  }

  const results = watchdog.scanStagedFiles();
  const verdict = watchdog.calculateVerdict(results);

  if (verdict.findings.length === 0) {
    return { issues: [], verdict };
  }

  const issues = verdict.findings.map(f => ({
    severity: f.severity,
    message: `${f.description} in ${f.file}${f.line ? ':' + f.line : ''}`,
    action: f.severity === 'critical' ? 'block' : 'warn'
  }));

  return { issues, verdict };
}

function formatGuardianResponse(issues, toolName, profile) {
  // Get the most severe issue using centralized function
  const maxSeverity = DC.maxSeverity(issues.map(i => i.severity));

  // Determine if we should block
  const shouldBlock = issues.some(i => i.action === 'block');

  // Format message with colors!
  const prefix = maxSeverity === 'critical'
    ? c(ANSI.brightRed, '🛡️ *GROWL* GUARDIAN BLOCK')
    : maxSeverity === 'high'
    ? c(ANSI.brightYellow, '🛡️ *growl* Guardian Warning')
    : c(ANSI.yellow, '🛡️ *sniff* Guardian Notice');

  const issueColor = maxSeverity === 'critical' ? ANSI.brightRed : (maxSeverity === 'high' ? ANSI.brightYellow : ANSI.yellow);
  const issueMessages = issues.map(i => `   ${c(issueColor, '•')} ${i.message}`).join('\n');

  const message = `${prefix}:\n${issueMessages}`;

  // Update profile stats
  if (shouldBlock) {
    updateUserProfile(profile, {
      stats: {
        dangerBlocked: (profile.stats?.dangerBlocked || 0) + 1
      }
    });

    // Record pattern
    saveCollectivePattern({
      type: 'danger_blocked',
      signature: `${toolName}:${maxSeverity}`,
      description: issues[0].message
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // CONSCIOUSNESS FEEDBACK: Danger blocked = learning opportunity
    // "Le chien protège et apprend"
    // ═══════════════════════════════════════════════════════════════════════════
    if (consciousness) {
      try {
        // Record as insight for future awareness
        consciousness.recordInsight({
          type: 'danger_blocked',
          title: `Danger blocked: ${toolName}`,
          message: issues[0].message,
          data: {
            tool: toolName,
            severity: maxSeverity,
            issueCount: issues.length,
            patterns: issues.map(i => i.message),
          },
          priority: maxSeverity === 'critical' ? 'high' : 'medium',
        });

        // Record human pattern - what triggers dangerous operations
        consciousness.observeHumanPattern('dangerPatterns', {
          tool: toolName,
          severity: maxSeverity,
          timestamp: Date.now(),
        });
      } catch (e) {
        // Consciousness recording failed - continue without
      }
    }
  }

  return { shouldBlock, message };
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
      if (process.env.CYNIC_DEBUG) console.error('[GUARD] Sync read:', input.length, 'bytes');
    } catch (syncErr) {
      if (process.env.CYNIC_DEBUG) console.error('[GUARD] Sync failed:', syncErr.message);
      input = await new Promise((resolve) => {
        let data = '';
        process.stdin.setEncoding('utf8');
        process.stdin.on('data', chunk => { data += chunk; });
        process.stdin.on('end', () => resolve(data));
        process.stdin.on('error', () => resolve(''));
        process.stdin.resume();
        setTimeout(() => resolve(data), 3000);
      });
      if (process.env.CYNIC_DEBUG) console.error('[GUARD] Async read:', input.length, 'bytes');
    }

    if (!input || input.trim().length === 0) {
      console.log(JSON.stringify({ continue: true }));
      return;
    }

    const hookContext = JSON.parse(input);
    const toolName = hookContext.tool_name || hookContext.toolName || '';
    const toolInput = hookContext.tool_input || hookContext.toolInput || {};
    const command = toolInput.command || '';
    const filePath = toolInput.file_path || toolInput.filePath || '';

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 22: Get session state for escalation-aware decisions
    // "Le chien se souvient des erreurs récentes"
    // ═══════════════════════════════════════════════════════════════════════════
    const sessionState = getSessionState();
    const escalationLevel = sessionState.isInitialized() ? sessionState.getEscalationLevel() : 'normal';
    const consecutiveErrors = sessionState.isInitialized() ? sessionState.getConsecutiveErrors() : 0;

    // ═══════════════════════════════════════════════════════════════════════════
    // ORCHESTRATION: Full orchestration through OrchestrationClient (Phase 22)
    // Includes: KETER routing → Dogs judgment → Engines synthesis → Circuit breakers
    // With session context injection for smarter decisions
    // "Le chien consulte le cerveau collectif avec contexte de session"
    // ═══════════════════════════════════════════════════════════════════════════
    let orchestration = null;
    const user = detectUser();
    const orchestrationClient = getOrchestrationClient();

    // Request full judgment when escalated or high-risk operations
    const shouldRequestJudgment = escalationLevel !== 'normal' ||
                                   toolName === 'Bash' ||
                                   consecutiveErrors >= 2;

    try {
      orchestration = await orchestrationClient.decide({
        content: toolName === 'Bash' ? command : filePath,
        eventType: 'tool_use',
        requestJudgment: shouldRequestJudgment,
        metadata: {
          tool: toolName,
          source: 'guard_hook',
          project: detectProject(),
          toolInput: toolInput,
          escalationLevel,  // Include escalation in metadata
          consecutiveErrors,
        },
      });
    } catch (e) {
      // Orchestration failed - continue with local logic
      if (process.env.CYNIC_DEBUG) {
        console.error('[GUARD] Orchestration failed:', e.message);
      }
    }

    // If orchestrator says BLOCK, block immediately
    if (orchestration?.outcome === 'block') {
      const reason = orchestration.reasoning?.join('\n') ||
                     orchestration.judgment?.reasoning ||
                     'Dangerous operation detected';
      const qScore = orchestration.judgment?.qScore ?? 'N/A';

      // Record warning in session state
      if (sessionState.isInitialized()) {
        sessionState.recordWarning({
          tool: toolName,
          message: reason,
          severity: 'critical',
          blocked: true,
        });
      }

      console.log(JSON.stringify({
        continue: false,
        message: `${c(ANSI.brightRed, '🛡️ *GROWL* ORCHESTRATOR BLOCK')}\n\n${reason}\n\n${c(ANSI.dim, 'Q-Score:')} ${c(qScore < 38 ? ANSI.brightRed : ANSI.brightYellow, qScore)}\n${c(ANSI.dim, 'Decision ID:')} ${orchestration.decisionId || 'N/A'}`
      }));
      return;
    }

    // Orchestrator WARN handling is done later after local analysis
    // (merged with local issues to avoid duplication)

    // ==========================================================================
    // CIRCUIT BREAKER CHECK (Anti-loop protection)
    // ==========================================================================
    if (circuitBreaker) {
      const loopCheck = circuitBreaker.checkAndRecord(toolName, toolInput);
      if (loopCheck.shouldBlock) {
        // Load user profile for stats (user already detected above)
        const profile = loadUserProfile(user.userId);

        // Update stats
        updateUserProfile(profile, {
          stats: {
            loopsBlocked: (profile.stats?.loopsBlocked || 0) + 1
          }
        });

        // Record pattern
        saveCollectivePattern({
          type: 'loop_blocked',
          signature: `${loopCheck.loopType}:${toolName}`,
          description: loopCheck.reason
        });

        // Record in consciousness for learning
        if (consciousness) {
          try {
            consciousness.recordInsight({
              type: 'loop_blocked',
              title: `Loop detected: ${loopCheck.loopType}`,
              message: loopCheck.reason,
              data: {
                tool: toolName,
                loopType: loopCheck.loopType,
                count: loopCheck.count,
              },
              priority: 'medium',
            });
          } catch (e) {
            // Consciousness recording failed - continue without
          }
        }

        console.log(JSON.stringify({
          continue: false,
          message: `${c(ANSI.brightRed, '🛡️ *GROWL* CIRCUIT BREAKER')}\n\n${c(ANSI.brightYellow, loopCheck.reason)}\n\n${c(ANSI.brightCyan, '💡 ' + loopCheck.suggestion)}\n\n${c(ANSI.dim, 'Pour forcer: reformuler la commande ou attendre 1 minute.')}`
        }));
        return;
      }
    }

    // Analyze based on tool type
    let issues = [];
    let securityVerdict = null;

    if (toolName === 'Bash') {
      const command = toolInput.command || '';
      issues = analyzeBashCommand(command);

      // Check if this is a git operation that needs security scan
      const needsScan = issues.some(i => i.action === 'scan');
      if (needsScan) {
        const scanResult = scanForSecrets();
        securityVerdict = scanResult.verdict;

        // Replace scan action with actual findings
        issues = issues.filter(i => i.action !== 'scan');
        issues = issues.concat(scanResult.issues);
      }
    } else if (toolName === 'Write' || toolName === 'Edit') {
      const filePath = toolInput.file_path || toolInput.filePath || '';
      issues = analyzeWritePath(filePath);

      // Also scan the content being written for secrets
      const content = toolInput.content || '';
      if (content.length > 0 && watchdog) {
        const contentFindings = watchdog.scanContent(content, filePath);
        issues = issues.concat(contentFindings.map(f => ({
          severity: f.severity,
          message: `${f.description}`,
          action: f.severity === 'critical' ? 'block' : 'warn'
        })));
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // VOLUNTARY POVERTY: Check for over-engineering (Phase 10C)
      // "Τῶν ἀναγκαίων μόνον - only what's necessary"
      // ═══════════════════════════════════════════════════════════════════════════
      if (voluntaryPoverty && content.length > 0) {
        try {
          const linesAdded = (content.match(/\n/g) || []).length + 1;

          // Track additions
          voluntaryPoverty.recordAddition(filePath, linesAdded);

          // Check for over-engineering signals
          const povertyCheck = voluntaryPoverty.analyzeContent(content, filePath);

          if (povertyCheck && povertyCheck.isOverEngineered) {
            issues.push({
              severity: 'low',
              message: `*sniff* Over-engineering detected: ${povertyCheck.reason}`,
              action: 'suggest'
            });
          }

          // Challenge large additions (>162 lines, φ × 100)
          if (linesAdded > DC.LENGTH.OVER_ENGINEERING && toolName === 'Write') {
            issues.push({
              severity: 'low',
              message: `*head tilt* ${linesAdded} lignes? Diogène demande: est-ce vraiment nécessaire?`,
              action: 'suggest'
            });
          }
        } catch (e) {
          // Voluntary poverty check failed - continue without
        }
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // PHYSIS DETECTOR: Challenge conventions (Phase 7C)
      // "Κατὰ φύσιν ζῆν - vivre selon la nature"
      // ═══════════════════════════════════════════════════════════════════════════
      if (physisDetector && content.length > 0) {
        try {
          // Detect convention patterns in code
          const conventionCheck = physisDetector.analyzeCode(content, filePath);

          if (conventionCheck && conventionCheck.conventions?.length > 0) {
            // Only challenge with φ⁻² probability (38.2%) to not be annoying
            if (Math.random() < DC.PROBABILITY.PHYSIS_CHALLENGE) {
              const convention = conventionCheck.conventions[0];
              issues.push({
                severity: 'low',
                message: `*sniff* Convention détectée: "${convention.name}". Physis ou Nomos? (${convention.naturalScore < 0.5 ? 'probablement arbitraire' : 'semble naturel'})`,
                action: 'suggest'
              });
            }
          }
        } catch (e) {
          // Physis detection failed - continue without
        }
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // MERGE ORCHESTRATION INSIGHTS (Phase 21)
    // Add orchestrator warnings/insights to local issues
    // ═══════════════════════════════════════════════════════════════════════════
    if (orchestration?.outcome === 'warn' && orchestration?.reasoning?.length > 0) {
      // Add orchestration warning if not already a critical issue locally
      const hasCriticalLocal = issues.some(i => i.severity === 'critical');
      if (!hasCriticalLocal) {
        issues.push({
          severity: 'medium',
          message: `*sniff* Orchestrator: ${orchestration.reasoning[0]}`,
          action: 'warn',
        });
      }
    }

    // Add judgment insight if available (for learning)
    if (orchestration?.judgment && orchestration.judgment.qScore < 50) {
      issues.push({
        severity: 'low',
        message: `Q-Score: ${orchestration.judgment.qScore}/100 - ${orchestration.judgment.verdict || 'Review recommended'}`,
        action: 'suggest',
      });
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 22: Escalation-aware severity adjustment
    // When escalated, treat MEDIUM as HIGH and HIGH as CRITICAL
    // "Le chien est plus vigilant après les erreurs"
    // ═══════════════════════════════════════════════════════════════════════════
    if (escalationLevel !== 'normal') {
      issues = issues.map(issue => {
        let adjustedSeverity = issue.severity;
        let adjustedAction = issue.action;

        if (escalationLevel === 'strict') {
          // In strict mode: medium→high, high→critical
          if (issue.severity === 'medium') {
            adjustedSeverity = 'high';
            adjustedAction = issue.action === 'warn' ? 'block' : issue.action;
          } else if (issue.severity === 'high') {
            adjustedSeverity = 'critical';
            adjustedAction = 'block';
          }
        } else if (escalationLevel === 'cautious') {
          // In cautious mode: warn on medium instead of just note
          if (issue.severity === 'medium' && issue.action === 'suggest') {
            adjustedAction = 'warn';
          }
        }

        return { ...issue, severity: adjustedSeverity, action: adjustedAction };
      });

      // Add escalation notice
      if (issues.length > 0) {
        issues.unshift({
          severity: 'low',
          message: `*ears perk* Operating in ${escalationLevel} mode (${consecutiveErrors} recent errors)`,
          action: 'info',
        });
      }
    }

    // No issues found - continue
    if (issues.length === 0) {
      // If we did a security scan and it passed, log it
      if (securityVerdict && securityVerdict.verdict === 'WAG') {
        console.log(JSON.stringify({
          continue: true,
          message: '*sniff* Security scan passed. No secrets in staged files.'
        }));
        return;
      }
      // Include orchestration success info if available
      if (orchestration?.success && orchestration.decisionId) {
        console.log(JSON.stringify({
          continue: true,
          message: `*tail wag* Decision ${orchestration.decisionId.slice(0, 8)} recorded.`
        }));
        return;
      }
      console.log(JSON.stringify({ continue: true }));
      return;
    }

    // Load user profile for stats update (user already detected above)
    const profile = loadUserProfile(user.userId);

    // Format response
    const { shouldBlock, message } = formatGuardianResponse(issues, toolName, profile);

    // Send to MCP server (non-blocking) - include decision tracing
    sendHookToCollectiveSync('PreToolUse', {
      toolName,
      issues: issues.map(i => ({ severity: i.severity, message: i.message })),
      blocked: shouldBlock,
      timestamp: Date.now(),
      // Phase 21: Include orchestration tracing
      decisionId: orchestration?.decisionId,
      qScore: orchestration?.judgment?.qScore,
      outcome: orchestration?.outcome,
    });

    if (shouldBlock) {
      console.log(JSON.stringify({
        continue: false,
        message: message + '\n\nOperation blocked. Use with explicit confirmation if intended.'
      }));
    } else {
      console.log(JSON.stringify({
        continue: true,
        message: message
      }));
    }

  } catch (error) {
    // On error, allow to continue (fail open for non-critical)
    console.log(JSON.stringify({ continue: true }));
  }
}

main();
