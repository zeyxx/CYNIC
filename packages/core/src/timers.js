/**
 * Timer Tracker - Debugging utility for finding leaked timers
 *
 * Only active when NODE_ENV=test. Zero overhead in production.
 * Usage:
 *   import { timerTracker } from '@cynic/core';
 *   timerTracker.track(handle, 'my-timer');
 *   // In test teardown:
 *   timerTracker.report();
 *   timerTracker.clearAll();
 *
 * @module @cynic/core/timers
 */

'use strict';

class TimerTracker {
  #active = new Map();
  #enabled = false;
  #counter = 0;

  enable() { this.#enabled = true; }
  disable() { this.#enabled = false; }

  track(handle, label) {
    if (!this.#enabled) return handle;
    this.#active.set(++this.#counter, { handle, label, at: Date.now() });
    return handle;
  }

  untrack(handle) {
    for (const [id, entry] of this.#active) {
      if (entry.handle === handle) { this.#active.delete(id); break; }
    }
  }

  report() {
    if (this.#active.size === 0) return;
    console.warn(`[TimerTracker] ${this.#active.size} active timers:`);
    for (const [, e] of this.#active) {
      console.warn(`  - ${e.label} (age: ${Date.now() - e.at}ms)`);
    }
  }

  clearAll() {
    for (const e of this.#active.values()) {
      clearInterval(e.handle);
      clearTimeout(e.handle);
    }
    this.#active.clear();
  }

  get size() { return this.#active.size; }
}

export const timerTracker = new TimerTracker();

// Auto-enable in test environment
if (process.env.NODE_ENV === 'test') {
  timerTracker.enable();
}
