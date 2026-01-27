/**
 * CYNIC TUI Dashboard - Widgets Index
 *
 * @module @cynic/node/cli/dashboard/widgets
 */

'use strict';

import { createHealthWidget } from './health.js';
import { createCollectiveWidget } from './collective.js';
import { createSingularityWidget } from './singularity.js';
import { createChainWidget } from './chain.js';
import { createVerdictsWidget } from './verdicts.js';
import { createEventsWidget } from './events.js';

export {
  createHealthWidget,
  createCollectiveWidget,
  createSingularityWidget,
  createChainWidget,
  createVerdictsWidget,
  createEventsWidget,
};

export default {
  createHealthWidget,
  createCollectiveWidget,
  createSingularityWidget,
  createChainWidget,
  createVerdictsWidget,
  createEventsWidget,
};
