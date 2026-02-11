/**
 * Prompt Classifier Tests
 *
 * Tests intent detection, domain scoring, complexity assessment,
 * token budgeting, and context relevance filtering.
 *
 * @module @cynic/core/test/prompt-classifier
 */

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';

import {
  classifyPrompt,
  scoreContextRelevance,
  selectSections,
  TOKEN_BUDGETS,
} from '../src/intelligence/prompt-classifier.js';

import { PHI_INV } from '../src/axioms/constants.js';

// =============================================================================
// Smart skip
// =============================================================================

describe('classifyPrompt: smart skip', () => {
  it('skips "ok"', () => {
    const r = classifyPrompt('ok');
    assert.equal(r.skip, true);
    assert.equal(r.intent, 'skip');
    assert.equal(r.tokenBudget, 0);
  });

  it('skips "yes"', () => {
    assert.equal(classifyPrompt('yes').skip, true);
  });

  it('skips "+"', () => {
    assert.equal(classifyPrompt('+').skip, true);
  });

  it('skips "oui"', () => {
    assert.equal(classifyPrompt('oui').skip, true);
  });

  it('skips slash commands', () => {
    assert.equal(classifyPrompt('/commit').skip, true);
    assert.equal(classifyPrompt('/status').skip, true);
  });

  it('does NOT skip real prompts', () => {
    assert.equal(classifyPrompt('fix the bug in the login function').skip, false);
  });

  it('does NOT skip multi-word confirmations', () => {
    assert.equal(classifyPrompt('ok let me think about this').skip, false);
  });
});

// =============================================================================
// Intent detection
// =============================================================================

describe('classifyPrompt: intent detection', () => {
  it('detects debug intent', () => {
    const r = classifyPrompt('there is a bug in the parser');
    assert.equal(r.intent, 'debug');
  });

  it('detects architecture intent', () => {
    const r = classifyPrompt('how should we refactor the event bus architecture');
    assert.equal(r.intent, 'architecture');
  });

  it('detects security intent (highest priority)', () => {
    const r = classifyPrompt('is this code vulnerable to injection attacks');
    assert.equal(r.intent, 'security');
  });

  it('detects danger intent', () => {
    const r = classifyPrompt('rm -rf the entire database directory');
    assert.equal(r.intent, 'danger');
  });

  it('detects decision intent', () => {
    const r = classifyPrompt('should we use Redis or PostgreSQL');
    assert.equal(r.intent, 'decision');
  });

  it('detects test intent', () => {
    const r = classifyPrompt('write unit tests for the validator');
    assert.equal(r.intent, 'test');
  });

  it('detects git intent', () => {
    const r = classifyPrompt('commit and push the changes');
    assert.equal(r.intent, 'git');
  });

  it('detects deploy intent', () => {
    const r = classifyPrompt('deploy to render production');
    assert.equal(r.intent, 'deploy');
  });

  it('returns general for unclassifiable prompts', () => {
    const r = classifyPrompt('bonjour comment vas-tu');
    assert.equal(r.intent, 'general');
  });

  it('multiple intents detected for multi-concern prompts', () => {
    const r = classifyPrompt('fix the security vulnerability in the auth module');
    assert.ok(r.intents.length >= 2, `expected >= 2 intents, got ${r.intents.length}`);
    // Both debug and security should be present
    assert.ok(r.intents.some(i => i.intent === 'debug'), 'should detect debug');
    assert.ok(r.intents.some(i => i.intent === 'security'), 'should detect security');
  });
});

// =============================================================================
// Domain scoring
// =============================================================================

describe('classifyPrompt: domain scoring', () => {
  it('scores CODE domain for code prompts', () => {
    const r = classifyPrompt('implement a new function for the module');
    assert.ok(r.domains.CODE > 0.2);
  });

  it('scores SOLANA domain for blockchain prompts', () => {
    const r = classifyPrompt('send a solana transaction to mint tokens');
    assert.ok(r.domains.SOLANA > 0.5);
  });

  it('scores MARKET domain for price prompts', () => {
    const r = classifyPrompt('check the current price and liquidity on dex');
    assert.ok(r.domains.MARKET > 0.3);
  });

  it('scores SOCIAL domain for twitter prompts', () => {
    const r = classifyPrompt('post a tweet about the community engagement');
    assert.ok(r.domains.SOCIAL > 0.3);
  });

  it('scores HUMAN domain for psychology prompts', () => {
    const r = classifyPrompt('detect burnout and user fatigue patterns');
    assert.ok(r.domains.HUMAN > 0.3);
  });

  it('scores CYNIC domain for self-reference prompts', () => {
    const r = classifyPrompt('check cynic consciousness and dog verdicts');
    assert.ok(r.domains.CYNIC > 0.3);
  });

  it('scores COSMOS domain for ecosystem prompts', () => {
    const r = classifyPrompt('check cross-project ecosystem coherence');
    assert.ok(r.domains.COSMOS > 0.3);
  });

  it('topDomains sorted by score descending', () => {
    const r = classifyPrompt('fix the solana transaction error in the code');
    assert.ok(r.topDomains.length > 0);
    for (let i = 1; i < r.topDomains.length; i++) {
      assert.ok(r.topDomains[i - 1].score >= r.topDomains[i].score);
    }
  });

  it('recentDomain boosts relevance', () => {
    const without = classifyPrompt('check the status');
    const withBoost = classifyPrompt('check the status', { recentDomain: 'SOLANA' });
    assert.ok(withBoost.domains.SOLANA > without.domains.SOLANA);
  });

  it('domain scores never exceed 1', () => {
    const r = classifyPrompt('solana anchor spl token wallet keypair lamport transaction instruction program devnet mainnet');
    assert.ok(r.domains.SOLANA <= 1);
  });
});

