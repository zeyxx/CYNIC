/**
 * Domain Wiring Factory — Auto-generate event listeners from config
 *
 * Replaces hand-written per-domain wiring with a config-driven factory.
 * Each domain follows the standard φ-cycle:
 *
 *   PERCEIVE → JUDGE → DECIDE → ACT → LEARN → ACCOUNT → EMERGE
 *   (C*.1)     (C*.2)   (C*.3)  (C*.4) (C*.5)  (C*.6)    (C*.7)
 *
 * Config defines: event names, module references, sampling, safety gates.
 * Factory wires the standard pipeline + domain-specific overrides.
 *
 * Usage:
 *   const wiring = createDomainWiring(config);
 *   wiring.wire({ judge, decider, actor, ... });
 *   wiring.getStats();
 *   wiring.stop();
 *
 * "Le chien connecte les fils — φ fait le reste" — κυνικός
 *
 * @module @cynic/node/services/create-domain-wiring
 */

'use strict';

import { createLogger, globalEventBus, EventType } from '@cynic/core';

const log = createLogger('DomainWiring');

// Fibonacci intervals (milliseconds)
const FIB_INTERVALS = {
  F7: 13 * 60 * 1000,  // 13 minutes
  F8: 21 * 60 * 1000,  // 21 minutes
  F9: 34 * 60 * 1000,  // 34 minutes
  F10: 55 * 60 * 1000, // 55 minutes
  F11: 89 * 60 * 1000, // 89 minutes
};

/**
 * Create a domain wiring instance from config.
 *
 * @param {Object} config - Domain wiring configuration
 * @param {string} config.name - Domain name (e.g., 'solana', 'cosmos', 'code')
 * @param {string} config.cell - Base cell reference (e.g., 'C2')
 * @param {Object[]} [config.perceptionEvents] - Perception event definitions
 * @param {string} config.perceptionEvents[].event - Event name to subscribe to
 * @param {string} [config.perceptionEvents[].handler] - Handler method on judge ('judge' or custom)
 * @param {number} [config.perceptionEvents[].sampling] - Sample 1 in N events (default: 1 = all)
 * @param {string} [config.verdictFilter] - Minimum verdict to trigger decision ('GROWL'|'BARK' default)
 * @param {string} [config.actorSafetyEnv] - Env var for actor live mode (e.g., 'SOLANA_ACTOR_LIVE')
 * @param {string} [config.emergenceInterval] - Fibonacci interval key (e.g., 'F8')
 * @param {string} [config.judgeInterval] - Fibonacci interval for periodic judge assessment
 * @param {Function} [config.onPerception] - Custom perception handler (overrides default)
 * @param {Function} [config.onJudgment] - Custom judgment→decision handler (overrides default)
 * @param {Function} [config.onDecision] - Custom decision→action handler (overrides default)
 * @param {Function} [config.onAction] - Custom action→accounting handler (overrides default)
 * @returns {Object} Wiring instance with wire(), stop(), getStats()
 */
