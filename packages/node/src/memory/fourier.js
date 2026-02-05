/**
 * Fourier Analysis for Pattern Detection
 *
 * "Tout signal se décompose en fréquences" - κυνικός
 *
 * Uses Fourier transforms to:
 * - Detect periodic patterns in user behavior (daily, weekly cycles)
 * - Find dominant frequencies in judgment scores
 * - Filter noise from genuine patterns
 * - Identify hidden periodicities in metrics
 *
 * φ-aligned: Max confidence 61.8% for frequency detection
 *
 * @module @cynic/node/memory/fourier
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * φ-aligned Fourier configuration
 */
export const FOURIER_CONFIG = {
  // Sample sizes (Fibonacci for natural harmony)
  MIN_SAMPLES: 8,            // Minimum for meaningful FFT
  DEFAULT_SAMPLES: 64,       // 2^6 - good balance
  MAX_SAMPLES: 256,          // 2^8 - max for real-time

  // Frequency detection thresholds (φ-aligned)
  MIN_POWER_THRESHOLD: PHI_INV_3,    // 23.6% - minimum power to be significant
  DOMINANT_THRESHOLD: PHI_INV_2,     // 38.2% - dominant frequency threshold
  STRONG_THRESHOLD: PHI_INV,         // 61.8% - strong signal threshold

  // Common periods to detect (in samples)
  KNOWN_PERIODS: {
    HOURLY: 1,           // 1 sample per hour
    QUARTER_DAY: 6,      // Every 4 hours (if hourly sampling)
    HALF_DAY: 12,        // Every 12 hours
    DAILY: 24,           // Daily cycle
    WEEKLY: 168,         // Weekly cycle (24*7)
    FIBONACCI_13: 13,    // Fib(7) - natural pattern
    FIBONACCI_21: 21,    // Fib(8) - natural pattern
    FIBONACCI_34: 34,    // Fib(9) - natural pattern
  },

  // Windowing functions
  WINDOW_TYPES: ['none', 'hann', 'hamming', 'blackman'],

  // Default window (Hann reduces spectral leakage)
  DEFAULT_WINDOW: 'hann',
};

// ═══════════════════════════════════════════════════════════════════════════════
// CORE FFT IMPLEMENTATION
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Complex number representation for FFT
 */
export class Complex {
  /**
   * @param {number} real - Real part
   * @param {number} imag - Imaginary part
   */
  constructor(real = 0, imag = 0) {
    this.re = real;
    this.im = imag;
  }

  /**
   * Add two complex numbers
   * @param {Complex} other
   * @returns {Complex}
   */
  add(other) {
    return new Complex(this.re + other.re, this.im + other.im);
  }

  /**
   * Subtract two complex numbers
   * @param {Complex} other
   * @returns {Complex}
   */
  sub(other) {
    return new Complex(this.re - other.re, this.im - other.im);
  }

  /**
   * Multiply two complex numbers
   * @param {Complex} other
   * @returns {Complex}
   */
  mul(other) {
    return new Complex(
      this.re * other.re - this.im * other.im,
      this.re * other.im + this.im * other.re
    );
  }

  /**
   * Get magnitude (absolute value)
   * @returns {number}
   */
  magnitude() {
    return Math.sqrt(this.re * this.re + this.im * this.im);
  }

  /**
   * Get phase angle in radians
   * @returns {number}
   */
  phase() {
    return Math.atan2(this.im, this.re);
  }

  /**
   * Get power (magnitude squared)
   * @returns {number}
   */
  power() {
    return this.re * this.re + this.im * this.im;
  }

  /**
   * Complex conjugate
   * @returns {Complex}
   */
  conjugate() {
    return new Complex(this.re, -this.im);
  }

  /**
   * Create from polar coordinates
   * @param {number} r - Magnitude
   * @param {number} theta - Phase angle
   * @returns {Complex}
   */
  static fromPolar(r, theta) {
    return new Complex(r * Math.cos(theta), r * Math.sin(theta));
  }
}

