/**
 * CYNIC Dashboard - Ecosystem View
 * Real-time GitHub monitoring, sources, updates
 *
 * "Le chien qui surveille l'horizon" - κυνικός
 */

import { Utils } from '../lib/utils.js';

const PHI_INV = 0.618;

// Priority colors
const PRIORITY_COLORS = {
  CRITICAL: '#ff4444',
  HIGH: '#ff8800',
  MEDIUM: '#ffcc00',
  LOW: '#88cc00',
  INFO: '#888888',
};

// Update type icons
const UPDATE_ICONS = {
  COMMIT: '📝',
  RELEASE: '🚀',
  ISSUE: '🐛',
  PR: '🔀',
  ANNOUNCEMENT: '📢',
};

export class EcosystemView {
  constructor(options = {}) {
    this.api = options.api;
    this.container = null;
    this.sources = [];
    this.updates = [];
    this.refreshInterval = null;
    this.stats = {
      totalSources: 0,
      totalUpdates: 0,
      lastFetch: null,
    };
  }

  /**
   * Render ecosystem view
   */
  render(container) {
    this.container = container;
    Utils.clearElement(container);
    container.classList.add('ecosystem-view');

    // Create layout
    const layout = Utils.createElement('div', { className: 'ecosystem-layout' });

    // Header section
    const header = this._renderHeader();
    layout.appendChild(header);

    // Main content: Sources + Updates
    const mainContent = Utils.createElement('div', { className: 'ecosystem-main' });

    // Left: Sources panel
    const sourcesPanel = Utils.createElement('section', { className: 'ecosystem-panel sources-panel' });
    sourcesPanel.innerHTML = `
      <div class="panel-header">
        <h2>📡 Tracked Sources</h2>
        <button class="btn btn-sm" id="add-source-btn">+ Add</button>
      </div>
      <div class="sources-list" id="sources-container">
        <div class="loading">Loading sources...</div>
      </div>
    `;
    mainContent.appendChild(sourcesPanel);

    // Right: Updates feed
    const updatesPanel = Utils.createElement('section', { className: 'ecosystem-panel updates-panel' });
    updatesPanel.innerHTML = `
      <div class="panel-header">
        <h2>📨 Recent Updates</h2>
        <div class="filter-controls">
          <select id="priority-filter">
            <option value="all">All Priorities</option>
            <option value="CRITICAL">Critical</option>
            <option value="HIGH">High</option>
            <option value="MEDIUM">Medium</option>
            <option value="LOW">Low</option>
          </select>
          <select id="type-filter">
            <option value="all">All Types</option>
            <option value="COMMIT">Commits</option>
            <option value="RELEASE">Releases</option>
            <option value="ISSUE">Issues</option>
            <option value="PR">PRs</option>
          </select>
        </div>
      </div>
      <div class="updates-feed" id="updates-container">
        <div class="loading">Loading updates...</div>
      </div>
    `;
    mainContent.appendChild(updatesPanel);

    layout.appendChild(mainContent);

    // Stats bar at bottom
    const statsBar = Utils.createElement('div', { className: 'ecosystem-stats', id: 'stats-bar' });
    layout.appendChild(statsBar);

    container.appendChild(layout);

    // Bind events
    this._bindEvents();

    // Initial load
    this.refresh();

    // Auto-refresh every φ × 60 seconds
    this.refreshInterval = setInterval(() => this.refresh(), Math.round(PHI_INV * 60 * 1000));
  }

  /**
   * Render header with actions
   */
  _renderHeader() {
    const header = Utils.createElement('div', { className: 'ecosystem-header' });
    header.innerHTML = `
      <div class="header-left">
        <h1>🌐 Ecosystem Monitor</h1>
        <span class="subtitle">Real-time GitHub tracking</span>
      </div>
      <div class="header-right">
        <button class="btn btn-primary" id="fetch-all-btn">
          🔄 Fetch All
        </button>
        <button class="btn" id="refresh-btn">
          ↻ Refresh
        </button>
      </div>
    `;
    return header;
  }

