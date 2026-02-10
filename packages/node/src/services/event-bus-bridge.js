/**
 * EventBusBridge — Three nervous systems, one bridge
 *
 * Connects CYNIC's 3 disjoint event buses:
 *   globalEventBus (core) ←→ getEventBus() (automation) ←→ AgentEventBus (dogs)
 *
 * Problem: Events on one bus are invisible to subscribers on other buses.
 *   - Dog patterns/decisions stay trapped on AgentEventBus
 *   - Learning milestones stay on automation bus
 *   - Judgments stay on core bus
 *
 * Solution: Selective forwarding with loop prevention.
 *
 * Existing manual bridges (coexist — this bridge adds NEW forwarding only):
 *   - AmbientConsensus._publishSignal() → DogSignals → automation + core
 *   - AmbientConsensus → CONSENSUS_COMPLETED → core
 *   - CollectivePack.handleHook() → hooks → core + automation
 *
 * NEW forwarding via this bridge:
 *   Agent → Core: patterns, anomalies, decisions, guidance, vulnerabilities
 *   Automation → Core: learning milestones
 *   Core → Automation: judgments (for tracking)
 *
 * "Three nervous systems. One spinal cord." — κυνικός
 *
 * @module @cynic/node/services/event-bus-bridge
 */

'use strict';

import {
  globalEventBus,
  EventType as CoreEventType,
  createLogger,
} from '@cynic/core';

import { getEventBus, EventType as AutomationEventType } from './event-bus.js';
import { AgentEvent } from '../agents/events.js';

const log = createLogger('EventBusBridge');

/**
 * Metadata tag to prevent infinite forwarding loops.
 * Events with this tag are NEVER re-forwarded.
 */
const BRIDGED_TAG = '_bridged';

/**
 * Bridge agent ID for registering on the AgentEventBus.
 */
const BRIDGE_AGENT_ID = 'bridge';

/**
 * Forwarding rules: Agent → Core
 *
 * These events originate on AgentEventBus and should be visible on globalEventBus.
 * Key: AgentEvent type string → Value: CoreEventType string
 *
 * NOTE: DogSignals + CONSENSUS_COMPLETED are already manually bridged
 *       via AmbientConsensus._publishSignal() — NOT duplicated here.
 */
const AGENT_TO_CORE = {
  [AgentEvent.PATTERN_DETECTED]: CoreEventType.PATTERN_DETECTED,
  [AgentEvent.ANOMALY_DETECTED]: CoreEventType.ANOMALY_DETECTED,
  [AgentEvent.CYNIC_DECISION]: 'cynic:decision',
  [AgentEvent.CYNIC_GUIDANCE]: 'cynic:guidance',
  [AgentEvent.CYNIC_OVERRIDE]: 'cynic:override',
  [AgentEvent.VULNERABILITY_DETECTED]: 'vulnerability:detected',
  [AgentEvent.REALITY_DRIFT_DETECTED]: 'reality:drift',
  [AgentEvent.DEPLOY_COMPLETED]: 'deploy:completed',
  [AgentEvent.DEPLOY_FAILED]: 'deploy:failed',
};

/**
 * Forwarding rules: Automation → Core
 *
 * These events originate on getEventBus() and should be visible on globalEventBus.
 */
const AUTOMATION_TO_CORE = {
  [AutomationEventType.LEARNING_CYCLE_COMPLETE]: 'learning:cycle:complete',
};

/**
 * Forwarding rules: Core → Automation
 *
 * These events originate on globalEventBus and should be visible on getEventBus().
 */
const CORE_TO_AUTOMATION = {
  [CoreEventType.JUDGMENT_CREATED]: AutomationEventType.JUDGMENT_CREATED,
};

/**
 * EventBusBridge
 *
 * Connects three event buses with selective, loop-safe forwarding.
 * Singleton — one bridge per process.
 */
