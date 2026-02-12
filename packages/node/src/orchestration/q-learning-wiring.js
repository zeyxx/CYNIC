/**
 * Q-Learning Wiring Gap 3 — Load Q-table from DB and record updates
 *
 * Ensures Q-Learning weights are:
 * 1. Loaded from PostgreSQL on daemon startup
 * 2. Applied live during routing decisions (via KabbalisticRouter)
 * 3. Recorded to DB for G1.3 metric tracking
 *
 * "Le chien se souvient qui appeler" - CYNIC learns who to call
 *
 * @module @cynic/node/orchestration/q-learning-wiring
 */

'use strict';

import { createLogger, globalEventBus, EventType } from '@cynic/core';
import { getQLearningServiceAsync } from './learning-service.js';
import { getPostgresClient } from '@cynic/persistence';

const log = createLogger('QLearningWiring');

let _wired = false;
let _weightUpdateListener = null;

/**
 * Wire Q-Learning system at daemon boot.
 *
 * Actions:
 * 1. Load Q-table from PostgreSQL (qlearning_state table)
 * 2. Subscribe to Q-weight updates and record to DB for G1.3 metric
 * 3. Ensure weights are available for KabbalisticRouter routing decisions
 *
 * @returns {Promise<{ learningService: Object, loaded: boolean }>}
 */
export async function wireQLearning() {
  if (_wired) {
    log.debug('Q-Learning already wired — skipping');
    return { learningService: await getQLearningServiceAsync(), loaded: true };
  }

  try {
    const db = getPostgresClient();

    // 1. Initialize LearningService with DB persistence
    const learningService = await getQLearningServiceAsync({
      persistence: db,
      serviceId: 'default',
    });

    // Load Q-table from DB (this is the critical missing step!)
    log.info('Loading Q-table from PostgreSQL...');
    const loaded = await learningService.load();

    if (loaded) {
      const stats = learningService.getStats();
      log.info('Q-table loaded from DB', {
        states: stats.qTableStats?.states || 0,
        updates: stats.qTableStats?.updates || 0,
        explorationRate: stats.explorationRate,
      });
    } else {
      log.info('No existing Q-table found — starting fresh');
    }

    // 2. Wire Q-weight update listener for G1.3 metric tracking
    _weightUpdateListener = async (event) => {
      try {
        const { state, action, qValue, delta, serviceId } = event.payload || {};
        if (!action || qValue === undefined) return;

        // Record Q-update to learning_events table for G1.3 metric
        await db.query(`
          INSERT INTO learning_events (
            loop_type, event_type, action_taken, q_value, reward, metadata
          ) VALUES ($1, $2, $3, $4, $5, $6)
        `, [
          'q-learning',
          'weight-update',
          action, // dog name
          qValue, // new Q-value
          delta, // reward signal
          JSON.stringify({
            state: state || [],
            serviceId: serviceId || 'default',
            timestamp: Date.now(),
          }),
        ]);

        log.debug('Q-weight update recorded', { action, qValue, delta });
      } catch (err) {
        // Non-blocking — DB write failure shouldn't break learning
        log.debug('Failed to record Q-weight update', { error: err.message });
      }
    };

    // Subscribe to Q-Learning weight updates
    globalEventBus.on(EventType.QLEARNING_WEIGHT_UPDATE || 'qlearning:weight-update', _weightUpdateListener);

    _wired = true;

    log.info('Q-Learning wired — weights loaded, updates tracked for G1.3');

    return { learningService, loaded };
  } catch (err) {
    log.warn('Q-Learning wiring failed — routing will use default weights', {
      error: err.message,
    });
    return { learningService: null, loaded: false };
  }
}

/**
 * Cleanup Q-Learning wiring.
 */
export function cleanupQLearning() {
  if (!_wired) return;

  if (_weightUpdateListener) {
    globalEventBus.removeListener(
      EventType.QLEARNING_WEIGHT_UPDATE || 'qlearning:weight-update',
      _weightUpdateListener
    );
    _weightUpdateListener = null;
  }

  _wired = false;
  log.info('Q-Learning wiring cleaned up');
}

/**
 * Check if Q-Learning is wired.
 * @returns {boolean}
 */
export function isQLearningWired() {
  return _wired;
}

export default {
  wireQLearning,
  cleanupQLearning,
  isQLearningWired,
};
