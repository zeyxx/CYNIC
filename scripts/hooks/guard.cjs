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

    // Analyze based on tool type
    let issues = [];

    if (toolName === 'Bash') {
      const command = toolInput.command || '';
      issues = analyzeBashCommand(command);
    } else if (toolName === 'Write' || toolName === 'Edit') {
      const filePath = toolInput.file_path || toolInput.filePath || '';
      issues = analyzeWritePath(filePath);
    }

    // No issues found - continue
    if (issues.length === 0) {
      console.log(JSON.stringify({ continue: true }));
      return;
    }

    // Load user profile for stats update
    const user = cynic.detectUser();
    const profile = cynic.loadUserProfile(user.userId);

    // Format response
    const { shouldBlock, message } = formatGuardianResponse(issues, toolName, profile);

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
