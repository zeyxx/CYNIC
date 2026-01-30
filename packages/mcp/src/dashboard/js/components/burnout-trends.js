/**
 * CYNIC Dashboard - Burnout Trends Component
 *
 * v1.1: Visualizes burnout detection trends and warnings
 *
 * @module @cynic/dashboard/components/burnout-trends
 */

const PHI_INV = 0.618;
const PHI_INV_2 = 0.382;

/**
 * Burnout Trends Component
 */
export class BurnoutTrends {
  constructor(options = {}) {
    this.api = options.api;
    this.onWarning = options.onWarning || (() => {});
    this.container = null;
    this.data = {
      current: null,
      trends: [],
      warnings: [],
      episodes: [],
    };
    this._refreshInterval = null;
  }

  /**
   * Render component
   */
  render(container) {
    this.container = container;
    this.container.innerHTML = this._template();
    this._bindEvents();
    this.refresh();
    this._startAutoRefresh();
  }

  /**
   * Generate template
   */
  _template() {
    return `
      <div class="burnout-trends">
        <div class="burnout-header">
          <h3>🔥 Burnout Monitor</h3>
          <span class="burnout-status" id="burnout-status">Loading...</span>
        </div>

        <div class="burnout-gauges">
          <div class="gauge-row">
            <div class="gauge" id="energy-gauge">
              <div class="gauge-label">Energy</div>
              <div class="gauge-bar">
                <div class="gauge-fill" id="energy-fill"></div>
              </div>
              <div class="gauge-value" id="energy-value">--</div>
            </div>
            <div class="gauge" id="frustration-gauge">
              <div class="gauge-label">Frustration</div>
              <div class="gauge-bar">
                <div class="gauge-fill" id="frustration-fill"></div>
              </div>
              <div class="gauge-value" id="frustration-value">--</div>
            </div>
          </div>
          <div class="gauge-row">
            <div class="gauge burnout-gauge" id="burnout-score-gauge">
              <div class="gauge-label">Burnout Risk</div>
              <div class="gauge-bar large">
                <div class="gauge-fill" id="burnout-fill"></div>
              </div>
              <div class="gauge-value" id="burnout-value">--</div>
            </div>
          </div>
        </div>

        <div class="burnout-chart" id="burnout-chart">
          <canvas id="trend-canvas" width="400" height="120"></canvas>
        </div>

        <div class="burnout-warnings" id="burnout-warnings">
          <h4>Recent Warnings</h4>
          <div class="warnings-list" id="warnings-list">
            <div class="no-warnings">No recent warnings</div>
          </div>
        </div>

        <div class="burnout-episodes" id="burnout-episodes">
          <h4>Burnout Episodes</h4>
          <div class="episodes-list" id="episodes-list">
            <div class="no-episodes">No recorded episodes</div>
          </div>
        </div>
      </div>
    `;
  }

  /**
   * Bind events
   */
  _bindEvents() {
    // Hover events for tooltips
  }

  /**
   * Refresh data
   */
  async refresh() {
    if (!this.api) {
      this._showMockData();
      return;
    }

    try {
      // Fetch burnout risk assessment
      const riskResult = await this.api.call('brain_psychology', {
        includeHistory: true,
        includeCalibration: true,
      });

      if (riskResult.success && riskResult.result) {
        this._updateFromResult(riskResult.result);
      }
    } catch (error) {
      console.error('Burnout trends refresh error:', error);
    }
  }

  /**
   * Update display from API result
   */
  _updateFromResult(result) {
    const dimensions = result.dimensions || {};
    const composites = result.composites || {};

    // Update current values
    this.data.current = {
      energy: dimensions.energy?.value ?? PHI_INV,
      frustration: dimensions.frustration?.value ?? PHI_INV_2,
      burnout: this._calculateBurnoutScore(
        dimensions.energy?.value ?? PHI_INV,
        dimensions.frustration?.value ?? PHI_INV_2,
      ),
    };

    // Update gauges
    this._updateGauges();

    // Update status
    this._updateStatus(composites);

    // Draw trend chart
    this._drawTrendChart();
  }

  /**
   * Calculate burnout score
   */
  _calculateBurnoutScore(energy, frustration) {
    return Math.min(1, (1 - energy) * frustration * 1.5);
  }

