/**
 * CYNIC Dashboard - Operator View
 * Metrics, chain visualization, alerts
 */

import { Utils } from '../lib/utils.js';
import { MetricsCards } from '../components/metrics-cards.js';
import { ChainViz } from '../components/chain-viz.js';
import { DogsStatus } from '../components/dogs-status.js';
import { PatternGallery } from '../components/pattern-gallery.js';
import { BurnoutTrends } from '../components/burnout-trends.js';

export class OperatorView {
  constructor(options = {}) {
    this.api = options.api;
    this.container = null;
    this.metricsCards = new MetricsCards();
    this.chainViz = new ChainViz({
      api: options.api,
      onBlockClick: (block) => this._onBlockClick(block),
      onJudgmentClick: (jdg) => this._onJudgmentClick(jdg),
    });
    this.dogsStatus = new DogsStatus({
      onDogClick: (dog) => this._onDogClick(dog),
    });
    this.patternGallery = new PatternGallery({
      api: options.api,
      onPatternClick: (pattern) => this._onPatternClick(pattern),
    });
    this.burnoutTrends = new BurnoutTrends({
      api: options.api,
      onWarning: (warning) => this._onBurnoutWarning(warning),
    });
    this.alerts = [];
  }

  /**
   * Render operator view
   */
  render(container) {
    this.container = container;
    Utils.clearElement(container);
    container.classList.add('operator-view');

    // Create layout
    const layout = Utils.createElement('div', { className: 'operator-layout' });

    // Top section: Metrics
    const metricsSection = Utils.createElement('section', { className: 'operator-section metrics-section' });
    const metricsHeader = Utils.createElement('div', { className: 'section-header' }, [
      Utils.createElement('h2', {}, ['System Metrics']),
      Utils.createElement('button', {
        className: 'btn btn-sm',
        onClick: () => this.refresh(),
      }, ['↻ Refresh']),
    ]);
    const metricsContent = Utils.createElement('div', { id: 'metrics-container' });
    metricsSection.appendChild(metricsHeader);
    metricsSection.appendChild(metricsContent);

    // Middle section: Chain + Dogs
    const middleSection = Utils.createElement('div', { className: 'operator-middle' });

    // Chain visualization
    const chainSection = Utils.createElement('section', { className: 'operator-section chain-section' });
    const chainContent = Utils.createElement('div', { id: 'chain-container' });
    chainSection.appendChild(chainContent);

    // Dogs status (grid)
    const dogsSection = Utils.createElement('section', { className: 'operator-section dogs-section' });
    const dogsHeader = Utils.createElement('div', { className: 'section-header' }, [
      Utils.createElement('h2', {}, ['The Collective (11 Dogs)']),
    ]);
    const dogsContent = Utils.createElement('div', { id: 'dogs-container' });
    dogsSection.appendChild(dogsHeader);
    dogsSection.appendChild(dogsContent);

    middleSection.appendChild(chainSection);
    middleSection.appendChild(dogsSection);

    // Pattern Gallery section
    const patternSection = Utils.createElement('section', { className: 'operator-section pattern-section' });
    const patternContent = Utils.createElement('div', { id: 'pattern-container' });
    patternSection.appendChild(patternContent);

    // Burnout Trends section (v1.1)
    const burnoutSection = Utils.createElement('section', { className: 'operator-section burnout-section' });
    const burnoutContent = Utils.createElement('div', { id: 'burnout-container' });
    burnoutSection.appendChild(burnoutContent);

    // Bottom section: Alerts
    const alertsSection = Utils.createElement('section', { className: 'operator-section alerts-section' });
    const alertsHeader = Utils.createElement('div', { className: 'section-header' }, [
      Utils.createElement('h2', {}, ['Recent Activity']),
      Utils.createElement('span', { className: 'badge', id: 'alerts-count' }, ['0']),
    ]);
    const alertsContent = Utils.createElement('div', { className: 'alerts-list', id: 'alerts-container' });
    alertsSection.appendChild(alertsHeader);
    alertsSection.appendChild(alertsContent);

    // Assemble
    layout.appendChild(metricsSection);
    layout.appendChild(middleSection);
    layout.appendChild(patternSection);
    layout.appendChild(burnoutSection);
    layout.appendChild(alertsSection);
    container.appendChild(layout);

    // Render components
    this.metricsCards.render(metricsContent, MetricsCards.getDefaultCards());
    this.chainViz.render(chainContent);
    this.patternGallery.render(patternContent);
    this.burnoutTrends.render(burnoutContent);
    this.dogsStatus.render(dogsContent, 'grid');
    this._renderAlerts();

    // Initial data load
    this.refresh();
  }

  /**
   * Refresh all data
   */
  async refresh() {
    if (!this.api) return;

    // Fetch all data in parallel
    const [healthResult, chainResult, collectiveResult] = await Promise.all([
      this.api.health(true),
      this.api.chain('status'),
      this.api.collectiveStatus(true),
    ]);

    // Update metrics from health result (judge stats)
    if (healthResult.success && healthResult.result) {
      const h = healthResult.result;
      // judge.getStats() returns: { totalJudgments, avgScore, verdicts, anomaliesDetected }
      this.metricsCards.updateCard('judgments', h.judge?.totalJudgments || 0);
      this.metricsCards.updateCard('qScore', h.judge?.avgScore || 0);
    }

    // Update chain
    if (chainResult.success && chainResult.result) {
      const chain = chainResult.result;
      this.chainViz.update({
        head: chain.head,
        stats: chain.stats,
        recentBlocks: chain.recentBlocks,
      });
      this.metricsCards.updateCard('blocks', chain.stats?.totalBlocks || 0);
    }

    // Update dogs
    if (collectiveResult.success && collectiveResult.result) {
      const collective = collectiveResult.result;
      this.dogsStatus.update(collective.agents);
    }

    // Fetch patterns for metrics
    const patternsResult = await this.api.patterns('all', 10);
    if (patternsResult.success && patternsResult.result) {
      this.metricsCards.updateCard('patterns', patternsResult.result.patterns?.length || 0);
    }
  }

