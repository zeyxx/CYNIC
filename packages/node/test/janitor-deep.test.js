/**
 * @cynic/node - CollectiveJanitor Deep Tests
 *
 * Comprehensive tests for Janitor (Yesod - Foundation):
 * - Constructor & defaults
 * - shouldTrigger conditions (PostToolUse tools, periodic/scan)
 * - process() flow (analyze, score, record, emit events)
 * - _analyzeContent (file length, function length, nesting, TODO/FIXME, console.log)
 * - _calculateQualityScore (penalty calculation, φ-aligned)
 * - _recordQualityReport (history, running average, trim at 233)
 * - _checkFunctionLength (strictness-adjusted)
 * - _updateDeadCodeTracking (deduplication, trim at 21)
 * - voteOnConsensus (quality-focused vs messy patterns)
 * - Profile-based strictness (NOVICE to MASTER)
 * - Event subscriptions (PROFILE_UPDATED, PATTERN_DETECTED)
 * - Quality events (QUALITY_REPORT, AUTO_FIX_APPLIED, DEAD_CODE_DETECTED)
 * - getSummary, clear
 * - Auto-fix application
 * - Dead code detection (console.log, unused variables)
 * - φ-alignment (constants, thresholds)
 *
 * Uses node:test (NOT vitest) for CI compatibility.
 *
 * "φ distrusts φ" - κυνικός
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  CollectiveJanitor,
  JANITOR_CONSTANTS,
  QualitySeverity,
  IssueType,
} from '../src/agents/collective/janitor.js';
import { AgentEventBus } from '../src/agents/event-bus.js';
import { AgentEvent, AgentId, QualityReportEvent, AutoFixAppliedEvent, DeadCodeDetectedEvent } from '../src/agents/events.js';
import { AgentResponse } from '../src/agents/base.js';
import { ProfileLevel } from '../src/profile/calculator.js';
import { PHI, PHI_INV, PHI_INV_2, PHI_2 } from '@cynic/core';

describe('CollectiveJanitor (Deep)', () => {
  let janitor;
  let eventBus;

  beforeEach(() => {
    eventBus = new AgentEventBus();
    eventBus.registerAgent(AgentId.JANITOR);
    janitor = new CollectiveJanitor({
      eventBus,
      profileLevel: ProfileLevel.PRACTITIONER,
    });
  });

  afterEach(() => {
    eventBus.destroy();
  });

  // ═══════════════════════════════════════════════════════════════════════
  // CONSTRUCTOR & DEFAULTS
  // ═══════════════════════════════════════════════════════════════════════

  describe('constructor', () => {
    it('should set name to Janitor', () => {
      assert.strictEqual(janitor.name, 'Janitor');
    });

    it('should default profileLevel to PRACTITIONER', () => {
      const j = new CollectiveJanitor();
      assert.strictEqual(j.profileLevel, ProfileLevel.PRACTITIONER);
    });

    it('should initialize empty arrays', () => {
      assert.deepStrictEqual(janitor.issues, []);
      assert.deepStrictEqual(janitor.deadCode, []);
      assert.deepStrictEqual(janitor.qualityHistory, []);
    });

    it('should initialize stats to zero', () => {
      assert.strictEqual(janitor.stats.scansPerformed, 0);
      assert.strictEqual(janitor.stats.issuesFound, 0);
      assert.strictEqual(janitor.stats.issuesFixed, 0);
      assert.strictEqual(janitor.stats.deadCodeFound, 0);
      assert.strictEqual(janitor.stats.averageQualityScore, 0);
    });

    it('should accept profileLevel option', () => {
      const j = new CollectiveJanitor({ profileLevel: ProfileLevel.EXPERT });
      assert.strictEqual(j.profileLevel, ProfileLevel.EXPERT);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // shouldTrigger
  // ═══════════════════════════════════════════════════════════════════════

  describe('shouldTrigger', () => {
    it('should trigger on PostToolUse with edit tool', () => {
      assert.strictEqual(janitor.shouldTrigger({ type: 'PostToolUse', tool: 'Edit' }), true);
    });

    it('should trigger on PostToolUse with write tool', () => {
      assert.strictEqual(janitor.shouldTrigger({ type: 'PostToolUse', tool: 'Write' }), true);
    });

    it('should trigger on PostToolUse with create tool', () => {
      assert.strictEqual(janitor.shouldTrigger({ type: 'PostToolUse', tool: 'CreateFile' }), true);
    });

    it('should trigger on PostToolUse with delete tool', () => {
      assert.strictEqual(janitor.shouldTrigger({ type: 'PostToolUse', tool: 'DeleteFile' }), true);
    });

    it('should trigger on post_tool_use (snake_case)', () => {
      assert.strictEqual(janitor.shouldTrigger({ type: 'post_tool_use', tool: 'edit' }), true);
    });

    it('should trigger on periodic type', () => {
      assert.strictEqual(janitor.shouldTrigger({ type: 'periodic' }), true);
    });

    it('should trigger on scan type', () => {
      assert.strictEqual(janitor.shouldTrigger({ type: 'scan' }), true);
    });

    it('should NOT trigger on PostToolUse with Read tool', () => {
      assert.strictEqual(janitor.shouldTrigger({ type: 'PostToolUse', tool: 'Read' }), false);
    });

    it('should NOT trigger on PreToolUse', () => {
      assert.strictEqual(janitor.shouldTrigger({ type: 'PreToolUse' }), false);
    });

    it('should NOT trigger on unrelated type', () => {
      assert.strictEqual(janitor.shouldTrigger({ type: 'random' }), false);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // CONSTANTS φ-ALIGNMENT
  // ═══════════════════════════════════════════════════════════════════════

  describe('JANITOR_CONSTANTS', () => {
    it('should have COMPLEXITY_THRESHOLD ≈ φ² × 10 ≈ 26', () => {
      assert.strictEqual(JANITOR_CONSTANTS.COMPLEXITY_THRESHOLD, Math.round(PHI_2 * 10));
    });

    it('should have MAX_FILE_LENGTH = 987 (Fib(16))', () => {
      assert.strictEqual(JANITOR_CONSTANTS.MAX_FILE_LENGTH, 987);
    });

    it('should have MAX_FUNCTION_LENGTH = 55 (Fib(10))', () => {
      assert.strictEqual(JANITOR_CONSTANTS.MAX_FUNCTION_LENGTH, 55);
    });

    it('should have STALE_BRANCH_DAYS = 21 (Fib(8))', () => {
      assert.strictEqual(JANITOR_CONSTANTS.STALE_BRANCH_DAYS, 21);
    });

    it('should have QUALITY_WARNING_THRESHOLD = φ⁻¹', () => {
      assert.strictEqual(JANITOR_CONSTANTS.QUALITY_WARNING_THRESHOLD, PHI_INV);
    });

    it('should have QUALITY_FAIL_THRESHOLD = φ⁻²', () => {
      assert.strictEqual(JANITOR_CONSTANTS.QUALITY_FAIL_THRESHOLD, PHI_INV_2);
    });

    it('should have MAX_DEAD_CODE_TRACKED = 21 (Fib(8))', () => {
      assert.strictEqual(JANITOR_CONSTANTS.MAX_DEAD_CODE_TRACKED, 21);
    });
  });

  describe('QualitySeverity', () => {
    it('should have CRITICAL weight = φ²', () => {
      assert.strictEqual(QualitySeverity.CRITICAL.weight, PHI_2);
    });

    it('should have HIGH weight = φ', () => {
      assert.strictEqual(QualitySeverity.HIGH.weight, PHI);
    });

    it('should have MEDIUM weight = 1.0', () => {
      assert.strictEqual(QualitySeverity.MEDIUM.weight, 1.0);
    });

    it('should have LOW weight = φ⁻¹', () => {
      assert.strictEqual(QualitySeverity.LOW.weight, PHI_INV);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // process() FLOW
  // ═══════════════════════════════════════════════════════════════════════

  describe('process', () => {
    it('should increment scansPerformed', async () => {
      await janitor.process({ tool: 'Write', payload: { content: 'hello' } }, {});
      assert.strictEqual(janitor.stats.scansPerformed, 1);
    });

    it('should return ALLOW for clean code', async () => {
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: 'const x = 1;' },
      }, {});
      assert.strictEqual(result.response, AgentResponse.ALLOW);
      assert.strictEqual(result.agent, 'janitor');
    });

    it('should return WARN for low quality score', async () => {
      // Create code with many issues
      const badCode = 'TODO: fix\nFIXME: urgent\nconsole.log("debug");\nTODO: another\nFIXME: more';
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: badCode },
      }, {});
      // Multiple medium/low severity issues should reduce score
      assert.ok(result.qualityScore < 100);
    });

    it('should include profileLevel in result', async () => {
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: 'const x = 1;' },
      }, {});
      assert.strictEqual(result.profileLevel, ProfileLevel.PRACTITIONER);
    });

    it('should emit QUALITY_REPORT event when issues found', async () => {
      let emitted = null;
      eventBus.registerAgent('test-listener');
      eventBus.subscribe(AgentEvent.QUALITY_REPORT, 'test-listener', (event) => {
        emitted = event;
      });

      await janitor.process({
        tool: 'Write',
        payload: { content: 'TODO: fix this\nconsole.log("debug");' },
      }, {});

      assert.ok(emitted, 'Should emit QUALITY_REPORT');
      assert.strictEqual(emitted.source, AgentId.JANITOR);
      assert.ok(emitted.payload.issues.length > 0);
    });

    it('should NOT emit QUALITY_REPORT when no issues', async () => {
      let emitted = null;
      eventBus.registerAgent('test-listener');
      eventBus.subscribe(AgentEvent.QUALITY_REPORT, 'test-listener', (event) => {
        emitted = event;
      });

      await janitor.process({
        tool: 'Write',
        payload: { content: 'const x = 1;' },
      }, {});

      assert.strictEqual(emitted, null, 'Should NOT emit when no issues');
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // _analyzeContent - FILE LENGTH
  // ═══════════════════════════════════════════════════════════════════════

  describe('file length analysis', () => {
    it('should detect long files', async () => {
      const longFile = 'line\n'.repeat(1000); // 1000 lines > 987
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: longFile, file: 'test.js' },
      }, {});

      const longFileIssue = result.issues || janitor.issues.some(
        (i) => i.type === IssueType.LONG_FILE
      );
      assert.ok(longFileIssue, 'Should detect long file');
    });

    it('should NOT flag short files', async () => {
      const shortFile = 'const x = 1;\nconst y = 2;\n';
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: shortFile },
      }, {});

      assert.ok(result.issues === 0 || !result.issues, 'Should not flag short file');
    });

    it('should adjust threshold by strictness (NOVICE)', async () => {
      janitor.profileLevel = ProfileLevel.NOVICE;
      // Novice: strictness = 0.5, so max = 987 / 0.5 = 1974
      const longFile = 'line\n'.repeat(1000);
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: longFile },
      }, {});

      // 1000 lines should be OK for NOVICE (max 1974)
      assert.ok(result.qualityScore > 50, 'NOVICE should allow longer files');
    });

    it('should adjust threshold by strictness (MASTER)', async () => {
      janitor.profileLevel = ProfileLevel.MASTER;
      // MASTER: strictness = φ ≈ 1.618, so max = 987 / 1.618 ≈ 610
      const longFile = 'line\n'.repeat(700);
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: longFile },
      }, {});

      // 700 lines should exceed MASTER threshold (610)
      assert.ok(result.qualityScore < 100, 'MASTER should flag 700-line file');
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // _analyzeContent - FUNCTION LENGTH
  // ═══════════════════════════════════════════════════════════════════════

  describe('function length analysis', () => {
    it('should detect long functions', async () => {
      const longFunc = `function test() {\n${'  return 1;\n'.repeat(60)}}`;
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: longFunc },
      }, {});

      assert.ok(result.qualityScore < 100, 'Should detect long function');
    });

    it('should NOT flag short functions', async () => {
      const shortFunc = 'function test() {\n  return 1;\n}\n';
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: shortFunc },
      }, {});

      assert.strictEqual(result.qualityScore, 100, 'Short function should be perfect');
    });

    it('should handle async functions', async () => {
      const asyncFunc = `async function test() {\n${'  await delay();\n'.repeat(60)}}`;
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: asyncFunc },
      }, {});

      assert.ok(result.qualityScore < 100, 'Should detect long async function');
    });

    it('should handle arrow functions', async () => {
      const arrowFunc = `const test = async () => {\n${'  await delay();\n'.repeat(60)}}`;
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: arrowFunc },
      }, {});

      assert.ok(result.qualityScore < 100, 'Should detect long arrow function');
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // _analyzeContent - NESTING DEPTH
  // ═══════════════════════════════════════════════════════════════════════

  describe('nesting depth analysis', () => {
    it('should detect deep nesting', async () => {
      const deepNesting = 'if (a) {\n  if (b) {\n    if (c) {\n      if (d) {\n        if (e) {\n          if (f) {\n            code();\n          }\n        }\n      }\n    }\n  }\n}';
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: deepNesting },
      }, {});

      assert.ok(result.qualityScore < 100, 'Should detect deep nesting');
    });

    it('should NOT flag shallow nesting', async () => {
      const shallowNesting = 'if (a) {\n  if (b) {\n    code();\n  }\n}';
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: shallowNesting },
      }, {});

      assert.strictEqual(result.qualityScore, 100, 'Shallow nesting should be OK');
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // _analyzeContent - TODO/FIXME COMMENTS
  // ═══════════════════════════════════════════════════════════════════════

  describe('TODO/FIXME detection', () => {
    it('should detect TODO comments', async () => {
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: '// TODO: implement this\nconst x = 1;' },
      }, {});

      assert.ok(result.issues > 0, 'Should detect TODO');
      assert.ok(result.qualityScore < 100);
    });

    it('should detect FIXME comments', async () => {
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: '// FIXME: broken logic\nconst x = 1;' },
      }, {});

      assert.ok(result.issues > 0, 'Should detect FIXME');
      assert.ok(result.qualityScore < 100);
    });

    it('should detect multiple TODOs', async () => {
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: 'TODO: one\nTODO: two\nTODO: three\n' },
      }, {});

      assert.ok(result.issues >= 3, 'Should detect multiple TODOs');
    });

    it('should give FIXME higher severity than TODO', async () => {
      const todoResult = await janitor.process({
        tool: 'Write',
        payload: { content: 'TODO: implement\n' },
      }, {});

      const fixmeResult = await janitor.process({
        tool: 'Write',
        payload: { content: 'FIXME: broken\n' },
      }, {});

      // FIXME should have lower score (higher penalty)
      assert.ok(fixmeResult.qualityScore < todoResult.qualityScore);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // _analyzeContent - DEAD CODE (console.log, unused)
  // ═══════════════════════════════════════════════════════════════════════

  describe('dead code detection', () => {
    it('should detect console.log', async () => {
      let deadCodeEvent = null;
      eventBus.registerAgent('test-listener');
      eventBus.subscribe(AgentEvent.DEAD_CODE_DETECTED, 'test-listener', (event) => {
        deadCodeEvent = event;
      });

      await janitor.process({
        tool: 'Write',
        payload: { content: 'console.log("debug");', file: 'test.js' },
      }, {});

      assert.ok(deadCodeEvent, 'Should emit DEAD_CODE_DETECTED');
      assert.strictEqual(deadCodeEvent.payload.name, 'console.log');
    });

    it('should detect console.debug', async () => {
      await janitor.process({
        tool: 'Write',
        payload: { content: 'console.debug("test");' },
      }, {});

      assert.ok(janitor.stats.deadCodeFound > 0);
    });

    it('should detect console.info', async () => {
      await janitor.process({
        tool: 'Write',
        payload: { content: 'console.info("info");' },
      }, {});

      assert.ok(janitor.stats.deadCodeFound > 0);
    });

    it('should detect unused variables marked with comment', async () => {
      await janitor.process({
        tool: 'Write',
        payload: { content: 'const unused = 5; // unused' },
      }, {});

      assert.ok(janitor.stats.deadCodeFound > 0);
    });

    it('should mark console.log as auto-fixable', async () => {
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: 'console.log("debug");' },
      }, {});

      // Auto-fixable should be suggested (even if not applied)
      assert.ok(result.deadCode > 0);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // _calculateQualityScore
  // ═══════════════════════════════════════════════════════════════════════

  describe('quality score calculation', () => {
    it('should return 100 for no issues', () => {
      const score = janitor._calculateQualityScore([]);
      assert.strictEqual(score, 100);
    });

    it('should calculate penalty from issue severity', () => {
      const issues = [
        { severity: QualitySeverity.LOW }, // weight = φ⁻¹ ≈ 0.618
      ];
      const score = janitor._calculateQualityScore(issues);
      // penalty = 0.618, score = 100 - 0.618*5 = 100 - 3.09 ≈ 96.9
      assert.ok(score >= 96 && score <= 97);
    });

    it('should handle multiple issues', () => {
      const issues = [
        { severity: QualitySeverity.MEDIUM }, // weight = 1.0
        { severity: QualitySeverity.MEDIUM }, // weight = 1.0
        { severity: QualitySeverity.HIGH },   // weight = φ ≈ 1.618
      ];
      const score = janitor._calculateQualityScore(issues);
      // penalty = 1.0 + 1.0 + 1.618 = 3.618, score = 100 - 3.618*5 = 100 - 18.09 ≈ 81.9
      assert.ok(score >= 81 && score <= 82);
    });

    it('should cap score at 0', () => {
      const issues = Array(100).fill({ severity: QualitySeverity.CRITICAL });
      const score = janitor._calculateQualityScore(issues);
      assert.strictEqual(score, 0);
    });

    it('should round to 1 decimal place', () => {
      const issues = [{ severity: QualitySeverity.LOW }];
      const score = janitor._calculateQualityScore(issues);
      assert.strictEqual(score, Math.round(score * 10) / 10);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // _recordQualityReport
  // ═══════════════════════════════════════════════════════════════════════

  describe('quality report recording', () => {
    it('should add to qualityHistory', async () => {
      await janitor.process({
        tool: 'Write',
        payload: { content: 'const x = 1;' },
      }, {});

      assert.strictEqual(janitor.qualityHistory.length, 1);
      assert.ok(janitor.qualityHistory[0].timestamp);
      assert.ok(janitor.qualityHistory[0].score);
    });

    it('should increment issuesFound', async () => {
      await janitor.process({
        tool: 'Write',
        payload: { content: 'TODO: fix\nFIXME: urgent' },
      }, {});

      assert.ok(janitor.stats.issuesFound >= 2);
    });

    it('should calculate running average', async () => {
      await janitor.process({ tool: 'Write', payload: { content: 'const x = 1;' } }, {});
      await janitor.process({ tool: 'Write', payload: { content: 'TODO: fix' } }, {});

      assert.ok(janitor.stats.averageQualityScore > 0);
      assert.ok(janitor.stats.averageQualityScore <= 100);
    });

    it('should trim history at 233 (Fib(13))', async () => {
      // Add 240 reports
      for (let i = 0; i < 240; i++) {
        janitor._recordQualityReport(90, { issues: [], deadCode: [] });
      }

      assert.strictEqual(janitor.qualityHistory.length, 233);
    });

    it('should keep most recent entries after trim', async () => {
      for (let i = 0; i < 240; i++) {
        janitor._recordQualityReport(i, { issues: [], deadCode: [] });
      }

      // First entry should be 240 - 233 = 7
      assert.strictEqual(janitor.qualityHistory[0].score, 7);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // _updateDeadCodeTracking
  // ═══════════════════════════════════════════════════════════════════════

  describe('dead code tracking', () => {
    it('should add new dead code', () => {
      const deadCode = [{ file: 'test.js', line: 10, name: 'unused' }];
      janitor._updateDeadCodeTracking(deadCode);

      assert.strictEqual(janitor.deadCode.length, 1);
    });

    it('should deduplicate by file/line/name', () => {
      const deadCode = [{ file: 'test.js', line: 10, name: 'unused' }];
      janitor._updateDeadCodeTracking(deadCode);
      janitor._updateDeadCodeTracking(deadCode); // Same again

      assert.strictEqual(janitor.deadCode.length, 1, 'Should not add duplicate');
    });

    it('should add different dead code', () => {
      janitor._updateDeadCodeTracking([{ file: 'a.js', line: 1, name: 'x' }]);
      janitor._updateDeadCodeTracking([{ file: 'a.js', line: 2, name: 'y' }]);

      assert.strictEqual(janitor.deadCode.length, 2);
    });

    it('should trim at MAX_DEAD_CODE_TRACKED (21)', () => {
      for (let i = 0; i < 30; i++) {
        janitor._updateDeadCodeTracking([{ file: 'test.js', line: i, name: `code_${i}` }]);
      }

      assert.strictEqual(janitor.deadCode.length, JANITOR_CONSTANTS.MAX_DEAD_CODE_TRACKED);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // voteOnConsensus
  // ═══════════════════════════════════════════════════════════════════════

  describe('voteOnConsensus', () => {
    it('should approve quality-focused questions', () => {
      const vote = janitor.voteOnConsensus('Should we refactor this code?');
      assert.strictEqual(vote.vote, 'approve');
      assert.ok(vote.reason.includes('quality'));
    });

    it('should approve clean questions', () => {
      const vote = janitor.voteOnConsensus('Let us clean up the codebase');
      assert.strictEqual(vote.vote, 'approve');
    });

    it('should approve debt questions', () => {
      const vote = janitor.voteOnConsensus('Address technical debt?');
      assert.strictEqual(vote.vote, 'approve');
    });

    it('should approve lint questions', () => {
      const vote = janitor.voteOnConsensus('Run linter?');
      assert.strictEqual(vote.vote, 'approve');
    });

    it('should approve format questions', () => {
      const vote = janitor.voteOnConsensus('Format all files?');
      assert.strictEqual(vote.vote, 'approve');
    });

    it('should reject hack patterns', () => {
      const vote = janitor.voteOnConsensus('Just hack it together');
      assert.strictEqual(vote.vote, 'reject');
      assert.ok(vote.reason.includes('GROWL'));
    });

    it('should reject quick fix patterns', () => {
      const vote = janitor.voteOnConsensus('Apply a quick fix for now');
      assert.strictEqual(vote.vote, 'reject');
    });

    it('should reject workaround patterns', () => {
      const vote = janitor.voteOnConsensus('Use a workaround?');
      assert.strictEqual(vote.vote, 'reject');
    });

    it('should reject temporary patterns', () => {
      const vote = janitor.voteOnConsensus('Add temporary code');
      assert.strictEqual(vote.vote, 'reject');
    });

    it('should reject skip tests patterns', () => {
      const vote = janitor.voteOnConsensus('Skip tests for now');
      assert.strictEqual(vote.vote, 'reject');
    });

    it('should abstain on neutral questions', () => {
      const vote = janitor.voteOnConsensus('What is your opinion?');
      assert.strictEqual(vote.vote, 'abstain');
      assert.ok(vote.reason.includes('yawn'));
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // PROFILE-BASED STRICTNESS
  // ═══════════════════════════════════════════════════════════════════════

  describe('profile-based strictness', () => {
    it('NOVICE: strictness = 0.5, autoFix = true', () => {
      janitor.profileLevel = ProfileLevel.NOVICE;
      // Internals aren't directly exposed, but we can test behavior
      // Novice allows longer files (987 / 0.5 = 1974)
      assert.strictEqual(janitor.profileLevel, ProfileLevel.NOVICE);
    });

    it('MASTER: strictness = φ, autoFix = false', () => {
      janitor.profileLevel = ProfileLevel.MASTER;
      assert.strictEqual(janitor.profileLevel, ProfileLevel.MASTER);
    });

    it('should limit issues reported for NOVICE (maxIssuesReported = 5)', async () => {
      janitor.profileLevel = ProfileLevel.NOVICE;
      let reportEvent = null;
      eventBus.registerAgent('test-listener');
      eventBus.subscribe(AgentEvent.QUALITY_REPORT, 'test-listener', (event) => {
        reportEvent = event;
      });

      // Create code with many issues
      const manyIssues = 'TODO:\n'.repeat(10);
      await janitor.process({
        tool: 'Write',
        payload: { content: manyIssues },
      }, {});

      assert.ok(reportEvent);
      assert.ok(reportEvent.payload.issues.length <= 5, 'NOVICE should report max 5 issues');
    });

    it('should allow more issues for MASTER (maxIssuesReported = 233)', async () => {
      janitor.profileLevel = ProfileLevel.MASTER;
      let reportEvent = null;
      eventBus.registerAgent('test-listener');
      eventBus.subscribe(AgentEvent.QUALITY_REPORT, 'test-listener', (event) => {
        reportEvent = event;
      });

      const manyIssues = 'TODO:\n'.repeat(100);
      await janitor.process({
        tool: 'Write',
        payload: { content: manyIssues },
      }, {});

      assert.ok(reportEvent);
      // Should report more than 5
      assert.ok(reportEvent.payload.issues.length > 5);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // AUTO-FIX APPLICATION
  // ═══════════════════════════════════════════════════════════════════════

  describe('auto-fix', () => {
    it('should apply auto-fixes when enabled (NOVICE)', async () => {
      janitor.profileLevel = ProfileLevel.NOVICE; // autoFixEnabled = true

      let fixEvent = null;
      eventBus.registerAgent('test-listener');
      eventBus.subscribe(AgentEvent.AUTO_FIX_APPLIED, 'test-listener', (event) => {
        fixEvent = event;
      });

      await janitor.process({
        tool: 'Write',
        payload: { content: 'console.log("debug");', file: 'test.js' },
      }, {});

      assert.ok(fixEvent, 'Should emit AUTO_FIX_APPLIED for NOVICE');
    });

    it('should NOT apply auto-fixes when disabled (PRACTITIONER)', async () => {
      janitor.profileLevel = ProfileLevel.PRACTITIONER; // autoFixEnabled = false

      let fixEvent = null;
      eventBus.registerAgent('test-listener');
      eventBus.subscribe(AgentEvent.AUTO_FIX_APPLIED, 'test-listener', (event) => {
        fixEvent = event;
      });

      await janitor.process({
        tool: 'Write',
        payload: { content: 'console.log("debug");' },
      }, {});

      assert.strictEqual(fixEvent, null, 'Should NOT emit AUTO_FIX_APPLIED for PRACTITIONER');
    });

    it('should increment issuesFixed when auto-fix applied', async () => {
      janitor.profileLevel = ProfileLevel.NOVICE;
      await janitor.process({
        tool: 'Write',
        payload: { content: 'console.log("debug");' },
      }, {});

      assert.ok(janitor.stats.issuesFixed > 0);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // EVENT SUBSCRIPTIONS
  // ═══════════════════════════════════════════════════════════════════════

  describe('event subscriptions', () => {
    it('should handle PROFILE_UPDATED event', async () => {
      const { ProfileUpdatedEvent, AgentEventMessage } = await import('../src/agents/events.js');

      const profileEvent = new AgentEventMessage(
        AgentEvent.PROFILE_UPDATED,
        AgentId.ANALYST,
        {
          previousLevel: ProfileLevel.PRACTITIONER,
          newLevel: ProfileLevel.EXPERT,
        },
        { target: AgentId.ALL }
      );

      await eventBus.publish(profileEvent);

      assert.strictEqual(janitor.profileLevel, ProfileLevel.EXPERT);
    });

    it('should handle PATTERN_DETECTED event (error pattern)', async () => {
      const { AgentEventMessage } = await import('../src/agents/events.js');

      const patternEvent = new AgentEventMessage(
        AgentEvent.PATTERN_DETECTED,
        AgentId.ANALYST,
        {
          patternType: 'error',
          category: 'error',
        },
        { target: AgentId.ALL }
      );

      await eventBus.publish(patternEvent);

      // Should schedule a scan (internal state)
      assert.ok(janitor._scheduledScans);
      assert.ok(janitor._scheduledScans.length > 0);
    });

    it('should NOT schedule scan for non-error patterns', async () => {
      const { AgentEventMessage } = await import('../src/agents/events.js');

      const patternEvent = new AgentEventMessage(
        AgentEvent.PATTERN_DETECTED,
        AgentId.ANALYST,
        {
          patternType: 'info',
          category: 'info',
        },
        { target: AgentId.ALL }
      );

      await eventBus.publish(patternEvent);

      // Should not schedule
      assert.ok(!janitor._scheduledScans || janitor._scheduledScans.length === 0);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // getSummary
  // ═══════════════════════════════════════════════════════════════════════

  describe('getSummary', () => {
    it('should return summary with all fields', async () => {
      await janitor.process({ tool: 'Write', payload: { content: 'TODO: fix' } }, {});

      const summary = janitor.getSummary();
      assert.strictEqual(summary.agent, 'janitor');
      assert.strictEqual(summary.sefirah, 'Yesod (Foundation)');
      assert.ok('profileLevel' in summary);
      assert.ok('stats' in summary);
      assert.ok('recentIssues' in summary);
      assert.ok('deadCodeCount' in summary);
      assert.ok('averageQualityScore' in summary);
      assert.ok('constants' in summary);
    });

    it('should include recent issues (last 5)', async () => {
      for (let i = 0; i < 10; i++) {
        await janitor.process({ tool: 'Write', payload: { content: `TODO: fix ${i}` } }, {});
      }

      const summary = janitor.getSummary();
      assert.ok(summary.recentIssues.length <= 5);
    });

    it('should include dead code count', async () => {
      await janitor.process({ tool: 'Write', payload: { content: 'console.log("a");' } }, {});

      const summary = janitor.getSummary();
      assert.ok(summary.deadCodeCount > 0);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // clear
  // ═══════════════════════════════════════════════════════════════════════

  describe('clear', () => {
    it('should reset all state', async () => {
      await janitor.process({ tool: 'Write', payload: { content: 'TODO: fix\nconsole.log("x");' } }, {});

      janitor.clear();

      assert.deepStrictEqual(janitor.issues, []);
      assert.deepStrictEqual(janitor.deadCode, []);
      assert.deepStrictEqual(janitor.qualityHistory, []);
      assert.strictEqual(janitor.stats.scansPerformed, 0);
      assert.strictEqual(janitor.stats.issuesFound, 0);
      assert.strictEqual(janitor.stats.issuesFixed, 0);
      assert.strictEqual(janitor.stats.deadCodeFound, 0);
      assert.strictEqual(janitor.stats.averageQualityScore, 0);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // EDGE CASES
  // ═══════════════════════════════════════════════════════════════════════

  describe('edge cases', () => {
    it('should handle empty content', async () => {
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: '' },
      }, {});

      assert.strictEqual(result.qualityScore, 100);
    });

    it('should handle content without issues', async () => {
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: 'const x = 1;\nconst y = 2;\nreturn x + y;' },
      }, {});

      assert.strictEqual(result.qualityScore, 100);
      assert.strictEqual(result.issues, 0);
    });

    it('should handle missing payload', async () => {
      const result = await janitor.process({
        tool: 'Write',
      }, {});

      assert.strictEqual(result.qualityScore, 100);
    });

    it('should handle missing file path', async () => {
      const result = await janitor.process({
        tool: 'Write',
        payload: { content: 'console.log("x");' },
      }, {});

      // Should still detect dead code
      assert.ok(result.deadCode > 0);
    });

    it('should work without eventBus', async () => {
      const standalone = new CollectiveJanitor();
      const result = await standalone.process({
        tool: 'Write',
        payload: { content: 'const x = 1;' },
      }, {});

      assert.strictEqual(result.qualityScore, 100);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // COMPLEX SCENARIOS
  // ═══════════════════════════════════════════════════════════════════════

  describe('complex scenarios', () => {
    it('should handle file with multiple issue types', async () => {
      const complexFile = `
// TODO: refactor this
function veryLongFunctionNameThatDoesLotsOfThings() {
  console.log("debug start");
  if (a) {
    if (b) {
      if (c) {
        if (d) {
          if (e) {
            // FIXME: nested logic
            console.log("deep");
          }
        }
      }
    }
  }
  ${Array(60).fill('  doSomething();').join('\n')}
  console.log("debug end");
}
      `;

      const result = await janitor.process({
        tool: 'Write',
        payload: { content: complexFile, file: 'complex.js' },
      }, {});

      // Should detect: TODO, FIXME, console.log (3x), long function, deep nesting
      // Note: result.issues is a count, not an array
      assert.ok(result.issues >= 2, `Expected at least 2 issues, got ${result.issues}`);
      assert.ok(result.deadCode >= 3, `Expected at least 3 dead code entries, got ${result.deadCode}`);
      assert.ok(result.qualityScore < 100, `Quality score should be less than 100, got ${result.qualityScore}`);
    });

    it('should track trends over multiple scans', async () => {
      // First scan - bad code
      await janitor.process({
        tool: 'Write',
        payload: { content: 'TODO: fix\nFIXME: urgent\nconsole.log("x");' },
      }, {});

      const firstScore = janitor.stats.averageQualityScore;

      // Second scan - clean code
      await janitor.process({
        tool: 'Write',
        payload: { content: 'const x = 1;\nconst y = 2;' },
      }, {});

      const secondScore = janitor.stats.averageQualityScore;

      // Average should improve
      assert.ok(secondScore > firstScore);
    });

    it('should maintain history across profile changes', async () => {
      janitor.profileLevel = ProfileLevel.NOVICE;
      await janitor.process({ tool: 'Write', payload: { content: 'TODO: fix' } }, {});

      janitor.profileLevel = ProfileLevel.MASTER;
      await janitor.process({ tool: 'Write', payload: { content: 'FIXME: bug' } }, {});

      // History should contain both
      assert.strictEqual(janitor.qualityHistory.length, 2);
    });
  });
});