class EventBusBridge {
  constructor() {
    /** @type {import('../agents/event-bus.js').AgentEventBus | null} */
    this._agentBus = null;
    this._running = false;
    this._unsubscribers = [];
    this._stats = {
      agentToCore: 0,
      automationToCore: 0,
      coreToAutomation: 0,
      loopsPrevented: 0,
      errors: 0,
      startedAt: null,
    };
  }

  /**
   * Start the bridge.
   *
   * @param {Object} options
   * @param {import('../agents/event-bus.js').AgentEventBus} [options.agentBus] - The CollectivePack's AgentEventBus
   */
  start(options = {}) {
    if (this._running) {
      log.debug('Bridge already running');
      return;
    }

    this._agentBus = options.agentBus || null;
    this._stats.startedAt = Date.now();

    // ─── Agent → Core ────────────────────────────────────────────────────
    if (this._agentBus) {
      this._wireAgentToCore();
    } else {
      log.debug('No AgentEventBus provided — agent→core bridge skipped');
    }

    // ─── Automation → Core ───────────────────────────────────────────────
    this._wireAutomationToCore();

    // ─── Core → Automation ───────────────────────────────────────────────
    this._wireCoreToAutomation();

    this._running = true;
    log.info('EventBusBridge started', {
      agentBus: !!this._agentBus,
      rules: {
        agentToCore: Object.keys(AGENT_TO_CORE).length,
        automationToCore: Object.keys(AUTOMATION_TO_CORE).length,
        coreToAutomation: Object.keys(CORE_TO_AUTOMATION).length,
      },
    });
  }

  /**
   * Wire Agent → Core forwarding.
   *
   * Registers as 'bridge' agent on the AgentEventBus,
   * subscribes to forwarded event types, and publishes on globalEventBus.
   * @private
   */
  _wireAgentToCore() {
    const agentBus = this._agentBus;

    // Register bridge agent (required by AgentEventBus.subscribe)
    if (!agentBus.isAgentRegistered(BRIDGE_AGENT_ID)) {
      agentBus.registerAgent(BRIDGE_AGENT_ID);
    }

    // Subscribe to each event type we want to forward
    for (const [agentType, coreType] of Object.entries(AGENT_TO_CORE)) {
      try {
        const subId = agentBus.subscribe(agentType, BRIDGE_AGENT_ID, (event) => {
          // Loop prevention: skip if already bridged
          if (event.metadata?.[BRIDGED_TAG]) {
            this._stats.loopsPrevented++;
            return;
          }

          try {
            globalEventBus.publish(coreType, {
              ...event.payload,
              _agentSource: event.source,
              _agentPriority: event.priority,
            }, {
              source: `bridge:${event.source}`,
              correlationId: event.correlationId,
              metadata: { [BRIDGED_TAG]: true, originalBus: 'agent', originalType: agentType },
            });
            this._stats.agentToCore++;
          } catch (err) {
            this._stats.errors++;
            log.error('Agent→Core forward failed', { agentType, coreType, error: err.message });
          }
        });

        // Store subscription ID for cleanup
        this._unsubscribers.push(() => agentBus.unsubscribe(subId));
      } catch (err) {
        // Max listeners or registration error — log and continue
        log.warn('Agent→Core subscription failed', { agentType, error: err.message });
      }
    }
  }

  /**
   * Wire Automation → Core forwarding.
   * @private
   */
  _wireAutomationToCore() {
    const automationBus = getEventBus();

    for (const [autoType, coreType] of Object.entries(AUTOMATION_TO_CORE)) {
      const unsub = automationBus.subscribe(autoType, (event) => {
        // Loop prevention
        if (event.meta?.[BRIDGED_TAG] || event.data?.[BRIDGED_TAG]) {
          this._stats.loopsPrevented++;
          return;
        }

        try {
          globalEventBus.publish(coreType, {
            ...event.data,
          }, {
            source: `bridge:${event.meta?.source || 'automation'}`,
            metadata: { [BRIDGED_TAG]: true, originalBus: 'automation', originalType: autoType },
          });
          this._stats.automationToCore++;
        } catch (err) {
          this._stats.errors++;
          log.error('Automation→Core forward failed', { autoType, coreType, error: err.message });
        }
      });

      this._unsubscribers.push(unsub);
    }
  }