export function createDomainWiring(config) {
  const {
    name,
    cell,
    perceptionEvents = [],
    verdictFilter = ['GROWL', 'BARK'],
    actorSafetyEnv = null,
    emergenceInterval = 'F8',
    judgeInterval = null,
    onPerception = null,
    onJudgment = null,
    onDecision = null,
    onAction = null,
  } = config;

  const Name = capitalize(name);
  const _unsubscribers = [];
  const _intervals = [];
  let _wired = false;
  let _modules = {};
  let _persistence = null;

  const _stats = {
    judgments: 0,
    decisions: 0,
    actions: 0,
    learnings: 0,
    accountingOps: 0,
    emergencePatterns: 0,
    perceptionEvents: 0,
    persistenceOps: 0,
    persistenceErrors: 0,
  };

  // Sampling counters (per perception event)
  const _samplingCounters = {};

  return {
    /**
     * Wire domain event listeners.
     *
     * @param {Object} modules - Domain modules to wire
     * @param {Object} [modules.judge] - Domain judge instance
     * @param {Object} [modules.decider] - Domain decider instance
     * @param {Object} [modules.actor] - Domain actor instance
     * @param {Object} [modules.learner] - Domain learner instance
     * @param {Object} [modules.accountant] - Domain accountant instance
     * @param {Object} [modules.emergence] - Domain emergence instance
     * @param {Object} [modules.persistence] - Persistence manager (for unified_signals)
     * @param {string} [modules.sessionId] - Session ID
     */
    wire(modules = {}) {
      if (_wired) {
        log.debug(`${Name} wiring already active — skipping`);
        return;
      }

      _modules = modules;
      _persistence = modules.persistence || null;

      const { judge, decider, actor, learner, accountant, emergence } = modules;

      if (!judge && perceptionEvents.length > 0) {
        log.debug(`${Name} wiring skipped (no judge for perception events)`);
        return;
      }

      _wired = true;
      log.debug(`Wiring ${Name} event listeners`, { events: perceptionEvents.length });

      // ─── STAGE 1: PERCEPTION → JUDGMENT ─────────────────────────
      for (const pe of perceptionEvents) {
        const sampling = pe.sampling || 1;
        _samplingCounters[pe.event] = 0;

        const unsub = globalEventBus.subscribe(pe.event, (event) => {
          try {
            // Fibonacci sampling
            _samplingCounters[pe.event]++;
            if (sampling > 1 && _samplingCounters[pe.event] % sampling !== 0) return;

            _stats.perceptionEvents++;
            const data = event.payload || event;

            // Custom or default perception handler
            if (onPerception) {
              onPerception(pe.event, data, { judge, emergence, stats: _stats, emit: emitJudgment });
              return;
            }

            // Default: call judge method
            const handlerName = pe.handler || 'judge';
            const judgeFn = typeof judge[handlerName] === 'function'
              ? judge[handlerName].bind(judge)
              : typeof judge.judge === 'function'
                ? judge.judge.bind(judge)
                : null;

            if (!judgeFn) return;

            const judgment = judgeFn(data);
            _stats.judgments++;

            emitJudgment(judgment, data, pe.event);

            // Feed emergence
            if (emergence && typeof emergence.recordActivity === 'function') {
              emergence.recordActivity(data);
            }
          } catch (err) {
            log.debug(`${Name} perception handler error`, { event: pe.event, error: err.message });
          }
        });
        _unsubscribers.push(unsub);
      }

      // ─── STAGE 2: JUDGMENT → DECISION ──────────────────────────
      if (decider) {
        const unsub = globalEventBus.subscribe(`${name}:judgment`, (event) => {
          try {
            if (onJudgment) {
              onJudgment(event, { decider, stats: _stats, emit: emitDecision });
              return;
            }

            const payload = event.payload || event;
            const judgment = payload.judgment || payload;
            const verdict = judgment.verdict || judgment.score?.verdict;

            // Filter by verdict severity
            if (verdictFilter.length > 0 && !verdictFilter.includes(verdict)) return;

            const decision = typeof decider.decide === 'function'
              ? decider.decide(judgment, { source: 'auto' })
              : null;

            if (!decision) return;

            _stats.decisions++;
            emitDecision(decision, judgment);
          } catch (err) {
            log.debug(`${Name} judgment→decision error`, { error: err.message });
          }
        });
        _unsubscribers.push(unsub);
      }

      // ─── STAGE 3: DECISION → ACTION + LEARNING ────────────────
      if (actor || learner) {
        const unsub = globalEventBus.subscribe(`${name}:decision`, (event) => {
          try {
            if (onDecision) {
              onDecision(event, { actor, learner, stats: _stats, emit: emitAction });
              return;
            }

            const payload = event.payload || event;
            const decision = payload.decision || payload;
            if (!decision) return;

            const isLive = actorSafetyEnv
              ? process.env[actorSafetyEnv] === 'true'
              : true;

            // Execute action
            if (actor && isLive) {
              const executeFn = typeof actor.execute === 'function'
                ? actor.execute.bind(actor)
                : typeof actor.act === 'function'
                  ? actor.act.bind(actor)
                  : null;

              if (executeFn) {
                const result = executeFn(decision.type || decision.action, decision.params || decision);
                if (result && typeof result.catch === 'function') {
                  result.catch(err => log.debug(`${Name}Actor.execute failed`, { error: err.message }));
                }
              }
            }

            _stats.actions++;
            emitAction(decision, isLive);

            // Record outcome for learning
            if (learner && typeof learner.recordOutcome === 'function') {
              learner.recordOutcome({
                type: decision.type || decision.action || 'unknown',
                executed: isLive,
                decision,
                timestamp: Date.now(),
              });
              _stats.learnings++;
            }
          } catch (err) {
            log.debug(`${Name} decision→action error`, { error: err.message });
          }
        });
        _unsubscribers.push(unsub);
      }

      // ─── STAGE 4: ACTION → ACCOUNTING ──────────────────────────
      if (accountant) {
        const unsub = globalEventBus.subscribe(`${name}:action`, (event) => {
          try {
            if (onAction) {
              onAction(event, { accountant, stats: _stats });
              return;
            }

            const payload = event.payload || event;
            const recordFn = typeof accountant.recordTransaction === 'function'
              ? accountant.recordTransaction.bind(accountant)
              : typeof accountant.record === 'function'
                ? accountant.record.bind(accountant)
                : null;

            if (recordFn) {
              recordFn({
                type: `${name}_action`,
                ...payload,
                timestamp: Date.now(),
              });
              _stats.accountingOps++;
            }
          } catch (err) {
            log.debug(`${Name} action→accountant error`, { error: err.message });
          }
        });
        _unsubscribers.push(unsub);
      }

      // ─── STAGE 5: EMERGENCE (Fibonacci intervals) ─────────────
      if (emergence && typeof emergence.analyze === 'function') {
        const interval = FIB_INTERVALS[emergenceInterval] || FIB_INTERVALS.F8;
        const timer = setInterval(() => {
          try {
            const patterns = emergence.analyze();
            if (patterns && patterns.length > 0) {
              for (const pattern of patterns) {
                globalEventBus.publish(EventType.PATTERN_DETECTED, {
                  source: `${Name}Emergence`,
                  key: pattern.type || pattern.key || 'unknown',
                  significance: pattern.significance || 'medium',
                  category: name,
                  ...pattern,
                }, { source: 'domain-wiring' });
              }
              _stats.emergencePatterns += patterns.length;
            }
          } catch (err) {
            log.debug(`${Name}Emergence.analyze failed`, { error: err.message });
          }
        }, interval);
        _intervals.push(timer);
      }

      // ─── STAGE 6: PERIODIC JUDGE ASSESSMENT ────────────────────
      if (judge && judgeInterval) {
        const interval = FIB_INTERVALS[judgeInterval] || FIB_INTERVALS.F8;
        const assessFn = typeof judge.assessPeriodic === 'function'
          ? judge.assessPeriodic.bind(judge)
          : typeof judge.assess === 'function'
            ? judge.assess.bind(judge)
            : null;

        if (assessFn) {
          const timer = setInterval(() => {
            try {
              const result = assessFn();
              if (result) {
                emitJudgment(result, { source: 'periodic' }, 'periodic');
              }
            } catch (err) {
              log.debug(`${Name}Judge periodic assessment failed`, { error: err.message });
            }
          }, interval);
          _intervals.push(timer);
        }
      }

      // ─── STAGE 7: PERSISTENCE (all pipeline events) ───────────
      if (_persistence && typeof _persistence.query === 'function') {
        for (const stage of ['judgment', 'decision', 'action']) {
          const unsub = globalEventBus.subscribe(`${name}:${stage}`, (event) => {
            persistEvent(`${name}:${stage}`, event);
          });
          _unsubscribers.push(unsub);
        }
      }
    },

    /**
     * Stop all listeners and intervals.
     */
    stop() {
      for (const unsub of _unsubscribers) {
        if (typeof unsub === 'function') unsub();
      }
      for (const timer of _intervals) {
        clearInterval(timer);
      }
      _unsubscribers.length = 0;
      _intervals.length = 0;
      _wired = false;
    },

    /**
     * Get wiring stats.
     */
    getStats() {
      return { domain: name, cell, wired: _wired, ..._stats };
    },

    /**
     * Check if wired.
     */
    isWired() {
      return _wired;
    },

    /**
     * Get domain name.
     */
    getName() {
      return name;
    },
  };

  // ═══════════════════════════════════════════════════════════════
  // INTERNAL HELPERS
  // ═══════════════════════════════════════════════════════════════

  function emitJudgment(judgment, sourceData, eventSource) {
    globalEventBus.publish(`${name}:judgment`, {
      type: eventSource,
      judgment,
      ...sourceData,
    }, { source: 'domain-wiring' });
  }

  function emitDecision(decision, judgment) {
    globalEventBus.publish(`${name}:decision`, {
      decision,
      judgment,
    }, { source: 'domain-wiring' });
  }

  function emitAction(decision, executed) {
    globalEventBus.publish(`${name}:action`, {
      source: `${Name}Actor`,
      decision: { type: decision.type || decision.action, executed },
    }, { source: 'domain-wiring' });
  }

  function persistEvent(eventName, event) {
    const payload = event.payload || event;
    const id = `${name.slice(0, 3)}_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

    _persistence.query(`
      INSERT INTO unified_signals (id, source, session_id, input, outcome, metadata)
      VALUES ($1, $2, $3, $4, $5, $6)
    `, [
      id,
      eventName,
      _modules.sessionId || null,
      JSON.stringify(payload),
      JSON.stringify({ recorded: true }),
      JSON.stringify({ cell, domain: name, timestamp: Date.now() }),
    ]).then(() => {
      _stats.persistenceOps++;
    }).catch(err => {
      _stats.persistenceErrors++;
      log.debug(`${Name} persistence failed`, { event: eventName, error: err.message });
    });
  }
}

// ═════════════════════════════════════════════════════════════════
// AGGREGATE WIRING MANAGER
// ═════════════════════════════════════════════════════════════════

/**
 * Create a wiring manager that handles multiple domains.
 *
 * @param {Object[]} configs - Array of domain wiring configs
 * @returns {Object} Manager with wireAll(), stopAll(), getStats()
 */
export function createWiringManager(configs) {
  const _wirings = configs.map(c => createDomainWiring(c));

  return {
    /**
     * Wire all domains.
     *
     * @param {Object} allModules - Map of domain → modules
     */
    wireAll(allModules) {
      for (const wiring of _wirings) {
        const domainModules = allModules[wiring.getName()] || {};
        wiring.wire(domainModules);
      }
    },

    /**
     * Wire a single domain by name.
     */
    wireDomain(domainName, modules) {
      const wiring = _wirings.find(w => w.getName() === domainName);
      if (wiring) wiring.wire(modules);
    },

    /**
     * Stop all wirings.
     */
    stopAll() {
      for (const wiring of _wirings) {
        wiring.stop();
      }
    },

    /**
     * Get stats for all domains.
     */
    getStats() {
      return _wirings.map(w => w.getStats());
    },

    /**
     * Get wiring by domain name.
     */
    getWiring(domainName) {
      return _wirings.find(w => w.getName() === domainName) || null;
    },

    /**
     * Get all wiring instances.
     */
    getAll() {
      return [..._wirings];
    },
  };
}

// ═════════════════════════════════════════════════════════════════
// HELPERS
// ═════════════════════════════════════════════════════════════════

function capitalize(s) {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

export { FIB_INTERVALS };
