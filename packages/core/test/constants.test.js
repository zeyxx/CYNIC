/**
 * @cynic/core - Axiom Constants Tests
 *
 * Tests the mathematical foundation of CYNIC.
 * If these fail, EVERYTHING is wrong.
 *
 * "φ derives all" - κυνικός
 *
 * @module @cynic/core/test/constants
 */

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';

import {
  // PHI constants
  PHI,
  PHI_INV,
  PHI_INV_2,
  PHI_INV_3,
  PHI_2,
  PHI_3,

  // Timing
  TIMING_BASE_MS,
  TICK_MS,
  MICRO_MS,
  SLOT_MS,
  BLOCK_MS,
  EPOCH_MS,
  CYCLE_MS,

  // Network
  GOSSIP_FANOUT,
  CONSENSUS_THRESHOLD,
  MIN_PATTERN_SOURCES,
  GOVERNANCE_QUORUM,

  // Helpers
  FIBONACCI,
  fib,

  // Thresholds
  THRESHOLDS,
  EMERGENCE,

  // Score
  MAX_SCORE,
  DECIMAL_PRECISION,

  // Axioms
  AXIOMS,

  // Non-φ
  HUMAN,
  SYSTEM,
} from '../src/axioms/constants.js';

// Tolerance for floating-point comparisons
const EPSILON = 1e-10;

function approxEqual(a, b, epsilon = EPSILON) {
  return Math.abs(a - b) < epsilon;
}

// =============================================================================
// PHI MATHEMATICAL PROPERTIES
// =============================================================================

describe('PHI - The Golden Ratio', () => {
  describe('Mathematical Definition', () => {
    it('PHI should equal (1 + sqrt(5)) / 2', () => {
      const expected = (1 + Math.sqrt(5)) / 2;
      assert.ok(approxEqual(PHI, expected), `PHI=${PHI}, expected=${expected}`);
    });

    it('PHI should be approximately 1.618033988749895', () => {
      assert.ok(approxEqual(PHI, 1.618033988749895));
    });

    it('PHI should be irrational (many decimal places)', () => {
      assert.ok(PHI.toString().length > 10, 'PHI should have many decimal places');
    });
  });

  describe('PHI Inverse Properties', () => {
    it('PHI * PHI_INV should equal 1', () => {
      const product = PHI * PHI_INV;
      assert.ok(approxEqual(product, 1), `PHI * PHI_INV = ${product}, expected 1`);
    });

    it('PHI - 1 should equal PHI_INV', () => {
      const diff = PHI - 1;
      assert.ok(approxEqual(diff, PHI_INV), `PHI - 1 = ${diff}, expected ${PHI_INV}`);
    });

    it('1 / PHI should equal PHI_INV', () => {
      const inverse = 1 / PHI;
      assert.ok(approxEqual(inverse, PHI_INV), `1/PHI = ${inverse}, expected ${PHI_INV}`);
    });

    it('PHI_INV should be approximately 0.618 (61.8%)', () => {
      assert.ok(PHI_INV > 0.617 && PHI_INV < 0.619);
    });
  });

  describe('PHI Power Properties', () => {
    it('PHI_INV_2 should equal PHI_INV^2', () => {
      const expected = PHI_INV * PHI_INV;
      assert.ok(approxEqual(PHI_INV_2, expected), `PHI_INV_2=${PHI_INV_2}, expected=${expected}`);
    });

    it('PHI_INV_3 should equal PHI_INV^3', () => {
      const expected = PHI_INV * PHI_INV * PHI_INV;
      assert.ok(approxEqual(PHI_INV_3, expected), `PHI_INV_3=${PHI_INV_3}, expected=${expected}`);
    });

    it('PHI_2 should equal PHI^2', () => {
      const expected = PHI * PHI;
      assert.ok(approxEqual(PHI_2, expected), `PHI_2=${PHI_2}, expected=${expected}`);
    });

    it('PHI_2 should equal PHI + 1', () => {
      const expected = PHI + 1;
      assert.ok(approxEqual(PHI_2, expected), `PHI_2=${PHI_2}, PHI+1=${expected}`);
    });

    it('PHI_3 should equal PHI^3', () => {
      const expected = PHI * PHI * PHI;
      assert.ok(approxEqual(PHI_3, expected), `PHI_3=${PHI_3}, expected=${expected}`);
    });

    it('PHI_3 should equal PHI_2 + PHI (Fibonacci property)', () => {
      const expected = PHI_2 + PHI;
      assert.ok(approxEqual(PHI_3, expected), `PHI_3=${PHI_3}, PHI_2+PHI=${expected}`);
    });
  });

  describe('PHI Complementary Properties', () => {
    it('PHI_INV + PHI_INV_2 should equal 1', () => {
      const sum = PHI_INV + PHI_INV_2;
      assert.ok(approxEqual(sum, 1), `PHI_INV + PHI_INV_2 = ${sum}, expected 1`);
    });

    it('PHI_INV_2 should equal 1 - PHI_INV', () => {
      const expected = 1 - PHI_INV;
      assert.ok(approxEqual(PHI_INV_2, expected), `PHI_INV_2=${PHI_INV_2}, 1-PHI_INV=${expected}`);
    });
  });
});

