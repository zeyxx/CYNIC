/**
 * System Domain Tools
 *
 * Tools for system operations:
 * - Health: System health checks
 * - Metrics: Performance metrics
 * - Diagnostics: Agent diagnostics
 * - Collective: Multi-agent status
 *
 * @module @cynic/mcp/tools/domains/system
 */

'use strict';

import { PHI_INV, IDENTITY } from '@cynic/core';

// φ⁻² = 0.382 (secondary threshold)
const PHI_INV_2 = 0.382;

/**
 * Create health tool definition
 * @param {Object} node - CYNICNode instance (optional)
 * @param {Object} judge - CYNICJudge instance
 * @param {Object} persistence - PersistenceManager instance (optional)
 * @param {Object} automationExecutor - AutomationExecutor instance (optional)
 * @param {Object} thermodynamics - ThermodynamicsTracker instance (optional, Phase 2)
 * @param {Object} heartbeat - HeartbeatService instance (optional, AXE 5: OBSERVE)
 * @param {Object} slaTracker - SLATracker instance (optional, AXE 5: OBSERVE)
 * @returns {Object} Tool definition
 */
export function createHealthTool(node, judge, persistence = null, automationExecutor = null, thermodynamics = null, heartbeat = null, slaTracker = null) {
  return {
    name: 'brain_health',
    description: 'Get CYNIC system health status including uptime, SLA compliance, node status, and capability metrics.',
    inputSchema: {
      type: 'object',
      properties: {
        verbose: { type: 'boolean', description: 'Include detailed statistics' },
      },
    },
    handler: async (params) => {
      const { verbose = false } = params;

      const health = {
        status: 'healthy',
        identity: {
          name: IDENTITY.name,
          greek: IDENTITY.greek,
        },
        phi: { maxConfidence: PHI_INV, minDoubt: PHI_INV_2 },
        timestamp: Date.now(),
      };

      if (node) {
        const info = node.getInfo();
        health.node = {
          status: info.status,
          uptime: info.uptime,
          id: info.id?.slice(0, 16) + '...',
        };
      }

      // Add persistence health
      if (persistence) {
        try {
          health.persistence = await persistence.health();
          health.persistence.capabilities = persistence.capabilities;
        } catch (e) {
          health.persistence = { status: 'error', error: e.message };
        }
      }

      // AXE 5: OBSERVE - Uptime awareness (99.9% target)
      if (heartbeat) {
        try {
          const status = heartbeat.getStatus();
          const uptime1h = status.metrics.systemUptime1h || 0;
          const uptime24h = status.metrics.systemUptime24h || 0;

          // Progress bars
          const uptimeBar = '█'.repeat(Math.round(uptime1h * 10)) +
                            '░'.repeat(10 - Math.round(uptime1h * 10));

          health.uptime = {
            overall: status.overall.status,
            system: Math.round(status.metrics.systemUptime * 1000) / 10,
            system1h: Math.round(uptime1h * 1000) / 10,
            system24h: Math.round(uptime24h * 1000) / 10,
            uptimeBar,
            components: Object.fromEntries(
              Object.entries(status.components).map(([name, c]) => [
                name,
                {
                  healthy: c.healthy,
                  uptime: Math.round(c.uptime * 1000) / 10,
                  latencyMs: c.latencyMs,
                  error: c.error,
                },
              ])
            ),
            running: status.running,
            pings: status.metrics.totalPings,
          };

          // Update overall status based on uptime
          if (status.overall.status === 'critical') {
            health.status = 'critical';
          } else if (status.overall.status === 'degraded' && health.status !== 'critical') {
            health.status = 'degraded';
          }
        } catch (e) {
          health.uptime = { error: e.message };
        }
      }

      // AXE 5: OBSERVE - SLA compliance (99.9% target)
      if (slaTracker) {
        try {
          const slaStatus = slaTracker.getStatus();
          health.sla = {
            status: slaStatus.status,
            compliant: slaTracker.isCompliant(),
            violations24h: slaStatus.violationCount24h,
            targets: {
              uptime: '99.9%',
              postgres: '99.95%',
              mcpP99: '<500ms',
            },
            recentViolations: slaStatus.recentViolations?.slice(0, 3).map(v => ({
              target: v.target,
              severity: v.severity,
            })),
          };
        } catch (e) {
          health.sla = { error: e.message };
        }
      }

      // Add daemon stats (Phase 18: Automation Layer)
      if (automationExecutor) {
        try {
          const daemonStats = automationExecutor.getStats?.() || {};
          health.daemon = {
            running: automationExecutor.running || false,
            uptime: daemonStats.uptime || 0,
            tasksProcessed: daemonStats.tasksProcessed || 0,
            goalsUpdated: daemonStats.goalsUpdated || 0,
            learningCycles: daemonStats.learningCycles || 0,
            triggersEvaluated: daemonStats.triggersEvaluated || 0,
            lastLearningCycle: daemonStats.lastLearningCycle || null,
          };
        } catch (e) {
          health.daemon = { running: false, error: e.message };
        }
      }

      // Add thermodynamics state (Phase 2)
      // "Ἐνέργεια - the activity of being" - κυνικός
      if (thermodynamics) {
        try {
          const thermoState = thermodynamics.getState();
          const recommendation = thermodynamics.getRecommendation();
          const stats = thermodynamics.getStats();

          // Create progress bars
          const tempBar = '█'.repeat(Math.round(Math.min(100, (thermoState.temperature / 81) * 100) / 10)) +
                          '░'.repeat(10 - Math.round(Math.min(100, (thermoState.temperature / 81) * 100) / 10));
          const effBar = '█'.repeat(Math.round(thermoState.efficiency / 10)) +
                         '░'.repeat(10 - Math.round(thermoState.efficiency / 10));

          health.thermodynamics = {
            heat: thermoState.heat,
            work: thermoState.work,
            entropy: thermoState.entropy,
            temperature: thermoState.temperature,
            temperatureBar: tempBar,
            efficiency: thermoState.efficiency,
            efficiencyBar: effBar,
            carnotLimit: thermoState.carnotLimit,
            isCritical: thermoState.isCritical,
            isLowEfficiency: thermoState.isLowEfficiency,
            sessionDuration: thermoState.sessionDuration,
            recommendation: {
              level: recommendation.level,
              message: recommendation.message,
              action: recommendation.action,
            },
            stats: {
              thermalRunaways: stats.thermalRunaways,
              entropyResets: stats.entropyResets,
              peakEfficiency: Math.round(stats.peakEfficiency * 100),
              totals: stats.totals,
            },
          };
        } catch (e) {
          health.thermodynamics = { error: e.message };
        }
      }

      if (verbose) {
        health.judge = judge.getStats();
        health.tools = ['brain_cynic_judge', 'brain_cynic_digest', 'brain_health', 'brain_search', 'brain_patterns', 'brain_cynic_feedback', 'brain_collective_status'];

        // Add judgment stats from persistence
        if (persistence?.judgments) {
          try {
            health.judgmentStats = await persistence.getJudgmentStats();
          } catch (e) {
            // Ignore
          }
        }
      }

      return health;
    },
  };
}

