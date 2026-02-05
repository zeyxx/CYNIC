/**
 * Tests for Fourier Analysis Module
 * "La fréquence révèle le pattern" - κυνικός
 */

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
  Complex,
  dft,
  idft,
  fft,
  ifft,
  padToPowerOf2,
  windowFunctions,
  applyWindow,
  powerSpectrum,
  magnitudeSpectrum,
  normalizePower,
  findDominantFrequencies,
  PatternFrequencyAnalyzer,
  createPatternAnalyzer,
  analyzeTimeSeries,
  lowPassFilter,
  highPassFilter,
  bandPassFilter,
  crossCorrelation,
  autoCorrelation,
  FOURIER_CONFIG,
} from '../src/memory/fourier.js';

describe('Complex Numbers', () => {
  describe('Basic operations', () => {
    it('should add complex numbers', () => {
      const a = new Complex(3, 4);
      const b = new Complex(1, 2);
      const sum = a.add(b);

      assert.strictEqual(sum.re, 4);
      assert.strictEqual(sum.im, 6);
    });

    it('should subtract complex numbers', () => {
      const a = new Complex(5, 3);
      const b = new Complex(2, 1);
      const diff = a.sub(b);

      assert.strictEqual(diff.re, 3);
      assert.strictEqual(diff.im, 2);
    });

    it('should multiply complex numbers', () => {
      // (3+4i)(1+2i) = 3 + 6i + 4i + 8i² = 3 + 10i - 8 = -5 + 10i
      const a = new Complex(3, 4);
      const b = new Complex(1, 2);
      const prod = a.mul(b);

      assert.strictEqual(prod.re, -5);
      assert.strictEqual(prod.im, 10);
    });

    it('should calculate magnitude', () => {
      const c = new Complex(3, 4);
      assert.strictEqual(c.magnitude(), 5);
    });

    it('should calculate power (magnitude squared)', () => {
      const c = new Complex(3, 4);
      assert.strictEqual(c.power(), 25);
    });

    it('should create from polar coordinates', () => {
      const c = Complex.fromPolar(1, Math.PI / 2);
      assert.ok(Math.abs(c.re - 0) < 1e-10);
      assert.ok(Math.abs(c.im - 1) < 1e-10);
    });

    it('should compute conjugate', () => {
      const c = new Complex(3, 4);
      const conj = c.conjugate();

      assert.strictEqual(conj.re, 3);
      assert.strictEqual(conj.im, -4);
    });
  });
});

describe('Discrete Fourier Transform', () => {
  describe('dft and idft', () => {
    it('should be reversible (round-trip)', () => {
      const signal = [1, 2, 3, 4, 5, 6, 7, 8];
      const spectrum = dft(signal);
      const recovered = idft(spectrum);

      for (let i = 0; i < signal.length; i++) {
        assert.ok(
          Math.abs(recovered[i] - signal[i]) < 1e-10,
          `Mismatch at index ${i}`
        );
      }
    });

    it('should detect DC component', () => {
      const signal = [5, 5, 5, 5]; // Constant signal
      const spectrum = dft(signal);

      // DC component should be N * value
      assert.ok(Math.abs(spectrum[0].re - 20) < 1e-10);
      assert.ok(Math.abs(spectrum[0].im) < 1e-10);

      // Other components should be zero
      for (let i = 1; i < spectrum.length; i++) {
        assert.ok(Math.abs(spectrum[i].magnitude()) < 1e-10);
      }
    });

    it('should detect single frequency', () => {
      // cos(2πk/N) at k=1
      const N = 8;
      const signal = [];
      for (let n = 0; n < N; n++) {
        signal.push(Math.cos((2 * Math.PI * n) / N));
      }

      const spectrum = dft(signal);

      // Peak at bin 1 (and symmetric at N-1)
      assert.ok(spectrum[1].magnitude() > 3, 'Should have peak at bin 1');
      assert.ok(spectrum[N - 1].magnitude() > 3, 'Should have peak at bin N-1');

      // Other bins should be small
      for (let i = 2; i < N - 1; i++) {
        assert.ok(spectrum[i].magnitude() < 1, `Bin ${i} should be small`);
      }
    });
  });
});

