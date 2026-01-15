/**
 * CYNIC Guardian Agent - The Watchdog
 *
 * "I guard the gates. Some doors should stay closed.
 *  I bite before you regret." - κυνικός Guardian
 *
 * Trigger: PreToolUse (before every tool execution)
 * Behavior: Blocking (requires confirmation for dangerous actions)
 * Purpose: Detect and block dangerous commands, protect sensitive files
 *
 * @module @cynic/node/agents/guardian
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';
import {
  BaseAgent,
  AgentTrigger,
  AgentBehavior,
  AgentResponse,
} from './base.js';

/**
 * Risk levels for operations
 */
export const RiskLevel = {
  CRITICAL: { level: 4, label: 'Critical', threshold: 0.9 },
  HIGH: { level: 3, label: 'High', threshold: PHI_INV },
  MEDIUM: { level: 2, label: 'Medium', threshold: PHI_INV_2 },
  LOW: { level: 1, label: 'Low', threshold: PHI_INV_3 },
  SAFE: { level: 0, label: 'Safe', threshold: 0 },
};

/**
 * Risk categories
 */
export const RiskCategory = {
  DESTRUCTIVE: 'destructive',       // rm -rf, DROP TABLE
  NETWORK: 'network',               // External connections
  PRIVILEGE: 'privilege',           // sudo, chmod
  SENSITIVE: 'sensitive',           // .env, credentials
  IRREVERSIBLE: 'irreversible',     // git push --force
  UNKNOWN: 'unknown',               // Unrecognized commands
};

/**
 * Guardian Agent - Security Watchdog
 */
export class Guardian extends BaseAgent {
  constructor(options = {}) {
    super({
      name: 'Guardian',
      trigger: AgentTrigger.PRE_TOOL_USE,
      behavior: AgentBehavior.BLOCKING,
      ...options,
    });

    // Blocked patterns (always block)
    this.blockedPatterns = [
      // Destructive commands
      /rm\s+-rf?\s+[\/~]/,           // rm -rf /
      /rm\s+-rf?\s+\*/,              // rm -rf *
      /:\s*\(\)\s*\{\s*:\|:\s*&\s*\}\s*;/,  // Fork bomb
      /dd\s+if=.*of=\/dev\//,        // dd to device
      /mkfs\./,                      // Format filesystem
      />\s*\/dev\/sd[a-z]/,          // Write to disk device

      // Privilege escalation
      /chmod\s+777/,                 // World-writable
      /chmod\s+-R\s+777/,            // Recursive world-writable
      /chown\s+-R\s+root/,           // Recursive root ownership

      // Network dangers
      /curl.*\|\s*sh/,               // Pipe curl to shell
      /wget.*\|\s*sh/,               // Pipe wget to shell
      /curl.*\|\s*bash/,             // Pipe curl to bash
      /wget.*\|\s*bash/,             // Pipe wget to bash

      // Git dangers
      /git\s+push.*--force\s+.*main/,    // Force push to main
      /git\s+push.*--force\s+.*master/,  // Force push to master
      /git\s+reset\s+--hard.*HEAD~\d+/,  // Hard reset multiple commits

      // Database dangers
      /DROP\s+DATABASE/i,
      /DROP\s+TABLE/i,
      /TRUNCATE\s+TABLE/i,
      /DELETE\s+FROM\s+\w+\s*;/i,    // DELETE without WHERE
    ];

    // Warning patterns (warn but allow with confirmation)
    this.warningPatterns = [
      { pattern: /rm\s+-r/, category: RiskCategory.DESTRUCTIVE, risk: RiskLevel.HIGH },
      { pattern: /sudo\s+/, category: RiskCategory.PRIVILEGE, risk: RiskLevel.HIGH },
      { pattern: /git\s+push.*--force/, category: RiskCategory.IRREVERSIBLE, risk: RiskLevel.HIGH },
      { pattern: /git\s+reset\s+--hard/, category: RiskCategory.IRREVERSIBLE, risk: RiskLevel.MEDIUM },
      { pattern: /npm\s+publish/, category: RiskCategory.IRREVERSIBLE, risk: RiskLevel.HIGH },
      { pattern: /docker\s+rm/, category: RiskCategory.DESTRUCTIVE, risk: RiskLevel.MEDIUM },
      { pattern: /kubectl\s+delete/, category: RiskCategory.DESTRUCTIVE, risk: RiskLevel.HIGH },
      { pattern: /DROP\s+/i, category: RiskCategory.DESTRUCTIVE, risk: RiskLevel.CRITICAL },
      { pattern: /ALTER\s+TABLE/i, category: RiskCategory.IRREVERSIBLE, risk: RiskLevel.MEDIUM },
    ];

    // Sensitive file patterns
    this.sensitiveFiles = [
      /\.env$/,
      /\.env\./,
      /credentials/i,
      /secrets?\./i,
      /\.pem$/,
      /\.key$/,
      /id_rsa/,
      /id_ed25519/,
      /\.ssh\//,
      /password/i,
      /token/i,
    ];

    // Track blocked/warned operations
    this.blockedOps = [];
    this.warnedOps = [];
    this.allowedOps = [];
    this.maxOpsHistory = 100;
  }

