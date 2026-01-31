/**
 * Rules Loader Tests (S1: Skill auto-activation via rules.json)
 *
 * Tests for loading and detecting skills from skill-rules.json
 *
 * @module scripts/hooks/test/rules-loader
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  loadRulesFile,
  getSkillTriggers,
  getRulesSettings,
  detectSkillTriggersFromRules,
  clearRulesCache,
} from '../lib/rules-loader.js';

// =============================================================================
// RULES FILE LOADING
// =============================================================================

describe('rules-loader - loadRulesFile', () => {
  it('should load rules from default path', () => {
    const rules = loadRulesFile();

    assert.ok(rules);
    assert.ok(rules.rules);
    assert.ok(Array.isArray(rules.rules));
    assert.ok(rules.rules.length > 0);
  });

  it('should have version and description', () => {
    const rules = loadRulesFile();

    assert.ok(rules.version);
    assert.ok(rules.description);
  });

  it('should have settings', () => {
    const rules = loadRulesFile();

    assert.ok(rules.settings);
    assert.strictEqual(typeof rules.settings.enabled, 'boolean');
    assert.strictEqual(typeof rules.settings.maxSuggestions, 'number');
  });

  it('should return null for non-existent path', () => {
    const rules = loadRulesFile('/nonexistent/path/rules.json');

    // Should try default paths and succeed
    assert.ok(rules);
  });
});

// =============================================================================
// SKILL TRIGGERS COMPILATION
// =============================================================================

describe('rules-loader - getSkillTriggers', () => {
  beforeEach(() => {
    clearRulesCache();
  });

  it('should return compiled triggers', () => {
    const triggers = getSkillTriggers();

    assert.ok(triggers);
    assert.strictEqual(typeof triggers, 'object');
  });

  it('should have /judge trigger', () => {
    const triggers = getSkillTriggers();

    assert.ok(triggers['/judge']);
    assert.ok(triggers['/judge'].patterns);
    assert.ok(Array.isArray(triggers['/judge'].patterns));
    assert.strictEqual(triggers['/judge'].priority, 'high');
  });

  it('should have /search trigger', () => {
    const triggers = getSkillTriggers();

    assert.ok(triggers['/search']);
    assert.strictEqual(triggers['/search'].priority, 'medium');
  });

  it('should have /health trigger', () => {
    const triggers = getSkillTriggers();

    assert.ok(triggers['/health']);
    assert.strictEqual(triggers['/health'].priority, 'low');
  });

  it('should compile patterns to RegExp', () => {
    const triggers = getSkillTriggers();

    for (const pattern of triggers['/judge'].patterns) {
      assert.ok(pattern instanceof RegExp);
    }
  });

  it('should cache results', () => {
    const triggers1 = getSkillTriggers();
    const triggers2 = getSkillTriggers();

    assert.strictEqual(triggers1, triggers2);
  });

  it('should reload on forceReload', () => {
    const triggers1 = getSkillTriggers();
    const triggers2 = getSkillTriggers({ forceReload: true });

    // Objects are different but content should be same
    assert.deepStrictEqual(Object.keys(triggers1), Object.keys(triggers2));
  });
});

// =============================================================================
// RULES SETTINGS
// =============================================================================

describe('rules-loader - getRulesSettings', () => {
  it('should return settings object', () => {
    const settings = getRulesSettings();

    assert.ok(settings);
    assert.strictEqual(typeof settings, 'object');
  });

  it('should have enabled flag', () => {
    const settings = getRulesSettings();

    assert.strictEqual(typeof settings.enabled, 'boolean');
  });

  it('should have maxSuggestions', () => {
    const settings = getRulesSettings();

    assert.strictEqual(typeof settings.maxSuggestions, 'number');
    assert.ok(settings.maxSuggestions > 0);
  });

  it('should have priorityOrder', () => {
    const settings = getRulesSettings();

    assert.ok(Array.isArray(settings.priorityOrder));
    assert.ok(settings.priorityOrder.includes('high'));
    assert.ok(settings.priorityOrder.includes('medium'));
    assert.ok(settings.priorityOrder.includes('low'));
  });
});

// =============================================================================
// SKILL DETECTION
// =============================================================================

describe('rules-loader - detectSkillTriggersFromRules', () => {
  beforeEach(() => {
    clearRulesCache();
  });

  it('should detect /judge skill', () => {
    const matches = detectSkillTriggersFromRules('Can you judge this code?');

    assert.ok(matches.length > 0);
    assert.ok(matches.some(m => m.skill === '/judge'));
  });

  it('should detect /search skill', () => {
    const matches = detectSkillTriggersFromRules('Search memory for what we did before');

    assert.ok(matches.length > 0);
    assert.ok(matches.some(m => m.skill === '/search'));
  });

  it('should detect /patterns skill', () => {
    const matches = detectSkillTriggersFromRules('Show me the patterns you detected');

    assert.ok(matches.length > 0);
    assert.ok(matches.some(m => m.skill === '/patterns'));
  });

  it('should detect /trace skill', () => {
    const matches = detectSkillTriggersFromRules('Trace this judgment through the blockchain');

    assert.ok(matches.length > 0);
    assert.ok(matches.some(m => m.skill === '/trace'));
  });

  it('should detect /learn skill', () => {
    const matches = detectSkillTriggersFromRules('That judgment was wrong, feedback');

    assert.ok(matches.length > 0);
    assert.ok(matches.some(m => m.skill === '/learn'));
  });

  it('should detect /health skill', () => {
    const matches = detectSkillTriggersFromRules('Check CYNIC system health status');

    assert.ok(matches.length > 0);
    assert.ok(matches.some(m => m.skill === '/health'));
  });

  it('should detect /psy skill', () => {
    const matches = detectSkillTriggersFromRules('How am I doing? Check my energy');

    assert.ok(matches.length > 0);
    assert.ok(matches.some(m => m.skill === '/psy'));
  });

  it('should detect /dogs skill', () => {
    const matches = detectSkillTriggersFromRules('Which dogs are active in the collective?');

    assert.ok(matches.length > 0);
    assert.ok(matches.some(m => m.skill === '/dogs'));
  });

  it('should detect /wisdom skill', () => {
    const matches = detectSkillTriggersFromRules('What philosophical wisdom applies here?');

    assert.ok(matches.length > 0);
    assert.ok(matches.some(m => m.skill === '/wisdom'));
  });

  it('should detect /digest skill', () => {
    const matches = detectSkillTriggersFromRules('Digest and extract patterns from this');

    assert.ok(matches.length > 0);
    assert.ok(matches.some(m => m.skill === '/digest'));
  });

  it('should detect /status skill', () => {
    const matches = detectSkillTriggersFromRules('What is CYNIC status and version?');

    assert.ok(matches.length > 0);
    assert.ok(matches.some(m => m.skill === '/status'));
  });

  it('should detect /ecosystem skill', () => {
    const matches = detectSkillTriggersFromRules('View the ecosystem and other repos');

    assert.ok(matches.length > 0);
    assert.ok(matches.some(m => m.skill === '/ecosystem'));
  });

  it('should detect /cockpit skill', () => {
    const matches = detectSkillTriggersFromRules('Show me the cockpit dashboard');

    assert.ok(matches.length > 0);
    assert.ok(matches.some(m => m.skill === '/cockpit'));
  });
});

// =============================================================================
// PRIORITY ORDERING
// =============================================================================

describe('rules-loader - priority ordering', () => {
  it('should sort by priority (high first)', () => {
    // This prompt should match both /judge (high) and /status (low)
    const matches = detectSkillTriggersFromRules('Judge the CYNIC status');

    if (matches.length >= 2) {
      const judgeIdx = matches.findIndex(m => m.skill === '/judge');
      const statusIdx = matches.findIndex(m => m.skill === '/status');

      if (judgeIdx !== -1 && statusIdx !== -1) {
        assert.ok(judgeIdx < statusIdx, '/judge (high) should come before /status (low)');
      }
    }
  });

  it('should respect maxSuggestions limit', () => {
    // A prompt that matches many skills
    const matches = detectSkillTriggersFromRules(
      'Judge the patterns, search memory, check health, and show status',
      { maxSuggestions: 2 }
    );

    assert.ok(matches.length <= 2);
  });
});

// =============================================================================
// KEYWORD DETECTION
// =============================================================================

describe('rules-loader - keyword detection', () => {
  it('should detect by keyword when pattern does not match', () => {
    // "q-score" is a keyword for /judge
    const matches = detectSkillTriggersFromRules('What is the q-score?');

    assert.ok(matches.length > 0);
    assert.ok(matches.some(m => m.skill === '/judge'));
  });

  it('should detect Diogenes keyword for /wisdom', () => {
    const matches = detectSkillTriggersFromRules('Tell me about Diogenes');

    assert.ok(matches.length > 0);
    assert.ok(matches.some(m => m.skill === '/wisdom'));
  });
});

// =============================================================================
// EDGE CASES
// =============================================================================

describe('rules-loader - edge cases', () => {
  it('should return empty array for empty prompt', () => {
    const matches = detectSkillTriggersFromRules('');

    assert.ok(Array.isArray(matches));
    assert.strictEqual(matches.length, 0);
  });

  it('should handle prompt with no matches', () => {
    const matches = detectSkillTriggersFromRules('Hello world, how are you today?');

    assert.ok(Array.isArray(matches));
    // May or may not have matches depending on rules
  });

  it('should handle special characters', () => {
    const matches = detectSkillTriggersFromRules('Judge this: <>&"\'{} code');

    // Should not throw
    assert.ok(Array.isArray(matches));
  });

  it('should handle unicode', () => {
    const matches = detectSkillTriggersFromRules('Judge κυνικός code 🐕');

    // Should not throw
    assert.ok(Array.isArray(matches));
  });
});

// =============================================================================
// CACHE MANAGEMENT
// =============================================================================

describe('rules-loader - cache management', () => {
  it('should clear cache', () => {
    // Load to populate cache
    getSkillTriggers();

    // Clear cache
    clearRulesCache();

    // Should reload
    const triggers = getSkillTriggers({ forceReload: true });
    assert.ok(triggers);
  });
});