/**
 * P4: createAgentsStatusTool REMOVED — deprecated, use brain_collective_status
 */
function _dead_createAgentsStatusTool(collective) {
  return {
    name: 'brain_agents_status',
    description: 'DEPRECATED: Use brain_collective_status instead. Returns collective status for backwards compatibility.',
    inputSchema: {
      type: 'object',
      properties: {
        verbose: { type: 'boolean', description: 'Include detailed per-agent statistics' },
        agent: { type: 'string', enum: ['guardian', 'analyst', 'scholar', 'architect', 'sage', 'cynic', 'janitor', 'scout', 'cartographer', 'oracle', 'deployer'], description: 'Get status for specific agent only' },
      },
    },
    handler: async (params) => {
      const { verbose = false, agent = null } = params;

      // Redirect to collective
      if (!collective) {
        return {
          status: 'unavailable',
          message: 'Collective not initialized. Use brain_collective_status.',
          deprecated: true,
          timestamp: Date.now(),
        };
      }

      const summary = collective.getSummary();

      // If specific agent requested
      if (agent && summary.agents[agent]) {
        return {
          agent,
          ...summary.agents[agent],
          deprecated: true,
          useInstead: 'brain_collective_status',
          timestamp: Date.now(),
        };
      }

      // Return summary with deprecation notice
      return {
        deprecated: true,
        useInstead: 'brain_collective_status',
        dogCount: summary.dogCount,
        profileLevel: summary.profileLevel,
        dogs: Object.fromEntries(
          Object.entries(summary.agents).map(([name, data]) => [
            name,
            {
              invocations: data.invocations || 0,
              actions: data.actions || 0,
              blocks: data.blocks || 0,
              warnings: data.warnings || 0,
            },
          ])
        ),
        timestamp: Date.now(),
      };
    },
  };
}

