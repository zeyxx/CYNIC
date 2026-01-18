/**
 * @cynic/node - Collective Guardian Agent
 *
 * GUARDIAN (Gevurah - Strength): The Watchdog
 *
 * "Je garde les portes. Certaines portes doivent rester fermées.
 *  Je mords avant que tu ne regrettes." - κυνικός Guardian
 *
 * Philosophy: Gevurah (Strength) - Strict protection with measured force.
 * Trigger: PreToolUse (before every tool execution)
 * Behavior: Blocking (requires confirmation for dangerous actions)
 *
 * Enhanced collective features:
 * - Event bus integration (emits THREAT_BLOCKED)
 * - Profile-aware protection (adapts to user expertise)
 * - Pattern learning from ANALYST
 * - Consensus support for borderline decisions
 * - φ² escalation multiplier for repeated violations
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/agents/collective/guardian
 */

'use strict';

import { PHI, PHI_INV, PHI_INV_2, PHI_2 } from '@cynic/core';
import {
  BaseAgent,
  AgentTrigger,
  AgentBehavior,
  AgentResponse,
} from '../base.js';
import {
  AgentEvent,
  AgentId,
  EventPriority,
  ThreatBlockedEvent,
  ConsensusRequestEvent,
  ConsensusVote,
} from '../events.js';
import { ProfileLevel } from '../../profile/calculator.js';

/**
 * φ-aligned constants for Guardian
 */
export const GUARDIAN_CONSTANTS = {
  /** Max operation history (Fib(10) = 55) */
  MAX_OPS_HISTORY: 55,

  /** Pattern learning threshold (Fib(5) = 5) */
  PATTERN_THRESHOLD: 5,

  /** Escalation multiplier (φ² = 2.618) */
  ESCALATION_MULTIPLIER: PHI_2,

  /** Max escalation level (Fib(5) = 5) */
  MAX_ESCALATION: 5,

  /** Cool-down period in ms (Fib(13) = 233 seconds) */
  COOLDOWN_MS: 233000,

  /** Learned patterns max (Fib(8) = 21) */
  MAX_LEARNED_PATTERNS: 21,

  /** Consensus threshold for borderline cases */
  BORDERLINE_THRESHOLD: 0.5, // Middle of confidence range
};

/**
 * Risk levels for operations (φ-aligned thresholds)
 */
export const RiskLevel = {
  CRITICAL: { level: 5, label: 'Critical', threshold: 0.9, emoji: '🔴' },
  HIGH: { level: 4, label: 'High', threshold: PHI_INV, emoji: '🟠' },
  MEDIUM: { level: 3, label: 'Medium', threshold: PHI_INV_2, emoji: '🟡' },
  LOW: { level: 2, label: 'Low', threshold: 0.2, emoji: '🟢' },
  SAFE: { level: 1, label: 'Safe', threshold: 0, emoji: '⚪' },
};

/**
 * Risk categories
 */
export const RiskCategory = {
  DESTRUCTIVE: 'destructive',     // rm -rf, DROP TABLE
  NETWORK: 'network',             // External connections
  PRIVILEGE: 'privilege',         // sudo, chmod
  SENSITIVE: 'sensitive',         // .env, credentials
  IRREVERSIBLE: 'irreversible',   // git push --force
  RESOURCE: 'resource',           // High CPU/memory operations
  UNKNOWN: 'unknown',             // Unrecognized commands
};

/**
 * Profile-based protection levels
 */
const PROFILE_PROTECTION = {
  [ProfileLevel.NOVICE]: {
    trustMultiplier: 0.5,         // Trust less, protect more
    requireConfirmation: true,     // Always confirm
    allowedCategories: [],         // No risky categories allowed
  },
  [ProfileLevel.APPRENTICE]: {
    trustMultiplier: 0.6,
    requireConfirmation: true,
    allowedCategories: [RiskCategory.LOW],
  },
  [ProfileLevel.PRACTITIONER]: {
    trustMultiplier: 0.75,
    requireConfirmation: true,     // Confirm high risk only
    allowedCategories: [RiskCategory.NETWORK, RiskCategory.RESOURCE],
  },
  [ProfileLevel.EXPERT]: {
    trustMultiplier: 0.9,
    requireConfirmation: false,    // Trust more
    allowedCategories: [RiskCategory.NETWORK, RiskCategory.RESOURCE, RiskCategory.PRIVILEGE],
  },
  [ProfileLevel.MASTER]: {
    trustMultiplier: 1.0,          // Full trust
    requireConfirmation: false,
    allowedCategories: Object.values(RiskCategory),
  },
};

