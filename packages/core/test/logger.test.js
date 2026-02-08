/**
 * @cynic/core - Logger Tests
 *
 * Tests structured logging system:
 * - Log levels
 * - Logger instances
 * - Output formatting
 * - Configuration
 *
 * "All actions leave traces" - κυνικός
 *
 * @module @cynic/core/test/logger
 */

import { describe, it, beforeEach, afterEach, mock } from 'node:test';
import assert from 'node:assert/strict';

import {
  Logger,
  LogLevel,
  createLogger,
  setLogLevel,
  setJsonOutput,
  logger,
} from '../src/logger.js';

// =============================================================================
// LOG LEVEL TESTS
// =============================================================================

describe('LogLevel', () => {
  it('should have correct numeric values', () => {
    assert.strictEqual(LogLevel.TRACE, 0);
    assert.strictEqual(LogLevel.DEBUG, 1);
    assert.strictEqual(LogLevel.INFO, 2);
    assert.strictEqual(LogLevel.WARN, 3);
    assert.strictEqual(LogLevel.ERROR, 4);
    assert.strictEqual(LogLevel.FATAL, 5);
  });

  it('should have ascending severity', () => {
    assert.ok(LogLevel.TRACE < LogLevel.DEBUG);
    assert.ok(LogLevel.DEBUG < LogLevel.INFO);
    assert.ok(LogLevel.INFO < LogLevel.WARN);
    assert.ok(LogLevel.WARN < LogLevel.ERROR);
    assert.ok(LogLevel.ERROR < LogLevel.FATAL);
  });
});

// =============================================================================
// LOGGER CONSTRUCTION TESTS
// =============================================================================

describe('Logger', () => {
  describe('Construction', () => {
    it('should create logger with context', () => {
      const log = new Logger('MyModule');

      assert.strictEqual(log.context, 'MyModule');
    });

    it('should create child logger', () => {
      const parent = new Logger('Parent');
      const child = parent.child('Child');

      assert.strictEqual(child.context, 'Parent:Child');
    });

    it('should chain child loggers', () => {
      const root = new Logger('App');
      const service = root.child('UserService');
      const method = service.child('getUser');

      assert.strictEqual(method.context, 'App:UserService:getUser');
    });
  });

  describe('Logging Methods', () => {
    let log;
    let originalLog;
    let originalError;
    let capturedOutput;

    beforeEach(() => {
      log = new Logger('Test');
      capturedOutput = [];

      // Mock console
      originalLog = console.log;
      originalError = console.error;
      console.log = (...args) => capturedOutput.push({ type: 'log', args });
      console.error = (...args) => capturedOutput.push({ type: 'error', args });

      // Set level to TRACE to capture everything
      setLogLevel(LogLevel.TRACE);
    });

    afterEach(() => {
      console.log = originalLog;
      console.error = originalError;
      setLogLevel(LogLevel.INFO);
    });

    it('should have trace method', () => {
      assert.strictEqual(typeof log.trace, 'function');
    });

    it('should have debug method', () => {
      assert.strictEqual(typeof log.debug, 'function');
    });

    it('should have info method', () => {
      assert.strictEqual(typeof log.info, 'function');
    });

    it('should have warn method', () => {
      assert.strictEqual(typeof log.warn, 'function');
    });

    it('should have error method', () => {
      assert.strictEqual(typeof log.error, 'function');
    });

    it('should have fatal method', () => {
      assert.strictEqual(typeof log.fatal, 'function');
    });

    it('should output to stderr for all levels (stdout kept clean for hooks/MCP)', () => {
      log.info('Test message');

      assert.strictEqual(capturedOutput.length, 1);
      assert.strictEqual(capturedOutput[0].type, 'error');
    });

    it('should output to stderr for warn and above', () => {
      log.warn('Warning message');

      assert.strictEqual(capturedOutput.length, 1);
      assert.strictEqual(capturedOutput[0].type, 'error');
    });

    it('should include context in output', () => {
      log.info('Test message');

      const output = capturedOutput[0].args[0];
      assert.ok(output.includes('Test'), 'Should include context');
    });

    it('should include extra data', () => {
      log.info('User created', { userId: '123', email: 'test@test.com' });

      const output = capturedOutput[0].args[0];
      assert.ok(output.includes('userId') || output.includes('123'));
    });
  });
});

// =============================================================================
// LOG LEVEL FILTERING TESTS
// =============================================================================

describe('Log Level Filtering', () => {
  let log;
  let capturedOutput;
  let originalLog;
  let originalError;

  beforeEach(() => {
    log = new Logger('Filter');
    capturedOutput = [];
    originalLog = console.log;
    originalError = console.error;
    console.log = (...args) => capturedOutput.push(args);
    console.error = (...args) => capturedOutput.push(args);
  });

  afterEach(() => {
    console.log = originalLog;
    console.error = originalError;
    setLogLevel(LogLevel.INFO);
  });

  it('should filter out logs below current level', () => {
    setLogLevel(LogLevel.WARN);

    log.trace('Should not appear');
    log.debug('Should not appear');
    log.info('Should not appear');
    log.warn('Should appear');

    assert.strictEqual(capturedOutput.length, 1);
  });

  it('should allow all logs at TRACE level', () => {
    setLogLevel(LogLevel.TRACE);

    log.trace('Trace');
    log.debug('Debug');
    log.info('Info');

    assert.strictEqual(capturedOutput.length, 3);
  });

  it('should block all logs at level above FATAL', () => {
    setLogLevel(LogLevel.FATAL + 1);

    log.trace('No');
    log.debug('No');
    log.info('No');
    log.warn('No');
    log.error('No');
    log.fatal('No');

    assert.strictEqual(capturedOutput.length, 0);
  });
});

