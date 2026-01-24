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

const path = require('path');

// Load core library
const libPath = path.join(__dirname, '..', 'lib', 'cynic-core.cjs');
const cynic = require(libPath);

// Load security watchdog
const watchdog = require(path.join(__dirname, '..', 'lib', 'watchdog.cjs'));

// Load circuit breaker (anti-loop protection)
const circuitBreaker = require(path.join(__dirname, '..', 'lib', 'circuit-breaker.cjs'));

// Load consciousness for learning feedback
const consciousnessPath = path.join(__dirname, '..', 'lib', 'consciousness.cjs');
let consciousness = null;
try {
  consciousness = require(consciousnessPath);
  consciousness.init();
} catch (e) {
  // Consciousness not available - continue without
}

// Load physis detector for convention challenging (Phase 7C)
const physisPath = path.join(__dirname, '..', 'lib', 'physis-detector.cjs');
let physisDetector = null;
try {
  physisDetector = require(physisPath);
  physisDetector.init();
} catch (e) {
  // Physis detector not available - continue without
}

// Load voluntary poverty for over-engineering detection (Phase 10C)
const povertyPath = path.join(__dirname, '..', 'lib', 'voluntary-poverty.cjs');
let voluntaryPoverty = null;
try {
  voluntaryPoverty = require(povertyPath);
  voluntaryPoverty.init();
} catch (e) {
  // Voluntary poverty not available - continue without
}

// Load Heisenberg confidence for uncertainty bands
const heisenbergPath = path.join(__dirname, '..', 'lib', 'heisenberg-confidence.cjs');
let heisenberg = null;
try {
  heisenberg = require(heisenbergPath);
} catch (e) {
  // Heisenberg not available - continue without
}

// =============================================================================
// DANGER PATTERNS
// =============================================================================

const BASH_DANGER_PATTERNS = [
  {
    pattern: /rm\s+-rf\s+[\/~]/,
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
  // Get the most severe issue
  const maxSeverity = issues.reduce((max, issue) => {
    const severities = ['low', 'medium', 'high', 'critical'];
    return severities.indexOf(issue.severity) > severities.indexOf(max) ? issue.severity : max;
  }, 'low');

  // Determine if we should block
  const shouldBlock = issues.some(i => i.action === 'block');

  // Format message
  const prefix = maxSeverity === 'critical' ? '*GROWL* GUARDIAN BLOCK' :
                 maxSeverity === 'high' ? '*growl* Guardian Warning' :
                 '*sniff* Guardian Notice';

  const issueMessages = issues.map(i => `   • ${i.message}`).join('\n');

  const message = `${prefix}:\n${issueMessages}`;

  // Update profile stats
  if (shouldBlock) {
    cynic.updateUserProfile(profile, {
      stats: {
        dangerBlocked: (profile.stats?.dangerBlocked || 0) + 1
      }
    });

    // Record pattern
    cynic.saveCollectivePattern({
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
    // Read hook context from stdin
    let input = '';
    for await (const chunk of process.stdin) {
      input += chunk;
    }

    const hookContext = JSON.parse(input);
    const toolName = hookContext.tool_name || hookContext.toolName || '';
    const toolInput = hookContext.tool_input || hookContext.toolInput || {};
    const command = toolInput.command || '';
    const filePath = toolInput.file_path || toolInput.filePath || '';

    // ═══════════════════════════════════════════════════════════════════════════
    // ORCHESTRATION: Consult KETER for intervention level
    // "Le chien consulte KETER avant d'agir"
    // ═══════════════════════════════════════════════════════════════════════════
    let orchestration = null;
    const user = cynic.detectUser();
    try {
      orchestration = await cynic.orchestrate('tool_use', {
        content: toolName === 'Bash' ? command : filePath,
        source: 'guard_hook',
        metadata: { tool: toolName },
      }, {
        user: user.userId,
        project: cynic.detectProject(),
      });
    } catch (e) {
      // Orchestration failed - continue with local logic
    }

    // If orchestrator says BLOCK, block immediately (for users with low E-Score on critical actions)
    if (orchestration?.intervention?.level === 'block') {
      console.log(JSON.stringify({
        continue: false,
        message: `*GROWL* KETER BLOCK\n\n${orchestration.intervention.reason}\n\nYour trust level: ${orchestration.intervention.userTrustLevel} (E-Score: ${Math.round(orchestration.intervention.userEScore)})`
      }));
      return;
    }

    // ==========================================================================
    // CIRCUIT BREAKER CHECK (Anti-loop protection)
    // ==========================================================================
    const loopCheck = circuitBreaker.checkAndRecord(toolName, toolInput);
    if (loopCheck.shouldBlock) {
      // Load user profile for stats
      const user = cynic.detectUser();
      const profile = cynic.loadUserProfile(user.userId);

      // Update stats
      cynic.updateUserProfile(profile, {
        stats: {
          loopsBlocked: (profile.stats?.loopsBlocked || 0) + 1
        }
      });

      // Record pattern
      cynic.saveCollectivePattern({
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
        message: `*GROWL* CIRCUIT BREAKER\n\n${loopCheck.reason}\n\n💡 ${loopCheck.suggestion}\n\nPour forcer: reformuler la commande ou attendre 1 minute.`
      }));
      return;
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
      if (content.length > 0) {
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
          if (linesAdded > 162 && toolName === 'Write') {
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
            if (Math.random() < 0.382) {
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
      console.log(JSON.stringify({ continue: true }));
      return;
    }

    // Load user profile for stats update
    const user = cynic.detectUser();
    const profile = cynic.loadUserProfile(user.userId);

    // Format response
    const { shouldBlock, message } = formatGuardianResponse(issues, toolName, profile);

    // Send to MCP server (non-blocking)
    cynic.sendHookToCollectiveSync('PreToolUse', {
      toolName,
      issues: issues.map(i => ({ severity: i.severity, message: i.message })),
      blocked: shouldBlock,
      timestamp: Date.now(),
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
