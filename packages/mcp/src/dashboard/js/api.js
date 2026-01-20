/**
 * CYNIC Dashboard - API Client
 * API + SSE connection to MCP server
 */

export class API {
  constructor(baseUrl = '') {
    this.baseUrl = baseUrl || window.location.origin;
    this.connected = false;
    this.sse = null;
    this.listeners = new Map();
    this.reconnectAttempts = 0;
    this.maxReconnectAttempts = 5;
    this.reconnectDelay = 1000;
  }

  /**
   * Initialize API and connect SSE
   */
  async init() {
    // Check health first
    const health = await this.getHealth();
    if (health) {
      this.connected = true;
      this.connectSSE();
      return true;
    }
    return false;
  }

  /**
   * Connect to SSE endpoint
   */
  connectSSE() {
    if (this.sse) {
      this.sse.close();
    }

    this.sse = new EventSource(`${this.baseUrl}/sse`);

    this.sse.onopen = () => {
      console.log('SSE connected');
      this.connected = true;
      this.reconnectAttempts = 0;
      this._emit('connection', { status: 'connected' });
    };

    this.sse.onerror = (err) => {
      console.error('SSE error:', err);
      this.connected = false;
      this._emit('connection', { status: 'disconnected' });

      // Attempt reconnection
      if (this.reconnectAttempts < this.maxReconnectAttempts) {
        this.reconnectAttempts++;
        const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);
        console.log(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);
        setTimeout(() => this.connectSSE(), delay);
      }
    };

