/**
 * CYNIC TUI Dashboard - Chain Detail Screen
 *
 * Detailed view of the PoJ blockchain
 *
 * @module @cynic/node/cli/dashboard/screens/chain
 */

'use strict';

import blessed from 'blessed';
import { COLORS, formatTime, truncate, VERDICT_COLORS } from '../theme.js';

/**
 * Create Chain Detail Screen
 */
export function createChainScreen(screen, dataFetcher, options = {}) {
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
    style: { bg: 'magenta', fg: 'white' },
    content: ' {bold}🔗 POJ BLOCKCHAIN - Detailed View{/}',
    tags: true,
  });

  // Status panel
  const statusPanel = blessed.box({
    parent: container,
    label: ' Chain Status ',
    top: 1,
    left: 0,
    width: '40%',
    height: '40%',
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.accent },
      label: { fg: COLORS.accent, bold: true },
    },
    tags: true,
  });

  const statusContent = blessed.box({
    parent: statusPanel,
    top: 0,
    left: 1,
    right: 1,
    bottom: 0,
    tags: true,
  });

  // Recent blocks panel
  const blocksPanel = blessed.box({
    parent: container,
    label: ' Recent Blocks ',
    top: 1,
    left: '40%',
    width: '60%',
    height: '40%',
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.primary },
      label: { fg: COLORS.primary, bold: true },
    },
    tags: true,
  });

  const blocksContent = blessed.list({
    parent: blocksPanel,
    top: 0,
    left: 1,
    right: 1,
    bottom: 0,
    tags: true,
    scrollable: true,
    keys: true,
    vi: true,
    mouse: true,
    style: {
      selected: { bg: 'blue', fg: 'white' },
    },
  });

  // Anchoring history panel
  const anchorPanel = blessed.box({
    parent: container,
    label: ' Solana Anchoring ',
    top: '41%',
    left: 0,
    width: '100%',
    height: '50%',
    border: { type: 'line' },
    style: {
      border: { fg: COLORS.phi },
      label: { fg: COLORS.phi, bold: true },
    },
    tags: true,
  });

  const anchorContent = blessed.log({
    parent: anchorPanel,
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
    content: ' {bold}[V]{/}erify  {bold}[F]{/}lush  {bold}[B]{/}ack to Main  {bold}[Q]{/}uit',
    tags: true,
  });

  // Block cache
  let recentBlocks = [];

  /**
   * Fetch recent blocks
   */
  async function fetchBlocks() {
    const result = await dataFetcher.callTool('brain_poj_chain', {
      action: 'recent',
      limit: 10,
    });

    if (result.success && result.result?.blocks) {
      recentBlocks = result.result.blocks;
      updateBlocksList();
    }
  }

  /**
   * Update blocks list display
   */
  function updateBlocksList() {
    const items = recentBlocks.map(block => {
      const slot = block.slot || block.blockNumber || '?';
      const judgments = block.judgmentCount || block.judgments?.length || 0;
      const merkle = truncate(block.merkleRoot || '', 8);
      return `#${slot.toString().padStart(5)} │ ${judgments} judgments │ ${merkle}`;
    });

    blocksContent.setItems(items);
  }

  /**
   * Update status panel
   */
  function updateStatus(data) {
    const chain = data?.chain || {};

    const lines = [
      `{bold}Initialized:{/} ${chain.initialized ? '{green-fg}Yes{/}' : '{red-fg}No{/}'}`,
      `{bold}Head Block:{/} #${chain.headSlot || 0}`,
      `{bold}Total Blocks:{/} ${chain.totalBlocks || 0}`,
      `{bold}Pending:{/} ${chain.pendingJudgments || 0} judgments`,
      '',
      `{bold}Anchoring:{/} ${chain.anchoringEnabled ? '{green-fg}Enabled{/}' : '{yellow-fg}Disabled{/}'}`,
      `{bold}Network:{/} ${chain.anchorNetwork || 'devnet'}`,
      `{bold}Last Anchor:{/} ${chain.lastAnchorTime ? formatTime(chain.lastAnchorTime) : 'never'}`,
      '',
      `{bold}Merkle Root:{/}`,
      `  ${chain.merkleRoot || '(empty)'}`,
    ];

    statusContent.setContent(lines.join('\n'));
  }

  /**
   * Update screen with data
   */
  function update(data) {
    updateStatus(data);
    screen.render();
  }

  /**
   * Show the screen
   */
  async function show() {
    container.show();
    await fetchBlocks();
    screen.render();
  }

  /**
   * Hide the screen
   */
  function hide() {
    container.hide();
  }

  /**
   * Verify chain integrity
   */
  async function verify() {
    anchorContent.log('{cyan-fg}Verifying chain integrity...{/}');
    screen.render();

    const result = await dataFetcher.callTool('brain_poj_chain', { action: 'verify' });

    if (result.success) {
      const r = result.result;
      if (r.valid) {
        anchorContent.log(`{green-fg}✓ Chain valid! ${r.blocksChecked} blocks verified.{/}`);
      } else {
        anchorContent.log(`{red-fg}✗ Chain invalid! ${r.errors?.length || 0} errors found.{/}`);
        for (const err of (r.errors || [])) {
          anchorContent.log(`  {red-fg}• ${err}{/}`);
        }
      }
    } else {
      anchorContent.log(`{red-fg}Error: ${result.error}{/}`);
    }

    screen.render();
  }

  /**
   * Force create a new block
   */
  async function flush() {
    anchorContent.log('{yellow-fg}Flushing pending judgments to new block...{/}');
    screen.render();

    const result = await dataFetcher.callTool('brain_poj_chain', { action: 'flush' });

    if (result.success) {
      anchorContent.log(`{green-fg}✓ Block created: #${result.result.blockNumber || '?'}{/}`);
      await fetchBlocks();
    } else {
      anchorContent.log(`{red-fg}Error: ${result.error}{/}`);
    }

    screen.render();
  }

  return {
    container,
    update,
    show,
    hide,
    verify,
    flush,
  };
}

export default createChainScreen;
