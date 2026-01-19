/**
 * CYNIC Dashboard - Spatial Audio
 *
 * Web Audio API sounds for tool events (Vibecraft pattern)
 * No external dependencies - pure browser audio
 *
 * Each station category has a unique tone:
 * - judgment (gold): Major chord, bright
 * - search (blue): Gentle ping
 * - chain (orange): Deep pulse
 * - ecosystem (green): Nature-like chirp
 * - collective (orange/brown): Pack howl
 * - system (teal): Mechanical click
 * - learning (green/purple): Rising tone
 *
 * "phi distrusts phi" - kunikos
 */

/**
 * Audio manager singleton
 */
class CYNICAudio {
  constructor() {
    this.context = null;
    this.enabled = false;
    this.volume = 0.3;

    // φ-aligned frequencies (based on golden ratio)
    this.baseFreq = 261.63; // Middle C
    this.phi = 1.618;

    // Station sound profiles
    this.profiles = {
      judgment: { freq: 440, type: 'triangle', duration: 0.2, detune: 0 },
      search: { freq: 523.25, type: 'sine', duration: 0.15, detune: 100 },
      chain: { freq: 164.81, type: 'square', duration: 0.3, detune: -50 },
      ecosystem: { freq: 659.25, type: 'sine', duration: 0.1, detune: 200 },
      collective: { freq: 293.66, type: 'sawtooth', duration: 0.25, detune: 0 },
      system: { freq: 392, type: 'square', duration: 0.08, detune: 0 },
      learning: { freq: 349.23, type: 'triangle', duration: 0.2, detune: 150 },
      orchestration: { freq: 466.16, type: 'sawtooth', duration: 0.15, detune: 50 },
      session: { freq: 523.25, type: 'sine', duration: 0.3, detune: 0 },
      analysis: { freq: 493.88, type: 'triangle', duration: 0.12, detune: 100 },
      deploy: { freq: 329.63, type: 'square', duration: 0.2, detune: -100 },
      lsp: { freq: 587.33, type: 'sine', duration: 0.1, detune: 50 },
      other: { freq: 261.63, type: 'sine', duration: 0.15, detune: 0 },
    };
  }

  /**
   * Initialize audio context (must be called after user interaction)
   */
  init() {
    if (this.context) return;

    try {
      this.context = new (window.AudioContext || window.webkitAudioContext)();
      this.enabled = true;
      console.log('🔊 CYNIC Audio: Initialized');
    } catch (err) {
      console.warn('🔇 CYNIC Audio: Web Audio API not available', err);
      this.enabled = false;
    }
  }

  /**
   * Enable audio (requires user interaction)
   */
  enable() {
    console.log('🔊 CYNIC Audio: enable() called');
    this.init();
    if (this.context?.state === 'suspended') {
      console.log('🔊 CYNIC Audio: Resuming suspended context');
      this.context.resume();
    }
    this.enabled = true;
    console.log('🔊 CYNIC Audio: Enabled, context state:', this.context?.state);
    return this.enabled;
  }

  /**
   * Disable audio
   */
  disable() {
    this.enabled = false;
  }

  /**
   * Toggle audio
   * @returns {boolean} New enabled state
   */
  toggle() {
    if (this.enabled) {
      this.disable();
    } else {
      this.enable();
    }
    return this.enabled;
  }

  /**
   * Set volume
   * @param {number} vol - Volume 0-1
   */
  setVolume(vol) {
    this.volume = Math.max(0, Math.min(1, vol));
  }

  /**
   * Play sound for a category
   * @param {string} category - Station category
   * @param {Object} [options] - Override options
   */
  play(category, options = {}) {
    if (!this.enabled || !this.context) return;

    const profile = this.profiles[category] || this.profiles.other;
    const {
      freq = profile.freq,
      type = profile.type,
      duration = profile.duration,
      detune = profile.detune,
    } = options;

    this._playTone(freq, type, duration, detune);
  }

  /**
   * Play tool start sound
   * @param {string} category - Station category
   */
  playToolStart(category) {
    this.play(category, { duration: 0.1 });
  }

