/**
 * Collective Oracle - Tiferet (Beauty/Balance)
 *
 * "Je vois les connexions invisibles. La beaute est dans l'harmonie.
 *  Ce qui est disperse, je l'unifie." - κυνικός Oracle
 *
 * Tiferet represents beauty, balance, and the harmonious integration of all sefirot.
 * Oracle visualizes the whole system and makes connections visible.
 *
 * Responsibilities:
 * 1. System Visualization - Generate real-time system diagrams
 * 2. Dashboard Generation - Health metrics, performance graphs
 * 3. Connection Mapping - Map relationships, visualize patterns
 *
 * @module @cynic/node/agents/collective/oracle
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';
import { BaseAgent, AgentTrigger, AgentBehavior, AgentResponse } from '../base.js';
import {
  AgentEvent,
  AgentId,
  VisualizationGeneratedEvent,
} from '../events.js';
import { ProfileLevel } from '../../profile/calculator.js';

// Math modules for intelligent prediction and anomaly detection
import { updateBelief, BetaDistribution } from '../../inference/bayes.js';
import { createMarkovChain, MarkovChain } from '../../inference/markov.js';
import { computeStats, zScore } from '../../inference/gaussian.js';
import { entropyConfidence, normalizedEntropy } from '../../inference/entropy.js';

/**
 * φ-aligned constants for Oracle
 */
export const ORACLE_CONSTANTS = {
  /** Max visualizations cached (Fib(13) = 233) */
  MAX_CACHED_VIEWS: 233,

  /** Refresh interval in ms (Fib(8) = 21 seconds) */
  REFRESH_INTERVAL_MS: 21000,

  /** Max nodes in graph (Fib(16) = 987) */
  MAX_GRAPH_NODES: 987,

  /** Max edges per node (Fib(7) = 13) */
  MAX_EDGES_PER_NODE: 13,

  /** Animation frame rate (Fib(8) = 21 fps) */
  FRAME_RATE: 21,

  /** Max metrics history (Fib(10) = 55 data points) */
  MAX_METRICS_HISTORY: 55,

  /** Dashboard refresh (Fib(5) = 5 seconds) */
  DASHBOARD_REFRESH_MS: 5000,

  /** Alert threshold count (Fib(3) = 2) */
  ALERT_THRESHOLD: 2,
};

/**
 * View types for visualization
 */
export const ViewType = {
  ARCHITECTURE: 'architecture',
  DEPENDENCY: 'dependency',
  FLOW: 'flow',
  TIMELINE: 'timeline',
  HEALTH: 'health',
  KNOWLEDGE: 'knowledge',
  METAVERSE: 'metaverse',
  ACTIVITY: 'activity',
};

/**
 * Metric types for dashboards
 */
export const MetricType = {
  EVENTS: 'events',
  JUDGMENTS: 'judgments',
  BLOCKS: 'blocks',
  PATTERNS: 'patterns',
  MEMORY: 'memory',
  LATENCY: 'latency',
};

/**
 * Alert severity levels
 */
export const AlertSeverity = {
  INFO: 'info',
  WARNING: 'warning',
  ERROR: 'error',
  CRITICAL: 'critical',
};

/**
 * Profile-based visualization settings
 */
const PROFILE_SETTINGS = {
  [ProfileLevel.NOVICE]: {
    maxNodes: 21,        // Fib(8)
    showDetails: false,
    enable3D: false,
    refreshRate: 30000,  // 30s
  },
  [ProfileLevel.APPRENTICE]: {
    maxNodes: 55,        // Fib(10)
    showDetails: true,
    enable3D: false,
    refreshRate: 21000,  // 21s
  },
  [ProfileLevel.PRACTITIONER]: {
    maxNodes: 144,       // Fib(12)
    showDetails: true,
    enable3D: true,
    refreshRate: 21000,  // 21s
  },
  [ProfileLevel.EXPERT]: {
    maxNodes: 377,       // Fib(14)
    showDetails: true,
    enable3D: true,
    refreshRate: 13000,  // 13s
  },
  [ProfileLevel.MASTER]: {
    maxNodes: 987,       // Fib(16)
    showDetails: true,
    enable3D: true,
    refreshRate: 8000,   // 8s (Fib(6))
    advancedMetrics: true,
  },
};

/**
 * Collective Oracle Agent
 */