/**
 * Discrete Fourier Transform (naive O(n²) implementation)
 * Used for verification and small arrays
 *
 * X[k] = Σ(n=0 to N-1) x[n] * e^(-2πi*k*n/N)
 *
 * @param {number[]} signal - Input signal (real values)
 * @returns {Complex[]} Frequency spectrum
 */
export function dft(signal) {
  const N = signal.length;
  const spectrum = new Array(N);

  for (let k = 0; k < N; k++) {
    let sum = new Complex(0, 0);
    for (let n = 0; n < N; n++) {
      const theta = (-2 * Math.PI * k * n) / N;
      const twiddle = Complex.fromPolar(1, theta);
      sum = sum.add(twiddle.mul(new Complex(signal[n], 0)));
    }
    spectrum[k] = sum;
  }

  return spectrum;
}

/**
 * Inverse Discrete Fourier Transform
 *
 * x[n] = (1/N) * Σ(k=0 to N-1) X[k] * e^(2πi*k*n/N)
 *
 * @param {Complex[]} spectrum - Frequency spectrum
 * @returns {number[]} Reconstructed signal (real parts)
 */
export function idft(spectrum) {
  const N = spectrum.length;
  const signal = new Array(N);

  for (let n = 0; n < N; n++) {
    let sum = new Complex(0, 0);
    for (let k = 0; k < N; k++) {
      const theta = (2 * Math.PI * k * n) / N;
      const twiddle = Complex.fromPolar(1, theta);
      sum = sum.add(twiddle.mul(spectrum[k]));
    }
    signal[n] = sum.re / N;
  }

  return signal;
}

/**
 * Fast Fourier Transform (Cooley-Tukey radix-2 DIT algorithm)
 * O(n log n) complexity
 *
 * @param {number[]|Complex[]} input - Input signal (padded to power of 2)
 * @param {boolean} [inverse=false] - If true, compute inverse FFT
 * @returns {Complex[]} Frequency spectrum
 */
export function fft(input, inverse = false) {
  // Convert to Complex if needed
  const x = input.map(v =>
    v instanceof Complex ? v : new Complex(v, 0)
  );

  const N = x.length;

  // Base case
  if (N <= 1) return x;

  // Ensure power of 2
  if ((N & (N - 1)) !== 0) {
    throw new Error(`FFT requires power of 2 length, got ${N}`);
  }

  // Bit-reversal permutation
  const bits = Math.log2(N);
  for (let i = 0; i < N; i++) {
    const j = reverseBits(i, bits);
    if (j > i) {
      [x[i], x[j]] = [x[j], x[i]];
    }
  }

  // Cooley-Tukey iterative FFT
  const sign = inverse ? 1 : -1;

  for (let size = 2; size <= N; size *= 2) {
    const halfSize = size / 2;
    const angle = (sign * 2 * Math.PI) / size;

    // Twiddle factor for this stage
    const wBase = Complex.fromPolar(1, angle);

    for (let i = 0; i < N; i += size) {
      let w = new Complex(1, 0);

      for (let j = 0; j < halfSize; j++) {
        const even = x[i + j];
        const odd = w.mul(x[i + j + halfSize]);

        x[i + j] = even.add(odd);
        x[i + j + halfSize] = even.sub(odd);

        w = w.mul(wBase);
      }
    }
  }

  // Scale for inverse FFT
  if (inverse) {
    for (let i = 0; i < N; i++) {
      x[i] = new Complex(x[i].re / N, x[i].im / N);
    }
  }

  return x;
}

/**
 * Inverse FFT
 * @param {Complex[]} spectrum - Frequency spectrum
 * @returns {Complex[]} Time domain signal
 */
export function ifft(spectrum) {
  return fft(spectrum, true);
}

