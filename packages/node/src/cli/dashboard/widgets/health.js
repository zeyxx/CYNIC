/**
 * CYNIC TUI Dashboard - Health Widget
 *
 * Displays system health status
 *
 * @module @cynic/node/cli/dashboard/widgets/health
 */

'use strict';

import blessed from 'blessed';
import { COLORS } from '../theme.js';

/**
 * Format uptime in human readable form
 */
function formatUptime(ms) {
  if (!ms || ms <= 0) return '0s';
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (hours > 0) return `${hours}h ${minutes % 60}m`;
  if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
  return `${seconds}s`;
}

/**
 * Format bytes
 */
function formatBytes(bytes) {
  if (!bytes || bytes < 1024) return `${bytes || 0}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
}

/**
 * Create Health Widget
 */
export function createHealthWidget(parent, options = {}) {
  const box = blessed.box({
    parent,
    label: ' SYSTEM HEALTH ',
    top: options.top || 0,
    left: options.left || 0,
    width: options.width || '33%',
    height: options.height || '50%',
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.primary },
      label: { fg: COLORS.primary, bold: true },
    },
    tags: true,
  });

  // Content box inside
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
    const node = health.node || {};
    const persistence = health.persistence || {};
    const judge = health.judge || {};

    const statusIcon = health.status === 'healthy' ? `{green-fg}✓{/}` : `{red-fg}✗{/}`;
    const nodeStatus = node.status === 'active' ? `{green-fg}✓{/}` : `{yellow-fg}?{/}`;

    // Persistence status
    const redisStatus = persistence.redis?.connected ? `{green-fg}✓{/}` : `{red-fg}✗{/}`;
    const pgStatus = persistence.postgres?.connected ? `{green-fg}✓{/}` : `{red-fg}✗{/}`;
    const dagStatus = persistence.graph?.initialized ? `{green-fg}✓{/}` : `{yellow-fg}○{/}`;

    // Memory usage (from process if available)
    const memUsed = process.memoryUsage?.()?.heapUsed || 0;

    const lines = [
      `{bold}Status:{/} ${statusIcon} ${health.status || 'unknown'}`,
      `{bold}Uptime:{/} ${formatUptime(node.uptime)}`,
      `{bold}Memory:{/} ${formatBytes(memUsed)}`,
      `{bold}Judged:{/} ${judge.totalJudgments || 0}`,
      '',
      '{bold}PERSISTENCE{/}',
      `Redis:    ${redisStatus}`,
      `Postgres: ${pgStatus}`,
      `DAG:      ${dagStatus}`,
    ];

    content.setContent(lines.join('\n'));
  }

  return { box, update };
}

export default createHealthWidget;