/**
 * Create metrics tool definition
 * @param {Object} metricsService - MetricsService instance
 * @returns {Object} Tool definition
 */
export function createMetricsTool(metricsService) {
  return {
    name: 'brain_metrics',
    description: 'Get CYNIC metrics in various formats. Prometheus format for monitoring, JSON for inspection, HTML for dashboard.',
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['collect', 'prometheus', 'alerts', 'clear_alert', 'stats', 'html'],
          description: 'Action: collect (raw metrics), prometheus (Prometheus format), alerts (active alerts), clear_alert (acknowledge alert), stats (service stats), html (dashboard)',
        },
        alertType: {
          type: 'string',
          description: 'Alert type to clear (for clear_alert action)',
        },
      },
    },
    handler: async (params) => {
      const { action = 'collect', alertType } = params;

      if (!metricsService) {
        return {
          error: 'Metrics service not available',
          hint: 'MetricsService provides monitoring capabilities',
          timestamp: Date.now(),
        };
      }

      switch (action) {
        case 'collect': {
          const metrics = await metricsService.collect();
          return {
            action: 'collect',
            metrics,
            alerts: metricsService.getAlerts(),
            message: `*sniff* Collected metrics in ${metricsService.getStats().lastCollectMs}ms.`,
            timestamp: Date.now(),
          };
        }

        case 'prometheus': {
          const prometheus = await metricsService.toPrometheus();
          return {
            action: 'prometheus',
            format: 'text/plain',
            content: prometheus,
            message: '*tail wag* Metrics exported in Prometheus format.',
            timestamp: Date.now(),
          };
        }

        case 'alerts': {
          const alerts = metricsService.getAlerts();
          return {
            action: 'alerts',
            alerts,
            total: alerts.length,
            critical: alerts.filter(a => a.level === 'critical').length,
            warning: alerts.filter(a => a.level === 'warning').length,
            message: alerts.length === 0
              ? '*tail wag* No active alerts.'
              : `*ears perk* ${alerts.length} active alerts (${alerts.filter(a => a.level === 'critical').length} critical).`,
            timestamp: Date.now(),
          };
        }

        case 'clear_alert': {
          if (!alertType) {
            return {
              error: 'alertType required for clear_alert action',
              availableAlerts: metricsService.getAlerts().map(a => a.type),
              timestamp: Date.now(),
            };
          }
          const cleared = metricsService.clearAlert(alertType);
          return {
            action: 'clear_alert',
            alertType,
            cleared,
            message: cleared
              ? `*yawn* Alert '${alertType}' acknowledged.`
              : `*head tilt* Alert '${alertType}' not found.`,
            timestamp: Date.now(),
          };
        }

        case 'stats': {
          const stats = metricsService.getStats();
          return {
            action: 'stats',
            ...stats,
            message: `*sniff* ${stats.collectCount} collections, ${stats.alertsTriggered} alerts triggered.`,
            timestamp: Date.now(),
          };
        }

        case 'html': {
          const html = await metricsService.toHTML();
          return {
            action: 'html',
            format: 'text/html',
            content: html,
            message: '*tail wag* Dashboard HTML generated.',
            timestamp: Date.now(),
          };
        }

        default:
          return {
            error: `Unknown action: ${action}`,
            validActions: ['collect', 'prometheus', 'alerts', 'clear_alert', 'stats', 'html'],
            timestamp: Date.now(),
          };
      }
    },
  };
}

/**
 * Create collective status tool definition
 * @param {Object} collective - CollectivePack instance (The Five Dogs + CYNIC)
 * @returns {Object} Tool definition
 */
