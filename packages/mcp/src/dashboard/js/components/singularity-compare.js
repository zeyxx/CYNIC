/**
 * CYNIC Dashboard - Singularity Compare Component
 * Phase 2.5.3 - Before/after comparison, version tracking
 *
 * "φ distrusts φ" - κυνικός
 */

/* global prompt, confirm */

import { Utils } from '../lib/utils.js';

// Golden ratio
const PHI = 1.618033988749895;

// Dimension info
const DIMENSIONS = {
  codebase: { label: 'Codebase', icon: '🏗️', color: '#667eea' },
  collective: { label: 'Collective', icon: '🐕', color: '#4ecdc4' },
  wisdom: { label: 'Wisdom', icon: '📚', color: '#ffd93d' },
  autonomy: { label: 'Autonomy', icon: '🤖', color: '#f093fb' },
};

export class SingularityCompare {
  constructor(options = {}) {
    this.api = options.api || null;
    this.container = null;
    this.snapshots = [];
    this.compareA = null;
    this.compareB = null;
    this.loading = false;
  }

  /**
   * Render comparison view
   */
  render(container) {
    this.container = container;
    Utils.clearElement(container);
    container.classList.add('singularity-compare');

    // Header
    const header = Utils.createElement('div', { className: 'compare-header' }, [
      Utils.createElement('div', { className: 'compare-title' }, [
        Utils.createElement('span', { className: 'title-icon' }, ['⚖️']),
        'Version Comparison',
      ]),
      Utils.createElement('button', {
        className: 'compare-snapshot-btn',
        onClick: () => this._takeSnapshot(),
      }, ['📸 Snapshot']),
    ]);

    // Snapshot selector
    const selectorSection = Utils.createElement('div', {
      className: 'compare-selector',
      id: 'compare-selector',
    });

    // Comparison view
    const comparisonSection = Utils.createElement('div', {
      className: 'compare-view',
      id: 'compare-view',
    });

    // Difference analysis
    const diffSection = Utils.createElement('div', {
      className: 'compare-diff',
      id: 'compare-diff',
    });

    container.appendChild(header);
    container.appendChild(selectorSection);
    container.appendChild(comparisonSection);
    container.appendChild(diffSection);

    // Load snapshots
    this._loadSnapshots();
    this._renderSelector();
  }

  /**
   * Update with current score data
   */
  updateCurrent(composite, dimensions) {
    // Store current as potential snapshot data
    this._currentData = {
      composite,
      dimensions: { ...dimensions },
      timestamp: Date.now(),
    };
  }

  /**
   * Load saved snapshots
   */
  _loadSnapshots() {
    try {
      const saved = localStorage.getItem('cynic-singularity-snapshots');
      this.snapshots = saved ? JSON.parse(saved) : [];
    } catch (err) {
      console.warn('Failed to load snapshots:', err);
      this.snapshots = [];
    }

    // Add demo snapshots if none exist
    if (this.snapshots.length === 0) {
      this._addDemoSnapshots();
    }
  }

  /**
   * Add demo snapshots for initial display
   */
  _addDemoSnapshots() {
    const now = Date.now();
    const dayMs = 24 * 60 * 60 * 1000;

    this.snapshots = [
      {
        id: 'demo-1',
        name: 'Initial State',
        timestamp: now - 30 * dayMs,
        composite: 35,
        dimensions: { codebase: 40, collective: 45, wisdom: 25, autonomy: 20 },
      },
      {
        id: 'demo-2',
        name: 'Phase 2 Complete',
        timestamp: now - 14 * dayMs,
        composite: 55,
        dimensions: { codebase: 60, collective: 70, wisdom: 45, autonomy: 35 },
      },
      {
        id: 'demo-3',
        name: 'Dashboard Launch',
        timestamp: now - 7 * dayMs,
        composite: 68,
        dimensions: { codebase: 72, collective: 85, wisdom: 65, autonomy: 48 },
      },
    ];
  }

  /**
   * Save snapshots to localStorage
   */
  _saveSnapshots() {
    try {
      localStorage.setItem('cynic-singularity-snapshots', JSON.stringify(this.snapshots));
    } catch (err) {
      console.warn('Failed to save snapshots:', err);
    }
  }

  /**
   * Take a new snapshot
   */
  _takeSnapshot() {
    if (!this._currentData) {
      console.warn('No current data to snapshot');
      return;
    }

    const name = prompt('Snapshot name:', `Snapshot ${new Date().toLocaleDateString()}`);
    if (!name) return;

    const snapshot = {
      id: `snap-${Date.now()}`,
      name,
      timestamp: Date.now(),
      composite: this._currentData.composite,
      dimensions: { ...this._currentData.dimensions },
    };

    this.snapshots.push(snapshot);
    this._saveSnapshots();
    this._renderSelector();
  }

  /**
   * Delete a snapshot
   */
  _deleteSnapshot(id) {
    this.snapshots = this.snapshots.filter(s => s.id !== id);
    this._saveSnapshots();

    if (this.compareA?.id === id) this.compareA = null;
    if (this.compareB?.id === id) this.compareB = null;

    this._renderSelector();
    this._renderComparison();
  }

