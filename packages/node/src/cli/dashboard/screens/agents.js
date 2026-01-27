/**
 * CYNIC TUI Dashboard - Agents Detail Screen
 *
 * Detailed view of The 11 Dogs (Collective)
 *
 * @module @cynic/node/cli/dashboard/screens/agents
 */

'use strict';

import blessed from 'blessed';
import { COLORS, DOG_ICONS, progressBar } from '../theme.js';

/**
 * Dog configurations with full details
 */
const DOG_CONFIG = {
  guardian: {
    name: 'Guardian',
    sefira: 'Gevurah',
    meaning: 'Strength',
    role: 'Security & Protection',
    description: 'Protects against security vulnerabilities and dangerous operations.',
  },
  analyst: {
    name: 'Analyst',
    sefira: 'Binah',
    meaning: 'Understanding',
    role: 'Pattern Analysis',
    description: 'Analyzes patterns and extracts insights from judgments.',
  },
  scholar: {
    name: 'Scholar',
    sefira: 'Daat',
    meaning: 'Knowledge',
    role: 'Knowledge Extraction',
    description: 'Extracts and preserves knowledge from conversations.',
  },
  architect: {
    name: 'Architect',
    sefira: 'Chesed',
    meaning: 'Kindness',
    role: 'Design Review',
    description: 'Reviews architecture and suggests improvements.',
  },
  sage: {
    name: 'Sage',
    sefira: 'Chochmah',
    meaning: 'Wisdom',
    role: 'Guidance & Teaching',
    description: 'Provides wisdom and guidance from past learnings.',
  },
  cynic: {
    name: 'CYNIC',
    sefira: 'Keter',
    meaning: 'Crown',
    role: 'Meta-Consciousness',
    description: 'The central consciousness coordinating all dogs.',
  },
  janitor: {
    name: 'Janitor',
    sefira: 'Yesod',
    meaning: 'Foundation',
    role: 'Code Quality',
    description: 'Maintains code cleanliness and quality standards.',
  },
  scout: {
    name: 'Scout',
    sefira: 'Netzach',
    meaning: 'Victory',
    role: 'Discovery',
    description: 'Discovers new patterns and opportunities.',
  },
  cartographer: {
    name: 'Cartographer',
    sefira: 'Malkhut',
    meaning: 'Kingdom',
    role: 'Reality Mapping',
    description: 'Maps the codebase and its relationships.',
  },
  oracle: {
    name: 'Oracle',
    sefira: 'Tiferet',
    meaning: 'Beauty',
    role: 'Visualization',
    description: 'Reveals hidden connections and patterns.',
  },
  deployer: {
    name: 'Deployer',
    sefira: 'Hod',
    meaning: 'Splendor',
    role: 'Deployment',
    description: 'Handles deployment and infrastructure tasks.',
  },
};

const DOG_ORDER = [
  'cynic', 'guardian', 'analyst', 'scholar', 'architect',
  'sage', 'janitor', 'scout', 'cartographer', 'oracle', 'deployer',
];

/**
 * Create Agents Screen
 */