export function createCollectiveStatusTool(collective) {
  return {
    name: 'brain_collective_status',
    description: 'Get status and statistics for The Collective (11 Dogs + CYNIC meta-consciousness). Shows all agents with Sefirot mappings, event bus stats, and CYNIC\'s current state.',
    inputSchema: {
      type: 'object',
      properties: {
        verbose: { type: 'boolean', description: 'Include detailed per-agent statistics' },
        agent: {
          type: 'string',
          enum: ['guardian', 'analyst', 'scholar', 'architect', 'sage', 'cynic', 'janitor', 'scout', 'cartographer', 'oracle', 'deployer'],
          description: 'Get status for specific agent only',
        },
      },
    },
    handler: async (params) => {
      const { verbose = false, agent = null } = params;

      if (!collective) {
        return {
          status: 'unavailable',
          message: '*growl* Collective not initialized. Only legacy agents available.',
          hint: 'Use brain_agents_status for legacy Four Dogs.',
          timestamp: Date.now(),
        };
      }

      const summary = collective.getSummary();
      const collectiveState = collective.getCollectiveState();

      // Sefirot mapping for display
      const sefirotMap = {
        guardian: { sefira: 'Gevurah', meaning: 'Strength', role: 'Security & Protection' },
        analyst: { sefira: 'Binah', meaning: 'Understanding', role: 'Pattern Analysis' },
        scholar: { sefira: 'Daat', meaning: 'Knowledge', role: 'Knowledge Extraction' },
        architect: { sefira: 'Chesed', meaning: 'Kindness', role: 'Design Review' },
        sage: { sefira: 'Chochmah', meaning: 'Wisdom', role: 'Guidance & Teaching' },
        cynic: { sefira: 'Keter', meaning: 'Crown', role: 'Meta-Consciousness' },
        janitor: { sefira: 'Yesod', meaning: 'Foundation', role: 'Code Quality' },
        scout: { sefira: 'Netzach', meaning: 'Victory', role: 'Discovery' },
        cartographer: { sefira: 'Malkhut', meaning: 'Kingdom', role: 'Reality Mapping' },
        oracle: { sefira: 'Tiferet', meaning: 'Beauty', role: 'Visualization' },
        deployer: { sefira: 'Hod', meaning: 'Splendor', role: 'Deployment' },
      };

      // If specific agent requested
      if (agent && summary.agents[agent]) {
        const agentSummary = summary.agents[agent];
        const sefira = sefirotMap[agent];
        return {
          agent,
          sefira: sefira?.sefira,
          meaning: sefira?.meaning,
          role: sefira?.role,
          ...agentSummary,
          profileLevel: summary.profileLevel,
          timestamp: Date.now(),
        };
      }

      // Build dogs summary with Sefirot info
      const dogs = {};
      for (const [name, info] of Object.entries(sefirotMap)) {
        const agentData = summary.agents[name];
        dogs[name] = {
          sefira: info.sefira,
          meaning: info.meaning,
          role: info.role,
          active: !!agentData,
          ...(agentData ? {
            invocations: agentData.invocations || agentData.stats?.invocations || 0,
          } : {}),
        };
      }

      // Basic response
      const response = {
        status: 'active',
        dogCount: summary.dogCount,
        agentCount: summary.agentCount,
        profileLevel: summary.profileLevel,
        cynicState: collectiveState?.metaState || 'unknown',
        eventBusStats: summary.eventBusStats,
        dogs,
        collectiveStats: summary.collectiveStats,
        message: `*tail wag* Collective active. CYNIC at ${collectiveState?.metaState || 'unknown'} state. ${summary.agentCount} agents ready.`,
        timestamp: Date.now(),
      };

      // Add verbose details
      if (verbose) {
        response.agents = summary.agents;
        response.collectiveState = collectiveState;
      }

      return response;
    },
  };
}

/**
 * Create agent diagnostic tool - tests agent routing directly
 * @param {Object} collective - CollectivePack instance
 * @returns {Object} Tool definition
 */