  /**
   * Render snapshot selector
   */
  _renderSelector() {
    const container = document.getElementById('compare-selector');
    if (!container) return;

    Utils.clearElement(container);

    // Column A selector
    const colA = Utils.createElement('div', { className: 'selector-column' }, [
      Utils.createElement('div', { className: 'selector-label' }, ['Compare A']),
      this._createSnapshotDropdown('a'),
    ]);

    // VS indicator
    const vs = Utils.createElement('div', { className: 'selector-vs' }, ['VS']);

    // Column B selector
    const colB = Utils.createElement('div', { className: 'selector-column' }, [
      Utils.createElement('div', { className: 'selector-label' }, ['Compare B']),
      this._createSnapshotDropdown('b'),
    ]);

    container.appendChild(colA);
    container.appendChild(vs);
    container.appendChild(colB);

    // Snapshots list
    const snapshotsList = Utils.createElement('div', { className: 'snapshots-list' });

    snapshotsList.appendChild(
      Utils.createElement('div', { className: 'snapshots-header' }, [
        `Saved Snapshots (${this.snapshots.length})`,
      ])
    );

    if (this.snapshots.length === 0) {
      snapshotsList.appendChild(
        Utils.createElement('div', { className: 'snapshots-empty' }, [
          'No snapshots. Click "📸 Snapshot" to save current state.',
        ])
      );
    } else {
      for (const snap of this.snapshots.sort((a, b) => b.timestamp - a.timestamp)) {
        snapshotsList.appendChild(
          Utils.createElement('div', { className: 'snapshot-item' }, [
            Utils.createElement('div', { className: 'snapshot-info' }, [
              Utils.createElement('span', { className: 'snapshot-name' }, [snap.name]),
              Utils.createElement('span', { className: 'snapshot-date' }, [
                Utils.formatTime(snap.timestamp, 'date'),
              ]),
              Utils.createElement('span', { className: 'snapshot-score' }, [
                `Score: ${snap.composite}`,
              ]),
            ]),
            Utils.createElement('button', {
              className: 'snapshot-delete',
              onClick: (e) => {
                e.stopPropagation();
                if (confirm(`Delete "${snap.name}"?`)) {
                  this._deleteSnapshot(snap.id);
                }
              },
            }, ['×']),
          ])
        );
      }
    }

    container.appendChild(snapshotsList);
  }

  /**
   * Create snapshot dropdown
   */
  _createSnapshotDropdown(side) {
    const current = side === 'a' ? this.compareA : this.compareB;

    const select = Utils.createElement('select', {
      className: 'snapshot-select',
      onChange: (e) => {
        const snap = this.snapshots.find(s => s.id === e.target.value);
        if (side === 'a') {
          this.compareA = snap || null;
        } else {
          this.compareB = snap || null;
        }
        this._renderComparison();
        this._renderDiff();
      },
    });

    // Empty option
    const emptyOpt = Utils.createElement('option', { value: '' }, ['-- Select --']);
    select.appendChild(emptyOpt);

    // Current option
    if (this._currentData) {
      const currentOpt = Utils.createElement('option', { value: 'current' }, [
        `Current (${this._currentData.composite})`,
      ]);
      if (current === 'current') currentOpt.selected = true;
      select.appendChild(currentOpt);
    }

    // Snapshot options
    for (const snap of this.snapshots.sort((a, b) => b.timestamp - a.timestamp)) {
      const opt = Utils.createElement('option', { value: snap.id }, [
        `${snap.name} (${snap.composite})`,
      ]);
      if (current?.id === snap.id) opt.selected = true;
      select.appendChild(opt);
    }

    // Handle current selection
    select.addEventListener('change', (e) => {
      if (e.target.value === 'current' && this._currentData) {
        if (side === 'a') {
          this.compareA = { ...this._currentData, id: 'current', name: 'Current' };
        } else {
          this.compareB = { ...this._currentData, id: 'current', name: 'Current' };
        }
        this._renderComparison();
        this._renderDiff();
      }
    });

    return select;
  }

  /**
   * Render comparison view
   */
  _renderComparison() {
    const container = document.getElementById('compare-view');
    if (!container) return;

    Utils.clearElement(container);

    if (!this.compareA && !this.compareB) {
      container.appendChild(
        Utils.createElement('div', { className: 'compare-empty' }, [
          'Select two snapshots to compare',
        ])
      );
      return;
    }

    const grid = Utils.createElement('div', { className: 'compare-grid' });

    // Header row
    grid.appendChild(Utils.createElement('div', { className: 'compare-cell header' }, ['Dimension']));
    grid.appendChild(Utils.createElement('div', { className: 'compare-cell header a' }, [
      this.compareA?.name || '—',
    ]));
    grid.appendChild(Utils.createElement('div', { className: 'compare-cell header b' }, [
      this.compareB?.name || '—',
    ]));
    grid.appendChild(Utils.createElement('div', { className: 'compare-cell header diff' }, ['Diff']));

    // Composite row
    this._addCompareRow(grid, 'Composite', '📊',
      this.compareA?.composite,
      this.compareB?.composite
    );

    // Dimension rows
    for (const [key, dim] of Object.entries(DIMENSIONS)) {
      this._addCompareRow(grid, dim.label, dim.icon,
        this.compareA?.dimensions?.[key],
        this.compareB?.dimensions?.[key],
        dim.color
      );
    }

    container.appendChild(grid);
  }

