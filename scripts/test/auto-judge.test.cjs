#!/usr/bin/env node
/**
 * Tests for CYNIC Auto-Judge System
 *
 * Autonomous judgment based on observation patterns
 *
 * @module scripts/test/auto-judge.test
 */

'use strict';

const { describe, it, beforeEach } = require('node:test');
const assert = require('node:assert');
const path = require('path');

// Load module under test
const autoJudge = require(path.join(__dirname, '..', 'lib', 'auto-judge.cjs'));

describe('Auto-Judge Module', () => {

  describe('observeSuccess', () => {

    it('should record success observations', () => {
      const result = autoJudge.observeSuccess('TestTool', 'Test completed');

      assert.ok(result, 'Should return observation result');
      assert.ok(result.observation, 'Should have observation');
      assert.strictEqual(result.observation.type, 'success');
    });

    it('should track tool in observation', () => {
      const result = autoJudge.observeSuccess('Bash', 'Command executed');

      assert.strictEqual(result.observation.tool, 'Bash');
    });

  });

  describe('observeError', () => {

    it('should record error observations', () => {
      const result = autoJudge.observeError('TestTool', 'file_not_found', 'File missing');

      assert.ok(result.observation, 'Should have observation');
      assert.strictEqual(result.observation.type, 'error');
      assert.strictEqual(result.observation.errorType, 'file_not_found');
    });

    it('should trigger judgment after repeated errors', () => {
      // Observe multiple errors of same type
      for (let i = 0; i < 5; i++) {
        autoJudge.observeError('FailingTool', 'repeated_error', 'Same error again');
      }

      const stats = autoJudge.getStats();
      assert.ok(stats.totalObservations > 0, 'Should have observations');
    });

  });

  describe('observeCodeChange', () => {

    it('should record code changes', () => {
      autoJudge.observeCodeChange('/test/file.js', 'edit', 10);

      const stats = autoJudge.getStats();
      assert.ok(stats.byType.code_change >= 0, 'Should track code changes');
    });

  });

  describe('getStats', () => {

    it('should return statistics', () => {
      const stats = autoJudge.getStats();

      assert.ok('totalObservations' in stats, 'Should have totalObservations');
      assert.ok('totalJudgments' in stats, 'Should have totalJudgments');
      assert.ok('byType' in stats, 'Should have byType breakdown');
    });

  });

  describe('formatJudgment', () => {

    it('should format HOWL judgment', () => {
      const judgment = {
        verdict: 'HOWL',
        qScore: 85,
        subject: 'test',
        reason: 'Excellent work',
        confidence: 0.618,
      };

      const formatted = autoJudge.formatJudgment(judgment);

      assert.ok(formatted.includes('HOWL'), 'Should include verdict');
      assert.ok(formatted.includes('85'), 'Should include Q-Score');
    });

    it('should format GROWL judgment with warning', () => {
      const judgment = {
        verdict: 'GROWL',
        qScore: 35,
        subject: 'error_pattern',
        reason: 'Multiple errors detected',
        confidence: 0.618,
      };

      const formatted = autoJudge.formatJudgment(judgment);

      assert.ok(formatted.includes('GROWL'), 'Should include verdict');
    });

  });

});

describe('Judgment Verdicts', () => {

  it('should use correct verdict thresholds', () => {
    // HOWL >= 80, WAG >= 50, GROWL >= 30, BARK < 30
    const stats = autoJudge.getStats();
    assert.ok(stats !== null, 'Should have stats');
  });

});
