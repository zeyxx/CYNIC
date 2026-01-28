/**
 * CYNIC Dashboard - Sidebar Component
 * Navigation + 11 Dogs status
 */

import { Utils } from '../lib/utils.js';
import { Formulas } from '../lib/formulas.js';

export class Sidebar {
  constructor(options = {}) {
    this.onNavClick = options.onNavClick || (() => {});
    this.container = null;
    this.activeNav = 'operator';
    this.dogs = Formulas.DOGS;
    this.stats = { judgments: 0, blocks: 0 };
    this.alerts = [];
  }

  /**
   * Render sidebar
   */
  render(container) {
    this.container = container;
    Utils.clearElement(container);

    // Navigation section
    const navSection = this._createSection('Navigation', this._createNavigation());

    // Dogs section
    const dogsSection = this._createSection('11 Dogs (Sefirot)', this._createDogsStatus());

    // Stats section
    const statsSection = this._createSection('Statistics', this._createStats());

    // Alerts section
    const alertsSection = this._createSection('Recent Alerts', this._createAlerts());

    container.appendChild(navSection);
    container.appendChild(dogsSection);
    container.appendChild(statsSection);
    container.appendChild(alertsSection);
  }

  /**
   * Create section wrapper
   */
  _createSection(title, content) {
    const section = Utils.createElement('div', { className: 'sidebar-section' }, [
      Utils.createElement('div', { className: 'sidebar-title' }, [title]),
      content,
    ]);
    return section;
  }

  /**
   * Create navigation
   */
  _createNavigation() {
    const navItems = [
      { id: 'operator', icon: '📊', label: 'Operator', mode: 'operator' },
      { id: 'dev', icon: '🔬', label: 'Dev', mode: 'dev' },
      { id: 'arch', icon: '🏗️', label: 'Architecture', mode: 'arch' },
      { id: 'live', icon: '📡', label: 'Live', mode: 'live' },
      { id: 'knowledge', icon: '🧠', label: 'Knowledge', mode: 'knowledge' },
      { id: 'memory', icon: '💾', label: 'Memory', mode: 'memory' },
      { id: 'autonomy', icon: '🤖', label: 'Autonomy', mode: 'autonomy' },
      { id: 'resilience', icon: '⚡', label: 'Resilience', mode: 'resilience' },
      { id: 'decisions', icon: '🎯', label: 'Decisions', mode: 'decisions' },
      { id: 'ecosystem', icon: '🌐', label: 'Ecosystem', mode: 'ecosystem' },
      { id: 'singularity', icon: '✨', label: 'Singularity', mode: 'singularity' },
    ];

    const nav = Utils.createElement('nav', { className: 'sidebar-nav' });

    for (const item of navItems) {
      const navItem = Utils.createElement('div', {
        className: `sidebar-nav-item${item.id === this.activeNav ? ' active' : ''}`,
        dataset: { nav: item.id },
        onClick: () => this.onNavClick(item),
      }, [
        Utils.createElement('span', { className: 'icon' }, [item.icon]),
        Utils.createElement('span', {}, [item.label]),
      ]);
      nav.appendChild(navItem);
    }

    return nav;
  }

  /**
   * Create dogs status list
   */
  _createDogsStatus() {
    const dogsContainer = Utils.createElement('div', { className: 'dogs-container', id: 'dogs-status' });

    for (const dog of this.dogs) {
      const dogItem = Utils.createElement('div', {
        className: `dog-item${dog.active ? ' active' : ''}`,
        dataset: { sefirot: dog.sefirot.toLowerCase(), dogId: dog.id },
      }, [
        Utils.createElement('span', {
          className: `dog-status ${dog.active ? 'active' : 'inactive'}`,
        }),
        Utils.createElement('span', { className: 'dog-name' }, [dog.name]),
        Utils.createElement('span', { className: 'dog-sefirot' }, [dog.sefirot]),
      ]);
      dogsContainer.appendChild(dogItem);
    }

    return dogsContainer;
  }

  /**
   * Create stats display
   */
  _createStats() {
    const statsContainer = Utils.createElement('div', { className: 'sidebar-stats', id: 'sidebar-stats' }, [
      this._createStatItem('Judgments', this.stats.judgments),
      this._createStatItem('Blocks', this.stats.blocks),
    ]);
    return statsContainer;
  }

  _createStatItem(label, value) {
    return Utils.createElement('div', { className: 'sidebar-stat' }, [
      Utils.createElement('div', { className: 'sidebar-stat-value' }, [Utils.formatNumber(value)]),
      Utils.createElement('div', { className: 'sidebar-stat-label' }, [label]),
    ]);
  }

  /**
   * Create alerts list
   */
  _createAlerts() {
    const alertsContainer = Utils.createElement('div', { className: 'alerts-container', id: 'sidebar-alerts' });

    if (this.alerts.length === 0) {
      alertsContainer.appendChild(
        Utils.createElement('div', { className: 'text-muted text-sm' }, ['No alerts'])
      );
    } else {
      for (const alert of this.alerts.slice(0, 5)) {
        const alertItem = Utils.createElement('div', {
          className: `alert-item ${alert.type || ''}`,
        }, [
          Utils.createElement('span', { className: 'alert-icon' }, [
            alert.type === 'error' ? '🚨' : alert.type === 'warning' ? '⚠️' : 'ℹ️',
          ]),
          Utils.createElement('div', { className: 'alert-content' }, [
            Utils.createElement('div', { className: 'alert-message' }, [alert.message]),
            Utils.createElement('div', { className: 'alert-time' }, [Utils.formatTime(alert.timestamp)]),
          ]),
        ]);
        alertsContainer.appendChild(alertItem);
      }
    }

    return alertsContainer;
  }

  /**
   * Set active navigation
   */
  setActiveNav(navId) {
    this.activeNav = navId;
    this.container?.querySelectorAll('.sidebar-nav-item').forEach(item => {
      item.classList.toggle('active', item.dataset.nav === navId);
    });
  }

  /**
   * Update dogs status from API
   */
  updateDogs(agents) {
    if (!agents) return;

    const dogsContainer = document.getElementById('dogs-status');
    if (!dogsContainer) return;

    for (const [name, data] of Object.entries(agents)) {
      const dogEl = dogsContainer.querySelector(`[data-dog-id="${name.toLowerCase()}"]`);
      if (dogEl) {
        const isActive = data.active !== false;
        dogEl.classList.toggle('active', isActive);
        const statusEl = dogEl.querySelector('.dog-status');
        if (statusEl) {
          statusEl.className = `dog-status ${isActive ? 'active' : 'inactive'}`;
        }
      }
    }
  }

  /**
   * Update stats
   */
  updateStats(stats) {
    this.stats = { ...this.stats, ...stats };
    const container = document.getElementById('sidebar-stats');
    if (container) {
      Utils.clearElement(container);
      container.appendChild(this._createStatItem('Judgments', this.stats.judgments));
      container.appendChild(this._createStatItem('Blocks', this.stats.blocks));
    }
  }

  /**
   * Add alert
   */
  addAlert(alert) {
    this.alerts.unshift(alert);
    if (this.alerts.length > 10) {
      this.alerts = this.alerts.slice(0, 10);
    }
    this._refreshAlerts();
  }

  _refreshAlerts() {
    const container = document.getElementById('sidebar-alerts');
    if (container) {
      Utils.clearElement(container);
      const alertsEl = this._createAlerts();
      container.replaceWith(alertsEl);
    }
  }
}

// Export to window
window.CYNICSidebar = Sidebar;
