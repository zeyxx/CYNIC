/**
 * CYNIC JSON Render Service
 *
 * Enables streaming JSON responses with component-based rendering.
 * Inspired by vercel-labs/json-render for safe, progressive UI generation.
 *
 * Features:
 * - Component catalog (constrained component generation)
 * - Streaming JSON responses (progressive rendering)
 * - Conditional visibility (show/hide based on data)
 * - Action binding (click handlers, form submissions)
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/mcp/json-render
 */

'use strict';

// φ-derived constants
const PHI = 1.618033988749895;
const MAX_CHILDREN = Math.round(21 * PHI); // ~34

/**
 * Component types supported by CYNIC dashboard
 */
export const ComponentType = {
  // Layout
  Container: 'container',
  Row: 'row',
  Column: 'column',
  Card: 'card',
  Divider: 'divider',

  // Text
  Heading: 'heading',
  Text: 'text',
  Code: 'code',
  Badge: 'badge',
  Label: 'label',

  // Data Display
  Table: 'table',
  List: 'list',
  Tree: 'tree',
  Chart: 'chart',
  Metric: 'metric',
  Progress: 'progress',

  // Interactive
  Button: 'button',
  Input: 'input',
  Select: 'select',
  Checkbox: 'checkbox',
  Toggle: 'toggle',

  // CYNIC-specific
  JudgmentCard: 'judgment-card',
  DogStatus: 'dog-status',
  ChainBlock: 'chain-block',
  PatternCard: 'pattern-card',
  AxiomScore: 'axiom-score',
};

/**
 * Component catalog with schema definitions
 */
const COMPONENT_CATALOG = {
  [ComponentType.Container]: {
    props: ['id', 'className', 'style'],
    children: true,
  },
  [ComponentType.Row]: {
    props: ['gap', 'align', 'justify'],
    children: true,
  },
  [ComponentType.Column]: {
    props: ['span', 'gap'],
    children: true,
  },
  [ComponentType.Card]: {
    props: ['title', 'subtitle', 'variant', 'collapsible', 'collapsed'],
    children: true,
  },
  [ComponentType.Heading]: {
    props: ['level', 'text'],
    children: false,
  },
  [ComponentType.Text]: {
    props: ['text', 'variant', 'color'],
    children: false,
  },
  [ComponentType.Code]: {
    props: ['code', 'language', 'lineNumbers'],
    children: false,
  },
  [ComponentType.Badge]: {
    props: ['text', 'variant', 'color'],
    children: false,
  },
  [ComponentType.Table]: {
    props: ['columns', 'rows', 'sortable', 'searchable'],
    children: false,
  },
  [ComponentType.List]: {
    props: ['items', 'ordered', 'variant'],
    children: false,
  },
  [ComponentType.Chart]: {
    props: ['type', 'data', 'options'],
    children: false,
  },
  [ComponentType.Metric]: {
    props: ['label', 'value', 'unit', 'change', 'trend'],
    children: false,
  },
  [ComponentType.Progress]: {
    props: ['value', 'max', 'label', 'variant'],
    children: false,
  },
  [ComponentType.Button]: {
    props: ['text', 'variant', 'action', 'disabled'],
    children: false,
  },
  [ComponentType.Input]: {
    props: ['name', 'type', 'placeholder', 'value', 'validation'],
    children: false,
  },
  [ComponentType.Select]: {
    props: ['name', 'options', 'value', 'placeholder'],
    children: false,
  },
  // CYNIC-specific components
  [ComponentType.JudgmentCard]: {
    props: ['judgment', 'showBreakdown', 'showTimeline'],
    children: false,
  },
  [ComponentType.DogStatus]: {
    props: ['dog', 'showStats', 'compact'],
    children: false,
  },
  [ComponentType.ChainBlock]: {
    props: ['block', 'showJudgments', 'showHash'],
    children: false,
  },
  [ComponentType.PatternCard]: {
    props: ['pattern', 'showConfidence', 'showSource'],
    children: false,
  },
  [ComponentType.AxiomScore]: {
    props: ['axiom', 'score', 'dimensions', 'animated'],
    children: false,
  },
};

/**
 * JSON Render Service
 */
export class JSONRenderService {
  constructor(options = {}) {
    this.catalog = { ...COMPONENT_CATALOG, ...options.customComponents };
    this.strict = options.strict !== false; // Reject unknown components by default
    this.stats = {
      nodesRendered: 0,
      streamsCreated: 0,
      validationErrors: 0,
    };
  }

