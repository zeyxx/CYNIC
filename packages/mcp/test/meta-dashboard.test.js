/**
 * Meta Dashboard Tests
 *
 * Tests for the MetaDashboard service that provides CYNIC self-analysis.
 *
 * @module @cynic/mcp/test/meta-dashboard
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import {
  MetaDashboard,
  CYNIC_ARCHITECTURE,
  createMetaTool,
} from '../src/meta-dashboard.js';

describe('CYNIC_ARCHITECTURE', () => {
  it('has required structure', () => {
    assert.ok(CYNIC_ARCHITECTURE.version);
    assert.ok(CYNIC_ARCHITECTURE.packages);
    assert.ok(CYNIC_ARCHITECTURE.dogs);
    assert.ok(CYNIC_ARCHITECTURE.axioms);
    assert.ok(CYNIC_ARCHITECTURE.gaps);
    assert.ok(CYNIC_ARCHITECTURE.ecosystem);
  });

  it('has all packages defined', () => {
    const expected = ['core', 'protocol', 'persistence', 'node', 'mcp', 'client'];
    for (const pkg of expected) {
      assert.ok(CYNIC_ARCHITECTURE.packages[pkg], `Missing package: ${pkg}`);
    }
  });

  it('has all axioms defined', () => {
    const expected = ['PHI', 'VERIFY', 'CULTURE', 'BURN'];
    for (const axiom of expected) {
      assert.ok(CYNIC_ARCHITECTURE.axioms[axiom], `Missing axiom: ${axiom}`);
      assert.ok(CYNIC_ARCHITECTURE.axioms[axiom].symbol);
      assert.ok(CYNIC_ARCHITECTURE.axioms[axiom].world);
      assert.ok(CYNIC_ARCHITECTURE.axioms[axiom].dimensions);
    }
  });

  it('has v1 dogs defined', () => {
    const expected = ['observer', 'digester', 'guardian', 'mentor'];
    for (const dog of expected) {
      assert.ok(CYNIC_ARCHITECTURE.dogs.v1[dog], `Missing v1 dog: ${dog}`);
    }
  });

  it('has v2 collective dogs defined', () => {
    const v2 = CYNIC_ARCHITECTURE.dogs.v2_collective;
    assert.ok(Object.keys(v2).length >= 5);
    assert.ok(v2.guardian);
    assert.ok(v2.analyst);
  });

  it('has gaps categorized', () => {
    assert.ok(Array.isArray(CYNIC_ARCHITECTURE.gaps.critical));
    assert.ok(Array.isArray(CYNIC_ARCHITECTURE.gaps.important));
    assert.ok(Array.isArray(CYNIC_ARCHITECTURE.gaps.wishlist));
  });
});

describe('MetaDashboard', () => {
  let dashboard;

  beforeEach(() => {
    dashboard = new MetaDashboard();
  });

  describe('constructor', () => {
    it('initializes with architecture', () => {
      assert.ok(dashboard.architecture);
      assert.equal(dashboard.architecture, CYNIC_ARCHITECTURE);
    });
  });

  describe('analyze', () => {
    it('returns complete analysis', () => {
      const analysis = dashboard.analyze();

      assert.ok(analysis.identity);
      assert.ok(analysis.packages);
      assert.ok(analysis.dogs);
      assert.ok(analysis.axioms);
      assert.ok(analysis.gaps);
      assert.ok(analysis.metrics);
      assert.ok(analysis.recommendations);
    });
  });

  describe('getIdentity', () => {
    it('returns CYNIC identity', () => {
      const identity = dashboard.getIdentity();

      assert.equal(identity.name, 'CYNIC');
      assert.ok(identity.etymology.includes('κυνικός'));
      assert.ok(identity.maxConfidence <= 0.62); // φ⁻¹ ≈ 0.618
      assert.ok(identity.version);
      assert.ok(Array.isArray(identity.axioms));
      assert.equal(identity.axioms.length, 4);
    });
  });

  describe('getPackagesStatus', () => {
    it('returns packages overview', () => {
      const status = dashboard.getPackagesStatus();

      assert.ok(status.packages);
      assert.ok(status.overall);
      assert.ok(status.overall.total >= 6);
      assert.ok(typeof status.overall.avgCompleteness === 'number');
    });

    it('calculates module ratios', () => {
      const status = dashboard.getPackagesStatus();

      for (const pkg of Object.values(status.packages)) {
        assert.ok(pkg.modules);
        assert.ok(typeof pkg.modules.total === 'number');
        assert.ok(typeof pkg.modules.complete === 'number');
        assert.ok(typeof pkg.modules.ratio === 'number');
        assert.ok(pkg.modules.ratio >= 0 && pkg.modules.ratio <= 1);
      }
    });
  });

  describe('getDogsStatus', () => {
    it('returns dogs status', () => {
      const status = dashboard.getDogsStatus();

      assert.ok(status.v1);
      assert.ok(status.v2);
      assert.ok(status.missing);
      assert.ok(status.total);
    });

    it('includes v1 dog details', () => {
      const status = dashboard.getDogsStatus();

      assert.ok(status.v1.active >= 4);
      assert.ok(Array.isArray(status.v1.list));
      for (const dog of status.v1.list) {
        assert.ok(dog.id);
        assert.ok(dog.name);
        assert.ok(dog.trigger);
        assert.ok(typeof dog.blocking === 'boolean');
      }
    });

    it('includes v2 collective details', () => {
      const status = dashboard.getDogsStatus();

      assert.ok(status.v2.implemented >= 5);
      assert.ok(Array.isArray(status.v2.list));
      for (const dog of status.v2.list) {
        assert.ok(dog.id);
        assert.ok(dog.name);
        assert.ok(dog.sephirah);
      }
    });

    it('includes missing dogs', () => {
      const status = dashboard.getDogsStatus();

      assert.ok(status.missing.count >= 5);
      assert.ok(Array.isArray(status.missing.list));
    });

    it('calculates total correctly', () => {
      const status = dashboard.getDogsStatus();

      assert.equal(
        status.total.implemented,
        status.v1.active + status.v2.implemented
      );
    });
  });

  describe('getAxiomsStatus', () => {
    it('returns axioms array', () => {
      const axioms = dashboard.getAxiomsStatus();

      assert.ok(Array.isArray(axioms));
      assert.equal(axioms.length, 4);
    });

    it('includes axiom details', () => {
      const axioms = dashboard.getAxiomsStatus();

      for (const axiom of axioms) {
        assert.ok(axiom.name);
        assert.ok(axiom.symbol);
        assert.ok(axiom.world);
        assert.ok(axiom.question);
        assert.ok(axiom.color);
        assert.ok(typeof axiom.dimensionCount === 'number');
        assert.ok(Array.isArray(axiom.dimensions));
      }
    });

    it('has correct worlds mapping', () => {
      const axioms = dashboard.getAxiomsStatus();
      const worldMap = {};
      for (const axiom of axioms) {
        worldMap[axiom.name] = axiom.world;
      }

      assert.equal(worldMap.PHI, 'Atzilut');
      assert.equal(worldMap.VERIFY, 'Beriah');
      assert.equal(worldMap.CULTURE, 'Yetzirah');
      assert.equal(worldMap.BURN, 'Assiah');
    });
  });

  describe('getGaps', () => {
    it('returns gaps analysis', () => {
      const gaps = dashboard.getGaps();

      assert.ok(gaps.critical);
      assert.ok(gaps.important);
      assert.ok(gaps.wishlist);
      assert.ok(gaps.summary);
    });

    it('calculates summary correctly', () => {
      const gaps = dashboard.getGaps();

      assert.equal(gaps.summary.critical, gaps.critical.length);
      assert.equal(gaps.summary.important, gaps.important.length);
      assert.equal(gaps.summary.wishlist, gaps.wishlist.length);
      assert.equal(
        gaps.summary.total,
        gaps.critical.length + gaps.important.length + gaps.wishlist.length
      );
    });

    it('gaps have required fields', () => {
      const gaps = dashboard.getGaps();
      const allGaps = [...gaps.critical, ...gaps.important, ...gaps.wishlist];

      for (const gap of allGaps) {
        assert.ok(gap.id);
        assert.ok(gap.area);
        assert.ok(gap.title);
        assert.ok(gap.description);
        assert.ok(gap.priority);
        assert.ok(Array.isArray(gap.packages));
      }
    });
  });

  describe('getMetrics', () => {
    it('returns metrics', () => {
      const metrics = dashboard.getMetrics();

      assert.ok(metrics.linesOfCode);
      assert.ok(metrics.packages);
      assert.ok(metrics.testFiles);
      assert.ok(metrics.dimensions);
      assert.ok(metrics.mcpTools);
      assert.ok(metrics.maxConfidence);
    });
  });

  describe('getRecommendations', () => {
    it('returns recommendations', () => {
      const recs = dashboard.getRecommendations();

      assert.ok(Array.isArray(recs.priorities));
      assert.ok(Array.isArray(recs.nextSteps));
      assert.ok(recs.philosophy);
    });

    it('priorities are ordered', () => {
      const recs = dashboard.getRecommendations();

      for (let i = 1; i < recs.priorities.length; i++) {
        assert.ok(recs.priorities[i].priority >= recs.priorities[i - 1].priority);
      }
    });

    it('next steps have structure', () => {
      const recs = dashboard.getRecommendations();

      for (const step of recs.nextSteps) {
        assert.ok(typeof step.step === 'number');
        assert.ok(step.action);
        assert.ok(step.description);
      }
    });
  });

  describe('toAscii', () => {
    it('returns ASCII dashboard', () => {
      const ascii = dashboard.toAscii();

      assert.ok(typeof ascii === 'string');
      assert.ok(ascii.includes('CYNIC'));
      assert.ok(ascii.includes('META DASHBOARD'));
      assert.ok(ascii.includes('IDENTITY'));
      assert.ok(ascii.includes('PACKAGES'));
      assert.ok(ascii.includes('THE DOGS'));
      assert.ok(ascii.includes('GAPS'));
    });
  });

  describe('toJSON', () => {
    it('returns valid JSON', () => {
      const json = dashboard.toJSON();

      assert.ok(typeof json === 'string');
      const parsed = JSON.parse(json);
      assert.ok(parsed.identity);
      assert.ok(parsed.packages);
    });
  });

  describe('_progressBar', () => {
    it('creates progress bar', () => {
      const bar0 = dashboard._progressBar(0, 10);
      const bar50 = dashboard._progressBar(0.5, 10);
      const bar100 = dashboard._progressBar(1, 10);

      assert.equal(bar0, '[░░░░░░░░░░]');
      assert.equal(bar50, '[█████░░░░░]');
      assert.equal(bar100, '[██████████]');
    });
  });
});

describe('createMetaTool', () => {
  it('creates MCP tool definition', () => {
    const tool = createMetaTool();

    assert.equal(tool.name, 'brain_meta');
    assert.ok(tool.description);
    assert.ok(tool.inputSchema);
    assert.ok(typeof tool.handler === 'function');
  });

  it('handler returns packages status', async () => {
    const tool = createMetaTool();
    const result = await tool.handler({ action: 'packages' });

    assert.ok(result.content);
    assert.ok(result.content[0].text);
    const parsed = JSON.parse(result.content[0].text);
    assert.ok(parsed.packages);
  });

  it('handler returns dogs status', async () => {
    const tool = createMetaTool();
    const result = await tool.handler({ action: 'dogs' });

    assert.ok(result.content);
    const parsed = JSON.parse(result.content[0].text);
    assert.ok(parsed.v1);
    assert.ok(parsed.v2);
  });

  it('handler returns axioms status', async () => {
    const tool = createMetaTool();
    const result = await tool.handler({ action: 'axioms' });

    assert.ok(result.content);
    const parsed = JSON.parse(result.content[0].text);
    assert.ok(Array.isArray(parsed));
    assert.equal(parsed.length, 4);
  });

  it('handler returns gaps', async () => {
    const tool = createMetaTool();
    const result = await tool.handler({ action: 'gaps' });

    assert.ok(result.content);
    const parsed = JSON.parse(result.content[0].text);
    assert.ok(parsed.summary);
  });

  it('handler returns recommendations', async () => {
    const tool = createMetaTool();
    const result = await tool.handler({ action: 'recommendations' });

    assert.ok(result.content);
    const parsed = JSON.parse(result.content[0].text);
    assert.ok(parsed.priorities);
    assert.ok(parsed.nextSteps);
  });

  it('handler returns ascii', async () => {
    const tool = createMetaTool();
    const result = await tool.handler({ action: 'ascii' });

    assert.ok(result.content);
    assert.ok(result.content[0].text.includes('CYNIC META DASHBOARD'));
  });

  it('handler returns json', async () => {
    const tool = createMetaTool();
    const result = await tool.handler({ action: 'json' });

    assert.ok(result.content);
    const parsed = JSON.parse(result.content[0].text);
    assert.ok(parsed.identity);
  });

  it('handler defaults to analyze', async () => {
    const tool = createMetaTool();
    const result = await tool.handler({});

    assert.ok(result.content);
    assert.ok(result.content[0].text.includes('CYNIC META DASHBOARD'));
  });
});
