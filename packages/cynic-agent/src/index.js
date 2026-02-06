/**
 * @cynic/agent - Autonomous Solana AI Agent
 *
 * "Le chien qui pense, juge, et agit" - κυνικός
 *
 * Architecture:
 * ┌─────────────────────────────────────────────────────────────┐
 * │                    CYNIC AGENT LOOP                         │
 * ├─────────────────────────────────────────────────────────────┤
 * │                                                             │
 * │   PERCEIVE ──→ JUDGE ──→ DECIDE ──→ ACT ──→ LEARN          │
 * │      ↑                                          │           │
 * │      └──────────────────────────────────────────┘           │
 * │                                                             │
 * │   Solana    CYNIC 25D   φ-bounded   gasdf      Unified     │
 * │   Events    Judgment    Confidence  Relayer   Signal       │
 * │                                                             │
 * └─────────────────────────────────────────────────────────────┘
 *
 * @module @cynic/agent
 */

'use strict';

import 'dotenv/config';
import { EventEmitter } from 'eventemitter3';
import { PHI_INV, PHI_INV_2, createLogger, globalEventBus, EventType } from '@cynic/core';

import { Perceiver, KNOWN_TOKENS } from './perceiver.js';
import { Decider } from './decider.js';
import { Executor } from './executor.js';
import { Learner } from './learner.js';

const log = createLogger('CynicAgent');

