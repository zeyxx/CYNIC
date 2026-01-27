/**
 * @cynic/node - Collective Janitor Agent
 *
 * JANITOR (Yesod - Foundation): The Cleaner
 *
 * "Je nettoie les fondations. La qualité est un acte d'amour.
 *  Le code mort est un mensonge silencieux." - κυνικός Janitor
 *
 * Philosophy: Yesod (Foundation) - Maintain clean foundations for all to build on.
 * Trigger: Periodic, PostToolUse (after code modifications)
 * Behavior: Non-blocking (reports issues, suggests fixes)
 *
 * Collective features:
 * - Event bus integration (emits QUALITY_REPORT, AUTO_FIX_APPLIED, DEAD_CODE_DETECTED)
 * - Profile-aware strictness (adapts to user expertise)
 * - Pattern learning from ANALYST
 * - φ-aligned quality thresholds
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/agents/collective/janitor
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
  QualityReportEvent,
  AutoFixAppliedEvent,
  DeadCodeDetectedEvent,
} from '../events.js';
import { ProfileLevel } from '../../profile/calculator.js';
import { DogAutonomousBehaviors } from './autonomous.js';

/**
 * φ-aligned constants for Janitor
 */
export const JANITOR_CONSTANTS = {
  /** Max cyclomatic complexity threshold (φ² × 10 ≈ 26) */
  COMPLEXITY_THRESHOLD: Math.round(PHI_2 * 10),

  /** Max file length (Fib(16) = 987 lines) */
  MAX_FILE_LENGTH: 987,

  /** Max function length (Fib(10) = 55 lines) */
  MAX_FUNCTION_LENGTH: 55,

  /** Stale branch age (Fib(8) = 21 days) */
  STALE_BRANCH_DAYS: 21,

  /** Quality score threshold for warning (φ⁻¹ = 61.8%) */
  QUALITY_WARNING_THRESHOLD: PHI_INV,

  /** Quality score threshold for fail (φ⁻² = 38.2%) */
  QUALITY_FAIL_THRESHOLD: PHI_INV_2,

  /** Max issues before detailed report (Fib(13) = 233) */
  MAX_DETAILED_ISSUES: 233,

  /** Max dead code entries tracked (Fib(8) = 21) */
  MAX_DEAD_CODE_TRACKED: 21,

  /** Scan interval (Fib(21) = 10946 ms ≈ 11 seconds for dev) */
  SCAN_INTERVAL_MS: 10946,

  /** Max files per scan (Fib(8) = 21) */
  MAX_FILES_PER_SCAN: 21,
};

/**
 * Quality issue severity levels
 */
export const QualitySeverity = {
  CRITICAL: { level: 4, label: 'Critical', weight: PHI_2, emoji: '🔴' },
  HIGH: { level: 3, label: 'High', weight: PHI, emoji: '🟠' },
  MEDIUM: { level: 2, label: 'Medium', weight: 1.0, emoji: '🟡' },
  LOW: { level: 1, label: 'Low', weight: PHI_INV, emoji: '🟢' },
};

/**
 * Issue types the Janitor detects
 */
export const IssueType = {
  // Code structure
  HIGH_COMPLEXITY: 'high_complexity',
  LONG_FILE: 'long_file',
  LONG_FUNCTION: 'long_function',
  DEEP_NESTING: 'deep_nesting',

  // Dead code
  UNUSED_VARIABLE: 'unused_variable',
  UNUSED_FUNCTION: 'unused_function',
  UNUSED_IMPORT: 'unused_import',
  UNREACHABLE_CODE: 'unreachable_code',

  // Style
  INCONSISTENT_NAMING: 'inconsistent_naming',
  MISSING_DOCUMENTATION: 'missing_documentation',
  INCONSISTENT_INDENTATION: 'inconsistent_indentation',

  // Maintenance
  TODO_COMMENT: 'todo_comment',
  FIXME_COMMENT: 'fixme_comment',
  STALE_BRANCH: 'stale_branch',
  OUTDATED_DEPENDENCY: 'outdated_dependency',
};

/**
 * Profile-based strictness levels
 */
