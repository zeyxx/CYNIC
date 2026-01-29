/**
 * CYNIC Psychology Dashboard
 *
 * "Comprendre l'humain pour mieux l'aider" - κυνικός
 *
 * Generates a visual dashboard of psychological state.
 *
 * @module @cynic/scripts/psy-dashboard
 */

'use strict';

const path = require('path');

// Import centralized color system
let colors;
try {
  colors = require('./colors.cjs');
} catch {
  colors = null;
}

// Lazy load modules
let psychology = null;
let thermodynamics = null;
let biases = null;
let learning = null;

function loadModules() {
  try { psychology = require('./human-psychology.cjs'); psychology.init(); } catch {}
  try { thermodynamics = require('./cognitive-thermodynamics.cjs'); thermodynamics.init(); } catch {}
  try { biases = require('./cognitive-biases.cjs'); } catch {}
  try { learning = require('./learning-loop.cjs'); } catch {}
}

// Use centralized ANSI or fallback
const ANSI = colors?.ANSI || {
  reset: '\x1b[0m', bold: '\x1b[1m', dim: '\x1b[2m',
  red: '\x1b[31m', green: '\x1b[32m', yellow: '\x1b[33m',
  blue: '\x1b[34m', magenta: '\x1b[35m', cyan: '\x1b[36m',
  brightRed: '\x1b[91m', brightGreen: '\x1b[92m', brightYellow: '\x1b[93m',
  brightCyan: '\x1b[96m', brightWhite: '\x1b[97m',
};

// Color helper - use centralized colorize or simple fallback
let useColor = true;
const c = (color, text) => useColor ? `${color}${text}${ANSI.reset}` : text;

// Helpers
const bar = (val, max = 1) => {
  const pct = Math.min(1, val / max);
  return '█'.repeat(Math.round(pct * 10)) + '░'.repeat(10 - Math.round(pct * 10));
};

// Colored bar based on value (green=good, yellow=warning, red=critical)
const colorBar = (val, max = 1, inverse = false) => {
  // ANSI is available at module level from colors.cjs
  const pct = Math.min(1, val / max);
  const barStr = bar(val, max);

  // For inverse metrics like frustration, high=bad
  if (inverse) {
    if (pct > 0.6) return c(ANSI.brightRed, barStr);
    if (pct > 0.38) return c(ANSI.yellow, barStr);
    return c(ANSI.brightGreen, barStr);
  }

  // Normal metrics: high=good
  if (pct > 0.6) return c(ANSI.brightGreen, barStr);
  if (pct > 0.38) return c(ANSI.yellow, barStr);
  return c(ANSI.brightRed, barStr);
};

const trend = (t) => t === 'rising' ? '↑' : t === 'falling' ? '↓' : '→';

const pct = (val) => Math.round(val * 100) + '%';

/**
 * Generate the full psychology dashboard
 * @param {boolean} enableColor - Whether to use ANSI colors
 */
