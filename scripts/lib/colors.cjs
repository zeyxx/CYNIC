/**
 * CYNIC Color System
 *
 * "Les couleurs révèlent la vérité" - κυνικός
 *
 * Centralized ANSI color definitions for consistent UX across all dashboards.
 *
 * @module @cynic/scripts/colors
 */

'use strict';

// =============================================================================
// ANSI CODES
// =============================================================================

const ANSI = {
  // Reset
  reset: '\x1b[0m',

  // Styles
  bold: '\x1b[1m',
  dim: '\x1b[2m',
  italic: '\x1b[3m',
  underline: '\x1b[4m',

  // Standard colors
  black: '\x1b[30m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
  cyan: '\x1b[36m',
  white: '\x1b[37m',

  // Bright colors
  brightBlack: '\x1b[90m',
  brightRed: '\x1b[91m',
  brightGreen: '\x1b[92m',
  brightYellow: '\x1b[93m',
  brightBlue: '\x1b[94m',
  brightMagenta: '\x1b[95m',
  brightCyan: '\x1b[96m',
  brightWhite: '\x1b[97m',

  // Backgrounds
  bgBlack: '\x1b[40m',
  bgRed: '\x1b[41m',
  bgGreen: '\x1b[42m',
  bgYellow: '\x1b[43m',
  bgBlue: '\x1b[44m',
  bgMagenta: '\x1b[45m',
  bgCyan: '\x1b[46m',
  bgWhite: '\x1b[47m',
};

// =============================================================================
// SEMANTIC COLORS (φ-aligned thresholds)
// =============================================================================

const SEMANTIC = {
  // Status colors
  success: ANSI.brightGreen,
  warning: ANSI.yellow,
  error: ANSI.brightRed,
  info: ANSI.brightCyan,
  muted: ANSI.dim,

  // Health indicators (φ thresholds: 0.618, 0.382)
  healthy: ANSI.brightGreen,    // > 61.8%
  caution: ANSI.yellow,         // 38.2% - 61.8%
  critical: ANSI.brightRed,     // < 38.2%

  // Elements
  header: ANSI.bold + ANSI.brightCyan,
  subheader: ANSI.brightWhite,
  label: ANSI.white,
  value: ANSI.brightCyan,
  accent: ANSI.magenta,
  border: ANSI.cyan,

  // CYNIC-specific
  cynic: ANSI.brightWhite,
  dog: ANSI.brightYellow,
  phi: ANSI.brightMagenta,
};

// =============================================================================
// DASHBOARD THEMES
// =============================================================================

const THEMES = {
  // /psy - Human psychology
  psy: {
    border: ANSI.magenta,
    header: ANSI.bold + ANSI.brightCyan,
    section: ANSI.brightWhite,
    label: ANSI.white,
    value: ANSI.brightCyan,
  },

  // /health - System health
  health: {
    border: ANSI.cyan,
    header: ANSI.bold + ANSI.brightCyan,
    section: ANSI.brightWhite,
    label: ANSI.white,
    value: ANSI.brightCyan,
  },

  // /dogs - Collective Dogs
  dogs: {
    border: ANSI.cyan,
    header: ANSI.bold + ANSI.brightYellow,
    section: ANSI.white,
    label: ANSI.white,
    value: ANSI.brightGreen,
  },

  // /status - Self-status
  status: {
    border: ANSI.cyan,
    header: ANSI.bold + ANSI.brightCyan,
    section: ANSI.brightWhite,
    label: ANSI.white,
    value: ANSI.brightCyan,
  },

  // /cockpit - Ecosystem cockpit
  cockpit: {
    border: ANSI.magenta,
    header: ANSI.bold + ANSI.brightCyan,
    section: ANSI.brightWhite,
    label: ANSI.white,
    value: ANSI.brightCyan,
  },

  // awaken - Session banner
  awaken: {
    border: ANSI.cyan,
    header: ANSI.bold + ANSI.brightCyan,
    section: ANSI.brightWhite,
    cynic: ANSI.brightWhite,
  },
};

// =============================================================================
// DOG COLORS (Sefirot)
// =============================================================================

