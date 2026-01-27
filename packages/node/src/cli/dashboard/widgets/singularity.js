/**
 * CYNIC TUI Dashboard - Singularity Index Widget
 *
 * Displays the Singularity Index gauge and dimensions
 *
 * @module @cynic/node/cli/dashboard/widgets/singularity
 */

'use strict';

import blessed from 'blessed';
import { COLORS, progressBar, getThresholdColor, MAX_CONFIDENCE } from '../theme.js';

/**
 * Create Singularity Widget
 */
export function createSingularityWidget(parent, options = {}) {
  const box = blessed.box({
    parent,
    label: ' SINGULARITY INDEX ',
    top: options.top || 0,
    left: options.left || '66%',
    width: options.width || '34%',
    height: options.height || '50%',
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.phi },
      label: { fg: COLORS.phi, bold: true },
    },
    tags: true,
  });

  const content = blessed.box({
    parent: box,
    top: 0,
    left: 1,
    right: 1,
    bottom: 0,
    tags: true,
  });

  /**
   * Calculate singularity score from metrics
   */
  function calculateSingularityScore(data) {
    // This is a simplified calculation
    // Real implementation would use brain_meta tool
    const metrics = data?.metrics?.metrics || {};
    const health = data?.health || {};
    const collective = data?.collective || {};

    // Component scores (0-100)
    const scores = {
      codebase: 0,
      collective: 0,
      wisdom: 0,
      autonomy: 0,
    };

    // Codebase: based on patterns detected
    const patterns = data?.patterns?.patterns || [];
    scores.codebase = Math.min(patterns.length * 10, 100);

    // Collective: based on active dogs
    const dogs = collective.dogs || {};
    const activeDogs = Object.values(dogs).filter(d => d.active).length;
    scores.collective = Math.round((activeDogs / 11) * 100);

    // Wisdom: based on total judgments
    const judgments = health.judge?.totalJudgments || 0;
    scores.wisdom = Math.min(Math.round(Math.log10(judgments + 1) * 20), 100);

    // Autonomy: based on chain initialization
    const chain = data?.chain || {};
    scores.autonomy = chain.initialized ? 70 : 30;

    // Overall score (weighted average)
    const overall = Math.round(
      scores.codebase * 0.25 +
      scores.collective * 0.25 +
      scores.wisdom * 0.25 +
      scores.autonomy * 0.25
    );

    return { overall, ...scores };
  }

  /**
   * Update widget with new data
   */
  function update(data) {
    const scores = calculateSingularityScore(data);
    const color = getThresholdColor(scores.overall);

    const lines = [
      `{bold}Score:{/} {${color}-fg}${progressBar(scores.overall, 100, 10)}{/} ${scores.overall}/100`,
      `{bold}Trend:{/} ▲ Rising`,
      `{bold}φ:{/} {magenta-fg}${(MAX_CONFIDENCE * 100).toFixed(1)}%{/}`,
      '',
      '{bold}Dimensions:{/}',
      `  Codebase:   ${progressBar(scores.codebase, 100, 6)} ${scores.codebase}%`,
      `  Collective: ${progressBar(scores.collective, 100, 6)} ${scores.collective}%`,
      `  Wisdom:     ${progressBar(scores.wisdom, 100, 6)} ${scores.wisdom}%`,
      `  Autonomy:   ${progressBar(scores.autonomy, 100, 6)} ${scores.autonomy}%`,
    ];

    content.setContent(lines.join('\n'));
  }

  return { box, update };
}

export default createSingularityWidget;