  /**
   * Validate a component tree
   * @param {Object} tree - Component tree
   * @returns {Object} Validation result
   */
  validate(tree) {
    const errors = [];
    this._validateNode(tree, errors, []);

    return {
      valid: errors.length === 0,
      errors,
      nodeCount: this._countNodes(tree),
    };
  }

  /**
   * Render component tree to HTML string
   * @param {Object} tree - Component tree
   * @param {Object} [data] - Data context for bindings
   * @returns {string} HTML string
   */
  render(tree, data = {}) {
    const validation = this.validate(tree);
    if (!validation.valid && this.strict) {
      throw new Error(`Invalid component tree: ${validation.errors[0]}`);
    }

    this.stats.nodesRendered += validation.nodeCount;
    return this._renderNode(tree, data);
  }

  /**
   * Create a streaming renderer
   * @param {Object} tree - Component tree
   * @param {Object} [data] - Data context
   * @returns {AsyncGenerator} Streaming generator
   */
  async *stream(tree, data = {}) {
    this.stats.streamsCreated++;

    // Validate first
    const validation = this.validate(tree);
    if (!validation.valid && this.strict) {
      yield JSON.stringify({ error: validation.errors[0] });
      return;
    }

    // Stream nodes progressively
    yield* this._streamNode(tree, data, 0);
  }

  /**
   * Create component programmatically
   * @param {string} type - Component type
   * @param {Object} props - Component props
   * @param {Array} [children] - Child components
   * @returns {Object} Component node
   */
  createComponent(type, props = {}, children = []) {
    if (this.strict && !this.catalog[type]) {
      throw new Error(`Unknown component type: ${type}`);
    }

    return {
      type,
      props,
      children: children.length > 0 ? children : undefined,
    };
  }

  /**
   * Build a layout from CYNIC data
   * @param {string} layoutType - Type of layout (judgment, chain, dogs, patterns)
   * @param {Object} data - Data to render
   * @returns {Object} Component tree
   */
  buildLayout(layoutType, data) {
    switch (layoutType) {
      case 'judgment':
        return this._buildJudgmentLayout(data);
      case 'chain':
        return this._buildChainLayout(data);
      case 'dogs':
        return this._buildDogsLayout(data);
      case 'patterns':
        return this._buildPatternsLayout(data);
      case 'dashboard':
        return this._buildDashboardLayout(data);
      default:
        return this.createComponent(ComponentType.Text, { text: 'Unknown layout type' });
    }
  }

  /**
   * Get service statistics
   * @returns {Object} Statistics
   */
  getStats() {
    return {
      ...this.stats,
      catalogSize: Object.keys(this.catalog).length,
    };
  }

  // ============ Private Methods ============

  /**
   * Validate a single node
   * @private
   */
  _validateNode(node, errors, path) {
    if (!node || typeof node !== 'object') {
      errors.push(`Invalid node at ${path.join('.')}`);
      return;
    }

    const { type, props: _props, children, show: _show } = node;

    // Check type
    if (!type) {
      errors.push(`Missing type at ${path.join('.') || 'root'}`);
      return;
    }

    if (this.strict && !this.catalog[type]) {
      errors.push(`Unknown component type "${type}" at ${path.join('.') || 'root'}`);
      this.stats.validationErrors++;
    }

    // Check children
    if (children) {
      if (!Array.isArray(children)) {
        errors.push(`Children must be array at ${path.join('.')}`);
      } else if (children.length > MAX_CHILDREN) {
        errors.push(`Too many children (${children.length} > ${MAX_CHILDREN}) at ${path.join('.')}`);
      } else {
        children.forEach((child, i) => {
          this._validateNode(child, errors, [...path, `children[${i}]`]);
        });
      }
    }
  }

  /**
   * Count nodes in tree
   * @private
   */
  _countNodes(node) {
    if (!node) return 0;
    let count = 1;
    if (node.children) {
      count += node.children.reduce((sum, child) => sum + this._countNodes(child), 0);
    }
    return count;
  }

  /**
   * Render a single node to HTML
   * @private
   */
  _renderNode(node, data) {
    const { type, props = {}, children, show } = node;

    // Check visibility condition
    if (show !== undefined && !this._evaluateCondition(show, data)) {
      return '';
    }

    // Resolve data bindings in props
    const resolvedProps = this._resolveBindings(props, data);

    // Render based on type
    const html = this._renderComponent(type, resolvedProps, children, data);
    return html;
  }

