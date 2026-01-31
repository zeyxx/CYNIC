#!/usr/bin/env node
/**
 * Dashboard Builder - Composable CLI Dashboard Architecture
 *
 * "Build dashboards like a dog builds trust - one section at a time"
 *
 * Provides a fluent, composable API for building CLI dashboards.
 * All dashboards share the same visual language and φ-aligned thresholds.
 *
 * @module @cynic/scripts/dashboard-builder
 */

'use strict';

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

const PHI_INV = 0.618;
const PHI_INV_2 = 0.382;
const PHI_INV_3 = 0.236;

// Default width for headers/separators
const DEFAULT_WIDTH = 60;

// ═══════════════════════════════════════════════════════════════════════════
// ANSI COLORS
// ═══════════════════════════════════════════════════════════════════════════

const ANSI = Object.freeze({
  reset: '\x1b[0m',
  bold: '\x1b[1m',
  dim: '\x1b[2m',
  // Standard colors
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
  cyan: '\x1b[36m',
  white: '\x1b[37m',
  // Bright colors
  brightRed: '\x1b[91m',
  brightGreen: '\x1b[92m',
  brightYellow: '\x1b[93m',
  brightBlue: '\x1b[94m',
  brightMagenta: '\x1b[95m',
  brightCyan: '\x1b[96m',
  brightWhite: '\x1b[97m',
});

// ═══════════════════════════════════════════════════════════════════════════
// RENDERER - Low-level rendering utilities
// ═══════════════════════════════════════════════════════════════════════════

class Renderer {
  constructor(options = {}) {
    this.useColor = options.useColor !== false;
    this.width = options.width || DEFAULT_WIDTH;
  }

  /**
   * Apply color to text
   */
  c(color, text) {
    return this.useColor ? `${color}${text}${ANSI.reset}` : text;
  }

  /**
   * Create a progress bar
   * @param {number} value - Current value (0-max)
   * @param {number} max - Maximum value
   * @param {Object} options - { inverse: false, width: 10 }
   */
  progressBar(value, max = 1, options = {}) {
    const { inverse = false, width = 10 } = options;
    const pct = Math.min(1, Math.max(0, value / max));
    const filled = Math.round(pct * width);
    const barStr = '█'.repeat(filled) + '░'.repeat(width - filled);

    let color;
    if (inverse) {
      // High = bad (for heat, load, etc.)
      color = pct > PHI_INV ? ANSI.brightRed : pct > PHI_INV_2 ? ANSI.yellow : ANSI.brightGreen;
    } else {
      // High = good (for health, score, etc.)
      color = pct > PHI_INV ? ANSI.brightGreen : pct > PHI_INV_2 ? ANSI.yellow : ANSI.brightRed;
    }

    return this.c(color, barStr);
  }

  /**
   * Get color based on φ thresholds
   */
  thresholdColor(value, max = 100, inverse = false) {
    const pct = value / max;
    if (inverse) {
      return pct > PHI_INV ? ANSI.brightRed : pct > PHI_INV_2 ? ANSI.yellow : ANSI.brightGreen;
    }
    return pct > PHI_INV ? ANSI.brightGreen : pct > PHI_INV_2 ? ANSI.yellow : ANSI.brightRed;
  }

  /**
   * Create header line (═══)
   */
  headerLine(char = '═') {
    return this.c(ANSI.cyan, char.repeat(this.width));
  }

  /**
   * Create section separator (───)
   */
  sectionLine(title = '', char = '─') {
    if (!title) {
      return char.repeat(this.width);
    }
    const padding = char.repeat(Math.max(0, this.width - title.length - 4));
    return this.c(ANSI.brightWhite, `── ${title} ${padding}`);
  }

  /**
   * Create status icon
   */
  icon(status) {
    switch (status) {
      case 'success':
      case 'healthy':
      case true:
        return this.c(ANSI.brightGreen, '✅');
      case 'warning':
        return this.c(ANSI.yellow, '⚠️');
      case 'error':
      case 'critical':
      case false:
        return this.c(ANSI.brightRed, '❌');
      case 'info':
        return this.c(ANSI.brightCyan, 'ℹ️');
      case 'fire':
        return this.c(ANSI.brightRed, '🔥');
      case 'dog':
        return '🐕';
      default:
        return this.c(ANSI.dim, '○');
    }
  }

