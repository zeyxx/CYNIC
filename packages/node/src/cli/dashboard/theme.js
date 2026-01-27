/**
 * CYNIC TUI Dashboard - Theme
 *
 * Color scheme and styling constants
 * φ-aligned design principles
 *
 * @module @cynic/node/cli/dashboard/theme
 */

'use strict';

import { PHI_INV } from '@cynic/core';

/**
 * Color palette
 */
export const COLORS = {
  // Primary colors
  primary: 'cyan',
  secondary: 'white',
  accent: 'magenta',

  // Status colors
  success: 'green',
  warning: 'yellow',
  danger: 'red',
  muted: 'gray',

  // φ-related values
  phi: 'magenta',

  // Background
  bg: 'black',
  bgLight: 'gray',
};

/**
 * Verdict colors
 */
export const VERDICT_COLORS = {
  HOWL: 'green',   // Excellent (81-100)
  WAG: 'cyan',     // Good (62-80)
  GROWL: 'yellow', // Warning (38-61)
  BARK: 'red',     // Bad (0-37)
};

/**
 * Verdict icons
 */
export const VERDICT_ICONS = {
  HOWL: '🟢',
  WAG: '🔵',
  GROWL: '🟡',
  BARK: '🔴',
};

/**
 * Dog icons by name
 */
export const DOG_ICONS = {
  guardian: '🛡️',
  analyst: '🔍',
  scholar: '📚',
  architect: '🏛️',
  sage: '🧙',
  cynic: '🐕',
  janitor: '🧹',
  scout: '🔭',
  cartographer: '🗺️',
  oracle: '👁️',
  deployer: '🚀',
};

/**
 * φ-aligned layout ratios
 */
export const PHI_RATIO = {
  major: 0.618, // φ⁻¹
  minor: 0.382, // φ⁻²
};

/**
 * φ-aligned max confidence
 */
export const MAX_CONFIDENCE = PHI_INV;

/**
 * Create a progress bar string
 */
export function progressBar(value, max = 100, width = 10) {
  const ratio = Math.min(value / max, 1);
  const filled = Math.round(ratio * width);
  const empty = width - filled;
  return '█'.repeat(filled) + '░'.repeat(empty);
}

/**
 * Get color based on value threshold
 */
export function getThresholdColor(value) {
  if (value >= 81) return VERDICT_COLORS.HOWL;
  if (value >= 62) return VERDICT_COLORS.WAG;
  if (value >= 38) return VERDICT_COLORS.GROWL;
  return VERDICT_COLORS.BARK;
}

/**
 * Format timestamp as time string
 */
export function formatTime(ts) {
  const d = new Date(ts);
  return d.toLocaleTimeString('en-US', { hour12: false });
}

/**
 * Truncate string with ellipsis
 */
export function truncate(str, maxLen) {
  if (!str || str.length <= maxLen) return str || '';
  return str.slice(0, maxLen - 3) + '...';
}

export default {
  COLORS,
  VERDICT_COLORS,
  VERDICT_ICONS,
  DOG_ICONS,
  PHI_RATIO,
  MAX_CONFIDENCE,
  progressBar,
  getThresholdColor,
  formatTime,
  truncate,
};
