/**
 * @cynic/core - CLI Utilities Tests
 *
 * v1.1: Tests for CLI progress, colors, and status display
 *
 * @module @cynic/core/test/cli
 */

import { describe, it, mock, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  ANSI,
  Colors,
  supportsColor,
  colorize,
  getThresholdColor,
  BAR_CHARS,
  progressBar,
  SPINNER_FRAMES,
  createSpinner,
  statusLine,
  DOG_EMOJI,
  truncate,
  pad,
  formatDuration,
  formatBytes,
  formatTime,
  formatDate,
  BOX,
  horizontalLine,
  box,
} from '../src/cli/index.js';

// =============================================================================
// ANSI CONSTANTS TESTS
// =============================================================================

describe('ANSI constants', () => {
  it('should have reset code', () => {
    assert.strictEqual(ANSI.reset, '\x1b[0m');
  });

  it('should have style codes', () => {
    assert.ok(ANSI.bold);
    assert.ok(ANSI.dim);
    assert.ok(ANSI.italic);
    assert.ok(ANSI.underline);
  });

  it('should have color codes', () => {
    assert.ok(ANSI.red);
    assert.ok(ANSI.green);
    assert.ok(ANSI.yellow);
    assert.ok(ANSI.blue);
    assert.ok(ANSI.magenta);
    assert.ok(ANSI.cyan);
    assert.ok(ANSI.white);
  });

  it('should have cursor control codes', () => {
    assert.ok(ANSI.cursorUp);
    assert.ok(ANSI.clearLine);
    assert.ok(ANSI.saveCursor);
    assert.ok(ANSI.restoreCursor);
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(ANSI));
  });
});

describe('Colors', () => {
  it('should have brand colors', () => {
    assert.ok(Colors.cynic);
    assert.ok(Colors.phi);
  });

  it('should have status colors', () => {
    assert.ok(Colors.success);
    assert.ok(Colors.warning);
    assert.ok(Colors.danger);
    assert.ok(Colors.info);
    assert.ok(Colors.muted);
  });

  it('should have verdict colors', () => {
    assert.ok(Colors.howl);
    assert.ok(Colors.wag);
    assert.ok(Colors.growl);
    assert.ok(Colors.bark);
  });

  it('should have Dog (Sefirot) colors', () => {
    assert.ok(Colors.keter);
    assert.ok(Colors.chochmah);
    assert.ok(Colors.binah);
    assert.ok(Colors.chesed);
    assert.ok(Colors.gevurah);
    assert.ok(Colors.tiferet);
    assert.ok(Colors.netzach);
    assert.ok(Colors.hod);
    assert.ok(Colors.yesod);
    assert.ok(Colors.malkhut);
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(Colors));
  });
});

describe('colorize', () => {
  const originalEnv = process.env.NO_COLOR;

  afterEach(() => {
    if (originalEnv !== undefined) {
      process.env.NO_COLOR = originalEnv;
    } else {
      delete process.env.NO_COLOR;
    }
  });

  it('should colorize text when colors supported', () => {
    delete process.env.NO_COLOR;
    process.env.FORCE_COLOR = '1';
    const result = colorize('test', ANSI.red);
    assert.ok(result.includes(ANSI.red));
    assert.ok(result.includes('test'));
    assert.ok(result.includes(ANSI.reset));
    delete process.env.FORCE_COLOR;
  });

  it('should return plain text when colors disabled', () => {
    process.env.NO_COLOR = '1';
    const result = colorize('test', ANSI.red);
    assert.strictEqual(result, 'test');
  });
});

describe('getThresholdColor', () => {
  it('should return howl for high values (>= 85.4%)', () => {
    assert.strictEqual(getThresholdColor(90), Colors.howl);
    assert.strictEqual(getThresholdColor(100), Colors.howl);
  });

  it('should return wag for good values (>= 61.8%)', () => {
    assert.strictEqual(getThresholdColor(70), Colors.wag);
    assert.strictEqual(getThresholdColor(62), Colors.wag);
  });

  it('should return growl for moderate values (>= 38.2%)', () => {
    assert.strictEqual(getThresholdColor(50), Colors.growl);
    assert.strictEqual(getThresholdColor(40), Colors.growl);
  });

  it('should return bark for low values (< 38.2%)', () => {
    assert.strictEqual(getThresholdColor(30), Colors.bark);
    assert.strictEqual(getThresholdColor(0), Colors.bark);
  });
});

