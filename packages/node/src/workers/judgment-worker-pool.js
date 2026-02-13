/**
 * CYNIC Judgment Worker Pool
 *
 * TRUE CPU parallelization for dimension scoring via worker threads.
 * Replaces Promise.all() over sync functions with actual parallel execution.
 *
 * Pool size = CPU cores × φ (golden ratio utilization ~61.8%)
 *
 * "φ flows in parallel streams" - κυνικός
 *
 * @module @cynic/node/workers/judgment-worker-pool
 */

'use strict';

import { Worker } from 'worker_threads';
import { EventEmitter } from 'events';
import { createLogger, PHI_INV } from '@cynic/core';
import { cpus } from 'os';
import path from 'path';
import { fileURLToPath } from 'url';

const log = createLogger('JudgmentWorkerPool');

// Get __dirname equivalent in ESM
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

/**
 * Worker Pool for parallel dimension scoring
 *
 * Architecture:
 * - N workers (φ × CPU count)
 * - Round-robin task distribution
 * - Automatic retry on worker failure
 * - Graceful shutdown with pending task completion
 *
 * @example
 * const pool = new JudgmentWorkerPool();
 * const scores = await pool.scoreChunk(dimensions, item, context);
 * await pool.close();
 */
export class JudgmentWorkerPool extends EventEmitter {
  constructor(options = {}) {
    super();

    // Pool size = CPU cores × φ (golden ratio utilization)
    const cpuCount = cpus().length;
    this.poolSize = options.poolSize || Math.ceil(cpuCount * PHI_INV);

    // Worker script path
    this.workerPath = options.workerPath ||
      path.resolve(__dirname, 'judgment-worker-impl.js');

    this.workers = [];
    this.taskQueue = [];
    this.activeTasks = new Map();
    this.nextWorkerId = 0; // Round-robin counter

    this.stats = {
      tasksProcessed: 0,
      tasksQueued: 0,
      tasksFailed: 0,
      workersIdle: 0,
      avgProcessingTimeMs: 0,
      totalProcessingTimeMs: 0,
    };

    this._initialized = false;
    this._closing = false;

    log.info('Worker pool created', {
      poolSize: this.poolSize,
      cpuCount,
      utilizationRatio: PHI_INV,
    });
  }

  /**
   * Initialize worker threads
   * @private
   */
  async _initWorkers() {
    if (this._initialized) return;

    log.info('Initializing worker pool', { workerPath: this.workerPath });

    for (let i = 0; i < this.poolSize; i++) {
      await this._spawnWorker(i);
    }

    this._initialized = true;
    this.stats.workersIdle = this.poolSize;

    log.info('Worker pool initialized', {
      poolSize: this.poolSize,
      workersReady: this.workers.length,
    });
  }

  /**
   * Spawn a worker thread
   * @private
   * @param {number} id - Worker ID
   */
  async _spawnWorker(id) {
    try {
      const worker = new Worker(this.workerPath);

      worker.on('message', ({ taskId, result, error }) => {
        this._handleWorkerResponse(worker, taskId, result, error);
      });

      worker.on('error', (err) => {
        log.error(`Worker ${id} error`, { workerId: id, error: err.message });
        this.emit('worker:error', { workerId: id, error: err });
      });

      worker.on('exit', (code) => {
        if (code !== 0 && !this._closing) {
          log.warn(`Worker ${id} exited unexpectedly`, { code });
          this.emit('worker:exit', { workerId: id, code });

          // Remove dead worker
          const idx = this.workers.findIndex(w => w.id === id);
          if (idx >= 0) {
            this.workers.splice(idx, 1);
            this.stats.workersIdle = this.workers.filter(w => !w.busy).length;

            // Respawn worker if not closing
            if (!this._closing) {
              this._spawnWorker(id).catch(err =>
                log.error('Worker respawn failed', { workerId: id, error: err.message })
              );
            }
          }
        }
      });

      this.workers.push({
        worker,
        id,
        busy: false,
        tasksCompleted: 0,
      });

      log.debug(`Worker ${id} spawned`, { workerId: id });

    } catch (err) {
      log.error(`Failed to spawn worker ${id}`, { error: err.message });
      throw err;
    }
  }