// =============================================================================
// SET LOG LEVEL TESTS
// =============================================================================

describe('setLogLevel', () => {
  afterEach(() => {
    setLogLevel(LogLevel.INFO);
  });

  it('should accept numeric level', () => {
    setLogLevel(LogLevel.DEBUG);

    // Logger should now output debug messages
    // (we verify by checking it doesn't throw)
    assert.doesNotThrow(() => setLogLevel(0));
    assert.doesNotThrow(() => setLogLevel(5));
  });

  it('should accept string level', () => {
    assert.doesNotThrow(() => setLogLevel('DEBUG'));
    assert.doesNotThrow(() => setLogLevel('INFO'));
    assert.doesNotThrow(() => setLogLevel('WARN'));
    assert.doesNotThrow(() => setLogLevel('ERROR'));
  });

  it('should accept lowercase string', () => {
    assert.doesNotThrow(() => setLogLevel('debug'));
    assert.doesNotThrow(() => setLogLevel('info'));
  });

  it('should default to INFO for invalid string', () => {
    // Should not throw, just default to INFO
    assert.doesNotThrow(() => setLogLevel('INVALID'));
  });
});

// =============================================================================
// JSON OUTPUT TESTS
// =============================================================================

describe('setJsonOutput', () => {
  let log;
  let capturedOutput;
  let originalLog;
  let originalError;

  beforeEach(() => {
    log = new Logger('JSON');
    capturedOutput = [];
    originalLog = console.log;
    originalError = console.error;
    console.log = (...args) => capturedOutput.push(args);
    console.error = (...args) => capturedOutput.push(args);
    setLogLevel(LogLevel.INFO);
  });

  afterEach(() => {
    console.log = originalLog;
    console.error = originalError;
    setJsonOutput(false);
    setLogLevel(LogLevel.INFO);
  });

  it('should output JSON when enabled', () => {
    setJsonOutput(true);
    log.info('Test message', { key: 'value' });

    const output = capturedOutput[0][0];
    // Should be valid JSON
    assert.doesNotThrow(() => JSON.parse(output));

    const parsed = JSON.parse(output);
    assert.strictEqual(parsed.message, 'Test message');
    assert.strictEqual(parsed.context, 'JSON');
    assert.strictEqual(parsed.key, 'value');
  });

  it('should output text when disabled', () => {
    setJsonOutput(false);
    log.info('Test message');

    const output = capturedOutput[0][0];
    // Should NOT be pure JSON (may contain colors, spaces)
    assert.ok(output.includes('Test message'));
  });
});

// =============================================================================
// CREATE LOGGER TESTS
// =============================================================================

describe('createLogger', () => {
  it('should create new Logger instance', () => {
    const log = createLogger('MyContext');

    assert.ok(log instanceof Logger);
    assert.strictEqual(log.context, 'MyContext');
  });

  it('should create independent loggers', () => {
    const log1 = createLogger('Context1');
    const log2 = createLogger('Context2');

    assert.notStrictEqual(log1, log2);
    assert.strictEqual(log1.context, 'Context1');
    assert.strictEqual(log2.context, 'Context2');
  });
});

// =============================================================================
// DEFAULT LOGGER TESTS
// =============================================================================

describe('Default Logger', () => {
  it('should export default logger instance', () => {
    assert.ok(logger instanceof Logger);
  });

  it('should have CYNIC context', () => {
    assert.strictEqual(logger.context, 'CYNIC');
  });

  it('should have all logging methods', () => {
    assert.strictEqual(typeof logger.trace, 'function');
    assert.strictEqual(typeof logger.debug, 'function');
    assert.strictEqual(typeof logger.info, 'function');
    assert.strictEqual(typeof logger.warn, 'function');
    assert.strictEqual(typeof logger.error, 'function');
    assert.strictEqual(typeof logger.fatal, 'function');
  });
});

// =============================================================================
// EDGE CASES
// =============================================================================

describe('Edge Cases', () => {
  let log;
  let capturedOutput;
  let originalLog;

  beforeEach(() => {
    log = new Logger('Edge');
    capturedOutput = [];
    originalLog = console.log;
    console.log = (...args) => capturedOutput.push(args);
    setLogLevel(LogLevel.INFO);
    setJsonOutput(false);
  });

  afterEach(() => {
    console.log = originalLog;
  });

  it('should handle empty message', () => {
    assert.doesNotThrow(() => log.info(''));
  });

  it('should handle null extra data', () => {
    assert.doesNotThrow(() => log.info('Message', null));
  });

  it('should handle undefined extra data', () => {
    assert.doesNotThrow(() => log.info('Message', undefined));
  });

  it('should handle complex extra data', () => {
    assert.doesNotThrow(() => log.info('Message', {
      nested: { deep: { value: 123 } },
      array: [1, 2, 3],
      date: new Date().toISOString(),
    }));
  });

  it('should handle special characters in message', () => {
    assert.doesNotThrow(() => log.info('Message with "quotes" and \\backslash'));
  });

  it('should handle unicode in message', () => {
    assert.doesNotThrow(() => log.info('φ⁻¹ = 0.618 - κυνικός'));
  });

  it('should handle very long message', () => {
    const longMessage = 'x'.repeat(10000);
    assert.doesNotThrow(() => log.info(longMessage));
  });

  it('should handle empty context', () => {
    const emptyLog = new Logger('');
    assert.doesNotThrow(() => emptyLog.info('Message'));
  });

  it('should handle context with special characters', () => {
    const specialLog = new Logger('Module:Sub:Sub2');
    assert.doesNotThrow(() => specialLog.info('Message'));
  });
});
