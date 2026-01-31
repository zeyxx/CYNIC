/**
 * @cynic/node - Agent Booster Tests
 *
 * Tests for fast code transforms without LLM.
 *
 * @module @cynic/node/test/agent-booster
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert/strict';

import {
  AgentBooster,
  createAgentBooster,
  TransformIntent,
  TransformStatus,
} from '../src/routing/agent-booster.js';

// =============================================================================
// CONSTANTS TESTS
// =============================================================================

describe('TransformIntent', () => {
  it('should have all 12 transform intents', () => {
    const intents = Object.values(TransformIntent);
    assert.strictEqual(intents.length, 12);
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(TransformIntent));
  });

  it('should have expected intents', () => {
    assert.strictEqual(TransformIntent.VAR_TO_CONST, 'var-to-const');
    assert.strictEqual(TransformIntent.VAR_TO_LET, 'var-to-let');
    assert.strictEqual(TransformIntent.ADD_TYPES, 'add-types');
    assert.strictEqual(TransformIntent.REMOVE_CONSOLE, 'remove-console');
    assert.strictEqual(TransformIntent.SORT_IMPORTS, 'sort-imports');
  });
});

describe('TransformStatus', () => {
  it('should have all status values', () => {
    assert.strictEqual(TransformStatus.SUCCESS, 'success');
    assert.strictEqual(TransformStatus.NO_CHANGE, 'no_change');
    assert.strictEqual(TransformStatus.PARTIAL, 'partial');
    assert.strictEqual(TransformStatus.ERROR, 'error');
    assert.strictEqual(TransformStatus.UNSUPPORTED, 'unsupported');
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(TransformStatus));
  });
});

// =============================================================================
// AGENT BOOSTER CONSTRUCTION
// =============================================================================

describe('AgentBooster', () => {
  let booster;

  beforeEach(() => {
    booster = createAgentBooster();
  });

  describe('Construction', () => {
    it('should create with factory', () => {
      const b = createAgentBooster();
      assert.ok(b instanceof AgentBooster);
    });

    it('should initialize stats', () => {
      const stats = booster.getStats();
      assert.strictEqual(stats.transforms, 0);
      assert.strictEqual(stats.totalSaved, 0);
      assert.strictEqual(stats.avgTimeMs, 0);
    });

    it('should initialize byIntent stats for all intents', () => {
      const stats = booster.getStats();
      for (const intent of Object.values(TransformIntent)) {
        assert.strictEqual(stats.byIntent[intent], 0);
      }
    });
  });

  // ===========================================================================
  // INTENT DETECTION
  // ===========================================================================

  describe('canHandle()', () => {
    describe('VAR_TO_CONST detection', () => {
      const cases = [
        'convert var to const',
        'change all vars to const',
        'replace var with const',
        'var to const',
      ];

      for (const request of cases) {
        it(`should detect "${request}"`, () => {
          const result = booster.canHandle(request);
          assert.ok(result, `Should handle "${request}"`);
          assert.strictEqual(result.intent, TransformIntent.VAR_TO_CONST);
        });
      }
    });

    describe('VAR_TO_LET detection', () => {
      const cases = [
        'convert var to let',
        'change vars to let',
        'var to let',
      ];

      for (const request of cases) {
        it(`should detect "${request}"`, () => {
          const result = booster.canHandle(request);
          assert.ok(result);
          assert.strictEqual(result.intent, TransformIntent.VAR_TO_LET);
        });
      }
    });

    describe('Other intent detection', () => {
      const cases = [
        { request: 'add types', intent: TransformIntent.ADD_TYPES },
        { request: 'add typescript types', intent: TransformIntent.ADD_TYPES },
        { request: 'add async await', intent: TransformIntent.ADD_ASYNC_AWAIT },
        { request: 'convert to async-await', intent: TransformIntent.ADD_ASYNC_AWAIT },
        { request: 'add error handling', intent: TransformIntent.ADD_ERROR_HANDLING },
        { request: 'wrap in try-catch', intent: TransformIntent.ADD_ERROR_HANDLING },
        { request: 'add logging', intent: TransformIntent.ADD_LOGGING },
        { request: 'add logs', intent: TransformIntent.ADD_LOGGING },
        { request: 'remove console.log', intent: TransformIntent.REMOVE_CONSOLE },
        { request: 'strip all console', intent: TransformIntent.REMOVE_CONSOLE },
        { request: 'remove debugger', intent: TransformIntent.REMOVE_DEBUGGER },
        { request: 'delete all debugger', intent: TransformIntent.REMOVE_DEBUGGER },
        { request: 'add semicolons', intent: TransformIntent.ADD_SEMICOLONS },
        { request: 'remove unused imports', intent: TransformIntent.REMOVE_UNUSED_IMPORTS },
        { request: 'clean imports', intent: TransformIntent.REMOVE_UNUSED_IMPORTS },
        { request: 'sort imports', intent: TransformIntent.SORT_IMPORTS },
        { request: 'add use strict', intent: TransformIntent.ADD_STRICT_MODE },
      ];

      for (const { request, intent } of cases) {
        it(`should detect "${request}" as ${intent}`, () => {
          const result = booster.canHandle(request);
          assert.ok(result, `Should handle "${request}"`);
          assert.strictEqual(result.intent, intent);
        });
      }
    });

    describe('Unhandled requests', () => {
      const cases = [
        'refactor the entire codebase',
        'add authentication',
        'optimize performance',
        'explain this code',
        '',
        null,
      ];

      for (const request of cases) {
        it(`should not handle "${request}"`, () => {
          const result = booster.canHandle(request);
          assert.strictEqual(result, null);
        });
      }
    });
  });

  // ===========================================================================
  // TRANSFORM TESTS
  // ===========================================================================

  describe('transform()', () => {
    describe('VAR_TO_CONST', () => {
      it('should convert single var to const', () => {
        const code = 'var x = 5;';
        const result = booster.transform({
          code,
          intent: TransformIntent.VAR_TO_CONST,
        });

        assert.strictEqual(result.status, TransformStatus.SUCCESS);
        assert.ok(result.code.includes('const x ='));
        assert.ok(!result.code.includes('var'));
      });

      it('should convert multiple vars to const', () => {
        const code = `var a = 1;
var b = 2;
var c = 3;`;
        const result = booster.transform({
          code,
          intent: TransformIntent.VAR_TO_CONST,
        });

        assert.strictEqual(result.status, TransformStatus.SUCCESS);
        assert.ok(result.code.includes('const a ='));
        assert.ok(result.code.includes('const b ='));
        assert.ok(result.code.includes('const c ='));
      });

      it('should not convert reassigned vars', () => {
        const code = `var x = 1;
x = 2;`;
        const result = booster.transform({
          code,
          intent: TransformIntent.VAR_TO_CONST,
        });

        // Should be partial or no_change since x is reassigned
        assert.ok(result.status === TransformStatus.NO_CHANGE ||
                  result.status === TransformStatus.PARTIAL);
      });

      it('should handle no vars', () => {
        const code = 'const x = 5;';
        const result = booster.transform({
          code,
          intent: TransformIntent.VAR_TO_CONST,
        });

        assert.strictEqual(result.status, TransformStatus.NO_CHANGE);
      });
    });

    describe('VAR_TO_LET', () => {
      it('should convert all vars to let', () => {
        const code = `var a = 1;
var b = 2;`;
        const result = booster.transform({
          code,
          intent: TransformIntent.VAR_TO_LET,
        });

        assert.strictEqual(result.status, TransformStatus.SUCCESS);
        assert.ok(result.code.includes('let a'));
        assert.ok(result.code.includes('let b'));
        assert.ok(!result.code.includes('var'));
      });

      it('should handle no vars', () => {
        const code = 'let x = 5;';
        const result = booster.transform({
          code,
          intent: TransformIntent.VAR_TO_LET,
        });

        assert.strictEqual(result.status, TransformStatus.NO_CHANGE);
      });
    });

    describe('ADD_TYPES', () => {
      it('should add string type', () => {
        const code = `const name = "hello";`;
        const result = booster.transform({
          code,
          intent: TransformIntent.ADD_TYPES,
        });

        assert.strictEqual(result.status, TransformStatus.PARTIAL);
        assert.ok(result.code.includes(': string'));
      });

      it('should add number type', () => {
        const code = `const count = 42;`;
        const result = booster.transform({
          code,
          intent: TransformIntent.ADD_TYPES,
        });

        assert.strictEqual(result.status, TransformStatus.PARTIAL);
        assert.ok(result.code.includes(': number'));
      });

      it('should add boolean type', () => {
        const code = `const flag = true;`;
        const result = booster.transform({
          code,
          intent: TransformIntent.ADD_TYPES,
        });

        assert.strictEqual(result.status, TransformStatus.PARTIAL);
        assert.ok(result.code.includes(': boolean'));
      });
    });

    describe('ADD_ASYNC_AWAIT', () => {
      it('should convert .then() to await', () => {
        // Pattern expects: variable.then(param => {
        const code = `function getData() {
  promise.then(result => {
    console.log(result);
  });
}`;
        const result = booster.transform({
          code,
          intent: TransformIntent.ADD_ASYNC_AWAIT,
        });

        assert.strictEqual(result.status, TransformStatus.PARTIAL);
        assert.ok(result.code.includes('await promise'));
        assert.ok(result.code.includes('async function'));
      });

      it('should handle no .then()', () => {
        const code = `async function getData() {
  const result = await fetch('/api');
}`;
        const result = booster.transform({
          code,
          intent: TransformIntent.ADD_ASYNC_AWAIT,
        });

        assert.strictEqual(result.status, TransformStatus.NO_CHANGE);
      });
    });

    describe('ADD_ERROR_HANDLING', () => {
      it('should wrap in try-catch', () => {
        const code = `const data = JSON.parse(input);
console.log(data);`;
        const result = booster.transform({
          code,
          intent: TransformIntent.ADD_ERROR_HANDLING,
        });

        assert.strictEqual(result.status, TransformStatus.SUCCESS);
        assert.ok(result.code.includes('try {'));
        assert.ok(result.code.includes('} catch (error) {'));
        assert.ok(result.code.includes('throw error'));
      });

      it('should not double-wrap existing try-catch', () => {
        const code = `try {
  doSomething();
} catch (e) {
  console.error(e);
}`;
        const result = booster.transform({
          code,
          intent: TransformIntent.ADD_ERROR_HANDLING,
        });

        assert.strictEqual(result.status, TransformStatus.NO_CHANGE);
      });
    });

    describe('ADD_LOGGING', () => {
      it('should add logging to functions', () => {
        const code = `function processData(data) {
  return data.map(x => x * 2);
}`;
        const result = booster.transform({
          code,
          intent: TransformIntent.ADD_LOGGING,
        });

        assert.strictEqual(result.status, TransformStatus.SUCCESS);
        assert.ok(result.code.includes("console.log('[processData] called')"));
      });

      it('should add logging to arrow functions', () => {
        const code = `const processData = (data) => {
  return data.map(x => x * 2);
};`;
        const result = booster.transform({
          code,
          intent: TransformIntent.ADD_LOGGING,
        });

        assert.strictEqual(result.status, TransformStatus.SUCCESS);
        assert.ok(result.code.includes("console.log('[processData] called')"));
      });
    });

    describe('REMOVE_CONSOLE', () => {
      it('should remove console.log statements', () => {
        const code = `console.log('debug');
const x = 5;
console.log('more debug');`;
        const result = booster.transform({
          code,
          intent: TransformIntent.REMOVE_CONSOLE,
        });

        assert.strictEqual(result.status, TransformStatus.SUCCESS);
        assert.ok(!result.code.includes('console.log'));
        assert.ok(result.code.includes('const x = 5'));
      });

      it('should handle no console statements', () => {
        const code = 'const x = 5;';
        const result = booster.transform({
          code,
          intent: TransformIntent.REMOVE_CONSOLE,
        });

        assert.strictEqual(result.status, TransformStatus.NO_CHANGE);
      });
    });

    describe('REMOVE_DEBUGGER', () => {
      it('should remove debugger statements', () => {
        const code = `function test() {
  debugger;
  return 5;
}`;
        const result = booster.transform({
          code,
          intent: TransformIntent.REMOVE_DEBUGGER,
        });

        assert.strictEqual(result.status, TransformStatus.SUCCESS);
        assert.ok(!result.code.includes('debugger'));
      });

      it('should handle no debugger statements', () => {
        const code = 'const x = 5;';
        const result = booster.transform({
          code,
          intent: TransformIntent.REMOVE_DEBUGGER,
        });

        assert.strictEqual(result.status, TransformStatus.NO_CHANGE);
      });
    });

    describe('ADD_SEMICOLONS', () => {
      it('should add missing semicolons', () => {
        const code = `const x = 5
let y = 10
return x + y`;
        const result = booster.transform({
          code,
          intent: TransformIntent.ADD_SEMICOLONS,
        });

        assert.strictEqual(result.status, TransformStatus.SUCCESS);
        assert.ok(result.code.includes('const x = 5;'));
        assert.ok(result.code.includes('let y = 10;'));
      });

      it('should handle code with semicolons', () => {
        const code = `const x = 5;
let y = 10;`;
        const result = booster.transform({
          code,
          intent: TransformIntent.ADD_SEMICOLONS,
        });

        assert.strictEqual(result.status, TransformStatus.NO_CHANGE);
      });
    });

    describe('REMOVE_UNUSED_IMPORTS', () => {
      it('should remove unused imports', () => {
        const code = `import { foo, bar, baz } from './module';

const result = foo();`;
        const result = booster.transform({
          code,
          intent: TransformIntent.REMOVE_UNUSED_IMPORTS,
        });

        assert.strictEqual(result.status, TransformStatus.SUCCESS);
        assert.ok(result.code.includes('foo'));
        assert.ok(!result.code.includes('bar,'));
        assert.ok(!result.code.includes('baz'));
      });

      it('should keep all used imports', () => {
        const code = `import { foo, bar } from './module';

foo();
bar();`;
        const result = booster.transform({
          code,
          intent: TransformIntent.REMOVE_UNUSED_IMPORTS,
        });

        assert.strictEqual(result.status, TransformStatus.NO_CHANGE);
      });

      it('should handle no imports', () => {
        const code = 'const x = 5;';
        const result = booster.transform({
          code,
          intent: TransformIntent.REMOVE_UNUSED_IMPORTS,
        });

        assert.strictEqual(result.status, TransformStatus.NO_CHANGE);
      });
    });

    describe('SORT_IMPORTS', () => {
      it('should sort imports alphabetically', () => {
        const code = `import { z } from 'zebra';
import { a } from 'alpha';
import { m } from 'middle';

const x = 5;`;
        const result = booster.transform({
          code,
          intent: TransformIntent.SORT_IMPORTS,
        });

        assert.strictEqual(result.status, TransformStatus.SUCCESS);
        const lines = result.code.split('\n');
        assert.ok(lines[0].includes('alpha'));
        assert.ok(lines[1].includes('middle'));
        assert.ok(lines[2].includes('zebra'));
      });

      it('should handle already sorted imports', () => {
        const code = `import { a } from 'alpha';
import { z } from 'zebra';

const x = 5;`;
        const result = booster.transform({
          code,
          intent: TransformIntent.SORT_IMPORTS,
        });

        assert.strictEqual(result.status, TransformStatus.NO_CHANGE);
      });

      it('should handle single import', () => {
        const code = `import { a } from 'alpha';

const x = 5;`;
        const result = booster.transform({
          code,
          intent: TransformIntent.SORT_IMPORTS,
        });

        assert.strictEqual(result.status, TransformStatus.NO_CHANGE);
      });
    });

    describe('ADD_STRICT_MODE', () => {
      it('should add use strict', () => {
        const code = `const x = 5;
console.log(x);`;
        const result = booster.transform({
          code,
          intent: TransformIntent.ADD_STRICT_MODE,
        });

        assert.strictEqual(result.status, TransformStatus.SUCCESS);
        assert.ok(result.code.startsWith("'use strict'"));
      });

      it('should not double-add use strict', () => {
        const code = `'use strict';
const x = 5;`;
        const result = booster.transform({
          code,
          intent: TransformIntent.ADD_STRICT_MODE,
        });

        assert.strictEqual(result.status, TransformStatus.NO_CHANGE);
      });
    });

    describe('Error handling', () => {
      it('should handle invalid code input', () => {
        const result = booster.transform({
          code: null,
          intent: TransformIntent.VAR_TO_CONST,
        });

        assert.strictEqual(result.status, TransformStatus.ERROR);
      });

      it('should handle unknown intent', () => {
        const result = booster.transform({
          code: 'const x = 5;',
          intent: 'unknown-intent',
        });

        assert.strictEqual(result.status, TransformStatus.UNSUPPORTED);
      });
    });
  });

  // ===========================================================================
  // STATISTICS
  // ===========================================================================

  describe('Statistics', () => {
    it('should track transform count', () => {
      booster.transform({ code: 'var x = 5;', intent: TransformIntent.VAR_TO_CONST });
      booster.transform({ code: 'var y = 10;', intent: TransformIntent.VAR_TO_LET });

      const stats = booster.getStats();
      assert.strictEqual(stats.transforms, 2);
    });

    it('should track by intent', () => {
      booster.transform({ code: 'var x = 5;', intent: TransformIntent.VAR_TO_CONST });
      booster.transform({ code: 'var y = 10;', intent: TransformIntent.VAR_TO_CONST });
      booster.transform({ code: 'var z = 15;', intent: TransformIntent.VAR_TO_LET });

      const stats = booster.getStats();
      assert.strictEqual(stats.byIntent[TransformIntent.VAR_TO_CONST], 2);
      assert.strictEqual(stats.byIntent[TransformIntent.VAR_TO_LET], 1);
    });

    it('should track saved LLM calls', () => {
      booster.transform({ code: 'var x = 5;', intent: TransformIntent.VAR_TO_CONST });
      booster.transform({ code: "'use strict';\nconst x = 5;", intent: TransformIntent.ADD_STRICT_MODE });

      const stats = booster.getStats();
      // Only successful transforms count
      assert.strictEqual(stats.totalSaved, 1);
    });

    it('should track average time', () => {
      booster.transform({ code: 'var x = 5;', intent: TransformIntent.VAR_TO_CONST });
      booster.transform({ code: 'var y = 10;', intent: TransformIntent.VAR_TO_LET });

      const stats = booster.getStats();
      assert.ok(stats.avgTimeMs >= 0);
    });

    it('should reset stats', () => {
      booster.transform({ code: 'var x = 5;', intent: TransformIntent.VAR_TO_CONST });
      booster.resetStats();

      const stats = booster.getStats();
      assert.strictEqual(stats.transforms, 0);
      assert.strictEqual(stats.totalSaved, 0);
    });
  });

  // ===========================================================================
  // EVENTS
  // ===========================================================================

  describe('Events', () => {
    it('should emit transform event', async () => {
      const events = [];
      booster.on('transform', (data) => events.push(data));

      booster.transform({ code: 'var x = 5;', intent: TransformIntent.VAR_TO_CONST });

      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].status, TransformStatus.SUCCESS);
      assert.ok(events[0].elapsed >= 0);
    });
  });

  // ===========================================================================
  // PERFORMANCE
  // ===========================================================================

  describe('Performance', () => {
    it('should complete transforms in <10ms', () => {
      const code = `var a = 1;
var b = 2;
var c = 3;
console.log(a, b, c);`;

      const start = performance.now();
      booster.transform({ code, intent: TransformIntent.VAR_TO_CONST });
      const elapsed = performance.now() - start;

      assert.ok(elapsed < 10, `Transform should be fast, took ${elapsed}ms`);
    });

    it('should handle bulk transforms efficiently', () => {
      const code = 'var x = 5;';
      const start = performance.now();

      for (let i = 0; i < 100; i++) {
        booster.transform({ code, intent: TransformIntent.VAR_TO_CONST });
      }

      const elapsed = performance.now() - start;
      const avgTime = elapsed / 100;

      assert.ok(avgTime < 1, `Average transform time should be <1ms, was ${avgTime}ms`);
    });
  });
});

// =============================================================================
// INTEGRATION TESTS
// =============================================================================

describe('Agent Booster Integration', () => {
  it('should handle detect-then-transform workflow', () => {
    const booster = createAgentBooster();

    const request = 'convert all var to const';
    const detection = booster.canHandle(request);

    assert.ok(detection);
    assert.strictEqual(detection.intent, TransformIntent.VAR_TO_CONST);

    const code = `var name = "test";
var count = 42;`;

    const result = booster.transform({
      code,
      intent: detection.intent,
    });

    assert.strictEqual(result.status, TransformStatus.SUCCESS);
    assert.ok(result.code.includes('const name'));
    assert.ok(result.code.includes('const count'));
  });

  it('should chain multiple transforms', () => {
    const booster = createAgentBooster();

    let code = `var name = "test"
console.log(name)
debugger`;

    // Convert var to const
    const result1 = booster.transform({
      code,
      intent: TransformIntent.VAR_TO_CONST,
    });

    // Remove console
    const result2 = booster.transform({
      code: result1.code,
      intent: TransformIntent.REMOVE_CONSOLE,
    });

    // Remove debugger
    const result3 = booster.transform({
      code: result2.code,
      intent: TransformIntent.REMOVE_DEBUGGER,
    });

    // Add semicolons
    const result4 = booster.transform({
      code: result3.code,
      intent: TransformIntent.ADD_SEMICOLONS,
    });

    assert.ok(!result4.code.includes('var'));
    assert.ok(!result4.code.includes('console'));
    assert.ok(!result4.code.includes('debugger'));

    const stats = booster.getStats();
    assert.strictEqual(stats.transforms, 4);
  });

  it('should maintain code integrity through transforms', () => {
    const booster = createAgentBooster();

    const originalCode = `import { foo } from './foo';

function processData(data) {
  var result = data.map(x => x * 2);
  console.log('Result:', result);
  return result;
}

export { processData };`;

    // Apply var-to-const
    const result = booster.transform({
      code: originalCode,
      intent: TransformIntent.VAR_TO_CONST,
    });

    // Should preserve imports and exports
    assert.ok(result.code.includes("import { foo }"));
    assert.ok(result.code.includes("export { processData }"));
    assert.ok(result.code.includes("function processData"));
    assert.ok(result.code.includes("const result"));
  });
});
