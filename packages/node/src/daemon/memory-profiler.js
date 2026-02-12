/**
 * Memory Profiler - Find the leak
 *
 * Tracks memory growth over time and identifies suspects.
 */

'use strict';

import v8 from 'v8';
import fs from 'fs';
import path from 'path';
import os from 'os';
import { createLogger } from '@cynic/core';

const log = createLogger('MemoryProfiler');

/**
 * Take a heap snapshot and save to file
 * @returns {string} Snapshot file path
 */
export function takeHeapSnapshot() {
  const timestamp = Date.now();
  const filename = `heap-${timestamp}.heapsnapshot`;
  const filepath = path.join(os.homedir(), '.cynic', 'daemon', filename);

  log.info(`Taking heap snapshot: ${filepath}`);
  const snapshot = v8.writeHeapSnapshot(filepath);

  return snapshot;
}

/**
 * Get detailed memory usage breakdown
 * @returns {Object} Memory stats
 */
export function getMemoryStats() {
  const usage = process.memoryUsage();
  const heapStats = v8.getHeapStatistics();

  return {
    timestamp: Date.now(),
    rss: Math.round(usage.rss / 1024 / 1024),
    heapTotal: Math.round(usage.heapTotal / 1024 / 1024),
    heapUsed: Math.round(usage.heapUsed / 1024 / 1024),
    external: Math.round(usage.external / 1024 / 1024),
    arrayBuffers: Math.round(usage.arrayBuffers / 1024 / 1024),

    // V8 heap stats
    totalHeapSize: Math.round(heapStats.total_heap_size / 1024 / 1024),
    totalHeapSizeExecutable: Math.round(heapStats.total_heap_size_executable / 1024 / 1024),
    totalPhysicalSize: Math.round(heapStats.total_physical_size / 1024 / 1024),
    totalAvailableSize: Math.round(heapStats.total_available_size / 1024 / 1024),
    usedHeapSize: Math.round(heapStats.used_heap_size / 1024 / 1024),
    heapSizeLimit: Math.round(heapStats.heap_size_limit / 1024 / 1024),
    mallocedMemory: Math.round(heapStats.malloced_memory / 1024 / 1024),
    peakMallocedMemory: Math.round(heapStats.peak_malloced_memory / 1024 / 1024),
    numberOfNativeContexts: heapStats.number_of_native_contexts,
    numberOfDetachedContexts: heapStats.number_of_detached_contexts,
  };
}

/**
 * Track memory growth over time
 */
export class MemoryTracker {
  constructor() {
    this.samples = [];
    this.startTime = Date.now();
  }

  sample() {
    const stats = getMemoryStats();
    this.samples.push(stats);

    if (this.samples.length > 1) {
      const prev = this.samples[this.samples.length - 2];
      const delta = {
        heapUsed: stats.heapUsed - prev.heapUsed,
        rss: stats.rss - prev.rss,
        external: stats.external - prev.external,
        detachedContexts: stats.numberOfDetachedContexts - prev.numberOfDetachedContexts,
      };

      log.info('Memory delta', delta);
    }

    return stats;
  }

  getGrowthRate() {
    if (this.samples.length < 2) return null;

    const first = this.samples[0];
    const last = this.samples[this.samples.length - 1];
    const elapsed = (last.timestamp - first.timestamp) / 1000; // seconds

    return {
      heapUsedPerSecond: (last.heapUsed - first.heapUsed) / elapsed,
      rssPerSecond: (last.rss - first.rss) / elapsed,
      externalPerSecond: (last.external - first.external) / elapsed,
      totalSamples: this.samples.length,
      elapsedSeconds: elapsed,
    };
  }

  getSummary() {
    if (this.samples.length === 0) return null;

    const first = this.samples[0];
    const last = this.samples[this.samples.length - 1];
    const growth = this.getGrowthRate();

    return {
      startTime: new Date(first.timestamp).toISOString(),
      endTime: new Date(last.timestamp).toISOString(),
      elapsedSeconds: Math.round((last.timestamp - first.timestamp) / 1000),

      initial: first,
      final: last,

      delta: {
        heapUsed: last.heapUsed - first.heapUsed,
        rss: last.rss - first.rss,
        external: last.external - first.external,
        detachedContexts: last.numberOfDetachedContexts - first.numberOfDetachedContexts,
      },

      growth,

      // Leak indicators
      indicators: {
        rapidHeapGrowth: growth?.heapUsedPerSecond > 5, // >5MB/s is suspicious
        detachedContextLeak: (last.numberOfDetachedContexts - first.numberOfDetachedContexts) > 10,
        externalMemoryLeak: growth?.externalPerSecond > 1,
      },
    };
  }
}

/**
 * Run a memory profiling session
 * @param {number} duration - Duration in seconds
 * @param {number} interval - Sample interval in seconds
 */
export async function profileMemory(duration = 60, interval = 5) {
  log.info(`Starting memory profile: ${duration}s duration, ${interval}s interval`);

  const tracker = new MemoryTracker();

  // Take initial snapshot
  const snapshot1 = takeHeapSnapshot();
  tracker.sample();

  // Sample at intervals
  const samples = Math.floor(duration / interval);
  for (let i = 0; i < samples; i++) {
    await new Promise(r => setTimeout(r, interval * 1000));
    tracker.sample();
  }

  // Take final snapshot
  const snapshot2 = takeHeapSnapshot();
  tracker.sample();

  const summary = tracker.getSummary();

  log.info('Profile complete', summary);

  return {
    snapshot1,
    snapshot2,
    summary,
  };
}