// =============================================================================
// TIMING CONSTANTS
// =============================================================================

describe('Timing Constants', () => {
  describe('Base Timing', () => {
    it('TIMING_BASE_MS should be 100ms', () => {
      assert.strictEqual(TIMING_BASE_MS, 100);
    });

    it('BLOCK_MS should equal TIMING_BASE_MS', () => {
      assert.strictEqual(BLOCK_MS, TIMING_BASE_MS);
    });
  });

  describe('PHI-Derived Timings', () => {
    it('TICK_MS should be approximately BASE * PHI_INV_3 (23.6ms)', () => {
      const expected = TIMING_BASE_MS * PHI_INV_3;
      assert.ok(Math.abs(TICK_MS - expected) < 0.5, `TICK_MS=${TICK_MS}, expected≈${expected}`);
    });

    it('MICRO_MS should be approximately BASE * PHI_INV_2 (38.2ms)', () => {
      const expected = TIMING_BASE_MS * PHI_INV_2;
      assert.ok(Math.abs(MICRO_MS - expected) < 0.5, `MICRO_MS=${MICRO_MS}, expected≈${expected}`);
    });

    it('SLOT_MS should be approximately BASE * PHI_INV (61.8ms)', () => {
      const expected = TIMING_BASE_MS * PHI_INV;
      assert.ok(Math.abs(SLOT_MS - expected) < 0.5, `SLOT_MS=${SLOT_MS}, expected≈${expected}`);
    });

    it('EPOCH_MS should be approximately BASE * PHI (161.8ms)', () => {
      const expected = TIMING_BASE_MS * PHI;
      assert.ok(Math.abs(EPOCH_MS - expected) < 0.5, `EPOCH_MS=${EPOCH_MS}, expected≈${expected}`);
    });

    it('CYCLE_MS should be approximately BASE * PHI_2 (261.8ms)', () => {
      const expected = TIMING_BASE_MS * PHI_2;
      assert.ok(Math.abs(CYCLE_MS - expected) < 0.5, `CYCLE_MS=${CYCLE_MS}, expected≈${expected}`);
    });
  });

  describe('Timing Hierarchy', () => {
    it('timings should be in ascending order', () => {
      assert.ok(TICK_MS < MICRO_MS, 'TICK < MICRO');
      assert.ok(MICRO_MS < SLOT_MS, 'MICRO < SLOT');
      assert.ok(SLOT_MS < BLOCK_MS, 'SLOT < BLOCK');
      assert.ok(BLOCK_MS < EPOCH_MS, 'BLOCK < EPOCH');
      assert.ok(EPOCH_MS < CYCLE_MS, 'EPOCH < CYCLE');
    });

    it('each timing should be roughly PHI times the previous', () => {
      // MICRO / TICK ≈ PHI
      const ratio1 = MICRO_MS / TICK_MS;
      assert.ok(Math.abs(ratio1 - PHI) < 0.1, `MICRO/TICK=${ratio1}, expected≈${PHI}`);

      // SLOT / MICRO ≈ PHI
      const ratio2 = SLOT_MS / MICRO_MS;
      assert.ok(Math.abs(ratio2 - PHI) < 0.1, `SLOT/MICRO=${ratio2}, expected≈${PHI}`);

      // EPOCH / BLOCK ≈ PHI
      const ratio3 = EPOCH_MS / BLOCK_MS;
      assert.ok(Math.abs(ratio3 - PHI) < 0.1, `EPOCH/BLOCK=${ratio3}, expected≈${PHI}`);
    });
  });
});