  /**
   * Bind UI events
   */
  _bindEvents() {
    // Fetch all button
    document.getElementById('fetch-all-btn')?.addEventListener('click', () => {
      this._fetchAll();
    });

    // Refresh button
    document.getElementById('refresh-btn')?.addEventListener('click', () => {
      this.refresh();
    });

    // Add source button
    document.getElementById('add-source-btn')?.addEventListener('click', () => {
      this._showAddSourceDialog();
    });

    // Filters
    document.getElementById('priority-filter')?.addEventListener('change', () => {
      this._renderUpdates();
    });
    document.getElementById('type-filter')?.addEventListener('change', () => {
      this._renderUpdates();
    });
  }

  /**
   * Refresh data from API
   */
  async refresh() {
    await Promise.all([
      this._loadSources(),
      this._loadUpdates(),
    ]);
    this._updateStats();
  }

  /**
   * Load sources from ecosystem monitor
   */
  async _loadSources() {
    try {
      const result = await this.api.callTool('brain_ecosystem_monitor', { action: 'sources' });
      if (result.success && result.result) {
        this.sources = result.result.sources || [];
        this._renderSources();
      }
    } catch (err) {
      console.error('Failed to load sources:', err);
    }
  }

  /**
   * Load updates from ecosystem monitor
   */
  async _loadUpdates() {
    try {
      const result = await this.api.callTool('brain_ecosystem_monitor', { action: 'updates', limit: 50 });
      if (result.success && result.result) {
        this.updates = result.result.updates || [];
        this._renderUpdates();
      }
    } catch (err) {
      console.error('Failed to load updates:', err);
    }
  }

  /**
   * Render sources list
   */
  _renderSources() {
    const container = document.getElementById('sources-container');
    if (!container) return;

    if (this.sources.length === 0) {
      container.innerHTML = `
        <div class="empty-state">
          <p>No sources tracked yet.</p>
          <button class="btn btn-primary" id="add-first-source">+ Add your first source</button>
        </div>
      `;
      document.getElementById('add-first-source')?.addEventListener('click', () => {
        this._showAddSourceDialog();
      });
      return;
    }

    container.innerHTML = this.sources.map(source => this._renderSourceCard(source)).join('');

    // Bind source actions
    this.sources.forEach(source => {
      document.getElementById(`fetch-${source.id}`)?.addEventListener('click', () => {
        this._fetchSource(source.id);
      });
      document.getElementById(`remove-${source.id}`)?.addEventListener('click', () => {
        this._removeSource(source.id);
      });
    });
  }

