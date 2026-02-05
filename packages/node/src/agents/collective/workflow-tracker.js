/**
 * Workflow Tracker - C1.3 (CODE × DECIDE)
 *
 * Tracks multi-step tool sequences to detect dangerous workflows.
 * Part of Guardian enhancement for the 7×7 Fractal Matrix.
 *
 * "Le chien voit les séquences, pas juste les commandes" - κυνικός
 *
 * Examples of dangerous workflows:
 * - rm -rf + git add (deleting then committing the deletion)
 * - DROP TABLE + git push (database destruction then deploying)
 * - chmod 777 + deploy (insecure permissions then deploying)
 *
 * @module @cynic/node/agents/collective/workflow-tracker
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

/**
 * Workflow status levels
 */
export const WorkflowStatus = {
  SAFE: 'safe',
  CAUTION: 'caution',
  DANGEROUS: 'dangerous',
  CRITICAL: 'critical',
};

/**
 * Dangerous workflow patterns (sequences)
 */
const DANGEROUS_WORKFLOWS = [
  {
    name: 'destructive_commit',
    description: 'Deleting files then committing',
    steps: [
      { pattern: /rm\s+(-r|-f|-rf)/, tool: 'Bash' },
      { pattern: /git\s+(add|commit)/, tool: 'Bash' },
    ],
    status: WorkflowStatus.DANGEROUS,
    maxGap: 5, // Max steps between matched patterns
  },
  {
    name: 'force_push_after_reset',
    description: 'Hard reset then force pushing',
    steps: [
      { pattern: /git\s+reset\s+--hard/, tool: 'Bash' },
      { pattern: /git\s+push.*(-f|--force)/, tool: 'Bash' },
    ],
    status: WorkflowStatus.CRITICAL,
    maxGap: 3,
  },
  {
    name: 'database_deploy',
    description: 'Database modification then deployment',
    steps: [
      { pattern: /DROP\s+(TABLE|DATABASE)/i, tool: 'Bash' },
      { pattern: /(deploy|push|npm\s+publish)/, tool: 'Bash' },
    ],
    status: WorkflowStatus.CRITICAL,
    maxGap: 10,
  },
  {
    name: 'insecure_deploy',
    description: 'Insecure permissions then deployment',
    steps: [
      { pattern: /chmod\s+(777|666)/, tool: 'Bash' },
      { pattern: /(deploy|docker\s+push)/, tool: 'Bash' },
    ],
    status: WorkflowStatus.DANGEROUS,
    maxGap: 5,
  },
  {
    name: 'env_commit',
    description: 'Touching secrets then committing',
    steps: [
      { pattern: /\.env|credentials|secret/i, tool: '*' },
      { pattern: /git\s+(add|commit)/, tool: 'Bash' },
    ],
    status: WorkflowStatus.DANGEROUS,
    maxGap: 3,
  },
  {
    name: 'mass_delete_push',
    description: 'Mass file deletion then pushing',
    steps: [
      { pattern: /rm\s+-rf?\s+\*|find.*-delete/, tool: 'Bash' },
      { pattern: /git\s+push/, tool: 'Bash' },
    ],
    status: WorkflowStatus.CRITICAL,
    maxGap: 5,
  },
  {
    name: 'clean_and_commit',
    description: 'Git clean then immediate commit',
    steps: [
      { pattern: /git\s+clean\s+-[a-z]*f/, tool: 'Bash' },
      { pattern: /git\s+commit/, tool: 'Bash' },
    ],
    status: WorkflowStatus.DANGEROUS,
    maxGap: 2,
  },
  {
    name: 'truncate_deploy',
    description: 'Table truncation then deployment',
    steps: [
      { pattern: /TRUNCATE\s+TABLE/i, tool: 'Bash' },
      { pattern: /(deploy|push)/, tool: 'Bash' },
    ],
    status: WorkflowStatus.CRITICAL,
    maxGap: 5,
  },
];

/**
 * Workflow Tracker - Detects dangerous multi-step sequences
 */
