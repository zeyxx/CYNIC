/**
 * ModelIntelligence — Learned Model Selection via Thompson Sampling
 *
 * CYNIC's model management was a static if/else. Now it LEARNS.
 *
 * Every "Model X needed for Task Y" is a HYPOTHESIS.
 * Hypotheses must be tested (Popper × φ).
 *
 * Architecture (fractal of CYNIC's own cycle):
 *   PERCEIVE: detect current model + capabilities
 *   JUDGE:    rate outcome quality per model×task
 *   DECIDE:   Thompson Sampling selects model (explore φ⁻³)
 *   ACT:      recommend or execute
 *   LEARN:    outcome → update Thompson arms → falsify
 *   EMERGE:   discover model-task affinity patterns
 *
 * "Le chien ne suppose pas — il renifle" — κυνικός
 *
 * Cell: C6.3 (CYNIC×DECIDE) + C6.5 (CYNIC×LEARN)
 * Persistence: ~/.cynic/models/intelligence-state.json
 * Events: model:recommendation → globalEventBus
 *
 * @module @cynic/node/learning/model-intelligence
 */

'use strict';

import { EventEmitter } from 'events';
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'fs';
import { join } from 'path';
import { homedir } from 'os';
import {
  PHI_INV, PHI_INV_2, PHI_INV_3,
  createLogger, globalEventBus, EventType,
} from '@cynic/core';
import { phiBound } from '@cynic/core';
import { ThompsonSampler } from './thompson-sampler.js';

const log = createLogger('ModelIntelligence');

// =============================================================================
// CONSTANTS
// =============================================================================

/**
 * Model tiers — ordered by capability (and cost)
 */
export const ModelTier = {
  OPUS: 'opus',
  SONNET: 'sonnet',
  HAIKU: 'haiku',
  OLLAMA: 'ollama',
};

/** Ordered tiers: index = capability rank (higher = more capable) */
const TIER_ORDER = [ModelTier.OLLAMA, ModelTier.HAIKU, ModelTier.SONNET, ModelTier.OPUS];
const TIER_RANK = Object.fromEntries(TIER_ORDER.map((t, i) => [t, i]));

/**
 * Task types that model intelligence tracks.
 * Maps to common routing task types.
 */
export const TaskCategory = {
  CODE_REVIEW:    'code_review',     // Code quality, PR review
  CODE_WRITE:     'code_write',      // Writing new code
  ARCHITECTURE:   'architecture',    // Design, planning
  DEBUG:          'debug',           // Debugging, error analysis
  SECURITY:       'security',        // Security review
  KNOWLEDGE:      'knowledge',       // Q&A, documentation
  SIMPLE:         'simple',          // Grep, search, simple ops
  ROUTING:        'routing',         // Internal CYNIC routing
};

/**
 * Initial tool-model affinity (heuristic → learned).
 * Each tool → minimum tier that's likely sufficient.
 */
const TOOL_AFFINITY_PRIORS = {
  // Simple tools — Haiku sufficient
  brain_health: ModelTier.HAIKU,
  brain_search: ModelTier.HAIKU,
  brain_patterns: ModelTier.HAIKU,
  brain_memory_search: ModelTier.HAIKU,
  brain_topology: ModelTier.HAIKU,
  brain_cost: ModelTier.HAIKU,
  brain_metrics: ModelTier.HAIKU,
  brain_collective_status: ModelTier.HAIKU,

  // Moderate tools — Sonnet preferred
  brain_cynic_judge: ModelTier.SONNET,
  brain_accounting: ModelTier.SONNET,
  brain_distribution: ModelTier.SONNET,
  brain_ecosystem: ModelTier.SONNET,
  brain_learning: ModelTier.SONNET,
  brain_codebase: ModelTier.SONNET,
  brain_docs: ModelTier.SONNET,

  // Complex tools — Opus preferred
  brain_orchestrate: ModelTier.OPUS,
  brain_cynic_refine: ModelTier.OPUS,
  brain_integrator: ModelTier.OPUS,
};

/**
 * Map routing task types → TaskCategory
 */