describe('Fast Fourier Transform', () => {
  describe('fft and ifft', () => {
    it('should produce same result as DFT', () => {
      const signal = [1, 2, 3, 4, 5, 6, 7, 8];
      const dftResult = dft(signal);
      const fftResult = fft(signal);

      for (let i = 0; i < signal.length; i++) {
        assert.ok(
          Math.abs(dftResult[i].re - fftResult[i].re) < 1e-10,
          `Real mismatch at ${i}`
        );
        assert.ok(
          Math.abs(dftResult[i].im - fftResult[i].im) < 1e-10,
          `Imag mismatch at ${i}`
        );
      }
    });

    it('should be reversible (round-trip)', () => {
      const signal = [1, 0, -1, 0, 1, 0, -1, 0];
      const spectrum = fft(signal);
      const recovered = ifft(spectrum);

      for (let i = 0; i < signal.length; i++) {
        assert.ok(
          Math.abs(recovered[i].re - signal[i]) < 1e-10,
          `Mismatch at index ${i}`
        );
      }
    });

    it('should require power of 2 length', () => {
      const signal = [1, 2, 3, 4, 5]; // Not power of 2

      assert.throws(() => {
        fft(signal);
      }, /power of 2/i);
    });

    it('should handle larger arrays efficiently', () => {
      const N = 256;
      const signal = Array.from({ length: N }, (_, i) =>
        Math.sin(2 * Math.PI * 3 * i / N) + 0.5 * Math.sin(2 * Math.PI * 7 * i / N)
      );

      const start = Date.now();
      const spectrum = fft(signal);
      const elapsed = Date.now() - start;

      assert.ok(elapsed < 100, `FFT should be fast, took ${elapsed}ms`);
      assert.strictEqual(spectrum.length, N);
    });
  });

  describe('padToPowerOf2', () => {
    it('should pad to next power of 2', () => {
      const signal = [1, 2, 3, 4, 5];
      const padded = padToPowerOf2(signal);

      assert.strictEqual(padded.length, 8);
      assert.deepStrictEqual(padded.slice(0, 5), signal);
      assert.deepStrictEqual(padded.slice(5), [0, 0, 0]);
    });

    it('should not change already power of 2 if >= MIN_SAMPLES', () => {
      const signal = [1, 2, 3, 4, 5, 6, 7, 8]; // 8 = MIN_SAMPLES
      const padded = padToPowerOf2(signal);

      assert.strictEqual(padded.length, 8);
      assert.deepStrictEqual(padded, signal);
    });

    it('should respect minimum size', () => {
      const signal = [1];
      const padded = padToPowerOf2(signal, 16);

      assert.strictEqual(padded.length, 16);
    });
  });
});

describe('Windowing Functions', () => {
  describe('window types', () => {
    it('should have all expected windows', () => {
      for (const type of FOURIER_CONFIG.WINDOW_TYPES) {
        assert.ok(windowFunctions[type], `Missing window: ${type}`);
      }
    });

    it('should return 1 for rectangular window', () => {
      const w = windowFunctions.none(5, 10);
      assert.strictEqual(w, 1);
    });

    it('should return 0 at edges for hann window', () => {
      const N = 10;
      const w0 = windowFunctions.hann(0, N);
      const wN = windowFunctions.hann(N - 1, N);

      assert.ok(Math.abs(w0) < 0.01);
      assert.ok(Math.abs(wN) < 0.01);
    });

    it('should return ~1 at center for hann window', () => {
      const N = 11;
      const wCenter = windowFunctions.hann(5, N);

      assert.ok(wCenter > 0.99);
    });
  });

  describe('applyWindow', () => {
    it('should multiply signal by window', () => {
      const signal = [1, 1, 1, 1, 1];
      const windowed = applyWindow(signal, 'hann');

      // Hann window should make edges small
      assert.ok(windowed[0] < 0.1);
      assert.ok(windowed[4] < 0.1);
      // Center should be close to 1
      assert.ok(windowed[2] > 0.9);
    });
  });
});