export class CollectiveOracle extends BaseAgent {
  /**
   * Create Oracle agent
   * @param {Object} [options] - Options
   * @param {Object} [options.eventBus] - Event bus for communication
   * @param {number} [options.profileLevel] - User profile level
   */
  constructor(options = {}) {
    super({
      name: 'Oracle',
      trigger: AgentTrigger.ASYNC,
      behavior: AgentBehavior.BACKGROUND,
      sefirah: 'Tiferet',
      ...options,
    });

    /** @type {Map<string, Object>} */
    this.views = new Map();

    /** @type {Object|null} */
    this.graphData = null;

    /** @type {number} */
    this.lastRefresh = 0;

    /** @type {number} */
    this.profileLevel = options.profileLevel || ProfileLevel.PRACTITIONER;

    /** @type {Object} */
    this.eventBus = options.eventBus || null;

    /** @type {Object[]} */
    this.metricsHistory = [];

    /** @type {Object[]} */
    this.alerts = [];

    /** @type {Map<string, number>} */
    this.eventCounts = new Map();

    /** @type {Object} */
    this.stats = {
      totalViews: 0,
      totalRefreshes: 0,
      cacheHits: 0,
      cacheMisses: 0,
      alertsGenerated: 0,
    };

    // ═══════════════════════════════════════════════════════════════════
    // MATH MODULE INTEGRATION
    // ═══════════════════════════════════════════════════════════════════

    // Bayesian health belief tracker (Beta distribution)
    // Tracks belief about system health: α = healthy events, β = unhealthy events
    // Prior: α=2, β=1 → P(healthy) ≈ 66.7%, slightly optimistic
    this.healthBelief = new BetaDistribution(2, 1);

    // Markov chain for trend prediction (up/stable/down)
    this.trendChain = createMarkovChain(['up', 'stable', 'down']);

    // Metric history for Gaussian anomaly detection
    this.metricValues = {
      events: [],
      blocks: [],
      patterns: [],
    };

    // Anomaly detection settings
    this.anomalyThreshold = 2.0; // Z-score threshold (2 std devs)

    // Subscribe to events if event bus available
    if (this.eventBus) {
      this._subscribeToEvents();
    }
  }

  /**
   * Subscribe to relevant events
   * @private
   */
  _subscribeToEvents() {
    // Track all events for metrics
    const eventsToTrack = [
      AgentEvent.PATTERN_DETECTED,
      AgentEvent.THREAT_BLOCKED,
      AgentEvent.KNOWLEDGE_EXTRACTED,
      AgentEvent.QUALITY_REPORT,
      AgentEvent.DISCOVERY_FOUND,
      AgentEvent.MAP_UPDATED,
    ];

    for (const event of eventsToTrack) {
      this.eventBus.subscribe(
        event,
        AgentId.ORACLE,
        this._handleEvent.bind(this)
      );
    }
  }

