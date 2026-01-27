/**
 * CYNIC Dashboard - Memory View
 * Phase 16 - Total Memory visualization
 *
 * Features:
 * - Search memories with natural language
 * - Browse decisions by project/type
 * - Review lessons with prevention tips
 * - Memory timeline visualization
 *
 * "φ remembers everything" - κυνικός
 */

import { Utils } from '../lib/utils.js';

/**
 * Memory types for filtering
 */
const MEMORY_TYPES = {
  all: { label: 'All Memories', icon: '📚' },
  summary: { label: 'Summaries', icon: '📝' },
  key_moment: { label: 'Key Moments', icon: '⭐' },
  decision: { label: 'Decisions', icon: '🧭' },
  preference: { label: 'Preferences', icon: '💡' },
};

/**
 * Decision types for filtering
 */
const DECISION_TYPES = {
  all: { label: 'All Types', icon: '📁' },
  pattern: { label: 'Patterns', icon: '🔄' },
  technology: { label: 'Technology', icon: '🔧' },
  structure: { label: 'Structure', icon: '🏗️' },
  naming: { label: 'Naming', icon: '🏷️' },
};

/**
 * Lesson severities
 */
const LESSON_SEVERITIES = {
  critical: { label: 'Critical', icon: '🔴', color: '#ef4444' },
  high: { label: 'High', icon: '🟠', color: '#f97316' },
  medium: { label: 'Medium', icon: '🟡', color: '#eab308' },
  low: { label: 'Low', icon: '🟢', color: '#22c55e' },
};

export class MemoryView {
  constructor(options = {}) {
    this.api = options.api || null;
    this.container = null;
    this.activeTab = 'search';

    // Data cache
    this.memories = [];
    this.decisions = [];
    this.lessons = [];
    this.searchResults = [];

    // Filters
    this.filters = {
      memoryType: 'all',
      decisionType: 'all',
      lessonSeverity: 'all',
      searchQuery: '',
    };
  }

  /**
   * Render memory view
   */
  render(container) {
    this.container = container;
    Utils.clearElement(container);
    container.classList.add('memory-view');

    // Header
    const header = Utils.createElement('div', { className: 'memory-header' }, [
      Utils.createElement('div', { className: 'memory-title' }, [
        Utils.createElement('h1', {}, ['📚 Total Memory']),
        Utils.createElement('span', { className: 'memory-subtitle' }, [
          'CYNIC remembers everything - search, browse, learn',
        ]),
      ]),
    ]);

    // Tab navigation
    const tabs = Utils.createElement('div', { className: 'memory-tabs' }, [
      this._createTab('search', '🔍', 'Search', 'Natural language memory search'),
      this._createTab('decisions', '🧭', 'Decisions', 'Architectural decisions'),
      this._createTab('lessons', '📖', 'Lessons', 'Mistakes and corrections'),
      this._createTab('timeline', '📅', 'Timeline', 'Memory timeline'),
    ]);

    // Tab content
    const content = Utils.createElement('div', {
      className: 'memory-content',
      id: 'memory-content',
    });

    container.appendChild(header);
    container.appendChild(tabs);
    container.appendChild(content);

    // Render initial tab
    this._renderActiveTab();
  }

  /**
   * Create tab button
   */
  _createTab(id, icon, label, description) {
    const isActive = this.activeTab === id;

    return Utils.createElement('button', {
      className: `memory-tab ${isActive ? 'active' : ''}`,
      dataset: { tab: id },
      onClick: () => this._switchTab(id),
    }, [
      Utils.createElement('span', { className: 'tab-icon' }, [icon]),
      Utils.createElement('div', { className: 'tab-text' }, [
        Utils.createElement('span', { className: 'tab-label' }, [label]),
        Utils.createElement('span', { className: 'tab-description' }, [description]),
      ]),
    ]);
  }