  /**
   * Render a single source card
   */
  _renderSourceCard(source) {
    const lastFetch = source.lastFetch
      ? new Date(source.lastFetch).toLocaleTimeString()
      : 'Never';

    const healthClass = source.errorCount > 0 ? 'unhealthy' : 'healthy';
    const branchBadge = source.branch !== 'main' ? `<span class="branch-badge">${source.branch}</span>` : '';

    return `
      <div class="source-card ${healthClass}" data-id="${source.id}">
        <div class="source-header">
          <span class="source-icon">📦</span>
          <span class="source-name">${source.owner}/${source.repo}</span>
          ${branchBadge}
        </div>
        <div class="source-meta">
          <span class="meta-item" title="Updates">📨 ${source.updatesCount || 0}</span>
          <span class="meta-item" title="Fetches">🔄 ${source.fetchCount || 0}</span>
          <span class="meta-item" title="Errors">❌ ${source.errorCount || 0}</span>
        </div>
        <div class="source-tracking">
          ${source.trackCommits ? '<span class="track-badge">Commits</span>' : ''}
          ${source.trackReleases ? '<span class="track-badge">Releases</span>' : ''}
          ${source.trackIssues ? '<span class="track-badge">Issues</span>' : ''}
        </div>
        <div class="source-footer">
          <span class="last-fetch">Last: ${lastFetch}</span>
          <div class="source-actions">
            <button class="btn btn-xs" id="fetch-${source.id}" title="Fetch now">🔄</button>
            <button class="btn btn-xs btn-danger" id="remove-${source.id}" title="Remove">✕</button>
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Render updates feed
   */
  _renderUpdates() {
    const container = document.getElementById('updates-container');
    if (!container) return;

    // Get filters
    const priorityFilter = document.getElementById('priority-filter')?.value || 'all';
    const typeFilter = document.getElementById('type-filter')?.value || 'all';

    // Filter updates
    let filtered = this.updates;
    if (priorityFilter !== 'all') {
      filtered = filtered.filter(u => u.priority === priorityFilter);
    }
    if (typeFilter !== 'all') {
      filtered = filtered.filter(u => u.type === typeFilter);
    }

    if (filtered.length === 0) {
      container.innerHTML = `
        <div class="empty-state">
          <p>No updates match your filters.</p>
        </div>
      `;
      return;
    }

    // Sort by timestamp (newest first)
    filtered.sort((a, b) => (b.timestamp || 0) - (a.timestamp || 0));

    container.innerHTML = filtered.map(update => this._renderUpdateCard(update)).join('');
  }

  /**
   * Render a single update card
   */
  _renderUpdateCard(update) {
    const icon = UPDATE_ICONS[update.type] || '📋';
    const priorityColor = PRIORITY_COLORS[update.priority] || '#888';
    const time = update.timestamp
      ? this._formatRelativeTime(update.timestamp)
      : 'Unknown';
    const repo = update.meta?.repo || 'Unknown';

    return `
      <div class="update-card" style="border-left: 3px solid ${priorityColor}">
        <div class="update-header">
          <span class="update-icon">${icon}</span>
          <span class="update-type">${update.type}</span>
          <span class="update-priority" style="color: ${priorityColor}">${update.priority}</span>
          <span class="update-time">${time}</span>
        </div>
        <div class="update-title">
          <a href="${update.url}" target="_blank" rel="noopener">${this._escapeHtml(update.title || 'No title')}</a>
        </div>
        <div class="update-meta">
          <span class="update-repo">📦 ${repo}</span>
          <span class="update-author">👤 ${update.author || 'Unknown'}</span>
          <span class="update-id">#${update.id}</span>
        </div>
      </div>
    `;
  }

  /**
   * Update stats bar
   */
  _updateStats() {
    const statsBar = document.getElementById('stats-bar');
    if (!statsBar) return;

    const totalUpdates = this.updates.length;
    const byPriority = {};
    const byType = {};

    this.updates.forEach(u => {
      byPriority[u.priority] = (byPriority[u.priority] || 0) + 1;
      byType[u.type] = (byType[u.type] || 0) + 1;
    });

    statsBar.innerHTML = `
      <div class="stat">
        <span class="stat-label">Sources</span>
        <span class="stat-value">${this.sources.length}</span>
      </div>
      <div class="stat">
        <span class="stat-label">Updates</span>
        <span class="stat-value">${totalUpdates}</span>
      </div>
      <div class="stat">
        <span class="stat-label">Critical</span>
        <span class="stat-value" style="color: ${PRIORITY_COLORS.CRITICAL}">${byPriority.CRITICAL || 0}</span>
      </div>
      <div class="stat">
        <span class="stat-label">High</span>
        <span class="stat-value" style="color: ${PRIORITY_COLORS.HIGH}">${byPriority.HIGH || 0}</span>
      </div>
      <div class="stat">
        <span class="stat-label">Releases</span>
        <span class="stat-value">${byType.RELEASE || 0}</span>
      </div>
      <div class="stat">
        <span class="stat-label">Commits</span>
        <span class="stat-value">${byType.COMMIT || 0}</span>
      </div>
      <div class="stat stat-phi">
        <span class="stat-label">φ⁻¹</span>
        <span class="stat-value">61.8%</span>
      </div>
    `;
  }

  /**
   * Fetch all sources
   */
  async _fetchAll() {
    const btn = document.getElementById('fetch-all-btn');
    if (btn) {
      btn.disabled = true;
      btn.textContent = '⏳ Fetching...';
    }

    try {
      await this.api.callTool('brain_ecosystem_monitor', { action: 'fetch' });
      await this.refresh();
    } catch (err) {
      console.error('Fetch all failed:', err);
    } finally {
      if (btn) {
        btn.disabled = false;
        btn.textContent = '🔄 Fetch All';
      }
    }
  }

  /**
   * Fetch single source
   */
  async _fetchSource(sourceId) {
    const btn = document.getElementById(`fetch-${sourceId}`);
    if (btn) {
      btn.disabled = true;
      btn.textContent = '⏳';
    }

    try {
      await this.api.callTool('brain_ecosystem_monitor', { action: 'fetch', sourceId });
      await this.refresh();
    } catch (err) {
      console.error('Fetch source failed:', err);
    } finally {
      if (btn) {
        btn.disabled = false;
        btn.textContent = '🔄';
      }
    }
  }

  /**
   * Remove source
   */
  async _removeSource(sourceId) {
    if (!confirm('Remove this source?')) return;

    try {
      await this.api.callTool('brain_ecosystem_monitor', { action: 'untrack', sourceId });
      await this.refresh();
    } catch (err) {
      console.error('Remove source failed:', err);
    }
  }

  /**
   * Show add source dialog
   */
  _showAddSourceDialog() {
    const dialog = Utils.createElement('div', { className: 'modal-overlay' });
    dialog.innerHTML = `
      <div class="modal">
        <div class="modal-header">
          <h3>Add GitHub Source</h3>
          <button class="modal-close" id="close-dialog">✕</button>
        </div>
        <div class="modal-body">
          <div class="form-group">
            <label>Owner</label>
            <input type="text" id="source-owner" placeholder="e.g., solana-labs" />
          </div>
          <div class="form-group">
            <label>Repository</label>
            <input type="text" id="source-repo" placeholder="e.g., solana" />
          </div>
          <div class="form-group">
            <label>Branch (optional)</label>
            <input type="text" id="source-branch" placeholder="main" />
          </div>
          <div class="form-group">
            <label>
              <input type="checkbox" id="track-commits" checked /> Track commits
            </label>
          </div>
          <div class="form-group">
            <label>
              <input type="checkbox" id="track-releases" checked /> Track releases
            </label>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn" id="cancel-add">Cancel</button>
          <button class="btn btn-primary" id="confirm-add">Add Source</button>
        </div>
      </div>
    `;

    document.body.appendChild(dialog);

    // Bind dialog events
    document.getElementById('close-dialog')?.addEventListener('click', () => {
      dialog.remove();
    });
    document.getElementById('cancel-add')?.addEventListener('click', () => {
      dialog.remove();
    });
    document.getElementById('confirm-add')?.addEventListener('click', async () => {
      const owner = document.getElementById('source-owner')?.value?.trim();
      const repo = document.getElementById('source-repo')?.value?.trim();
      const branch = document.getElementById('source-branch')?.value?.trim();
      const trackCommits = document.getElementById('track-commits')?.checked;
      const trackReleases = document.getElementById('track-releases')?.checked;

      if (!owner || !repo) {
        alert('Owner and repository are required');
        return;
      }

      try {
        await this.api.callTool('brain_ecosystem_monitor', {
          action: 'track',
          owner,
          repo,
          ...(branch && { branch }),
          trackCommits,
          trackReleases,
        });
        dialog.remove();
        await this.refresh();
      } catch (err) {
        console.error('Add source failed:', err);
        alert('Failed to add source');
      }
    });

    // Close on overlay click
    dialog.addEventListener('click', (e) => {
      if (e.target === dialog) dialog.remove();
    });
  }

  /**
   * Format relative time
   */
  _formatRelativeTime(timestamp) {
    const now = Date.now();
    const diff = now - timestamp;

    const seconds = Math.floor(diff / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);

    if (days > 0) return `${days}d ago`;
    if (hours > 0) return `${hours}h ago`;
    if (minutes > 0) return `${minutes}m ago`;
    return 'Just now';
  }

  /**
   * Escape HTML
   */
  _escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }

  /**
   * Cleanup on unmount
   */
  unmount() {
    if (this.refreshInterval) {
      clearInterval(this.refreshInterval);
      this.refreshInterval = null;
    }
  }
}