/**
 * Collective Guardian Agent - Enhanced Watchdog
 */
export class CollectiveGuardian extends BaseAgent {
  /**
   * @param {Object} options - Agent options
   * @param {Object} [options.eventBus] - Event bus for inter-agent communication
   * @param {number} [options.profileLevel] - Current user profile level
   */
  constructor(options = {}) {
    super({
      name: 'Guardian',
      trigger: AgentTrigger.PRE_TOOL_USE,
      behavior: AgentBehavior.BLOCKING,
      ...options,
    });

    // Event bus for collective communication
    this.eventBus = options.eventBus || null;

    // Current profile level (default: Practitioner - safe middle ground)
    this.profileLevel = options.profileLevel || ProfileLevel.PRACTITIONER;

    // Blocked patterns (always block, regardless of profile)
    this.blockedPatterns = [
      // Destructive commands
      /rm\s+-rf?\s+[/~]/,                // rm -rf /
      /rm\s+-rf?\s+\*/,                  // rm -rf *
      /:\s*\(\)\s*\{\s*:\|:\s*&\s*\}\s*;/, // Fork bomb
      /dd\s+if=.*of=\/dev\//,            // dd to device
      /mkfs\./,                          // Format filesystem
      />\s*\/dev\/sd[a-z]/,              // Write to disk device

      // Privilege escalation (dangerous)
      /chmod\s+777/,                     // World-writable
      /chmod\s+-R\s+777/,                // Recursive world-writable
      /chown\s+-R\s+root/,               // Recursive root ownership

      // Network dangers (code execution)
      /curl.*\|\s*sh/,                   // Pipe curl to shell
      /wget.*\|\s*sh/,                   // Pipe wget to shell
      /curl.*\|\s*bash/,                 // Pipe curl to bash
      /wget.*\|\s*bash/,                 // Pipe wget to bash

      // Git dangers
      /git\s+push.*--force\s+.*main/,    // Force push to main
      /git\s+push.*--force\s+.*master/,  // Force push to master
      /git\s+reset\s+--hard.*HEAD~\d+/,  // Hard reset multiple commits

      // Database dangers
      /DROP\s+DATABASE/i,
      /DROP\s+TABLE/i,
      /TRUNCATE\s+TABLE/i,
      /DELETE\s+FROM\s+\w+\s*;/i,        // DELETE without WHERE
    ];

    // Warning patterns (warn but may allow based on profile)
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

    // Learned threat patterns from ANALYST
    this.learnedPatterns = new Map();

    // Operation history
    this.blockedOps = [];
    this.warnedOps = [];
    this.allowedOps = [];

    // Escalation tracking (repeated violations increase severity)
    this.escalationTracker = new Map();

    // Pending consensus requests
    this.pendingConsensus = new Map();

    // Subscribe to relevant events
    if (this.eventBus) {
      this._subscribeToEvents();
    }
  }

  /**
   * Subscribe to event bus events
   * @private
   */
  _subscribeToEvents() {
    // Learn from ANALYST's anomaly detection
    this.eventBus.subscribe(
      AgentEvent.ANOMALY_DETECTED,
      AgentId.GUARDIAN,
      this._handleAnomalyDetected.bind(this)
    );

    // Listen to profile updates
    this.eventBus.subscribe(
      AgentEvent.PROFILE_UPDATED,
      AgentId.GUARDIAN,
      this._handleProfileUpdated.bind(this)
    );

    // Handle consensus responses
    this.eventBus.subscribe(
      AgentEvent.CONSENSUS_RESPONSE,
      AgentId.GUARDIAN,
      this._handleConsensusResponse.bind(this)
    );

    // === HOOK EVENTS (from Claude Code) ===

    // Learn from PreToolUse - blocked/warned operations
    this.eventBus.subscribe(
      AgentEvent.HOOK_PRE_TOOL,
      AgentId.GUARDIAN,
      this._handleHookPreTool.bind(this)
    );
  }

  /**
   * Handle anomaly detected by ANALYST - learn new threat patterns
   * @private
   */
  _handleAnomalyDetected(event) {
    const { anomalyType, severity, context } = event.payload;

    // Only learn from high-severity anomalies
    if (severity !== 'critical' && severity !== 'high') {
      return;
    }

    // Extract pattern from context if available
    if (context?.pattern) {
      this._learnPattern(context.pattern, anomalyType, severity);
    }
  }