/**
 * Reverse bits in an integer
 * @param {number} x - Integer to reverse
 * @param {number} bits - Number of bits
 * @returns {number} Reversed integer
 */
function reverseBits(x, bits) {
  let result = 0;
  for (let i = 0; i < bits; i++) {
    result = (result << 1) | (x & 1);
    x >>= 1;
  }
  return result;
}

/**
 * Pad signal to next power of 2
 * @param {number[]} signal - Input signal
 * @param {number} [minSize=FOURIER_CONFIG.MIN_SAMPLES] - Minimum size
 * @returns {number[]} Padded signal
 */
export function padToPowerOf2(signal, minSize = FOURIER_CONFIG.MIN_SAMPLES) {
  const targetSize = Math.max(
    minSize,
    Math.pow(2, Math.ceil(Math.log2(signal.length)))
  );

  if (signal.length === targetSize) return signal;

  // Zero-pad
  const padded = new Array(targetSize).fill(0);
  for (let i = 0; i < signal.length; i++) {
    padded[i] = signal[i];
  }
  return padded;
}

// ═══════════════════════════════════════════════════════════════════════════════
// WINDOWING FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Window functions to reduce spectral leakage
 */
export const windowFunctions = {
  /**
   * No windowing (rectangular)
   */
  none: (n, N) => 1,

  /**
   * Hann window (cosine-squared)
   * Good general-purpose window
   */
  hann: (n, N) => 0.5 * (1 - Math.cos((2 * Math.PI * n) / (N - 1))),

  /**
   * Hamming window
   * Better sidelobe suppression than Hann
   */
  hamming: (n, N) => 0.54 - 0.46 * Math.cos((2 * Math.PI * n) / (N - 1)),

  /**
   * Blackman window
   * Even better sidelobe suppression, wider main lobe
   */
  blackman: (n, N) =>
    0.42 -
    0.5 * Math.cos((2 * Math.PI * n) / (N - 1)) +
    0.08 * Math.cos((4 * Math.PI * n) / (N - 1)),

  /**
   * φ-window (golden ratio inspired)
   * Custom window using φ proportions
   */
  phi: (n, N) => {
    const x = (2 * n) / (N - 1) - 1; // -1 to 1
    return Math.pow(1 - Math.abs(x), PHI_INV);
  },
};

/**
 * Apply windowing function to signal
 * @param {number[]} signal - Input signal
 * @param {string} [windowType='hann'] - Window type
 * @returns {number[]} Windowed signal
 */
export function applyWindow(signal, windowType = FOURIER_CONFIG.DEFAULT_WINDOW) {
  const windowFn = windowFunctions[windowType] || windowFunctions.hann;
  const N = signal.length;

  return signal.map((x, n) => x * windowFn(n, N));
}

// ═══════════════════════════════════════════════════════════════════════════════
// POWER SPECTRUM ANALYSIS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Compute power spectrum from FFT result
 * @param {Complex[]} spectrum - FFT result
 * @returns {number[]} Power spectrum (magnitude squared)
 */
export function powerSpectrum(spectrum) {
  // Only return first half (positive frequencies) due to symmetry
  const N = spectrum.length;
  const halfN = Math.floor(N / 2);

  const power = new Array(halfN);
  for (let i = 0; i < halfN; i++) {
    power[i] = spectrum[i].power();
  }

  return power;
}

/**
 * Compute magnitude spectrum
 * @param {Complex[]} spectrum - FFT result
 * @returns {number[]} Magnitude spectrum
 */
export function magnitudeSpectrum(spectrum) {
  const N = spectrum.length;
  const halfN = Math.floor(N / 2);

  const mag = new Array(halfN);
  for (let i = 0; i < halfN; i++) {
    mag[i] = spectrum[i].magnitude();
  }

  return mag;
}

/**
 * Normalize power spectrum to [0, 1]
 * @param {number[]} power - Power spectrum
 * @returns {number[]} Normalized power
 */
