/**
 * ToolRegistry Tests
 *
 * Tests for OCP-compliant tool management system.
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

import { ToolRegistry } from '../src/tools/registry.js';

// ═══════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════

function makeFactory(name, domain, requires = [], tools = []) {
  return {
    name,
    domain,
    requires,
    create: (options) => tools.length > 0 ? tools : [{ name: `tool_${name}`, handler: async () => ({}) }],
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('ToolRegistry', () => {
  let registry;

  beforeEach(() => {
    registry = new ToolRegistry();
  });

  describe('register()', () => {
    it('registers a factory', () => {
      registry.register(makeFactory('judge', 'judgment'));
      assert.strictEqual(registry.factoryCount, 1);
    });

    it('registers domain', () => {
      registry.register(makeFactory('judge', 'judgment'));
      assert.deepStrictEqual(registry.getDomains(), ['judgment']);
    });

    it('throws if factory has no name', () => {
      assert.throws(
        () => registry.register({ create: () => [] }),
        /must have a name/
      );
    });

    it('throws if factory has no create function', () => {
      assert.throws(
        () => registry.register({ name: 'bad' }),
        /must have a create function/
      );
    });

    it('returns registry for chaining', () => {
      const result = registry.register(makeFactory('a', 'x'));
      assert.strictEqual(result, registry);
    });
  });

  describe('registerAll()', () => {
    it('registers multiple factories', () => {
      registry.registerAll([
        makeFactory('a', 'domain1'),
        makeFactory('b', 'domain2'),
        makeFactory('c', 'domain1'),
      ]);
      assert.strictEqual(registry.factoryCount, 3);
    });
  });

  describe('createAll()', () => {
    it('creates tools from all factories', () => {
      registry.register(makeFactory('judge', 'judgment'));
      registry.register(makeFactory('search', 'memory'));
      const tools = registry.createAll();

      assert.strictEqual(tools.length, 2);
      assert.strictEqual(registry.toolCount, 2);
    });

    it('skips factories with missing dependencies', () => {
      registry.register(makeFactory('needs_db', 'data', ['database']));
      const tools = registry.createAll({}); // No database provided

      assert.strictEqual(tools.length, 0);
    });

    it('creates tools when dependencies satisfied', () => {
      registry.register(makeFactory('needs_db', 'data', ['database']));
      const tools = registry.createAll({ database: {} });

      assert.strictEqual(tools.length, 1);
    });

    it('handles factory create errors gracefully', () => {
      registry.register({
        name: 'broken',
        domain: 'broken',
        create: () => { throw new Error('factory broken'); },
      });
      const tools = registry.createAll();

      assert.strictEqual(tools.length, 0);
    });

    it('handles factory returning single tool (not array)', () => {
      registry.register({
        name: 'single',
        domain: 'test',
        create: () => ({ name: 'single_tool', handler: async () => ({}) }),
      });
      const tools = registry.createAll();

      assert.strictEqual(tools.length, 1);
      assert.strictEqual(tools[0].name, 'single_tool');
    });

    it('filters out null/unnamed tools', () => {
      registry.register({
        name: 'mixed',
        domain: 'test',
        create: () => [
          { name: 'good_tool', handler: async () => ({}) },
          null,
          { handler: async () => ({}) }, // no name
        ],
      });
      const tools = registry.createAll();

      assert.strictEqual(tools.length, 1);
    });
  });

  describe('createByDomain()', () => {
    it('creates tools for specific domain only', () => {
      registry.register(makeFactory('judge', 'judgment'));
      registry.register(makeFactory('search', 'memory'));
      const tools = registry.createByDomain('judgment');

      assert.strictEqual(tools.length, 1);
    });

    it('returns empty for unknown domain', () => {
      registry.register(makeFactory('judge', 'judgment'));
      const tools = registry.createByDomain('nonexistent');

      assert.strictEqual(tools.length, 0);
    });
  });

  describe('get()', () => {
    it('retrieves created tool by name', () => {
      registry.register(makeFactory('judge', 'judgment'));
      registry.createAll();
      const tool = registry.get('tool_judge');

      assert.ok(tool);
      assert.strictEqual(tool.name, 'tool_judge');
    });

    it('returns undefined for unknown tool', () => {
      assert.strictEqual(registry.get('nonexistent'), undefined);
    });
  });

  describe('getToolNames()', () => {
    it('returns all tool names after createAll', () => {
      registry.register(makeFactory('a', 'x'));
      registry.register(makeFactory('b', 'y'));
      registry.createAll();

      const names = registry.getToolNames();
      assert.strictEqual(names.length, 2);
      assert.ok(names.includes('tool_a'));
      assert.ok(names.includes('tool_b'));
    });
  });

  describe('clear()', () => {
    it('clears all state', () => {
      registry.register(makeFactory('judge', 'judgment'));
      registry.createAll();

      registry.clear();

      assert.strictEqual(registry.factoryCount, 0);
      assert.strictEqual(registry.toolCount, 0);
      assert.deepStrictEqual(registry.getDomains(), []);
    });
  });
});