const PROFILE_STRICTNESS = {
  [ProfileLevel.NOVICE]: {
    strictnessMultiplier: 0.5,    // More lenient
    autoFixEnabled: true,         // Auto-fix simple issues
    detailedExplanations: true,   // Explain issues in detail
    maxIssuesReported: 5,         // Don't overwhelm
  },
  [ProfileLevel.APPRENTICE]: {
    strictnessMultiplier: 0.7,
    autoFixEnabled: true,
    detailedExplanations: true,
    maxIssuesReported: 13,        // Fib(7)
  },
  [ProfileLevel.PRACTITIONER]: {
    strictnessMultiplier: 1.0,    // Standard strictness
    autoFixEnabled: false,        // User handles fixes
    detailedExplanations: false,
    maxIssuesReported: 34,        // Fib(9)
  },
  [ProfileLevel.EXPERT]: {
    strictnessMultiplier: 1.2,    // Higher standards
    autoFixEnabled: false,
    detailedExplanations: false,
    maxIssuesReported: 89,        // Fib(11)
  },
  [ProfileLevel.MASTER]: {
    strictnessMultiplier: PHI,    // φ strictness
    autoFixEnabled: false,
    detailedExplanations: false,
    maxIssuesReported: 233,       // Fib(13)
  },
};

/**
 * Collective Janitor Agent - Code Quality & Hygiene
 *
 * The Janitor maintains clean foundations:
 * - Detects code quality issues
 * - Finds dead code
 * - Tracks technical debt
 * - Suggests and optionally applies fixes
 */
export class CollectiveJanitor extends BaseAgent {
  /**
   * Create Janitor agent
   * @param {Object} [options] - Agent options
   */
  constructor(options = {}) {
    super({
      name: 'Janitor',
      trigger: AgentTrigger.POST_TOOL_USE,
      behavior: AgentBehavior.NON_BLOCKING,
      ...options,
    });

    // Event bus for collective communication
    this.eventBus = options.eventBus || null;

    // Autonomous capabilities (Phase 16: Total Memory + Full Autonomy)
    this.autonomous = options.autonomous || null;

    // Current profile level (affects strictness)
    this.profileLevel = options.profileLevel || ProfileLevel.PRACTITIONER;

    // Tracked issues
    this.issues = [];
    this.deadCode = [];
    this.qualityHistory = [];

    // Statistics
    this.stats = {
      scansPerformed: 0,
      issuesFound: 0,
      issuesFixed: 0,
      deadCodeFound: 0,
      averageQualityScore: 0,
    };

    // Subscribe to events if bus provided
    if (this.eventBus) {
      this._subscribeToEvents();
    }
  }

  /**
   * Subscribe to collective events
   * @private
   */
  _subscribeToEvents() {
    // Listen for profile updates
    this.eventBus.subscribe(
      AgentEvent.PROFILE_UPDATED,
      AgentId.JANITOR,
      (event) => this._handleProfileUpdate(event)
    );

    // Listen for patterns from Analyst (might indicate code issues)
    this.eventBus.subscribe(
      AgentEvent.PATTERN_DETECTED,
      AgentId.JANITOR,
      (event) => this._handlePatternDetected(event)
    );
  }

  /**
   * Handle profile level updates
   * @private
   */
  _handleProfileUpdate(event) {
    const newLevel = event.payload?.newLevel;
    if (newLevel !== undefined) {
      // newLevel can be a number (1-8) or string ('NOVICE', etc.)
      this.profileLevel = newLevel;
    }
  }

  /**
   * Handle patterns detected by Analyst
   * @private
   */
  _handlePatternDetected(event) {
    const pattern = event.payload;

    // If Analyst detects error patterns, might indicate code issues
    if (pattern.patternType === 'error' || pattern.category === 'error') {
      // Schedule a quality scan
      this._scheduleQualityScan(pattern);
    }
  }

  /**
   * Check if agent should trigger
   * @param {Object} event - Event to check
   * @returns {boolean} Whether to trigger
   */
  shouldTrigger(event) {
    const type = event.type || event.name || '';

    // Trigger after file modifications
    if (type === 'PostToolUse' || type === 'post_tool_use') {
      const tool = event.tool || event.payload?.tool || '';
      const toolLower = tool.toLowerCase();

      // Trigger after code editing tools
      return toolLower.includes('edit') ||
             toolLower.includes('write') ||
             toolLower.includes('create') ||
             toolLower.includes('delete');
    }

    // Periodic trigger for scanning
    if (type === 'periodic' || type === 'scan') {
      return true;
    }

    return false;
  }