const DOG_COLORS = {
  CYNIC: ANSI.brightWhite,       // Keter - Crown
  SCOUT: ANSI.brightGreen,       // Netzach - Victory
  GUARDIAN: ANSI.brightRed,      // Gevurah - Strength
  DEPLOYER: ANSI.yellow,         // Hod - Splendor
  ARCHITECT: ANSI.brightBlue,    // Chesed - Kindness
  JANITOR: ANSI.magenta,         // Yesod - Foundation
  ORACLE: ANSI.brightYellow,     // Tiferet - Beauty
  ANALYST: ANSI.brightWhite,     // Binah - Understanding
  SAGE: ANSI.cyan,               // Chochmah - Wisdom
  SCHOLAR: ANSI.yellow,          // Daat - Knowledge
  CARTOGRAPHER: ANSI.green,      // Malkhut - Kingdom
};

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/**
 * Global color enable flag
 */
let _useColor = true;

/**
 * Set whether colors are enabled
 * @param {boolean} enabled - Whether to use colors
 */
function setColorEnabled(enabled) {
  _useColor = enabled;
}

/**
 * Check if colors are enabled
 * @returns {boolean}
 */
function isColorEnabled() {
  return _useColor;
}

/**
 * Colorize text with ANSI code
 * @param {string} color - ANSI color code
 * @param {string} text - Text to colorize
 * @returns {string} Colorized text
 */
function colorize(color, text) {
  return _useColor ? `${color}${text}${ANSI.reset}` : text;
}

/**
 * Short alias for colorize
 */
const c = colorize;

/**
 * Create a progress bar with color based on value
 * @param {number} value - Current value
 * @param {number} max - Maximum value
 * @param {boolean} inverse - Whether high values are bad
 * @returns {string} Colored progress bar
 */
function progressBar(value, max = 1, inverse = false) {
  const pct = Math.min(1, value / max);
  const filled = Math.round(pct * 10);
  const empty = 10 - filled;
  const barStr = '█'.repeat(filled) + '░'.repeat(empty);

  let color;
  if (inverse) {
    // High = bad (e.g., frustration, heat)
    color = pct > 0.618 ? ANSI.brightRed : (pct > 0.382 ? ANSI.yellow : ANSI.brightGreen);
  } else {
    // High = good (e.g., energy, health)
    color = pct > 0.618 ? ANSI.brightGreen : (pct > 0.382 ? ANSI.yellow : ANSI.brightRed);
  }

  return colorize(color, barStr);
}

/**
 * Get color for a percentage value
 * @param {number} pct - Percentage (0-100)
 * @param {boolean} inverse - Whether high values are bad
 * @returns {string} ANSI color code
 */
function getHealthColor(pct, inverse = false) {
  const normalized = pct / 100;
  if (inverse) {
    return normalized > 0.618 ? ANSI.brightRed : (normalized > 0.382 ? ANSI.yellow : ANSI.brightGreen);
  }
  return normalized > 0.618 ? ANSI.brightGreen : (normalized > 0.382 ? ANSI.yellow : ANSI.brightRed);
}

/**
 * Get trend arrow
 * @param {string} trend - 'rising', 'falling', or 'stable'
 * @returns {string} Arrow symbol
 */
function trendArrow(trend) {
  return trend === 'rising' ? '↑' : trend === 'falling' ? '↓' : '→';
}

/**
 * Format percentage
 * @param {number} value - Value 0-1
 * @returns {string} Formatted percentage
 */
function formatPct(value) {
  return Math.round(value * 100) + '%';
}

/**
 * Check if terminal supports colors
 * @returns {boolean}
 */
function supportsColor() {
  // Check NO_COLOR env
  if (process.env.NO_COLOR) return false;

  // Check FORCE_COLOR env
  if (process.env.FORCE_COLOR) return true;

  // Check if stdout is TTY
  return process.stdout.isTTY !== false;
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Raw ANSI codes
  ANSI,

  // Semantic colors
  SEMANTIC,

  // Dashboard themes
  THEMES,

  // Dog colors
  DOG_COLORS,

  // Helpers
  setColorEnabled,
  isColorEnabled,
  colorize,
  c,
  progressBar,
  getHealthColor,
  trendArrow,
  formatPct,
  supportsColor,
};

