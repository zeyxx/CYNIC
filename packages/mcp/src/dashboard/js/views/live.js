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
import { getStationInfo, getToolIcon, getToolColor } from '../lib/station-map.js';
import { cynicAudio } from '../lib/audio.js';

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
      types: ['judgment', 'pattern', 'digest', 'event', 'block', 'tool'],
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
      tools: 0,
    };
    // Vibecraft pattern: Track pending tools for duration calculation
    this.pendingTools = new Map();
    // Vibecraft pattern: Tool timeline (recent tools as icon strip)
    this.recentTools = [];
    this.maxTimelineTools = 15;
    // Session management
    this.session = null;
    this.sessionLoading = false;
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
            <span class="thinking-indicator ${this.pendingTools.size > 0 ? 'active' : ''}">
              <span class="dot"></span>
              <span class="dot"></span>
              <span class="dot"></span>
            </span>
          </div>
          <div class="live-stats">
            <span class="stat" title="Total received">📥 ${this.stats.totalReceived}</span>
            <span class="stat" title="Tools executed">🔧 ${this.stats.tools}</span>
            <span class="stat" title="Judgments">⚖️ ${this.stats.judgments}</span>
            <span class="stat" title="Patterns">🔮 ${this.stats.patterns}</span>
            <span class="stat" title="Blocks">⛓️ ${this.stats.blocks}</span>
          </div>
          <div class="tool-timeline" title="Recent tool activity">
            ${this._renderToolTimeline()}
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
            <button class="audio-btn" title="Toggle audio">🔇</button>
            <button class="clear-btn" title="Clear observations">🗑️</button>
          </div>
          <!-- Prompt Injection -->
          <div class="prompt-inject">
            <input type="text" class="prompt-input" placeholder="Enter prompt to judge or digest..." maxlength="500">
            <div class="prompt-actions">
              <button class="prompt-btn judge-btn" title="Judge this prompt">⚖️ Judge</button>
              <button class="prompt-btn digest-btn" title="Digest this prompt">🧠 Digest</button>
            </div>
          </div>
          <!-- Session Management -->
          <div class="session-zone">
            ${this._renderSessionUI()}
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

    // Debug: Log connection attempt with full URL
    const sseUrl = new URL('/sse', window.location.origin).href;
    console.log('🔴 [SSE] Attempting connection to:', sseUrl);
    console.log('🔴 [SSE] Current location:', window.location.href);

    try {
      this.eventSource = new EventSource('/sse');
      console.log('🔴 [SSE] EventSource created, readyState:', this.eventSource.readyState);

      this.eventSource.onopen = () => {
        this.isConnected = true;
        this._updateStatus();
        console.log('🔴 [SSE] ✅ CONNECTED - readyState:', this.eventSource.readyState);
      };

      this.eventSource.onerror = (err) => {
        this.isConnected = false;
        this._updateStatus();
        console.error('🔴 [SSE] ❌ ERROR - readyState:', this.eventSource?.readyState);
        console.error('🔴 [SSE] Error details:', err);
      };

      // Listen for different event types
      this.eventSource.addEventListener('judgment', (e) => {
        console.log('🔴 [SSE] Event: judgment', e.data?.slice(0, 100));
        this._handleEvent('judgment', e);
      });
      this.eventSource.addEventListener('block', (e) => {
        console.log('🔴 [SSE] Event: block', e.data?.slice(0, 100));
        this._handleEvent('block', e);
      });
      this.eventSource.addEventListener('pattern', (e) => {
        console.log('🔴 [SSE] Event: pattern', e.data?.slice(0, 100));
        this._handleEvent('pattern', e);
      });
      this.eventSource.addEventListener('message', (e) => {
        console.log('🔴 [SSE] Event: message', e.data?.slice(0, 100));
        this._handleEvent('event', e);
      });
      // Tool execution events (Vibecraft pattern - duration tracking)
      this.eventSource.addEventListener('tool_pre', (e) => {
        console.log('🔴 [SSE] Event: tool_pre', e.data?.slice(0, 100));
        this._handleToolEvent('tool_pre', e);
      });
      this.eventSource.addEventListener('tool_post', (e) => {
        console.log('🔴 [SSE] Event: tool_post', e.data?.slice(0, 100));
        this._handleToolEvent('tool_post', e);
      });
      // Also listen for endpoint event (sent on connect)
      this.eventSource.addEventListener('endpoint', (e) => {
        console.log('🔴 [SSE] Event: endpoint (connect confirmation)', e.data);
      });

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
      if (type === 'judgment') {
        this.stats.judgments++;
        cynicAudio.playJudgment(data.verdict || 'WAG');
      }
      if (type === 'pattern') {
        this.stats.patterns++;
        cynicAudio.playPattern();
      }
      if (type === 'block') {
        this.stats.blocks++;
        cynicAudio.playBlock(data.slot || 0);
      }
      if (type === 'event') this.stats.events++;

      this._renderObservationsList();
      this._updateStats();

    } catch (err) {
      console.error('🔴 Live: Failed to parse event', err);
    }
  }

  /**
   * Handle tool execution events (Vibecraft pattern)
   * Tracks pre/post for duration calculation
   */
  _handleToolEvent(type, event) {
    try {
      const data = JSON.parse(event.data);
      const { toolUseId, tool, duration, dogsNotified } = data;

      if (type === 'tool_pre') {
        // Store pending tool for duration matching
        this.pendingTools.set(toolUseId, {
          tool,
          startTime: data.timestamp || Date.now(),
          input: data.input,
        });

        // Play tool start sound (Vibecraft audio pattern)
        const stationInfo = getStationInfo(tool);
        cynicAudio.playToolStart(stationInfo.category);

        // Add to tool timeline (Vibecraft icon strip)
        this.recentTools.unshift({
          name: tool,
          toolUseId,
          status: 'running',
          timestamp: Date.now(),
        });
        // Trim timeline
        if (this.recentTools.length > this.maxTimelineTools) {
          this.recentTools = this.recentTools.slice(0, this.maxTimelineTools);
        }

        // Create observation for tool start
        const observation = {
          id: `obs_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
          type: 'tool',
          subtype: 'start',
          timestamp: Date.now(),
          data: {
            tool,
            toolUseId,
            dogsNotified,
            status: 'running',
          },
        };
        this.observations.unshift(observation);

      } else if (type === 'tool_post') {
        // Match with pending tool
        const pending = this.pendingTools.get(toolUseId);
        const calculatedDuration = pending
          ? (data.timestamp || Date.now()) - pending.startTime
          : duration;

        // Remove pending
        this.pendingTools.delete(toolUseId);

        // Play tool complete sound (Vibecraft audio pattern)
        const stationInfo = getStationInfo(tool);
        cynicAudio.playToolComplete(stationInfo.category, data.success !== false);

        // Update tool in timeline (mark as complete)
        const timelineTool = this.recentTools.find(t => t.toolUseId === toolUseId);
        if (timelineTool) {
          timelineTool.status = 'complete';
          timelineTool.duration = calculatedDuration;
        }

        // Create observation for tool completion
        const observation = {
          id: `obs_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
          type: 'tool',
          subtype: 'complete',
          timestamp: Date.now(),
          data: {
            tool,
            toolUseId,
            duration: calculatedDuration,
            dogsNotified,
            success: data.success,
            status: 'complete',
          },
        };
        this.observations.unshift(observation);
        this.stats.tools++;
      }

      // Trim observations
      if (this.observations.length > this.maxObservations) {
        this.observations = this.observations.slice(0, this.maxObservations);
      }

      this.stats.totalReceived++;
      this._renderObservationsList();
      this._updateStats();
      this._updateToolTimeline();
      this._updateThinkingIndicator();

    } catch (err) {
      console.error('🔴 Live: Failed to parse tool event', err);
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
    const judgmentId = escapeHtml(data.id || data.judgmentId || '');
    return `
      <div class="judgment-badge verdict-${verdict.toLowerCase()}">
        <span class="verdict">${verdict}</span>
        <span class="score">Q: ${score.toFixed(1)}</span>
        ${judgmentId ? `<button class="trace-btn" data-judgment-id="${judgmentId}" title="Trace integrity chain">🔗</button>` : ''}
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
      case 'tool':
        return this._renderToolDetail(obs.data);
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
   * Render trace detail (integrity chain visualization)
   */
  _renderTraceDetail(trace) {
    if (!trace) {
      return '<div class="trace-loading">Loading trace...</div>';
    }

    // Handle not found case
    if (trace.found === false) {
      return `
        <div class="trace-detail">
          <div class="trace-header verdict-unknown">
            <span class="trace-title">Integrity Trace</span>
            <span class="trace-score">0%</span>
            <span class="trace-verdict">NOT FOUND</span>
          </div>
          <div class="trace-error">
            <div class="error-icon">🔍</div>
            <div class="error-message">${escapeHtml(trace.error || 'Judgment not found')}</div>
            ${trace.hint ? `<div class="error-hint">${escapeHtml(trace.hint)}</div>` : ''}
          </div>
          <div class="trace-footer">
            <span class="trace-note">φ distrusts φ - verify independently</span>
          </div>
        </div>
      `;
    }

    // Extract layers from trace response
    const layers = trace.layers || {};
    const judgment = layers.judgment;
    const pojBlock = layers.pojBlock;
    const merkleProof = layers.merkleProof;
    const solanaAnchor = layers.solanaAnchor;
    const integrityScore = trace.integrityScore ?? 0;
    const verdict = judgment?.verdict || 'UNKNOWN';
    const verdictClass = verdict.toLowerCase();

    // Build chain visualization
    const chainLayers = [];

    // Layer 1: Judgment
    if (judgment) {
      const jdgStatus = judgment.status === 'verified' ? 'found' : 'missing';
      chainLayers.push(`
        <div class="trace-layer judgment">
          <div class="layer-icon">⚖️</div>
          <div class="layer-content">
            <div class="layer-title">Judgment</div>
            <div class="layer-id">${escapeHtml(judgment.id || 'N/A')}</div>
            ${judgment.verdict ? `<div class="layer-verdict verdict-${judgment.verdict.toLowerCase()}">${escapeHtml(judgment.verdict)}</div>` : ''}
            ${judgment.qScore !== undefined ? `<div class="layer-score">Q: ${judgment.qScore.toFixed(1)}</div>` : ''}
          </div>
          <div class="layer-status ${jdgStatus}">${jdgStatus === 'found' ? '✓' : '✗'}</div>
        </div>
      `);
    }

    // Layer 2: PoJ Block
    if (pojBlock) {
      const pojStatus = pojBlock.status === 'verified' ? 'found' : pojBlock.status === 'pending' ? 'pending' : 'missing';
      const pojIcon = pojStatus === 'found' ? '✓' : pojStatus === 'pending' ? '⏳' : '✗';
      chainLayers.push(`
        <div class="trace-layer poj-block">
          <div class="layer-icon">⛓️</div>
          <div class="layer-content">
            <div class="layer-title">PoJ Block</div>
            <div class="layer-id">${pojBlock.slot !== undefined ? `Slot #${pojBlock.slot}` : (pojBlock.message || 'N/A')}</div>
            ${pojBlock.judgmentCount ? `<div class="layer-info">${pojBlock.judgmentCount} judgments</div>` : ''}
          </div>
          <div class="layer-status ${pojStatus}">${pojIcon}</div>
        </div>
      `);
    }

    // Layer 3: Merkle Proof
    if (merkleProof) {
      const merkleStatus = merkleProof.status === 'verified' ? 'found' : merkleProof.status === 'partial' ? 'pending' : 'missing';
      const merkleIcon = merkleStatus === 'found' ? '✓' : merkleStatus === 'pending' ? '⚠' : '✗';
      chainLayers.push(`
        <div class="trace-layer merkle">
          <div class="layer-icon">🌳</div>
          <div class="layer-content">
            <div class="layer-title">Merkle Proof</div>
            <div class="layer-info">${merkleProof.chainValid ? 'Chain valid' : (merkleProof.message || `${merkleProof.blocksChecked || 0} blocks checked`)}</div>
            ${merkleProof.status === 'verified' ? '<div class="layer-verified">Verified</div>' : ''}
          </div>
          <div class="layer-status ${merkleStatus}">${merkleIcon}</div>
        </div>
      `);
    }

    // Layer 4: Solana Anchor
    if (solanaAnchor) {
      const solStatus = solanaAnchor.status === 'anchored' ? 'found' : solanaAnchor.status === 'enabled' ? 'pending' : 'missing';
      const solIcon = solStatus === 'found' ? '✓' : solStatus === 'pending' ? '⏳' : '✗';
      chainLayers.push(`
        <div class="trace-layer solana">
          <div class="layer-icon">◎</div>
          <div class="layer-content">
            <div class="layer-title">Solana Anchor</div>
            ${solanaAnchor.txSignature ? `
              <div class="layer-id">${escapeHtml(String(solanaAnchor.txSignature).slice(0, 16))}...</div>
              ${solanaAnchor.slot ? `<div class="layer-info">Slot ${solanaAnchor.slot}</div>` : ''}
            ` : `<div class="layer-info">${solanaAnchor.anchoringActive ? 'Anchoring enabled' : 'Not anchored yet'}</div>`}
          </div>
          <div class="layer-status ${solStatus}">${solIcon}</div>
        </div>
      `);
    }

    return `
      <div class="trace-detail">
        <div class="trace-header verdict-${verdictClass}">
          <span class="trace-title">Integrity Trace</span>
          <span class="trace-score">${integrityScore}%</span>
          <span class="trace-verdict">${escapeHtml(verdict)}</span>
        </div>
        <div class="trace-chain">
          ${chainLayers.join('<div class="trace-connector">↓</div>')}
        </div>
        <div class="trace-footer">
          <span class="trace-note">φ distrusts φ - verify independently</span>
        </div>
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
   * Render tool detail (Vibecraft-inspired)
   */
  _renderToolDetail(data) {
    const stationInfo = getStationInfo(data.tool);
    const toolName = escapeHtml(data.tool?.replace('brain_', '') || 'unknown');
    const isRunning = data.status === 'running';

    return `
      <div class="tool-detail">
        <div class="tool-header" style="border-color: ${stationInfo.color}">
          <span class="tool-icon" style="font-size: 2rem">${stationInfo.icon}</span>
          <div class="tool-info">
            <div class="tool-name">${toolName}</div>
            <div class="tool-station" style="color: ${stationInfo.color}">${escapeHtml(stationInfo.station)}</div>
          </div>
          <div class="tool-status ${isRunning ? 'running' : 'complete'}">
            ${isRunning ? '⏳ Running' : '✓ Complete'}
          </div>
        </div>
        ${data.duration ? `
          <div class="tool-metric">
            <span class="metric-label">Duration</span>
            <span class="metric-value">${data.duration}ms</span>
          </div>
        ` : ''}
        ${data.dogsNotified ? `
          <div class="tool-metric">
            <span class="metric-label">Dogs Notified</span>
            <span class="metric-value">${data.dogsNotified} 🐕</span>
          </div>
        ` : ''}
        ${data.toolUseId ? `
          <div class="tool-metric">
            <span class="metric-label">Tool Use ID</span>
            <span class="metric-value mono">${escapeHtml(data.toolUseId)}</span>
          </div>
        ` : ''}
      </div>
    `;
  }

  /**
   * Render tool timeline (Vibecraft-inspired icon strip)
   */
  _renderToolTimeline() {
    if (this.recentTools.length === 0) {
      return '<div class="timeline-empty">No tools yet</div>';
    }

    return this.recentTools.map((tool, index) => {
      const stationInfo = getStationInfo(tool.name);
      const isRecent = index < 3;
      const isRunning = tool.status === 'running';

      return `
        <div class="timeline-tool ${isRecent ? 'recent' : ''} ${isRunning ? 'running' : ''}"
             style="--tool-color: ${stationInfo.color}"
             title="${escapeHtml(tool.name)} ${tool.duration ? `(${tool.duration}ms)` : ''}">
          <span class="timeline-icon">${stationInfo.icon}</span>
          ${isRunning ? '<span class="timeline-spinner"></span>' : ''}
        </div>
      `;
    }).join('');
  }

  /**
   * Update tool timeline
   */
  _updateToolTimeline() {
    const timelineEl = this.container?.querySelector('.tool-timeline');
    if (timelineEl) {
      timelineEl.innerHTML = this._renderToolTimeline();
    }
  }

  /**
   * Update thinking indicator (Vibecraft pattern - animated dots)
   */
  _updateThinkingIndicator() {
    const indicator = this.container?.querySelector('.thinking-indicator');
    if (indicator) {
      indicator.classList.toggle('active', this.pendingTools.size > 0);
    }
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
      case 'tool': {
        const toolIcon = getToolIcon(obs.data.tool);
        const status = obs.data.status === 'running' ? '⏳' : '✓';
        const duration = obs.data.duration ? ` (${obs.data.duration}ms)` : '';
        const dogs = obs.data.dogsNotified ? ` → ${obs.data.dogsNotified} 🐕` : '';
        const toolName = obs.data.tool?.replace('brain_', '') || 'unknown';
        return `${status} ${toolIcon} ${toolName}${duration}${dogs}`;
      }
      default:
        return JSON.stringify(obs.data).slice(0, 100);
    }
  }

  /**
   * Get icon for type
   */
  _getTypeIcon(type) {
    const icons = {
      judgment: '⚖️',
      pattern: '🔮',
      block: '⛓️',
      digest: '📖',
      event: '📡',
      tool: '🔧',
    };
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
        <span class="stat" title="Total received">📥 ${this.stats.totalReceived}</span>
        <span class="stat" title="Tools executed">🔧 ${this.stats.tools}</span>
        <span class="stat" title="Judgments">⚖️ ${this.stats.judgments}</span>
        <span class="stat" title="Patterns">🔮 ${this.stats.patterns}</span>
        <span class="stat" title="Blocks">⛓️ ${this.stats.blocks}</span>
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

    // Audio toggle button (Vibecraft pattern)
    const audioBtn = this.container?.querySelector('.audio-btn');
    if (audioBtn) {
      console.log('🔊 [Audio] Button found, attaching listener');
      audioBtn.addEventListener('click', () => {
        console.log('🔊 [Audio] Button clicked, current state:', cynicAudio.getStatus());
        const enabled = cynicAudio.toggle();
        console.log('🔊 [Audio] Toggled, now enabled:', enabled);
        audioBtn.textContent = enabled ? '🔊' : '🔇';
        audioBtn.classList.toggle('active', enabled);
        if (enabled) {
          console.log('🔊 [Audio] Playing connect sound...');
          cynicAudio.playConnect();
        }
      });
    } else {
      console.warn('🔊 [Audio] Button NOT found!');
    }

    // Prompt injection handlers
    const promptInput = this.container?.querySelector('.prompt-input');
    const judgeBtn = this.container?.querySelector('.judge-btn');
    const digestBtn = this.container?.querySelector('.digest-btn');

    if (judgeBtn && promptInput) {
      judgeBtn.addEventListener('click', () => this._handlePromptAction('judge', promptInput));
    }
    if (digestBtn && promptInput) {
      digestBtn.addEventListener('click', () => this._handlePromptAction('digest', promptInput));
    }
    // Enter key submits as judge
    if (promptInput) {
      promptInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
          this._handlePromptAction('judge', promptInput);
        }
      });
    }

    // Session management listeners
    this._attachSessionEventListeners();

    this._attachListEventListeners();
  }

  /**
   * Handle prompt injection action
   * @param {string} action - 'judge' or 'digest'
   * @param {HTMLInputElement} input - Input element
   */
  async _handlePromptAction(action, input) {
    const prompt = input.value.trim();
    if (!prompt) {
      console.warn('🔴 [Prompt] Empty prompt, ignoring');
      return;
    }

    console.log(`🔴 [Prompt] ${action}: "${prompt.slice(0, 50)}..."`);

    // Disable buttons while processing
    const btns = this.container?.querySelectorAll('.prompt-btn');
    btns?.forEach(btn => btn.disabled = true);
    input.disabled = true;

    try {
      const endpoint = action === 'judge' ? '/api/tools/brain_cynic_judge' : '/api/tools/brain_cynic_digest';
      const body = action === 'judge'
        ? { item: { type: 'user_prompt', content: prompt } }
        : { content: prompt, type: 'conversation' };

      const response = await fetch(endpoint, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const result = await response.json();
      console.log(`🔴 [Prompt] ${action} result:`, result);

      // Clear input on success
      input.value = '';

      // Play audio feedback
      if (action === 'judge' && result.verdict) {
        cynicAudio.playJudgment(result.verdict);
      } else {
        cynicAudio.playToolComplete('analysis', true);
      }

    } catch (err) {
      console.error(`🔴 [Prompt] ${action} error:`, err);
      cynicAudio.playToolComplete('system', false);
    } finally {
      // Re-enable buttons
      btns?.forEach(btn => btn.disabled = false);
      input.disabled = false;
      input.focus();
    }
  }

  /**
   * Render session management UI
   */
  _renderSessionUI() {
    if (this.sessionLoading) {
      return `
        <div class="session-loading">
          <span class="session-spinner"></span>
          <span>Loading session...</span>
        </div>
      `;
    }

    if (this.session) {
      return `
        <div class="session-active">
          <div class="session-info">
            <span class="session-indicator"></span>
            <div class="session-details">
              <span class="session-user">🐕 ${escapeHtml(this.session.userId)}</span>
              ${this.session.project ? `<span class="session-project">📁 ${escapeHtml(this.session.project)}</span>` : ''}
              <span class="session-id">ID: ${escapeHtml(this.session.sessionId?.slice(0, 12) || '...')}...</span>
            </div>
          </div>
          <button class="session-btn end-btn" title="End session">🛑 End</button>
        </div>
      `;
    }

    return `
      <div class="session-start">
        <input type="text" class="session-user-input" placeholder="User ID (wallet, email...)" maxlength="100">
        <input type="text" class="session-project-input" placeholder="Project (optional)" maxlength="50">
        <button class="session-btn start-btn" title="Start new session">🚀 Start Session</button>
      </div>
    `;
  }

  /**
   * Handle session start
   */
  async _handleSessionStart() {
    const userInput = this.container?.querySelector('.session-user-input');
    const projectInput = this.container?.querySelector('.session-project-input');

    const userId = userInput?.value.trim();
    if (!userId) {
      console.warn('🔴 [Session] No userId provided');
      userInput?.focus();
      return;
    }

    const project = projectInput?.value.trim() || undefined;

    console.log(`🔴 [Session] Starting for user: ${userId}, project: ${project || 'none'}`);
    this.sessionLoading = true;
    this._updateSessionUI();

    try {
      const response = await fetch('/api/tools/brain_session_start', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ userId, project }),
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const result = await response.json();
      console.log('🔴 [Session] Started:', result);

      this.session = {
        sessionId: result.sessionId,
        userId,
        project,
        startTime: Date.now(),
      };

      cynicAudio.playConnect();

    } catch (err) {
      console.error('🔴 [Session] Start error:', err);
      cynicAudio.playToolComplete('system', false);
    } finally {
      this.sessionLoading = false;
      this._updateSessionUI();
    }
  }

  /**
   * Handle session end
   */
  async _handleSessionEnd() {
    if (!this.session) return;

    console.log(`🔴 [Session] Ending: ${this.session.sessionId}`);
    this.sessionLoading = true;
    this._updateSessionUI();

    try {
      const response = await fetch('/api/tools/brain_session_end', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ sessionId: this.session.sessionId }),
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const result = await response.json();
      console.log('🔴 [Session] Ended:', result);

      this.session = null;
      cynicAudio.playDisconnect();

    } catch (err) {
      console.error('🔴 [Session] End error:', err);
      cynicAudio.playToolComplete('system', false);
    } finally {
      this.sessionLoading = false;
      this._updateSessionUI();
    }
  }

  /**
   * Update session UI without full re-render
   */
  _updateSessionUI() {
    const sessionZone = this.container?.querySelector('.session-zone');
    if (sessionZone) {
      sessionZone.innerHTML = this._renderSessionUI();
      this._attachSessionEventListeners();
    }
  }

  /**
   * Attach session event listeners
   */
  _attachSessionEventListeners() {
    const startBtn = this.container?.querySelector('.session-btn.start-btn');
    const endBtn = this.container?.querySelector('.session-btn.end-btn');
    const userInput = this.container?.querySelector('.session-user-input');

    if (startBtn) {
      startBtn.addEventListener('click', () => this._handleSessionStart());
    }
    if (endBtn) {
      endBtn.addEventListener('click', () => this._handleSessionEnd());
    }
    if (userInput) {
      userInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
          this._handleSessionStart();
        }
      });
    }
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

    // Trace button handlers (all content is server-sourced and escaped)
    this.container?.querySelectorAll('.trace-btn').forEach(btn => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const judgmentId = btn.dataset.judgmentId;
        if (judgmentId) {
          this._handleTraceRequest(judgmentId);
        }
      });
    });
  }

  /**
   * Handle trace request
   * Note: All content rendered via innerHTML is escaped via escapeHtml()
   */
  async _handleTraceRequest(judgmentId) {
    console.log(`[Trace] Requesting trace for: ${judgmentId}`);

    // Show loading in detail panel
    const detailEl = this.container?.querySelector('.observation-detail');
    if (detailEl) {
      // Using innerHTML with escaped content as per existing codebase pattern
      detailEl.innerHTML = this._renderTraceDetail(null);
    }

    try {
      const response = await fetch(`/api/tools/brain_trace`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ judgmentId }),
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const result = await response.json();
      console.log('[Trace] Result:', result);

      // Extract the actual trace data from the API response wrapper
      const traceData = result.result || result;
      console.log('[Trace] Data:', traceData);

      // Update detail panel with trace (all values escaped in _renderTraceDetail)
      if (detailEl) {
        detailEl.innerHTML = `
          <div class="detail-content">
            <div class="detail-header">
              <span class="type-badge trace">trace</span>
              <span class="timestamp">${escapeHtml(new Date().toLocaleString())}</span>
            </div>
            <div class="detail-body">
              ${this._renderTraceDetail(traceData)}
            </div>
            <div class="detail-actions">
              <button class="action-btn copy" data-action="copy-trace" title="Copy trace JSON">Copy</button>
            </div>
          </div>
        `;

        // Attach copy handler
        const copyBtn = detailEl.querySelector('.action-btn[data-action="copy-trace"]');
        if (copyBtn) {
          copyBtn.addEventListener('click', () => {
            navigator.clipboard.writeText(JSON.stringify(traceData, null, 2));
            cynicAudio.playToolComplete('analysis', true);
          });
        }
      }

      // Play success sound
      cynicAudio.playToolComplete('analysis', true);

    } catch (err) {
      console.error('[Trace] Error:', err);
      if (detailEl) {
        detailEl.innerHTML = `
          <div class="detail-content">
            <div class="detail-header">
              <span class="type-badge error">error</span>
            </div>
            <div class="detail-body">
              <div class="error-message">Failed to trace judgment: ${escapeHtml(err.message)}</div>
            </div>
          </div>
        `;
      }
      cynicAudio.playToolComplete('system', false);
    }
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