const TASK_TYPE_MAP = {
  PreToolUse: TaskCategory.SECURITY,
  PostToolUse: TaskCategory.CODE_REVIEW,
  code: TaskCategory.CODE_WRITE,
  commit: TaskCategory.CODE_REVIEW,
  review: TaskCategory.CODE_REVIEW,
  design: TaskCategory.ARCHITECTURE,
  architecture: TaskCategory.ARCHITECTURE,
  security: TaskCategory.SECURITY,
  deployment: TaskCategory.SECURITY,
  debug: TaskCategory.DEBUG,
  knowledge: TaskCategory.KNOWLEDGE,
  question: TaskCategory.KNOWLEDGE,
  simple: TaskCategory.SIMPLE,
  search: TaskCategory.SIMPLE,
  default: TaskCategory.ROUTING,
};

/**
 * Falsification state for a hypothesis
 */
const HypothesisState = {
  UNTESTED: 'untested',
  TESTING: 'testing',
  SUPPORTED: 'supported',    // Survived tests (but never certain)
  FALSIFIED: 'falsified',    // Cheaper model works
  INCONCLUSIVE: 'inconclusive',
};

// =============================================================================
// PERSISTENCE
// =============================================================================

const PERSIST_DIR = join(homedir(), '.cynic', 'models');
const PERSIST_PATH = join(PERSIST_DIR, 'intelligence-state.json');

// =============================================================================
// MODEL INTELLIGENCE
// =============================================================================

export class ModelIntelligence extends EventEmitter {
  /**
   * @param {Object} [options]
   * @param {string} [options.persistPath] - Override persistence path (for tests)
   */
  constructor(options = {}) {
    super();

    this._persistPath = options.persistPath || PERSIST_PATH;

    // Thompson Sampler per task category
    // Arms are model tiers: opus, sonnet, haiku, ollama
    this._samplers = new Map();
    for (const category of Object.values(TaskCategory)) {
      const sampler = new ThompsonSampler();
      // Initialize all model arms with informed priors
      for (const tier of TIER_ORDER) {
        // Prior: higher-tier models start with slight advantage
        // opus: Beta(3,1), sonnet: Beta(2,1), haiku: Beta(1.5,1), ollama: Beta(1,1)
        const rank = TIER_RANK[tier];
        sampler.initArm(tier, 1 + rank * 0.7, 1);
      }
      this._samplers.set(category, sampler);
    }

    // Tool-model affinity (learned over time)
    this._toolAffinity = { ...TOOL_AFFINITY_PRIORS };

    // Falsification hypotheses
    // Key: `${taskCategory}:${currentTier}` → hypothesis state
    this._hypotheses = new Map();

    // Active falsification experiments
    // Key: experimentId → { taskCategory, expensiveTier, cheapTier, startTime, outcomes }
    this._experiments = new Map();
    this._experimentCounter = 0;

    // Outcome history (rolling window for quality assessment)
    this._outcomes = [];
    this._maxOutcomes = 200; // ~F(12) sessions worth

    // Stats
    this._stats = {
      selectionsTotal: 0,
      selectionsByModel: { opus: 0, sonnet: 0, haiku: 0, ollama: 0 },
      falsificationsProposed: 0,
      falsificationsCompleted: 0,
      falsificationsSucceeded: 0, // cheaper model worked
      outcomesRecorded: 0,
      downgrades: 0,
    };

    // Load persisted state
    this._load();
  }

  // ===========================================================================
  // PERCEIVE: Model detection
  // ===========================================================================

  /**
   * Detect which model is currently active from environment.
   *
   * @returns {{ tier: string, label: string, source: string }}
   */
  detectCurrentModel() {
    // Check environment variables
    const modelEnv = process.env.CLAUDE_MODEL || process.env.ANTHROPIC_MODEL || '';
    const modelLower = modelEnv.toLowerCase();

    if (modelLower.includes('opus')) return { tier: ModelTier.OPUS, label: modelEnv, source: 'env' };
    if (modelLower.includes('haiku')) return { tier: ModelTier.HAIKU, label: modelEnv, source: 'env' };
    if (modelLower.includes('sonnet')) return { tier: ModelTier.SONNET, label: modelEnv, source: 'env' };
    if (modelLower.includes('ollama') || modelLower.includes('llama')) return { tier: ModelTier.OLLAMA, label: modelEnv, source: 'env' };

    // Check process args (Claude Code passes model info)
    const argsStr = process.argv.join(' ').toLowerCase();
    if (argsStr.includes('opus')) return { tier: ModelTier.OPUS, label: 'opus (argv)', source: 'argv' };
    if (argsStr.includes('haiku')) return { tier: ModelTier.HAIKU, label: 'haiku (argv)', source: 'argv' };

    // Default assumption: Sonnet (most common)
    return { tier: ModelTier.SONNET, label: 'unknown (assuming Sonnet)', source: 'default' };
  }