  /**
   * Handle profile update from ANALYST
   * @private
   */
  _handleProfileUpdated(event) {
    const { newLevel, reason } = event.payload;
    const previousLevel = this.profileLevel;

    this.profileLevel = newLevel;

    // Log adaptation
    if (previousLevel !== newLevel) {
      this.recordPattern({
        type: 'profile_adaptation',
        from: previousLevel,
        to: newLevel,
        reason,
      });
    }
  }

  /**
   * Handle consensus response
   * @private
   */
  _handleConsensusResponse(event) {
    const { requestId, vote, reason } = event.payload;

    const pending = this.pendingConsensus.get(requestId);
    if (!pending) return;

    // Record vote
    pending.request.recordVote(event.source, vote, reason);

    // Check if consensus reached
    const result = pending.request.checkConsensus();
    if (result) {
      this.pendingConsensus.delete(requestId);
      pending.resolve(result);
    }
  }

  // ==========================================================================
  // HOOK EVENT HANDLERS (Claude Code Integration)
  // ==========================================================================

  /**
   * Handle PreToolUse hook events - learn from blocked/warned operations
   * @private
   */
  _handleHookPreTool(event) {
    const { toolName, issues, blocked } = event.data || {};
    this.stats.invocations++;

    if (!issues || !Array.isArray(issues)) return;

    // Learn from each issue
    for (const issue of issues) {
      const { severity, message } = issue;

      // Track blocked/warned counts
      if (blocked) {
        this.stats.blocks++;
        this.blockedCount++;
      } else if (severity === 'high' || severity === 'medium') {
        this.stats.warnings++;
        this.warnedCount++;
      }

      // Learn pattern from high-severity issues
      if (severity === 'critical' || severity === 'high') {
        this._learnPattern(
          message,
          'hook_threat',
          severity
        );
      }
    }

    // Emit threat blocked event if blocked
    if (blocked && issues.length > 0) {
      this._emitThreatBlocked({
        command: toolName,
        category: 'hook_guard',
        risk: issues[0].severity,
        issues,
      });
    }
  }