export function normalizePower(power) {
  const maxPower = Math.max(...power);
  if (maxPower === 0) return power.map(() => 0);
  return power.map(p => p / maxPower);
}

/**
 * Find dominant frequencies in power spectrum
 * @param {number[]} power - Normalized power spectrum
 * @param {number} sampleRate - Samples per unit time
 * @param {Object} [options={}] - Options
 * @param {number} [options.threshold=PHI_INV_3] - Minimum power threshold
 * @param {number} [options.maxPeaks=5] - Maximum peaks to return
 * @returns {Object[]} Array of {frequency, power, period, significance}
 */
export function findDominantFrequencies(power, sampleRate = 1, options = {}) {
  const {
    threshold = FOURIER_CONFIG.MIN_POWER_THRESHOLD,
    maxPeaks = 5,
  } = options;

  const N = power.length * 2; // Original signal length
  const peaks = [];

  // Skip DC component (index 0)
  for (let i = 1; i < power.length - 1; i++) {
    // Is this a local maximum above threshold?
    if (power[i] >= threshold &&
        power[i] > power[i - 1] &&
        power[i] > power[i + 1]) {
      const frequency = (i * sampleRate) / N;
      const period = 1 / frequency;

      // Classify significance (φ-aligned)
      let significance;
      if (power[i] >= FOURIER_CONFIG.STRONG_THRESHOLD) {
        significance = 'strong';
      } else if (power[i] >= FOURIER_CONFIG.DOMINANT_THRESHOLD) {
        significance = 'dominant';
      } else {
        significance = 'weak';
      }

      peaks.push({
        bin: i,
        frequency,
        period,
        power: power[i],
        significance,
        // φ-bounded confidence
        confidence: Math.min(PHI_INV, power[i]),
      });
    }
  }

  // Sort by power, return top peaks
  return peaks
    .sort((a, b) => b.power - a.power)
    .slice(0, maxPeaks);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PATTERN FREQUENCY ANALYZER
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * PatternFrequencyAnalyzer - Analyze temporal patterns in CYNIC data
 *
 * Detects cyclic patterns in:
 * - User activity (daily/weekly rhythms)
 * - Judgment scores over time
 * - Error frequencies
 * - Pattern occurrence rates
 */
export class PatternFrequencyAnalyzer {
  /**
   * @param {Object} [options={}]
   * @param {string} [options.windowType='hann'] - Window function
   * @param {number} [options.sampleRate=1] - Samples per unit time
   */
  constructor(options = {}) {
    this.windowType = options.windowType || FOURIER_CONFIG.DEFAULT_WINDOW;
    this.sampleRate = options.sampleRate || 1; // Default: 1 sample per hour

    // Analysis history
    this.analyses = [];
    this.detectedCycles = new Map(); // period -> evidence count
  }

  /**
   * Analyze a time series for periodic patterns
   * @param {number[]} signal - Time series data
   * @param {Object} [options={}]
   * @param {string} [options.name] - Name for this analysis
   * @param {number} [options.sampleRate] - Override sample rate
   * @returns {Object} Analysis result
   */
  analyze(signal, options = {}) {
    const name = options.name || `analysis_${Date.now()}`;
    const sampleRate = options.sampleRate || this.sampleRate;

    if (signal.length < FOURIER_CONFIG.MIN_SAMPLES) {
      return {
        success: false,
        error: `Need at least ${FOURIER_CONFIG.MIN_SAMPLES} samples, got ${signal.length}`,
        name,
      };
    }

    // 1. Detrend (remove DC offset)
    const mean = signal.reduce((a, b) => a + b, 0) / signal.length;
    const detrended = signal.map(x => x - mean);

    // 2. Apply window
    const windowed = applyWindow(detrended, this.windowType);

    // 3. Pad to power of 2
    const padded = padToPowerOf2(windowed);

    // 4. FFT
    const spectrum = fft(padded);

    // 5. Power spectrum
    const power = powerSpectrum(spectrum);
    const normalizedPower = normalizePower(power);

    // 6. Find peaks
    const peaks = findDominantFrequencies(normalizedPower, sampleRate);

    // 7. Match known periods
    const matchedPeriods = this._matchKnownPeriods(peaks, sampleRate);

    // 8. Update cycle evidence
    for (const peak of peaks) {
      const roundedPeriod = Math.round(peak.period);
      const count = this.detectedCycles.get(roundedPeriod) || 0;
      this.detectedCycles.set(roundedPeriod, count + 1);
    }

    // Build result
    const result = {
      success: true,
      name,
      timestamp: Date.now(),
      stats: {
        samples: signal.length,
        paddedLength: padded.length,
        mean,
        variance: this._variance(signal, mean),
        sampleRate,
      },
      spectrum: {
        power: normalizedPower,
        maxPower: Math.max(...normalizedPower),
        totalEnergy: power.reduce((a, b) => a + b, 0),
      },
      peaks,
      matchedPeriods,
      // φ-bounded overall confidence
      confidence: this._calculateOverallConfidence(peaks),
    };

    // Save to history
    this.analyses.push({
      name,
      timestamp: result.timestamp,
      peakCount: peaks.length,
      confidence: result.confidence,
    });

    // Prune old analyses
    if (this.analyses.length > 100) {
      this.analyses = this.analyses.slice(-100);
    }

    return result;
  }

  /**
   * Match detected peaks to known periods
   * @param {Object[]} peaks - Detected peaks
   * @param {number} sampleRate - Sample rate
   * @returns {Object[]} Matched periods
   * @private
   */
  _matchKnownPeriods(peaks, sampleRate) {
    const matches = [];
    const tolerance = 0.15; // 15% tolerance

    for (const [periodName, periodValue] of Object.entries(FOURIER_CONFIG.KNOWN_PERIODS)) {
      for (const peak of peaks) {
        const ratio = peak.period / periodValue;
        if (Math.abs(ratio - 1) <= tolerance) {
          matches.push({
            name: periodName,
            expectedPeriod: periodValue,
            detectedPeriod: peak.period,
            power: peak.power,
            confidence: peak.confidence,
            matchQuality: 1 - Math.abs(ratio - 1) / tolerance,
          });
        }
      }
    }

    return matches.sort((a, b) => b.power - a.power);
  }

  /**
   * Calculate variance
   * @private
   */
  _variance(signal, mean) {
    const squaredDiffs = signal.map(x => Math.pow(x - mean, 2));
    return squaredDiffs.reduce((a, b) => a + b, 0) / signal.length;
  }

  /**
   * Calculate overall confidence (φ-bounded)
   * @private
   */
  _calculateOverallConfidence(peaks) {
    if (peaks.length === 0) return 0;

    // Weighted average of peak confidences
    const totalPower = peaks.reduce((sum, p) => sum + p.power, 0);
    if (totalPower === 0) return 0;

    const weightedConf = peaks.reduce(
      (sum, p) => sum + p.confidence * p.power,
      0
    ) / totalPower;

    // Cap at φ⁻¹
    return Math.min(PHI_INV, weightedConf);
  }

  /**
   * Get most frequently detected cycles
   * @param {number} [limit=5]
   * @returns {Object[]} Top cycles
   */
  getTopCycles(limit = 5) {
    return Array.from(this.detectedCycles.entries())
      .map(([period, count]) => ({ period, count }))
      .sort((a, b) => b.count - a.count)
      .slice(0, limit);
  }

  /**
   * Check if a specific period is significant
   * @param {number} period - Period to check
   * @param {number} [minEvidence=3] - Minimum evidence count
   * @returns {boolean}
   */
  isPeriodSignificant(period, minEvidence = 3) {
    const count = this.detectedCycles.get(Math.round(period)) || 0;
    return count >= minEvidence;
  }

  /**
   * Get analyzer stats
   * @returns {Object}
   */
  getStats() {
    return {
      totalAnalyses: this.analyses.length,
      uniqueCycles: this.detectedCycles.size,
      topCycles: this.getTopCycles(3),
      windowType: this.windowType,
      sampleRate: this.sampleRate,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SIGNAL FILTERING
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Low-pass filter - removes high frequency noise
 * @param {number[]} signal - Input signal
 * @param {number} cutoffFreq - Cutoff frequency (0 to 0.5)
 * @returns {number[]} Filtered signal
 */
export function lowPassFilter(signal, cutoffFreq) {
  const padded = padToPowerOf2(signal);
  const spectrum = fft(padded);
  const N = spectrum.length;
  const cutoffBin = Math.floor(cutoffFreq * N);

  // Zero out frequencies above cutoff
  for (let i = cutoffBin; i < N - cutoffBin; i++) {
    spectrum[i] = new Complex(0, 0);
  }

  // IFFT and return real part
  const filtered = ifft(spectrum);
  return filtered.slice(0, signal.length).map(c => c.re);
}

/**
 * High-pass filter - removes low frequency trends
 * @param {number[]} signal - Input signal
 * @param {number} cutoffFreq - Cutoff frequency (0 to 0.5)
 * @returns {number[]} Filtered signal
 */
export function highPassFilter(signal, cutoffFreq) {
  const padded = padToPowerOf2(signal);
  const spectrum = fft(padded);
  const N = spectrum.length;
  const cutoffBin = Math.floor(cutoffFreq * N);

  // Zero out frequencies below cutoff (and symmetrically at high end)
  for (let i = 0; i < cutoffBin; i++) {
    spectrum[i] = new Complex(0, 0);
    spectrum[N - 1 - i] = new Complex(0, 0);
  }

  const filtered = ifft(spectrum);
  return filtered.slice(0, signal.length).map(c => c.re);
}

/**
 * Band-pass filter - keeps frequencies in a range
 * @param {number[]} signal - Input signal
 * @param {number} lowCutoff - Low cutoff frequency
 * @param {number} highCutoff - High cutoff frequency
 * @returns {number[]} Filtered signal
 */
export function bandPassFilter(signal, lowCutoff, highCutoff) {
  const padded = padToPowerOf2(signal);
  const spectrum = fft(padded);
  const N = spectrum.length;
  const lowBin = Math.floor(lowCutoff * N);
  const highBin = Math.floor(highCutoff * N);

  // Keep only frequencies in the band
  const filtered = spectrum.map((c, i) => {
    if ((i >= lowBin && i <= highBin) ||
        (i >= N - highBin && i <= N - lowBin)) {
      return c;
    }
    return new Complex(0, 0);
  });

  const result = ifft(filtered);
  return result.slice(0, signal.length).map(c => c.re);
}

/**
 * Smooth signal using moving average in frequency domain
 * More efficient than time-domain convolution for large signals
 * @param {number[]} signal - Input signal
 * @param {number} [smoothingFactor=0.1] - Smoothing amount (0=none, 1=max)
 * @returns {number[]} Smoothed signal
 */
export function spectralSmooth(signal, smoothingFactor = 0.1) {
  // Use low-pass filter with φ-scaled cutoff
  const cutoff = 0.5 * (1 - smoothingFactor * PHI_INV);
  return lowPassFilter(signal, cutoff);
}

// ═══════════════════════════════════════════════════════════════════════════════
// CROSS-CORRELATION FOR PATTERN MATCHING
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Cross-correlation via FFT
 * Finds how similar two signals are at different time lags
 *
 * @param {number[]} signal1 - First signal
 * @param {number[]} signal2 - Second signal
 * @returns {Object} {correlation: number[], maxLag: number, maxCorrelation: number}
 */
export function crossCorrelation(signal1, signal2) {
  // Pad both to same power of 2
  const targetLength = Math.pow(2, Math.ceil(Math.log2(
    Math.max(signal1.length, signal2.length) * 2
  )));

  const padded1 = padToPowerOf2(signal1, targetLength);
  const padded2 = padToPowerOf2(signal2, targetLength);

  // FFT of both
  const fft1 = fft(padded1);
  const fft2 = fft(padded2);

  // Multiply FFT1 with conjugate of FFT2
  const product = fft1.map((c, i) => c.mul(fft2[i].conjugate()));

  // IFFT gives correlation
  const corr = ifft(product);
  const correlation = corr.map(c => c.re);

  // Find maximum
  let maxCorr = -Infinity;
  let maxLag = 0;
  for (let i = 0; i < correlation.length; i++) {
    if (correlation[i] > maxCorr) {
      maxCorr = correlation[i];
      maxLag = i;
    }
  }

  // Adjust lag for wraparound
  if (maxLag > correlation.length / 2) {
    maxLag = maxLag - correlation.length;
  }

  return {
    correlation,
    maxLag,
    maxCorrelation: maxCorr,
    normalized: maxCorr / (Math.sqrt(
      signal1.reduce((a, b) => a + b * b, 0) *
      signal2.reduce((a, b) => a + b * b, 0)
    ) || 1),
  };
}

/**
 * Auto-correlation - find self-similarity/periodicity
 * @param {number[]} signal - Input signal
 * @returns {Object} {correlation: number[], dominantPeriod: number, periodStrength: number}
 */
export function autoCorrelation(signal) {
  const result = crossCorrelation(signal, signal);

  // Find dominant period (first strong peak after lag 0)
  let dominantPeriod = 0;
  let periodStrength = 0;
  const threshold = result.maxCorrelation * PHI_INV_2; // 38.2% of max

  for (let i = 2; i < result.correlation.length / 2; i++) {
    // Local maximum above threshold
    if (result.correlation[i] > threshold &&
        result.correlation[i] > result.correlation[i - 1] &&
        result.correlation[i] > result.correlation[i + 1]) {
      dominantPeriod = i;
      periodStrength = result.correlation[i] / result.maxCorrelation;
      break;
    }
  }

  return {
    ...result,
    dominantPeriod,
    periodStrength,
    // φ-bounded confidence in detected period
    periodConfidence: Math.min(PHI_INV, periodStrength),
  };
}

// ═══════════════════════════════════════════════════════════════════════════════
// EXPORTS FOR MEMORY INTEGRATION
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Create a pattern frequency analyzer for CYNIC memory
 * @param {Object} [options={}]
 * @returns {PatternFrequencyAnalyzer}
 */
export function createPatternAnalyzer(options = {}) {
  return new PatternFrequencyAnalyzer(options);
}

/**
 * Quick analysis of a time series
 * @param {number[]} signal - Time series data
 * @param {Object} [options={}]
 * @returns {Object} Analysis result
 */
export function analyzeTimeSeries(signal, options = {}) {
  const analyzer = new PatternFrequencyAnalyzer(options);
  return analyzer.analyze(signal, options);
}

export default {
  // Core FFT
  Complex,
  dft,
  idft,
  fft,
  ifft,
  padToPowerOf2,

  // Windowing
  windowFunctions,
  applyWindow,

  // Spectrum analysis
  powerSpectrum,
  magnitudeSpectrum,
  normalizePower,
  findDominantFrequencies,

  // Pattern analyzer
  PatternFrequencyAnalyzer,
  createPatternAnalyzer,
  analyzeTimeSeries,

  // Filtering
  lowPassFilter,
  highPassFilter,
  bandPassFilter,
  spectralSmooth,

  // Correlation
  crossCorrelation,
  autoCorrelation,

  // Config
  FOURIER_CONFIG,
};
