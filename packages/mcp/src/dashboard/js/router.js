/**
 * CYNIC Dashboard - Router
 * Mode switching logic
 */

export class Router {
  constructor() {
    this.currentMode = 'operator';
    this.modes = ['operator', 'dev', 'arch', 'live', 'knowledge', 'memory', 'autonomy', 'singularity'];
    this.listeners = new Map();
    this.history = [];
  }

  /**
   * Initialize router
   */
  init() {
    // Check URL hash for initial mode
    const hash = window.location.hash.slice(1);
    if (this.modes.includes(hash)) {
      this.currentMode = hash;
    }

    // Listen for hash changes
    window.addEventListener('hashchange', () => {
      const newMode = window.location.hash.slice(1);
      if (this.modes.includes(newMode) && newMode !== this.currentMode) {
        this.navigate(newMode, false);
      }
    });

    // Initial render
    this._updateViews();

    return this;
  }

  /**
   * Navigate to mode
   */
  navigate(mode, updateHash = true) {
    if (!this.modes.includes(mode)) {
      console.warn(`Invalid mode: ${mode}`);
      return;
    }

    if (mode === this.currentMode) {
      return;
    }

    const previousMode = this.currentMode;
    this.currentMode = mode;
    this.history.push({ mode, timestamp: Date.now() });

    // Update URL hash
    if (updateHash) {
      window.location.hash = mode;
    }

    // Update views
    this._updateViews();

    // Emit event
    this._emit('modeChange', {
      mode,
      previousMode,
    });
  }

  /**
   * Update view visibility
   */
  _updateViews() {
    // Update views
    this.modes.forEach(mode => {
      const view = document.getElementById(`view-${mode}`);
      if (view) {
        view.classList.toggle('active', mode === this.currentMode);
      }
    });

    // Update mode buttons
    document.querySelectorAll('.mode-btn').forEach(btn => {
      btn.classList.toggle('active', btn.dataset.mode === this.currentMode);
    });
  }

  /**
   * Get current mode
   */
  getMode() {
    return this.currentMode;
  }

  /**
   * Check if mode is active
   */
  isMode(mode) {
    return this.currentMode === mode;
  }

  /**
   * Go back in history
   */
  back() {
    if (this.history.length > 1) {
      this.history.pop(); // Remove current
      const previous = this.history[this.history.length - 1];
      if (previous) {
        this.navigate(previous.mode);
      }
    }
  }

  /**
   * Event listener management
   */
  on(event, callback) {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event).add(callback);
    return () => this.listeners.get(event)?.delete(callback);
  }

  _emit(event, data) {
    this.listeners.get(event)?.forEach(cb => {
      try {
        cb(data);
      } catch (err) {
        console.error(`Error in router ${event} listener:`, err);
      }
    });
  }
}

// Create singleton instance
export const router = new Router();

// Export to window
window.CYNICRouter = router;