  /**
   * Handle judgment event (called by app.js)
   */
  onJudgment(judgment) {
    this._addAlert({
      type: 'info',
      message: `Judgment: ${judgment?.verdict || 'PROCESSED'} (Q=${judgment?.qScore || '?'})`,
      timestamp: Date.now(),
    });
    // Update metrics
    const currentJudgments = parseInt(document.getElementById('metric-judgments')?.textContent || '0');
    this.metricsCards.updateCard('judgments', currentJudgments + 1);
  }

  /**
   * Handle new block event (called by app.js)
   */
  onNewBlock(block) {
    this.chainViz.addBlock(block);
    this._addAlert({
      type: 'success',
      message: `New block #${block?.blockNumber} created`,
      timestamp: Date.now(),
    });
    // Update block count
    const currentBlocks = parseInt(document.getElementById('metric-blocks')?.textContent || '0');
    this.metricsCards.updateCard('blocks', currentBlocks + 1);
  }

  /**
   * Add alert (public API)
   */
  addAlert(alert) {
    this._addAlert(alert);
  }

  /**
   * Handle SSE events (generic handler)
   */
  handleEvent(event) {
    switch (event.type) {
      case 'judgment':
        this.onJudgment(event.data);
        break;

      case 'block':
        this.onNewBlock(event.data);
        break;

      case 'pattern':
        this.onNewPattern(event.data);
        break;

      case 'agent':
        if (event.data?.agents) {
          this.dogsStatus.update(event.data.agents);
        }
        break;

      case 'burnout':
        if (event.data) {
          this._onBurnoutWarning(event.data);
          this.burnoutTrends.refresh();
        }
        break;
    }
  }

  /**
   * Add alert to list
   */
  _addAlert(alert) {
    this.alerts.unshift(alert);
    if (this.alerts.length > 20) {
      this.alerts = this.alerts.slice(0, 20);
    }
    this._renderAlerts();

    // Update badge
    const badge = document.getElementById('alerts-count');
    if (badge) {
      badge.textContent = String(this.alerts.length);
    }
  }

  /**
   * Render alerts list
   */
  _renderAlerts() {
    const container = document.getElementById('alerts-container');
    if (!container) return;

    Utils.clearElement(container);

    if (this.alerts.length === 0) {
      container.appendChild(
        Utils.createElement('div', { className: 'text-muted text-center' }, ['No recent activity'])
      );
      return;
    }

    for (const alert of this.alerts) {
      const alertEl = Utils.createElement('div', { className: `alert-item ${alert.type}` }, [
        Utils.createElement('span', { className: 'alert-icon' }, [
          alert.type === 'success' ? '✓' : alert.type === 'warning' ? '⚠' : alert.type === 'error' ? '✕' : 'ℹ',
        ]),
        Utils.createElement('div', { className: 'alert-content' }, [
          Utils.createElement('span', { className: 'alert-message' }, [alert.message]),
          Utils.createElement('span', { className: 'alert-time' }, [Utils.formatTime(alert.timestamp)]),
        ]),
      ]);
      container.appendChild(alertEl);
    }
  }

  /**
   * Handle block click
   */
  _onBlockClick(block) {
    console.log('Block clicked:', block);
  }

  /**
   * Handle dog click
   */
  _onDogClick(dog) {
    console.log('Dog clicked:', dog);
    this._addAlert({
      type: 'info',
      message: `${dog.name} (${dog.sefirot}): ${dog.role}`,
      timestamp: Date.now(),
    });
  }

  /**
   * Handle judgment click (from chain viz)
   */
  _onJudgmentClick(jdg) {
    console.log('Judgment clicked:', jdg);
    const verdict = jdg.verdict || 'UNKNOWN';
    const q = jdg.Q || jdg.qScore || 0;
    this._addAlert({
      type: verdict === 'HOWL' || verdict === 'WAG' ? 'success' : verdict === 'BARK' ? 'error' : 'warning',
      message: `Judgment ${jdg.id?.slice(0, 12) || '?'}: ${verdict} (Q=${Math.round(q)})`,
      timestamp: Date.now(),
    });
  }

  /**
   * Handle pattern click (from pattern gallery)
   */
  _onPatternClick(pattern) {
    console.log('Pattern clicked:', pattern);
    this._addAlert({
      type: 'info',
      message: `Pattern: ${pattern.category || 'unknown'} - ${pattern.description || pattern.type || 'detected'}`,
      timestamp: Date.now(),
    });
  }

  /**
   * Handle new pattern event (called by app.js)
   */
  onNewPattern(pattern) {
    this.patternGallery.addPattern(pattern);
    this._addAlert({
      type: 'warning',
      message: `New pattern: ${pattern.category || 'unknown'} detected`,
      timestamp: Date.now(),
    });
  }

  /**
   * Handle burnout warning (v1.1)
   */
  _onBurnoutWarning(warning) {
    const severity = warning.severity || 'low';
    const type = severity === 'critical' ? 'error' : severity === 'elevated' ? 'warning' : 'info';
    this._addAlert({
      type,
      message: `🔥 Burnout ${severity}: ${warning.message || warning.warningType || 'Risk detected'}`,
      timestamp: Date.now(),
    });
  }

  /**
   * Cleanup
   */
  destroy() {
    this.burnoutTrends?.destroy();
    this.container = null;
  }
}

// Export to window
window.CYNICOperatorView = OperatorView;