// =============================================================================
// FIBONACCI SEQUENCE
// =============================================================================

describe('Fibonacci Sequence', () => {
  describe('FIBONACCI Array', () => {
    it('should have 15 elements', () => {
      assert.strictEqual(FIBONACCI.length, 15);
    });

    it('should start with 1, 1', () => {
      assert.strictEqual(FIBONACCI[0], 1);
      assert.strictEqual(FIBONACCI[1], 1);
    });

    it('each element should be sum of previous two', () => {
      for (let i = 2; i < FIBONACCI.length; i++) {
        const expected = FIBONACCI[i - 1] + FIBONACCI[i - 2];
        assert.strictEqual(FIBONACCI[i], expected, `Fib[${i}] should be ${expected}`);
      }
    });

    it('should contain known Fibonacci numbers', () => {
      const expected = [1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610];
      assert.deepStrictEqual(FIBONACCI, expected);
    });
  });

  describe('fib() Function', () => {
    it('fib(1) should return 1', () => {
      assert.strictEqual(fib(1), 1);
    });

    it('fib(7) should return 13 (GOSSIP_FANOUT)', () => {
      assert.strictEqual(fib(7), 13);
    });

    it('fib(8) should return 21 (MAX_VALIDATORS)', () => {
      assert.strictEqual(fib(8), 21);
    });

    it('fib(14) should return 377 (MAX_ROOTS)', () => {
      assert.strictEqual(fib(14), 377);
    });

    it('should throw for n < 1', () => {
      assert.throws(() => fib(0), /Fibonacci index must be/);
      assert.throws(() => fib(-1), /Fibonacci index must be/);
    });

    it('should throw for n > 15', () => {
      assert.throws(() => fib(16), /Fibonacci index must be/);
      assert.throws(() => fib(100), /Fibonacci index must be/);
    });

    it('fib(n) should match FIBONACCI[n-1]', () => {
      for (let n = 1; n <= 15; n++) {
        assert.strictEqual(fib(n), FIBONACCI[n - 1]);
      }
    });
  });

  describe('Fibonacci-PHI Relationship', () => {
    it('ratio of consecutive Fibonacci numbers should approach PHI', () => {
      // The larger the index, the closer to PHI
      for (let i = 8; i < FIBONACCI.length; i++) {
        const ratio = FIBONACCI[i] / FIBONACCI[i - 1];
        assert.ok(Math.abs(ratio - PHI) < 0.01, `Fib[${i}]/Fib[${i-1}]=${ratio}, expected≈${PHI}`);
      }
    });
  });
});

// =============================================================================
// NETWORK CONSTANTS
// =============================================================================

describe('Network Constants', () => {
  it('GOSSIP_FANOUT should be Fib(7) = 13', () => {
    assert.strictEqual(GOSSIP_FANOUT, 13);
    assert.strictEqual(GOSSIP_FANOUT, fib(7));
  });

  it('CONSENSUS_THRESHOLD should be PHI_INV (61.8%)', () => {
    assert.strictEqual(CONSENSUS_THRESHOLD, PHI_INV);
  });

  it('MIN_PATTERN_SOURCES should be Fib(4) = 3', () => {
    assert.strictEqual(MIN_PATTERN_SOURCES, 3);
    assert.strictEqual(MIN_PATTERN_SOURCES, fib(4));
  });

  it('GOVERNANCE_QUORUM should be Fib(5) = 5', () => {
    assert.strictEqual(GOVERNANCE_QUORUM, 5);
    assert.strictEqual(GOVERNANCE_QUORUM, fib(5));
  });
});