  /**
   * Play tool complete sound
   * @param {string} category - Station category
   * @param {boolean} success - Whether tool succeeded
   */
  playToolComplete(category, success = true) {
    if (!success) {
      // Error: dissonant tone
      this.play(category, { detune: -200, duration: 0.2 });
    } else {
      // Success: φ-elevated tone
      const profile = this.profiles[category] || this.profiles.other;
      this.play(category, { freq: profile.freq * this.phi * 0.618 });
    }
  }

  /**
   * Play judgment sound with verdict
   * @param {string} verdict - HOWL, WAG, GROWL, BARK
   */
  playJudgment(verdict) {
    const verdictSounds = {
      HOWL: { freq: 523.25, type: 'triangle', duration: 0.3, detune: 100 },
      WAG: { freq: 440, type: 'sine', duration: 0.25, detune: 50 },
      GROWL: { freq: 293.66, type: 'sawtooth', duration: 0.2, detune: -50 },
      BARK: { freq: 196, type: 'square', duration: 0.15, detune: -100 },
    };
    const sound = verdictSounds[verdict] || verdictSounds.WAG;
    this._playTone(sound.freq, sound.type, sound.duration, sound.detune);
  }

  /**
   * Play block created sound
   * @param {number} slot - Block slot number
   */
  playBlock(slot) {
    // Ascending tone based on slot
    const freq = 164.81 + (slot % 12) * 20;
    this._playTone(freq, 'square', 0.3, 0);
    // Second tone (octave)
    setTimeout(() => {
      this._playTone(freq * 2, 'sine', 0.2, 100);
    }, 100);
  }

  /**
   * Play pattern detected sound
   */
  playPattern() {
    this._playTone(659.25, 'triangle', 0.15, 200);
    setTimeout(() => {
      this._playTone(880, 'sine', 0.1, 100);
    }, 80);
  }

  /**
   * Play connection sound
   */
  playConnect() {
    this._playTone(523.25, 'sine', 0.2, 0);
    setTimeout(() => {
      this._playTone(659.25, 'sine', 0.15, 50);
    }, 100);
  }

  /**
   * Play disconnect sound
   */
  playDisconnect() {
    this._playTone(440, 'sine', 0.2, 0);
    setTimeout(() => {
      this._playTone(329.63, 'sine', 0.3, -50);
    }, 100);
  }

  /**
   * Internal: Play a tone
   * @private
   */
  _playTone(freq, type, duration, detune = 0) {
    if (!this.context || !this.enabled) {
      console.log('🔇 CYNIC Audio: _playTone skipped (context:', !!this.context, ', enabled:', this.enabled, ')');
      return;
    }

    console.log('🔊 CYNIC Audio: Playing tone', { freq, type, duration, detune });
    try {
      const osc = this.context.createOscillator();
      const gain = this.context.createGain();

      osc.type = type;
      osc.frequency.value = freq;
      osc.detune.value = detune;

      // ADSR envelope
      const now = this.context.currentTime;
      gain.gain.setValueAtTime(0, now);
      gain.gain.linearRampToValueAtTime(this.volume, now + 0.01); // Attack
      gain.gain.linearRampToValueAtTime(this.volume * 0.7, now + duration * 0.3); // Decay
      gain.gain.linearRampToValueAtTime(this.volume * 0.5, now + duration * 0.7); // Sustain
      gain.gain.linearRampToValueAtTime(0, now + duration); // Release

      osc.connect(gain);
      gain.connect(this.context.destination);

      osc.start(now);
      osc.stop(now + duration);
    } catch (err) {
      console.warn('🔇 CYNIC Audio: Failed to play tone', err);
    }
  }

  /**
   * Get audio status
   * @returns {Object} Status
   */
  getStatus() {
    return {
      enabled: this.enabled,
      volume: this.volume,
      contextState: this.context?.state || 'uninitialized',
    };
  }
}

// Singleton
export const cynicAudio = new CYNICAudio();

// Export to window for non-module scripts
if (typeof window !== 'undefined') {
  window.CYNICAudio = cynicAudio;
}
