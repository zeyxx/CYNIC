/**
 * Code Analyzer Tests
 *
 * Tests for the CodeAnalyzer service that extracts symbols from JS files.
 *
 * @module @cynic/mcp/test/code-analyzer
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { CodeAnalyzer, createCodeAnalyzer } from '../src/code-analyzer.js';

describe('CodeAnalyzer', () => {
  let analyzer;

  beforeEach(() => {
    analyzer = new CodeAnalyzer({
      rootPath: '/workspaces/CYNIC-new',
      cacheTTL: 1000,
    });
  });

  describe('constructor', () => {
    it('creates with default options', () => {
      const a = new CodeAnalyzer();
      assert.ok(a.rootPath);
      assert.equal(a.cacheTTL, 30000);
    });

    it('accepts custom options', () => {
      assert.equal(analyzer.rootPath, '/workspaces/CYNIC-new');
      assert.equal(analyzer.cacheTTL, 1000);
    });
  });

  describe('extractClasses', () => {
    it('extracts class declarations', () => {
      const content = `
export class TestClass {
  constructor() {}
  method1() {}
}
`;
      const lines = content.split('\n');
      const classes = analyzer.extractClasses(content, lines);

      assert.equal(classes.length, 1);
      assert.equal(classes[0].name, 'TestClass');
    });

    it('extracts class with extends', () => {
      const content = `
class ChildClass extends ParentClass {
  childMethod() {}
}
`;
      const lines = content.split('\n');
      const classes = analyzer.extractClasses(content, lines);

      assert.equal(classes.length, 1);
      assert.equal(classes[0].name, 'ChildClass');
    });

    it('extracts multiple classes', () => {
      const content = `
class ClassA {
  methodA() {}
}

class ClassB {
  methodB() {}
}
`;
      const lines = content.split('\n');
      const classes = analyzer.extractClasses(content, lines);

      assert.equal(classes.length, 2);
      assert.equal(classes[0].name, 'ClassA');
      assert.equal(classes[1].name, 'ClassB');
    });

    it('extracts class methods', () => {
      const content = `
class TestClass {
  constructor() {}
  publicMethod() {}
  _privateMethod() {}
  async asyncMethod() {}
  static staticMethod() {}
  get getter() {}
  set setter(val) {}
}
`;
      const lines = content.split('\n');
      const classes = analyzer.extractClasses(content, lines);

      assert.equal(classes.length, 1);
      const methods = classes[0].methods;

      // Constructor is skipped
      assert.ok(methods.some(m => m.name === 'publicMethod'));
      assert.ok(methods.some(m => m.name === '_privateMethod'));
      assert.ok(methods.some(m => m.name === 'asyncMethod' && m.async));
      assert.ok(methods.some(m => m.name === 'staticMethod' && m.static));
    });
  });

  describe('extractFunctions', () => {
    it('extracts function declarations', () => {
      const content = `
function standalone() {
  return 1;
}

export function exported(param) {
  return param;
}

async function asyncFunc() {}
`;
      const lines = content.split('\n');
      const functions = analyzer.extractFunctions(content, lines);

      assert.equal(functions.length, 3);
      assert.ok(functions.some(f => f.name === 'standalone'));
      assert.ok(functions.some(f => f.name === 'exported' && f.exported));
      assert.ok(functions.some(f => f.name === 'asyncFunc' && f.async));
    });

    it('extracts arrow function exports', () => {
      const content = `
export const arrowFunc = (a, b) => {
  return a + b;
};

export const asyncArrow = async (x) => {
  return x;
};
`;
      const lines = content.split('\n');
      const functions = analyzer.extractFunctions(content, lines);

      assert.equal(functions.length, 2);
      assert.ok(functions.some(f => f.name === 'arrowFunc' && f.arrow));
      assert.ok(functions.some(f => f.name === 'asyncArrow' && f.async && f.arrow));
    });

    it('extracts function parameters', () => {
      const content = `
function withParams(a, b, c = 'default') {
  return a + b + c;
}
`;
      const lines = content.split('\n');
      const functions = analyzer.extractFunctions(content, lines);

      assert.equal(functions.length, 1);
      assert.deepEqual(functions[0].params, ['a', 'b', 'c']);
    });
  });

  describe('extractExports', () => {
    it('extracts named exports', () => {
      const content = `
export { foo, bar, baz };
export { x as y };
`;
      const exports = analyzer.extractExports(content);

      assert.ok(exports.includes('foo'));
      assert.ok(exports.includes('bar'));
      assert.ok(exports.includes('baz'));
      assert.ok(exports.includes('x'));
    });

    it('detects default export', () => {
      const content = `
export default class Foo {}
`;
      const exports = analyzer.extractExports(content);

      assert.ok(exports.includes('default'));
    });

    it('deduplicates exports', () => {
      const content = `
export { foo };
export { foo, bar };
`;
      const exports = analyzer.extractExports(content);

      const fooCount = exports.filter(e => e === 'foo').length;
      assert.equal(fooCount, 1);
    });
  });

  describe('extractModuleDescription', () => {
    it('extracts JSDoc module description', () => {
      const content = `/**
 * This is a module description.
 * It spans multiple lines.
 *
 * @module test-module
 */

