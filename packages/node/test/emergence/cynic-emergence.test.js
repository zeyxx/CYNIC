/**
 * Test: CynicEmergence (C6.7 CYNIC × EMERGE)
 *
 * Tests pattern accumulation and detection across cycles.
 */

'use strict';

import { describe, it, before, after, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { EventType, globalEventBus } from '@cynic/core';
import { getCynicEmergence, resetCynicEmergence, SignificanceLevel } from '../../src/emergence/cynic-emergence.js';
import { getUnifiedSignalStore, resetUnifiedSignalStore } from '../../src/learning/unified-signal.js';

describe('CynicEmergence (C6.7)', () => {
  let emergence;
  let signalStore;

  beforeEach(() => {
    resetCynicEmergence();
    resetUnifiedSignalStore();
    emergence = getCynicEmergence();
    signalStore = getUnifiedSignalStore();
  });

  after(() => {
    resetCynicEmergence();
    resetUnifiedSignalStore();
  });

  describe('accumulate()', () => {
    it('should accumulate cycle data into rolling buffers', () => {
      emergence.accumulate({
        cycleId: 'cycle_1',
        judgment: { dog: 'Guardian', verdict: 'BARK', qScore: 45 },
        decision: { approved: true, agreementRatio: 0.8, voterCount: 5 },
        metrics: { health: 0.7, heapUsed: 0.5 },
      });

      const health = emergence.getHealth();
      assert.equal(health.dataPoints.dogEvents, 1);
      assert.equal(health.dataPoints.consensusResults, 1);
      assert.equal(health.dataPoints.healthSnapshots, 1);
    });

    it('should detect heap pressure pattern after 10 cycles', () => {
      const patterns = [];
      emergence.on('pattern_detected', (pattern) => patterns.push(pattern));

      // Simulate 10 cycles with high heap
      for (let i = 0; i < 10; i++) {
        emergence.accumulate({
          cycleId: `cycle_${i}`,
          judgment: { dog: 'Guardian', verdict: 'BARK' },
          decision: { approved: true },
          metrics: { memoryLoad: 0.75 }, // Above φ⁻¹ (0.618)
        });
      }

      // Should detect memory pressure pattern
      const memoryPattern = patterns.find(p => p.type === 'memory_pressure');
      assert.ok(memoryPattern, 'Memory pressure pattern should be detected');
      assert.equal(memoryPattern.significance, SignificanceLevel.HIGH);
    });

    it('should detect dog dominance shift', () => {
      const patterns = [];
      emergence.on('pattern_detected', (pattern) => patterns.push(pattern));

      // First 5 cycles: Guardian dominant
      for (let i = 0; i < 5; i++) {
        emergence.accumulate({
          cycleId: `cycle_${i}`,
          judgment: { dog: 'Guardian', verdict: 'BARK' },
        });
      }

      // Next 10 cycles: Synthesizer dominant (shift)
      for (let i = 5; i < 15; i++) {
        emergence.accumulate({
          cycleId: `cycle_${i}`,
          judgment: { dog: 'Synthesizer', verdict: 'WAG' },
        });
      }

      // Should detect dominance shift
      const shiftPattern = patterns.find(p => p.type === 'dog_dominance_shift');
      assert.ok(shiftPattern, 'Dog dominance shift should be detected');
      assert.equal(shiftPattern.data.dominantDog, 'Synthesizer');
    });

    it('should detect consensus quality decline', () => {
      const patterns = [];
      emergence.on('pattern_detected', (pattern) => patterns.push(pattern));

      // 10 cycles with low approval rate
      for (let i = 0; i < 10; i++) {
        emergence.accumulate({
          cycleId: `cycle_${i}`,
          decision: {
            approved: i % 4 === 0, // 25% approval (below φ⁻² = 38.2%)
            agreementRatio: 0.3,
            voterCount: 5,
          },
        });
      }

      // Should detect consensus quality issue
      const qualityPattern = patterns.find(p => p.type === 'consensus_quality_change');
      assert.ok(qualityPattern, 'Consensus quality decline should be detected');
      assert.ok(
        qualityPattern.significance === SignificanceLevel.HIGH ||
        qualityPattern.significance === SignificanceLevel.CRITICAL
      );
    });

    it('should emit patterns to unified_signals', async () => {
      const emittedEvents = [];
      globalEventBus.subscribe(EventType.CYNIC_EMERGENCE, (event) => {
        emittedEvents.push(event.payload);
      });

      // Simulate 10 cycles with high heap to trigger pattern detection
      for (let i = 0; i < 10; i++) {
        emergence.accumulate({
          cycleId: `cycle_${i}`,
          judgment: { dog: 'Guardian', verdict: 'BARK' },
          decision: { approved: true },
          metrics: { memoryLoad: 0.75 }, // High memory
        });
      }

      // Wait for async processing
      await new Promise(resolve => setTimeout(resolve, 50));

      // Should have emitted to event bus
      assert.ok(emittedEvents.length > 0, 'Should emit CYNIC_EMERGENCE events');
      const event = emittedEvents[0];
      assert.equal(event.cell, 'C6.7');
      assert.equal(event.dimension, 'CYNIC');
      assert.equal(event.analysis, 'EMERGE');
      assert.ok(event.pattern);

      // Should have persisted to unified_signals
      const stats = signalStore.getStats();
      assert.ok(stats.totalReceived > 0, 'Should record signals to UnifiedSignalStore');
    });

    it('should provide suggestions in pattern messages', () => {
      const patterns = [];
      emergence.on('pattern_detected', (pattern) => patterns.push(pattern));

      // Trigger memory pressure pattern
      for (let i = 0; i < 10; i++) {
        emergence.accumulate({
          cycleId: `cycle_${i}`,
          metrics: { memoryLoad: 0.85 }, // Critical pressure
        });
      }

      const memoryPattern = patterns.find(p => p.type === 'memory_pressure');
      assert.ok(memoryPattern);
      assert.ok(memoryPattern.message.includes('Memory load'));
      assert.ok(memoryPattern.message.includes('%'));
    });
  });

  describe('pattern detection thresholds', () => {
    it('should use φ-based thresholds', () => {
      const patterns = [];
      emergence.on('pattern_detected', (pattern) => patterns.push(pattern));

      // Test φ⁻¹ threshold for dog dominance (61.8%)
      for (let i = 0; i < 20; i++) {
        emergence.accumulate({
          cycleId: `cycle_${i}`,
          judgment: {
            dog: i < 13 ? 'Guardian' : 'Synthesizer', // 65% Guardian (>φ⁻¹)
            verdict: 'BARK',
          },
        });
      }

      const dominance = patterns.find(p => p.type === 'dog_dominance_shift');
      assert.ok(dominance, 'Should detect dominance at >61.8%');
      assert.ok(dominance.data.ratio >= 0.618);
    });

    it('should φ-bound confidence values', () => {
      const patterns = [];
      emergence.on('pattern_detected', (pattern) => patterns.push(pattern));

      // Trigger any pattern
      for (let i = 0; i < 10; i++) {
        emergence.accumulate({
          cycleId: `cycle_${i}`,
          metrics: { memoryLoad: 0.7 },
        });
      }

      // All patterns should have confidence ≤ φ⁻¹
      for (const pattern of patterns) {
        assert.ok(pattern.confidence <= 0.618, `Confidence ${pattern.confidence} exceeds φ⁻¹`);
      }
    });
  });

  describe('getPatterns()', () => {
    it('should return detected patterns with limit', () => {
      // Generate 30 patterns
      for (let i = 0; i < 30; i++) {
        emergence.accumulate({
          cycleId: `cycle_${i}`,
          metrics: { memoryLoad: 0.7 },
        });
      }

      const patterns = emergence.getPatterns(5);
      assert.ok(patterns.length <= 5);
    });
  });

  describe('getStats()', () => {
    it('should return statistics with baselines', () => {
      emergence.accumulate({
        cycleId: 'cycle_1',
        judgment: { dog: 'Guardian', verdict: 'BARK' },
        decision: { approved: true, agreementRatio: 0.8 },
        metrics: { health: 0.7, memoryLoad: 0.5 },
      });

      const stats = emergence.getStats();
      assert.ok(stats.dogEventsRecorded > 0);
      assert.ok(stats.consensusRecorded > 0);
      assert.ok(stats.healthSnapshotsRecorded > 0);
      assert.ok(stats.baselines);
      assert.ok(stats.baselines.dogDistribution);
    });
  });
});