// =============================================================================
// PROGRESS INDICATOR TESTS
// =============================================================================

describe('BAR_CHARS', () => {
  it('should have bar characters', () => {
    assert.strictEqual(BAR_CHARS.filled, '█');
    assert.strictEqual(BAR_CHARS.empty, '░');
    assert.strictEqual(BAR_CHARS.half, '▓');
    assert.strictEqual(BAR_CHARS.light, '▒');
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(BAR_CHARS));
  });
});

describe('progressBar', () => {
  it('should create progress bar at 0%', () => {
    process.env.NO_COLOR = '1';
    const bar = progressBar(0, 100, { width: 10 });
    assert.ok(bar.includes('░░░░░░░░░░'));
    delete process.env.NO_COLOR;
  });

  it('should create progress bar at 50%', () => {
    process.env.NO_COLOR = '1';
    const bar = progressBar(50, 100, { width: 10 });
    assert.ok(bar.includes('█████░░░░░'));
    delete process.env.NO_COLOR;
  });

  it('should create progress bar at 100%', () => {
    process.env.NO_COLOR = '1';
    const bar = progressBar(100, 100, { width: 10 });
    assert.ok(bar.includes('██████████'));
    delete process.env.NO_COLOR;
  });

  it('should show percentage when requested', () => {
    process.env.NO_COLOR = '1';
    const bar = progressBar(75, 100, { width: 10, showPercent: true });
    assert.ok(bar.includes('75%'));
    delete process.env.NO_COLOR;
  });

  it('should handle values over max', () => {
    process.env.NO_COLOR = '1';
    const bar = progressBar(150, 100, { width: 10 });
    assert.ok(bar.includes('██████████'));
    delete process.env.NO_COLOR;
  });
});

describe('SPINNER_FRAMES', () => {
  it('should have 10 frames', () => {
    assert.strictEqual(SPINNER_FRAMES.length, 10);
  });

  it('should use Braille characters', () => {
    assert.ok(SPINNER_FRAMES[0].includes('⠋'));
  });
});

describe('createSpinner', () => {
  it('should create spinner with message', () => {
    const spinner = createSpinner('Loading...');
    assert.ok(spinner);
    assert.ok(typeof spinner.start === 'function');
    assert.ok(typeof spinner.stop === 'function');
    assert.ok(typeof spinner.success === 'function');
    assert.ok(typeof spinner.fail === 'function');
    assert.ok(typeof spinner.update === 'function');
  });
});

// =============================================================================
// STATUS LINE TESTS
// =============================================================================

describe('statusLine', () => {
  beforeEach(() => {
    process.env.NO_COLOR = '1';
  });

  afterEach(() => {
    delete process.env.NO_COLOR;
  });

  it('should create status line with heat', () => {
    const line = statusLine({ heat: 42 });
    assert.ok(line.includes('🔥42°'));
  });

  it('should create status line with efficiency', () => {
    const line = statusLine({ efficiency: 61 });
    assert.ok(line.includes('η:61%'));
  });

  it('should create status line with dog', () => {
    const line = statusLine({ dog: '🧠' });
    assert.ok(line.includes('🧠'));
  });

  it('should create status line with state', () => {
    const line = statusLine({ state: 'FLOW' });
    assert.ok(line.includes('⚡FLOW'));
  });

  it('should create status line with patterns', () => {
    const line = statusLine({ patterns: 3 });
    assert.ok(line.includes('+3 pattern'));
  });

  it('should combine multiple parts', () => {
    const line = statusLine({ heat: 30, efficiency: 62, dog: '🧠' });
    assert.ok(line.includes('🔥30°'));
    assert.ok(line.includes('η:62%'));
    assert.ok(line.includes('🧠'));
    assert.ok(line.includes('│'));
  });
});

describe('DOG_EMOJI', () => {
  it('should have all dogs', () => {
    assert.strictEqual(DOG_EMOJI.cynic, '🧠');
    assert.strictEqual(DOG_EMOJI.guardian, '🛡️');
    assert.strictEqual(DOG_EMOJI.oracle, '🔮');
    assert.strictEqual(DOG_EMOJI.architect, '🏗️');
    assert.strictEqual(DOG_EMOJI.scout, '🔍');
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(DOG_EMOJI));
  });
});

// =============================================================================
// FORMATTING TESTS
// =============================================================================