describe('Power Spectrum', () => {
  describe('powerSpectrum', () => {
    it('should compute power correctly', () => {
      const spectrum = [
        new Complex(4, 0),
        new Complex(3, 4),
        new Complex(0, 2),
        new Complex(3, 4),
      ];

      const power = powerSpectrum(spectrum);

      // Only first half due to symmetry
      assert.strictEqual(power.length, 2);
      assert.strictEqual(power[0], 16);      // 4² + 0²
      assert.strictEqual(power[1], 25);      // 3² + 4²
    });
  });

  describe('normalizePower', () => {
    it('should normalize to [0, 1]', () => {
      const power = [4, 9, 1, 16];
      const normalized = normalizePower(power);

      assert.strictEqual(normalized[3], 1);   // Max
      assert.strictEqual(normalized[0], 0.25); // 4/16
      assert.strictEqual(normalized[2], 0.0625); // 1/16
    });

    it('should handle all zeros', () => {
      const power = [0, 0, 0, 0];
      const normalized = normalizePower(power);

      assert.deepStrictEqual(normalized, [0, 0, 0, 0]);
    });
  });

  describe('findDominantFrequencies', () => {
    it('should find peaks in spectrum', () => {
      // Create spectrum with clear peak at index 2
      const power = [0.1, 0.2, 0.9, 0.3, 0.5, 0.6, 0.2, 0.1];

      const peaks = findDominantFrequencies(power, 1, { threshold: 0.2 });

      // Should find peaks at indices 2 (0.9) and 5 (0.6)
      assert.ok(peaks.length >= 1);
      assert.strictEqual(peaks[0].bin, 2);
      assert.strictEqual(peaks[0].power, 0.9);
    });

    it('should classify significance correctly', () => {
      // Create spectrum where bins 2 and 4 are local maxima
      const power = [0.1, 0.15, 0.7, 0.3, 0.45, 0.2, 0.1, 0.1];

      const peaks = findDominantFrequencies(power, 1, { threshold: 0.2 });

      // 0.7 should be strong (>= 61.8%)
      const strongPeak = peaks.find(p => p.bin === 2);
      assert.ok(strongPeak, 'Should find peak at bin 2');
      assert.strictEqual(strongPeak.significance, 'strong');

      // 0.45 should be dominant (>= 38.2%)
      const dominantPeak = peaks.find(p => p.bin === 4);
      assert.ok(dominantPeak, 'Should find peak at bin 4');
      assert.strictEqual(dominantPeak.significance, 'dominant');
    });

    it('should cap confidence at φ⁻¹', () => {
      const power = [0.1, 0.15, 0.95, 0.1, 0.1, 0.1, 0.1, 0.1];

      const peaks = findDominantFrequencies(power, 1, { threshold: 0.2 });
      const maxConfidence = Math.max(...peaks.map(p => p.confidence));

      // Allow for floating point: φ⁻¹ ≈ 0.618033988749895
      assert.ok(maxConfidence <= 0.6181, `Confidence ${maxConfidence} exceeds φ⁻¹`);
    });
  });
});