  /**
   * Format a key-value pair with alignment
   */
  kv(key, value, options = {}) {
    const { keyWidth = 12, indent = 3, keyColor = ANSI.dim, valueColor = ANSI.brightCyan } = options;
    const paddedKey = key.padEnd(keyWidth);
    return ' '.repeat(indent) + this.c(keyColor, paddedKey) + this.c(valueColor, String(value));
  }

  /**
   * Indent text
   */
  indent(text, spaces = 3) {
    return ' '.repeat(spaces) + text;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION - Individual dashboard section
// ═══════════════════════════════════════════════════════════════════════════

class Section {
  constructor(title, renderer) {
    this.title = title;
    this.renderer = renderer;
    this.lines = [];
    this._visible = true;
  }

  /**
   * Add a line to the section
   */
  line(text) {
    this.lines.push(text);
    return this;
  }

  /**
   * Add an empty line
   */
  blank() {
    this.lines.push('');
    return this;
  }

  /**
   * Add a key-value pair
   */
  kv(key, value, options = {}) {
    this.lines.push(this.renderer.kv(key, value, options));
    return this;
  }

  /**
   * Add a status item with icon
   */
  status(label, isHealthy, extra = '') {
    const icon = this.renderer.icon(isHealthy);
    const extraStr = extra ? this.renderer.c(ANSI.dim, ` ${extra}`) : '';
    this.lines.push(this.renderer.indent(`${icon} ${label}${extraStr}`));
    return this;
  }

  /**
   * Add a metric with progress bar
   */
  metric(label, value, max, options = {}) {
    const { unit = '', inverse = false, showPct = true } = options;
    const bar = this.renderer.progressBar(value, max, { inverse });
    const pct = Math.round((value / max) * 100);
    const color = this.renderer.thresholdColor(value, max, inverse);
    const valueStr = showPct ? `${pct}%` : `${value}${unit}`;
    this.lines.push(this.renderer.indent(`${label.padEnd(12)} [${bar}] ${this.renderer.c(color, valueStr)}`));
    return this;
  }

  /**
   * Set visibility condition
   */
  when(condition) {
    this._visible = !!condition;
    return this;
  }

  /**
   * Render section to lines array
   */
  render() {
    if (!this._visible || this.lines.length === 0) {
      return [];
    }

    const output = [];
    if (this.title) {
      output.push(this.renderer.sectionLine(this.title));
    }
    output.push(...this.lines);
    output.push(''); // Blank line after section
    return output;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// DASHBOARD BUILDER - Fluent API for building dashboards
// ═══════════════════════════════════════════════════════════════════════════

class DashboardBuilder {
  constructor(options = {}) {
    this.options = {
      useColor: options.useColor !== false,
      width: options.width || DEFAULT_WIDTH,
      title: options.title || 'CYNIC DASHBOARD',
      subtitle: options.subtitle || '',
      icon: options.icon || '🐕',
      ...options,
    };

    this.renderer = new Renderer(this.options);
    this.sections = [];
    this._footerVoice = null;
    this._footerVoiceColor = ANSI.dim;
    this._footerNote = 'φ⁻¹ confidence: 61.8% max';
  }

  /**
   * Add a section to the dashboard
   */
  section(title) {
    const section = new Section(title, this.renderer);
    this.sections.push(section);
    return section;
  }

  /**
   * Add a section using a builder function
   */
  addSection(title, builderFn) {
    const section = this.section(title);
    builderFn(section, this.renderer);
    return this;
  }

  /**
   * Set footer voice (dog personality)
   */
  voice(text, color = ANSI.dim) {
    this._footerVoice = text;
    this._footerVoiceColor = color;
    return this;
  }

  /**
   * Set footer note
   */
  note(text) {
    this._footerNote = text;
    return this;
  }

  /**
   * Build the dashboard
   */
  build() {
    const lines = [];
    const r = this.renderer;

    // Header
    lines.push(r.headerLine());
    const titleLine = `${this.options.icon} ${this.options.title}`;
    const fullTitle = this.options.subtitle
      ? `${titleLine} - "${this.options.subtitle}"`
      : titleLine;
    lines.push(r.c(ANSI.bold + ANSI.brightCyan, fullTitle));
    lines.push(r.headerLine());
    lines.push('');

    // Sections
    for (const section of this.sections) {
      const sectionLines = section.render();
      lines.push(...sectionLines);
    }

    // Footer
    lines.push(r.headerLine());
    if (this._footerVoice) {
      lines.push(r.c(this._footerVoiceColor, this._footerVoice));
    }
    if (this._footerNote) {
      lines.push(r.c(ANSI.dim, this._footerNote));
    }
    lines.push(r.headerLine());

    return lines.join('\n');
  }

  /**
   * Build and print to console
   */
  print() {
    console.log(this.build());
    return this;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// FACTORY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create a new dashboard builder
 */
function createDashboard(options = {}) {
  return new DashboardBuilder(options);
}

/**
 * Create a renderer for custom use
 */
function createRenderer(options = {}) {
  return new Renderer(options);
}

// ═══════════════════════════════════════════════════════════════════════════
// PRESET SECTION BUILDERS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Hooks section builder
 */
function buildHooksSection(section, renderer, hooks = []) {
  let healthy = 0;
  let totalEngines = 0;

  for (const hook of hooks) {
    if (hook.exists) {
      healthy++;
      totalEngines += hook.engines || 0;
      const extra = hook.engines > 0 ? `(${hook.engines} engines)` : '';
      section.status(hook.name, true, extra);
    } else {
      section.status(hook.name, false, 'missing');
    }
  }

  const allHealthy = healthy === hooks.length;
  const statusColor = allHealthy ? ANSI.brightGreen : ANSI.brightYellow;
  section.line(renderer.indent(`${renderer.c(ANSI.dim, 'Status:')} ${renderer.c(statusColor, `${healthy}/${hooks.length} healthy`)}`));

  return { healthy, total: hooks.length, engines: totalEngines, allHealthy };
}

/**
 * Thermodynamics section builder
 */
function buildThermoSection(section, renderer, thermo) {
  if (!thermo) return section.when(false);

  const heatColor = thermo.isCritical ? ANSI.brightRed : thermo.heat > 50 ? ANSI.yellow : ANSI.green;
  const heatWarning = thermo.isCritical ? renderer.c(ANSI.brightRed, ' 🔥 CRITICAL') : '';

  section
    .kv('Heat (Q):', `${thermo.heat} units${heatWarning}`, { valueColor: heatColor })
    .kv('Work (W):', `${thermo.work} units`, { valueColor: ANSI.brightGreen })
    .metric('Temperature', thermo.temperature, 81, { inverse: true, showPct: false, unit: '°' })
    .metric('Efficiency', thermo.efficiency, 100, { inverse: false });

  return section;
}

/**
 * Dogs session section builder
 */
function buildDogsSection(section, renderer, summary, dogColors = {}) {
  if (!summary || summary.totalActions === 0) {
    return section.when(false);
  }

  section.line(renderer.indent(
    `Session: ${renderer.c(ANSI.brightCyan, summary.duration + ' min')} │ ` +
    `Actions: ${renderer.c(ANSI.brightGreen, String(summary.totalActions))}`
  ));

  if (summary.topDog) {
    const color = dogColors[summary.topDog.name] || ANSI.white;
    const icon = summary.topDog.dog?.icon || '🐕';
    section.line(renderer.indent(`Top Dog: ${renderer.c(color, icon + ' ' + summary.topDog.name)}`));
  }

  return section;
}

// ═══════════════════════════════════════════════════════════════════════════
// EXPORTS
// ═══════════════════════════════════════════════════════════════════════════

module.exports = {
  // Classes
  DashboardBuilder,
  Section,
  Renderer,
  // Factory functions
  createDashboard,
  createRenderer,
  // Preset builders
  buildHooksSection,
  buildThermoSection,
  buildDogsSection,
  // Constants
  ANSI,
  PHI_INV,
  PHI_INV_2,
  PHI_INV_3,
  DEFAULT_WIDTH,
};