  /**
   * Update gauge displays
   */
  _updateGauges() {
    const { energy, frustration, burnout } = this.data.current || {};

    // Energy gauge (higher is better)
    this._setGauge('energy', energy, true);

    // Frustration gauge (lower is better)
    this._setGauge('frustration', frustration, false);

    // Burnout gauge (lower is better)
    this._setGauge('burnout', burnout, false);
  }

  /**
   * Set a gauge value
   */
  _setGauge(name, value, higherIsBetter) {
    if (value === undefined) return;

    const percent = Math.round(value * 100);
    const fillEl = document.getElementById(`${name}-fill`);
    const valueEl = document.getElementById(`${name}-value`);

    if (fillEl) {
      fillEl.style.width = `${percent}%`;

      // Color based on health
      let color;
      if (higherIsBetter) {
        color = percent >= 62 ? 'var(--success)' :
                percent >= 38 ? 'var(--warning)' : 'var(--danger)';
      } else {
        color = percent <= 38 ? 'var(--success)' :
                percent <= 62 ? 'var(--warning)' : 'var(--danger)';
      }
      fillEl.style.backgroundColor = color;
    }

    if (valueEl) {
      valueEl.textContent = `${percent}%`;
    }
  }

  /**
   * Update status display
   */
  _updateStatus(composites) {
    const statusEl = document.getElementById('burnout-status');
    if (!statusEl) return;

    let status, color;
    if (composites?.burnoutRisk) {
      status = '⚠️ Burnout Risk';
      color = 'var(--danger)';
    } else if (composites?.flow) {
      status = '✨ Flow State';
      color = 'var(--success)';
    } else if (composites?.grind) {
      status = '⚙️ Grind Mode';
      color = 'var(--warning)';
    } else {
      status = '😊 Stable';
      color = 'var(--muted)';
    }

    statusEl.textContent = status;
    statusEl.style.color = color;
  }

  /**
   * Draw trend chart
   */
  _drawTrendChart() {
    const canvas = document.getElementById('trend-canvas');
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    const width = canvas.width;
    const height = canvas.height;

    // Clear
    ctx.fillStyle = 'rgba(0,0,0,0.3)';
    ctx.fillRect(0, 0, width, height);

    // Draw grid
    ctx.strokeStyle = 'rgba(255,255,255,0.1)';
    ctx.beginPath();
    for (let y = 0; y <= height; y += height / 4) {
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
    }
    ctx.stroke();

    // Draw threshold lines
    const thresholdY = height * (1 - PHI_INV);
    ctx.strokeStyle = 'rgba(255,255,0,0.3)';
    ctx.setLineDash([5, 5]);
    ctx.beginPath();
    ctx.moveTo(0, thresholdY);
    ctx.lineTo(width, thresholdY);
    ctx.stroke();
    ctx.setLineDash([]);

    // Draw trend line (mock data for now)
    const points = this.data.trends.length > 0 ? this.data.trends :
      this._generateMockTrend();

    if (points.length < 2) return;

    ctx.strokeStyle = 'rgba(255,100,100,0.8)';
    ctx.lineWidth = 2;
    ctx.beginPath();

    const step = width / (points.length - 1);
    for (let i = 0; i < points.length; i++) {
      const x = i * step;
      const y = height * (1 - points[i]);
      if (i === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    }
    ctx.stroke();

    // Draw current point
    if (this.data.current?.burnout !== undefined) {
      const currentX = width - 5;
      const currentY = height * (1 - this.data.current.burnout);
      ctx.fillStyle = 'rgba(255,100,100,1)';
      ctx.beginPath();
      ctx.arc(currentX, currentY, 4, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  /**
   * Generate mock trend data
   */
  _generateMockTrend() {
    const trend = [];
    let value = 0.3;
    for (let i = 0; i < 20; i++) {
      value += (Math.random() - 0.5) * 0.1;
      value = Math.max(0, Math.min(1, value));
      trend.push(value);
    }
    return trend;
  }

  /**
   * Show mock data when API not available
   */
  _showMockData() {
    this.data.current = {
      energy: 0.7,
      frustration: 0.3,
      burnout: 0.25,
    };
    this._updateGauges();
    this._updateStatus({ flow: true });
    this._drawTrendChart();
  }

  /**
   * Start auto-refresh
   */
  _startAutoRefresh() {
    // Refresh every 30 seconds
    this._refreshInterval = setInterval(() => this.refresh(), 30000);
  }

  /**
   * Destroy component
   */
  destroy() {
    if (this._refreshInterval) {
      clearInterval(this._refreshInterval);
    }
  }
}

export default BurnoutTrends;