  // ===========================================================================
  // DECIDE: Model selection via Thompson Sampling
  // ===========================================================================

  /**
   * Select optimal model for a task.
   *
   * Uses Thompson Sampling with per-task-type beliefs.
   * φ⁻³ (23.6%) of the time, proposes falsification experiments.
   *
   * @param {string} taskType - Routing task type (PreToolUse, code, etc.)
   * @param {Object} [context] - Additional context
   * @param {string} [context.tool] - Tool being used (for affinity)
   * @param {string} [context.budgetLevel] - Current budget level
   * @param {number} [context.velocity] - Current burn velocity
   * @returns {{ model: string, reason: string, confidence: number, experiment: Object|null }}
   */
  selectModel(taskType, context = {}) {
    const category = this._categorize(taskType);
    const sampler = this._samplers.get(category);

    this._stats.selectionsTotal++;

    // Budget override: exhausted → always Haiku
    if (context.budgetLevel === 'exhausted') {
      this._stats.selectionsByModel.haiku++;
      return {
        model: ModelTier.HAIKU,
        reason: 'budget exhausted — forced minimum',
        confidence: phiBound(0.55),
        experiment: null,
        category,
      };
    }

    // Budget pressure: critical → cap at Sonnet
    const maxTier = context.budgetLevel === 'critical'
      ? ModelTier.SONNET
      : ModelTier.OPUS;

    // Tool affinity: if tool is known, use as floor
    const toolFloor = context.tool ? (this._toolAffinity[context.tool] || null) : null;

    // Available models (respect budget cap)
    const maxRank = TIER_RANK[maxTier];
    const available = TIER_ORDER.filter(t => TIER_RANK[t] <= maxRank);

    // Falsification check: φ⁻³ of the time, propose downgrade test
    let experiment = null;
    if (Math.random() < PHI_INV_3) {
      experiment = this._proposeFalsification(category, sampler);
      if (experiment) {
        // Use the cheaper model for this experiment
        const cheaperModel = experiment.cheapTier;
        if (available.includes(cheaperModel)) {
          this._stats.selectionsByModel[cheaperModel]++;
          return {
            model: cheaperModel,
            reason: `falsification test: can ${cheaperModel} replace ${experiment.expensiveTier} for ${category}?`,
            confidence: phiBound(0.4), // Low confidence — this is an experiment
            experiment,
            category,
          };
        }
      }
    }

    // Thompson Sampling: select from available models
    const selected = sampler.selectArm(available);
    const model = selected || ModelTier.SONNET; // fallback

    // Apply tool affinity floor
    if (toolFloor && TIER_RANK[model] < TIER_RANK[toolFloor]) {
      const upgraded = toolFloor;
      this._stats.selectionsByModel[upgraded]++;
      return {
        model: upgraded,
        reason: `tool affinity: ${context.tool} needs ${upgraded}+`,
        confidence: phiBound(0.5),
        experiment: null,
        category,
      };
    }

    this._stats.selectionsByModel[model]++;

    // Compute confidence from sampler uncertainty
    const expectedValue = sampler.getExpectedValue(model);
    const uncertainty = sampler.getUncertainty(model);
    const confidence = phiBound(expectedValue * (1 - uncertainty * 4));

    return {
      model,
      reason: `Thompson selected (EV=${(expectedValue * 100).toFixed(1)}%, uncertainty=${(uncertainty * 100).toFixed(1)}%)`,
      confidence,
      experiment: null,
      category,
    };
  }

  // ===========================================================================
  // LEARN: Record outcomes
  // ===========================================================================