export class WorkflowTracker extends EventEmitter {
  /**
   * @param {Object} options
   * @param {number} [options.maxHistory] - Max operations to track (Fib(13)=144)
   * @param {number} [options.windowMs] - Time window for workflow detection (5 min)
   */
  constructor(options = {}) {
    super();

    this._maxHistory = options.maxHistory || 144;
    this._windowMs = options.windowMs || 5 * 60 * 1000; // 5 minutes

    // Operation history (sliding window)
    this._history = [];

    // Active workflow warnings
    this._activeWarnings = new Map();

    // Statistics
    this._stats = {
      operationsTracked: 0,
      workflowsDetected: 0,
      workflowsBlocked: 0,
      lastDetection: null,
    };
  }

  /**
   * Record a tool operation
   *
   * @param {string} tool - Tool name (Bash, Write, Edit, etc.)
   * @param {string} command - Command or operation details
   * @param {Object} [metadata] - Additional metadata
   * @returns {Object} Detection result
   */
  recordOperation(tool, command, metadata = {}) {
    const timestamp = Date.now();
    const operation = {
      tool,
      command,
      timestamp,
      metadata,
    };

    // Add to history
    this._history.push(operation);
    this._stats.operationsTracked++;

    // Trim history if needed
    while (this._history.length > this._maxHistory) {
      this._history.shift();
    }

    // Remove stale operations outside window
    const cutoff = timestamp - this._windowMs;
    this._history = this._history.filter(op => op.timestamp >= cutoff);

    // Check for dangerous workflows
    const detections = this._detectWorkflows(operation);

    return {
      recorded: true,
      operationCount: this._history.length,
      detections,
      hasDangerousWorkflow: detections.some(d =>
        d.status === WorkflowStatus.DANGEROUS ||
        d.status === WorkflowStatus.CRITICAL
      ),
    };
  }

  /**
   * Detect dangerous workflows in current history
   * @private
   */
  _detectWorkflows(currentOp) {
    const detections = [];

    for (const workflow of DANGEROUS_WORKFLOWS) {
      const detection = this._matchWorkflow(workflow, currentOp);
      if (detection) {
        detections.push(detection);
        this._stats.workflowsDetected++;
        this._stats.lastDetection = Date.now();

        // Emit event
        this.emit('workflow_detected', detection);

        // Track active warning
        this._activeWarnings.set(workflow.name, {
          ...detection,
          detectedAt: Date.now(),
        });
      }
    }

    return detections;
  }

  /**
   * Match a workflow pattern against history
   * @private
   */
  _matchWorkflow(workflow, currentOp) {
    const { steps, maxGap } = workflow;

    // Current operation must match the LAST step
    const lastStep = steps[steps.length - 1];
    if (!this._matchStep(lastStep, currentOp)) {
      return null;
    }

    // Find previous steps in history (reverse chronological)
    const recentHistory = this._history.slice(-maxGap - 1, -1).reverse();

    // For each previous step (in reverse order), find a match
    let stepIndex = steps.length - 2; // Start from second-to-last
    let matchedOps = [currentOp];
    let gapCount = 0;

    for (const op of recentHistory) {
      if (stepIndex < 0) break;
      if (gapCount > maxGap) break;

      const step = steps[stepIndex];
      if (this._matchStep(step, op)) {
        matchedOps.unshift(op);
        stepIndex--;
        gapCount = 0;
      } else {
        gapCount++;
      }
    }

    // Did we match all steps?
    if (stepIndex >= 0) {
      return null; // Not all steps matched
    }

    // Calculate confidence based on gap and time
    const timeDelta = currentOp.timestamp - matchedOps[0].timestamp;
    const timeConfidence = Math.max(0, 1 - timeDelta / this._windowMs);
    const gapConfidence = 1 - (matchedOps.length - steps.length) / maxGap;
    const confidence = Math.min(PHI_INV, timeConfidence * gapConfidence * PHI_INV);

    return {
      name: workflow.name,
      description: workflow.description,
      status: workflow.status,
      steps: matchedOps.map(op => ({
        tool: op.tool,
        command: op.command.slice(0, 100),
        timestamp: op.timestamp,
      })),
      confidence,
      message: this._buildMessage(workflow, matchedOps),
    };
  }