export function createAgentDiagnosticTool(collective) {
  return {
    name: 'brain_agent_diagnostic',
    description: 'INTERNAL: Test agent routing and shouldTrigger logic directly. For debugging only.',
    inputSchema: {
      type: 'object',
      properties: {
        eventType: { type: 'string', description: 'Event type to test (e.g., PostConversation)' },
        testContent: { type: 'string', description: 'Test content for the event' },
      },
    },
    handler: async (params) => {
      const { eventType = 'PostConversation', testContent = 'Test content' } = params;

      if (!collective) {
        return { error: 'Collective not available', timestamp: Date.now() };
      }

      // Create test event
      const testEvent = {
        type: eventType,
        content: testContent,
        tool: 'brain_agent_diagnostic',
        timestamp: Date.now(),
      };

      // Get all agents from collective and test shouldTrigger
      const agentResults = {};
      for (const agent of collective.agents) {
        const shouldTrigger = agent.shouldTrigger(testEvent);
        agentResults[agent.name] = {
          exists: true,
          sefirah: agent.sefirah,
          trigger: agent.trigger,
          shouldTriggerResult: shouldTrigger,
          invocations: agent.stats?.invocations || 0,
        };
      }

      const results = {
        testEvent,
        agents: agentResults,
        collectiveStats: collective.getStats?.() || {},
        timestamp: Date.now(),
      };

      // Test full pipeline via processEvent
      try {
        const processResult = await collective.processEvent(testEvent, {});
        results.processResult = processResult;
      } catch (e) {
        results.processError = e.message;
      }

      return results;
    },
  };
}

/**
 * Create consensus tool for inter-agent voting
 * @param {Object} collective - CollectivePack instance
 * @returns {Object} Tool definition
 */