  /**
   * Wire Core → Automation forwarding.
   * @private
   */
  _wireCoreToAutomation() {
    const automationBus = getEventBus();

    for (const [coreType, autoType] of Object.entries(CORE_TO_AUTOMATION)) {
      const unsub = globalEventBus.subscribe(coreType, (event) => {
        // Loop prevention
        if (event.metadata?.[BRIDGED_TAG]) {
          this._stats.loopsPrevented++;
          return;
        }

        try {
          automationBus.publish(autoType, {
            ...event.payload,
          }, {
            source: `bridge:${event.source || 'core'}`,
            [BRIDGED_TAG]: true,
          });
          this._stats.coreToAutomation++;
        } catch (err) {
          this._stats.errors++;
          log.error('Core→Automation forward failed', { coreType, autoType, error: err.message });
        }
      });

      this._unsubscribers.push(unsub);
    }
  }

  /**
   * Late-bind the AgentEventBus (when CollectivePack initializes after bridge).
   *
   * @param {import('../agents/event-bus.js').AgentEventBus} agentBus
   */
  setAgentBus(agentBus) {
    if (this._agentBus) {
      log.warn('AgentEventBus already set — skipping');
      return;
    }

    this._agentBus = agentBus;

    if (this._running) {
      this._wireAgentToCore();
      log.info('Agent→Core bridge late-bound');
    }
  }

  /**
   * Get bridge statistics.
   */
  getStats() {
    return {
      running: this._running,
      uptime: this._stats.startedAt ? Date.now() - this._stats.startedAt : 0,
      forwarded: {
        agentToCore: this._stats.agentToCore,
        automationToCore: this._stats.automationToCore,
        coreToAutomation: this._stats.coreToAutomation,
        total: this._stats.agentToCore + this._stats.automationToCore + this._stats.coreToAutomation,
      },
      loopsPrevented: this._stats.loopsPrevented,
      errors: this._stats.errors,
      rules: {
        agentToCore: Object.keys(AGENT_TO_CORE),
        automationToCore: Object.keys(AUTOMATION_TO_CORE),
        coreToAutomation: Object.keys(CORE_TO_AUTOMATION),
      },
      buses: {
        core: { type: 'CYNICEventBus', status: 'always available' },
        automation: { type: 'EventBus', status: 'always available' },
        agent: { type: 'AgentEventBus', status: this._agentBus ? 'connected' : 'not connected' },
      },
    };
  }

  /**
   * Stop the bridge and clean up all subscriptions.
   */
  stop() {
    for (const unsub of this._unsubscribers) {
      try {
        unsub();
      } catch (_) {
        // Ignore cleanup errors
      }
    }
    this._unsubscribers = [];

    // Unregister bridge agent from AgentEventBus
    if (this._agentBus?.isAgentRegistered?.(BRIDGE_AGENT_ID)) {
      try {
        this._agentBus.unregisterAgent(BRIDGE_AGENT_ID);
      } catch (_) {
        // Ignore
      }
    }

    this._agentBus = null;
    this._running = false;
    log.info('EventBusBridge stopped', { stats: this._stats });
  }

  /**
   * Reset for testing — stops and clears stats.
   */
  _resetForTesting() {
    this.stop();
    this._stats = {
      agentToCore: 0,
      automationToCore: 0,
      coreToAutomation: 0,
      loopsPrevented: 0,
      errors: 0,
      startedAt: null,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

export const eventBusBridge = new EventBusBridge();

export {
  BRIDGED_TAG,
  BRIDGE_AGENT_ID,
  AGENT_TO_CORE,
  AUTOMATION_TO_CORE,
  CORE_TO_AUTOMATION,
};