  /**
   * Render component to HTML
   * @private
   */
  _renderComponent(type, props, children, data) {
    const childHtml = children
      ? children.map(c => this._renderNode(c, data)).join('')
      : '';

    switch (type) {
      case ComponentType.Container:
        return `<div class="cynic-container ${props.className || ''}" style="${props.style || ''}">${childHtml}</div>`;

      case ComponentType.Row:
        return `<div class="cynic-row" style="gap: ${props.gap || '1rem'}; align-items: ${props.align || 'stretch'}; justify-content: ${props.justify || 'flex-start'}">${childHtml}</div>`;

      case ComponentType.Column:
        return `<div class="cynic-column" style="flex: ${props.span || 1}; gap: ${props.gap || '0.5rem'}">${childHtml}</div>`;

      case ComponentType.Card:
        return `<div class="cynic-card ${props.variant || ''}">
          ${props.title ? `<div class="cynic-card-header"><h3>${props.title}</h3>${props.subtitle ? `<p>${props.subtitle}</p>` : ''}</div>` : ''}
          <div class="cynic-card-body">${childHtml}</div>
        </div>`;

      case ComponentType.Heading: {
        const level = props.level || 2;
        return `<h${level} class="cynic-heading">${this._escapeHtml(props.text)}</h${level}>`;
      }

      case ComponentType.Text:
        return `<p class="cynic-text ${props.variant || ''}" style="color: ${props.color || 'inherit'}">${this._escapeHtml(props.text)}</p>`;

      case ComponentType.Code:
        return `<pre class="cynic-code" data-language="${props.language || 'text'}"><code>${this._escapeHtml(props.code)}</code></pre>`;

      case ComponentType.Badge:
        return `<span class="cynic-badge ${props.variant || ''}" style="background: ${props.color || 'var(--accent)'}">${this._escapeHtml(props.text)}</span>`;

      case ComponentType.Metric: {
        const trend = props.trend === 'up' ? '↑' : props.trend === 'down' ? '↓' : '';
        return `<div class="cynic-metric">
          <span class="label">${this._escapeHtml(props.label)}</span>
          <span class="value">${props.value}${props.unit || ''}</span>
          ${props.change ? `<span class="change ${props.trend || ''}">${trend} ${props.change}</span>` : ''}
        </div>`;
      }

      case ComponentType.Progress: {
        const percent = ((props.value / (props.max || 100)) * 100).toFixed(1);
        return `<div class="cynic-progress ${props.variant || ''}">
          <div class="bar" style="width: ${percent}%"></div>
          ${props.label ? `<span class="label">${props.label}: ${percent}%</span>` : ''}
        </div>`;
      }

      case ComponentType.Table:
        return this._renderTable(props);

      case ComponentType.List: {
        const tag = props.ordered ? 'ol' : 'ul';
        const items = (props.items || []).map(item => `<li>${this._escapeHtml(String(item))}</li>`).join('');
        return `<${tag} class="cynic-list ${props.variant || ''}">${items}</${tag}>`;
      }

      case ComponentType.Button:
        return `<button class="cynic-button ${props.variant || ''}" ${props.disabled ? 'disabled' : ''} data-action="${props.action || ''}">${this._escapeHtml(props.text)}</button>`;

      // CYNIC-specific components
      case ComponentType.JudgmentCard:
        return this._renderJudgmentCard(props);

      case ComponentType.DogStatus:
        return this._renderDogStatus(props);

      case ComponentType.AxiomScore:
        return this._renderAxiomScore(props);

      default:
        return `<div class="cynic-unknown" data-type="${type}">${childHtml || JSON.stringify(props)}</div>`;
    }
  }

  /**
   * Stream nodes progressively
   * @private
   */
  async *_streamNode(node, data, depth) {
    const { type, props = {}, children, show } = node;

    // Check visibility
    if (show !== undefined && !this._evaluateCondition(show, data)) {
      return;
    }

    // Resolve props
    const resolvedProps = this._resolveBindings(props, data);

    // Yield opening
    yield JSON.stringify({
      action: 'open',
      depth,
      type,
      props: resolvedProps,
    }) + '\n';

    // Stream children
    if (children) {
      for (const child of children) {
        yield* this._streamNode(child, data, depth + 1);
      }
    }

    // Yield closing
    yield JSON.stringify({
      action: 'close',
      depth,
      type,
    }) + '\n';
  }

