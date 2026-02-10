/**
 * Tests for Adaptive Boot configuration (awaken.js getBootConfig)
 *
 * Since awaken.js is a script (not a module), we duplicate the getBootConfig
 * logic here for testing. Any changes to getBootConfig in awaken.js MUST be
 * reflected here.
 *
 * @module test/adaptive-boot
 */

'use strict';

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';

// ═══════════════════════════════════════════════════════════════════════════
// DUPLICATED FROM awaken.js (keep in sync!)
// ═══════════════════════════════════════════════════════════════════════════

const FACT_INJECTION_LIMIT = 50;

function getBootConfig(experienceLevel) {
  switch (experienceLevel) {
    case 'expert':
      return {
        includeIdentity: false,
        includeKernel: false,
        includeDogTree: false,
        factLimit: 5,
        reflectionLimit: 1,
        includeBurnAnalysis: false,
        includeArchDecisions: false,
        bannerDogTree: false,
        contextLabel: 'EXPERT',
      };
    case 'experienced':
      return {
        includeIdentity: false,
        includeKernel: false,
        includeDogTree: false,
        factLimit: 15,
        reflectionLimit: 3,
        includeBurnAnalysis: false,
        includeArchDecisions: false,
        bannerDogTree: false,
        contextLabel: 'EXPERIENCED',
      };
    case 'learning':
      return {
        includeIdentity: true,
        includeKernel: false,
        includeDogTree: true,
        factLimit: 30,
        reflectionLimit: 5,
        includeBurnAnalysis: true,
        includeArchDecisions: true,
        bannerDogTree: true,
        contextLabel: 'LEARNING',
      };
    case 'new':
    default:
      return {
        includeIdentity: true,
        includeKernel: true,
        includeDogTree: true,
        factLimit: FACT_INJECTION_LIMIT,
        reflectionLimit: 10,
        includeBurnAnalysis: true,
        includeArchDecisions: true,
        bannerDogTree: true,
        contextLabel: 'NEW',
      };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('Adaptive Boot Config', () => {
  describe('getBootConfig', () => {
    it('returns full config for new users', () => {
      const config = getBootConfig('new');
      assert.equal(config.includeIdentity, true);
      assert.equal(config.includeKernel, true);
      assert.equal(config.includeDogTree, true);
      assert.equal(config.factLimit, 50);
      assert.equal(config.reflectionLimit, 10);
      assert.equal(config.includeBurnAnalysis, true);
      assert.equal(config.includeArchDecisions, true);
      assert.equal(config.bannerDogTree, true);
      assert.equal(config.contextLabel, 'NEW');
    });

    it('skips kernel for learning users', () => {
      const config = getBootConfig('learning');
      assert.equal(config.includeIdentity, true, 'identity still shown');
      assert.equal(config.includeKernel, false, 'kernel redundant with CLAUDE.md');
      assert.equal(config.includeDogTree, true, 'still learning the tree');
      assert.equal(config.factLimit, 30);
      assert.equal(config.reflectionLimit, 5);
      assert.equal(config.contextLabel, 'LEARNING');
    });

    it('skips identity, kernel, dogs, burn, arch for experienced users', () => {
      const config = getBootConfig('experienced');
      assert.equal(config.includeIdentity, false);
      assert.equal(config.includeKernel, false);
      assert.equal(config.includeDogTree, false);
      assert.equal(config.factLimit, 15);
      assert.equal(config.reflectionLimit, 3);
      assert.equal(config.includeBurnAnalysis, false);
      assert.equal(config.includeArchDecisions, false);
      assert.equal(config.bannerDogTree, false);
      assert.equal(config.contextLabel, 'EXPERIENCED');
    });

    it('provides minimal config for expert users', () => {
      const config = getBootConfig('expert');
      assert.equal(config.includeIdentity, false);
      assert.equal(config.includeKernel, false);
      assert.equal(config.includeDogTree, false);
      assert.equal(config.factLimit, 5);
      assert.equal(config.reflectionLimit, 1);
      assert.equal(config.includeBurnAnalysis, false);
      assert.equal(config.includeArchDecisions, false);
      assert.equal(config.bannerDogTree, false);
      assert.equal(config.contextLabel, 'EXPERT');
    });

    it('defaults to new for unknown experience levels', () => {
      const config = getBootConfig('unknown_level');
      assert.equal(config.includeIdentity, true);
      assert.equal(config.includeKernel, true);
      assert.equal(config.factLimit, 50);
      assert.equal(config.contextLabel, 'NEW');
    });

    it('defaults to new for undefined', () => {
      const config = getBootConfig(undefined);
      assert.equal(config.contextLabel, 'NEW');
    });
  });

  describe('fact limit progression', () => {
    it('decreases monotonically with experience', () => {
      const newConfig = getBootConfig('new');
      const learningConfig = getBootConfig('learning');
      const experiencedConfig = getBootConfig('experienced');
      const expertConfig = getBootConfig('expert');

      assert.ok(newConfig.factLimit > learningConfig.factLimit, 'new > learning');
      assert.ok(learningConfig.factLimit > experiencedConfig.factLimit, 'learning > experienced');
      assert.ok(experiencedConfig.factLimit > expertConfig.factLimit, 'experienced > expert');
    });

    it('expert factLimit is at least 1', () => {
      const config = getBootConfig('expert');
      assert.ok(config.factLimit >= 1, 'at least 1 fact for expert');
    });
  });

  describe('reflection limit progression', () => {
    it('decreases with experience', () => {
      const levels = ['new', 'learning', 'experienced', 'expert'];
      const limits = levels.map(l => getBootConfig(l).reflectionLimit);

      for (let i = 1; i < limits.length; i++) {
        assert.ok(limits[i] <= limits[i - 1], `${levels[i]} (${limits[i]}) <= ${levels[i - 1]} (${limits[i - 1]})`);
      }
    });
  });

  describe('section skipping progression', () => {
    it('new users see everything', () => {
      const config = getBootConfig('new');
      const allTrue = config.includeIdentity && config.includeKernel &&
        config.includeDogTree && config.includeBurnAnalysis && config.includeArchDecisions;
      assert.equal(allTrue, true, 'all sections included for new users');
    });

    it('expert users see only dynamic data', () => {
      const config = getBootConfig('expert');
      const allSkipped = !config.includeIdentity && !config.includeKernel &&
        !config.includeDogTree && !config.includeBurnAnalysis && !config.includeArchDecisions;
      assert.equal(allSkipped, true, 'all static sections skipped for expert');
    });

    it('learning users get identity but not kernel', () => {
      const config = getBootConfig('learning');
      assert.equal(config.includeIdentity, true, 'identity shown for reinforcement');
      assert.equal(config.includeKernel, false, 'kernel already in CLAUDE.md');
    });
  });

  describe('compression savings estimation', () => {
    it('calculates correct savings string', () => {
      const config = getBootConfig('experienced');
      const sectionsSkipped = [];
      if (!config.includeIdentity) sectionsSkipped.push('identity');
      if (!config.includeKernel) sectionsSkipped.push('kernel');
      if (!config.includeDogTree) sectionsSkipped.push('dogs');
      if (!config.includeBurnAnalysis) sectionsSkipped.push('burn');
      if (!config.includeArchDecisions) sectionsSkipped.push('archDecisions');

      assert.equal(sectionsSkipped.length, 5, 'experienced skips 5 sections');
      assert.equal(config.factLimit, 15, 'fact limit is 15');

      const savings = `${sectionsSkipped.length} sections skipped, facts ${FACT_INJECTION_LIMIT}→${config.factLimit}`;
      assert.equal(savings, '5 sections skipped, facts 50→15');
    });

    it('new users have zero savings', () => {
      const config = getBootConfig('new');
      const sectionsSkipped = [];
      if (!config.includeIdentity) sectionsSkipped.push('identity');
      if (!config.includeKernel) sectionsSkipped.push('kernel');
      if (!config.includeDogTree) sectionsSkipped.push('dogs');
      if (!config.includeBurnAnalysis) sectionsSkipped.push('burn');
      if (!config.includeArchDecisions) sectionsSkipped.push('archDecisions');

      assert.equal(sectionsSkipped.length, 0, 'no sections skipped for new');
      assert.equal(config.factLimit, 50, 'full fact limit');
    });
  });
});