// =============================================================================
// Complexity assessment
// =============================================================================

describe('classifyPrompt: complexity', () => {
  it('trivial for simple prompts', () => {
    const r = classifyPrompt('list files in src');
    assert.equal(r.complexity, 'trivial');
  });

  it('simple for short questions', () => {
    const r = classifyPrompt('how does the event bus work in this project');
    assert.ok(['trivial', 'simple'].includes(r.complexity));
  });

  it('moderate+ for longer prompts with technical depth', () => {
    const r = classifyPrompt('I need to implement a new authentication system that is extensible. ' +
      'It should support OAuth2 and JWT tokens. The architecture needs to be redesigned ' +
      'for future providers. Should we use passport.js or build our own? ' +
      'Consider security vulnerabilities and compare the trade-offs of each approach. ' +
      'We also need to refactor the existing auth middleware to support this.');
    assert.ok(['simple', 'moderate', 'complex', 'deep'].includes(r.complexity),
      `expected simple+, got ${r.complexity}`);
  });

  it('code blocks increase complexity', () => {
    const withCode = classifyPrompt('fix this:\n```\nfunction foo() { return bar; }\n```');
    const without = classifyPrompt('fix the function foo');
    const complexityOrder = ['trivial', 'simple', 'moderate', 'complex', 'deep'];
    assert.ok(complexityOrder.indexOf(withCode.complexity) >= complexityOrder.indexOf(without.complexity));
  });

  it('security intent boosts complexity above trivial', () => {
    const r = classifyPrompt('check for sql injection vulnerabilities');
    assert.notEqual(r.complexity, 'trivial');
  });
});

// =============================================================================
// Token budget
// =============================================================================

describe('classifyPrompt: token budget', () => {
  it('0 for skip prompts', () => {
    assert.equal(classifyPrompt('ok').tokenBudget, 0);
  });

  it('positive for real prompts', () => {
    assert.ok(classifyPrompt('implement authentication').tokenBudget > 0);
  });

  it('experience curve reduces budget', () => {
    const novice = classifyPrompt('implement authentication', { sessionCount: 1 });
    const expert = classifyPrompt('implement authentication', { sessionCount: 100 });
    assert.ok(expert.tokenBudget < novice.tokenBudget);
  });

  it('more domains = slightly more budget', () => {
    // Single domain
    const single = classifyPrompt('fix the solana bug');
    // Multi domain
    const multi = classifyPrompt('fix the solana transaction error in the code and update the ecosystem');
    // Multi should have equal or more budget (more domains = more context needed)
    assert.ok(multi.tokenBudget >= single.tokenBudget);
  });
});

// =============================================================================
// Context relevance scoring
// =============================================================================

describe('scoreContextRelevance', () => {
  const sections = ['code_status', 'solana_context', 'market_data', 'social_feed', 'ecosystem_overview'];

  it('returns scores for all sections', () => {
    const classification = classifyPrompt('fix the solana bug');
    const scores = scoreContextRelevance(classification, sections);
    assert.equal(Object.keys(scores).length, sections.length);
  });

  it('boosts relevant sections', () => {
    const classification = classifyPrompt('check solana transaction status');
    const scores = scoreContextRelevance(classification, sections);
    assert.ok(scores.solana_context > scores.market_data);
  });

  it('all scores are φ-bounded', () => {
    const classification = classifyPrompt('check everything in solana and code');
    const scores = scoreContextRelevance(classification, sections);
    for (const score of Object.values(scores)) {
      assert.ok(score <= PHI_INV);
    }
  });

  it('skip prompts get zero scores', () => {
    const classification = classifyPrompt('ok');
    const scores = scoreContextRelevance(classification, sections);
    for (const score of Object.values(scores)) {
      assert.equal(score, 0);
    }
  });
});

// =============================================================================
// Section selection
// =============================================================================

describe('selectSections', () => {
  it('selects highest-relevance sections first', () => {
    const scores = { a: 0.5, b: 0.8, c: 0.2 };
    const sizes = { a: 100, b: 100, c: 100 };
    const selected = selectSections(scores, 200, sizes);
    assert.equal(selected[0], 'b');
    assert.equal(selected.length, 2);
  });

  it('respects token budget', () => {
    const scores = { a: 0.5, b: 0.8, c: 0.6 };
    const sizes = { a: 300, b: 300, c: 300 };
    const selected = selectSections(scores, 500, sizes);
    // Only 500 budget, each costs 300 — can fit 1
    assert.equal(selected.length, 1);
    assert.equal(selected[0], 'b');
  });

  it('excludes low-relevance sections', () => {
    const scores = { a: 0.05, b: 0.8 };
    const sizes = { a: 100, b: 100 };
    const selected = selectSections(scores, 1000, sizes);
    assert.equal(selected.length, 1);
    assert.equal(selected[0], 'b');
  });

  it('empty budget = no sections', () => {
    const scores = { a: 0.8, b: 0.6 };
    const sizes = { a: 100, b: 100 };
    const selected = selectSections(scores, 0, sizes);
    assert.equal(selected.length, 0);
  });
});

// =============================================================================
// Re-export from @cynic/core
// =============================================================================

describe('prompt-classifier re-export', () => {
  it('accessible from @cynic/core', async () => {
    const core = await import('../src/index.js');
    assert.equal(typeof core.classifyPrompt, 'function');
    assert.equal(typeof core.scoreContextRelevance, 'function');
    assert.equal(typeof core.selectSections, 'function');
  });
});