  /**
   * Add a comparison row
   */
  _addCompareRow(grid, label, icon, valueA, valueB, color = '#4ecdc4') {
    const diff = (valueA !== undefined && valueB !== undefined)
      ? valueB - valueA
      : null;

    grid.appendChild(Utils.createElement('div', { className: 'compare-cell label' }, [
      Utils.createElement('span', { className: 'cell-icon' }, [icon]),
      label,
    ]));

    grid.appendChild(Utils.createElement('div', { className: 'compare-cell value a' }, [
      valueA !== undefined ? String(valueA) : '—',
    ]));

    grid.appendChild(Utils.createElement('div', { className: 'compare-cell value b' }, [
      valueB !== undefined ? String(valueB) : '—',
    ]));

    const diffClass = diff === null ? '' : diff > 0 ? 'positive' : diff < 0 ? 'negative' : '';
    grid.appendChild(Utils.createElement('div', {
      className: `compare-cell diff ${diffClass}`,
    }, [
      diff === null ? '—' : `${diff > 0 ? '+' : ''}${diff}`,
    ]));
  }

  /**
   * Render difference analysis
   */
  _renderDiff() {
    const container = document.getElementById('compare-diff');
    if (!container) return;

    Utils.clearElement(container);

    if (!this.compareA || !this.compareB) {
      return;
    }

    container.appendChild(
      Utils.createElement('div', { className: 'diff-header' }, ['Analysis'])
    );

    const diffComposite = this.compareB.composite - this.compareA.composite;
    const percentChange = ((diffComposite / this.compareA.composite) * 100).toFixed(1);

    const content = Utils.createElement('div', { className: 'diff-content' });

    // Overall change
    const overall = Utils.createElement('div', { className: 'diff-overall' }, [
      Utils.createElement('span', { className: 'diff-icon' }, [
        diffComposite > 0 ? '📈' : diffComposite < 0 ? '📉' : '➡️',
      ]),
      Utils.createElement('span', { className: 'diff-text' }, [
        diffComposite > 0
          ? `Improved by ${diffComposite} points (${percentChange}%)`
          : diffComposite < 0
          ? `Declined by ${Math.abs(diffComposite)} points (${percentChange}%)`
          : 'No change in composite score',
      ]),
    ]);
    content.appendChild(overall);

    // Dimension changes
    const changes = [];
    for (const [key, dim] of Object.entries(DIMENSIONS)) {
      const valA = this.compareA.dimensions?.[key] || 0;
      const valB = this.compareB.dimensions?.[key] || 0;
      const diff = valB - valA;
      if (diff !== 0) {
        changes.push({ key, dim, diff, valA, valB });
      }
    }

    if (changes.length > 0) {
      const changesEl = Utils.createElement('div', { className: 'diff-changes' });

      // Sort by absolute change
      changes.sort((a, b) => Math.abs(b.diff) - Math.abs(a.diff));

      for (const change of changes) {
        changesEl.appendChild(
          Utils.createElement('div', {
            className: `diff-change ${change.diff > 0 ? 'positive' : 'negative'}`,
          }, [
            Utils.createElement('span', { className: 'change-icon' }, [change.dim.icon]),
            Utils.createElement('span', { className: 'change-label' }, [change.dim.label]),
            Utils.createElement('span', { className: 'change-value' }, [
              `${change.valA} → ${change.valB} (${change.diff > 0 ? '+' : ''}${change.diff})`,
            ]),
          ])
        );
      }

      content.appendChild(changesEl);
    }

    // φ insight
    const phiFactor = (this.compareB.composite / this.compareA.composite);
    const isPhiAligned = Math.abs(phiFactor - PHI) < 0.1 || Math.abs(phiFactor - 1/PHI) < 0.1;

    content.appendChild(
      Utils.createElement('div', { className: 'diff-phi' }, [
        Utils.createElement('span', { className: 'phi-symbol' }, ['φ']),
        Utils.createElement('span', { className: 'phi-insight' }, [
          isPhiAligned
            ? `Growth ratio (${phiFactor.toFixed(3)}) approaches golden proportion!`
            : `Growth ratio: ${phiFactor.toFixed(3)}`,
        ]),
      ])
    );

    container.appendChild(content);
  }

  /**
   * Cleanup
   */
  destroy() {
    this.container = null;
    this.snapshots = [];
  }
}

// Export to window
window.CYNICSingularityCompare = SingularityCompare;