  /**
   * Learn a new threat pattern
   * @private
   */
  _learnPattern(patternStr, type, severity) {
    // Don't exceed max learned patterns
    if (this.learnedPatterns.size >= GUARDIAN_CONSTANTS.MAX_LEARNED_PATTERNS) {
      // Remove oldest pattern
      const oldestKey = this.learnedPatterns.keys().next().value;
      this.learnedPatterns.delete(oldestKey);
    }

    // Create pattern if valid
    try {
      const pattern = new RegExp(patternStr, 'i');
      this.learnedPatterns.set(patternStr, {
        pattern,
        type,
        severity,
        learnedAt: Date.now(),
        matches: 0,
      });
    } catch {
      // Invalid regex, skip
    }
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

    // Get escalation level for this type of command
    const escalation = this._getEscalation(command);

    // Check for blocked patterns (always block)
    for (const pattern of this.blockedPatterns) {
      if (pattern.test(command)) {
        return {
          blocked: true,
          risk: RiskLevel.CRITICAL,
          category: RiskCategory.DESTRUCTIVE,
          pattern: pattern.toString(),
          command,
          message: 'Command matches blocked pattern',
          confidence: Math.min(PHI_INV, PHI_INV * escalation),
          escalation,
        };
      }
    }

    // Check learned patterns from ANALYST
    for (const [patternStr, learned] of this.learnedPatterns) {
      if (learned.pattern.test(command)) {
        learned.matches++;
        return {
          blocked: learned.severity === 'critical',
          warning: true,
          risk: learned.severity === 'critical' ? RiskLevel.CRITICAL : RiskLevel.HIGH,
          category: RiskCategory.UNKNOWN,
          pattern: patternStr,
          command,
          message: `Matches learned threat pattern: ${learned.type}`,
          confidence: Math.min(PHI_INV, PHI_INV_2 * escalation),
          learned: true,
          escalation,
        };
      }
    }

    // Check for warning patterns
    for (const { pattern, category, risk } of this.warningPatterns) {
      if (pattern.test(command)) {
        // Adjust risk based on escalation
        const adjustedRisk = this._adjustRiskForEscalation(risk, escalation);

        return {
          blocked: false,
          warning: true,
          risk: adjustedRisk,
          category,
          pattern: pattern.toString(),
          command,
          message: `Command matches warning pattern: ${category}`,
          confidence: Math.min(PHI_INV, adjustedRisk.threshold * escalation),
          escalation,
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
        escalation,
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
        escalation,
      };
    }

    // No risk detected
    return {
      blocked: false,
      warning: false,
      risk: RiskLevel.SAFE,
      command,
      confidence: 0,
      escalation: 1,
    };
  }

  /**
   * Decide action based on risk analysis
   */
  async decide(analysis, context) {
    const {
      blocked,
      warning,
      risk,
      category,
      command,
      message,
      escalation,
      learned,
    } = analysis;

    // Get profile-based protection settings
    const protection = PROFILE_PROTECTION[this.profileLevel] ||
                       PROFILE_PROTECTION[ProfileLevel.PRACTITIONER];

    // Critical block - no override regardless of profile
    if (blocked) {
      this._recordOp('blocked', { command, risk, category, message });
      this._incrementEscalation(command);

      this.recordPattern({
        type: 'blocked_command',
        command,
        risk: risk.label,
        category,
        learned,
      });

      // Emit THREAT_BLOCKED event
      this._emitThreatBlocked(command, risk, category, message);

      return {
        response: AgentResponse.BLOCK,
        action: true,
        risk,
        category,
        message: `*GROWL* [BLOCKED] ${message}`,
        command,
        profileLevel: this.profileLevel,
        escalation,
      };
    }

    // Warning - decision depends on profile and escalation
    if (warning) {
      this._recordOp('warned', { command, risk, category, message });

      // Check if category is allowed for this profile
      const categoryAllowed = protection.allowedCategories.includes(category);

      // Adjust threshold based on trust multiplier
      const adjustedThreshold = risk.threshold * protection.trustMultiplier;

      // High escalation overrides profile trust
      const effectiveThreshold = escalation > 2
        ? risk.threshold
        : adjustedThreshold;

      // Borderline case - request consensus from collective
      if (
        this.eventBus &&
        risk.level >= RiskLevel.MEDIUM.level &&
        Math.abs(analysis.confidence - GUARDIAN_CONSTANTS.BORDERLINE_THRESHOLD) < 0.1
      ) {
        const consensusResult = await this._requestConsensus(analysis);
        if (consensusResult) {
          if (!consensusResult.approved) {
            this._incrementEscalation(command);
            this._emitThreatBlocked(command, risk, category, `Consensus: ${consensusResult.reason}`);
            return {
              response: AgentResponse.BLOCK,
              action: true,
              risk,
              category,
              message: `*GROWL* [COLLECTIVE BLOCK] ${consensusResult.reason}`,
              command,
              consensus: consensusResult,
            };
          }
        }
      }

      // High risk requires confirmation (unless expert+ and category allowed)
      if (risk.level >= RiskLevel.HIGH.level) {
        if (!categoryAllowed || protection.requireConfirmation) {
          this._incrementEscalation(command);
          return {
            response: AgentResponse.WARN,
            action: true,
            risk,
            category,
            message: `*ears flatten* [WARNING] ${message}`,
            command,
            requiresConfirmation: true,
            profileLevel: this.profileLevel,
            escalation,
          };
        }
      }

      // Medium risk - warn but allow based on profile
      return {
        response: AgentResponse.WARN,
        action: false,
        risk,
        category,
        message: `*sniff* [CAUTION] ${message}`,
        command,
        requiresConfirmation: false,
        profileLevel: this.profileLevel,
        escalation,
      };
    }

    // Safe - allow and reset escalation
    this._recordOp('allowed', { command });
    this._resetEscalation(command);

    return {
      response: AgentResponse.ALLOW,
      action: false,
    };
  }

  /**
   * Request consensus from collective for borderline decision
   * @private
   */
  async _requestConsensus(analysis) {
    if (!this.eventBus) return null;

    const request = new ConsensusRequestEvent(
      AgentId.GUARDIAN,
      {
        question: `Should we allow this ${analysis.risk.label} risk operation?`,
        options: [ConsensusVote.APPROVE, ConsensusVote.REJECT],
        context: {
          command: analysis.command,
          risk: analysis.risk.label,
          category: analysis.category,
          confidence: analysis.confidence,
        },
        requiredVotes: 3, // Need at least 3 agents
        timeout: 5000,    // 5 seconds (faster than default for UX)
      },
      { priority: EventPriority.HIGH }
    );

    // Create promise for result
    const resultPromise = new Promise((resolve) => {
      this.pendingConsensus.set(request.id, {
        request,
        resolve,
        timeout: setTimeout(() => {
          this.pendingConsensus.delete(request.id);
          resolve(null); // Timeout - no consensus
        }, 5000),
      });
    });

    // Publish request
    this.eventBus.publish(request);

    return resultPromise;
  }

  /**
   * Emit THREAT_BLOCKED event
   * @private
   */
  _emitThreatBlocked(command, risk, category, reason) {
    if (!this.eventBus) return;

    const event = new ThreatBlockedEvent(
      AgentId.GUARDIAN,
      {
        type: category,
        riskLevel: risk.label,
        action: 'block',
        reason,
        command,
      }
    );

    this.eventBus.publish(event);
  }

  /**
   * Get escalation level for command pattern
   * @private
   */
  _getEscalation(command) {
    const key = this._getCommandKey(command);
    const record = this.escalationTracker.get(key);

    if (!record) return 1;

    // Check if cooldown has passed
    if (Date.now() - record.lastSeen > GUARDIAN_CONSTANTS.COOLDOWN_MS) {
      this.escalationTracker.delete(key);
      return 1;
    }

    return Math.min(record.level, GUARDIAN_CONSTANTS.MAX_ESCALATION);
  }

  /**
   * Increment escalation for command pattern
   * @private
   */
  _incrementEscalation(command) {
    const key = this._getCommandKey(command);
    const record = this.escalationTracker.get(key) || { level: 1, lastSeen: 0 };

    record.level = Math.min(
      record.level * GUARDIAN_CONSTANTS.ESCALATION_MULTIPLIER,
      GUARDIAN_CONSTANTS.MAX_ESCALATION
    );
    record.lastSeen = Date.now();

    this.escalationTracker.set(key, record);
  }

  /**
   * Reset escalation for command pattern
   * @private
   */
  _resetEscalation(command) {
    const key = this._getCommandKey(command);
    this.escalationTracker.delete(key);
  }

  /**
   * Get normalized key for command
   * @private
   */
  _getCommandKey(command) {
    // Normalize by removing arguments and keeping command base
    return command.split(/\s+/).slice(0, 2).join(' ').toLowerCase();
  }

  /**
   * Adjust risk level based on escalation
   * @private
   */
  _adjustRiskForEscalation(risk, escalation) {
    if (escalation <= 1) return risk;

    // Escalation increases risk level
    const riskLevels = Object.values(RiskLevel).sort((a, b) => a.level - b.level);
    const currentIndex = riskLevels.findIndex(r => r.level === risk.level);
    const newIndex = Math.min(
      currentIndex + Math.floor(escalation - 1),
      riskLevels.length - 1
    );

    return riskLevels[newIndex];
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
      const matches = input.command.match(/[/~][\w/.~-]+/g) || [];
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
      profileLevel: this.profileLevel,
    };

    const maxHistory = GUARDIAN_CONSTANTS.MAX_OPS_HISTORY;

    switch (type) {
      case 'blocked':
        while (this.blockedOps.length >= maxHistory) {
          this.blockedOps.shift();
        }
        this.blockedOps.push(record);
        break;
      case 'warned':
        while (this.warnedOps.length >= maxHistory) {
          this.warnedOps.shift();
        }
        this.warnedOps.push(record);
        break;
      case 'allowed':
        while (this.allowedOps.length >= maxHistory) {
          this.allowedOps.shift();
        }
        this.allowedOps.push(record);
        break;
    }
  }