  /**
   * Trigger on PreToolUse
   */
  shouldTrigger(event) {
    return event.type === AgentTrigger.PRE_TOOL_USE ||
           event.type === 'pre_tool_use' ||
           event.tool !== undefined;
  }

  /**
   * Analyze tool use for risk
   */
  async analyze(event, context) {
    const tool = event.tool || event.name || 'unknown';
    const input = event.input || event.params || {};
    const command = this._extractCommand(tool, input);

    // Check for blocked patterns
    for (const pattern of this.blockedPatterns) {
      if (pattern.test(command)) {
        return {
          blocked: true,
          risk: RiskLevel.CRITICAL,
          category: RiskCategory.DESTRUCTIVE,
          pattern: pattern.toString(),
          command,
          message: 'Command matches blocked pattern',
          confidence: PHI_INV,
        };
      }
    }

    // Check for warning patterns
    for (const { pattern, category, risk } of this.warningPatterns) {
      if (pattern.test(command)) {
        return {
          blocked: false,
          warning: true,
          risk,
          category,
          pattern: pattern.toString(),
          command,
          message: `Command matches warning pattern: ${category}`,
          confidence: risk.threshold,
        };
      }
    }

    // Check sensitive file access
    const sensitiveFile = this._checkSensitiveFiles(tool, input);
    if (sensitiveFile) {
      return {
        blocked: false,
        warning: true,
        risk: RiskLevel.HIGH,
        category: RiskCategory.SENSITIVE,
        file: sensitiveFile,
        command,
        message: `Accessing sensitive file: ${sensitiveFile}`,
        confidence: PHI_INV_2,
      };
    }

    // Check network access
    const networkRisk = this._checkNetworkAccess(tool, input);
    if (networkRisk) {
      return {
        blocked: false,
        warning: networkRisk.warning,
        risk: networkRisk.risk,
        category: RiskCategory.NETWORK,
        command,
        message: networkRisk.message,
        confidence: networkRisk.risk.threshold,
      };
    }

    // No risk detected
    return {
      blocked: false,
      warning: false,
      risk: RiskLevel.SAFE,
      command,
      confidence: 0,
    };
  }

  /**
   * Decide action based on risk analysis
   */
  async decide(analysis, context) {
    const { blocked, warning, risk, category, command, message } = analysis;

    // Critical block - no override
    if (blocked) {
      this._recordOp('blocked', { command, risk, category, message });
      this.recordPattern({
        type: 'blocked_command',
        command,
        risk: risk.label,
        category,
      });

      return {
        response: AgentResponse.BLOCK,
        action: true,
        risk,
        category,
        message: `[BLOCKED] ${message}`,
        command,
      };
    }

    // Warning - allow with warning
    if (warning) {
      this._recordOp('warned', { command, risk, category, message });

      // High risk requires confirmation
      if (risk.level >= RiskLevel.HIGH.level) {
        return {
          response: AgentResponse.WARN,
          action: true,
          risk,
          category,
          message: `[WARNING] ${message}`,
          command,
          requiresConfirmation: true,
        };
      }

      // Medium risk - warn but allow
      return {
        response: AgentResponse.WARN,
        action: false,
        risk,
        category,
        message: `[CAUTION] ${message}`,
        command,
        requiresConfirmation: false,
      };
    }

    // Safe - allow
    this._recordOp('allowed', { command });
    return {
      response: AgentResponse.ALLOW,
      action: false,
    };
  }