  /**
   * Evaluate visibility condition
   * @private
   */
  _evaluateCondition(condition, data) {
    if (typeof condition === 'boolean') return condition;
    if (typeof condition === 'string') {
      // Simple path lookup
      return !!this._resolvePath(condition, data);
    }
    if (typeof condition === 'object') {
      // Complex condition
      if (condition.and) {
        return condition.and.every(c => this._evaluateCondition(c, data));
      }
      if (condition.or) {
        return condition.or.some(c => this._evaluateCondition(c, data));
      }
      if (condition.not) {
        return !this._evaluateCondition(condition.not, data);
      }
      if (condition.eq) {
        const [path, value] = condition.eq;
        return this._resolvePath(path, data) === value;
      }
    }
    return true;
  }

  /**
   * Resolve data bindings in props
   * @private
   */
  _resolveBindings(props, data) {
    const resolved = {};
    for (const [key, value] of Object.entries(props)) {
      if (typeof value === 'string' && value.startsWith('$')) {
        // Data binding: $path.to.value
        resolved[key] = this._resolvePath(value.slice(1), data);
      } else if (typeof value === 'object' && value !== null) {
        resolved[key] = this._resolveBindings(value, data);
      } else {
        resolved[key] = value;
      }
    }
    return resolved;
  }

  /**
   * Resolve a path in data object
   * @private
   */
  _resolvePath(path, data) {
    return path.split('.').reduce((obj, key) => obj?.[key], data);
  }

  /**
   * Render table component
   * @private
   */
  _renderTable(props) {
    const { columns = [], rows = [] } = props;
    const headers = columns.map(c => `<th>${this._escapeHtml(c.label || c.key)}</th>`).join('');
    const body = rows.map(row => {
      const cells = columns.map(c => `<td>${this._escapeHtml(String(row[c.key] ?? ''))}</td>`).join('');
      return `<tr>${cells}</tr>`;
    }).join('');

    return `<table class="cynic-table">
      <thead><tr>${headers}</tr></thead>
      <tbody>${body}</tbody>
    </table>`;
  }

  /**
   * Render judgment card
   * @private
   */
  _renderJudgmentCard(props) {
    const j = props.judgment || {};
    const verdict = j.verdict || 'UNKNOWN';
    const score = j.Q ?? j.score ?? 0;

    return `<div class="cynic-judgment-card verdict-${verdict.toLowerCase()}">
      <div class="header">
        <span class="verdict">${verdict}</span>
        <span class="score">Q: ${score.toFixed(1)}</span>
      </div>
      ${props.showBreakdown && j.breakdown ? `<div class="breakdown">
        ${Object.entries(j.breakdown).map(([k, v]) => `<div class="axiom"><span>${k}</span><span>${v.toFixed(1)}</span></div>`).join('')}
      </div>` : ''}
    </div>`;
  }

  /**
   * Render dog status
   * @private
   */
  _renderDogStatus(props) {
    const dog = props.dog || {};
    const compact = props.compact;

    return `<div class="cynic-dog-status ${compact ? 'compact' : ''}">
      <span class="name">${dog.name || 'Unknown'}</span>
      <span class="sefira">${dog.sefira || ''}</span>
      ${props.showStats && dog.invocations !== undefined ? `<span class="stats">${dog.invocations} invocations</span>` : ''}
    </div>`;
  }

  /**
   * Render axiom score
   * @private
   */
  _renderAxiomScore(props) {
    const { axiom, score, dimensions, animated } = props;
    const percent = ((score / 25) * 100).toFixed(1);

    return `<div class="cynic-axiom-score ${animated ? 'animated' : ''}" data-axiom="${axiom}">
      <div class="header">
        <span class="name">${axiom}</span>
        <span class="score">${score.toFixed(1)}/25</span>
      </div>
      <div class="bar" style="width: ${percent}%"></div>
      ${dimensions ? `<div class="dimensions">
        ${dimensions.map(d => `<span class="dim">${d.name}: ${d.score.toFixed(1)}</span>`).join('')}
      </div>` : ''}
    </div>`;
  }

  /**
   * Build judgment layout
   * @private
   */
  _buildJudgmentLayout(data) {
    const judgment = data.judgment || data;
    return this.createComponent(ComponentType.Card, { title: 'Judgment Result', variant: 'judgment' }, [
      this.createComponent(ComponentType.JudgmentCard, { judgment, showBreakdown: true }),
    ]);
  }