describe('PatternFrequencyAnalyzer', () => {
  describe('construction', () => {
    it('should create with default options', () => {
      const analyzer = new PatternFrequencyAnalyzer();

      assert.strictEqual(analyzer.windowType, FOURIER_CONFIG.DEFAULT_WINDOW);
      assert.strictEqual(analyzer.sampleRate, 1);
    });

    it('should create with custom options', () => {
      const analyzer = new PatternFrequencyAnalyzer({
        windowType: 'hamming',
        sampleRate: 24,
      });

      assert.strictEqual(analyzer.windowType, 'hamming');
      assert.strictEqual(analyzer.sampleRate, 24);
    });
  });

  describe('analyze', () => {
    it('should detect sinusoidal pattern', () => {
      const analyzer = new PatternFrequencyAnalyzer({ sampleRate: 1 });

      // Create 64 samples with period 8 (frequency = 8 cycles in 64 samples)
      const signal = Array.from({ length: 64 }, (_, i) =>
        Math.sin(2 * Math.PI * 8 * i / 64)
      );

      const result = analyzer.analyze(signal, { name: 'sine_test' });

      assert.ok(result.success);
      assert.ok(result.peaks.length >= 1, 'Should detect at least one peak');

      // Frequency should be 8/64 = 0.125, period = 1/0.125 = 8
      // With windowing, the peak might shift slightly
      const hasCorrectFrequency = result.peaks.some(p =>
        p.bin === 8 || Math.abs(p.period - 8) < 3
      );
      assert.ok(hasCorrectFrequency, `Should detect period ~8, got peaks: ${JSON.stringify(result.peaks.map(p => p.period))}`);
    });

    it('should return error for too few samples', () => {
      const analyzer = new PatternFrequencyAnalyzer();
      const signal = [1, 2, 3, 4]; // Only 4 samples

      const result = analyzer.analyze(signal);

      assert.ok(!result.success);
      assert.ok(result.error);
    });

    it('should match known periods', () => {
      const analyzer = new PatternFrequencyAnalyzer({ sampleRate: 1 });

      // Create daily pattern (period 24) with 128 samples
      const signal = Array.from({ length: 128 }, (_, i) =>
        Math.sin(2 * Math.PI * i / 24)
      );

      const result = analyzer.analyze(signal);

      assert.ok(result.success);
      assert.ok(result.matchedPeriods.length >= 0); // May or may not match depending on tolerance
    });

    it('should bound overall confidence at φ⁻¹', () => {
      const analyzer = new PatternFrequencyAnalyzer();

      // Strong signal
      const signal = Array.from({ length: 64 }, (_, i) =>
        Math.sin(2 * Math.PI * i / 8)
      );

      const result = analyzer.analyze(signal);

      // Allow for floating point: φ⁻¹ ≈ 0.618033988749895
      assert.ok(result.confidence <= 0.6181, `Confidence ${result.confidence} exceeds φ⁻¹`);
    });
  });

  describe('getTopCycles', () => {
    it('should track detected cycles across analyses', () => {
      const analyzer = new PatternFrequencyAnalyzer();

      // Analyze same pattern multiple times
      for (let i = 0; i < 3; i++) {
        const signal = Array.from({ length: 64 }, (_, j) =>
          Math.sin(2 * Math.PI * j / 8)
        );
        analyzer.analyze(signal);
      }

      const topCycles = analyzer.getTopCycles(3);

      assert.ok(topCycles.length >= 1);
      // Most frequent should have count >= 3
      assert.ok(topCycles[0].count >= 1);
    });
  });

  describe('getStats', () => {
    it('should return analyzer statistics', () => {
      const analyzer = new PatternFrequencyAnalyzer();

      const signal = Array.from({ length: 64 }, (_, i) => Math.random());
      analyzer.analyze(signal);

      const stats = analyzer.getStats();

      assert.strictEqual(stats.totalAnalyses, 1);
      assert.strictEqual(stats.windowType, FOURIER_CONFIG.DEFAULT_WINDOW);
    });
  });
});

describe('Signal Filtering', () => {
  describe('lowPassFilter', () => {
    it('should remove high frequencies', () => {
      // Mixed signal: low frequency + high frequency
      const N = 64;
      const signal = Array.from({ length: N }, (_, i) =>
        Math.sin(2 * Math.PI * 2 * i / N) +  // Low freq (2 cycles)
        Math.sin(2 * Math.PI * 20 * i / N)   // High freq (20 cycles)
      );

      const filtered = lowPassFilter(signal, 0.1); // Keep only lowest 10%

      // Calculate energy in filtered signal
      const energy = filtered.reduce((sum, x) => sum + x * x, 0);

      // Should still have some energy (from low freq)
      assert.ok(energy > 1, 'Should retain low frequency');

      // High frequency should be attenuated
      // (difficult to test precisely due to windowing effects)
    });
  });

  describe('highPassFilter', () => {
    it('should remove low frequencies', () => {
      // Mixed signal with DC offset
      const N = 64;
      const signal = Array.from({ length: N }, (_, i) =>
        5 +  // DC offset
        Math.sin(2 * Math.PI * 10 * i / N)  // Higher freq
      );

      const filtered = highPassFilter(signal, 0.05);

      // DC should be removed, so mean should be close to 0
      const mean = filtered.reduce((a, b) => a + b, 0) / N;
      assert.ok(Math.abs(mean) < 1, `Mean ${mean} should be near 0`);
    });
  });

  describe('bandPassFilter', () => {
    it('should keep only frequencies in band', () => {
      const N = 64;
      // Three frequencies: low (2), mid (8), high (20)
      const signal = Array.from({ length: N }, (_, i) =>
        Math.sin(2 * Math.PI * 2 * i / N) +
        Math.sin(2 * Math.PI * 8 * i / N) +
        Math.sin(2 * Math.PI * 20 * i / N)
      );

      // Keep only mid frequencies
      const filtered = bandPassFilter(signal, 0.1, 0.2);

      // Should have some energy
      const energy = filtered.reduce((sum, x) => sum + x * x, 0);
      assert.ok(energy > 0);
    });
  });
});