// Lazy-load CollectivePack to avoid hard dependency on @cynic/node
let _collectiveModule = null;
async function loadCollective() {
  if (_collectiveModule !== null) return _collectiveModule;
  try {
    const mod = await import('@cynic/node');
    _collectiveModule = mod;
    return mod;
  } catch (e) {
    log.debug('@cynic/node not available', { error: e.message });
    _collectiveModule = false;
    return null;
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════════════════

export const AGENT_CONFIG = {
  // φ-aligned confidence thresholds
  minConfidenceToAct: PHI_INV_2,      // 38.2% minimum to consider action
  maxConfidence: PHI_INV,              // 61.8% maximum (never exceed)

  // Loop timing (Fibonacci-based)
  tickInterval: 5000,                  // 5s base tick
  cooldownAfterAction: 8000,           // 8s cooldown after action
  maxActionsPerHour: 21,               // Fibonacci limit

  // Risk management
  maxPositionSize: 0.1,                // 10% max of portfolio per trade
  stopLossPercent: 0.08,               // 8% stop loss (φ⁻² ≈ 0.0819)

  // Learning
  feedbackDelayMs: 30000,              // Wait 30s before evaluating outcome
};

// ═══════════════════════════════════════════════════════════════════════════════
// Agent States
// ═══════════════════════════════════════════════════════════════════════════════

export const AgentState = {
  IDLE: 'idle',
  PERCEIVING: 'perceiving',
  JUDGING: 'judging',
  DECIDING: 'deciding',
  ACTING: 'acting',
  LEARNING: 'learning',
  COOLDOWN: 'cooldown',
  ERROR: 'error',
};

// ═══════════════════════════════════════════════════════════════════════════════
// CynicAgent Class
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * CynicAgent - Autonomous Solana AI Agent
 *
 * Runs the perceive→judge→decide→act→learn loop autonomously.
 * All decisions are φ-bounded (max 61.8% confidence).
 */
export class CynicAgent extends EventEmitter {
  /**
   * Create a new CynicAgent
   *
   * @param {Object} options - Configuration options
   * @param {string} options.name - Agent name
   * @param {Object} options.wallet - Wallet configuration
   * @param {Object} options.strategy - Trading strategy parameters
   */
  constructor(options = {}) {
    super();

    this.name = options.name || 'cynic-agent-0';
    this.config = { ...AGENT_CONFIG, ...options.config };

    // State
    this.state = AgentState.IDLE;
    this.isRunning = false;
    this.tickTimer = null;

    // Components
    this.perceiver = new Perceiver(options.perceiver);
    this.decider = new Decider(options.decider);
    this.executor = new Executor(options.executor);
    this.learner = new Learner(options.learner);

    // Collective (wired lazily on start)
    this.pack = null;
    this.guardian = null;

    // Metrics
    this.metrics = {
      startedAt: null,
      tickCount: 0,
      perceptions: 0,
      judgments: 0,
      decisions: 0,
      actions: 0,
      actionsThisHour: 0,
      successfulActions: 0,
      failedActions: 0,
      totalPnL: 0,
      lastActionAt: null,
    };

    // Wire component events
    this._wireEvents();

    log.info(`Agent "${this.name}" created`, { config: this.config });
  }

  /**
   * Wire internal component events
   * @private
   */
  _wireEvents() {
    // Perceiver events
    this.perceiver.on('signal', (signal) => {
      this.metrics.perceptions++;
      this.emit('perception', signal);
    });

    this.perceiver.on('opportunity', (opp) => {
      this.emit('opportunity', opp);
      // Trigger judgment cycle
      if (this.state === AgentState.IDLE) {
        this._processOpportunity(opp);
      }
    });

    // Executor events
    this.executor.on('action_complete', (result) => {
      this.metrics.actions++;
      if (result.success) {
        this.metrics.successfulActions++;
      } else {
        this.metrics.failedActions++;
      }
      this.emit('action_complete', result);

      // Schedule learning
      setTimeout(() => {
        this._evaluateOutcome(result);
      }, this.config.feedbackDelayMs);
    });

    // Learner events - CLOSE THE LOOP: feed adjustments back to Decider
    this.learner.on('lesson', (lesson) => {
      this.emit('lesson', lesson);

      const adjustments = this.learner.getDimensionAdjustments();
      if (Object.keys(adjustments).length > 0) {
        this.decider.applyWeightAdjustments(adjustments);
      }
    });
  }

  /**
   * Start the agent loop
   */
  async start() {
    if (this.isRunning) {
      log.warn('Agent already running');
      return;
    }

    log.info(`Starting agent "${this.name}"...`);

    // Initialize components
    await this.perceiver.start();
    await this.executor.init();

    // Wire CollectivePack (11 Dogs, SharedMemory, AmbientConsensus)
    try {
      const mod = await loadCollective();
      if (mod && mod.getCollectivePackAsync) {
        this.pack = await mod.getCollectivePackAsync();
        log.info('CollectivePack wired', {
          agents: this.pack.agents?.size || 0,
          hasConsensus: !!this.pack.ambientConsensus,
        });

        // Extract Guardian from the collective
        if (this.pack.guardian) {
          this.guardian = this.pack.guardian;
          log.info('Guardian Dog wired for risk blocking');
        }

        // Wire E-Score provider to Executor
        if (mod.calculateCompositeEScore) {
          this.executor._getEScore = mod.calculateCompositeEScore;
          log.debug('Dynamic E-Score wired to Executor');
        }
      }
    } catch (e) {
      log.debug('Collective init skipped', { error: e.message });
    }

    this.isRunning = true;
    this.metrics.startedAt = Date.now();
    this.state = AgentState.PERCEIVING;

    // Start tick loop
    this._tick();
    this.tickTimer = setInterval(() => this._tick(), this.config.tickInterval);

    // Reset hourly action count
    setInterval(() => {
      this.metrics.actionsThisHour = 0;
    }, 60 * 60 * 1000);

    this.emit('started', { name: this.name, timestamp: Date.now() });

    // Announce on globalEventBus
    globalEventBus.emit(EventType.COMPONENT_READY, {
      id: `agent:${this.name}`,
      payload: { component: 'CynicAgent', name: this.name, hasPack: !!this.pack },
    });

    log.info(`Agent "${this.name}" started`, { hasPack: !!this.pack });
  }

  /**
   * Stop the agent loop
   */
  async stop() {
    if (!this.isRunning) return;

    log.info(`Stopping agent "${this.name}"...`);

    this.isRunning = false;

    if (this.tickTimer) {
      clearInterval(this.tickTimer);
      this.tickTimer = null;
    }

    await this.perceiver.stop();

    this.state = AgentState.IDLE;
    this.emit('stopped', { name: this.name, timestamp: Date.now() });
    log.info(`Agent "${this.name}" stopped`);
  }

  /**
   * Main tick - called every tickInterval
   * @private
   */
  async _tick() {
    if (!this.isRunning) return;

    this.metrics.tickCount++;

    try {
      // Check if in cooldown
      if (this.state === AgentState.COOLDOWN) {
        const elapsed = Date.now() - this.metrics.lastActionAt;
        if (elapsed < this.config.cooldownAfterAction) {
          return; // Still in cooldown
        }
        this.state = AgentState.PERCEIVING;
      }

      // Check hourly action limit
      if (this.metrics.actionsThisHour >= this.config.maxActionsPerHour) {
        log.debug('Hourly action limit reached');
        return;
      }

      // Get latest perception
      const perception = await this.perceiver.getLatest();
      if (!perception) return;

      this.emit('tick', {
        tickCount: this.metrics.tickCount,
        state: this.state,
        perception: perception.summary,
      });

    } catch (err) {
      log.error('Tick error', { error: err.message });
      this.state = AgentState.ERROR;
      this.emit('error', err);
    }
  }

  /**
   * Process a detected opportunity
   * @private
   */
  async _processOpportunity(opportunity) {
    try {
      // 0. COLLECTIVE PRE-CHECK: Ask Dogs for consensus before committing resources
      if (this.pack?.ambientConsensus) {
        try {
          const packInput = {
            type: 'trading_opportunity',
            content: `${opportunity.direction} ${opportunity.token}: magnitude=${(opportunity.magnitude * 100).toFixed(1)}%`,
            confidence: opportunity.confidence,
            source: 'cynic-agent',
          };
          const consensus = await this.pack.ambientConsensus.requestVote(packInput);
          if (consensus && consensus.verdict === 'REJECT') {
            log.info('Collective rejected opportunity', {
              token: opportunity.token,
              agreement: consensus.agreement,
            });
            this.state = AgentState.PERCEIVING;
            return;
          }
          // Attach collective confidence to opportunity for downstream use
          opportunity.collectiveConfidence = consensus?.agreement || null;
        } catch (e) {
          log.debug('Collective pre-check skipped', { error: e.message });
        }
      }

      // 1. JUDGE
      this.state = AgentState.JUDGING;
      const judgment = await this.decider.judge(opportunity);
      this.metrics.judgments++;
      this.emit('judgment', judgment);

      // Emit to globalEventBus for cross-system visibility
      globalEventBus.emit(EventType.JUDGMENT_CREATED, {
        id: judgment.id,
        payload: {
          qScore: judgment.qScore,
          verdict: judgment.verdict,
          confidence: judgment.confidence,
          itemType: 'trading_opportunity',
          source: 'cynic-agent',
          token: opportunity.token,
          direction: opportunity.direction,
        },
      });

      // 2. DECIDE
      this.state = AgentState.DECIDING;
      const decision = await this.decider.decide(judgment, {
        maxConfidence: this.config.maxConfidence,
        minConfidence: this.config.minConfidenceToAct,
      });
      this.metrics.decisions++;
      this.emit('decision', decision);

      // 3. GUARDIAN CHECK (between DECIDE and ACT)
      if (decision.action !== 'HOLD' && this.guardian) {
        try {
          const guardianInput = {
            tool_name: 'execute_trade',
            tool_input: JSON.stringify({
              action: decision.action,
              token: decision.token,
              size: decision.size,
              confidence: decision.confidence,
              qScore: decision.qScore,
            }),
          };
          // Use Dog's analyze method if available, fallback to execute
          const analyzeFn = this.guardian.analyze || this.guardian.execute;
          const risk = await analyzeFn.call(this.guardian, guardianInput);

          if (risk.blocked) {
            log.warn('Guardian blocked trade', { reason: risk.message });
            this.emit('guardian_block', { decision, risk });
            this.state = AgentState.PERCEIVING;
            return;
          }

          if (risk.warning) {
            log.info('Guardian warning', { message: risk.message });
            this.emit('guardian_warning', { decision, risk });
          }
        } catch (e) {
          log.debug('Guardian check failed, proceeding', { error: e.message });
        }
      }

      // 4. ACT (only if decision says go)
      if (decision.action !== 'HOLD' && decision.confidence >= this.config.minConfidenceToAct) {
        this.state = AgentState.ACTING;

        // Check action limit
        if (this.metrics.actionsThisHour >= this.config.maxActionsPerHour) {
          log.info('Action limit reached, skipping execution');
          this.state = AgentState.PERCEIVING;
          return;
        }

        const result = await this.executor.execute(decision);
        this.metrics.lastActionAt = Date.now();
        this.metrics.actionsThisHour++;

        // Enter cooldown
        this.state = AgentState.COOLDOWN;

        // Record for learning
        this.learner.recordAction({
          opportunity,
          judgment,
          decision,
          result,
        });

      } else {
        log.debug('Decision: HOLD', { confidence: decision.confidence });
        this.state = AgentState.PERCEIVING;
      }

    } catch (err) {
      log.error('Opportunity processing error', { error: err.message });
      this.state = AgentState.ERROR;
      this.emit('error', err);

      // Recover
      setTimeout(() => {
        this.state = AgentState.PERCEIVING;
      }, 5000);
    }
  }

  /**
   * Evaluate outcome of an action for learning
   * @private
   */
  async _evaluateOutcome(actionResult) {
    this.state = AgentState.LEARNING;

    try {
      const outcome = await this.learner.evaluateOutcome(actionResult);

      if (outcome.pnl !== undefined) {
        this.metrics.totalPnL += outcome.pnl;
      }

      this.emit('outcome', outcome);

    } catch (err) {
      log.error('Outcome evaluation error', { error: err.message });
    }

    this.state = AgentState.PERCEIVING;
  }

  /**
   * Get agent status
   */
  getStatus() {
    return {
      name: this.name,
      state: this.state,
      isRunning: this.isRunning,
      uptime: this.metrics.startedAt ? Date.now() - this.metrics.startedAt : 0,
      metrics: { ...this.metrics },
      config: this.config,
      components: {
        perceiver: this.perceiver.getStatus(),
        decider: this.decider.getStatus(),
        executor: this.executor.getStatus(),
        learner: this.learner.getStatus(),
      },
    };
  }

  /**
   * Get health assessment
   */
  getHealth() {
    const successRate = this.metrics.actions > 0
      ? this.metrics.successfulActions / this.metrics.actions
      : 0;

    let status = 'healthy';
    let score = PHI_INV;

    if (this.state === AgentState.ERROR) {
      status = 'error';
      score = 0.1;
    } else if (successRate < PHI_INV_2) {
      status = 'degraded';
      score = PHI_INV_2;
    }

    return {
      status,
      score,
      state: this.state,
      successRate,
      actionsThisHour: this.metrics.actionsThisHour,
      maxActionsPerHour: this.config.maxActionsPerHour,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Factory
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Create a new CynicAgent with default configuration
 *
 * @param {Object} options - Agent options
 * @returns {CynicAgent}
 */
export function createAgent(options = {}) {
  return new CynicAgent(options);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Exports
// ═══════════════════════════════════════════════════════════════════════════════

export { Perceiver, KNOWN_TOKENS, SignalType } from './perceiver.js';
export { Decider, Action, Verdict, Dimensions } from './decider.js';
export { Executor, ExecutionStatus, SOL_MINT, USDC_MINT } from './executor.js';
export { Learner, OutcomeType } from './learner.js';

export default {
  CynicAgent,
  createAgent,
  AgentState,
  AGENT_CONFIG,
  KNOWN_TOKENS,
};