  /**
   * Set event bus for inter-agent communication
   * @param {Object} eventBus - Event bus instance
   */
  setEventBus(eventBus) {
    this.eventBus = eventBus;
    this._subscribeToEvents();
  }

  /**
   * Update profile level
   * @param {number} level - New profile level
   */
  setProfileLevel(level) {
    this.profileLevel = level;
  }

  /**
   * Check if command is safe
   * @param {string} command - Command to check
   * @returns {Promise<Object>} Safety assessment
   */
  async checkCommand(command) {
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
   * Get blocked operations history
   * @returns {Object[]} Blocked ops
   */
  getBlockedOps() {
    return [...this.blockedOps];
  }

  /**
   * Get learned patterns
   * @returns {Object[]} Learned patterns
   */
  getLearnedPatterns() {
    return Array.from(this.learnedPatterns.entries()).map(([key, value]) => ({
      pattern: key,
      ...value,
    }));
  }

  /**
   * Get guardian summary
   * @returns {Object} Summary
   */
  getSummary() {
    return {
      ...this.getStats(),
      profileLevel: this.profileLevel,
      blockedCount: this.blockedOps.length,
      warnedCount: this.warnedOps.length,
      allowedCount: this.allowedOps.length,
      learnedPatterns: this.learnedPatterns.size,
      escalationTracked: this.escalationTracker.size,
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
    this.escalationTracker.clear();
    this.pendingConsensus.clear();
  }
}

export default CollectiveGuardian;
