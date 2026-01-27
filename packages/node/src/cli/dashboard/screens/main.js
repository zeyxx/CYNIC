/**
 * CYNIC TUI Dashboard - Main Screen
 *
 * The main cockpit view with all widgets
 *
 * @module @cynic/node/cli/dashboard/screens/main
 */

'use strict';

import blessed from 'blessed';
import { COLORS, formatTime, MAX_CONFIDENCE } from '../theme.js';
import {
  createHealthWidget,
  createCollectiveWidget,
  createSingularityWidget,
  createChainWidget,
  createVerdictsWidget,
  createEventsWidget,
} from '../widgets/index.js';

/**
 * Create Main Screen
 */
export function createMainScreen(screen, options = {}) {
  // Main container
  const container = blessed.box({
    parent: screen,
    top: 0,
    left: 0,
    width: '100%',
    height: '100%',
    tags: true,
  });

  // Header bar
  const header = blessed.box({
    parent: container,
    top: 0,
    left: 0,
    width: '100%',
    height: 1,
    style: {
      bg: 'blue',
      fg: 'white',
    },
    tags: true,
  });

  // Widgets container (below header)
  const widgetsContainer = blessed.box({
    parent: container,
    top: 1,
    left: 0,
    width: '100%',
    height: '100%-3',
    tags: true,
  });

  // Create widgets
  const healthWidget = createHealthWidget(widgetsContainer, {
    top: 0,
    left: 0,
    width: '33%',
    height: '50%',
  });

  const collectiveWidget = createCollectiveWidget(widgetsContainer, {
    top: 0,
    left: '33%',
    width: '34%',
    height: '50%',
  });

  const singularityWidget = createSingularityWidget(widgetsContainer, {
    top: 0,
    left: '67%',
    width: '33%',
    height: '50%',
  });

  const chainWidget = createChainWidget(widgetsContainer, {
    top: '50%',
    left: 0,
    width: '50%',
    height: '30%',
  });

  const verdictsWidget = createVerdictsWidget(widgetsContainer, {
    top: '50%',
    left: '50%',
    width: '50%',
    height: '30%',
  });

  const eventsWidget = createEventsWidget(widgetsContainer, {
    top: '80%',
    left: 0,
    width: '100%',
    height: '20%',
  });

  // Footer bar (keyboard shortcuts)
  const footer = blessed.box({
    parent: container,
    bottom: 0,
    left: 0,
    width: '100%',
    height: 1,
    style: {
      bg: 'black',
      fg: 'white',
    },
    tags: true,
    content: ' {bold}[H]{/}ealth  {bold}[C]{/}hain  {bold}[P]{/}atterns  {bold}[J]{/}udgments  {bold}[A]{/}gents  {bold}[R]{/}efresh  {bold}[Q]{/}uit',
  });

  // Store previous data for diff detection
  let prevData = null;

  /**
   * Update header with current time and status
   */
  function updateHeader(data) {
    const time = formatTime(Date.now());
    const connected = data?.connected;
    const status = connected ? '{green-fg}●{/}' : '{red-fg}●{/}';
    const phi = `φ=${(MAX_CONFIDENCE * 100).toFixed(1)}%`;

    header.setContent(` {bold}🐕 CYNIC COCKPIT{/}                                             ${status} ${time}  ${phi}`);
  }

  /**
   * Update all widgets with new data
   */
  function update(data) {
    updateHeader(data);
    healthWidget.update(data);
    collectiveWidget.update(data);
    singularityWidget.update(data);
    chainWidget.update(data);
    verdictsWidget.update(data);
    eventsWidget.update(data, prevData);

    prevData = data;
    screen.render();
  }

  /**
   * Focus events log for scrolling
   */
  function focusEvents() {
    eventsWidget.focus();
  }

  /**
   * Show the screen
   */
  function show() {
    container.show();
    screen.render();
  }

  /**
   * Hide the screen
   */
  function hide() {
    container.hide();
  }

  /**
   * Push an event to the log
   */
  function pushEvent(event) {
    eventsWidget.pushEvent(event);
    screen.render();
  }

  return {
    container,
    update,
    show,
    hide,
    focusEvents,
    pushEvent,
  };
}

export default createMainScreen;
