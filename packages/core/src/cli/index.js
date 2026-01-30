/**
 * CYNIC CLI Utilities
 *
 * v1.1: Shared CLI utilities for progress, colors, and status display.
 *
 * @module @cynic/core/cli
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '../axioms/constants.js';

// =============================================================================
// ANSI COLORS
// =============================================================================

/**
 * ANSI escape codes for terminal colors
 */
export const ANSI = Object.freeze({
  // Reset
  reset: '\x1b[0m',

  // Styles
  bold: '\x1b[1m',
  dim: '\x1b[2m',
  italic: '\x1b[3m',
  underline: '\x1b[4m',
  blink: '\x1b[5m',
  inverse: '\x1b[7m',
  hidden: '\x1b[8m',
  strikethrough: '\x1b[9m',

  // Foreground colors
  black: '\x1b[30m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
  cyan: '\x1b[36m',
  white: '\x1b[37m',
  gray: '\x1b[90m',

  // Bright foreground colors
  brightRed: '\x1b[91m',
  brightGreen: '\x1b[92m',
  brightYellow: '\x1b[93m',
  brightBlue: '\x1b[94m',
  brightMagenta: '\x1b[95m',
  brightCyan: '\x1b[96m',
  brightWhite: '\x1b[97m',

  // Background colors
  bgBlack: '\x1b[40m',
  bgRed: '\x1b[41m',
  bgGreen: '\x1b[42m',
  bgYellow: '\x1b[43m',
  bgBlue: '\x1b[44m',
  bgMagenta: '\x1b[45m',
  bgCyan: '\x1b[46m',
  bgWhite: '\x1b[47m',

  // Cursor control
  cursorUp: '\x1b[1A',
  cursorDown: '\x1b[1B',
  cursorForward: '\x1b[1C',
  cursorBack: '\x1b[1D',
  clearLine: '\x1b[2K',
  clearScreen: '\x1b[2J',
  saveCursor: '\x1b[s',
  restoreCursor: '\x1b[u',
});

/**
 * CYNIC semantic color palette
 */
export const Colors = Object.freeze({
  // Brand
  cynic: ANSI.cyan,
  phi: ANSI.magenta,

  // Status
  success: ANSI.green,
  warning: ANSI.yellow,
  danger: ANSI.red,
  info: ANSI.blue,
  muted: ANSI.gray,

  // Verdicts
  howl: ANSI.green,       // Excellent (81-100)
  wag: ANSI.cyan,         // Good (62-80)
  growl: ANSI.yellow,     // Warning (38-61)
  bark: ANSI.red,         // Bad (0-37)

  // Dogs (Sefirot)
  keter: ANSI.white,      // CYNIC
  chochmah: ANSI.cyan,    // Sage
  binah: ANSI.white,      // Analyst
  daat: ANSI.yellow,      // Scholar
  chesed: ANSI.blue,      // Architect
  gevurah: ANSI.red,      // Guardian
  tiferet: ANSI.yellow,   // Oracle
  netzach: ANSI.green,    // Scout
  hod: ANSI.yellow,       // Deployer
  yesod: ANSI.magenta,    // Janitor
  malkhut: ANSI.green,    // Cartographer
});

/**
 * Check if colors are supported
 * @returns {boolean}
 */
export function supportsColor() {
  if (process.env.NO_COLOR) return false;
  if (process.env.FORCE_COLOR) return true;
  return process.stdout.isTTY;
}

/**
 * Colorize text
 * @param {string} text - Text to colorize
 * @param {string} color - ANSI escape code
 * @returns {string} Colorized text
 */
export function colorize(text, color) {
  if (!supportsColor()) return text;
  return `${color}${text}${ANSI.reset}`;
}

/**
 * Get color based on value threshold (φ-aligned)
 * @param {number} value - Value 0-100
 * @returns {string} ANSI color code
 */
export function getThresholdColor(value) {
  const normalized = Math.min(100, Math.max(0, value)) / 100;
  if (normalized >= PHI_INV + PHI_INV_3) return Colors.howl;   // >= 85.4%
  if (normalized >= PHI_INV) return Colors.wag;                 // >= 61.8%
  if (normalized >= PHI_INV_2) return Colors.growl;             // >= 38.2%
  return Colors.bark;
}

// =============================================================================
// PROGRESS INDICATORS
// =============================================================================

/**
 * Progress bar characters
 */
export const BAR_CHARS = Object.freeze({
  filled: '█',
  empty: '░',
  half: '▓',
  light: '▒',
  arrow: '▸',
  dot: '●',
});

