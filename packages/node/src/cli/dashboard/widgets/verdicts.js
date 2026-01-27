/**
 * CYNIC TUI Dashboard - Verdicts Widget
 *
 * Displays verdict distribution chart
 *
 * @module @cynic/node/cli/dashboard/widgets/verdicts
 */

'use strict';

import blessed from 'blessed';
import { COLORS, VERDICT_COLORS, progressBar } from '../theme.js';

/**
 * Create Verdicts Widget
 */
export function createVerdictsWidget(parent, options = {}) {
  const box = blessed.box({
    parent,
    label: ' VERDICT DISTRIBUTION ',
    top: options.top || '50%',
    left: options.left || '50%',
    width: options.width || '50%',
    height: options.height || '25%',
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.primary },
      label: { fg: COLORS.primary, bold: true },
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
   * Update widget with new data
   */
  function update(data) {
    const health = data?.health || {};
    const judge = health.judge || {};
    const stats = health.judgmentStats || {};

    // Get verdict counts
    const verdictCounts = stats.verdicts || {
      HOWL: 0,
      WAG: 0,
      GROWL: 0,
      BARK: 0,
    };

    const total = Object.values(verdictCounts).reduce((a, b) => a + b, 0) || 1;

    // Calculate percentages
    const percentages = {
      HOWL: Math.round((verdictCounts.HOWL / total) * 100),
      WAG: Math.round((verdictCounts.WAG / total) * 100),
      GROWL: Math.round((verdictCounts.GROWL / total) * 100),
      BARK: Math.round((verdictCounts.BARK / total) * 100),
    };

    const lines = [
      `{${VERDICT_COLORS.HOWL}-fg}HOWL{/}  ${progressBar(percentages.HOWL, 100, 10)} ${percentages.HOWL.toString().padStart(3)}%`,
      `{${VERDICT_COLORS.WAG}-fg}WAG{/}   ${progressBar(percentages.WAG, 100, 10)} ${percentages.WAG.toString().padStart(3)}%`,
      `{${VERDICT_COLORS.GROWL}-fg}GROWL{/} ${progressBar(percentages.GROWL, 100, 10)} ${percentages.GROWL.toString().padStart(3)}%`,
      `{${VERDICT_COLORS.BARK}-fg}BARK{/}  ${progressBar(percentages.BARK, 100, 10)} ${percentages.BARK.toString().padStart(3)}%`,
    ];

    if (total > 1) {
      lines.push('');
      lines.push(`{gray-fg}Total: ${total} judgments{/}`);
    }

    content.setContent(lines.join('\n'));
  }

  return { box, update };
}

export default createVerdictsWidget;