  /**
   * Score a chunk of dimensions in parallel
   *
   * Distributes dimension scoring across worker threads for TRUE parallelization.
   * Each worker scores its assigned dimensions independently.
   *
   * @param {Array} dimensions - Dimension configs to score [{name, config}]
   * @param {Object} item - Item to score
   * @param {Object} context - Scoring context
   * @returns {Promise<Object>} Dimension scores {dimName: score}
   */
  async scoreChunk(dimensions, item, context = {}) {
    // Lazy init on first use
    if (!this._initialized) {
      await this._initWorkers();
    }

    if (this._closing) {
      throw new Error('Worker pool is closing');
    }

    // Create task for each dimension
    const tasks = dimensions.map(dim => ({
      taskId: `task_${Date.now()}_${Math.random().toString(36).slice(2, 11)}`,
      type: 'score_dimension',
      dimension: dim,
      item,
      context,
    }));

    // Submit tasks in parallel
    const promises = tasks.map(task => this._submitTask(task));

    // Wait for all scores
    const results = await Promise.all(promises);

    // Convert array to object
    const scores = {};
    for (const result of results) {
      if (result && result.dimName) {
        scores[result.dimName] = result.score;
      }
    }

    return scores;
  }

  /**
   * Submit a task to the pool
   * @private
   * @param {Object} task - Task descriptor
   * @returns {Promise<Object>} Task result
   */
  _submitTask(task) {
    return new Promise((resolve, reject) => {
      this.taskQueue.push({
        ...task,
        resolve,
        reject,
        createdAt: Date.now(),
        attempts: 0,
        maxAttempts: 3,
      });

      this.stats.tasksQueued++;
      this._processQueue();
    });
  }

  /**
   * Process task queue (round-robin distribution)
   * @private
   */
  _processQueue() {
    if (this.taskQueue.length === 0) return;
    if (this._closing) return;

    // Find idle workers
    const idleWorkers = this.workers.filter(w => !w.busy);
    if (idleWorkers.length === 0) return;

    // Distribute tasks to idle workers (round-robin)
    for (const worker of idleWorkers) {
      if (this.taskQueue.length === 0) break;

      const task = this.taskQueue.shift();
      this.stats.tasksQueued--;

      this._assignTaskToWorker(worker, task);
    }
  }

  /**
   * Assign task to a specific worker
   * @private
   * @param {Object} workerInfo - Worker info object
   * @param {Object} task - Task to assign
   */
  _assignTaskToWorker(workerInfo, task) {
    workerInfo.busy = true;
    this.stats.workersIdle = this.workers.filter(w => !w.busy).length;

    this.activeTasks.set(task.taskId, {
      ...task,
      worker: workerInfo,
      startedAt: Date.now(),
    });

    // Send task to worker
    workerInfo.worker.postMessage({
      type: task.type,
      taskId: task.taskId,
      dimension: task.dimension,
      item: task.item,
      context: task.context,
    });

    log.debug('Task assigned to worker', {
      taskId: task.taskId,
      workerId: workerInfo.id,
      queueLength: this.taskQueue.length,
    });
  }