// =============================================================================
// THRESHOLDS
// =============================================================================

describe('THRESHOLDS', () => {
  describe('Verdict Thresholds', () => {
    it('HOWL should be 80', () => {
      assert.strictEqual(THRESHOLDS.HOWL, 80);
    });

    it('WAG should be 50', () => {
      assert.strictEqual(THRESHOLDS.WAG, 50);
    });

    it('GROWL should be approximately PHI_INV_2 * 100 (38)', () => {
      const expected = Math.round(PHI_INV_2 * 100);
      assert.strictEqual(THRESHOLDS.GROWL, expected);
    });

    it('BARK should be 0', () => {
      assert.strictEqual(THRESHOLDS.BARK, 0);
    });

    it('thresholds should be in descending order', () => {
      assert.ok(THRESHOLDS.HOWL > THRESHOLDS.WAG);
      assert.ok(THRESHOLDS.WAG > THRESHOLDS.GROWL);
      assert.ok(THRESHOLDS.GROWL > THRESHOLDS.BARK);
    });
  });

  describe('Confidence Bounds', () => {
    it('MAX_CONFIDENCE should be PHI_INV (61.8%)', () => {
      assert.strictEqual(THRESHOLDS.MAX_CONFIDENCE, PHI_INV);
    });

    it('MIN_DOUBT should be PHI_INV_2 (38.2%)', () => {
      assert.strictEqual(THRESHOLDS.MIN_DOUBT, PHI_INV_2);
    });

    it('MAX_CONFIDENCE + MIN_DOUBT should equal 1', () => {
      const sum = THRESHOLDS.MAX_CONFIDENCE + THRESHOLDS.MIN_DOUBT;
      assert.ok(approxEqual(sum, 1), `MAX_CONFIDENCE + MIN_DOUBT = ${sum}`);
    });
  });

  describe('Health Thresholds', () => {
    it('HEALTHY should be approximately 62 (PHI_INV * 100)', () => {
      assert.strictEqual(THRESHOLDS.HEALTHY, Math.round(PHI_INV * 100));
    });

    it('WARNING should be approximately 38 (PHI_INV_2 * 100)', () => {
      assert.strictEqual(THRESHOLDS.WARNING, Math.round(PHI_INV_2 * 100));
    });

    it('CRITICAL should be approximately 24 (PHI_INV_3 * 100)', () => {
      assert.strictEqual(THRESHOLDS.CRITICAL, Math.round(PHI_INV_3 * 100));
    });

    it('health thresholds should be in descending order', () => {
      assert.ok(THRESHOLDS.HEALTHY > THRESHOLDS.WARNING);
      assert.ok(THRESHOLDS.WARNING > THRESHOLDS.CRITICAL);
    });
  });

  describe('Anomaly Thresholds', () => {
    it('ANOMALY_LOW should match CRITICAL (24)', () => {
      assert.strictEqual(THRESHOLDS.ANOMALY_LOW, THRESHOLDS.CRITICAL);
    });

    it('ANOMALY_HIGH should be 80 (HOWL threshold)', () => {
      assert.strictEqual(THRESHOLDS.ANOMALY_HIGH, 80);
    });

    it('ANOMALY_RATIO should be PHI_INV_2 (38.2%)', () => {
      assert.strictEqual(THRESHOLDS.ANOMALY_RATIO, PHI_INV_2);
    });
  });
});

// =============================================================================
// EMERGENCE THRESHOLDS
// =============================================================================