/**
 * Create a progress bar string
 * @param {number} value - Current value
 * @param {number} max - Maximum value
 * @param {Object} [options={}] - Options
 * @param {number} [options.width=10] - Bar width
 * @param {boolean} [options.showPercent=false] - Show percentage
 * @param {boolean} [options.colored=true] - Use threshold colors
 * @returns {string} Progress bar string
 */
export function progressBar(value, max = 100, options = {}) {
  const { width = 10, showPercent = false, colored = true } = options;

  const ratio = Math.min(value / max, 1);
  const percent = Math.round(ratio * 100);
  const filled = Math.round(ratio * width);
  const empty = width - filled;

  let bar = BAR_CHARS.filled.repeat(filled) + BAR_CHARS.empty.repeat(empty);

  if (colored && supportsColor()) {
    bar = colorize(bar, getThresholdColor(percent));
  }

  if (showPercent) {
    bar = `[${bar}] ${percent.toString().padStart(3)}%`;
  } else {
    bar = `[${bar}]`;
  }

  return bar;
}

/**
 * Spinner frames (Braille animation)
 */
export const SPINNER_FRAMES = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/**
 * Create a spinner
 * @param {string} [message=''] - Message to show
 * @returns {Object} Spinner controller
 */
export function createSpinner(message = '') {
  let frameIndex = 0;
  let interval = null;
  let currentMessage = message;

  return {
    /**
     * Start the spinner
     */
    start() {
      if (interval) return;
      process.stdout.write(ANSI.saveCursor);
      interval = setInterval(() => {
        const frame = SPINNER_FRAMES[frameIndex];
        frameIndex = (frameIndex + 1) % SPINNER_FRAMES.length;
        process.stdout.write(`${ANSI.restoreCursor}${ANSI.clearLine}${colorize(frame, Colors.cynic)} ${currentMessage}`);
      }, 80);
    },

    /**
     * Update spinner message
     * @param {string} msg
     */
    update(msg) {
      currentMessage = msg;
    },

    /**
     * Stop spinner with success
     * @param {string} [msg]
     */
    success(msg) {
      this.stop();
      console.log(`${colorize('✓', Colors.success)} ${msg || currentMessage}`);
    },

    /**
     * Stop spinner with failure
     * @param {string} [msg]
     */
    fail(msg) {
      this.stop();
      console.log(`${colorize('✗', Colors.danger)} ${msg || currentMessage}`);
    },

    /**
     * Stop spinner with warning
     * @param {string} [msg]
     */
    warn(msg) {
      this.stop();
      console.log(`${colorize('⚠', Colors.warning)} ${msg || currentMessage}`);
    },

    /**
     * Stop the spinner
     */
    stop() {
      if (interval) {
        clearInterval(interval);
        interval = null;
        process.stdout.write(`${ANSI.restoreCursor}${ANSI.clearLine}`);
      }
    },
  };
}

// =============================================================================
// STATUS LINE
// =============================================================================

/**
 * Create a compact status line
 * @param {Object} status - Status data
 * @param {number} [status.heat] - Thermodynamic heat
 * @param {number} [status.efficiency] - Efficiency percentage
 * @param {string} [status.dog] - Active dog emoji
 * @param {string} [status.state] - Psychology state
 * @param {number} [status.patterns] - Patterns detected
 * @returns {string} Formatted status line
 */
export function statusLine(status = {}) {
  const parts = [];

  if (status.heat !== undefined) {
    const heatColor = status.heat > 50 ? Colors.danger : Colors.muted;
    parts.push(colorize(`🔥${status.heat}°`, heatColor));
  }

  if (status.efficiency !== undefined) {
    const effColor = getThresholdColor(status.efficiency);
    parts.push(colorize(`η:${status.efficiency}%`, effColor));
  }

  if (status.dog) {
    parts.push(status.dog);
  }

  if (status.state) {
    parts.push(colorize(`⚡${status.state}`, Colors.cynic));
  }

  if (status.patterns !== undefined && status.patterns > 0) {
    parts.push(colorize(`📊 +${status.patterns} pattern`, Colors.info));
  }

  return `[${parts.join(' │ ')}]`;
}

/**
 * Dog emoji map (for status display)
 */
export const DOG_EMOJI = Object.freeze({
  cynic: '🧠',
  analyst: '📊',
  scholar: '📚',
  sage: '🦉',
  guardian: '🛡️',
  oracle: '🔮',
  architect: '🏗️',
  deployer: '🚀',
  janitor: '🧹',
  scout: '🔍',
  cartographer: '🗺️',
});

// =============================================================================
// FORMATTING UTILITIES
// =============================================================================

/**
 * Truncate string with ellipsis
 * @param {string} str - String to truncate
 * @param {number} maxLen - Maximum length
 * @returns {string}
 */