  /**
   * Handle worker response
   * @private
   * @param {Worker} worker - Worker instance
   * @param {string} taskId - Task ID
   * @param {*} result - Task result
   * @param {string|null} error - Error message if failed
   */
  _handleWorkerResponse(worker, taskId, result, error) {
    const task = this.activeTasks.get(taskId);
    if (!task) {
      log.warn('Received response for unknown task', { taskId });
      return;
    }

    // Free worker
    task.worker.busy = false;
    task.worker.tasksCompleted++;
    this.stats.workersIdle = this.workers.filter(w => !w.busy).length;

    // Update stats
    const processingTime = Date.now() - task.startedAt;
    this.stats.tasksProcessed++;
    this.stats.totalProcessingTimeMs += processingTime;
    this.stats.avgProcessingTimeMs =
      this.stats.totalProcessingTimeMs / this.stats.tasksProcessed;

    // Handle result
    if (error) {
      log.warn('Task failed', { taskId, error, attempt: task.attempts + 1 });

      // Retry logic
      task.attempts++;
      if (task.attempts < task.maxAttempts) {
        log.info('Retrying task', { taskId, attempt: task.attempts });
        this.activeTasks.delete(taskId);
        this.taskQueue.unshift(task); // Retry at front of queue
        this._processQueue();
      } else {
        log.error('Task failed permanently', { taskId, attempts: task.attempts });
        this.stats.tasksFailed++;
        task.reject(new Error(error));
        this.activeTasks.delete(taskId);
      }
    } else {
      log.debug('Task completed', {
        taskId,
        workerId: task.worker.id,
        processingTimeMs: processingTime,
      });
      task.resolve(result);
      this.activeTasks.delete(taskId);
    }

    // Process next queued task
    this._processQueue();
  }

  /**
   * Get pool statistics
   * @returns {Object} Pool stats
   */
  getStats() {
    return {
      ...this.stats,
      poolSize: this.poolSize,
      workersAlive: this.workers.length,
      queueLength: this.taskQueue.length,
      activeTasksCount: this.activeTasks.size,
      workerStats: this.workers.map(w => ({
        id: w.id,
        busy: w.busy,
        tasksCompleted: w.tasksCompleted,
      })),
    };
  }

  /**
   * Close worker pool gracefully
   *
   * Waits for active tasks to complete, then terminates all workers.
   *
   * @param {number} [timeoutMs=5000] - Max wait time for active tasks
   * @returns {Promise<void>}
   */
  async close(timeoutMs = 5000) {
    if (this._closing) return;

    this._closing = true;
    log.info('Closing worker pool', {
      activeTasksCount: this.activeTasks.size,
      queuedTasksCount: this.taskQueue.length,
    });

    // Reject all queued tasks
    for (const task of this.taskQueue) {
      task.reject(new Error('Worker pool shutting down'));
    }
    this.taskQueue = [];

    // Wait for active tasks to complete (with timeout)
    const startWait = Date.now();
    while (this.activeTasks.size > 0 && (Date.now() - startWait < timeoutMs)) {
      await new Promise(resolve => setTimeout(resolve, 100));
    }

    // Force-reject any remaining active tasks
    for (const [taskId, task] of this.activeTasks) {
      log.warn('Force-closing task', { taskId });
      task.reject(new Error('Worker pool force-closed'));
    }
    this.activeTasks.clear();

    // Terminate all workers
    for (const { worker, id } of this.workers) {
      try {
        await worker.terminate();
        log.debug(`Worker ${id} terminated`);
      } catch (err) {
        log.warn(`Worker ${id} termination error`, { error: err.message });
      }
    }

    this.workers = [];
    this._initialized = false;

    log.info('Worker pool closed', {
      tasksProcessed: this.stats.tasksProcessed,
      tasksFailed: this.stats.tasksFailed,
    });
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON PATTERN (optional - for global pool reuse)
// ═══════════════════════════════════════════════════════════════════════════

let _globalPool = null;

/**
 * Get global singleton worker pool
 * @param {Object} [options] - Pool options (only used on first call)
 * @returns {JudgmentWorkerPool}
 */
export function getWorkerPool(options) {
  if (!_globalPool) {
    _globalPool = new JudgmentWorkerPool(options);
  }
  return _globalPool;
}

/**
 * Reset global worker pool (for testing)
 * @returns {Promise<void>}
 */
export async function resetWorkerPool() {
  if (_globalPool) {
    await _globalPool.close();
    _globalPool = null;
  }
}

export default { JudgmentWorkerPool, getWorkerPool, resetWorkerPool };