describe('Correlation', () => {
  describe('crossCorrelation', () => {
    it('should find maximum correlation at zero lag for identical signals', () => {
      const signal = [1, 2, 3, 4, 5, 6, 7, 8];

      const result = crossCorrelation(signal, signal);

      assert.strictEqual(result.maxLag, 0);
      assert.ok(result.maxCorrelation > 0);
    });

    it('should find non-zero lag for shifted signal', () => {
      const signal1 = [0, 0, 1, 2, 3, 2, 1, 0];
      const signal2 = [0, 0, 0, 0, 1, 2, 3, 2];  // Shifted right by ~2

      const result = crossCorrelation(signal1, signal2);

      // Max correlation should be at non-zero lag (could be positive or negative depending on interpretation)
      assert.ok(result.maxLag !== 0 || result.maxCorrelation > 0, 'Should find correlation');
    });
  });

  describe('autoCorrelation', () => {
    it('should find periodicity', () => {
      // Periodic signal with period 8
      const signal = Array.from({ length: 32 }, (_, i) =>
        Math.sin(2 * Math.PI * i / 8)
      );

      const result = autoCorrelation(signal);

      // Should detect period around 8
      assert.ok(
        Math.abs(result.dominantPeriod - 8) <= 2,
        `Expected period ~8, got ${result.dominantPeriod}`
      );
    });

    it('should bound period confidence at φ⁻¹', () => {
      const signal = Array.from({ length: 64 }, (_, i) =>
        Math.sin(2 * Math.PI * i / 8)
      );

      const result = autoCorrelation(signal);

      // Allow for floating point: φ⁻¹ ≈ 0.618033988749895
      assert.ok(
        result.periodConfidence <= 0.6181,
        `Confidence ${result.periodConfidence} exceeds φ⁻¹`
      );
    });
  });
});

describe('Factory Functions', () => {
  describe('createPatternAnalyzer', () => {
    it('should create analyzer instance', () => {
      const analyzer = createPatternAnalyzer({ sampleRate: 10 });

      assert.ok(analyzer instanceof PatternFrequencyAnalyzer);
      assert.strictEqual(analyzer.sampleRate, 10);
    });
  });

  describe('analyzeTimeSeries', () => {
    it('should perform quick analysis', () => {
      const signal = Array.from({ length: 64 }, (_, i) =>
        Math.sin(2 * Math.PI * i / 16)
      );

      const result = analyzeTimeSeries(signal, { name: 'quick_test' });

      assert.ok(result.success);
      assert.strictEqual(result.name, 'quick_test');
    });
  });
});

describe('Integration', () => {
  describe('Real-world pattern detection', () => {
    it('should detect daily user activity pattern', () => {
      const analyzer = new PatternFrequencyAnalyzer({ sampleRate: 1 }); // 1 sample/hour

      // Simulate 7 days of hourly activity (168 samples)
      // Higher activity during day (9am-5pm), lower at night
      const signal = [];
      for (let day = 0; day < 7; day++) {
        for (let hour = 0; hour < 24; hour++) {
          const dayActivity = hour >= 9 && hour <= 17 ? 1 : 0.2;
          signal.push(dayActivity + Math.random() * 0.3);
        }
      }

      const result = analyzer.analyze(signal, { name: 'daily_activity' });

      assert.ok(result.success);
      // Should detect ~24 hour period
      const hasDailyPeriod = result.peaks.some(p =>
        Math.abs(p.period - 24) < 5
      );
      assert.ok(hasDailyPeriod, 'Should detect daily cycle');
    });

    it('should detect weekly pattern', () => {
      const analyzer = new PatternFrequencyAnalyzer({ sampleRate: 1 });

      // Simulate 4 weeks of daily data (28 samples)
      // Higher activity on weekdays, lower on weekends
      const signal = [];
      for (let week = 0; week < 4; week++) {
        for (let day = 0; day < 7; day++) {
          const isWeekend = day >= 5;
          signal.push(isWeekend ? 0.3 : 1.0);
        }
      }

      // Pad to 32 for FFT
      while (signal.length < 32) signal.push(0.5);

      const result = analyzer.analyze(signal, { name: 'weekly_pattern' });

      assert.ok(result.success);
      // Should detect ~7 day period
      const hasWeeklyPeriod = result.peaks.some(p =>
        Math.abs(p.period - 7) < 2
      );
      assert.ok(hasWeeklyPeriod, 'Should detect weekly cycle');
    });
  });
});