  /**
   * Record the outcome of a model selection.
   *
   * Quality is binary for Thompson: success = task completed well.
   * Signal comes from tool:completed events (error = failure, no error = success).
   *
   * @param {Object} outcome
   * @param {string} outcome.taskType - Task type (or routing category)
   * @param {string} outcome.model - Model that was used
   * @param {boolean} outcome.success - Whether the task succeeded
   * @param {number} [outcome.qualityScore] - Optional quality score (0-1)
   * @param {string} [outcome.tool] - Tool that was used
   * @param {number} [outcome.durationMs] - How long it took
   * @param {string} [outcome.experimentId] - If part of a falsification experiment
   */
  recordOutcome(outcome) {
    const { taskType, model, success, qualityScore, tool, durationMs, experimentId } = outcome;

    if (!model || !taskType) return;

    const category = this._categorize(taskType);
    const sampler = this._samplers.get(category);

    if (sampler) {
      // Nuanced success: if quality score provided, use threshold
      // Quality >= φ⁻¹ (61.8%) = success, below = failure
      const isSuccess = qualityScore != null
        ? qualityScore >= PHI_INV
        : success !== false;

      sampler.update(model, isSuccess);
    }

    // Update tool affinity (learn from outcomes)
    if (tool && success !== false) {
      this._updateToolAffinity(tool, model, success !== false);
    }

    // Process falsification experiment
    if (experimentId && this._experiments.has(experimentId)) {
      this._recordExperimentOutcome(experimentId, outcome);
    }

    // Store in history
    this._outcomes.push({
      timestamp: Date.now(),
      category,
      model,
      success: success !== false,
      qualityScore: qualityScore || null,
      tool: tool || null,
      durationMs: durationMs || 0,
    });

    // Rolling window
    while (this._outcomes.length > this._maxOutcomes) {
      this._outcomes.shift();
    }

    this._stats.outcomesRecorded++;

    // Persist periodically (every 13 outcomes = Fibonacci)
    if (this._stats.outcomesRecorded % 13 === 0) {
      this.persist();
    }
  }

  // ===========================================================================
  // FALSIFY: Popperian downgrade testing
  // ===========================================================================

  /**
   * Propose a falsification experiment.
   *
   * Tests whether the current "best" model is actually needed
   * by scheduling a trial with a cheaper model.
   *
   * @param {string} category - Task category
   * @param {ThompsonSampler} sampler - Sampler for this category
   * @returns {Object|null} Experiment descriptor or null
   * @private
   */
  _proposeFalsification(category, sampler) {
    // Find current best model for this category
    const bestModel = sampler.selectArm(TIER_ORDER);
    const bestRank = TIER_RANK[bestModel];

    // Can't downgrade below Haiku (Ollama is separate)
    if (bestRank <= TIER_RANK[ModelTier.HAIKU]) return null;

    // Propose one tier down
    const cheaperTier = TIER_ORDER[bestRank - 1];
    if (!cheaperTier) return null;

    // Check if we already have an active experiment for this pair
    const hypKey = `${category}:${bestModel}→${cheaperTier}`;
    const activeExperiment = Array.from(this._experiments.values())
      .find(e => e.hypKey === hypKey && e.state === 'active');
    if (activeExperiment) return null;

    // Create experiment
    const experimentId = `exp_${++this._experimentCounter}_${Date.now()}`;
    const experiment = {
      id: experimentId,
      hypKey,
      taskCategory: category,
      expensiveTier: bestModel,
      cheapTier: cheaperTier,
      startTime: Date.now(),
      state: 'active',
      outcomes: { success: 0, failure: 0, total: 0 },
      // Need φ (5) trials to draw conclusion
      requiredTrials: 5,
    };

    this._experiments.set(experimentId, experiment);
    this._stats.falsificationsProposed++;

    log.info('Falsification proposed', {
      category,
      hypothesis: `${cheaperTier} can replace ${bestModel} for ${category}`,
      experimentId,
    });

    return experiment;
  }