    this.sse.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        this._emit('message', data);
      } catch {
        this._emit('message', { raw: event.data });
      }
    };

    // Listen for specific event types
    this.sse.addEventListener('endpoint', (event) => {
      console.log('SSE endpoint:', event.data);
    });

    this.sse.addEventListener('judgment', (event) => {
      const data = JSON.parse(event.data);
      this._emit('judgment', data);
    });

    this.sse.addEventListener('alert', (event) => {
      const data = JSON.parse(event.data);
      this._emit('alert', data);
    });

    this.sse.addEventListener('block', (event) => {
      const data = JSON.parse(event.data);
      this._emit('block', data);
    });

    // Tool execution events (for Live View timeline)
    this.sse.addEventListener('tool_pre', (event) => {
      const data = JSON.parse(event.data);
      this._emit('tool_pre', data);
    });

    this.sse.addEventListener('tool_post', (event) => {
      const data = JSON.parse(event.data);
      this._emit('tool_post', data);
    });

    // Dog events (Collective agent activity)
    this.sse.addEventListener('dogStatus', (event) => {
      const data = JSON.parse(event.data);
      this._emit('dogStatus', data);
    });

    this.sse.addEventListener('dogDecision', (event) => {
      const data = JSON.parse(event.data);
      this._emit('dogDecision', data);
    });

    this.sse.addEventListener('dogWarning', (event) => {
      const data = JSON.parse(event.data);
      this._emit('dogWarning', data);
    });

    // Hook events
    this.sse.addEventListener('hook', (event) => {
      const data = JSON.parse(event.data);
      this._emit('hook', data);
    });
  }

  /**
   * Disconnect SSE
   */
  disconnect() {
    if (this.sse) {
      this.sse.close();
      this.sse = null;
    }
    this.connected = false;
  }

  /**
   * Event listener management
   */
  on(event, callback) {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event).add(callback);
    return () => this.listeners.get(event)?.delete(callback);
  }

  _emit(event, data) {
    this.listeners.get(event)?.forEach(cb => {
      try {
        cb(data);
      } catch (err) {
        console.error(`Error in ${event} listener:`, err);
      }
    });
  }

  /**
   * GET health status
   */
  async getHealth() {
    try {
      const res = await fetch(`${this.baseUrl}/health`);
      if (!res.ok) return null;
      return await res.json();
    } catch (err) {
      console.error('Health check failed:', err);
      return null;
    }
  }

  /**
   * GET metrics (Prometheus format)
   */
  async getMetrics() {
    try {
      const res = await fetch(`${this.baseUrl}/metrics`);
      if (!res.ok) return null;
      return await res.text();
    } catch (err) {
      console.error('Metrics fetch failed:', err);
      return null;
    }
  }

  /**
   * List available tools
   */
  async listTools() {
    try {
      const res = await fetch(`${this.baseUrl}/api/tools`);
      if (!res.ok) return [];
      const data = await res.json();
      return data.tools || [];
    } catch (err) {
      console.error('List tools failed:', err);
      return [];
    }
  }

  /**
   * Call a tool via REST API
   */
  async callTool(toolName, args = {}) {
    try {
      const res = await fetch(`${this.baseUrl}/api/tools/${toolName}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(args),
      });

      const data = await res.json();

      if (!res.ok) {
        return { success: false, error: data.error || 'Unknown error' };
      }

      return {
        success: true,
        result: data.result,
        duration: data.duration,
      };
    } catch (err) {
      console.error(`Tool ${toolName} call failed:`, err);
      return { success: false, error: err.message };
    }
  }

  // ═══════════════════════════════════════════════════════════
  // TOOL HELPERS
  // ═══════════════════════════════════════════════════════════

  /**
   * Get system health
   */
  async health(verbose = true) {
    return this.callTool('brain_health', { verbose });
  }

  /**
   * Judge an item
   */
  async judge(item, context = {}) {
    return this.callTool('brain_cynic_judge', { item, context });
  }

  /**
   * Digest content
   */
  async digest(content, source, type = 'document') {
    return this.callTool('brain_cynic_digest', { content, source, type });
  }

  /**
   * Search knowledge
   */
  async search(query, type = 'all', limit = 10) {
    return this.callTool('brain_search', { query, type, limit });
  }

  /**
   * Get patterns
   */
  async patterns(category = 'all', limit = 10) {
    return this.callTool('brain_patterns', { category, limit });
  }

  /**
   * Get collective status
   */
  async collectiveStatus(verbose = false) {
    return this.callTool('brain_collective_status', { verbose });
  }

  /**
   * Get agents status (legacy)
   */
  async agentsStatus(verbose = false) {
    return this.callTool('brain_agents_status', { verbose });
  }

  /**
   * PoJ Chain operations
   */
  async chain(action = 'status', options = {}) {
    return this.callTool('brain_poj_chain', { action, ...options });
  }

  /**
   * Get codebase tree
   */
  async codebase(action = 'tree') {
    return this.callTool('brain_codebase', { action });
  }

  /**
   * Search codebase
   */
  async searchCodebase(query) {
    return this.callTool('brain_codebase', { action: 'search', query });
  }

  /**
   * Get ecosystem docs
   */
  async ecosystem(action = 'list', options = {}) {
    return this.callTool('brain_ecosystem', { action, ...options });
  }

  /**
   * Get metrics via tool
   */
  async metricsData(action = 'collect') {
    return this.callTool('brain_metrics', { action });
  }

  /**
   * Meta dashboard analysis
   */
  async meta(action = 'analyze', verbose = false) {
    return this.callTool('brain_meta', { action, verbose });
  }

  /**
   * Provide feedback on judgment
   */
  async feedback(judgmentId, outcome, reason, actualScore) {
    return this.callTool('brain_cynic_feedback', {
      judgmentId,
      outcome,
      reason,
      actualScore,
    });
  }

  /**
   * Start session
   */
  async sessionStart(userId, project, metadata = {}) {
    return this.callTool('brain_session_start', { userId, project, metadata });
  }

  /**
   * End session
   */
  async sessionEnd(sessionId) {
    return this.callTool('brain_session_end', { sessionId });
  }

  /**
   * Query documentation (Context7)
   */
  async queryDocs(libraryId, query) {
    return this.callTool('brain_docs', { action: 'query', libraryId, query });
  }

  /**
   * Trace judgment integrity (judgment → PoJ block → merkle → Solana anchor)
   */
  async trace(judgmentId, includeRaw = false) {
    return this.callTool('brain_trace', { judgmentId, includeRaw });
  }
}

// Create singleton instance
export const api = new API();

// Export to window
window.CYNICAPI = api;
