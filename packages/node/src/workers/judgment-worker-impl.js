/**
 * CYNIC Judgment Worker Implementation
 *
 * Worker thread that scores dimensions in parallel.
 * Runs in isolated V8 context for TRUE CPU parallelization.
 *
 * "φ flows in parallel streams" - κυνικός
 *
 * @module @cynic/node/workers/judgment-worker-impl
 */

'use strict';

import { parentPort } from 'worker_threads';
import { scoreDimension } from '../judge/scorers/index.js';
import { createLogger } from '@cynic/core';

const log = createLogger('JudgmentWorker');

if (!parentPort) {
  throw new Error('This module must be run as a Worker thread');
}

log.info('Worker thread initialized');

/**
 * Handle incoming tasks from main thread
 */
parentPort.on('message', async ({ type, taskId, dimension, item, context }) => {
  try {
    if (type === 'score_dimension') {
      // Score the dimension using real scorer
      const score = scoreDimension(dimension.name, item, context);

      // Send result back to main thread
      parentPort.postMessage({
        taskId,
        result: {
          dimName: dimension.name,
          axiom: dimension.axiom,
          score,
          scorer: 'worker',
        },
        error: null,
      });

    } else {
      throw new Error(`Unknown task type: ${type}`);
    }

  } catch (err) {
    log.error('Worker task failed', {
      taskId,
      error: err.message,
      stack: err.stack,
    });

    // Send error back to main thread
    parentPort.postMessage({
      taskId,
      result: null,
      error: err.message,
    });
  }
});

// Graceful shutdown
parentPort.on('close', () => {
  log.info('Worker shutting down');
});

log.debug('Worker ready to receive tasks');