  /**
   * Process event and analyze code quality
   * @param {Object} event - Event to process
   * @param {Object} [context] - Event context
   * @returns {Promise<Object>} Processing result
   */
  async process(event, context = {}) {
    this.stats.scansPerformed++;

    const settings = PROFILE_STRICTNESS[this.profileLevel] || PROFILE_STRICTNESS[ProfileLevel.PRACTITIONER];

    // Analyze the modified file(s)
    const analysis = await this._analyzeQuality(event, context, settings);

    // Calculate quality score
    const qualityScore = this._calculateQualityScore(analysis.issues);

    // Store in history
    this._recordQualityReport(qualityScore, analysis);

    // Emit quality report event
    if (this.eventBus && analysis.issues.length > 0) {
      const reportEvent = new QualityReportEvent({
        score: qualityScore,
        issues: analysis.issues.slice(0, settings.maxIssuesReported),
        suggestions: analysis.suggestions,
        filesAnalyzed: analysis.filesAnalyzed,
      });
      await this.eventBus.publish(reportEvent);
    }

    // Autonomous: Report quality metrics
    if (this.autonomous) {
      DogAutonomousBehaviors.janitor.reportQuality(
        this.autonomous,
        this.persistence?.userId || 'unknown',
        { score: qualityScore / 100, issues: analysis.issues.length }
      ).catch(() => { /* Non-critical */ });
    }

    // Emit dead code events
    for (const deadCode of analysis.deadCode) {
      if (this.eventBus) {
        const deadCodeEvent = new DeadCodeDetectedEvent(deadCode);
        await this.eventBus.publish(deadCodeEvent);
      }
    }

    // Auto-fix if enabled
    if (settings.autoFixEnabled && analysis.autoFixable.length > 0) {
      const fixResults = await this._applyAutoFixes(analysis.autoFixable);
      analysis.fixesApplied = fixResults;
    }

    // Determine response based on quality score
    let response = AgentResponse.ALLOW;
    if (qualityScore < JANITOR_CONSTANTS.QUALITY_FAIL_THRESHOLD) {
      response = AgentResponse.WARN;
    }

    return {
      agent: 'janitor',
      response,
      qualityScore,
      issues: analysis.issues.length,
      deadCode: analysis.deadCode.length,
      suggestions: analysis.suggestions,
      fixesApplied: analysis.fixesApplied || [],
      profileLevel: this.profileLevel,
    };
  }

  /**
   * Analyze code quality
   * @private
   */
  async _analyzeQuality(event, context, settings) {
    const issues = [];
    const deadCode = [];
    const suggestions = [];
    const autoFixable = [];

    // Get the file content if available
    const fileContent = event.payload?.content || context.fileContent;
    const filePath = event.payload?.file || context.filePath;

    if (fileContent) {
      // Analyze the content
      const contentAnalysis = this._analyzeContent(fileContent, filePath, settings);
      issues.push(...contentAnalysis.issues);
      deadCode.push(...contentAnalysis.deadCode);
      suggestions.push(...contentAnalysis.suggestions);
      autoFixable.push(...contentAnalysis.autoFixable);
    }

    return {
      filesAnalyzed: fileContent ? 1 : 0,
      issues,
      deadCode,
      suggestions,
      autoFixable,
    };
  }