export function createConsensusTool(collective) {
  return {
    name: 'brain_consensus',
    description: 'Request inter-agent consensus voting. Ask the 11 Dogs (Sefirot) to vote on a question. Use for important decisions that benefit from collective wisdom. Returns voting results with approval/rejection based on φ⁻¹ (61.8%) threshold.',
    inputSchema: {
      type: 'object',
      properties: {
        question: {
          type: 'string',
          description: 'The question to vote on (e.g., "Should we proceed with this risky operation?")',
        },
        context: {
          type: 'object',
          description: 'Additional context for agents to consider when voting',
          properties: {
            content: { type: 'string', description: 'Main content being evaluated' },
            risk: { type: 'string', enum: ['low', 'medium', 'high', 'critical'], description: 'Risk level' },
            domain: { type: 'string', description: 'Domain of the decision (security, design, knowledge, etc.)' },
          },
        },
        requiredVotes: {
          type: 'number',
          description: 'Minimum votes required (default: 3)',
          default: 3,
        },
        timeout: {
          type: 'number',
          description: 'Timeout in milliseconds (default: 10000)',
          default: 10000,
        },
      },
      required: ['question'],
    },
    handler: async (params) => {
      const { question, context = {} } = params;

      if (!collective) {
        return {
          status: 'unavailable',
          message: '*growl* Collective not initialized. Cannot request consensus.',
          timestamp: Date.now(),
        };
      }

      try {
        // Poll all 11 Sefirot directly using voteOnConsensus
        const allAgents = [
          'guardian', 'analyst', 'scholar', 'architect', 'sage',
          'cynic', 'janitor', 'scout', 'cartographer', 'oracle', 'deployer',
        ];

        const votes = [];

        for (const agentName of allAgents) {
          const agent = collective[agentName];
          if (agent && typeof agent.voteOnConsensus === 'function') {
            try {
              const result = agent.voteOnConsensus(question, context);
              votes.push({
                agent: agentName,
                vote: result.vote?.toUpperCase?.() || 'ABSTAIN',
                reason: result.reason || 'No reason provided',
              });
            } catch (e) {
              votes.push({ agent: agentName, vote: 'ABSTAIN', reason: `Error: ${e.message}` });
            }
          } else {
            // Agent doesn't have voteOnConsensus - use default abstain
            votes.push({ agent: agentName, vote: 'ABSTAIN', reason: 'No voting method available' });
          }
        }

        // Calculate consensus (only count approve/reject, not abstain)
        const approveCount = votes.filter(v => v.vote === 'APPROVE').length;
        const rejectCount = votes.filter(v => v.vote === 'REJECT').length;
        const decidingVotes = approveCount + rejectCount;

        // φ⁻² = 38.2% veto threshold - if rejects exceed this, veto
        const vetoThreshold = PHI_INV_2;
        const hasVeto = decidingVotes > 0 && (rejectCount / decidingVotes) > vetoThreshold;

        // φ⁻¹ = 61.8% approval threshold
        const approvalRatio = decidingVotes > 0 ? approveCount / decidingVotes : 0;
        const approved = !hasVeto && approvalRatio >= PHI_INV;

        // Build visual consensus display
        const abstainCount = votes.filter(v => v.vote === 'ABSTAIN').length;
        const visualVotes = votes.map(v => {
          const icon = v.vote === 'APPROVE' ? '✓' : v.vote === 'REJECT' ? '✗' : '○';
          return `${icon} ${v.agent.padEnd(12)}: ${v.vote.padEnd(8)}`;
        }).join('\n');

        const verdict = hasVeto
          ? '✗ VETOED'
          : approved
            ? '✓ APPROVED'
            : '✗ NOT APPROVED';

        const consensusDisplay = `
╔══════════════════════════════════════════════════════════════════╗
║ COLLECTIVE CONSENSUS                                              ║
║ "${question.substring(0, 50)}${question.length > 50 ? '...' : ''}"
╠══════════════════════════════════════════════════════════════════╣
${votes.map(v => {
  const icon = v.vote === 'APPROVE' ? '✓' : v.vote === 'REJECT' ? '✗' : '○';
  return `║ ${icon} ${v.agent.padEnd(12)}: ${v.vote.padEnd(8)}                                   ║`;
}).join('\n')}
╠══════════════════════════════════════════════════════════════════╣
║ Approve=${String(approveCount).padEnd(2)} Reject=${String(rejectCount).padEnd(2)} Abstain=${String(abstainCount).padEnd(2)} → ${(approvalRatio * 100).toFixed(1)}% (need 61.8%)       ║
║ RESULT: ${verdict.padEnd(57)}║
╚══════════════════════════════════════════════════════════════════╝`.trim();

        return {
          status: 'completed',
          approved,
          verdict,
          reason: hasVeto
            ? `Vetoed (${rejectCount} rejects exceed ${(vetoThreshold * 100).toFixed(1)}% threshold)`
            : approved
              ? `Consensus reached (${(approvalRatio * 100).toFixed(1)}% approval)`
              : `Consensus not reached (${(approvalRatio * 100).toFixed(1)}% approval, need ${(PHI_INV * 100).toFixed(1)}%)`,
          // Visual display for CLI output
          display: consensusDisplay,
          message: approved
            ? `*tail wag* The pack agrees. ${verdict}`
            : hasVeto
              ? `*GROWL* The pack rejects this. ${verdict}`
              : `*head tilt* No consensus. ${verdict}`,
          votes,
          stats: {
            approve: approveCount,
            reject: rejectCount,
            abstain: abstainCount,
            total: votes.length,
            decidingVotes,
            approvalRatio,
            vetoThreshold,
            consensusThreshold: PHI_INV,
          },
          timestamp: Date.now(),
        };
      } catch (e) {
        return {
          status: 'error',
          message: `*growl* Consensus failed: ${e.message}`,
          timestamp: Date.now(),
        };
      }
    },
  };
}

/**
 * Factory for system domain tools
 */
export const systemFactory = {
  name: 'system',
  domain: 'system',
  requires: [],

  /**
   * Create all system domain tools
   * @param {Object} options
   * @returns {Object[]} Tool definitions
   */
  create(options) {
    const {
      node,
      judge,
      persistence,
      metricsService,
      collective,
    } = options;

    const tools = [];

    // Health tool
    if (judge) {
      tools.push(createHealthTool(node, judge, persistence));
    }

    // Metrics tool
    if (metricsService) {
      tools.push(createMetricsTool(metricsService));
    }

    // Collective status
    if (collective) {
      tools.push(createCollectiveStatusTool(collective));
      // P4: createAgentsStatusTool removed (deprecated → brain_collective_status)
      tools.push(createAgentDiagnosticTool(collective));
      tools.push(createConsensusTool(collective));
    }

    return tools;
  },
};