  /**
   * Switch tab
   */
  _switchTab(tabId) {
    this.activeTab = tabId;

    // Update tab buttons
    this.container?.querySelectorAll('.memory-tab').forEach(tab => {
      tab.classList.toggle('active', tab.dataset.tab === tabId);
    });

    // Render tab content
    this._renderActiveTab();
  }

  /**
   * Render active tab content
   */
  _renderActiveTab() {
    const content = document.getElementById('memory-content');
    if (!content) return;

    Utils.clearElement(content);

    switch (this.activeTab) {
      case 'search':
        this._renderSearchTab(content);
        break;
      case 'decisions':
        this._renderDecisionsTab(content);
        break;
      case 'lessons':
        this._renderLessonsTab(content);
        break;
      case 'timeline':
        this._renderTimelineTab(content);
        break;
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // SEARCH TAB
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Render search tab
   */
  _renderSearchTab(container) {
    // Search input
    const searchBox = Utils.createElement('div', { className: 'search-box' }, [
      Utils.createElement('input', {
        type: 'text',
        placeholder: 'Search memories... (e.g., "authentication patterns", "API decisions")',
        className: 'search-input',
        value: this.filters.searchQuery,
        onInput: (e) => this._onSearchInput(e.target.value),
        onKeydown: (e) => e.key === 'Enter' && this._performSearch(),
      }),
      Utils.createElement('button', {
        className: 'search-btn',
        onClick: () => this._performSearch(),
      }, ['🔍 Search']),
    ]);

    // Memory type filter
    const typeFilter = Utils.createElement('div', { className: 'filter-row' }, [
      Utils.createElement('span', { className: 'filter-label' }, ['Type:']),
      ...Object.entries(MEMORY_TYPES).map(([key, { label, icon }]) =>
        Utils.createElement('button', {
          className: `filter-btn ${this.filters.memoryType === key ? 'active' : ''}`,
          onClick: () => this._setMemoryTypeFilter(key),
        }, [`${icon} ${label}`])
      ),
    ]);

    // Results container
    const results = Utils.createElement('div', {
      className: 'search-results',
      id: 'search-results',
    });

    // Render current results
    if (this.searchResults.length > 0) {
      this._renderSearchResults(results);
    } else {
      results.appendChild(Utils.createElement('div', { className: 'empty-state' }, [
        Utils.createElement('span', { className: 'empty-icon' }, ['🔍']),
        Utils.createElement('p', {}, ['Enter a search query to find memories']),
        Utils.createElement('p', { className: 'hint' }, [
          'Try: "what patterns did we use", "decisions about authentication"'
        ]),
      ]));
    }

    container.appendChild(searchBox);
    container.appendChild(typeFilter);
    container.appendChild(results);
  }

  /**
   * Handle search input
   */
  _onSearchInput(value) {
    this.filters.searchQuery = value;
  }

  /**
   * Set memory type filter
   */
  _setMemoryTypeFilter(type) {
    this.filters.memoryType = type;
    this._renderActiveTab();
  }

  /**
   * Perform search
   */
  async _performSearch() {
    if (!this.api || !this.filters.searchQuery.trim()) return;

    try {
      // Use brain_memory_search tool
      const result = await this.api.callTool('brain_memory_search', {
        query: this.filters.searchQuery,
        type: this.filters.memoryType === 'all' ? undefined : this.filters.memoryType,
        limit: 20,
      });

      if (result.success) {
        this.searchResults = result.result?.memories || [];
        this._renderActiveTab();
      }
    } catch (err) {
      console.error('Search failed:', err);
    }
  }

  /**
   * Render search results
   */
  _renderSearchResults(container) {
    for (const memory of this.searchResults) {
      const type = MEMORY_TYPES[memory.memoryType] || MEMORY_TYPES.all;

      const card = Utils.createElement('div', { className: 'memory-card' }, [
        Utils.createElement('div', { className: 'memory-card-header' }, [
          Utils.createElement('span', { className: 'memory-type-badge' }, [
            `${type.icon} ${type.label}`
          ]),
          Utils.createElement('span', { className: 'memory-date' }, [
            this._formatDate(memory.createdAt)
          ]),
        ]),
        Utils.createElement('div', { className: 'memory-card-content' }, [
          memory.content
        ]),
        memory.importance && Utils.createElement('div', { className: 'memory-importance' }, [
          `Importance: ${this._renderImportanceBar(memory.importance)}`
        ]),
      ]);

      container.appendChild(card);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // DECISIONS TAB
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Render decisions tab
   */
  _renderDecisionsTab(container) {
    // Type filter
    const typeFilter = Utils.createElement('div', { className: 'filter-row' }, [
      Utils.createElement('span', { className: 'filter-label' }, ['Type:']),
      ...Object.entries(DECISION_TYPES).map(([key, { label, icon }]) =>
        Utils.createElement('button', {
          className: `filter-btn ${this.filters.decisionType === key ? 'active' : ''}`,
          onClick: () => this._setDecisionTypeFilter(key),
        }, [`${icon} ${label}`])
      ),
    ]);

    // Load button
    const loadBtn = Utils.createElement('button', {
      className: 'load-btn',
      onClick: () => this._loadDecisions(),
    }, ['📥 Load Decisions']);

    // Decisions list
    const list = Utils.createElement('div', {
      className: 'decisions-list',
      id: 'decisions-list',
    });

    if (this.decisions.length > 0) {
      this._renderDecisionsList(list);
    } else {
      list.appendChild(Utils.createElement('div', { className: 'empty-state' }, [
        Utils.createElement('span', { className: 'empty-icon' }, ['🧭']),
        Utils.createElement('p', {}, ['No decisions loaded']),
        Utils.createElement('p', { className: 'hint' }, ['Click "Load Decisions" to fetch architectural decisions']),
      ]));
    }

    container.appendChild(typeFilter);
    container.appendChild(loadBtn);
    container.appendChild(list);
  }

  /**
   * Set decision type filter
   */
  _setDecisionTypeFilter(type) {
    this.filters.decisionType = type;
    this._renderActiveTab();
  }

  /**
   * Load decisions
   */
  async _loadDecisions() {
    if (!this.api) return;

    try {
      const result = await this.api.callTool('brain_memory_search', {
        query: 'architectural decision',
        type: 'decision',
        limit: 50,
      });

      if (result.success) {
        this.decisions = result.result?.memories || [];
        this._renderActiveTab();
      }
    } catch (err) {
      console.error('Load decisions failed:', err);
    }
  }

  /**
   * Render decisions list
   */
  _renderDecisionsList(container) {
    const filtered = this.filters.decisionType === 'all'
      ? this.decisions
      : this.decisions.filter(d => d.decisionType === this.filters.decisionType);

    for (const decision of filtered) {
      const type = DECISION_TYPES[decision.decisionType] || DECISION_TYPES.pattern;

      const card = Utils.createElement('div', { className: 'decision-card' }, [
        Utils.createElement('div', { className: 'decision-header' }, [
          Utils.createElement('span', { className: 'decision-type' }, [
            `${type.icon} ${type.label}`
          ]),
          Utils.createElement('span', { className: 'decision-status' }, [
            decision.status || 'active'
          ]),
        ]),
        Utils.createElement('h3', { className: 'decision-title' }, [decision.title]),
        Utils.createElement('p', { className: 'decision-description' }, [
          decision.description
        ]),
        decision.rationale && Utils.createElement('div', { className: 'decision-rationale' }, [
          Utils.createElement('strong', {}, ['Rationale: ']),
          decision.rationale,
        ]),
        decision.projectPath && Utils.createElement('div', { className: 'decision-project' }, [
          `📁 ${decision.projectPath}`
        ]),
      ]);

      container.appendChild(card);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // LESSONS TAB
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Render lessons tab
   */
  _renderLessonsTab(container) {
    // Severity filter
    const severityFilter = Utils.createElement('div', { className: 'filter-row' }, [
      Utils.createElement('span', { className: 'filter-label' }, ['Severity:']),
      Utils.createElement('button', {
        className: `filter-btn ${this.filters.lessonSeverity === 'all' ? 'active' : ''}`,
        onClick: () => this._setLessonSeverityFilter('all'),
      }, ['📋 All']),
      ...Object.entries(LESSON_SEVERITIES).map(([key, { label, icon }]) =>
        Utils.createElement('button', {
          className: `filter-btn ${this.filters.lessonSeverity === key ? 'active' : ''}`,
          onClick: () => this._setLessonSeverityFilter(key),
        }, [`${icon} ${label}`])
      ),
    ]);

    // Load button
    const loadBtn = Utils.createElement('button', {
      className: 'load-btn',
      onClick: () => this._loadLessons(),
    }, ['📥 Load Lessons']);

    // Lessons list
    const list = Utils.createElement('div', {
      className: 'lessons-list',
      id: 'lessons-list',
    });

    if (this.lessons.length > 0) {
      this._renderLessonsList(list);
    } else {
      list.appendChild(Utils.createElement('div', { className: 'empty-state' }, [
        Utils.createElement('span', { className: 'empty-icon' }, ['📖']),
        Utils.createElement('p', {}, ['No lessons loaded']),
        Utils.createElement('p', { className: 'hint' }, ['Click "Load Lessons" to fetch learned lessons']),
      ]));
    }

    container.appendChild(severityFilter);
    container.appendChild(loadBtn);
    container.appendChild(list);
  }

  /**
   * Set lesson severity filter
   */
  _setLessonSeverityFilter(severity) {
    this.filters.lessonSeverity = severity;
    this._renderActiveTab();
  }

  /**
   * Load lessons
   */
  async _loadLessons() {
    if (!this.api) return;

    try {
      const result = await this.api.callTool('brain_self_correction', {
        action: 'lessons',
        limit: 50,
      });

      if (result.success) {
        this.lessons = result.result?.lessons || [];
        this._renderActiveTab();
      }
    } catch (err) {
      console.error('Load lessons failed:', err);
    }
  }

  /**
   * Render lessons list
   */
  _renderLessonsList(container) {
    const filtered = this.filters.lessonSeverity === 'all'
      ? this.lessons
      : this.lessons.filter(l => l.severity === this.filters.lessonSeverity);

    for (const lesson of filtered) {
      const severity = LESSON_SEVERITIES[lesson.severity] || LESSON_SEVERITIES.medium;

      const card = Utils.createElement('div', {
        className: 'lesson-card',
        style: `border-left: 4px solid ${severity.color}`,
      }, [
        Utils.createElement('div', { className: 'lesson-header' }, [
          Utils.createElement('span', { className: 'lesson-severity' }, [
            `${severity.icon} ${severity.label}`
          ]),
          Utils.createElement('span', { className: 'lesson-category' }, [
            lesson.category || 'general'
          ]),
          lesson.occurrenceCount > 1 && Utils.createElement('span', { className: 'lesson-count' }, [
            `${lesson.occurrenceCount}x`
          ]),
        ]),
        Utils.createElement('div', { className: 'lesson-mistake' }, [
          Utils.createElement('strong', {}, ['❌ Mistake: ']),
          lesson.mistake,
        ]),
        Utils.createElement('div', { className: 'lesson-correction' }, [
          Utils.createElement('strong', {}, ['✅ Correction: ']),
          lesson.correction,
        ]),
        lesson.prevention && Utils.createElement('div', { className: 'lesson-prevention' }, [
          Utils.createElement('strong', {}, ['🛡️ Prevention: ']),
          lesson.prevention,
        ]),
      ]);

      container.appendChild(card);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // TIMELINE TAB
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Render timeline tab
   */
  _renderTimelineTab(container) {
    // Timeline controls
    const controls = Utils.createElement('div', { className: 'timeline-controls' }, [
      Utils.createElement('button', {
        className: 'load-btn',
        onClick: () => this._loadTimeline(),
      }, ['📥 Load Timeline']),
    ]);

    // Timeline visualization
    const timeline = Utils.createElement('div', {
      className: 'memory-timeline',
      id: 'memory-timeline',
    });

    // Combine memories for timeline
    const allItems = [
      ...this.memories.map(m => ({ ...m, itemType: 'memory' })),
      ...this.decisions.map(d => ({ ...d, itemType: 'decision' })),
      ...this.lessons.map(l => ({ ...l, itemType: 'lesson' })),
    ].sort((a, b) => new Date(b.createdAt) - new Date(a.createdAt));

    if (allItems.length > 0) {
      this._renderTimeline(timeline, allItems);
    } else {
      timeline.appendChild(Utils.createElement('div', { className: 'empty-state' }, [
        Utils.createElement('span', { className: 'empty-icon' }, ['📅']),
        Utils.createElement('p', {}, ['No timeline data']),
        Utils.createElement('p', { className: 'hint' }, ['Load memories, decisions, or lessons first']),
      ]));
    }

    container.appendChild(controls);
    container.appendChild(timeline);
  }

  /**
   * Load timeline data
   */
  async _loadTimeline() {
    await Promise.all([
      this._loadDecisions(),
      this._loadLessons(),
    ]);
    this._renderActiveTab();
  }

  /**
   * Render timeline
   */
  _renderTimeline(container, items) {
    const timeline = Utils.createElement('div', { className: 'timeline' });

    let currentDate = null;

    for (const item of items.slice(0, 50)) {
      const itemDate = this._formatDate(item.createdAt, 'date');

      // Date separator
      if (itemDate !== currentDate) {
        currentDate = itemDate;
        timeline.appendChild(Utils.createElement('div', { className: 'timeline-date' }, [
          itemDate
        ]));
      }

      // Timeline item
      const typeConfig = {
        memory: { icon: '📝', color: '#3b82f6' },
        decision: { icon: '🧭', color: '#8b5cf6' },
        lesson: { icon: '📖', color: '#ef4444' },
      }[item.itemType] || { icon: '📄', color: '#6b7280' };

      const timelineItem = Utils.createElement('div', {
        className: 'timeline-item',
        style: `border-left-color: ${typeConfig.color}`,
      }, [
        Utils.createElement('div', { className: 'timeline-item-header' }, [
          Utils.createElement('span', { className: 'timeline-icon' }, [typeConfig.icon]),
          Utils.createElement('span', { className: 'timeline-time' }, [
            this._formatDate(item.createdAt, 'time')
          ]),
          Utils.createElement('span', { className: 'timeline-type' }, [item.itemType]),
        ]),
        Utils.createElement('div', { className: 'timeline-item-content' }, [
          item.title || item.content || item.mistake || 'No content'
        ]),
      ]);

      timeline.appendChild(timelineItem);
    }

    container.appendChild(timeline);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // HELPERS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Render importance bar
   */
  _renderImportanceBar(importance) {
    const filled = Math.round(importance * 5);
    return '█'.repeat(filled) + '░'.repeat(5 - filled) + ` ${Math.round(importance * 100)}%`;
  }

  /**
   * Format date
   */
  _formatDate(dateStr, format = 'full') {
    if (!dateStr) return 'Unknown';

    const date = new Date(dateStr);

    if (format === 'date') {
      return date.toLocaleDateString();
    }
    if (format === 'time') {
      return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    }
    return date.toLocaleString();
  }

  /**
   * Refresh all data
   */
  async refresh() {
    await this._loadTimeline();
  }

  /**
   * Cleanup
   */
  destroy() {
    this.container = null;
    this.memories = [];
    this.decisions = [];
    this.lessons = [];
    this.searchResults = [];
  }
}

// Export to window
window.CYNICMemoryView = MemoryView;
