/**
 * CYNIC TUI Dashboard - Collective Widget
 *
 * Displays The 11 Dogs status
 *
 * @module @cynic/node/cli/dashboard/widgets/collective
 */

'use strict';

import blessed from 'blessed';
import { COLORS, DOG_ICONS } from '../theme.js';

/**
 * Dog name to display name mapping
 */
const DOG_NAMES = {
  guardian: 'Guardian',
  analyst: 'Analyst',
  scholar: 'Scholar',
  architect: 'Architect',
  sage: 'Sage',
  cynic: 'CYNIC',
  janitor: 'Janitor',
  scout: 'Scout',
  cartographer: 'Mapper',
  oracle: 'Oracle',
  deployer: 'Deployer',
};

/**
 * Create Collective Widget
 */
export function createCollectiveWidget(parent, options = {}) {
  const box = blessed.box({
    parent,
    label: ' THE COLLECTIVE ',
    top: options.top || 0,
    left: options.left || '33%',
    width: options.width || '33%',
    height: options.height || '50%',
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
    const collective = data?.collective || {};
    const dogs = collective.dogs || {};

    const lines = [];

    // Display each dog
    const dogOrder = [
      'guardian', 'analyst', 'scholar', 'architect',
      'sage', 'cynic', 'janitor', 'scout',
      'cartographer', 'oracle', 'deployer',
    ];

    for (const name of dogOrder) {
      const dog = dogs[name] || {};
      const icon = DOG_ICONS[name] || '🐕';
      const displayName = DOG_NAMES[name] || name;
      const active = dog.active;
      const statusIcon = active ? '{green-fg}●{/}' : '{gray-fg}○{/}';
      const invocations = dog.invocations || 0;

      // Format: 🛡️ Guardian [●] 12
      const line = `${icon} ${displayName.padEnd(10)} ${statusIcon} ${invocations > 0 ? invocations : ''}`;
      lines.push(line);
    }

    // Add collective stats if available
    if (collective.cynicState) {
      lines.push('');
      lines.push(`{bold}State:{/} ${collective.cynicState}`);
    }

    content.setContent(lines.join('\n'));
  }

  return { box, update };
}

export default createCollectiveWidget;