  /**
   * Record outcome for a falsification experiment.
   * @private
   */
  _recordExperimentOutcome(experimentId, outcome) {
    const experiment = this._experiments.get(experimentId);
    if (!experiment || experiment.state !== 'active') return;

    experiment.outcomes.total++;
    if (outcome.success !== false) {
      experiment.outcomes.success++;
    } else {
      experiment.outcomes.failure++;
    }

    // Check if we have enough trials
    if (experiment.outcomes.total >= experiment.requiredTrials) {
      const successRate = experiment.outcomes.success / experiment.outcomes.total;

      if (successRate >= PHI_INV) {
        // Falsified: cheaper model works! Downgrade.
        experiment.state = 'falsified';
        this._stats.falsificationsSucceeded++;
        this._stats.downgrades++;

        // Update hypothesis
        this._hypotheses.set(experiment.hypKey, {
          state: HypothesisState.FALSIFIED,
          successRate,
          trials: experiment.outcomes.total,
          timestamp: Date.now(),
        });

        log.info('Hypothesis FALSIFIED — downgrade validated', {
          category: experiment.taskCategory,
          from: experiment.expensiveTier,
          to: experiment.cheapTier,
          successRate: (successRate * 100).toFixed(1) + '%',
        });

        // Boost the cheaper model's Thompson arm
        const sampler = this._samplers.get(experiment.taskCategory);
        if (sampler) {
          // Give extra weight to cheaper model (3 bonus successes)
          for (let i = 0; i < 3; i++) {
            sampler.update(experiment.cheapTier, true);
          }
        }

      } else if (successRate < PHI_INV_2) {
        // Supported: cheaper model doesn't work well enough
        experiment.state = 'supported';

        this._hypotheses.set(experiment.hypKey, {
          state: HypothesisState.SUPPORTED,
          successRate,
          trials: experiment.outcomes.total,
          timestamp: Date.now(),
        });

        log.info('Hypothesis SUPPORTED — expensive model justified', {
          category: experiment.taskCategory,
          model: experiment.expensiveTier,
          cheaperSuccessRate: (successRate * 100).toFixed(1) + '%',
        });

      } else {
        // Inconclusive
        experiment.state = 'inconclusive';

        this._hypotheses.set(experiment.hypKey, {
          state: HypothesisState.INCONCLUSIVE,
          successRate,
          trials: experiment.outcomes.total,
          timestamp: Date.now(),
        });
      }

      this._stats.falsificationsCompleted++;
      this.persist();
    }
  }

  // ===========================================================================
  // TOOL AFFINITY (learned)
  // ===========================================================================

  /**
   * Update tool-model affinity based on outcome.
   * If a tool succeeds with a cheaper model, lower the floor.
   * @private
   */
  _updateToolAffinity(tool, model, success) {
    const currentFloor = this._toolAffinity[tool];
    if (!currentFloor) {
      // New tool — set affinity to whatever worked
      this._toolAffinity[tool] = model;
      return;
    }

    if (success && TIER_RANK[model] < TIER_RANK[currentFloor]) {
      // Cheaper model succeeded — lower the floor
      this._toolAffinity[tool] = model;
      log.debug('Tool affinity lowered', { tool, from: currentFloor, to: model });
    }
  }

  // ===========================================================================
  // EMERGE: Affinity matrix + patterns
  // ===========================================================================

  /**
   * Get the model-task affinity matrix.
   * Shows learned preferences across all task categories.
   *
   * @returns {Object} Matrix of category → model → score
   */
  getAffinityMatrix() {
    const matrix = {};

    for (const [category, sampler] of this._samplers) {
      matrix[category] = {};
      for (const tier of TIER_ORDER) {
        matrix[category][tier] = {
          expectedValue: sampler.getExpectedValue(tier),
          uncertainty: sampler.getUncertainty(tier),
          pulls: sampler.arms.get(tier)?.pulls || 0,
        };
      }
    }

    return matrix;
  }

  /**
   * Get tool-model affinity map.
   * @returns {Object} tool → minimum model tier
   */
  getToolAffinity() {
    return { ...this._toolAffinity };
  }

  /**
   * Get falsification status across all hypotheses.
   * @returns {Object[]} Hypothesis summaries
   */
  getHypotheses() {
    const hypotheses = [];
    for (const [key, hyp] of this._hypotheses) {
      hypotheses.push({
        hypothesis: key,
        state: hyp.state,
        successRate: hyp.successRate,
        trials: hyp.trials,
        age: Date.now() - hyp.timestamp,
      });
    }
    return hypotheses;
  }

  // ===========================================================================
  // STATS
  // ===========================================================================

  /**
   * Get comprehensive stats.
   * @returns {Object}
   */
  getStats() {
    return {
      ...this._stats,
      samplerMaturity: this._getSamplerMaturity(),
      activeExperiments: Array.from(this._experiments.values())
        .filter(e => e.state === 'active').length,
      hypothesesTested: this._hypotheses.size,
      toolsTracked: Object.keys(this._toolAffinity).length,
      outcomesInWindow: this._outcomes.length,
    };
  }

  /**
   * Get maturity signal for ContextCompressor.
   * @returns {{ maturity: number, converged: boolean }}
   */
  getMaturitySignal() {
    const maturity = this._getSamplerMaturity();
    return {
      maturity: phiBound(maturity),
      converged: maturity > PHI_INV_2,
    };
  }