export function truncate(str, maxLen) {
  if (!str || str.length <= maxLen) return str || '';
  return str.slice(0, maxLen - 3) + '...';
}

/**
 * Pad string to fixed width
 * @param {string} str - String to pad
 * @param {number} width - Target width
 * @param {string} [align='left'] - Alignment: left, right, center
 * @returns {string}
 */
export function pad(str, width, align = 'left') {
  const len = str.length;
  if (len >= width) return str.slice(0, width);

  const padding = width - len;

  switch (align) {
    case 'right':
      return ' '.repeat(padding) + str;
    case 'center':
      const left = Math.floor(padding / 2);
      const right = padding - left;
      return ' '.repeat(left) + str + ' '.repeat(right);
    default:
      return str + ' '.repeat(padding);
  }
}

/**
 * Format duration in human-readable form
 * @param {number} ms - Milliseconds
 * @returns {string}
 */
export function formatDuration(ms) {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  if (ms < 3600000) return `${Math.floor(ms / 60000)}m ${Math.floor((ms % 60000) / 1000)}s`;
  return `${Math.floor(ms / 3600000)}h ${Math.floor((ms % 3600000) / 60000)}m`;
}

/**
 * Format bytes in human-readable form
 * @param {number} bytes - Bytes
 * @returns {string}
 */
export function formatBytes(bytes) {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)}MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(1)}GB`;
}

/**
 * Format timestamp as time string
 * @param {number|Date} ts - Timestamp
 * @returns {string}
 */
export function formatTime(ts) {
  const d = ts instanceof Date ? ts : new Date(ts);
  return d.toLocaleTimeString('en-US', { hour12: false });
}

/**
 * Format date as short string
 * @param {number|Date} ts - Timestamp
 * @returns {string}
 */
export function formatDate(ts) {
  const d = ts instanceof Date ? ts : new Date(ts);
  return d.toISOString().slice(0, 10);
}

// =============================================================================
// BOX DRAWING
// =============================================================================

/**
 * Box drawing characters
 */
export const BOX = Object.freeze({
  // Single line
  topLeft: '┌',
  topRight: '┐',
  bottomLeft: '└',
  bottomRight: '┘',
  horizontal: '─',
  vertical: '│',
  teeLeft: '├',
  teeRight: '┤',
  teeTop: '┬',
  teeBottom: '┴',
  cross: '┼',

  // Double line
  dTopLeft: '╔',
  dTopRight: '╗',
  dBottomLeft: '╚',
  dBottomRight: '╝',
  dHorizontal: '═',
  dVertical: '║',
});

/**
 * Draw a horizontal line
 * @param {number} width - Line width
 * @param {boolean} [double=false] - Use double line
 * @returns {string}
 */
export function horizontalLine(width, double = false) {
  const char = double ? BOX.dHorizontal : BOX.horizontal;
  return char.repeat(width);
}

/**
 * Draw a box around text
 * @param {string} text - Text content
 * @param {Object} [options={}] - Options
 * @param {number} [options.padding=1] - Horizontal padding
 * @param {boolean} [options.double=false] - Use double lines
 * @returns {string}
 */
export function box(text, options = {}) {
  const { padding = 1, double = false } = options;
  const lines = text.split('\n');
  const maxWidth = Math.max(...lines.map(l => l.length));
  const innerWidth = maxWidth + padding * 2;

  const tl = double ? BOX.dTopLeft : BOX.topLeft;
  const tr = double ? BOX.dTopRight : BOX.topRight;
  const bl = double ? BOX.dBottomLeft : BOX.bottomLeft;
  const br = double ? BOX.dBottomRight : BOX.bottomRight;
  const h = double ? BOX.dHorizontal : BOX.horizontal;
  const v = double ? BOX.dVertical : BOX.vertical;

  const top = tl + h.repeat(innerWidth) + tr;
  const bottom = bl + h.repeat(innerWidth) + br;

  const content = lines.map(line => {
    const paddedLine = ' '.repeat(padding) + line + ' '.repeat(maxWidth - line.length + padding);
    return v + paddedLine + v;
  });

  return [top, ...content, bottom].join('\n');
}

// =============================================================================
// EXPORTS
// =============================================================================

export default {
  // Colors
  ANSI,
  Colors,
  supportsColor,
  colorize,
  getThresholdColor,

  // Progress
  BAR_CHARS,
  progressBar,
  SPINNER_FRAMES,
  createSpinner,

  // Status
  statusLine,
  DOG_EMOJI,

  // Formatting
  truncate,
  pad,
  formatDuration,
  formatBytes,
  formatTime,
  formatDate,

  // Box drawing
  BOX,
  horizontalLine,
  box,
};