describe('truncate', () => {
  it('should truncate long strings', () => {
    assert.strictEqual(truncate('Hello World', 8), 'Hello...');
  });

  it('should not truncate short strings', () => {
    assert.strictEqual(truncate('Hello', 10), 'Hello');
  });

  it('should handle null/undefined', () => {
    assert.strictEqual(truncate(null, 10), '');
    assert.strictEqual(truncate(undefined, 10), '');
  });

  it('should handle exact length', () => {
    assert.strictEqual(truncate('Hello', 5), 'Hello');
  });
});

describe('pad', () => {
  it('should pad left (right-align)', () => {
    assert.strictEqual(pad('Hi', 5, 'right'), '   Hi');
  });

  it('should pad right (left-align)', () => {
    assert.strictEqual(pad('Hi', 5, 'left'), 'Hi   ');
  });

  it('should pad center', () => {
    assert.strictEqual(pad('Hi', 6, 'center'), '  Hi  ');
  });

  it('should truncate if too long', () => {
    assert.strictEqual(pad('Hello World', 5), 'Hello');
  });
});

describe('formatDuration', () => {
  it('should format milliseconds', () => {
    assert.strictEqual(formatDuration(500), '500ms');
  });

  it('should format seconds', () => {
    assert.strictEqual(formatDuration(2500), '2.5s');
  });

  it('should format minutes', () => {
    assert.strictEqual(formatDuration(125000), '2m 5s');
  });

  it('should format hours', () => {
    assert.strictEqual(formatDuration(3725000), '1h 2m');
  });
});

describe('formatBytes', () => {
  it('should format bytes', () => {
    assert.strictEqual(formatBytes(500), '500B');
  });

  it('should format kilobytes', () => {
    assert.strictEqual(formatBytes(2048), '2.0KB');
  });

  it('should format megabytes', () => {
    assert.strictEqual(formatBytes(1048576), '1.0MB');
  });

  it('should format gigabytes', () => {
    assert.strictEqual(formatBytes(1073741824), '1.0GB');
  });
});

describe('formatTime', () => {
  it('should format timestamp', () => {
    const time = formatTime(new Date('2025-01-15T14:30:45'));
    assert.ok(/\d{2}:\d{2}:\d{2}/.test(time));
  });

  it('should handle Date object', () => {
    const time = formatTime(new Date());
    assert.ok(/\d{2}:\d{2}:\d{2}/.test(time));
  });
});

describe('formatDate', () => {
  it('should format date as ISO short', () => {
    const date = formatDate(new Date('2025-01-15T14:30:45'));
    assert.strictEqual(date, '2025-01-15');
  });
});

// =============================================================================
// BOX DRAWING TESTS
// =============================================================================

describe('BOX', () => {
  it('should have single line characters', () => {
    assert.strictEqual(BOX.topLeft, '┌');
    assert.strictEqual(BOX.topRight, '┐');
    assert.strictEqual(BOX.bottomLeft, '└');
    assert.strictEqual(BOX.bottomRight, '┘');
    assert.strictEqual(BOX.horizontal, '─');
    assert.strictEqual(BOX.vertical, '│');
  });

  it('should have double line characters', () => {
    assert.strictEqual(BOX.dTopLeft, '╔');
    assert.strictEqual(BOX.dTopRight, '╗');
    assert.strictEqual(BOX.dHorizontal, '═');
    assert.strictEqual(BOX.dVertical, '║');
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(BOX));
  });
});

describe('horizontalLine', () => {
  it('should create single line', () => {
    assert.strictEqual(horizontalLine(5), '─────');
  });

  it('should create double line', () => {
    assert.strictEqual(horizontalLine(5, true), '═════');
  });
});

describe('box', () => {
  it('should create box around text', () => {
    const result = box('Hello');
    assert.ok(result.includes('┌'));
    assert.ok(result.includes('┐'));
    assert.ok(result.includes('└'));
    assert.ok(result.includes('┘'));
    assert.ok(result.includes('Hello'));
  });

  it('should create double-line box', () => {
    const result = box('Hello', { double: true });
    assert.ok(result.includes('╔'));
    assert.ok(result.includes('╗'));
    assert.ok(result.includes('╚'));
    assert.ok(result.includes('╝'));
  });

  it('should handle multiline text', () => {
    const result = box('Line 1\nLine 2');
    assert.ok(result.includes('Line 1'));
    assert.ok(result.includes('Line 2'));
  });
});
