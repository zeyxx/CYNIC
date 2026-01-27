/**
 * CYNIC TUI Dashboard - Events Widget
 *
 * Displays scrollable event log
 *
 * @module @cynic/node/cli/dashboard/widgets/events
 */

'use strict';

import blessed from 'blessed';
import { COLORS, VERDICT_COLORS, formatTime, truncate } from '../theme.js';

// Max events to keep in memory (Fibonacci)
const MAX_EVENTS = 34;

/**
 * Create Events Widget
 */
export function createEventsWidget(parent, options = {}) {
  const box = blessed.box({
    parent,
    label: ' RECENT EVENTS ',
    top: options.top || '75%',
    left: options.left || 0,
    width: options.width || '100%',
    height: options.height || '20%',
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.muted },
      label: { fg: COLORS.muted, bold: true },
    },
    tags: true,
  });

  const content = blessed.log({
    parent: box,
    top: 0,
    left: 1,
    right: 1,
    bottom: 0,
    tags: true,
    scrollable: true,
    alwaysScroll: true,
    scrollbar: {
      ch: '│',
      style: { fg: COLORS.muted },
    },
    keys: true,
    vi: true,
    mouse: true,
  });

  // Internal event storage
  const events = [];

  /**
   * Add an event to the log
   */
  function addEvent(event) {
    events.push(event);
    if (events.length > MAX_EVENTS) {
      events.shift();
    }

    const time = formatTime(event.timestamp || Date.now());
    let line = `{gray-fg}${time}{/} │ `;

    switch (event.type) {
      case 'judgment':
        line += `Judgment #${event.id || '?'} │ ${truncate(event.item, 12)} │ Q: ${event.score || '?'} │ {${VERDICT_COLORS[event.verdict] || 'white'}-fg}${event.verdict}{/}`;
        break;

      case 'block':
        line += `{cyan-fg}Block #${event.slot}{/} created │ ${event.judgments} judgments │ Merkle: ${truncate(event.merkleRoot, 10)}`;
        break;

      case 'anchor':
        line += `{magenta-fg}Anchor{/} ${event.status} │ Block #${event.slot} → ${event.network}`;
        break;

      case 'pattern':
        line += `{yellow-fg}Pattern{/} detected │ ${event.name} │ confidence: ${Math.round(event.confidence * 100)}%`;
        break;

      case 'dog':
        line += `{green-fg}${event.dog}{/} │ ${event.action} │ ${truncate(event.detail, 30)}`;
        break;

      case 'alert':
        line += `{red-fg}ALERT{/} │ ${event.level} │ ${event.message}`;
        break;

      case 'connection':
        line += event.status === 'connected'
          ? '{green-fg}Connected{/} to MCP server'
          : '{red-fg}Disconnected{/} from MCP server';
        break;

      default:
        line += event.message || JSON.stringify(event);
    }

    content.log(line);
  }

  /**
   * Update widget with new data (synthesize events from changes)
   */
  function update(data, prevData) {
    // Connection status change
    if (data.connected !== prevData?.connected) {
      addEvent({
        type: 'connection',
        status: data.connected ? 'connected' : 'disconnected',
        timestamp: Date.now(),
      });
    }

    // Check for new patterns
    const currentPatterns = data?.patterns?.patterns || [];
    const prevPatterns = prevData?.patterns?.patterns || [];

    if (currentPatterns.length > prevPatterns.length) {
      const newPattern = currentPatterns[0];
      if (newPattern) {
        addEvent({
          type: 'pattern',
          name: newPattern.type || 'unknown',
          confidence: newPattern.confidence || 0.5,
          timestamp: newPattern.timestamp || Date.now(),
        });
      }
    }

    // Check for chain updates
    const currentSlot = data?.chain?.headSlot || 0;
    const prevSlot = prevData?.chain?.headSlot || 0;

    if (currentSlot > prevSlot && prevSlot > 0) {
      addEvent({
        type: 'block',
        slot: currentSlot,
        judgments: data?.chain?.pendingJudgments || 0,
        merkleRoot: data?.chain?.merkleRoot || '',
        timestamp: Date.now(),
      });
    }

    // Check for alerts
    const currentAlerts = data?.metrics?.alerts || [];
    const prevAlerts = prevData?.metrics?.alerts || [];

    if (currentAlerts.length > prevAlerts.length) {
      const newAlert = currentAlerts[currentAlerts.length - 1];
      if (newAlert) {
        addEvent({
          type: 'alert',
          level: newAlert.level || 'warning',
          message: newAlert.message || 'Unknown alert',
          timestamp: Date.now(),
        });
      }
    }
  }

  /**
   * Focus the events log (for scrolling)
   */
  function focus() {
    content.focus();
  }

  /**
   * Manual event injection (for external events)
   */
  function pushEvent(event) {
    addEvent(event);
  }

  return { box, update, focus, pushEvent, addEvent };
}

export default createEventsWidget;