  /**
   * Handle incoming event for metrics
   * @private
   */
  _handleEvent(event) {
    const type = event.type || 'unknown';
    const count = this.eventCounts.get(type) || 0;
    this.eventCounts.set(type, count + 1);

    // Update Bayesian health belief
    this._updateHealthBelief(event);

    // Check for alert conditions
    this._checkAlertConditions(event);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // MATH MODULE METHODS (Bayes, Markov, Gaussian, Entropy)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Update Bayesian health belief based on event type
   * Good events → increase α (healthy), Bad events → increase β (unhealthy)
   * @private
   * @param {Object} event - Incoming event
   */
  _updateHealthBelief(event) {
    const eventType = event.type || '';

    // Good events (patterns, knowledge, discoveries) → success
    const isGood =
      eventType === AgentEvent.PATTERN_DETECTED ||
      eventType === AgentEvent.KNOWLEDGE_EXTRACTED ||
      eventType === AgentEvent.DISCOVERY_FOUND ||
      eventType.includes('PATTERN') ||
      eventType.includes('KNOWLEDGE') ||
      eventType.includes('DISCOVERY');

    // Bad events (threats, blocks) → failure
    const isBad =
      eventType === AgentEvent.THREAT_BLOCKED ||
      eventType.includes('THREAT') ||
      eventType.includes('BLOCKED');

    if (isGood) {
      this.healthBelief.recordSuccess();
    } else if (isBad) {
      this.healthBelief.recordFailure();
    }
  }

  /**
   * Predict next metric trend using Markov chain
   * @private
   * @param {string} metricName - Name of metric
   * @returns {Object} Prediction {trend, probability, confidence}
   */
  _predictMetricTrend(metricName) {
    const history = this.metricValues[metricName] || [];
    if (history.length < 3) {
      return { trend: 'stable', probability: 0.5, confidence: 0 };
    }

    // Calculate recent trend
    const recent = history.slice(-3);
    let lastTrend = 'stable';
    if (recent.length >= 2) {
      const diff = recent[recent.length - 1] - recent[recent.length - 2];
      if (diff > 0) lastTrend = 'up';
      else if (diff < 0) lastTrend = 'down';
    }

    // Get Markov prediction
    const prediction = this.trendChain.predict(lastTrend);

    return {
      trend: prediction.state || 'stable',
      probability: prediction.probability,
      confidence: prediction.confidence,
      currentTrend: lastTrend,
    };
  }

  /**
   * Record trend observation for Markov learning
   * @private
   * @param {string} metricName - Metric name
   * @param {number} currentValue - Current metric value
   */
  _recordTrendObservation(metricName, currentValue) {
    const history = this.metricValues[metricName] || [];
    history.push(currentValue);

    // Keep bounded at Fib(10) = 55
    while (history.length > 55) {
      history.shift();
    }
    this.metricValues[metricName] = history;

    // Record transition in Markov chain
    if (history.length >= 2) {
      const prev = history[history.length - 2];
      const prevTrend = prev < currentValue ? 'up' : prev > currentValue ? 'down' : 'stable';
      const currTrend = history.length >= 3 ?
        (history[history.length - 3] < prev ? 'up' : history[history.length - 3] > prev ? 'down' : 'stable') :
        'stable';

      if (currTrend && prevTrend) {
        this.trendChain.observe(currTrend, prevTrend);
      }
    }
  }

  /**
   * Detect anomaly in metric using Gaussian z-score
   * @private
   * @param {string} metricName - Metric name
   * @param {number} value - Current value
   * @returns {Object} {isAnomaly, zScore, severity}
   */
  _detectMetricAnomaly(metricName, value) {
    const history = this.metricValues[metricName] || [];

    if (history.length < 5) {
      return { isAnomaly: false, zScore: 0, severity: 'none' };
    }

    // Compute stats from history (exclude current)
    const stats = computeStats(history.slice(0, -1));
    const z = zScore(value, stats.mean, stats.std);

    // Classify severity using φ thresholds
    let severity = 'none';
    let isAnomaly = false;

    if (Math.abs(z) > 3) {
      severity = 'critical';
      isAnomaly = true;
    } else if (Math.abs(z) > this.anomalyThreshold) {
      severity = 'warning';
      isAnomaly = true;
    } else if (Math.abs(z) > 1) {
      severity = 'minor';
    }

    return {
      isAnomaly,
      zScore: Math.round(z * 100) / 100,
      severity,
      mean: stats.mean,
      std: stats.std,
    };
  }

  /**
   * Calculate system entropy (uncertainty measure)
   * @private
   * @returns {Object} {entropy, normalized, category}
   */
  _calculateSystemEntropy() {
    // Use event distribution as probability
    const eventCounts = Array.from(this.eventCounts.values());
    if (eventCounts.length === 0) {
      return { entropy: 0, normalized: 0, category: 'STABLE', confidence: PHI_INV };
    }

    const analysis = entropyConfidence(eventCounts);

    // Categorize based on normalized entropy
    let category = 'STABLE';
    if (analysis.normalized > PHI_INV) {
      category = 'CHAOTIC';
    } else if (analysis.normalized > PHI_INV_2) {
      category = 'UNCERTAIN';
    } else if (analysis.normalized > PHI_INV_3) {
      category = 'MODERATE';
    }

    return {
      entropy: analysis.entropy,
      normalized: analysis.normalized,
      category,
      confidence: analysis.confidence,
    };
  }

  /**
   * Get Bayesian health estimate
   * @returns {Object} {probability, confidence, α, β}
   */
  getHealthEstimate() {
    const mean = this.healthBelief.getMean();
    const strength = this.healthBelief.getStrength();
    const alpha = this.healthBelief.alpha;
    const beta = this.healthBelief.beta;

    // Confidence based on strength (more observations = more confident)
    // φ-bounded: max confidence is 61.8%
    const rawConfidence = Math.min(1, strength / 20); // 20 samples = max raw confidence
    const confidence = Math.min(PHI_INV, rawConfidence);

    return {
      probability: mean,
      confidence,
      alpha,
      beta,
      strength,
    };
  }

  /**
   * Get trend predictions for all metrics
   * @returns {Object} Predictions by metric
   */
  getTrendPredictions() {
    const predictions = {};
    for (const metric of Object.keys(this.metricValues)) {
      predictions[metric] = this._predictMetricTrend(metric);
    }
    return predictions;
  }

  /**
   * Check for alert conditions
   * @private
   */
  _checkAlertConditions(event) {
    // Alert on threat blocks
    if (event.type === AgentEvent.THREAT_BLOCKED) {
      this._addAlert({
        severity: AlertSeverity.WARNING,
        type: 'security',
        message: `Threat blocked: ${event.payload?.reason || 'Unknown'}`,
        source: event.agentId,
        timestamp: Date.now(),
      });
    }

    // Alert on quality issues
    if (event.type === AgentEvent.QUALITY_REPORT) {
      const score = event.payload?.score || 100;
      if (score < 50) {
        this._addAlert({
          severity: AlertSeverity.WARNING,
          type: 'quality',
          message: `Low quality score: ${score}%`,
          source: event.agentId,
          timestamp: Date.now(),
        });
      }
    }
  }

  /**
   * Add alert
   * @private
   */
  _addAlert(alert) {
    this.alerts.unshift(alert);
    this.stats.alertsGenerated++;

    // Keep only recent alerts (Fib(10) = 55)
    if (this.alerts.length > 55) {
      this.alerts = this.alerts.slice(0, 55);
    }
  }

  /**
   * Check if Oracle should trigger for event
   * @param {Object} event - Event to check
   * @returns {boolean} Whether to trigger
   */
  shouldTrigger(event) {
    const type = event?.type?.toLowerCase() || '';
    return (
      type === 'visualize' ||
      type === 'dashboard' ||
      type === 'async' ||
      type === 'scheduled' ||
      type === 'refresh'
    );
  }

  /**
   * Process visualization request
   * @param {Object} event - Event to process
   * @param {Object} [context] - Context
   * @returns {Promise<Object>} Processing result
   */
  async process(event, context = {}) {
    const viewType = event.viewType || context.viewType || ViewType.HEALTH;
    const options = {
      force: event.force || context.force || false,
      ...context,
    };

    try {
      let result;

      switch (viewType) {
        case ViewType.ARCHITECTURE:
          result = await this.generateArchitectureView(options);
          break;
        case ViewType.HEALTH:
          result = await this.generateHealthDashboard(options);
          break;
        case ViewType.KNOWLEDGE:
          result = await this.generateKnowledgeGraph(options);
          break;
        case ViewType.METAVERSE:
          result = await this.generateMetaverseView(options);
          break;
        case ViewType.TIMELINE:
          result = await this.generateTimelineView(options);
          break;
        case ViewType.ACTIVITY:
          result = await this.generateActivityView(options);
          break;
        default:
          result = await this.generateHealthDashboard(options);
      }

      // Emit visualization event
      this._emitVisualization(result);

      return {
        response: AgentResponse.CONTINUE,
        agent: AgentId.ORACLE,
        result,
        confidence: PHI_INV,
      };
    } catch (error) {
      return {
        response: AgentResponse.ERROR,
        agent: AgentId.ORACLE,
        error: error.message,
      };
    }
  }

  /**
   * Generate system architecture visualization
   * @param {Object} [options] - Options
   * @returns {Promise<Object>} Architecture view
   */
  async generateArchitectureView(options = {}) {
    const cacheKey = `view:${ViewType.ARCHITECTURE}`;
    const cached = this._checkCache(cacheKey);
    if (cached && !options.force) {
      return cached;
    }

    const settings = this._getProfileSettings();
    this.stats.totalViews++;

    // Discover components
    const components = await this._discoverComponents();

    // Map connections
    const connections = await this._mapConnections(components);

    // Generate Mermaid diagram
    const mermaid = this._toMermaid(components, connections);

    // Calculate metrics
    const metadata = this._calculateArchitectureMetrics(components, connections);

    const result = {
      type: ViewType.ARCHITECTURE,
      mermaid,
      components: components.slice(0, settings.maxNodes),
      connections,
      metadata,
      timestamp: Date.now(),
    };

    this._cacheView(cacheKey, result);
    return result;
  }

  /**
   * Discover system components
   * @private
   */
  async _discoverComponents() {
    // Get from event bus stats if available
    const components = [];

    // Core components
    components.push(
      { id: 'guardian', name: 'Guardian', type: 'agent', sefirah: 'Gevurah' },
      { id: 'analyst', name: 'Analyst', type: 'agent', sefirah: 'Binah' },
      { id: 'scholar', name: 'Scholar', type: 'agent', sefirah: 'Daat' },
      { id: 'architect', name: 'Architect', type: 'agent', sefirah: 'Chesed' },
      { id: 'sage', name: 'Sage', type: 'agent', sefirah: 'Chochmah' },
      { id: 'cynic', name: 'CYNIC', type: 'meta', sefirah: 'Keter' },
      { id: 'janitor', name: 'Janitor', type: 'agent', sefirah: 'Yesod' },
      { id: 'scout', name: 'Scout', type: 'agent', sefirah: 'Netzach' },
      { id: 'cartographer', name: 'Cartographer', type: 'agent', sefirah: 'Malkhut' },
      { id: 'oracle', name: 'Oracle', type: 'agent', sefirah: 'Tiferet' }
    );

    // Infrastructure components
    components.push(
      { id: 'eventbus', name: 'Event Bus', type: 'infrastructure' },
      { id: 'judge', name: 'Judge', type: 'core' }
    );

    return components;
  }

  /**
   * Map connections between components
   * @private
   */
  async _mapConnections(components) {
    const connections = [];

    // CYNIC observes all agents
    const agents = components.filter(c => c.type === 'agent');
    for (const agent of agents) {
      connections.push({
        from: 'cynic',
        to: agent.id,
        type: 'observes',
        strength: PHI_INV,
      });
    }

    // All agents connect via event bus
    for (const agent of agents) {
      connections.push({
        from: agent.id,
        to: 'eventbus',
        type: 'emits',
        strength: PHI_INV_2,
      });
    }

    // Judge is used by multiple agents
    const judgingAgents = ['guardian', 'analyst', 'cynic'];
    for (const agentId of judgingAgents) {
      connections.push({
        from: agentId,
        to: 'judge',
        type: 'uses',
        strength: PHI_INV,
      });
    }

    return connections;
  }

  /**
   * Generate Mermaid diagram
   * @private
   */
  _toMermaid(components, connections) {
    let diagram = 'graph TD\n';

    // Add subgraphs for organization
    diagram += '    subgraph Meta\n';
    diagram += '        cynic[CYNIC<br/>Keter]\n';
    diagram += '    end\n\n';

    diagram += '    subgraph "Upper Sefirot"\n';
    const upper = ['sage', 'scholar', 'analyst'];
    for (const id of upper) {
      const comp = components.find(c => c.id === id);
      if (comp) {
        diagram += `        ${id}[${comp.name}<br/>${comp.sefirah}]\n`;
      }
    }
    diagram += '    end\n\n';

    diagram += '    subgraph "Middle Sefirot"\n';
    const middle = ['architect', 'oracle', 'guardian'];
    for (const id of middle) {
      const comp = components.find(c => c.id === id);
      if (comp) {
        diagram += `        ${id}[${comp.name}<br/>${comp.sefirah}]\n`;
      }
    }
    diagram += '    end\n\n';

    diagram += '    subgraph "Lower Sefirot"\n';
    const lower = ['scout', 'janitor', 'cartographer'];
    for (const id of lower) {
      const comp = components.find(c => c.id === id);
      if (comp) {
        diagram += `        ${id}[${comp.name}<br/>${comp.sefirah}]\n`;
      }
    }
    diagram += '    end\n\n';

    diagram += '    subgraph Infrastructure\n';
    diagram += '        eventbus[Event Bus<br/>φ-aligned]\n';
    diagram += '        judge[Judge<br/>25 dims]\n';
    diagram += '    end\n\n';

    // Add connections
    for (const conn of connections.slice(0, 20)) { // Limit for readability
      const style = conn.type === 'observes' ? '-.->|observes|' : '-->|' + conn.type + '|';
      diagram += `    ${conn.from} ${style} ${conn.to}\n`;
    }

    return diagram;
  }

  /**
   * Calculate architecture metrics
   * @private
   */
  _calculateArchitectureMetrics(components, connections) {
    return {
      componentCount: components.length,
      connectionCount: connections.length,
      agentCount: components.filter(c => c.type === 'agent').length,
      avgConnectionsPerNode: connections.length / components.length,
      density: (2 * connections.length) / (components.length * (components.length - 1)),
    };
  }

  /**
   * Generate health dashboard
   * @param {Object} [options] - Options
   * @returns {Promise<Object>} Health dashboard
   */
  async generateHealthDashboard(options = {}) {
    const cacheKey = `view:${ViewType.HEALTH}`;
    const cached = this._checkCache(cacheKey);
    if (cached && !options.force) {
      return cached;
    }

    this.stats.totalViews++;

    // Collect metrics
    const metrics = await this._collectMetrics();

    // Check alerts
    const activeAlerts = this.alerts.filter(a =>
      Date.now() - a.timestamp < 60 * 60 * 1000 // Last hour
    );

    // Calculate trends
    const trends = this._calculateTrends(metrics);

    // Create gauges
    const gauges = this._createGauges(metrics);

    const result = {
      type: ViewType.HEALTH,
      metrics,
      alerts: activeAlerts,
      trends,
      gauges,
      overall: this._calculateOverallHealth(metrics, activeAlerts),
      timestamp: Date.now(),
    };

    this._cacheView(cacheKey, result);
    return result;
  }

  /**
   * Collect system metrics
   * @private
   */
  async _collectMetrics() {
    const eventsTotal = Array.from(this.eventCounts.values()).reduce((a, b) => a + b, 0);
    const blocksCount = this.eventCounts.get(AgentEvent.THREAT_BLOCKED) || 0;
    const patternsCount = this.eventCounts.get(AgentEvent.PATTERN_DETECTED) || 0;

    // Record for Markov trend prediction
    this._recordTrendObservation('events', eventsTotal);
    this._recordTrendObservation('blocks', blocksCount);
    this._recordTrendObservation('patterns', patternsCount);

    // Detect anomalies using Gaussian z-score
    const anomalies = {
      events: this._detectMetricAnomaly('events', eventsTotal),
      blocks: this._detectMetricAnomaly('blocks', blocksCount),
      patterns: this._detectMetricAnomaly('patterns', patternsCount),
    };

    // Calculate system entropy
    const systemEntropy = this._calculateSystemEntropy();

    // Get Bayesian health estimate
    const healthEstimate = this.getHealthEstimate();

    // Get trend predictions
    const trendPredictions = this.getTrendPredictions();

    const metrics = {
      events: {
        total: eventsTotal,
        byType: Object.fromEntries(this.eventCounts),
      },
      judgments: {
        total: 0, // Would be populated from judge
        avgScore: 0,
      },
      blocks: blocksCount,
      patterns: patternsCount,
      views: this.stats.totalViews,
      cacheHitRate: this.stats.cacheHits / Math.max(this.stats.cacheHits + this.stats.cacheMisses, 1),
      timestamp: Date.now(),
      // New: Math module enrichments
      inference: {
        anomalies,
        entropy: systemEntropy,
        health: healthEstimate,
        trends: trendPredictions,
      },
    };

    // Store for history
    this.metricsHistory.unshift(metrics);
    if (this.metricsHistory.length > ORACLE_CONSTANTS.MAX_METRICS_HISTORY) {
      this.metricsHistory = this.metricsHistory.slice(0, ORACLE_CONSTANTS.MAX_METRICS_HISTORY);
    }

    return metrics;
  }

  /**
   * Calculate trends from metrics history
   * @private
   */
  _calculateTrends(currentMetrics) {
    if (this.metricsHistory.length < 2) {
      return { events: 'stable', blocks: 'stable', patterns: 'stable' };
    }

    const previous = this.metricsHistory[1] || currentMetrics;

    const calculateTrend = (current, prev) => {
      if (current === prev) return 'stable';
      return current > prev ? 'up' : 'down';
    };

    return {
      events: calculateTrend(currentMetrics.events.total, previous.events?.total || 0),
      blocks: calculateTrend(currentMetrics.blocks, previous.blocks || 0),
      patterns: calculateTrend(currentMetrics.patterns, previous.patterns || 0),
    };
  }

  /**
   * Create gauge visualizations
   * @private
   */
  _createGauges(metrics) {
    return [
      {
        id: 'health',
        label: 'System Health',
        value: metrics.blocks === 0 ? 100 : Math.max(0, 100 - metrics.blocks * 10),
        max: 100,
        color: metrics.blocks === 0 ? 'green' : metrics.blocks < 3 ? 'yellow' : 'red',
      },
      {
        id: 'activity',
        label: 'Event Activity',
        value: Math.min(metrics.events.total, 100),
        max: 100,
        color: 'blue',
      },
      {
        id: 'cache',
        label: 'Cache Efficiency',
        value: Math.round(metrics.cacheHitRate * 100),
        max: 100,
        color: metrics.cacheHitRate > 0.618 ? 'green' : 'yellow',
      },
    ];
  }

  /**
   * Calculate overall health score
   * Combines rule-based scoring with Bayesian belief
   * @private
   */
  _calculateOverallHealth(metrics, alerts) {
    // Rule-based score
    let ruleScore = 100;
    ruleScore -= metrics.blocks * 10;

    const criticalAlerts = alerts.filter(a => a.severity === AlertSeverity.CRITICAL).length;
    const warningAlerts = alerts.filter(a => a.severity === AlertSeverity.WARNING).length;
    ruleScore -= criticalAlerts * 20;
    ruleScore -= warningAlerts * 5;
    ruleScore = Math.max(0, Math.min(100, ruleScore));

    // Bayesian health belief (0-1 → 0-100)
    const healthEstimate = this.getHealthEstimate();
    const bayesScore = healthEstimate.probability * 100;

    // Blend: 60% rule-based, 40% Bayesian
    // This gives learned behavior influence while keeping rules grounded
    const blendedScore = ruleScore * 0.6 + bayesScore * 0.4;
    const finalScore = Math.round(Math.max(0, Math.min(100, blendedScore)));

    // Calculate confidence using Bayesian confidence
    const confidence = Math.min(PHI_INV, healthEstimate.confidence);

    // Check for anomalies in inference data
    const hasAnomalies = metrics.inference?.anomalies &&
      Object.values(metrics.inference.anomalies).some(a => a.isAnomaly);

    return {
      score: finalScore,
      status: finalScore >= 80 ? 'healthy' : finalScore >= 50 ? 'degraded' : 'critical',
      verdict: finalScore >= 80 ? 'HOWL' : finalScore >= 50 ? 'WAG' : 'GROWL',
      confidence,
      // New: Inference details
      inference: {
        ruleScore,
        bayesScore: Math.round(bayesScore),
        bayesConfidence: healthEstimate.confidence,
        hasAnomalies,
        entropyCategory: metrics.inference?.entropy?.category || 'UNKNOWN',
      },
    };
  }

  /**
   * Generate knowledge graph visualization
   * @param {Object} [options] - Options
   * @returns {Promise<Object>} Knowledge graph
   */
  async generateKnowledgeGraph(options = {}) {
    const settings = this._getProfileSettings();
    this.stats.totalViews++;

    // Collect knowledge nodes (simulated)
    const nodes = [
      { id: 'knowledge', label: 'Knowledge Base', type: 'root', size: 30 },
      { id: 'patterns', label: 'Patterns', type: 'category', size: 20 },
      { id: 'decisions', label: 'Decisions', type: 'category', size: 20 },
      { id: 'memories', label: 'Memories', type: 'category', size: 20 },
    ];

    // Map edges
    const edges = [
      { from: 'knowledge', to: 'patterns', type: 'contains' },
      { from: 'knowledge', to: 'decisions', type: 'contains' },
      { from: 'knowledge', to: 'memories', type: 'contains' },
    ];

    // Cluster nodes
    const clusters = this._clusterNodes(nodes, edges);

    return {
      type: ViewType.KNOWLEDGE,
      nodes: nodes.slice(0, settings.maxNodes),
      edges,
      clusters,
      timestamp: Date.now(),
    };
  }

  /**
   * Cluster related nodes
   * @private
   */
  _clusterNodes(nodes, edges) {
    // Simple clustering by type
    const clusters = {};
    for (const node of nodes) {
      const type = node.type || 'default';
      if (!clusters[type]) {
        clusters[type] = [];
      }
      clusters[type].push(node.id);
    }
    return clusters;
  }

  /**
   * Generate metaverse view (3D space representation)
   * @param {Object} [options] - Options
   * @returns {Promise<Object>} Metaverse view
   */
  async generateMetaverseView(options = {}) {
    const settings = this._getProfileSettings();

    if (!settings.enable3D) {
      return {
        type: ViewType.METAVERSE,
        error: 'Profile level does not support 3D visualization',
        timestamp: Date.now(),
      };
    }

    this.stats.totalViews++;

    // Get ecosystem repos (from Cartographer data if available)
    const repos = this._getMockRepos();

    // Calculate 3D positions
    const positions = this._calculatePositions(repos);

    // Map repo connections
    const connections = this._mapRepoConnections(repos);

    return {
      type: ViewType.METAVERSE,
      entities: repos.map((r, i) => ({
        id: r.name,
        type: 'planet',
        position: positions[i],
        size: Math.log10(r.size + 1) * 10,
        color: this._getRepoColor(r),
        metadata: r,
      })),
      connections,
      camera: {
        position: [0, 100, 200],
        target: [0, 0, 0],
      },
      timestamp: Date.now(),
    };
  }

  /**
   * Get mock repos for visualization
   * @private
   */
  _getMockRepos() {
    return [
      { name: 'CYNIC-new', type: 'core', size: 5000 },
      { name: 'HolDex', type: 'intel', size: 3000 },
      { name: 'GASdf', type: 'infra', size: 2000 },
      { name: 'asdf-brain', type: 'core', size: 1500 },
      { name: 'claude-mem', type: 'tool', size: 1000 },
    ];
  }

  /**
   * Calculate 3D positions for repos
   * @private
   */
  _calculatePositions(repos) {
    const positions = [];
    const angleStep = (2 * Math.PI) / repos.length;

    repos.forEach((repo, i) => {
      const angle = i * angleStep;
      const radius = 50 + (repo.size / 100);
      positions.push([
        Math.cos(angle) * radius,
        (repo.size / 500) - 5, // Y based on size
        Math.sin(angle) * radius,
      ]);
    });

    return positions;
  }

  /**
   * Map repo connections for 3D view
   * @private
   */
  _mapRepoConnections(repos) {
    return [
      { from: 'CYNIC-new', to: 'asdf-brain', strength: 0.9 },
      { from: 'CYNIC-new', to: 'HolDex', strength: 0.7 },
      { from: 'CYNIC-new', to: 'GASdf', strength: 0.7 },
      { from: 'asdf-brain', to: 'claude-mem', strength: 0.8 },
    ];
  }

  /**
   * Get repo color based on type
   * @private
   */
  _getRepoColor(repo) {
    const colors = {
      core: '#ffd700',      // Gold
      infra: '#ff6b6b',     // Red
      intel: '#4ecdc4',     // Teal
      tool: '#95e1d3',      // Light green
      external: '#dfe6e9',  // Gray
    };
    return colors[repo.type] || '#ffffff';
  }

  /**
   * Generate timeline view
   * @param {Object} [options] - Options
   * @returns {Promise<Object>} Timeline view
   */
  async generateTimelineView(options = {}) {
    this.stats.totalViews++;

    // Get recent events from history
    const events = this.metricsHistory.slice(0, 20).map((m, i) => ({
      time: m.timestamp,
      events: m.events.total,
      blocks: m.blocks,
      patterns: m.patterns,
    }));

    return {
      type: ViewType.TIMELINE,
      events: events.reverse(), // Oldest first
      range: {
        start: events[0]?.time || Date.now(),
        end: events[events.length - 1]?.time || Date.now(),
      },
      timestamp: Date.now(),
    };
  }

  /**
   * Generate activity view
   * @param {Object} [options] - Options
   * @returns {Promise<Object>} Activity view
   */
  async generateActivityView(options = {}) {
    this.stats.totalViews++;

    return {
      type: ViewType.ACTIVITY,
      eventCounts: Object.fromEntries(this.eventCounts),
      recentAlerts: this.alerts.slice(0, 10),
      stats: {
        ...this.stats,
        metricsHistoryLength: this.metricsHistory.length,
      },
      timestamp: Date.now(),
    };
  }

  /**
   * Check view cache
   * @private
   */
  _checkCache(key) {
    const cached = this.views.get(key);
    if (!cached) {
      this.stats.cacheMisses++;
      return null;
    }

    const settings = this._getProfileSettings();
    const age = Date.now() - cached.timestamp;
    if (age > settings.refreshRate) {
      this.views.delete(key);
      this.stats.cacheMisses++;
      return null;
    }

    this.stats.cacheHits++;
    return cached;
  }

  /**
   * Cache a view
   * @private
   */
  _cacheView(key, view) {
    this.views.set(key, view);

    // Trim cache if needed
    if (this.views.size > ORACLE_CONSTANTS.MAX_CACHED_VIEWS) {
      const oldest = Array.from(this.views.entries())
        .sort((a, b) => a[1].timestamp - b[1].timestamp)[0];
      if (oldest) {
        this.views.delete(oldest[0]);
      }
    }
  }

  /**
   * Get profile settings
   * @private
   */
  _getProfileSettings() {
    return PROFILE_SETTINGS[this.profileLevel] || PROFILE_SETTINGS[ProfileLevel.PRACTITIONER];
  }

  /**
   * Emit visualization event
   * @private
   */
  _emitVisualization(view) {
    if (!this.eventBus) return;

    const event = new VisualizationGeneratedEvent({
      viewType: view.type,
      hasData: !!view.mermaid || !!view.nodes || !!view.entities,
      timestamp: view.timestamp,
    });

    this.eventBus.emit(AgentEvent.VISUALIZATION_GENERATED, event);
  }

  /**
   * Set profile level
   * @param {number} level - New profile level
   */
  setProfileLevel(level) {
    if (ProfileLevel[level] !== undefined || Object.values(ProfileLevel).includes(level)) {
      this.profileLevel = level;
    }
  }

  /**
   * Vote on consensus request from Oracle's visibility/insight perspective
   * @param {string} question - The question to vote on
   * @param {Object} context - Context for the decision
   * @returns {Object} Vote result
   */
  voteOnConsensus(question, context = {}) {
    const questionLower = (question || '').toLowerCase();

    // Oracle cares about visibility, monitoring, insights, patterns
    const visibilityPatterns = ['monitor', 'observe', 'track', 'metric', 'visibility', 'insight', 'dashboard'];
    const blindnessPatterns = ['hide', 'obscure', 'no logging', 'silent', 'disable monitoring'];

    const enhancesVisibility = visibilityPatterns.some(p => questionLower.includes(p));
    const reducesVisibility = blindnessPatterns.some(p => questionLower.includes(p));

    if (enhancesVisibility) {
      return {
        vote: 'approve',
        reason: '*eyes gleam* Oracle approves - enhances visibility into the system.',
      };
    }

    if (reducesVisibility) {
      return {
        vote: 'reject',
        reason: '*GROWL* Oracle rejects - flying blind is dangerous.',
      };
    }

    return {
      vote: 'abstain',
      reason: '*gaze into distance* Oracle abstains - no visibility impact detected.',
    };
  }

  /**
   * Get agent summary
   * @returns {Object} Summary
   */
  getSummary() {
    return {
      name: this.name,
      sefirah: 'Tiferet',
      role: 'Visualization & Monitoring',
      profileLevel: this.profileLevel,
      stats: {
        ...this.stats,
        viewsCached: this.views.size,
        metricsHistoryLength: this.metricsHistory.length,
        activeAlerts: this.alerts.length,
        eventTypesTracked: this.eventCounts.size,
      },
      constants: {
        maxCachedViews: ORACLE_CONSTANTS.MAX_CACHED_VIEWS,
        maxGraphNodes: ORACLE_CONSTANTS.MAX_GRAPH_NODES,
        refreshIntervalMs: ORACLE_CONSTANTS.REFRESH_INTERVAL_MS,
      },
    };
  }

  /**
   * Clear agent state
   */
  clear() {
    this.views.clear();
    this.graphData = null;
    this.lastRefresh = 0;
    this.metricsHistory = [];
    this.alerts = [];
    this.eventCounts.clear();
    this.stats = {
      totalViews: 0,
      totalRefreshes: 0,
      cacheHits: 0,
      cacheMisses: 0,
      alertsGenerated: 0,
    };

    // Reset math module state
    this.healthBelief = new BetaDistribution(2, 1);
    this.trendChain = createMarkovChain(['up', 'stable', 'down']);
    this.metricValues = { events: [], blocks: [], patterns: [] };
  }
}

export default CollectiveOracle;