// =============================================================================
// CLI - Color Preview
// =============================================================================

if (require.main === module) {
  const header = '═══════════════════════════════════════════════════════════';

  console.log(c(SEMANTIC.header, header));
  console.log(c(SEMANTIC.header, '🎨 CYNIC COLOR SYSTEM - "Les couleurs révèlent la vérité"'));
  console.log(c(SEMANTIC.header, header));
  console.log('');

  // Semantic colors
  console.log(c(ANSI.brightWhite, '── SEMANTIC COLORS ────────────────────────────────────────'));
  console.log(`   ${c(SEMANTIC.success, '● Success')}  ${c(SEMANTIC.warning, '● Warning')}  ${c(SEMANTIC.error, '● Error')}  ${c(SEMANTIC.info, '● Info')}  ${c(SEMANTIC.muted, '● Muted')}`);
  console.log('');

  // Health indicators
  console.log(c(ANSI.brightWhite, '── HEALTH INDICATORS (φ thresholds) ───────────────────────'));
  console.log(`   ${c(SEMANTIC.healthy, '● Healthy')} (>61.8%)  ${c(SEMANTIC.caution, '● Caution')} (38-62%)  ${c(SEMANTIC.critical, '● Critical')} (<38%)`);
  console.log('');

  // Progress bars
  console.log(c(ANSI.brightWhite, '── PROGRESS BARS ──────────────────────────────────────────'));
  console.log(`   10%: [${progressBar(0.1)}]  Critical`);
  console.log(`   45%: [${progressBar(0.45)}]  Caution`);
  console.log(`   75%: [${progressBar(0.75)}]  Healthy`);
  console.log(`  100%: [${progressBar(1.0)}]  Max`);
  console.log('');

  // Inverse bars (for heat, frustration)
  console.log(c(ANSI.brightWhite, '── INVERSE BARS (heat, frustration) ───────────────────────'));
  console.log(`   10%: [${progressBar(0.1, 1, true)}]  Good (low)`);
  console.log(`   45%: [${progressBar(0.45, 1, true)}]  Caution`);
  console.log(`   75%: [${progressBar(0.75, 1, true)}]  Critical (high)`);
  console.log('');

  // Dog colors (Sefirot)
  console.log(c(ANSI.brightWhite, '── DOG COLORS (Sefirot) ───────────────────────────────────'));
  console.log(`   ${c(DOG_COLORS.CYNIC, '🧠 CYNIC')}     ${c(DOG_COLORS.SCOUT, '🔍 Scout')}      ${c(DOG_COLORS.GUARDIAN, '🛡️ Guardian')}`);
  console.log(`   ${c(DOG_COLORS.ARCHITECT, '🏗️ Architect')}  ${c(DOG_COLORS.ORACLE, '🔮 Oracle')}     ${c(DOG_COLORS.DEPLOYER, '🚀 Deployer')}`);
  console.log(`   ${c(DOG_COLORS.JANITOR, '🧹 Janitor')}    ${c(DOG_COLORS.ANALYST, '📊 Analyst')}    ${c(DOG_COLORS.SAGE, '🦉 Sage')}`);
  console.log(`   ${c(DOG_COLORS.SCHOLAR, '📚 Scholar')}    ${c(DOG_COLORS.CARTOGRAPHER, '🗺️ Cartographer')}`);
  console.log('');

  // Dashboard themes
  console.log(c(ANSI.brightWhite, '── DASHBOARD THEMES ───────────────────────────────────────'));
  console.log(`   ${c(THEMES.psy.border, '● /psy')}  ${c(THEMES.health.border, '● /health')}  ${c(THEMES.dogs.border, '● /dogs')}  ${c(THEMES.cockpit.border, '● /cockpit')}  ${c(THEMES.status.border, '● /status')}`);
  console.log('');

  // Footer
  console.log(c(SEMANTIC.header, header));
  console.log(c(ANSI.dim, '*tail wag* φ = 1.618, thresholds: 61.8% (healthy) / 38.2% (critical)'));
  console.log(c(SEMANTIC.header, header));
}