  /**
   * Build chain layout
   * @private
   */
  _buildChainLayout(data) {
    const blocks = data.blocks || [];
    return this.createComponent(ComponentType.Container, { className: 'chain-layout' }, [
      this.createComponent(ComponentType.Heading, { level: 2, text: 'PoJ Chain' }),
      this.createComponent(ComponentType.Row, { gap: '1rem' }, [
        this.createComponent(ComponentType.Metric, { label: 'Blocks', value: blocks.length }),
        this.createComponent(ComponentType.Metric, { label: 'Head', value: data.head || 0 }),
      ]),
      ...blocks.slice(0, 10).map(block =>
        this.createComponent(ComponentType.ChainBlock, { block, showHash: true })
      ),
    ]);
  }

  /**
   * Build dogs layout
   * @private
   */
  _buildDogsLayout(data) {
    const dogs = data.dogs || {};
    return this.createComponent(ComponentType.Container, { className: 'dogs-layout' }, [
      this.createComponent(ComponentType.Heading, { level: 2, text: 'The Collective' }),
      this.createComponent(ComponentType.Row, { gap: '1rem' },
        Object.values(dogs).map(dog =>
          this.createComponent(ComponentType.DogStatus, { dog, showStats: true })
        )
      ),
    ]);
  }

  /**
   * Build patterns layout
   * @private
   */
  _buildPatternsLayout(data) {
    const patterns = data.patterns || [];
    return this.createComponent(ComponentType.Container, { className: 'patterns-layout' }, [
      this.createComponent(ComponentType.Heading, { level: 2, text: 'Detected Patterns' }),
      this.createComponent(ComponentType.List, {
        items: patterns.map(p => `${p.category}: ${p.total} occurrences`),
      }),
    ]);
  }

  /**
   * Build dashboard layout
   * @private
   */
  _buildDashboardLayout(data) {
    return this.createComponent(ComponentType.Container, { className: 'dashboard' }, [
      this.createComponent(ComponentType.Row, { gap: '2rem' }, [
        this.createComponent(ComponentType.Column, { span: 1 }, [
          this.createComponent(ComponentType.Card, { title: 'Health' }, [
            this.createComponent(ComponentType.Metric, { label: 'Status', value: data.status || 'Unknown' }),
            this.createComponent(ComponentType.Metric, { label: 'Uptime', value: data.uptime || 0, unit: 's' }),
          ]),
        ]),
        this.createComponent(ComponentType.Column, { span: 2 }, [
          this.createComponent(ComponentType.Card, { title: 'Judgments' }, [
            this.createComponent(ComponentType.Metric, { label: 'Total', value: data.judgments || 0 }),
            this.createComponent(ComponentType.Metric, { label: 'Avg Q', value: (data.avgScore || 0).toFixed(1) }),
          ]),
        ]),
      ]),
    ]);
  }

  /**
   * Escape HTML special characters
   * @private
   */
  _escapeHtml(str) {
    if (typeof str !== 'string') return String(str);
    return str
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }
}

/**
 * Create JSON render tool for MCP
 * @param {JSONRenderService} renderer - Renderer instance
 * @returns {Object} Tool definition
 */
export function createJSONRenderTool(renderer) {
  return {
    name: 'brain_render',
    description: 'Render CYNIC data to HTML using the component catalog. Supports layouts: judgment, chain, dogs, patterns, dashboard.',
    inputSchema: {
      type: 'object',
      properties: {
        layout: {
          type: 'string',
          enum: ['judgment', 'chain', 'dogs', 'patterns', 'dashboard', 'custom'],
          description: 'Layout type to render',
        },
        data: {
          type: 'object',
          description: 'Data to render',
        },
        tree: {
          type: 'object',
          description: 'Custom component tree (for layout=custom)',
        },
      },
      required: ['layout'],
    },
    handler: async (params) => {
      const { layout, data = {}, tree } = params;

      if (layout === 'custom' && tree) {
        const validation = renderer.validate(tree);
        if (!validation.valid) {
          return { error: validation.errors, valid: false };
        }
        const html = renderer.render(tree, data);
        return { html, nodeCount: validation.nodeCount };
      }

      const componentTree = renderer.buildLayout(layout, data);
      const html = renderer.render(componentTree, data);

      return {
        layout,
        html,
        tree: componentTree,
        stats: renderer.getStats(),
      };
    },
  };
}

export default JSONRenderService;