function generateDashboard(enableColor = true) {
  loadModules();
  useColor = enableColor;

  // ANSI is available at module level from colors.cjs
  const lines = [];

  const header = '═══════════════════════════════════════════════════════════';
  lines.push(c(ANSI.magenta, header));
  lines.push(c(ANSI.bold + ANSI.brightCyan, '🧠 HUMAN PSYCHOLOGY - "φ observes, φ learns"'));
  lines.push(c(ANSI.magenta, header));
  lines.push('');

  // === PSYCHOLOGY DIMENSIONS ===
  if (psychology) {
    const summary = psychology.getSummary();
    const state = psychology.getState();

    lines.push(c(ANSI.brightWhite, '── DIMENSIONS ─────────────────────────────────────────────'));
    lines.push(`   Energy:      [${colorBar(summary.energy.value)}] ${c(ANSI.brightCyan, pct(summary.energy.value))} ${trend(summary.energy.trend)}`);
    lines.push(`   Focus:       [${colorBar(summary.focus.value)}] ${c(ANSI.brightCyan, pct(summary.focus.value))} ${trend(summary.focus.trend)}`);
    lines.push(`   Creativity:  [${colorBar(state.dimensions.creativity?.value || 0.5)}] ${c(ANSI.brightCyan, pct(state.dimensions.creativity?.value || 0.5))} ${trend(state.dimensions.creativity?.trend)}`);
    lines.push(`   Frustration: [${colorBar(summary.frustration.value, 1, true)}] ${c(ANSI.brightCyan, pct(summary.frustration.value))} ${trend(summary.frustration.trend)}`);
    lines.push(`   Confidence:  [${colorBar(state.dimensions.confidence?.value || 0.5)}] ${c(ANSI.brightCyan, pct(state.dimensions.confidence?.value || 0.5))} ${trend(state.dimensions.confidence?.trend)}`);
    lines.push('');

    // === EMOTIONS ===
    if (state.emotions) {
      lines.push(c(ANSI.brightWhite, '── EMOTIONS ───────────────────────────────────────────────'));
      const emotions = state.emotions;
      const emotionList = [
        { name: 'Joy', value: emotions.joy?.value || 0, icon: '😊', color: ANSI.brightGreen },
        { name: 'Curiosity', value: emotions.curiosity?.value || 0, icon: '🔍', color: ANSI.brightCyan },
        { name: 'Anxiety', value: emotions.anxiety?.value || 0, icon: '😰', color: ANSI.brightYellow },
        { name: 'Boredom', value: emotions.boredom?.value || 0, icon: '😴', color: ANSI.dim },
      ];
      const active = emotionList.filter(e => e.value > 0.3).sort((a, b) => b.value - a.value);
      if (active.length > 0) {
        lines.push(`   Active: ${active.map(e => `${e.icon} ${c(e.color, e.name)} ${pct(e.value)}`).join('  ')}`);
      } else {
        lines.push(`   ${c(ANSI.dim, 'Neutral state')}`);
      }
      lines.push('');
    }

    // === COMPOSITE STATE ===
    lines.push(c(ANSI.brightWhite, '── COMPOSITE STATE ────────────────────────────────────────'));
    lines.push(`   ${summary.emoji} ${c(ANSI.bold, summary.overallState.toUpperCase())}`);

    const composites = summary.composites;
    if (composites.flow) lines.push(`   ${c(ANSI.brightGreen, '✨ Flow state - optimal productivity')}`);
    if (composites.burnoutRisk) lines.push(`   ${c(ANSI.brightRed, '⚠️ BURNOUT RISK - Take a break immediately!')}`);
    if (composites.exploration) lines.push(`   ${c(ANSI.brightCyan, '🔍 Exploration mode - high curiosity')}`);
    if (composites.grind) lines.push(`   ${c(ANSI.yellow, '⚙️ Grind mode - focused but mechanical')}`);
    if (composites.procrastination) lines.push(`   ${c(ANSI.dim, '😴 Procrastination pattern detected')}`);
    if (composites.breakthrough) lines.push(`   ${c(ANSI.brightGreen, '🎉 Breakthrough moment!')}`);
    lines.push('');

    // === SESSION ===
    lines.push(c(ANSI.brightWhite, '── SESSION ────────────────────────────────────────────────'));
    lines.push(`   Duration: ${c(ANSI.brightCyan, summary.sessionMinutes + ' min')}`);

    // Recommend break if over focus cycle
    const FOCUS_CYCLE = 62; // φ⁻¹ × 100
    if (summary.sessionMinutes > FOCUS_CYCLE) {
      lines.push(`   ${c(ANSI.brightYellow, `⚠️ Over focus cycle (${FOCUS_CYCLE} min recommended)`)}`);
    }
    lines.push('');
  } else {
    lines.push('── PSYCHOLOGY ─────────────────────────────────────────────');
    lines.push('   Module not available');
    lines.push('');
  }

  // === THERMODYNAMICS ===
  if (thermodynamics) {
    const thermo = thermodynamics.getState();
    const rec = thermodynamics.getRecommendation();

    lines.push(c(ANSI.brightWhite, '── THERMODYNAMICS ─────────────────────────────────────────'));
    const heatColor = thermo.isCritical ? ANSI.brightRed : (thermo.heat > 50 ? ANSI.yellow : ANSI.green);
    lines.push(`   Heat (Q):     ${c(heatColor, thermo.heat + ' units')} ${thermo.isCritical ? c(ANSI.brightRed, '🔥 CRITICAL') : ''}`);
    lines.push(`   Work (W):     ${c(ANSI.brightGreen, thermo.work + ' units')}`);
    lines.push(`   Temperature:  [${colorBar(thermo.temperature, 81, true)}] ${thermo.temperature}°`);

    const effColor = thermo.efficiency > 50 ? ANSI.brightGreen : (thermo.efficiency > 30 ? ANSI.yellow : ANSI.brightRed);
    lines.push(`   Efficiency:   [${colorBar(thermo.efficiency, 100)}] ${c(effColor, thermo.efficiency + '%')} (φ max: 62%)`);

    if (rec.level !== 'GOOD') {
      const msgColor = rec.level === 'CRITICAL' ? ANSI.brightRed : ANSI.yellow;
      lines.push(`   ${c(msgColor, rec.message)}`);
    }
    lines.push('');
  }

  // === COGNITIVE BIASES ===
  if (biases) {
    try {
      const detected = biases.detectBiases?.() || biases.getDetectedBiases?.() || [];
      if (detected.length > 0) {
        lines.push(c(ANSI.brightWhite, '── COGNITIVE BIASES ───────────────────────────────────────'));
        for (const bias of detected.slice(0, 3)) {
          lines.push(`   ${c(ANSI.brightYellow, '⚠️ ' + bias.name)}: ${bias.description || bias.pattern}`);
        }
        lines.push('');
      }
    } catch {}
  }

  // === LEARNING CALIBRATION ===
  if (learning) {
    try {
      const calibration = learning.getCalibration?.();
      if (calibration) {
        lines.push(c(ANSI.brightWhite, '── CALIBRATION ────────────────────────────────────────────'));
        lines.push(`   CYNIC accuracy: ${c(ANSI.brightCyan, pct(calibration.accuracy || 0.618))}`);
        lines.push(`   Samples: ${c(ANSI.dim, calibration.samples || 0)}`);
        lines.push('');
      }
    } catch {}
  }

  // === FOOTER ===
  lines.push(c(ANSI.magenta, header));

  // Choose voice based on state
  let voice = '*sniff* φ observes. φ learns. φ helps.';
  let voiceColor = ANSI.dim;
  if (psychology) {
    const summary = psychology.getSummary();
    if (summary.composites.burnoutRisk) {
      voice = '*GROWL* Stop. Rest. The pack commands it.';
      voiceColor = ANSI.brightRed;
    } else if (summary.composites.flow) {
      voice = '*silent tail wag* Flow state. The pack protects.';
      voiceColor = ANSI.brightGreen;
    } else if (summary.frustration?.value > 0.5) {
      voice = '*concerned sniff* Frustration high. Different approach?';
      voiceColor = ANSI.yellow;
    }
  }
  lines.push(c(voiceColor, voice));
  lines.push(c(ANSI.magenta, header));

  return lines.join('\n');
}

/**
 * Get quick status (for compact display)
 */
function getQuickStatus() {
  loadModules();

  if (!psychology) return { available: false };

  const summary = psychology.getSummary();
  return {
    available: true,
    state: summary.overallState,
    emoji: summary.emoji,
    energy: Math.round(summary.energy.value * 100),
    focus: Math.round(summary.focus.value * 100),
    frustration: Math.round(summary.frustration.value * 100),
    composites: summary.composites,
    sessionMinutes: summary.sessionMinutes,
  };
}

// CLI execution
if (require.main === module) {
  const enableColor = !process.argv.includes('--no-color');
  console.log(generateDashboard(enableColor));
}

module.exports = {
  generateDashboard,
  getQuickStatus,
};