  /**
   * Analyze file content for quality issues
   * @private
   */
  _analyzeContent(content, filePath, settings) {
    const issues = [];
    const deadCode = [];
    const suggestions = [];
    const autoFixable = [];
    const lines = content.split('\n');
    const strictness = settings.strictnessMultiplier;

    // Check file length
    const maxLength = Math.round(JANITOR_CONSTANTS.MAX_FILE_LENGTH / strictness);
    if (lines.length > maxLength) {
      issues.push({
        type: IssueType.LONG_FILE,
        severity: QualitySeverity.MEDIUM,
        message: `File has ${lines.length} lines (max recommended: ${maxLength})`,
        file: filePath,
        line: null,
      });
      suggestions.push(`Consider splitting ${filePath} into smaller modules`);
    }

    // Analyze line by line
    let currentFunction = null;
    let functionStartLine = 0;
    let functionLines = 0;
    let nestingLevel = 0;
    let maxNesting = 0;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const lineNum = i + 1;

      // Track nesting
      const openBraces = (line.match(/{/g) || []).length;
      const closeBraces = (line.match(/}/g) || []).length;
      nestingLevel += openBraces - closeBraces;
      maxNesting = Math.max(maxNesting, nestingLevel);

      // Detect function start
      const funcMatch = line.match(/(?:function|async function|(?:async\s+)?(?:\w+\s*)?\(.*?\)\s*(?:=>)?)\s*{/);
      if (funcMatch) {
        // End previous function if exists
        if (currentFunction && functionLines > 0) {
          this._checkFunctionLength(currentFunction, functionStartLine, functionLines, strictness, issues, suggestions);
        }

        currentFunction = funcMatch[0];
        functionStartLine = lineNum;
        functionLines = 0;
      }

      if (currentFunction) {
        functionLines++;
      }

      // Check for TODO/FIXME comments
      if (line.includes('TODO')) {
        issues.push({
          type: IssueType.TODO_COMMENT,
          severity: QualitySeverity.LOW,
          message: 'TODO comment found',
          file: filePath,
          line: lineNum,
        });
      }

      if (line.includes('FIXME')) {
        issues.push({
          type: IssueType.FIXME_COMMENT,
          severity: QualitySeverity.MEDIUM,
          message: 'FIXME comment found - potential bug',
          file: filePath,
          line: lineNum,
        });
      }

      // Check for console.log (potential dead debug code)
      if (line.match(/console\.(log|debug|info)\s*\(/)) {
        deadCode.push({
          type: 'debug_statement',
          file: filePath,
          line: lineNum,
          name: 'console.log',
          confidence: 0.7,
        });
        autoFixable.push({
          type: 'remove_debug',
          file: filePath,
          line: lineNum,
          description: 'Remove console.log statement',
        });
      }

      // Check for unused variables (simple pattern)
      const unusedVarMatch = line.match(/(?:const|let|var)\s+(\w+)\s*=.*\/\/\s*unused/i);
      if (unusedVarMatch) {
        deadCode.push({
          type: 'unused_variable',
          file: filePath,
          line: lineNum,
          name: unusedVarMatch[1],
          confidence: 0.9,
        });
      }
    }

    // Check last function
    if (currentFunction && functionLines > 0) {
      this._checkFunctionLength(currentFunction, functionStartLine, functionLines, strictness, issues, suggestions);
    }

    // Check max nesting
    const maxNestingThreshold = Math.round(5 / strictness);
    if (maxNesting > maxNestingThreshold) {
      issues.push({
        type: IssueType.DEEP_NESTING,
        severity: QualitySeverity.HIGH,
        message: `Deep nesting detected (${maxNesting} levels, max recommended: ${maxNestingThreshold})`,
        file: filePath,
        line: null,
      });
      suggestions.push('Consider refactoring to reduce nesting depth');
    }

    // Track dead code found
    this.stats.deadCodeFound += deadCode.length;
    this._updateDeadCodeTracking(deadCode);

    return { issues, deadCode, suggestions, autoFixable };
  }

  /**
   * Check function length
   * @private
   */
  _checkFunctionLength(funcName, startLine, lineCount, strictness, issues, suggestions) {
    const maxFuncLength = Math.round(JANITOR_CONSTANTS.MAX_FUNCTION_LENGTH / strictness);
    if (lineCount > maxFuncLength) {
      issues.push({
        type: IssueType.LONG_FUNCTION,
        severity: QualitySeverity.MEDIUM,
        message: `Function has ${lineCount} lines (max recommended: ${maxFuncLength})`,
        file: null,
        line: startLine,
      });
      suggestions.push(`Consider breaking down function at line ${startLine} into smaller functions`);
    }
  }

  /**
   * Calculate quality score from issues
   * @private
   */
  _calculateQualityScore(issues) {
    if (issues.length === 0) {
      return 100;
    }

    // Calculate weighted penalty
    let penalty = 0;
    for (const issue of issues) {
      const severity = issue.severity || QualitySeverity.LOW;
      penalty += severity.weight;
    }

    // Score decreases with issues, capped at 0
    const score = Math.max(0, 100 - penalty * 5);
    return Math.round(score * 10) / 10;
  }

  /**
   * Record quality report in history
   * @private
   */
  _recordQualityReport(score, analysis) {
    this.qualityHistory.push({
      timestamp: Date.now(),
      score,
      issueCount: analysis.issues.length,
      deadCodeCount: analysis.deadCode.length,
    });

    // Update stats
    this.stats.issuesFound += analysis.issues.length;

    // Calculate running average
    const totalScores = this.qualityHistory.reduce((sum, r) => sum + r.score, 0);
    this.stats.averageQualityScore = Math.round(totalScores / this.qualityHistory.length * 10) / 10;

    // Trim history (keep Fib(13) = 233 entries)
    while (this.qualityHistory.length > 233) {
      this.qualityHistory.shift();
    }
  }

  /**
   * Update dead code tracking
   * @private
   */
  _updateDeadCodeTracking(newDeadCode) {
    for (const code of newDeadCode) {
      // Check if already tracked
      const exists = this.deadCode.find(
        (dc) => dc.file === code.file && dc.line === code.line && dc.name === code.name
      );

      if (!exists) {
        this.deadCode.push(code);
      }
    }

    // Trim to max tracked
    while (this.deadCode.length > JANITOR_CONSTANTS.MAX_DEAD_CODE_TRACKED) {
      this.deadCode.shift();
    }
  }

  /**
   * Apply auto-fixes
   * @private
   */
  async _applyAutoFixes(autoFixable) {
    const results = [];

    for (const fix of autoFixable) {
      // In a real implementation, this would modify the file
      // For now, we just record the intention

      results.push({
        type: fix.type,
        file: fix.file,
        line: fix.line,
        applied: false, // Would be true if actually applied
        description: fix.description,
      });

      // Emit auto-fix event
      if (this.eventBus) {
        const fixEvent = new AutoFixAppliedEvent({
          type: fix.type,
          file: fix.file,
          description: fix.description,
        });
        await this.eventBus.publish(fixEvent);
      }

      this.stats.issuesFixed++;
    }

    return results;
  }

  /**
   * Schedule a quality scan
   * @private
   */
  _scheduleQualityScan(pattern) {
    // In a real implementation, this would schedule an async scan
    // For now, we just log the intention
    if (!this._scheduledScans) {
      this._scheduledScans = [];
    }

    this._scheduledScans.push({
      reason: pattern.patternType || 'manual',
      scheduledAt: Date.now(),
    });
  }

  /**
   * Vote on consensus request from Janitor's code quality perspective
   * @param {string} question - The question to vote on
   * @param {Object} context - Context for the decision
   * @returns {Object} Vote result
   */
  voteOnConsensus(question, context = {}) {
    const questionLower = (question || '').toLowerCase();

    // Janitor cares about code quality, cleanliness, technical debt
    const qualityPatterns = ['clean', 'refactor', 'quality', 'debt', 'lint', 'format'];
    const messyPatterns = ['hack', 'quick fix', 'workaround', 'temporary', 'skip tests'];

    const isQualityFocused = qualityPatterns.some(p => questionLower.includes(p));
    const isMessy = messyPatterns.some(p => questionLower.includes(p));

    if (isQualityFocused) {
      return {
        vote: 'approve',
        reason: '*sniff* Janitor approves - quality-focused approach aligns with clean foundations.',
      };
    }

    if (isMessy) {
      return {
        vote: 'reject',
        reason: '*GROWL* Janitor rejects - this introduces technical debt. Clean it properly.',
      };
    }

    return {
      vote: 'abstain',
      reason: '*yawn* Janitor abstains - no strong quality implications either way.',
    };
  }

  /**
   * Get agent summary
   * @returns {Object} Summary of Janitor state
   */
  getSummary() {
    return {
      agent: 'janitor',
      sefirah: 'Yesod (Foundation)',
      profileLevel: this.profileLevel,
      stats: this.stats,
      recentIssues: this.issues.slice(-5),
      deadCodeCount: this.deadCode.length,
      averageQualityScore: this.stats.averageQualityScore,
      constants: JANITOR_CONSTANTS,
    };
  }

  /**
   * Clear agent state
   */
  clear() {
    this.issues = [];
    this.deadCode = [];
    this.qualityHistory = [];
    this.stats = {
      scansPerformed: 0,
      issuesFound: 0,
      issuesFixed: 0,
      deadCodeFound: 0,
      averageQualityScore: 0,
    };
  }
}

/**
 * Factory function to create Janitor
 * @param {Object} [options] - Janitor options
 * @returns {CollectiveJanitor} Janitor instance
 */
export function createJanitor(options = {}) {
  return new CollectiveJanitor(options);
}

export default CollectiveJanitor;