describe('EMERGENCE', () => {
  it('CONSCIOUSNESS_THRESHOLD should be PHI_INV * 100 (61.8)', () => {
    const expected = PHI_INV * 100;
    assert.ok(approxEqual(EMERGENCE.CONSCIOUSNESS_THRESHOLD, expected));
  });

  it('PATTERNS_FOR_MAX should be Fib(10) = 55', () => {
    assert.strictEqual(EMERGENCE.PATTERNS_FOR_MAX, 55);
    assert.strictEqual(EMERGENCE.PATTERNS_FOR_MAX, fib(10));
  });

  it('REFINEMENTS_FOR_MAX should be Fib(7) = 13', () => {
    assert.strictEqual(EMERGENCE.REFINEMENTS_FOR_MAX, 13);
    assert.strictEqual(EMERGENCE.REFINEMENTS_FOR_MAX, fib(7));
  });

  it('ANOMALIES_FOR_MAX should be Fib(8) = 21', () => {
    assert.strictEqual(EMERGENCE.ANOMALIES_FOR_MAX, 21);
    assert.strictEqual(EMERGENCE.ANOMALIES_FOR_MAX, fib(8));
  });

  it('persistence factors should use Fibonacci numbers', () => {
    // Note: FIBONACCI array is 0-indexed, fib() is 1-indexed
    // FIBONACCI[10] = 89, fib(11) = 89
    assert.strictEqual(EMERGENCE.PERSISTENCE_PATTERNS_FOR_MAX, FIBONACCI[10]); // 89
    assert.strictEqual(EMERGENCE.PERSISTENCE_HIGH_FREQ_FOR_MAX, FIBONACCI[4]); // 5
    assert.strictEqual(EMERGENCE.PERSISTENCE_JUDGMENTS_FOR_MAX, FIBONACCI[12]); // 233
    assert.strictEqual(EMERGENCE.PERSISTENCE_DAYS_FOR_MAX, FIBONACCI[4]); // 5
    assert.strictEqual(EMERGENCE.PERSISTENCE_MIN_FREQUENCY, FIBONACCI[3]); // 3
  });

  it('4 persistence factors * 25 = 100%', () => {
    assert.strictEqual(EMERGENCE.PERSISTENCE_FACTOR_WEIGHT * 4, 100);
  });
});

// =============================================================================
// SCORE CONSTANTS
// =============================================================================

describe('Score Constants', () => {
  it('MAX_SCORE should be 100', () => {
    assert.strictEqual(MAX_SCORE, 100);
  });

  it('DECIMAL_PRECISION should be 10', () => {
    assert.strictEqual(DECIMAL_PRECISION, 10);
  });

  it('rounding with DECIMAL_PRECISION should give 1 decimal place', () => {
    const value = 61.847;
    const rounded = Math.round(value * DECIMAL_PRECISION) / DECIMAL_PRECISION;
    assert.strictEqual(rounded, 61.8);
  });
});

// =============================================================================
// AXIOMS
// =============================================================================

