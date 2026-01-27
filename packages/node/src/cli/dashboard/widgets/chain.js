/**
 * CYNIC TUI Dashboard - Chain Widget
 *
 * Displays PoJ Blockchain status
 *
 * @module @cynic/node/cli/dashboard/widgets/chain
 */

'use strict';

import blessed from 'blessed';
import { COLORS, truncate, formatTime } from '../theme.js';

/**
 * Format relative time
 */
function formatRelativeTime(ms) {
  if (!ms || ms <= 0) return 'never';
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (seconds < 60) return `${seconds}s ago`;
  if (minutes < 60) return `${minutes}m ago`;
  return `${hours}h ago`;
}

/**
 * Create Chain Widget
 */
export function createChainWidget(parent, options = {}) {
  const box = blessed.box({
    parent,
    label: ' POJ BLOCKCHAIN ',
    top: options.top || '50%',
    left: options.left || 0,
    width: options.width || '50%',
    height: options.height || '25%',
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.accent },
      label: { fg: COLORS.accent, bold: true },
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
    const chain = data?.chain || {};

    const initialized = chain.initialized;
    const statusIcon = initialized ? `{green-fg}✓{/}` : `{yellow-fg}○{/}`;

    const headSlot = chain.headSlot || 0;
    const pending = chain.pendingJudgments || 0;
    const lastAnchor = chain.lastAnchorTime
      ? formatRelativeTime(Date.now() - chain.lastAnchorTime)
      : 'never';
    const merkleRoot = chain.merkleRoot
      ? truncate(chain.merkleRoot, 12)
      : '────────';

    // Anchoring status
    const anchorStatus = chain.anchoringEnabled
      ? `{green-fg}enabled{/}`
      : `{yellow-fg}disabled{/}`;

    const lines = [
      `{bold}Head Block:{/} #${headSlot.toLocaleString()}`,
      `{bold}Pending:{/} ${pending} judgments`,
      `{bold}Last Anchor:{/} ${lastAnchor}`,
      `{bold}Merkle Root:{/} ${merkleRoot}`,
      `{bold}Anchoring:{/} ${anchorStatus}`,
    ];

    content.setContent(lines.join('\n'));
  }

  return { box, update };
}

export default createChainWidget;