export function createAgentsScreen(screen, dataFetcher, options = {}) {
  const container = blessed.box({
    parent: screen,
    top: 0,
    left: 0,
    width: '100%',
    height: '100%',
    tags: true,
    hidden: true,
  });

  // Header
  const header = blessed.box({
    parent: container,
    top: 0,
    left: 0,
    width: '100%',
    height: 1,
    style: { bg: 'green', fg: 'white' },
    content: ' {bold}🐕 THE COLLECTIVE - 11 Dogs + Meta-Consciousness{/}',
    tags: true,
  });

  // Agent list
  const agentList = blessed.list({
    parent: container,
    label: ' Agents ',
    top: 1,
    left: 0,
    width: '30%',
    height: '90%',
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.success },
      label: { fg: COLORS.success, bold: true },
      selected: { bg: 'green', fg: 'white' },
    },
    tags: true,
    scrollable: true,
    keys: true,
    vi: true,
    mouse: true,
  });

  // Agent detail
  const detailPanel = blessed.box({
    parent: container,
    label: ' Agent Detail ',
    top: 1,
    left: '30%',
    width: '70%',
    height: '90%',
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.primary },
      label: { fg: COLORS.primary, bold: true },
    },
    tags: true,
  });

  const detailContent = blessed.box({
    parent: detailPanel,
    top: 0,
    left: 1,
    right: 1,
    bottom: 0,
    tags: true,
    scrollable: true,
  });

  // Footer
  const footer = blessed.box({
    parent: container,
    bottom: 0,
    left: 0,
    width: '100%',
    height: 1,
    style: { bg: 'black', fg: 'white' },
    content: ' {bold}[↑↓]{/} Navigate  {bold}[Enter]{/} Details  {bold}[D]{/}iagnostic  {bold}[B]{/}ack  {bold}[Q]{/}uit',
    tags: true,
  });

  // Cache collective data
  let collectiveData = {};

  /**
   * Fetch collective status
   */
  async function fetchCollective() {
    const result = await dataFetcher.callTool('brain_collective_status', { verbose: true });

    if (result.success) {
      collectiveData = result.result;
      updateAgentList();
    }
  }

  /**
   * Update agent list
   */
  function updateAgentList() {
    const dogs = collectiveData.dogs || {};

    const items = DOG_ORDER.map(name => {
      const config = DOG_CONFIG[name];
      const data = dogs[name] || {};
      const icon = DOG_ICONS[name] || '🐕';
      const active = data.active;
      const status = active ? '{green-fg}●{/}' : '{gray-fg}○{/}';
      return `${icon} ${config.name.padEnd(12)} ${status}`;
    });

    agentList.setItems(items);
  }

  /**
   * Show agent detail
   */
  function showDetail(index) {
    const name = DOG_ORDER[index];
    const config = DOG_CONFIG[name];
    const dogs = collectiveData.dogs || {};
    const data = dogs[name] || {};
    const agents = collectiveData.agents || {};
    const agentData = agents[name] || {};

    const icon = DOG_ICONS[name] || '🐕';
    const active = data.active;

    const lines = [
      `{bold}${icon} ${config.name}{/}`,
      '',
      `{bold}Sefira:{/} ${config.sefira} (${config.meaning})`,
      `{bold}Role:{/} ${config.role}`,
      `{bold}Status:{/} ${active ? '{green-fg}Active{/}' : '{gray-fg}Inactive{/}'}`,
      '',
      '{bold}Description:{/}',
      config.description,
      '',
      '{bold}Statistics:{/}',
      `  Invocations: ${data.invocations || agentData.invocations || 0}`,
      `  Actions: ${agentData.actions || 0}`,
      `  Blocks: ${agentData.blocks || 0}`,
      `  Warnings: ${agentData.warnings || 0}`,
    ];

    // Show stats if available
    if (agentData.stats) {
      lines.push('');
      lines.push('{bold}Detailed Stats:{/}');
      for (const [key, value] of Object.entries(agentData.stats)) {
        lines.push(`  ${key}: ${value}`);
      }
    }

    // Special display for CYNIC (Keter)
    if (name === 'cynic') {
      lines.push('');
      lines.push('{bold}Meta-State:{/} ' + (collectiveData.cynicState || 'unknown'));
      lines.push('{bold}Dog Count:{/} ' + (collectiveData.dogCount || 11));
      lines.push('{bold}Profile Level:{/} ' + (collectiveData.profileLevel || 'unknown'));
    }

    detailContent.setContent(lines.join('\n'));
    screen.render();
  }

  // Event handlers
  agentList.on('select', (item, index) => {
    showDetail(index);
  });

  /**
   * Run diagnostic for selected agent
   */
  async function runDiagnostic(index) {
    const name = DOG_ORDER[index];

    detailContent.setContent(`Running diagnostic for ${name}...`);
    screen.render();

    const result = await dataFetcher.callTool('brain_agent_diagnostic', {
      eventType: 'PostConversation',
      testContent: 'Dashboard diagnostic test',
    });

    if (result.success) {
      const agents = result.result.agents || {};
      const agentResult = agents[name];

      const lines = [
        `{bold}Diagnostic Result for ${name}{/}`,
        '',
        `{bold}Exists:{/} ${agentResult?.exists ? 'Yes' : 'No'}`,
        `{bold}Sefirah:{/} ${agentResult?.sefirah || 'unknown'}`,
        `{bold}Trigger:{/} ${agentResult?.trigger || 'unknown'}`,
        `{bold}Should Trigger:{/} ${agentResult?.shouldTriggerResult ? '{green-fg}Yes{/}' : '{yellow-fg}No{/}'}`,
        `{bold}Invocations:{/} ${agentResult?.invocations || 0}`,
        '',
        '{bold}Process Result:{/}',
        JSON.stringify(result.result.processResult || {}, null, 2).slice(0, 500),
      ];

      detailContent.setContent(lines.join('\n'));
    } else {
      detailContent.setContent(`Diagnostic failed: ${result.error}`);
    }

    screen.render();
  }

  /**
   * Update with data
   */
  function update(data) {
    if (data?.collective) {
      collectiveData = data.collective;
      updateAgentList();
    }
  }

  /**
   * Show the screen
   */
  async function show() {
    container.show();
    agentList.focus();
    await fetchCollective();
    if (DOG_ORDER.length > 0) {
      showDetail(0);
    }
    screen.render();
  }

  /**
   * Hide the screen
   */
  function hide() {
    container.hide();
  }

  /**
   * Get selected index
   */
  function getSelectedIndex() {
    return agentList.selected || 0;
  }

  return {
    container,
    update,
    show,
    hide,
    runDiagnostic,
    getSelectedIndex,
  };
}

export default createAgentsScreen;