describe('AXIOMS', () => {
  it('should have exactly 4 axioms', () => {
    assert.strictEqual(Object.keys(AXIOMS).length, 4);
  });

  it('should have PHI, VERIFY, CULTURE, BURN', () => {
    assert.ok('PHI' in AXIOMS);
    assert.ok('VERIFY' in AXIOMS);
    assert.ok('CULTURE' in AXIOMS);
    assert.ok('BURN' in AXIOMS);
  });

  describe('PHI Axiom', () => {
    it('should have correct properties', () => {
      assert.strictEqual(AXIOMS.PHI.symbol, 'φ');
      assert.strictEqual(AXIOMS.PHI.name, 'PHI');
      assert.strictEqual(AXIOMS.PHI.world, 'ATZILUT');
      assert.strictEqual(AXIOMS.PHI.color, '#FFD700'); // Gold
    });
  });

  describe('VERIFY Axiom', () => {
    it('should have correct properties', () => {
      assert.strictEqual(AXIOMS.VERIFY.symbol, '✓');
      assert.strictEqual(AXIOMS.VERIFY.name, 'VERIFY');
      assert.strictEqual(AXIOMS.VERIFY.world, 'BERIAH');
      assert.strictEqual(AXIOMS.VERIFY.color, '#4169E1'); // Royal Blue
    });
  });

  describe('CULTURE Axiom', () => {
    it('should have correct properties', () => {
      assert.strictEqual(AXIOMS.CULTURE.symbol, '⛩');
      assert.strictEqual(AXIOMS.CULTURE.name, 'CULTURE');
      assert.strictEqual(AXIOMS.CULTURE.world, 'YETZIRAH');
      assert.strictEqual(AXIOMS.CULTURE.color, '#228B22'); // Forest Green
    });
  });

  describe('BURN Axiom', () => {
    it('should have correct properties', () => {
      assert.strictEqual(AXIOMS.BURN.symbol, '🔥');
      assert.strictEqual(AXIOMS.BURN.name, 'BURN');
      assert.strictEqual(AXIOMS.BURN.world, 'ASSIAH');
      assert.strictEqual(AXIOMS.BURN.color, '#DC143C'); // Crimson
    });
  });

  describe('Kabbalistic Worlds Order', () => {
    it('should follow Tree of Life order: ATZILUT > BERIAH > YETZIRAH > ASSIAH', () => {
      const worldOrder = ['ATZILUT', 'BERIAH', 'YETZIRAH', 'ASSIAH'];
      const axiomWorlds = [
        AXIOMS.PHI.world,
        AXIOMS.VERIFY.world,
        AXIOMS.CULTURE.world,
        AXIOMS.BURN.world,
      ];
      assert.deepStrictEqual(axiomWorlds, worldOrder);
    });
  });
});

// =============================================================================
// NON-PHI CONSTANTS
// =============================================================================

describe('Non-PHI Constants', () => {
  describe('HUMAN', () => {
    it('WORDS_PER_MINUTE should be 200', () => {
      assert.strictEqual(HUMAN.WORDS_PER_MINUTE, 200);
    });
  });

  describe('SYSTEM', () => {
    it('MS_PER_SECOND should be 1000', () => {
      assert.strictEqual(SYSTEM.MS_PER_SECOND, 1000);
    });

    it('GIT_MAX_BUFFER should be 10MB', () => {
      assert.strictEqual(SYSTEM.GIT_MAX_BUFFER, 10 * 1024 * 1024);
    });
  });
});

// =============================================================================
// INTEGRATION TESTS
// =============================================================================

describe('Cross-Constant Consistency', () => {
  it('CONSENSUS_THRESHOLD should match THRESHOLDS.MAX_CONFIDENCE', () => {
    assert.strictEqual(CONSENSUS_THRESHOLD, THRESHOLDS.MAX_CONFIDENCE);
  });

  it('THRESHOLDS.ANOMALY_RATIO should match THRESHOLDS.MIN_DOUBT', () => {
    assert.strictEqual(THRESHOLDS.ANOMALY_RATIO, THRESHOLDS.MIN_DOUBT);
  });

  it('all PHI-derived values should be mathematically consistent', () => {
    // PHI^-1 + PHI^-2 = 1
    assert.ok(approxEqual(PHI_INV + PHI_INV_2, 1));

    // PHI * PHI^-1 = 1
    assert.ok(approxEqual(PHI * PHI_INV, 1));

    // PHI^2 = PHI + 1
    assert.ok(approxEqual(PHI_2, PHI + 1));

    // PHI^3 = PHI^2 + PHI
    assert.ok(approxEqual(PHI_3, PHI_2 + PHI));
  });

  it('Fibonacci and PHI should be related', () => {
    // Binet's formula: Fib(n) ≈ PHI^n / sqrt(5)
    const sqrt5 = Math.sqrt(5);
    for (let n = 10; n <= 15; n++) {
      const binet = Math.round(Math.pow(PHI, n) / sqrt5);
      assert.strictEqual(fib(n), binet, `Binet formula for Fib(${n})`);
    }
  });
});
