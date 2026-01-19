/**
 * CYNIC Dashboard - Live View
 *
 * Real-time observation viewer for judgments, patterns, and events.
 * Inspired by claude-mem's web viewer approach.
 *
 * Features:
 * - Live SSE streaming of events
 * - Filter by type (judgment, pattern, digest, event)
 * - Timeline visualization
 * - Detail panel for selected items
 *
 * Note: Uses innerHTML for DOM rendering. All data comes from trusted
 * CYNIC server sources and is escaped before rendering.
 *
 * "φ distrusts φ" - κυνικός
 */

// Note: api could be used for REST calls in the future
// import { api } from '../api.js';
import { formatTimestamp, truncate, debounce } from '../lib/utils.js';

/**
 * Escape HTML to prevent XSS
 * @param {string} str - String to escape
 * @returns {string} Escaped string
 */
function escapeHtml(str) {
  if (typeof str !== 'string') return String(str);
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

/**
 * Live View class
 */
export class LiveView {
  constructor() {
    this.container = null;
    this.observations = [];
    this.maxObservations = 100;
    this.filters = {
      types: ['judgment', 'pattern', 'digest', 'event', 'block'],
      search: '',
    };
    this.selectedId = null;
    this.eventSource = null;
    this.isConnected = false;
    this.stats = {
      totalReceived: 0,
      judgments: 0,
      patterns: 0,
      blocks: 0,
      events: 0,
    };
  }

  /**
   * Initialize the view
   */
  init(container) {
    this.container = container;
    this.render();
    this.connectSSE();
    return this;
  }

  /**
   * Render the view (uses innerHTML with escaped content)
   */
  render() {
    if (!this.container) return;

    // All dynamic content is escaped via escapeHtml
    this.container.innerHTML = `
      <div class="live-view">
        <div class="live-header">
          <div class="live-status">
            <span class="status-indicator ${this.isConnected ? 'connected' : 'disconnected'}"></span>
            <span class="status-text">${this.isConnected ? 'Live' : 'Disconnected'}</span>
          </div>
          <div class="live-stats">
            <span class="stat" title="Total received">📥 ${this.stats.totalReceived}</span>
            <span class="stat" title="Judgments">⚖️ ${this.stats.judgments}</span>
            <span class="stat" title="Patterns">🔮 ${this.stats.patterns}</span>
            <span class="stat" title="Blocks">⛓️ ${this.stats.blocks}</span>
          </div>
          <div class="live-controls">
            <input type="text" class="search-input" placeholder="Search..." value="${escapeHtml(this.filters.search)}">
            <div class="filter-buttons">
              ${this.filters.types.map(type => `
                <button class="filter-btn active" data-type="${escapeHtml(type)}">
                  ${this._getTypeIcon(type)} ${escapeHtml(type)}
                </button>
              `).join('')}
            </div>
            <button class="clear-btn" title="Clear observations">🗑️</button>
          </div>
        </div>

        <div class="live-body">
          <div class="observations-list">
            ${this._renderObservations()}
          </div>
          <div class="observation-detail">
            ${this._renderDetail()}
          </div>
        </div>
      </div>
    `;

    this._attachEventListeners();
  }

  /**
   * Connect to SSE endpoint
   */
  connectSSE() {
    if (this.eventSource) {
      this.eventSource.close();
    }

    try {
      this.eventSource = new EventSource('/sse');

      this.eventSource.onopen = () => {
        this.isConnected = true;
        this._updateStatus();
        console.log('🔴 Live: SSE connected');
      };

      this.eventSource.onerror = () => {
        this.isConnected = false;
        this._updateStatus();
        console.error('🔴 Live: SSE disconnected');
      };

      // Listen for different event types
      this.eventSource.addEventListener('judgment', (e) => this._handleEvent('judgment', e));
      this.eventSource.addEventListener('block', (e) => this._handleEvent('block', e));
      this.eventSource.addEventListener('pattern', (e) => this._handleEvent('pattern', e));
      this.eventSource.addEventListener('message', (e) => this._handleEvent('event', e));

    } catch (err) {
      console.error('🔴 Live: Failed to connect SSE', err);
      this.isConnected = false;
    }
  }

  /**
   * Disconnect SSE
   */
  disconnect() {
    if (this.eventSource) {
      this.eventSource.close();
      this.eventSource = null;
    }
    this.isConnected = false;
    this._updateStatus();
  }

  /**
   * Handle incoming event
   */
  _handleEvent(type, event) {
    try {
      const data = JSON.parse(event.data);

      const observation = {
        id: `obs_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
        type,
        timestamp: Date.now(),
        data,
      };

      this.observations.unshift(observation);

      if (this.observations.length > this.maxObservations) {
        this.observations = this.observations.slice(0, this.maxObservations);
      }

      this.stats.totalReceived++;
      if (type === 'judgment') this.stats.judgments++;
      if (type === 'pattern') this.stats.patterns++;
      if (type === 'block') this.stats.blocks++;
      if (type === 'event') this.stats.events++;

      this._renderObservationsList();
      this._updateStats();

    } catch (err) {
      console.error('🔴 Live: Failed to parse event', err);
    }
  }

  /**
   * Render observations list
   */
  _renderObservations() {
    const filtered = this._filterObservations();

    if (filtered.length === 0) {
      return `
        <div class="empty-state">
          <span class="icon">📡</span>
          <p>Waiting for observations...</p>
          <p class="hint">Judgments, patterns, and events will appear here in real-time</p>
        </div>
      `;
    }

    return filtered.map(obs => this._renderObservationItem(obs)).join('');
  }

  /**
   * Render single observation item
   */
  _renderObservationItem(obs) {
    const isSelected = obs.id === this.selectedId;
    const summary = this._getSummary(obs);

    return `
      <div class="observation-item ${escapeHtml(obs.type)} ${isSelected ? 'selected' : ''}" data-id="${escapeHtml(obs.id)}">
        <div class="obs-header">
          <span class="obs-icon">${this._getTypeIcon(obs.type)}</span>
          <span class="obs-type">${escapeHtml(obs.type)}</span>
          <span class="obs-time">${formatTimestamp(obs.timestamp)}</span>
        </div>
        <div class="obs-summary">${escapeHtml(truncate(summary, 100))}</div>
        ${obs.type === 'judgment' ? this._renderJudgmentBadge(obs.data) : ''}
      </div>
    `;
  }

  /**
   * Render judgment badge
   */
  _renderJudgmentBadge(data) {
    const verdict = escapeHtml(data.verdict || 'UNKNOWN');
    const score = data.Q ?? data.score ?? 0;
    return `
      <div class="judgment-badge verdict-${verdict.toLowerCase()}">
        <span class="verdict">${verdict}</span>
        <span class="score">Q: ${score.toFixed(1)}</span>
      </div>
    `;
  }

  /**
   * Render detail panel
   */
  _renderDetail() {
    const obs = this.observations.find(o => o.id === this.selectedId);

    if (!obs) {
      return `
        <div class="detail-empty">
          <span class="icon">👆</span>
          <p>Select an observation to view details</p>
        </div>
      `;
    }

    return `
      <div class="detail-content">
        <div class="detail-header">
          <span class="type-badge ${escapeHtml(obs.type)}">${this._getTypeIcon(obs.type)} ${escapeHtml(obs.type)}</span>
          <span class="timestamp">${new Date(obs.timestamp).toLocaleString()}</span>
        </div>
        <div class="detail-body">
          ${this._renderDetailContent(obs)}
        </div>
        <div class="detail-actions">
          <button class="action-btn copy" data-action="copy" title="Copy JSON">📋 Copy</button>
        </div>
      </div>
    `;
  }

  /**
   * Render detail content based on type
   */
  _renderDetailContent(obs) {
    switch (obs.type) {
      case 'judgment':
        return this._renderJudgmentDetail(obs.data);
      case 'block':
        return this._renderBlockDetail(obs.data);
      case 'pattern':
        return this._renderPatternDetail(obs.data);
      default:
        return `<pre class="json-view">${escapeHtml(JSON.stringify(obs.data, null, 2))}</pre>`;
    }
  }

  /**
   * Render judgment detail
   */
  _renderJudgmentDetail(data) {
    const verdict = escapeHtml(data.verdict || 'UNKNOWN');
    const score = data.Q ?? data.score ?? 0;

    let breakdownHtml = '';
    if (data.breakdown) {
      const rows = Object.entries(data.breakdown).map(([axiom, axScore]) => `
        <div class="axiom-row">
          <span class="axiom-name">${escapeHtml(axiom)}</span>
          <div class="axiom-bar">
            <div class="fill" style="width: ${(axScore / 25 * 100).toFixed(1)}%"></div>
          </div>
          <span class="axiom-score">${axScore.toFixed(1)}</span>
        </div>
      `).join('');
      breakdownHtml = `<div class="breakdown"><h4>Axiom Breakdown</h4>${rows}</div>`;
    }

    return `
      <div class="judgment-detail">
        <div class="verdict-large verdict-${verdict.toLowerCase()}">${verdict}</div>
        <div class="score-display">
          <span class="label">Q-Score</span>
          <span class="value">${score.toFixed(2)}</span>
        </div>
        ${breakdownHtml}
        ${data.confidence ? `<div class="confidence">Confidence: ${(data.confidence * 100).toFixed(1)}%</div>` : ''}
      </div>
    `;
  }

  /**
   * Render block detail
   */
  _renderBlockDetail(data) {
    return `
      <div class="block-detail">
        <div class="block-info">
          <div class="info-row">
            <span class="label">Slot</span>
            <span class="value">${data.slot ?? 'N/A'}</span>
          </div>
          <div class="info-row">
            <span class="label">Judgments</span>
            <span class="value">${data.judgments?.length || 0}</span>
          </div>
          ${data.hash ? `
            <div class="info-row">
              <span class="label">Hash</span>
              <span class="value hash">${escapeHtml(String(data.hash).slice(0, 16))}...</span>
            </div>
          ` : ''}
        </div>
      </div>
    `;
  }

  /**
   * Render pattern detail
   */
  _renderPatternDetail(data) {
    return `
      <div class="pattern-detail">
        <div class="pattern-category">${escapeHtml(data.category || 'unknown')}</div>
        <pre class="json-view">${escapeHtml(JSON.stringify(data, null, 2))}</pre>
      </div>
    `;
  }

  /**
   * Filter observations
   */
  _filterObservations() {
    return this.observations.filter(obs => {
      if (!this.filters.types.includes(obs.type)) return false;

      if (this.filters.search) {
        const searchLower = this.filters.search.toLowerCase();
        const summary = this._getSummary(obs).toLowerCase();
        if (!summary.includes(searchLower)) return false;
      }

      return true;
    });
  }

  /**
   * Get summary text
   */
  _getSummary(obs) {
    switch (obs.type) {
      case 'judgment':
        return `${obs.data.verdict || 'UNKNOWN'} - Q: ${(obs.data.Q ?? obs.data.score ?? 0).toFixed(1)}`;
      case 'block':
        return `Block #${obs.data.slot ?? '?'} with ${obs.data.judgments?.length || 0} judgments`;
      case 'pattern':
        return `${obs.data.category || 'Pattern'}: ${obs.data.total || 0} occurrences`;
      default:
        return JSON.stringify(obs.data).slice(0, 100);
    }
  }

  /**
   * Get icon for type
   */
  _getTypeIcon(type) {
    const icons = { judgment: '⚖️', pattern: '🔮', block: '⛓️', digest: '📖', event: '📡' };
    return icons[type] || '📋';
  }

  /**
   * Update status
   */
  _updateStatus() {
    const indicator = this.container?.querySelector('.status-indicator');
    const text = this.container?.querySelector('.status-text');
    if (indicator) {
      indicator.classList.toggle('connected', this.isConnected);
      indicator.classList.toggle('disconnected', !this.isConnected);
    }
    if (text) {
      text.textContent = this.isConnected ? 'Live' : 'Disconnected';
    }
  }

  /**
   * Update stats
   */
  _updateStats() {
    const statsEl = this.container?.querySelector('.live-stats');
    if (statsEl) {
      statsEl.innerHTML = `
        <span class="stat">📥 ${this.stats.totalReceived}</span>
        <span class="stat">⚖️ ${this.stats.judgments}</span>
        <span class="stat">🔮 ${this.stats.patterns}</span>
        <span class="stat">⛓️ ${this.stats.blocks}</span>
      `;
    }
  }

  /**
   * Re-render observations list
   */
  _renderObservationsList() {
    const listEl = this.container?.querySelector('.observations-list');
    if (listEl) {
      listEl.innerHTML = this._renderObservations();
      this._attachListEventListeners();
    }
  }

  /**
   * Attach event listeners
   */
  _attachEventListeners() {
    const searchInput = this.container?.querySelector('.search-input');
    if (searchInput) {
      searchInput.addEventListener('input', debounce((e) => {
        this.filters.search = e.target.value;
        this._renderObservationsList();
      }, 300));
    }

    this.container?.querySelectorAll('.filter-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const type = btn.dataset.type;
        btn.classList.toggle('active');
        if (this.filters.types.includes(type)) {
          this.filters.types = this.filters.types.filter(t => t !== type);
        } else {
          this.filters.types.push(type);
        }
        this._renderObservationsList();
      });
    });

    const clearBtn = this.container?.querySelector('.clear-btn');
    if (clearBtn) {
      clearBtn.addEventListener('click', () => {
        this.observations = [];
        this.selectedId = null;
        this.render();
      });
    }

    this._attachListEventListeners();
  }

  /**
   * Attach list event listeners
   */
  _attachListEventListeners() {
    this.container?.querySelectorAll('.observation-item').forEach(item => {
      item.addEventListener('click', () => {
        this.selectedId = item.dataset.id;
        this._updateSelection();
        this._updateDetail();
      });
    });

    this.container?.querySelectorAll('.action-btn[data-action="copy"]').forEach(btn => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const obs = this.observations.find(o => o.id === this.selectedId);
        if (obs) {
          navigator.clipboard.writeText(JSON.stringify(obs.data, null, 2));
        }
      });
    });
  }

  /**
   * Update selection
   */
  _updateSelection() {
    this.container?.querySelectorAll('.observation-item').forEach(item => {
      item.classList.toggle('selected', item.dataset.id === this.selectedId);
    });
  }

  /**
   * Update detail panel
   */
  _updateDetail() {
    const detailEl = this.container?.querySelector('.observation-detail');
    if (detailEl) {
      detailEl.innerHTML = this._renderDetail();
      this._attachListEventListeners();
    }
  }

  /**
   * Cleanup
   */
  destroy() {
    this.disconnect();
    this.container = null;
  }
}

// Export singleton
export const liveView = new LiveView();
window.CYNICLiveView = liveView;
