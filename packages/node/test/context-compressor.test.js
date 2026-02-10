/**
 * ContextCompressor Tests
 *
 * Tests that CYNIC's context injection reduces over time:
 * - Session 1: Full context (learning)
 * - Session 100: Minimal context (expert)
 *
 * "Le chien qui sait n'a pas besoin qu'on lui répète" — κυνικός
 *
 * @module @cynic/node/test/context-compressor
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { mkdirSync, existsSync, readFileSync, rmSync } from 'node:fs';

import {
  contextCompressor,
  TOPIC_CONFIG,
  EXPERIENCED_THRESHOLD,
  EXPERT_THRESHOLD,
  BACKOFF_WINDOW,
  BACKOFF_QUALITY_THRESHOLD,
  BACKOFF_DURATION,
  MAX_OUTCOMES,
} from '../src/services/context-compressor.js';

// ═══════════════════════════════════════════════════════════════════════════
// TEST HELPERS
// ═══════════════════════════════════════════════════════════════════════════

function getTempStatePath() {
  const dir = join(tmpdir(), `cynic-compressor-test-${Date.now()}-${Math.random().toString(36).slice(2)}`);
  mkdirSync(dir, { recursive: true });
  return join(dir, 'compressor-state.json');
}

const SAMPLE_FRAMING = [
  '── 🧠 CYNIC FRAME ────────────────────────────────────────',
  '   D = 45% [███████░░░] awake',
  '   Axioms: PHI × VERIFY × CULTURE (3/5)',
  '   Lead: 🛡️ Guardian (Gevurah) — protect mode',
  '   Route: security_detected | tier: haiku',
  '   Votes: Guardian approve(58%), Sage approve(52%), Scout approve(48%) [consensus]',
  '   Conscience: score 62/100, trend →stable, ok',
  '   Distribution: 3 builders, 7 repos, 2/3 services',
  '   Social: 47 tweets captured, 12 users',
  '   Accounting: dogs: 5 ops | code: 3 changes',
  '   Frame: VERIFY: Trust nothing. Prove everything.',
  '   Memory: "recurring security pattern" (4x)',
  '   Depth: Deep | User: experienced',
].join('\n');

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('ContextCompressor', () => {
  let tempPath;

  beforeEach(() => {
    contextCompressor._resetForTesting();
    tempPath = getTempStatePath();
    contextCompressor._setStatePath(tempPath);
  });

  afterEach(() => {
    contextCompressor._resetForTesting();
    try {
      const dir = join(tempPath, '..');
      if (existsSync(dir)) rmSync(dir, { recursive: true });
    } catch { /* ignore */ }
  });

  // ═══════════════════════════════════════════════════════════════════════
  // LIFECYCLE
  // ═══════════════════════════════════════════════════════════════════════

  describe('Lifecycle', () => {
    it('starts and stops cleanly', () => {
      contextCompressor.start();
      assert.equal(contextCompressor.getStats().running, true);

      contextCompressor.stop();
      assert.equal(contextCompressor.getStats().running, false);
    });

    it('increments session count on start', () => {
      contextCompressor.start();
      assert.equal(contextCompressor.getStats().totalSessions, 1);
    });

    it('persists state on stop', () => {
      contextCompressor.start();
      contextCompressor.shouldInject('framing_directive', { estimatedChars: 500 });
      contextCompressor.stop();

      assert.ok(existsSync(tempPath), 'State file should exist');
      const state = JSON.parse(readFileSync(tempPath, 'utf-8'));
      assert.equal(state.totalSessions, 1);
      assert.equal(state.totalInjections, 1);
    });

    it('loads state on restart', () => {
      contextCompressor.start();
      contextCompressor.shouldInject('framing_directive');
      contextCompressor.stop();

      contextCompressor._resetForTesting();
      contextCompressor._setStatePath(tempPath);
      contextCompressor.start();

      assert.equal(contextCompressor.getStats().totalSessions, 2);
      assert.equal(contextCompressor.getStats().lifetime.totalInjections, 1);
    });

    it('resets for testing', () => {
      contextCompressor.start();
      contextCompressor.shouldInject('framing_directive');
      contextCompressor._resetForTesting();

      assert.equal(contextCompressor.getStats().running, false);
      assert.equal(contextCompressor.getStats().totalSessions, 0);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // SHOULD INJECT
  // ═══════════════════════════════════════════════════════════════════════

  describe('shouldInject', () => {
    it('always injects when not running', () => {
      const result = contextCompressor.shouldInject('framing_directive');
      assert.equal(result.inject, true);
      assert.equal(result.reason, 'compressor_not_running');
    });

    it('always injects unknown topics', () => {
      contextCompressor.start();
      const result = contextCompressor.shouldInject('totally_unknown_topic');
      assert.equal(result.inject, true);
      assert.equal(result.reason, 'unknown_topic');
    });

    it('always injects when forced', () => {
      contextCompressor.start();
      const result = contextCompressor.shouldInject('complexity_analysis', { force: true });
      assert.equal(result.inject, true);
      assert.equal(result.reason, 'forced');
    });

    it('disables complexity_analysis immediately (session 0)', () => {
      contextCompressor.start();
      const result = contextCompressor.shouldInject('complexity_analysis');
      assert.equal(result.inject, false);
      assert.equal(result.reason, 'disabled');
    });

    it('disables optimize_analysis immediately (session 0)', () => {
      contextCompressor.start();
      const result = contextCompressor.shouldInject('optimize_analysis');
      assert.equal(result.inject, false);
      assert.equal(result.reason, 'disabled');
    });

    it('allows framing_directive (no staleTTL)', () => {
      contextCompressor.start();
      const r1 = contextCompressor.shouldInject('framing_directive');
      assert.equal(r1.inject, true);
      // Immediate re-inject: staleTTL is 0 so should still inject
      const r2 = contextCompressor.shouldInject('framing_directive');
      assert.equal(r2.inject, true);
    });

    it('blocks ecosystem_status within staleTTL', () => {
      contextCompressor.start();
      const r1 = contextCompressor.shouldInject('ecosystem_status');
      assert.equal(r1.inject, true);

      // Immediately — within 5min TTL
      const r2 = contextCompressor.shouldInject('ecosystem_status');
      assert.equal(r2.inject, false);
      assert.equal(r2.reason, 'stale_ttl');
    });

    it('blocks social_status within staleTTL', () => {
      contextCompressor.start();
      contextCompressor.shouldInject('social_status');
      const r2 = contextCompressor.shouldInject('social_status');
      assert.equal(r2.inject, false);
    });

    it('tracks chars saved on skip', () => {
      contextCompressor.start();
      contextCompressor.shouldInject('complexity_analysis', { estimatedChars: 300 });
      assert.equal(contextCompressor.getStats().session.charsSaved, 300);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // EXPERIENCE CURVE
  // ═══════════════════════════════════════════════════════════════════════

  describe('Experience Curve', () => {
    it('reports "new" for first 3 sessions', () => {
      contextCompressor.start();
      assert.equal(contextCompressor.getStats().experienceLevel, 'new');
    });

    it('reports "learning" after 4 sessions', () => {
      contextCompressor.start();
      contextCompressor._totalSessions = 5;
      assert.equal(contextCompressor.getStats().experienceLevel, 'learning');
    });

    it('reports "experienced" after threshold', () => {
      contextCompressor.start();
      contextCompressor._totalSessions = EXPERIENCED_THRESHOLD + 1;
      assert.equal(contextCompressor.getStats().experienceLevel, 'experienced');
    });

    it('reports "expert" after expert threshold', () => {
      contextCompressor.start();
      contextCompressor._totalSessions = EXPERT_THRESHOLD + 1;
      assert.equal(contextCompressor.getStats().experienceLevel, 'expert');
    });

    it('compression level increases with sessions', () => {
      contextCompressor.start();
      const newLevel = contextCompressor.getStats().compressionLevel;

      contextCompressor._totalSessions = EXPERT_THRESHOLD;
      const expertLevel = contextCompressor.getStats().compressionLevel;

      assert.ok(expertLevel > newLevel, 'Expert should compress more than new');
    });

    it('compression level capped at φ⁻¹ (62%)', () => {
      contextCompressor.start();
      contextCompressor._totalSessions = 1000;
      // Add max maturity signals
      contextCompressor.reportMaturity('router', { maturity: 1.0, converged: true });
      contextCompressor.reportMaturity('thompson', { maturity: 1.0, converged: true });
      assert.ok(contextCompressor.getStats().compressionLevel <= 62);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // COMPRESSION
  // ═══════════════════════════════════════════════════════════════════════

  describe('Compression', () => {
    it('returns full data for new sessions', () => {
      contextCompressor.start();
      contextCompressor._totalSessions = 1;
      const result = contextCompressor.compress('framing_directive', SAMPLE_FRAMING);
      assert.equal(result, SAMPLE_FRAMING);
    });

    it('compresses framing directive for experienced users', () => {
      contextCompressor.start();
      contextCompressor._totalSessions = EXPERT_THRESHOLD;
      contextCompressor.reportMaturity('router', { maturity: 0.5, converged: true });

      const result = contextCompressor.compress('framing_directive', SAMPLE_FRAMING);
      assert.ok(result.length < SAMPLE_FRAMING.length, 'Compressed should be shorter');
      // Should still contain D value and Frame
      assert.ok(result.includes('D ='), 'Should keep D value');
      assert.ok(result.includes('Frame:'), 'Should keep Frame');
    });

    it('keeps header and D in compressed framing', () => {
      contextCompressor.start();
      contextCompressor._totalSessions = EXPERT_THRESHOLD + 10;
      contextCompressor.reportMaturity('router', { maturity: 0.6, converged: true });

      const result = contextCompressor.compress('framing_directive', SAMPLE_FRAMING);
      assert.ok(result.includes('CYNIC FRAME'), 'Should keep header');
      assert.ok(result.includes('D ='), 'Should keep D');
    });

    it('returns non-compressible topics unchanged', () => {
      contextCompressor.start();
      contextCompressor._totalSessions = EXPERT_THRESHOLD;
      const input = 'Error: something failed';
      const result = contextCompressor.compress('error_perception', input);
      assert.equal(result, input);
    });

    it('returns null/undefined unchanged', () => {
      contextCompressor.start();
      assert.equal(contextCompressor.compress('framing_directive', null), null);
      assert.equal(contextCompressor.compress('framing_directive', undefined), undefined);
    });

    it('compresses routing when stable', () => {
      contextCompressor.start();
      contextCompressor._totalSessions = EXPERT_THRESHOLD;
      contextCompressor.reportMaturity('router', { maturity: 0.5, converged: true });

      // Simulate stable routing
      contextCompressor.reportRouting('Guardian', 'security');
      contextCompressor.reportRouting('Guardian', 'security');
      contextCompressor.reportRouting('Guardian', 'security');

      const fullRouting = '   Lead: 🛡️ Guardian (Gevurah) — protect mode\n   Votes: Guardian approve(58%)';
      const result = contextCompressor.compress('dog_routing', fullRouting);
      assert.ok(result.includes('stable'), 'Should indicate stable routing');
      assert.ok(result.length < fullRouting.length);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // MATURITY SIGNALS
  // ═══════════════════════════════════════════════════════════════════════

  describe('Maturity Signals', () => {
    it('accepts maturity reports', () => {
      contextCompressor.start();
      contextCompressor.reportMaturity('router', { maturity: 0.5, converged: false });
      const stats = contextCompressor.getStats();
      assert.ok(stats.maturitySignals.router);
      assert.equal(stats.maturitySignals.router.maturity, 0.5);
    });

    it('φ-bounds maturity at 61.8%', () => {
      contextCompressor.start();
      contextCompressor.reportMaturity('router', { maturity: 0.99, converged: true });
      const stats = contextCompressor.getStats();
      assert.ok(stats.maturitySignals.router.maturity <= 0.618034);
    });

    it('overall maturity aggregates signals', () => {
      contextCompressor.start();
      contextCompressor.reportMaturity('router', { maturity: 0.4, converged: false });
      contextCompressor.reportMaturity('thompson', { maturity: 0.6, converged: true });
      const overall = contextCompressor.getOverallMaturity();
      assert.ok(overall > 0.4 && overall < 0.6, `Expected between 0.4-0.6, got ${overall}`);
    });

    it('overall maturity is 0 with no signals', () => {
      contextCompressor.start();
      assert.equal(contextCompressor.getOverallMaturity(), 0);
    });

    it('persists maturity signals across sessions (cross-process)', () => {
      // Session 1: daemon writes maturity signals
      contextCompressor.start();
      contextCompressor.reportMaturity('thompson', { maturity: 0.45, converged: false });
      contextCompressor.reportMaturity('router', { maturity: 0.55, converged: true });
      contextCompressor.stop();

      // Verify state file includes maturity signals
      const raw = readFileSync(tempPath, 'utf-8');
      const state = JSON.parse(raw);
      assert.ok(state.maturitySignals, 'maturitySignals missing from persisted state');
      assert.ok(state.maturitySignals.thompson, 'thompson signal missing');
      assert.equal(state.maturitySignals.thompson.maturity, 0.45);
      assert.ok(state.maturitySignals.router, 'router signal missing');
      assert.equal(state.maturitySignals.router.converged, true);

      // Session 2: hook process reads maturity signals
      contextCompressor._resetForTesting();
      contextCompressor._setStatePath(tempPath);
      contextCompressor.start();

      const stats = contextCompressor.getStats();
      assert.ok(stats.maturitySignals.thompson, 'thompson not restored');
      assert.equal(stats.maturitySignals.thompson.maturity, 0.45);
      assert.ok(stats.maturitySignals.router, 'router not restored');
      assert.equal(stats.maturitySignals.router.converged, true);

      // Overall maturity should reflect restored signals
      const overall = contextCompressor.getOverallMaturity();
      assert.ok(overall > 0.4, `Expected > 0.4, got ${overall}`);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // ROUTING STABILITY
  // ═══════════════════════════════════════════════════════════════════════

  describe('Routing Stability', () => {
    it('detects stable routing after 3 same lead', () => {
      contextCompressor.start();
      assert.equal(contextCompressor.isRoutingStable(), false);

      contextCompressor.reportRouting('Guardian', 'security');
      contextCompressor.reportRouting('Guardian', 'security');
      assert.equal(contextCompressor.isRoutingStable(), false);

      contextCompressor.reportRouting('Guardian', 'security');
      assert.equal(contextCompressor.isRoutingStable(), true);
    });

    it('resets stability on dog change', () => {
      contextCompressor.start();
      contextCompressor.reportRouting('Guardian', 'security');
      contextCompressor.reportRouting('Guardian', 'security');
      contextCompressor.reportRouting('Guardian', 'security');
      assert.equal(contextCompressor.isRoutingStable(), true);

      contextCompressor.reportRouting('Architect', 'architecture');
      assert.equal(contextCompressor.isRoutingStable(), false);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // STATS
  // ═══════════════════════════════════════════════════════════════════════

  describe('Stats', () => {
    it('reports complete stats', () => {
      contextCompressor.start();
      const stats = contextCompressor.getStats();
      assert.equal(stats.running, true);
      assert.ok(stats.uptime >= 0);
      assert.ok(stats.session);
      assert.ok(stats.lifetime);
      assert.ok(stats.topics);
    });

    it('tracks session compression ratio', () => {
      contextCompressor.start();
      // 1 inject, 1 skip
      contextCompressor.shouldInject('framing_directive');
      contextCompressor.shouldInject('complexity_analysis');

      const stats = contextCompressor.getStats();
      assert.equal(stats.session.injections, 1);
      assert.equal(stats.session.skips, 1);
      assert.equal(stats.session.compressionRatio, 50);
    });

    it('tracks topic-level stats', () => {
      contextCompressor.start();
      contextCompressor.shouldInject('framing_directive', { estimatedChars: 800 });
      contextCompressor.shouldInject('framing_directive', { estimatedChars: 800 });

      const stats = contextCompressor.getStats();
      assert.equal(stats.topics.framing_directive.count, 2);
      assert.equal(stats.topics.framing_directive.totalChars, 1600);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // TOPIC CONFIG
  // ═══════════════════════════════════════════════════════════════════════

  describe('Topic Configuration', () => {
    it('has SILENT tools disabled at session 0', () => {
      assert.equal(TOPIC_CONFIG.complexity_analysis.disableAfter, 0);
      assert.equal(TOPIC_CONFIG.optimize_analysis.disableAfter, 0);
    });

    it('has awareness topics with 5min TTL', () => {
      assert.equal(TOPIC_CONFIG.ecosystem_status.staleTTL, 300_000);
      assert.equal(TOPIC_CONFIG.social_status.staleTTL, 300_000);
      assert.equal(TOPIC_CONFIG.accounting_status.staleTTL, 300_000);
    });

    it('has framing_directive compressible', () => {
      assert.equal(TOPIC_CONFIG.framing_directive.compressible, true);
    });

    it('has error_perception not compressible', () => {
      assert.equal(TOPIC_CONFIG.error_perception.compressible, false);
    });

    it('all topics have required fields', () => {
      for (const [name, config] of Object.entries(TOPIC_CONFIG)) {
        assert.ok(typeof config.staleTTL === 'number', `${name} missing staleTTL`);
        assert.ok(typeof config.compressible === 'boolean', `${name} missing compressible`);
        assert.ok('disableAfter' in config, `${name} missing disableAfter`);
      }
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // OUTCOME VERIFICATION (Circuit Breaker)
  // ═══════════════════════════════════════════════════════════════════════════

  describe('Outcome Verification', () => {
    beforeEach(() => {
      contextCompressor._resetForTesting();
      tempPath = getTempStatePath();
      contextCompressor._setStatePath(tempPath);
    });

    it('records session outcome', () => {
      contextCompressor.start();
      contextCompressor.recordSessionOutcome({ quality: 0.8, errorRate: 0.1, frustration: 0.1 });
      const outcomes = contextCompressor.getSessionOutcomes();
      assert.equal(outcomes.length, 1);
      assert.equal(outcomes[0].quality, 0.8);
      assert.equal(outcomes[0].errorRate, 0.1);
    });

    it('keeps rolling window of MAX_OUTCOMES', () => {
      contextCompressor.start();
      for (let i = 0; i < MAX_OUTCOMES + 5; i++) {
        contextCompressor.recordSessionOutcome({ quality: 0.5 + (i * 0.01), errorRate: 0, frustration: 0 });
      }
      const outcomes = contextCompressor.getSessionOutcomes();
      assert.equal(outcomes.length, MAX_OUTCOMES);
    });

    it('no backoff when quality is high', () => {
      contextCompressor.start();
      // Record good sessions
      for (let i = 0; i < BACKOFF_WINDOW; i++) {
        contextCompressor.recordSessionOutcome({ quality: 0.9, errorRate: 0.05, frustration: 0.05 });
      }
      const status = contextCompressor.getBackoffStatus();
      assert.equal(status.active, false);
      assert.equal(status.remaining, 0);
    });

    it('triggers backoff when quality is consistently low', () => {
      contextCompressor.start();
      // All sessions at 'new' level with bad quality
      for (let i = 0; i < BACKOFF_WINDOW; i++) {
        contextCompressor.recordSessionOutcome({ quality: 0.2, errorRate: 0.5, frustration: 0.8 });
      }
      const status = contextCompressor.getBackoffStatus();
      assert.equal(status.active, true);
      assert.ok(status.remaining > 0);
      assert.equal(status.reason, 'quality_degradation');
    });

    it('backoff degrades experience level by 1', () => {
      contextCompressor._resetForTesting();
      contextCompressor._setStatePath(tempPath);
      // Simulate experienced user (sessions > EXPERIENCED_THRESHOLD)
      contextCompressor._totalSessions = EXPERIENCED_THRESHOLD + 1;
      contextCompressor._running = true;

      // Raw level should be 'experienced'
      assert.equal(contextCompressor._getExperienceLevel(), 'experienced');

      // Record bad sessions at 'experienced' level
      for (let i = 0; i < BACKOFF_WINDOW; i++) {
        contextCompressor.recordSessionOutcome({ quality: 0.1, errorRate: 0.6, frustration: 0.9 });
      }

      // Effective level should be degraded to 'learning'
      assert.equal(contextCompressor.getEffectiveExperienceLevel(), 'learning');
      assert.equal(contextCompressor.getBackoffStatus().rawLevel, 'experienced');
    });

    it('backoff expires after BACKOFF_DURATION sessions', () => {
      contextCompressor._totalSessions = EXPERT_THRESHOLD + 1;
      contextCompressor._running = true;

      // Raw level should be 'expert'
      assert.equal(contextCompressor._getExperienceLevel(), 'expert');

      // Trigger backoff
      for (let i = 0; i < BACKOFF_WINDOW; i++) {
        contextCompressor.recordSessionOutcome({ quality: 0.1, errorRate: 0.6, frustration: 0.9 });
      }
      assert.equal(contextCompressor.getBackoffStatus().active, true);
      assert.equal(contextCompressor.getEffectiveExperienceLevel(), 'experienced');

      // Advance past backoff duration
      contextCompressor._totalSessions += BACKOFF_DURATION + 1;
      assert.equal(contextCompressor.getBackoffStatus().active, false);
      assert.equal(contextCompressor.getEffectiveExperienceLevel(), 'expert');
    });

    it('cannot degrade below new level', () => {
      contextCompressor.start();
      assert.equal(contextCompressor._getExperienceLevel(), 'new');

      // Record terrible sessions
      for (let i = 0; i < BACKOFF_WINDOW; i++) {
        contextCompressor.recordSessionOutcome({ quality: 0.0, errorRate: 1.0, frustration: 1.0 });
      }

      // Still 'new' — can't go lower
      assert.equal(contextCompressor.getEffectiveExperienceLevel(), 'new');
    });

    it('getStats includes backoff info', () => {
      contextCompressor.start();
      const stats = contextCompressor.getStats();
      assert.ok('backoff' in stats, 'stats should include backoff');
      assert.equal(stats.backoff.active, false);
    });

    it('persists and restores outcomes across sessions', () => {
      contextCompressor.start();
      contextCompressor.recordSessionOutcome({ quality: 0.7, errorRate: 0.1, frustration: 0.2 });
      contextCompressor.stop();

      // Reset in-memory state and reload
      contextCompressor._resetForTesting();
      contextCompressor._setStatePath(tempPath);
      contextCompressor.start();

      const outcomes = contextCompressor.getSessionOutcomes();
      assert.equal(outcomes.length, 1);
      assert.equal(outcomes[0].quality, 0.7);
    });

    it('persists and restores backoff across sessions', () => {
      contextCompressor._totalSessions = EXPERIENCED_THRESHOLD + 1;
      contextCompressor._running = true;
      contextCompressor._setStatePath(tempPath);

      // Trigger backoff
      for (let i = 0; i < BACKOFF_WINDOW; i++) {
        contextCompressor.recordSessionOutcome({ quality: 0.1, errorRate: 0.6, frustration: 0.9 });
      }
      assert.equal(contextCompressor.getBackoffStatus().active, true);
      contextCompressor._persistState();

      // Reset and reload
      const savedSessions = contextCompressor._totalSessions;
      contextCompressor._resetForTesting();
      contextCompressor._setStatePath(tempPath);
      contextCompressor._totalSessions = savedSessions; // Simulate same session count
      contextCompressor._running = true;
      contextCompressor._loadState();

      assert.equal(contextCompressor.getBackoffStatus().active, true);
    });

    it('quality is clamped to [0, 1]', () => {
      contextCompressor.start();
      contextCompressor.recordSessionOutcome({ quality: -0.5, errorRate: 0, frustration: 0 });
      contextCompressor.recordSessionOutcome({ quality: 1.5, errorRate: 0, frustration: 0 });
      const outcomes = contextCompressor.getSessionOutcomes();
      assert.equal(outcomes[0].quality, 0);
      assert.equal(outcomes[1].quality, 1);
    });

    it('handles missing outcome fields gracefully', () => {
      contextCompressor.start();
      contextCompressor.recordSessionOutcome({});
      const outcomes = contextCompressor.getSessionOutcomes();
      assert.equal(outcomes[0].quality, 0);
      assert.equal(outcomes[0].errorRate, 0);
      assert.equal(outcomes[0].frustration, 0);
    });

    it('only evaluates outcomes at current level for backoff', () => {
      // Start as 'new' with bad quality
      contextCompressor.start();
      for (let i = 0; i < BACKOFF_WINDOW; i++) {
        contextCompressor.recordSessionOutcome({ quality: 0.1, errorRate: 0.5, frustration: 0.5 });
      }
      // Backoff triggered at 'new' level
      assert.equal(contextCompressor.getBackoffStatus().active, true);

      // Advance to 'experienced' level
      contextCompressor._totalSessions = EXPERIENCED_THRESHOLD + 5;
      contextCompressor._backoffUntilSession = 0; // Clear old backoff

      // Record good sessions at 'experienced' level — no backoff
      for (let i = 0; i < BACKOFF_WINDOW; i++) {
        contextCompressor.recordSessionOutcome({ quality: 0.9, errorRate: 0, frustration: 0 });
      }
      assert.equal(contextCompressor.getBackoffStatus().active, false);
    });
  });
});