  /**
   * Average maturity across all samplers.
   * @private
   */
  _getSamplerMaturity() {
    let totalMaturity = 0;
    let count = 0;
    for (const sampler of this._samplers.values()) {
      const signal = sampler.getMaturitySignal();
      totalMaturity += signal.maturity;
      count++;
    }
    return count > 0 ? totalMaturity / count : 0;
  }

  // ===========================================================================
  // HELPERS
  // ===========================================================================

  /**
   * Categorize a routing task type.
   * @param {string} taskType
   * @returns {string} TaskCategory value
   * @private
   */
  _categorize(taskType) {
    return TASK_TYPE_MAP[taskType] || TaskCategory.ROUTING;
  }

  // ===========================================================================
  // PERSISTENCE
  // ===========================================================================

  /**
   * Persist state to disk.
   */
  persist() {
    try {
      const dir = join(this._persistPath, '..');
      if (!existsSync(dir)) {
        mkdirSync(dir, { recursive: true });
      }

      const state = {
        version: 1,
        timestamp: Date.now(),
        samplers: {},
        toolAffinity: this._toolAffinity,
        hypotheses: Object.fromEntries(this._hypotheses),
        stats: this._stats,
        experimentCounter: this._experimentCounter,
      };

      // Serialize Thompson Sampler state
      for (const [category, sampler] of this._samplers) {
        const arms = {};
        for (const [armId, data] of sampler.arms) {
          arms[armId] = { alpha: data.alpha, beta: data.beta, pulls: data.pulls };
        }
        state.samplers[category] = {
          arms,
          totalPulls: sampler.totalPulls,
        };
      }

      writeFileSync(this._persistPath, JSON.stringify(state, null, 2));
      log.debug('ModelIntelligence persisted', {
        path: this._persistPath,
        outcomes: this._stats.outcomesRecorded,
      });
    } catch (err) {
      log.debug('ModelIntelligence persist failed', { error: err.message });
    }
  }

  /**
   * Load persisted state from disk.
   * @private
   */
  _load() {
    try {
      if (!existsSync(this._persistPath)) return;

      const raw = readFileSync(this._persistPath, 'utf8');
      const state = JSON.parse(raw);

      if (state.version !== 1) return;

      // Restore Thompson Samplers
      if (state.samplers) {
        for (const [category, saved] of Object.entries(state.samplers)) {
          const sampler = this._samplers.get(category);
          if (!sampler) continue;

          for (const [armId, data] of Object.entries(saved.arms || {})) {
            // Direct set — initArm skips existing arms (already created in constructor)
            const arm = sampler.arms.get(armId);
            if (arm) {
              arm.alpha = data.alpha || 1;
              arm.beta = data.beta || 1;
              arm.pulls = data.pulls || 0;
            } else {
              sampler.initArm(armId, data.alpha || 1, data.beta || 1);
              const newArm = sampler.arms.get(armId);
              if (newArm) newArm.pulls = data.pulls || 0;
            }
          }
          sampler.totalPulls = saved.totalPulls || 0;
        }
      }

      // Restore tool affinity (merge with priors — learned overrides)
      if (state.toolAffinity) {
        this._toolAffinity = { ...TOOL_AFFINITY_PRIORS, ...state.toolAffinity };
      }

      // Restore hypotheses
      if (state.hypotheses) {
        this._hypotheses = new Map(Object.entries(state.hypotheses));
      }

      // Restore stats (merge to keep new fields)
      if (state.stats) {
        this._stats = { ...this._stats, ...state.stats };
      }

      if (state.experimentCounter) {
        this._experimentCounter = state.experimentCounter;
      }

      log.debug('ModelIntelligence loaded', {
        categories: this._samplers.size,
        hypotheses: this._hypotheses.size,
        totalOutcomes: this._stats.outcomesRecorded,
      });
    } catch (err) {
      log.debug('ModelIntelligence load failed (starting fresh)', { error: err.message });
    }
  }
}

// =============================================================================
// SINGLETON
// =============================================================================

let _instance = null;

/**
 * Get the ModelIntelligence singleton.
 * @param {Object} [options] - Options for first creation
 * @returns {ModelIntelligence}
 */
export function getModelIntelligence(options = {}) {
  if (!_instance) {
    _instance = new ModelIntelligence(options);
  }
  return _instance;
}

/**
 * Reset singleton (for tests).
 */
export function resetModelIntelligence() {
  if (_instance) {
    _instance.removeAllListeners();
    _instance = null;
  }
}

export default ModelIntelligence;