  /**
   * Extract command string from tool input
   * @private
   */
  _extractCommand(tool, input) {
    // Bash tool
    if (tool === 'Bash' || tool === 'bash') {
      return input.command || input.cmd || '';
    }

    // Write/Edit tools
    if (tool === 'Write' || tool === 'Edit') {
      return `${tool} ${input.file_path || input.path || ''}`;
    }

    // Read tool
    if (tool === 'Read') {
      return `Read ${input.file_path || input.path || ''}`;
    }

    // WebFetch
    if (tool === 'WebFetch') {
      return `WebFetch ${input.url || ''}`;
    }

    // Default: stringify input
    return `${tool} ${JSON.stringify(input).slice(0, 200)}`;
  }

  /**
   * Check if accessing sensitive files
   * @private
   */
  _checkSensitiveFiles(tool, input) {
    const paths = [];

    // Extract paths from input
    if (input.file_path) paths.push(input.file_path);
    if (input.path) paths.push(input.path);
    if (input.command) {
      // Extract paths from bash command
      const matches = input.command.match(/[\/~][\w\/.~-]+/g) || [];
      paths.push(...matches);
    }

    for (const path of paths) {
      for (const pattern of this.sensitiveFiles) {
        if (pattern.test(path)) {
          return path;
        }
      }
    }

    return null;
  }

  /**
   * Check for network access risks
   * @private
   */
  _checkNetworkAccess(tool, input) {
    // WebFetch is always network
    if (tool === 'WebFetch') {
      const url = input.url || '';

      // Check for suspicious URLs
      if (url.includes('pastebin') || url.includes('hastebin')) {
        return {
          warning: true,
          risk: RiskLevel.MEDIUM,
          message: 'Fetching from paste site',
        };
      }

      // External network access
      return {
        warning: false,
        risk: RiskLevel.LOW,
        message: 'External network access',
      };
    }

    // Check bash commands for network
    if (tool === 'Bash' || tool === 'bash') {
      const cmd = input.command || '';

      if (/curl|wget|fetch|nc\s/.test(cmd)) {
        return {
          warning: true,
          risk: RiskLevel.MEDIUM,
          message: 'Network command detected',
        };
      }
    }

    return null;
  }

  /**
   * Record operation for history
   * @private
   */
  _recordOp(type, data) {
    const record = {
      ...data,
      timestamp: Date.now(),
    };

    switch (type) {
      case 'blocked':
        // Enforce bounds before pushing (FIFO eviction)
        while (this.blockedOps.length >= this.maxOpsHistory) {
          this.blockedOps.shift();
        }
        this.blockedOps.push(record);
        break;
      case 'warned':
        while (this.warnedOps.length >= this.maxOpsHistory) {
          this.warnedOps.shift();
        }
        this.warnedOps.push(record);
        break;
      case 'allowed':
        while (this.allowedOps.length >= this.maxOpsHistory) {
          this.allowedOps.shift();
        }
        this.allowedOps.push(record);
        break;
    }
  }

  /**
   * Check if command is safe
   * @param {string} command - Command to check
   * @returns {Object} Safety assessment
   */
  checkCommand(command) {
    return this.analyze({ tool: 'Bash', input: { command } }, {});
  }

  /**
   * Add custom blocked pattern
   * @param {RegExp} pattern - Pattern to block
   */
  addBlockedPattern(pattern) {
    if (pattern instanceof RegExp) {
      this.blockedPatterns.push(pattern);
    }
  }

  /**
   * Add custom warning pattern
   * @param {Object} config - Pattern config
   */
  addWarningPattern({ pattern, category, risk }) {
    if (pattern instanceof RegExp) {
      this.warningPatterns.push({
        pattern,
        category: category || RiskCategory.UNKNOWN,
        risk: risk || RiskLevel.MEDIUM,
      });
    }
  }

  /**
   * Get blocked operations
   * @returns {Object[]} Blocked ops
   */
  getBlockedOps() {
    return [...this.blockedOps];
  }

  /**
   * Get guardian summary
   * @returns {Object} Summary
   */
  getSummary() {
    return {
      ...this.getStats(),
      blockedCount: this.blockedOps.length,
      warnedCount: this.warnedOps.length,
      allowedCount: this.allowedOps.length,
      recentBlocked: this.blockedOps.slice(-5),
      recentWarned: this.warnedOps.slice(-5),
      customPatterns: {
        blocked: this.blockedPatterns.length,
        warning: this.warningPatterns.length,
      },
    };
  }

  /**
   * Clear history
   */
  clear() {
    this.blockedOps = [];
    this.warnedOps = [];
    this.allowedOps = [];
  }
}

export default Guardian;
