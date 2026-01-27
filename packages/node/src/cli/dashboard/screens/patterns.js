/**
 * CYNIC TUI Dashboard - Patterns Screen
 *
 * Pattern gallery view
 *
 * @module @cynic/node/cli/dashboard/screens/patterns
 */

'use strict';

import blessed from 'blessed';
import { COLORS, formatTime, truncate, progressBar } from '../theme.js';

/**
 * Create Patterns Screen
 */
export function createPatternsScreen(screen, dataFetcher, options = {}) {
  const container = blessed.box({
    parent: screen,
    top: 0,
    left: 0,
    width: '100%',
    height: '100%',
    tags: true,
    hidden: true,
  });

  // Header
  const header = blessed.box({
    parent: container,
    top: 0,
    left: 0,
    width: '100%',
    height: 1,
    style: { bg: 'yellow', fg: 'black' },
    content: ' {bold}🔍 PATTERNS - Detected Learning Patterns{/}',
    tags: true,
  });

  // Category filter
  const categories = ['all', 'code', 'architecture', 'security', 'performance', 'behavior'];
  let currentCategory = 'all';

  // Pattern list
  const patternList = blessed.list({
    parent: container,
    label: ' Patterns ',
    top: 1,
    left: 0,
    width: '40%',
    height: '90%',
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.warning },
      label: { fg: COLORS.warning, bold: true },
      selected: { bg: 'yellow', fg: 'black' },
    },
    tags: true,
    scrollable: true,
    keys: true,
    vi: true,
    mouse: true,
  });

  // Pattern detail
  const detailPanel = blessed.box({
    parent: container,
    label: ' Pattern Detail ',
    top: 1,
    left: '40%',
    width: '60%',
    height: '90%',
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.primary },
      label: { fg: COLORS.primary, bold: true },
    },
    tags: true,
  });

  const detailContent = blessed.box({
    parent: detailPanel,
    top: 0,
    left: 1,
    right: 1,
    bottom: 0,
    tags: true,
    scrollable: true,
    keys: true,
    vi: true,
    mouse: true,
  });

  // Footer
  const footer = blessed.box({
    parent: container,
    bottom: 0,
    left: 0,
    width: '100%',
    height: 1,
    style: { bg: 'black', fg: 'white' },
    content: ` {bold}[←→]{/} Category: ${currentCategory}  {bold}[↑↓]{/} Navigate  {bold}[Enter]{/} Details  {bold}[B]{/}ack  {bold}[Q]{/}uit`,
    tags: true,
  });

  // Patterns cache
  let patterns = [];

  /**
   * Fetch patterns from API
   */
  async function fetchPatterns() {
    const result = await dataFetcher.callTool('brain_patterns', {
      category: currentCategory,
      limit: 34, // Fibonacci
    });

    if (result.success && result.result?.patterns) {
      patterns = result.result.patterns;
      updatePatternList();
    }
  }

  /**
   * Update pattern list display
   */
  function updatePatternList() {
    const items = patterns.map(p => {
      const conf = Math.round((p.confidence || 0.5) * 100);
      const type = truncate(p.type || p.patternType || 'unknown', 15);
      return `${type.padEnd(15)} │ ${conf}%`;
    });

    patternList.setItems(items.length > 0 ? items : ['No patterns found']);
  }

  /**
   * Show pattern detail
   */
  function showDetail(index) {
    const pattern = patterns[index];
    if (!pattern) {
      detailContent.setContent('No pattern selected');
      return;
    }

    const conf = (pattern.confidence || 0.5) * 100;
    const occurrences = pattern.occurrences || pattern.count || 1;

    const lines = [
      `{bold}Type:{/} ${pattern.type || pattern.patternType || 'unknown'}`,
      `{bold}Category:{/} ${pattern.category || 'general'}`,
      '',
      `{bold}Confidence:{/} ${progressBar(conf, 100, 10)} ${conf.toFixed(1)}%`,
      `{bold}Occurrences:{/} ${occurrences}`,
      `{bold}First Seen:{/} ${pattern.firstSeen ? formatTime(pattern.firstSeen) : 'unknown'}`,
      `{bold}Last Seen:{/} ${pattern.lastSeen ? formatTime(pattern.lastSeen) : 'unknown'}`,
      '',
      '{bold}Description:{/}',
      pattern.description || pattern.summary || 'No description available',
      '',
      '{bold}Evidence:{/}',
    ];

    // Add evidence/examples
    const evidence = pattern.evidence || pattern.examples || [];
    for (const e of evidence.slice(0, 5)) {
      lines.push(`  • ${truncate(typeof e === 'string' ? e : JSON.stringify(e), 50)}`);
    }

    if (pattern.insight) {
      lines.push('');
      lines.push('{bold}Insight:{/}');
      lines.push(pattern.insight);
    }

    detailContent.setContent(lines.join('\n'));
    screen.render();
  }

  // Event handlers
  patternList.on('select', (item, index) => {
    showDetail(index);
  });

  /**
   * Cycle through categories
   */
  function nextCategory() {
    const idx = categories.indexOf(currentCategory);
    currentCategory = categories[(idx + 1) % categories.length];
    updateFooter();
    fetchPatterns();
  }

  function prevCategory() {
    const idx = categories.indexOf(currentCategory);
    currentCategory = categories[(idx - 1 + categories.length) % categories.length];
    updateFooter();
    fetchPatterns();
  }

  function updateFooter() {
    footer.setContent(` {bold}[←→]{/} Category: ${currentCategory}  {bold}[↑↓]{/} Navigate  {bold}[Enter]{/} Details  {bold}[B]{/}ack  {bold}[Q]{/}uit`);
    screen.render();
  }

  /**
   * Update screen with data (for consistency)
   */
  function update(data) {
    // Patterns are fetched separately
  }

  /**
   * Show the screen
   */
  async function show() {
    container.show();
    patternList.focus();
    await fetchPatterns();
    if (patterns.length > 0) {
      showDetail(0);
    }
    screen.render();
  }

  /**
   * Hide the screen
   */
  function hide() {
    container.hide();
  }

  return {
    container,
    update,
    show,
    hide,
    nextCategory,
    prevCategory,
  };
}

export default createPatternsScreen;