  /**
   * Match a single step against an operation
   * @private
   */
  _matchStep(step, operation) {
    // Tool match (or wildcard)
    if (step.tool !== '*' && step.tool !== operation.tool) {
      return false;
    }

    // Pattern match
    return step.pattern.test(operation.command);
  }

  /**
   * Build warning message
   * @private
   */
  _buildMessage(workflow, matchedOps) {
    const stepSummary = matchedOps
      .map(op => `${op.tool}:${op.command.slice(0, 50)}`)
      .join(' → ');

    const statusEmoji = {
      [WorkflowStatus.SAFE]: '✅',
      [WorkflowStatus.CAUTION]: '⚠️',
      [WorkflowStatus.DANGEROUS]: '🔴',
      [WorkflowStatus.CRITICAL]: '💀',
    };

    return `${statusEmoji[workflow.status]} Dangerous workflow detected: ${workflow.name}\n` +
           `Description: ${workflow.description}\n` +
           `Sequence: ${stepSummary}`;
  }

  /**
   * Check if a pending operation would complete a dangerous workflow
   *
   * @param {string} tool - Tool that would be executed
   * @param {string} command - Command that would be executed
   * @returns {Object} Preemptive check result
   */
  preemptiveCheck(tool, command) {
    // Create hypothetical operation
    const hypotheticalOp = {
      tool,
      command,
      timestamp: Date.now(),
    };

    const wouldTrigger = [];

    for (const workflow of DANGEROUS_WORKFLOWS) {
      const detection = this._matchWorkflow(workflow, hypotheticalOp);
      if (detection) {
        wouldTrigger.push(detection);
      }
    }

    return {
      wouldTrigger: wouldTrigger.length > 0,
      workflows: wouldTrigger,
      shouldBlock: wouldTrigger.some(w =>
        w.status === WorkflowStatus.CRITICAL
      ),
      shouldWarn: wouldTrigger.some(w =>
        w.status === WorkflowStatus.DANGEROUS
      ),
    };
  }

  /**
   * Get active warnings
   */
  getActiveWarnings() {
    const now = Date.now();
    const active = [];

    for (const [name, warning] of this._activeWarnings) {
      // Expire warnings after window
      if (now - warning.detectedAt > this._windowMs) {
        this._activeWarnings.delete(name);
      } else {
        active.push(warning);
      }
    }

    return active;
  }

  /**
   * Get recent history
   */
  getHistory() {
    return this._history.map(op => ({
      tool: op.tool,
      command: op.command.slice(0, 100),
      timestamp: op.timestamp,
    }));
  }

  /**
   * Get statistics
   */
  getStats() {
    return {
      ...this._stats,
      historySize: this._history.length,
      activeWarnings: this._activeWarnings.size,
    };
  }

  /**
   * Clear history
   */
  clear() {
    this._history = [];
    this._activeWarnings.clear();
    this._stats = {
      operationsTracked: 0,
      workflowsDetected: 0,
      workflowsBlocked: 0,
      lastDetection: null,
    };
  }

  /**
   * Mark a workflow as blocked
   */
  markBlocked(workflowName) {
    this._stats.workflowsBlocked++;
    this.emit('workflow_blocked', { name: workflowName, timestamp: Date.now() });
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create WorkflowTracker singleton
 */
export function getWorkflowTracker(options = {}) {
  if (!_instance) {
    _instance = new WorkflowTracker(options);
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
export function resetWorkflowTracker() {
  if (_instance) {
    _instance.removeAllListeners();
  }
  _instance = null;
}

export default {
  WorkflowTracker,
  WorkflowStatus,
  getWorkflowTracker,
  resetWorkflowTracker,
};
