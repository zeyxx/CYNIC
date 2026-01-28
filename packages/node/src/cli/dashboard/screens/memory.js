/**
 * CYNIC TUI Dashboard - Memory Screen
 *
 * Search and browse memories, decisions, and lessons learned.
 * Phase 18: "φ remembers everything"
 *
 * @module @cynic/node/cli/dashboard/screens/memory
 */

'use strict';

import blessed from 'blessed';
import { COLORS, progressBar } from '../theme.js';

/**
 * Memory types with colors
 */
const MEMORY_TYPES = {
  summary: { label: 'Summary', color: 'cyan', icon: '📝' },
  key_moment: { label: 'Key Moment', color: 'yellow', icon: '⭐' },
  decision: { label: 'Decision', color: 'green', icon: '🎯' },
  preference: { label: 'Preference', color: 'magenta', icon: '💡' },
  lesson: { label: 'Lesson', color: 'red', icon: '📚' },
};

/**
 * Create Memory Screen
 */
export function createMemoryScreen(screen, dataFetcher, options = {}) {
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
    style: { bg: 'cyan', fg: 'black' },
    content: ' {bold}🧠 MEMORY - Search & Browse{/}',
    tags: true,
  });

  // Search box
  const searchBox = blessed.textbox({
    parent: container,
    label: ' Search ',
    top: 1,
    left: 0,
    width: '100%',
    height: 3,
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.primary },
      label: { fg: COLORS.primary, bold: true },
      focus: { border: { fg: COLORS.success } },
    },
    inputOnFocus: true,
    mouse: true,
  });

  // Memory list
  const memoryList = blessed.list({
    parent: container,
    label: ' Memories ',
    top: 4,
    left: 0,
    width: '50%',
    height: '75%',
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.success },
      label: { fg: COLORS.success, bold: true },
      selected: { bg: 'cyan', fg: 'black' },
    },
    tags: true,
    scrollable: true,
    keys: true,
    vi: true,
    mouse: true,
  });

  // Memory detail
  const detailPanel = blessed.box({
    parent: container,
    label: ' Detail ',
    top: 4,
    left: '50%',
    width: '50%',
    height: '75%',
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.warning },
      label: { fg: COLORS.warning, bold: true },
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
  });

  // Stats bar
  const statsBar = blessed.box({
    parent: container,
    label: ' Statistics ',
    top: '79%',
    left: 0,
    width: '100%',
    height: 5,
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.primary },
      label: { fg: COLORS.primary, bold: true },
    },
    tags: true,
  });

  // Footer
  const footer = blessed.box({
    parent: container,
    bottom: 0,
    left: 0,
    width: '100%',
    height: 1,
    style: { bg: 'black', fg: 'white' },
    content: ' {bold}[/]{/} Search  {bold}[↑↓]{/} Navigate  {bold}[Enter]{/} Details  {bold}[D]{/}ecisions  {bold}[L]{/}essons  {bold}[B]{/}ack',
    tags: true,
  });

  // Cache data
  let memories = [];
  let stats = { total: 0, byType: {} };
  let currentFilter = 'all';
  let searchQuery = '';

  /**
   * Fetch memories
   */
  async function fetchMemories(query = '') {
    const result = await dataFetcher.callTool('brain_search', {
      query: query || '*',
      limit: 50,
      filter: currentFilter !== 'all' ? { type: currentFilter } : undefined,
    });

    if (result.success) {
      memories = result.result.results || [];
      stats = result.result.stats || stats;
      updateMemoryList();
      updateStats();
    }
  }

  /**
   * Update memory list
   */
  function updateMemoryList() {
    const items = memories.map(m => {
      const type = MEMORY_TYPES[m.type] || MEMORY_TYPES.summary;
      const time = new Date(m.timestamp || m.created_at).toLocaleDateString();
      const importance = m.importance ? `${Math.round(m.importance * 100)}%` : '';

      return `${type.icon} {${type.color}-fg}[${type.label}]{/} ${time} ${m.content?.slice(0, 30) || m.title?.slice(0, 30)}... ${importance}`;
    });

    if (items.length === 0) {
      items.push('{gray-fg}No memories found. Try a different search.{/}');
    }

    memoryList.setItems(items);
  }

  /**
   * Update stats bar
   */
  function updateStats() {
    const byType = stats.byType || {};
    const parts = Object.entries(byType)
      .map(([type, count]) => {
        const t = MEMORY_TYPES[type] || { icon: '?', color: 'white' };
        return `${t.icon} ${type}: {${t.color}-fg}${count}{/}`;
      })
      .join('  ');

    statsBar.setContent(` Total: {bold}${stats.total || memories.length}{/}  |  ${parts || 'No stats'}`);
  }

  /**
   * Show memory detail
   */
  function showDetail(index) {
    if (index >= memories.length) {
      detailContent.setContent('No memory selected');
      return;
    }

    const m = memories[index];
    const type = MEMORY_TYPES[m.type] || MEMORY_TYPES.summary;

    const lines = [
      `{bold}${type.icon} ${type.label}{/}`,
      '',
      `{bold}ID:{/} ${m.id || 'N/A'}`,
      `{bold}Created:{/} ${new Date(m.timestamp || m.created_at).toLocaleString()}`,
    ];

    if (m.importance !== undefined) {
      const bar = progressBar(m.importance * 100, 20);
      lines.push(`{bold}Importance:{/} ${bar} ${Math.round(m.importance * 100)}%`);
    }

    lines.push('');
    lines.push('{bold}Content:{/}');
    lines.push(`  ${m.content || m.description || 'No content'}`);

    if (m.context) {
      lines.push('');
      lines.push('{bold}Context:{/}');
      for (const [key, value] of Object.entries(m.context)) {
        lines.push(`  ${key}: ${JSON.stringify(value)}`);
      }
    }

    // Type-specific fields
    if (m.type === 'decision' && m.rationale) {
      lines.push('');
      lines.push('{bold}Rationale:{/}');
      lines.push(`  ${m.rationale}`);
    }

    if (m.type === 'lesson') {
      if (m.mistake) {
        lines.push('');
        lines.push('{bold}Mistake:{/}');
        lines.push(`  ${m.mistake}`);
      }
      if (m.correction) {
        lines.push('');
        lines.push('{bold}Correction:{/}');
        lines.push(`  ${m.correction}`);
      }
      if (m.prevention) {
        lines.push('');
        lines.push('{bold}Prevention:{/}');
        lines.push(`  ${m.prevention}`);
      }
    }

    detailContent.setContent(lines.join('\n'));
    screen.render();
  }

  // Event handlers
  memoryList.on('select', (item, index) => {
    showDetail(index);
  });

  searchBox.on('submit', (value) => {
    searchQuery = value;
    fetchMemories(value);
    memoryList.focus();
  });

  /**
   * Filter by decisions
   */
  function filterDecisions() {
    currentFilter = currentFilter === 'decision' ? 'all' : 'decision';
    fetchMemories(searchQuery);
  }

  /**
   * Filter by lessons
   */
  function filterLessons() {
    currentFilter = currentFilter === 'lesson' ? 'all' : 'lesson';
    fetchMemories(searchQuery);
  }

  /**
   * Focus search
   */
  function focusSearch() {
    searchBox.focus();
  }

  /**
   * Update with data
   */
  function update(data) {
    if (data?.memories) {
      memories = data.memories;
      updateMemoryList();
    }
    if (data?.memoryStats) {
      stats = data.memoryStats;
      updateStats();
    }
  }

  /**
   * Show the screen
   */
  async function show() {
    container.show();
    memoryList.focus();
    await fetchMemories();
    if (memories.length > 0) {
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

  /**
   * Get selected index
   */
  function getSelectedIndex() {
    return memoryList.selected || 0;
  }

  return {
    container,
    update,
    show,
    hide,
    focusSearch,
    filterDecisions,
    filterLessons,
    getSelectedIndex,
  };
}

export default createMemoryScreen;