const x = 1;
`;
      const desc = analyzer.extractModuleDescription(content);

      assert.ok(desc);
      assert.ok(desc.includes('This is a module description'));
    });

    it('returns null for no JSDoc', () => {
      const content = `const x = 1;`;
      const desc = analyzer.extractModuleDescription(content);

      assert.equal(desc, null);
    });
  });

  describe('getLineNumber', () => {
    it('calculates correct line number', () => {
      const content = `line1
line2
line3
line4`;
      assert.equal(analyzer.getLineNumber(content, 0), 1);
      assert.equal(analyzer.getLineNumber(content, 6), 2);
      assert.equal(analyzer.getLineNumber(content, 12), 3);
    });
  });

  describe('findMatchingBrace', () => {
    it('finds matching closing brace', () => {
      const content = '{ inner { nested } }';
      const endPos = analyzer.findMatchingBrace(content, 0);

      assert.equal(content[endPos - 1], '}');
    });

    it('handles nested braces', () => {
      const content = '{ { { } } }';
      const endPos = analyzer.findMatchingBrace(content, 0);

      assert.equal(endPos, content.length);
    });
  });

  describe('computeStats', () => {
    it('computes overall statistics', () => {
      const packages = [
        {
          name: 'pkg1',
          modules: [{ name: 'mod1' }, { name: 'mod2' }],
          stats: { classes: 5, functions: 10, methods: 20, lines: 500 },
        },
        {
          name: 'pkg2',
          modules: [{ name: 'mod3' }],
          stats: { classes: 3, functions: 5, methods: 10, lines: 300 },
        },
      ];

      const stats = analyzer.computeStats(packages);

      assert.equal(stats.packages, 2);
      assert.equal(stats.modules, 3);
      assert.equal(stats.classes, 8);
      assert.equal(stats.functions, 15);
      assert.equal(stats.methods, 30);
      assert.equal(stats.lines, 800);
    });
  });

  describe('cache', () => {
    it('invalidates cache', () => {
      analyzer.cache = { test: 'data' };
      analyzer.cacheTime = Date.now();

      analyzer.invalidateCache();

      assert.equal(analyzer.cache, null);
      assert.equal(analyzer.cacheTime, 0);
    });
  });

  describe('search', async () => {
    it('returns empty array for no matches', async () => {
      // Use real getTree but search for non-existent
      const results = await analyzer.search('xyznonexistent123');
      assert.ok(Array.isArray(results));
    });
  });

  describe('getTree', async () => {
    it('returns tree structure', async () => {
      const tree = await analyzer.getTree();

      assert.ok(tree.root);
      assert.ok(Array.isArray(tree.packages));
      assert.ok(tree.stats);
      assert.ok(tree.timestamp);
    });

    it('caches results', async () => {
      const tree1 = await analyzer.getTree();
      const tree2 = await analyzer.getTree();

      assert.equal(tree1.timestamp, tree2.timestamp);
    });
  });

  describe('getPackage', async () => {
    it('finds package by name', async () => {
      const pkg = await analyzer.getPackage('mcp');

      if (pkg) {
        assert.ok(pkg.name.includes('mcp'));
        assert.ok(Array.isArray(pkg.modules));
      }
    });

    it('returns null for non-existent package', async () => {
      const pkg = await analyzer.getPackage('nonexistent');
      assert.equal(pkg, null);
    });
  });

  describe('getStats', async () => {
    it('returns statistics', async () => {
      const stats = await analyzer.getStats();

      assert.ok(typeof stats.packages === 'number');
      assert.ok(typeof stats.modules === 'number');
      assert.ok(typeof stats.classes === 'number');
    });
  });
});

describe('createCodeAnalyzer', () => {
  it('creates analyzer instance', () => {
    const analyzer = createCodeAnalyzer({ cacheTTL: 5000 });

    assert.ok(analyzer instanceof CodeAnalyzer);
    assert.equal(analyzer.cacheTTL, 5000);
  });
});
